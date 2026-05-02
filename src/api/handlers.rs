use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::io::{Cursor, Read};
use std::sync::Arc;
use uuid::Uuid;
use zip::ZipArchive;

use crate::api::models::*;
use crate::api::state::AppState;

// ─── SKILL.md Frontmatter Parsing ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SkillFrontmatter {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub when_to_use: Option<String>,
    #[serde(default)]
    pub argument_hint: Option<String>,
    #[serde(default)]
    pub arguments: Option<serde_yaml::Value>,
    #[serde(default)]
    pub disable_model_invocation: Option<bool>,
    #[serde(default)]
    pub user_invocable: Option<bool>,
    #[serde(default)]
    pub allowed_tools: Option<serde_yaml::Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub hooks: Option<serde_yaml::Value>,
    #[serde(default)]
    pub paths: Option<serde_yaml::Value>,
    #[serde(default)]
    pub shell: Option<String>,
}

#[derive(Debug)]
pub struct ParsedSkillMarkdown {
    pub frontmatter: SkillFrontmatter,
    pub content: String,
}

/// Convert a serde_yaml::Value to serde_json::Value
fn yaml_to_json(yaml_val: &serde_yaml::Value) -> Result<serde_json::Value, String> {
    // Use serde to convert via serialization
    serde_json::to_value(yaml_val)
        .map_err(|e| format!("Failed to convert YAML value to JSON: {}", e))
}

fn parse_skill_markdown(content: &str) -> Result<ParsedSkillMarkdown, String> {
    // Find frontmatter markers
    if !content.starts_with("---") {
        return Err("Missing opening frontmatter marker".to_string());
    }

    // Find the closing marker
    let content_without_opening = &content[3..]; // Remove "---"
    let closing_pos = content_without_opening
        .find("---")
        .ok_or("Missing closing frontmatter marker")?;

    // Extract frontmatter and body
    let frontmatter_str = &content_without_opening[..closing_pos];
    let markdown_content = &content_without_opening[closing_pos + 3..]; // Skip closing "---"

    // Parse YAML frontmatter
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| format!("Failed to parse frontmatter: {}", e))?;

    Ok(ParsedSkillMarkdown {
        frontmatter,
        content: markdown_content.trim().to_string(),
    })
}

// ─── Router ──────────────────────────────────────────────────────

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Providers
        .route("/api/providers", get(list_providers).post(create_provider))
        .route(
            "/api/providers/{id}",
            get(get_provider)
                .put(update_provider)
                .delete(delete_provider),
        )
        .route("/api/providers/{id}/activate", post(activate_provider))
        // Skills
        .route("/api/skills", get(list_skills).post(add_skill))
        .route("/api/skills/upload", post(upload_skill_package))
        .route(
            "/api/skills/{name}",
            delete(remove_skill).patch(toggle_skill),
        )
        // Tools
        .route("/api/tools", get(list_tools))
        // Chat
        .route("/api/chat", post(send_chat_message))
        .route(
            "/api/chat/history",
            get(get_chat_history).delete(clear_chat_history),
        )
        // Agent status
        .route("/api/agent/status", get(get_status))
        // ACP config
        .route(
            "/api/acp/config",
            get(get_acp_config).put(update_acp_config),
        )
        .with_state(state)
}

// ─── Provider Handlers ───────────────────────────────────────────

async fn list_providers(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderDto>> {
    let providers = state.providers.read().await;
    let active_id = state.active_provider_id.read().await;

    let list: Vec<ProviderDto> = providers
        .values()
        .map(|p| stored_provider_to_dto(p, active_id.as_deref()))
        .collect();

    Json(list)
}

async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProviderDto>, StatusCode> {
    let providers = state.providers.read().await;
    let active_id = state.active_provider_id.read().await;

    let provider = providers.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(stored_provider_to_dto(provider, active_id.as_deref())))
}

async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ProviderDto>, (StatusCode, Json<serde_json::Value>)> {
    let id = Uuid::new_v4().to_string();
    let is_active = state.active_provider_id.read().await.is_none();

    let stored = crate::api::state::StoredProvider {
        id: id.clone(),
        name: req.name,
        provider_type: req.provider_type,
        config_json: serde_json::to_value(&req.config).unwrap_or(json!(null)),
        is_active,
        created_at: Utc::now(),
    };

    let dto = stored_provider_to_dto(&stored, if is_active { Some(&id) } else { None });

    state.providers.write().await.insert(id.clone(), stored);

    if is_active {
        *state.active_provider_id.write().await = Some(id);
    }

    state.auto_save().await;

    Ok(Json(dto))
}

async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ProviderDto>, StatusCode> {
    let mut providers = state.providers.write().await;
    let stored = providers.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    stored.name = req.name;
    stored.provider_type = req.provider_type;
    stored.config_json = serde_json::to_value(&req.config).unwrap_or(json!(null));

    let active_id = state.active_provider_id.read().await;
    let dto = stored_provider_to_dto(stored, active_id.as_deref());

    drop(active_id);
    drop(providers);
    state.auto_save().await;

    Ok(Json(dto))
}

async fn delete_provider(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> StatusCode {
    state.providers.write().await.remove(&id);

    let mut active_id = state.active_provider_id.write().await;
    if active_id.as_deref() == Some(&id) {
        *active_id = None;
    }
    drop(active_id);

    state.auto_save().await;

    StatusCode::NO_CONTENT
}

async fn activate_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProviderDto>, StatusCode> {
    {
        let mut providers = state.providers.write().await;
        for p in providers.values_mut() {
            p.is_active = false;
        }
        let stored = providers.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
        stored.is_active = true;
    }

    *state.active_provider_id.write().await = Some(id.clone());

    let providers = state.providers.read().await;
    let stored = providers.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let dto = stored_provider_to_dto(stored, Some(&id));
    drop(providers);

    state.auto_save().await;

    Ok(Json(dto))
}

fn stored_provider_to_dto(
    stored: &crate::api::state::StoredProvider,
    active_id: Option<&str>,
) -> ProviderDto {
    ProviderDto {
        id: stored.id.clone(),
        name: stored.name.clone(),
        provider_type: stored.provider_type.clone(),
        config: serde_json::from_value(stored.config_json.clone()).unwrap_or(
            ProviderConfigDto::Custom(Box::new(CustomProviderConfigDto {
                base_url: String::new(),
                chat_path: String::new(),
                method: "POST".into(),
                auth_header: None,
                auth_prefix: "Bearer ".into(),
                api_key: None,
                extra_headers: Default::default(),
                request_template: None,
                response_content_path: None,
                response_tool_calls_path: None,
                response_model_path: None,
                response_finish_reason_path: None,
                default_model: String::new(),
                use_openai_format: true,
            })),
        ),
        is_active: active_id == Some(stored.id.as_str()),
        created_at: stored.created_at.to_rfc3339(),
    }
}

// ─── Skill Handlers ──────────────────────────────────────────────

async fn list_skills(State(state): State<Arc<AppState>>) -> Json<Vec<SkillDto>> {
    let skills = state.skills.read().await;
    let list: Vec<SkillDto> = skills.values().cloned().map(skill_to_dto).collect();
    Json(list)
}

async fn add_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSkillRequest>,
) -> Result<Json<SkillDto>, (StatusCode, Json<serde_json::Value>)> {
    let (name, description) = match req.skill_type.as_str() {
        "system_prompt" => (
            "system_prompt".to_string(),
            "Injects a system prompt to guide the model's behavior".to_string(),
        ),
        "memory" => (
            "memory".to_string(),
            "Manages conversation memory with a configurable message limit".to_string(),
        ),
        "context_prefix" => (
            "context_prefix".to_string(),
            "Prefixes user messages with additional context".to_string(),
        ),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Unknown skill type: {}", req.skill_type) })),
            ));
        }
    };

    let stored = crate::api::state::StoredSkill {
        name: name.clone(),
        description: description.clone(),
        skill_type: req.skill_type,
        config: req.config,
        is_active: true,
    };

    let dto = skill_to_dto(stored.clone());
    state.skills.write().await.insert(name, stored);

    state.auto_save().await;

    Ok(Json(dto))
}

async fn remove_skill(State(state): State<Arc<AppState>>, Path(name): Path<String>) -> StatusCode {
    state.skills.write().await.remove(&name);

    state.auto_save().await;

    StatusCode::NO_CONTENT
}

async fn toggle_skill(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<ToggleSkillRequest>,
) -> Result<Json<SkillDto>, StatusCode> {
    let mut skills = state.skills.write().await;
    let skill = skills.get_mut(&name).ok_or(StatusCode::NOT_FOUND)?;
    skill.is_active = req.is_active;
    let dto = skill_to_dto(skill.clone());
    drop(skills);

    state.auto_save().await;

    Ok(Json(dto))
}

fn skill_to_dto(skill: crate::api::state::StoredSkill) -> SkillDto {
    SkillDto {
        name: skill.name.clone(),
        description: skill.description.clone(),
        skill_type: skill.skill_type.clone(),
        config: skill.config.clone(),
        is_active: skill.is_active,
    }
}

// ─── Skill Package Upload ───────────────────────────────────────

/// Handles skill package upload (ZIP format)
/// Expected ZIP structure:
///   skill-package.zip
///     - manifest.json (SkillPackageManifest)
///     - (optional) other files referenced by the skill
async fn upload_skill_package(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Result<Json<UploadSkillPackageResponse>, (StatusCode, Json<serde_json::Value>)> {
    use multer::Multipart;

    // Extract Content-Type header to get boundary
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().map(|s| s.to_owned()).ok())
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Missing Content-Type header" })),
        ))?;

    println!("[DEBUG] Content-Type header: {}", content_type);

    // Extract boundary from Content-Type
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or_else(|| {
            eprintln!("[ERROR] Missing boundary in Content-Type");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Missing boundary in Content-Type" })),
            )
        })?
        .trim()
        .to_string();

    println!("[DEBUG] Extracted boundary: {}", boundary);

    // Parse multipart using multer - use Body::into_data_stream for Stream trait
    let body = request.into_body();
    let data_stream = body.into_data_stream();
    let mut multipart = Multipart::new(data_stream, boundary);

    println!("[DEBUG] Multipart parser created, starting to parse fields...");

    // Find the file field
    let mut zip_bytes: Option<Vec<u8>> = None;

    println!("[DEBUG] Starting to parse multipart fields...");

    let mut field_count = 0;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("[ERROR] Failed to parse multipart: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Failed to parse multipart: {}", e) })),
        )
    })? {
        field_count += 1;
        let name = field.name().unwrap_or("unknown").to_string();
        let content_type_field = field
            .content_type()
            .map(|mime| mime.to_string())
            .unwrap_or("unknown".to_string());
        let filename_field = field.file_name().unwrap_or("none").to_string();
        println!(
            "[DEBUG] Field {} - name: {}, content_type: {}, filename: {}",
            field_count, name, content_type_field, filename_field
        );

        if name == "file" || name == "package" {
            let buffer = field.bytes().await.map_err(|e| {
                eprintln!("[ERROR] Failed to read file bytes: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to read file: {}", e) })),
                )
            })?;
            println!("[DEBUG] File field size: {} bytes", buffer.len());
            zip_bytes = Some(buffer.to_vec());
            break;
        }
    }
    println!("[DEBUG] Total fields parsed: {}", field_count);

    let zip_bytes = zip_bytes.ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No file uploaded. Expected a field named 'file' or 'package'" })),
    ))?;

    // Parse the ZIP file
    println!("[DEBUG] Parsing ZIP file, size: {} bytes", zip_bytes.len());
    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        eprintln!("[ERROR] Failed to parse ZIP: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Invalid ZIP file: {}", e) })),
        )
    })?;
    println!("[DEBUG] ZIP parsed successfully, {} files", archive.len());

    // Find the skill directory (the first directory in the ZIP)
    println!("[DEBUG] Looking for skill directory in ZIP");
    let mut skill_dir_name: Option<String> = None;
    let mut _skill_dir_path: Option<String> = None;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| {
            eprintln!("[ERROR] Failed to access ZIP file at index {}: {}", i, e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to access ZIP file: {}", e) })),
            )
        })?;
        let name = file.name();
        println!("[DEBUG] Found file in ZIP: {}", name);

        // Find the first directory (ends with '/')
        if name.ends_with('/') && name.matches('/').count() == 1 {
            skill_dir_name = Some(name.trim_end_matches('/').to_string());
            _skill_dir_path = Some(name.to_string());
            println!(
                "[DEBUG] Found skill directory: {}",
                skill_dir_name.as_ref().unwrap()
            );
            break;
        }
    }

    let skill_dir_name = skill_dir_name.ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No skill directory found in ZIP. Expected format: skill-name.zip skill-name/SKILL.md" })),
    ))?;
    println!("[DEBUG] Skill directory confirmed: {}", skill_dir_name);

    // Read SKILL.md file
    println!("[DEBUG] Reading SKILL.md file");
    let skill_content = {
        let skill_md_path = format!("{}/SKILL.md", skill_dir_name.trim_end_matches('/'));
        println!("[DEBUG] Looking for SKILL.md at path: {}", skill_md_path);
        let mut skill_md_file = archive.by_name(&skill_md_path)
            .map_err(|e| {
                eprintln!("[ERROR] Failed to find SKILL.md in ZIP: {}, path: {}", e, skill_md_path);
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to find SKILL.md in ZIP: {} (path: {})", e, skill_md_path) })),
                )
            })?;

        let mut bytes = Vec::new();
        skill_md_file.read_to_end(&mut bytes).map_err(|e| {
            eprintln!("[ERROR] Failed to read SKILL.md: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to read SKILL.md: {}", e) })),
            )
        })?;
        let content = String::from_utf8(bytes).map_err(|e| {
            eprintln!("[ERROR] SKILL.md is not valid UTF-8: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("SKILL.md is not valid UTF-8: {}", e) })),
            )
        })?;
        println!(
            "[DEBUG] SKILL.md content read successfully, size: {} bytes",
            content.len()
        );
        content
    };

    // Parse SKILL.md
    println!("[DEBUG] Parsing SKILL.md markdown and frontmatter");
    let parsed_skill = parse_skill_markdown(&skill_content).map_err(|e| {
        eprintln!("[ERROR] Failed to parse SKILL.md: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Failed to parse SKILL.md: {}", e) })),
        )
    })?;
    println!("[DEBUG] SKILL.md parsed successfully");
    println!(
        "[DEBUG] Frontmatter: name={:?}, description={:?}",
        parsed_skill.frontmatter.name, parsed_skill.frontmatter.description
    );
    println!(
        "[DEBUG] Content length: {} chars",
        parsed_skill.content.len()
    );

    // Extract skill name and description
    let skill_name = parsed_skill
        .frontmatter
        .name
        .as_ref()
        .unwrap_or(&skill_dir_name)
        .clone();
    let skill_description = parsed_skill
        .frontmatter
        .description
        .clone()
        .unwrap_or_else(|| {
            parsed_skill
                .content
                .lines()
                .next()
                .unwrap_or("No description")
                .to_string()
        });

    // Validate skill name
    if skill_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Skill name cannot be empty" })),
        ));
    }

    // Build skill config from frontmatter and markdown content
    let mut skill_config = serde_json::Map::new();

    // Add frontmatter fields
    if let Some(name) = &parsed_skill.frontmatter.name {
        skill_config.insert("name".to_string(), serde_json::Value::String(name.clone()));
    }
    if let Some(description) = &parsed_skill.frontmatter.description {
        skill_config.insert(
            "description".to_string(),
            serde_json::Value::String(description.clone()),
        );
    }
    if let Some(when_to_use) = &parsed_skill.frontmatter.when_to_use {
        skill_config.insert(
            "when_to_use".to_string(),
            serde_json::Value::String(when_to_use.clone()),
        );
    }
    if let Some(argument_hint) = &parsed_skill.frontmatter.argument_hint {
        skill_config.insert(
            "argument_hint".to_string(),
            serde_json::Value::String(argument_hint.clone()),
        );
    }
    if let Some(arguments) = &parsed_skill.frontmatter.arguments {
        match yaml_to_json(arguments) {
            Ok(json_val) => {
                skill_config.insert("arguments".to_string(), json_val);
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to convert arguments to JSON: {}", e) })),
                ));
            }
        }
    }
    if let Some(disable_model_invocation) = parsed_skill.frontmatter.disable_model_invocation {
        skill_config.insert(
            "disable_model_invocation".to_string(),
            serde_json::Value::Bool(disable_model_invocation),
        );
    }
    if let Some(user_invocable) = parsed_skill.frontmatter.user_invocable {
        skill_config.insert(
            "user_invocable".to_string(),
            serde_json::Value::Bool(user_invocable),
        );
    }
    if let Some(allowed_tools) = &parsed_skill.frontmatter.allowed_tools {
        match yaml_to_json(allowed_tools) {
            Ok(json_val) => {
                skill_config.insert("allowed_tools".to_string(), json_val);
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(
                        json!({ "error": format!("Failed to convert allowed_tools to JSON: {}", e) }),
                    ),
                ));
            }
        }
    }
    if let Some(model) = &parsed_skill.frontmatter.model {
        skill_config.insert(
            "model".to_string(),
            serde_json::Value::String(model.clone()),
        );
    }
    if let Some(effort) = &parsed_skill.frontmatter.effort {
        skill_config.insert(
            "effort".to_string(),
            serde_json::Value::String(effort.clone()),
        );
    }
    if let Some(context) = &parsed_skill.frontmatter.context {
        skill_config.insert(
            "context".to_string(),
            serde_json::Value::String(context.clone()),
        );
    }
    if let Some(agent) = &parsed_skill.frontmatter.agent {
        skill_config.insert(
            "agent".to_string(),
            serde_json::Value::String(agent.clone()),
        );
    }
    if let Some(hooks) = &parsed_skill.frontmatter.hooks {
        match yaml_to_json(hooks) {
            Ok(json_val) => {
                skill_config.insert("hooks".to_string(), json_val);
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to convert hooks to JSON: {}", e) })),
                ));
            }
        }
    }
    if let Some(paths) = &parsed_skill.frontmatter.paths {
        match yaml_to_json(paths) {
            Ok(json_val) => {
                skill_config.insert("paths".to_string(), json_val);
            }
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to convert paths to JSON: {}", e) })),
                ));
            }
        }
    }
    if let Some(shell) = &parsed_skill.frontmatter.shell {
        skill_config.insert(
            "shell".to_string(),
            serde_json::Value::String(shell.clone()),
        );
    }

    // Add markdown content
    skill_config.insert(
        "content".to_string(),
        serde_json::Value::String(parsed_skill.content.clone()),
    );

    // Add version (default to "1.0.0")
    skill_config.insert(
        "_version".to_string(),
        serde_json::Value::String("1.0.0".to_string()),
    );

    // Store the skill
    let stored = crate::api::state::StoredSkill {
        name: skill_name.clone(),
        description: skill_description.clone(),
        skill_type: "skill".to_string(),
        config: serde_json::Value::Object(skill_config),
        is_active: true,
    };

    let skill_dto = skill_to_dto(stored.clone());

    // Check if skill already exists and update/overwrite
    state
        .skills
        .write()
        .await
        .insert(skill_name.clone(), stored);

    state.auto_save().await;

    // Create the parsed skill response
    let parsed = ParsedSkill {
        name: skill_name.clone(),
        description: skill_description.clone(),
        skill_type: "skill".to_string(),
        config: skill_dto.config.clone(),
        version: "1.0.0".to_string(),
        author: None,
    };

    Ok(Json(UploadSkillPackageResponse {
        skill: skill_dto,
        parsed,
    }))
}

// ─── Tool Handlers ───────────────────────────────────────────────

async fn list_tools(State(state): State<Arc<AppState>>) -> Json<Vec<ToolDto>> {
    let list: Vec<ToolDto> = state.tool_definitions.iter().map(ToolDto::from).collect();
    Json(list)
}

// ─── Chat Handlers ───────────────────────────────────────────────

async fn send_chat_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequestDto>,
) -> Result<Json<ChatResponseDto>, (StatusCode, Json<serde_json::Value>)> {
    // Build agent from current state
    let agent_result = state.build_agent().await;
    let mut agent =
        agent_result.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))))?;

    // Send message
    let response = agent.chat(&req.message).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    // Store in history
    let user_msg = crate::types::ChatMessage::user(&req.message);
    state.chat_history.write().await.push(user_msg);

    // Get the assistant message from the response
    let choice = &response.choices[0];
    state
        .chat_history
        .write()
        .await
        .push(choice.message.clone());

    // Persist chat history
    if let Err(e) = state.save_chat_history().await {
        tracing::warn!("Failed to save chat history after message: {}", e);
    }

    // Convert response
    let message_dto = ChatMessageDto::from(&choice.message);

    let tool_results_dto = if !choice
        .message
        .tool_calls
        .as_ref()
        .is_none_or(|c| c.is_empty())
    {
        // Collect tool results from history that were just added
        None // Tool results are embedded in the conversation flow
    } else {
        None
    };

    let usage_dto = response.usage.map(|u| UsageDto {
        prompt_tokens: u.prompt_tokens.unwrap_or(0),
        completion_tokens: u.completion_tokens.unwrap_or(0),
    });

    Ok(Json(ChatResponseDto {
        message: message_dto,
        tool_results: tool_results_dto,
        usage: usage_dto,
    }))
}

async fn get_chat_history(State(state): State<Arc<AppState>>) -> Json<Vec<ChatMessageDto>> {
    let history = state.chat_history.read().await;
    let list: Vec<ChatMessageDto> = history.iter().map(ChatMessageDto::from).collect();
    Json(list)
}

async fn clear_chat_history(State(state): State<Arc<AppState>>) -> StatusCode {
    state.chat_history.write().await.clear();
    if let Err(e) = state.save_chat_history().await {
        tracing::warn!("Failed to save chat history after clear: {}", e);
    }
    StatusCode::NO_CONTENT
}

// ─── Agent Status Handler ────────────────────────────────────────

async fn get_status(State(state): State<Arc<AppState>>) -> Json<AgentStatusDto> {
    let providers = state.providers.read().await;
    let active_id = state.active_provider_id.read().await;
    let skills = state.skills.read().await;
    let history = state.chat_history.read().await;

    let (active_provider, active_model) = if let Some(ref id) = *active_id {
        if let Some(p) = providers.get(id) {
            let model = match p.provider_type.as_str() {
                "openai" => p.config_json["default_model"].as_str().unwrap_or("gpt-4o"),
                "anthropic" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("claude-sonnet-4-20250514"),
                "custom" => p.config_json["default_model"].as_str().unwrap_or("default"),
                _ => "unknown",
            };
            (Some(p.name.clone()), Some(model.to_string()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let uptime = Utc::now()
        .signed_duration_since(state.start_time)
        .num_seconds()
        .max(0) as u64;

    Json(AgentStatusDto {
        status: if active_id.is_some() {
            "running".to_string()
        } else {
            "stopped".to_string()
        },
        active_provider,
        active_model,
        skills_count: skills.len(),
        tools_count: state.tool_definitions.len(),
        uptime_secs: uptime,
        message_count: history.len(),
    })
}

// ─── ACP Config Handlers ─────────────────────────────────────────

async fn get_acp_config(State(state): State<Arc<AppState>>) -> Json<AcpConfigDto> {
    let acp_config = state.acp_config.read().await;
    let providers = state.providers.read().await;
    let skills = state.skills.read().await;

    let available_providers: Vec<AcpProviderOptionDto> = providers
        .values()
        .map(|p| {
            let default_model = match p.provider_type.as_str() {
                "openai" => p.config_json["default_model"].as_str().unwrap_or("gpt-4o"),
                "anthropic" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("claude-sonnet-4-20250514"),
                _ => p.config_json["default_model"].as_str().unwrap_or("default"),
            };
            AcpProviderOptionDto {
                id: p.id.clone(),
                name: p.name.clone(),
                provider_type: p.provider_type.clone(),
                default_model: default_model.to_string(),
            }
        })
        .collect();

    let available_skills: Vec<AcpSkillOptionDto> = skills
        .values()
        .map(|s| AcpSkillOptionDto {
            name: s.name.clone(),
            description: s.description.clone(),
            is_active: acp_config.active_skill_names.contains(&s.name),
        })
        .collect();

    Json(AcpConfigDto {
        active_provider_id: acp_config.active_provider_id.clone(),
        active_skill_names: acp_config.active_skill_names.clone(),
        available_providers,
        available_skills,
    })
}

async fn update_acp_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateAcpConfigRequest>,
) -> Result<Json<AcpConfigDto>, (StatusCode, Json<serde_json::Value>)> {
    let mut acp_config = state.acp_config.write().await;

    if let Some(active_provider_id) = req.active_provider_id {
        // Validate that the provider exists if a non-empty ID is given
        if !active_provider_id.is_empty() {
            let providers = state.providers.read().await;
            if !providers.contains_key(&active_provider_id) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Provider '{}' not found", active_provider_id)
                    })),
                ));
            }
        }
        acp_config.active_provider_id = if active_provider_id.is_empty() {
            None
        } else {
            Some(active_provider_id)
        };
    }

    if let Some(active_skill_names) = req.active_skill_names {
        acp_config.active_skill_names = active_skill_names;
    }

    let providers = state.providers.read().await;
    let skills = state.skills.read().await;

    let available_providers: Vec<AcpProviderOptionDto> = providers
        .values()
        .map(|p| {
            let default_model = match p.provider_type.as_str() {
                "openai" => p.config_json["default_model"].as_str().unwrap_or("gpt-4o"),
                "anthropic" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("claude-sonnet-4-20250514"),
                _ => p.config_json["default_model"].as_str().unwrap_or("default"),
            };
            AcpProviderOptionDto {
                id: p.id.clone(),
                name: p.name.clone(),
                provider_type: p.provider_type.clone(),
                default_model: default_model.to_string(),
            }
        })
        .collect();

    let available_skills: Vec<AcpSkillOptionDto> = skills
        .values()
        .map(|s| AcpSkillOptionDto {
            name: s.name.clone(),
            description: s.description.clone(),
            is_active: acp_config.active_skill_names.contains(&s.name),
        })
        .collect();

    let dto = AcpConfigDto {
        active_provider_id: acp_config.active_provider_id.clone(),
        active_skill_names: acp_config.active_skill_names.clone(),
        available_providers,
        available_skills,
    };
    drop(acp_config);

    state.auto_save().await;

    Ok(Json(dto))
}
