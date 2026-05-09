use axum::extract::ws::Utf8Bytes;
use axum::{
    Json, Router,
    extract::{
        Path, Request, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use tracing::{debug, error};
// futures::sink::SinkExt is not needed for axum WebSocket
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{Cursor, Read};
use std::sync::Arc;
use uuid::Uuid;
use zip::ZipArchive;

use crate::api::models::*;
use crate::api::state::{AppState, StoredConfigProfile};
use crate::mcp::types::McpServerConfig;

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
        // Personas
        .route("/api/personas", get(list_personas).post(create_persona))
        .route(
            "/api/personas/{id}",
            get(get_persona).put(update_persona).delete(delete_persona),
        )
        // Config profiles
        .route(
            "/api/config-profiles",
            get(list_config_profiles).post(create_config_profile),
        )
        .route(
            "/api/config-profiles/{id}",
            get(get_config_profile)
                .put(update_config_profile)
                .delete(delete_config_profile),
        )
        .route(
            "/api/config-profiles/{id}/activate",
            post(activate_config_profile),
        )
        .route(
            "/api/config-profiles/{id}/provider",
            get(get_config_profile_provider),
        )
        .route(
            "/api/config-profiles/{id}/persona",
            get(get_config_profile_persona),
        )
        // ACP config
        .route(
            "/api/acp/config",
            get(get_acp_config).put(update_acp_config),
        )
        // Computer use config
        .route(
            "/api/computer-use/config",
            get(get_computer_use_config).put(update_computer_use_config),
        )
        // Web search config
        .route(
            "/api/web-search/config",
            get(get_web_search_config).put(update_web_search_config),
        )
        // Logs
        .route("/api/logs", get(get_logs).delete(clear_logs))
        .route("/api/logs/stream", get(ws_logs_upgrade))
        // Conversations
        .route(
            "/api/conversations",
            get(list_conversations).post(create_conversation),
        )
        .route(
            "/api/conversations/{id}",
            get(get_conversation).delete(delete_conversation),
        )
        .route(
            "/api/conversations/{id}/messages",
            post(add_message).get(get_conversation_messages),
        )
        // MCP servers
        .route(
            "/api/mcp/servers",
            get(list_mcp_servers).post(create_mcp_server),
        )
        .route(
            "/api/mcp/servers/{id}",
            get(get_mcp_server)
                .put(update_mcp_server)
                .delete(delete_mcp_server),
        )
        .route("/api/mcp/servers/{id}/toggle", post(toggle_mcp_server))
        // Platforms
        .route("/api/platforms", get(list_platforms).post(create_platform))
        .route(
            "/api/platforms/{id}",
            get(get_platform)
                .put(update_platform)
                .delete(delete_platform),
        )
        .route("/api/platforms/{id}/restart", post(restart_platform))
        // System
        .route("/api/system/restart", post(restart_system))
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

    debug!(content_type = %content_type, "Processing skill package upload");

    // Extract boundary from Content-Type
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or_else(|| {
            error!("Missing boundary in Content-Type");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Missing boundary in Content-Type" })),
            )
        })?
        .trim()
        .to_string();

    debug!(boundary = %boundary, "Extracted multipart boundary");

    // Parse multipart using multer - use Body::into_data_stream for Stream trait
    let body = request.into_body();
    let data_stream = body.into_data_stream();
    let mut multipart = Multipart::new(data_stream, boundary);

    debug!("Multipart parser created, starting to parse fields");

    // Find the file field
    let mut zip_bytes: Option<Vec<u8>> = None;

    let mut field_count = 0;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!(error = %e, "Failed to parse multipart");
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
        debug!(
            field_count, name = %name, content_type = %content_type_field, filename = %filename_field,
            "Parsed multipart field"
        );

        if name == "file" || name == "package" {
            let buffer = field.bytes().await.map_err(|e| {
                error!(error = %e, "Failed to read file bytes");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to read file: {}", e) })),
                )
            })?;
            debug!(size = buffer.len(), "Read file field bytes");
            zip_bytes = Some(buffer.to_vec());
            break;
        }
    }
    debug!(
        total_fields = field_count,
        "Finished parsing multipart fields"
    );

    let zip_bytes = zip_bytes.ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No file uploaded. Expected a field named 'file' or 'package'" })),
    ))?;

    // Parse the ZIP file
    debug!(size = zip_bytes.len(), "Parsing ZIP file");
    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        error!(error = %e, "Failed to parse ZIP file");
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Invalid ZIP file: {}", e) })),
        )
    })?;
    debug!(file_count = archive.len(), "ZIP parsed successfully");

    // Find the skill directory (the first directory in the ZIP)
    debug!("Looking for skill directory in ZIP");
    let mut skill_dir_name: Option<String> = None;
    let mut _skill_dir_path: Option<String> = None;

    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| {
            error!(index = i, error = %e, "Failed to access ZIP file at index");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to access ZIP file: {}", e) })),
            )
        })?;
        let name = file.name();
        debug!(file_name = %name, "Found file in ZIP");

        // Find the first directory (ends with '/')
        if name.ends_with('/') && name.matches('/').count() == 1 {
            skill_dir_name = Some(name.trim_end_matches('/').to_string());
            _skill_dir_path = Some(name.to_string());
            debug!(
                skill_dir = skill_dir_name.as_ref().unwrap(),
                "Found skill directory"
            );
            break;
        }
    }

    let skill_dir_name = skill_dir_name.ok_or((
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "No skill directory found in ZIP. Expected format: skill-name.zip containing skill-name/SKILL.md" })),
    ))?;
    debug!(skill_dir = %skill_dir_name, "Skill directory confirmed");

    // Read SKILL.md file
    debug!("Reading SKILL.md file");
    let skill_content = {
        let skill_md_path = format!("{}/SKILL.md", skill_dir_name.trim_end_matches('/'));
        debug!(path = %skill_md_path, "Looking for SKILL.md");
        let mut skill_md_file = archive.by_name(&skill_md_path)
            .map_err(|e| {
                error!(error = %e, path = %skill_md_path, "Failed to find SKILL.md in ZIP");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Failed to find SKILL.md in ZIP: {} (path: {})", e, skill_md_path) })),
                )
            })?;

        let mut bytes = Vec::new();
        skill_md_file.read_to_end(&mut bytes).map_err(|e| {
            error!(error = %e, "Failed to read SKILL.md");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to read SKILL.md: {}", e) })),
            )
        })?;
        let content = String::from_utf8(bytes).map_err(|e| {
            error!(error = %e, "SKILL.md is not valid UTF-8");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("SKILL.md is not valid UTF-8: {}", e) })),
            )
        })?;
        debug!(size = content.len(), "SKILL.md content read successfully");
        content
    };

    // Parse SKILL.md
    debug!("Parsing SKILL.md markdown and frontmatter");
    let parsed_skill = parse_skill_markdown(&skill_content).map_err(|e| {
        error!(error = %e, "Failed to parse SKILL.md");
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Failed to parse SKILL.md: {}", e) })),
        )
    })?;
    debug!(
        name = ?parsed_skill.frontmatter.name,
        description = ?parsed_skill.frontmatter.description,
        content_len = parsed_skill.content.len(),
        "SKILL.md parsed successfully"
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
    // Build agent from current state with user context
    let agent_result = state
        .build_agent_with_context(
            req.user_id.as_deref(),
            req.session_id.as_deref(),
            req.persona_id.as_deref(),
        )
        .await;
    let mut agent =
        agent_result.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))))?;

    // Send message
    let response = agent.chat(&req.message).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    // Ensure we have an active conversation
    let conversation_id = state.ensure_chat_conversation().await.map_err(|e| {
        tracing::error!("Failed to ensure chat conversation: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to initialize conversation: {}", e) })),
        )
    })?;

    // Add user message to conversation database
    let conv_db = state.conversation_db.read().await;
    if let Some(db) = conv_db.as_ref() {
        if let Err(e) = db
            .add_message(crate::conversation::models::AddMessageRequest {
                conversation_id: conversation_id.clone(),
                role: "user".to_string(),
                content: req.message.clone(),
            })
            .await
        {
            tracing::error!("Failed to add user message to database: {}", e);
        }
    }
    drop(conv_db);

    // Get the assistant message from the response
    let choice = &response.choices[0];
    let assistant_content = choice
        .message
        .content
        .as_ref()
        .and_then(|c| c.as_text())
        .unwrap_or("")
        .to_string();

    // Add assistant message to conversation database
    let conv_db = state.conversation_db.read().await;
    if let Some(db) = conv_db.as_ref() {
        if let Err(e) = db
            .add_message(crate::conversation::models::AddMessageRequest {
                conversation_id: conversation_id.clone(),
                role: "assistant".to_string(),
                content: assistant_content,
            })
            .await
        {
            tracing::error!("Failed to add assistant message to database: {}", e);
        }
    }
    drop(conv_db);

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
    // Ensure we have an active conversation
    let conversation_id = match state.ensure_chat_conversation().await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to ensure chat conversation: {}", e);
            return Json(Vec::new());
        }
    };

    // Get messages from database
    let conv_db = state.conversation_db.read().await;
    if let Some(db) = conv_db.as_ref() {
        match db.get_conversation_messages(&conversation_id).await {
            Ok(messages) => {
                // Convert database messages to DTOs
                let list: Vec<ChatMessageDto> = messages
                    .iter()
                    .map(|m| ChatMessageDto {
                        role: m.role.clone(),
                        content: m.content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    })
                    .collect();
                return Json(list);
            }
            Err(e) => {
                tracing::error!("Failed to get conversation messages from database: {}", e);
            }
        }
    }

    Json(Vec::new())
}

async fn clear_chat_history(State(state): State<Arc<AppState>>) -> StatusCode {
    // Clear chat history by deleting the current conversation from the database
    let conversation_id = match state.chat_conversation_id.read().await.clone() {
        Some(id) => id,
        None => {
            tracing::debug!("No active conversation to clear");
            return StatusCode::NO_CONTENT;
        }
    };

    let conv_db = state.conversation_db.read().await;
    if let Some(db) = conv_db.as_ref() {
        if let Err(e) = db.delete_conversation(&conversation_id).await {
            tracing::warn!("Failed to delete conversation from database: {}", e);
        } else {
            tracing::info!(
                "Cleared chat history by deleting conversation: {}",
                conversation_id
            );
            // Reset the active conversation ID so a new one will be created on next chat
            let mut conv_id = state.chat_conversation_id.write().await;
            *conv_id = None;
        }
    } else {
        tracing::warn!("Conversation database not initialized, cannot clear history");
    }
    StatusCode::NO_CONTENT
}

// ─── Helper: Get message count from database ─────────────────────

async fn get_message_count_from_db(state: &AppState) -> usize {
    let conversation_id = match state.chat_conversation_id.read().await.clone() {
        Some(id) => id,
        None => return 0,
    };

    let conv_db = state.conversation_db.read().await;
    if let Some(db) = conv_db.as_ref() {
        if let Ok(messages) = db.get_conversation_messages(&conversation_id).await {
            return messages.len();
        }
    }
    0
}

// ─── Agent Status Handler ────────────────────────────────────────

async fn get_status(State(state): State<Arc<AppState>>) -> Json<AgentStatusDto> {
    let providers = state.providers.read().await;
    let active_id = state.active_provider_id.read().await;
    let skills = state.skills.read().await;

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
        message_count: get_message_count_from_db(&state).await,
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

// ─── Logs ─────────────────────────────────────────────────────────

/// 获取所有日志
async fn get_logs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let logs = state.log_manager.get_logs().await;
    Json(serde_json::to_value(logs).unwrap_or(json!([])))
}

/// 清空所有日志
async fn clear_logs(State(state): State<Arc<AppState>>) -> StatusCode {
    state.log_manager.clear_logs().await;
    StatusCode::NO_CONTENT
}

/// WebSocket日志推送handler
async fn ws_logs_upgrade(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| ws_logs_handler(socket, state))
}

// ─── Computer Use Config Handlers ───────────────────────────────

async fn get_computer_use_config(State(state): State<Arc<AppState>>) -> Json<ComputerUseConfigDto> {
    let computer_use_config = state.computer_use_config.read().await;
    Json(ComputerUseConfigDto::from(&*computer_use_config))
}

async fn update_computer_use_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateComputerUseConfigRequest>,
) -> Result<Json<ComputerUseConfigDto>, (StatusCode, Json<serde_json::Value>)> {
    let mut computer_use_config = state.computer_use_config.write().await;

    // Update runtime if provided
    if let Some(runtime) = req.runtime {
        computer_use_config.runtime = match runtime.as_str() {
            "none" => crate::computer_use::ComputerUseRuntime::None,
            "local" => crate::computer_use::ComputerUseRuntime::Local,
            "sandbox" => crate::computer_use::ComputerUseRuntime::Sandbox,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Invalid runtime: {}. Must be 'none', 'local', or 'sandbox'", runtime)
                    })),
                ));
            }
        };
    }

    // Update require_admin if provided
    if let Some(require_admin) = req.require_admin {
        computer_use_config.require_admin = require_admin;
    }

    // Update admin_ids if provided
    if let Some(admin_ids) = req.admin_ids {
        computer_use_config.admin_ids = admin_ids;
    }

    // Update allowed_paths if provided
    if let Some(allowed_paths) = req.allowed_paths {
        computer_use_config.allowed_paths = allowed_paths;
    }

    // Update sandbox_config if provided
    if let Some(sandbox_config_dto) = req.sandbox_config {
        computer_use_config.sandbox_config = Some(crate::computer_use::SandboxConfig {
            driver: sandbox_config_dto.driver,
            endpoint: sandbox_config_dto.endpoint,
            profile: sandbox_config_dto.profile,
            ttl_secs: sandbox_config_dto.ttl_secs,
            enable_browser: sandbox_config_dto.enable_browser,
        });
    }

    let dto = ComputerUseConfigDto::from(&*computer_use_config);
    drop(computer_use_config);

    state.auto_save().await;

    Ok(Json(dto))
}

// ─── Web Search Config Handlers ─────────────────────────────────────

async fn get_web_search_config(State(state): State<Arc<AppState>>) -> Json<WebSearchConfigDto> {
    let config = state.web_search_config.read().await;
    Json(WebSearchConfigDto::from(&*config))
}

async fn update_web_search_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateWebSearchConfigRequest>,
) -> Result<Json<WebSearchConfigDto>, (StatusCode, Json<serde_json::Value>)> {
    let mut config = state.web_search_config.write().await;

    // Update search_engine if provided
    if let Some(search_engine) = req.search_engine {
        config.search_engine = match search_engine.as_str() {
            "duckduckgo" => crate::types::SearchEngine::DuckDuckGo,
            "tavily" => crate::types::SearchEngine::Tavily,
            "bocha" => crate::types::SearchEngine::BoCha,
            "baidu" => crate::types::SearchEngine::Baidu,
            "brave" => crate::types::SearchEngine::Brave,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Invalid search engine: {}. Must be 'duckduckgo', 'tavily', 'bocha', 'baidu', or 'brave'", search_engine)
                    })),
                ));
            }
        };
    }

    // Update api_key if provided
    if let Some(api_key) = req.api_key {
        config.api_key = api_key;
    }

    // Update max_results if provided
    if let Some(max_results) = req.max_results {
        config.max_results = max_results;
    }

    // Update enabled if provided
    if let Some(enabled) = req.enabled {
        config.enabled = enabled;
    }

    let dto = WebSearchConfigDto::from(&*config);
    drop(config);

    state.auto_save().await;

    Ok(Json(dto))
}

// ─── Persona Handlers ─────────────────────────────────────────────

/// List all personas.
async fn list_personas(State(state): State<Arc<AppState>>) -> Json<Vec<PersonaDto>> {
    let personas = state.personas.read().await;
    let list: Vec<PersonaDto> = personas
        .values()
        .map(|p| PersonaDto {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            prompt: p.prompt.clone(),
        })
        .collect();
    Json(list)
}

/// Get a specific persona by ID.
async fn get_persona(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PersonaDto>, StatusCode> {
    let personas = state.personas.read().await;
    match personas.get(&id) {
        Some(p) => Ok(Json(PersonaDto {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            prompt: p.prompt.clone(),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Create a new persona with an auto-generated UUID.
async fn create_persona(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePersonaRequest>,
) -> Result<Json<PersonaDto>, (StatusCode, Json<serde_json::Value>)> {
    // Validate that prompt is not empty
    if req.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Persona prompt cannot be empty" })),
        ));
    }

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Persona name cannot be empty" })),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();

    let stored = crate::api::state::StoredPersona {
        id: id.clone(),
        name: req.name.clone(),
        description: req.description.clone(),
        prompt: req.prompt.clone(),
    };

    {
        let mut personas = state.personas.write().await;
        personas.insert(id.clone(), stored);
    }

    tracing::info!(
        persona_id = %id,
        persona_name = %req.name,
        "Persona created"
    );

    state.auto_save().await;

    Ok(Json(PersonaDto {
        id,
        name: req.name,
        description: req.description,
        prompt: req.prompt,
    }))
}

/// Update an existing persona by ID.
async fn update_persona(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePersonaRequest>,
) -> Result<Json<PersonaDto>, (StatusCode, Json<serde_json::Value>)> {
    let mut personas = state.personas.write().await;

    let persona = personas.get_mut(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Persona not found" })),
        )
    })?;

    // Update fields if provided
    if let Some(name) = &req.name {
        if name.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Persona name cannot be empty" })),
            ));
        }
        persona.name = name.clone();
    }

    if let Some(description) = &req.description {
        persona.description = description.clone();
    }

    if let Some(prompt) = &req.prompt {
        if prompt.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Persona prompt cannot be empty" })),
            ));
        }
        persona.prompt = prompt.clone();
    }

    let dto = PersonaDto {
        id: persona.id.clone(),
        name: persona.name.clone(),
        description: persona.description.clone(),
        prompt: persona.prompt.clone(),
    };

    tracing::info!(persona_id = %id, "Persona updated");

    drop(personas);
    state.auto_save().await;

    Ok(Json(dto))
}

/// Delete a persona by ID.
async fn delete_persona(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let mut personas = state.personas.write().await;
    if personas.remove(&id).is_some() {
        tracing::info!(persona_id = %id, "Persona deleted");
        drop(personas);
        state.auto_save().await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Persona not found" })),
        ))
    }
}

// ─── Config Profile Handlers ─────────────────────────────────────

/// List all config profiles
async fn list_config_profiles(State(state): State<Arc<AppState>>) -> Json<Vec<ConfigProfileDto>> {
    let profiles = state.config_profiles.read().await;
    let dtos: Vec<ConfigProfileDto> = profiles
        .values()
        .map(|p| ConfigProfileDto {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            enable: p.enable,
            is_active: p.is_active,
            created_at: p.created_at.to_rfc3339().to_string(),
            updated_at: p.updated_at.to_rfc3339().to_string(),
            provider_id: p.provider_id.clone(),
            persona_id: p.persona_id.clone(),
            web_search_enabled: p.web_search_enabled,
            computer_use_enabled: p.computer_use_enabled,
            acp_enabled: p.acp_enabled,
            active_skill_names: p.active_skill_names.clone(),
            active_platform_ids: p.active_platform_ids.clone(),
            proxy_config: p.proxy_config.clone(),
        })
        .collect();
    Json(dtos)
}

/// Get a specific config profile by ID
async fn get_config_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigProfileDto>, (StatusCode, Json<Value>)> {
    let profiles = state.config_profiles.read().await;
    if let Some(p) = profiles.get(&id) {
        let dto = ConfigProfileDto {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            enable: p.enable,
            is_active: p.is_active,
            created_at: p.created_at.to_rfc3339().to_string(),
            updated_at: p.updated_at.to_rfc3339().to_string(),
            provider_id: p.provider_id.clone(),
            persona_id: p.persona_id.clone(),
            web_search_enabled: p.web_search_enabled,
            computer_use_enabled: p.computer_use_enabled,
            acp_enabled: p.acp_enabled,
            active_skill_names: p.active_skill_names.clone(),
            active_platform_ids: p.active_platform_ids.clone(),
            proxy_config: p.proxy_config.clone(),
        };
        Ok(Json(dto))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Config profile not found" })),
        ))
    }
}

/// Create a new config profile
async fn create_config_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConfigProfileRequest>,
) -> Result<Json<ConfigProfileDto>, (StatusCode, Json<Value>)> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    // Check if any active profile exists
    let profiles = state.config_profiles.read().await;
    let has_active = profiles.values().any(|p| p.is_active && p.enable);
    drop(profiles);

    // Create the profile - make it active if no other profile is active
    let is_active = !has_active;
    let profile = StoredConfigProfile {
        id: id.clone(),
        name: req.name.clone(),
        description: req.description.clone(),
        enable: req.enable,
        is_active,
        created_at: now,
        updated_at: now,
        provider_id: req.provider_id.clone(),
        persona_id: req.persona_id.clone(),
        web_search_enabled: req.web_search_enabled,
        computer_use_enabled: req.computer_use_enabled,
        acp_enabled: req.acp_enabled,
        active_skill_names: req.active_skill_names.clone(),
        active_platform_ids: req.active_platform_ids.clone(),
        proxy_config: req.proxy_config.clone(),
    };

    // Insert the profile
    let mut profiles = state.config_profiles.write().await;
    profiles.insert(id.clone(), profile);
    drop(profiles);

    // Build response directly
    let dto = ConfigProfileDto {
        id: id.clone(),
        name: req.name,
        description: req.description,
        enable: req.enable,
        is_active,
        created_at: now.to_rfc3339().to_string(),
        updated_at: now.to_rfc3339().to_string(),
        provider_id: req.provider_id,
        persona_id: req.persona_id,
        web_search_enabled: req.web_search_enabled,
        computer_use_enabled: req.computer_use_enabled,
        acp_enabled: req.acp_enabled,
        active_skill_names: req.active_skill_names,
        active_platform_ids: req.active_platform_ids,
        proxy_config: req.proxy_config,
    };

    state.auto_save().await;

    tracing::info!(profile_id = %id, profile_name = %dto.name, is_active = %is_active, "Config profile created");

    Ok(Json(dto))
}

/// Update an existing config profile
async fn update_config_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConfigProfileRequest>,
) -> Result<Json<ConfigProfileDto>, (StatusCode, Json<Value>)> {
    let mut profiles = state.config_profiles.write().await;

    if let Some(profile) = profiles.get_mut(&id) {
        // Update fields
        if let Some(name) = req.name {
            profile.name = name;
        }
        if let Some(description) = req.description {
            profile.description = description;
        }
        if let Some(enable) = req.enable {
            profile.enable = enable;
        }
        if let Some(provider_id) = req.provider_id {
            profile.provider_id = provider_id;
        }
        if let Some(persona_id) = req.persona_id {
            profile.persona_id = persona_id;
        }
        if let Some(web_search_enabled) = req.web_search_enabled {
            profile.web_search_enabled = web_search_enabled;
        }
        if let Some(computer_use_enabled) = req.computer_use_enabled {
            profile.computer_use_enabled = computer_use_enabled;
        }
        if let Some(acp_enabled) = req.acp_enabled {
            profile.acp_enabled = acp_enabled;
        }
        if let Some(active_skill_names) = req.active_skill_names {
            profile.active_skill_names = active_skill_names;
        }
        if let Some(ref active_platform_ids) = req.active_platform_ids {
            profile.active_platform_ids = active_platform_ids.clone();
        }
        let platforms_changed = req.active_platform_ids.is_some();
        let enable_changed = req.enable.is_some();
        let proxy_changed = req.proxy_config.is_some();
        if let Some(proxy_config) = req.proxy_config {
            profile.proxy_config = proxy_config;
        }
        profile.updated_at = Utc::now();

        let dto = ConfigProfileDto {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            enable: profile.enable,
            is_active: profile.is_active,
            created_at: profile.created_at.to_rfc3339().to_string(),
            updated_at: profile.updated_at.to_rfc3339().to_string(),
            provider_id: profile.provider_id.clone(),
            persona_id: profile.persona_id.clone(),
            web_search_enabled: profile.web_search_enabled,
            computer_use_enabled: profile.computer_use_enabled,
            acp_enabled: profile.acp_enabled,
            active_skill_names: profile.active_skill_names.clone(),
            active_platform_ids: profile.active_platform_ids.clone(),
            proxy_config: profile.proxy_config.clone(),
        };

        let is_active = dto.is_active;
        drop(profiles);
        state.auto_save().await;

        // If this is the active profile and something that affects running
        // adapters changed, synchronize accordingly (hot-reload).
        if is_active && (platforms_changed || proxy_changed || enable_changed) {
            state.sync_platforms_with_active_profile().await;
        }

        tracing::info!(profile_id = %id, profile_name = %dto.name, "Config profile updated");

        Ok(Json(dto))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Config profile not found" })),
        ))
    }
}

/// Delete a config profile
async fn delete_config_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut profiles = state.config_profiles.write().await;

    if let Some(profile) = profiles.remove(&id) {
        // If we deleted an active profile, activate another one if available
        if profile.is_active {
            if let Some(first) = profiles.values_mut().next() {
                first.is_active = true;
            }
        }

        drop(profiles);
        state.auto_save().await;

        tracing::info!(profile_id = %id, profile_name = %profile.name, "Config profile deleted");

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Config profile not found" })),
        ))
    }
}

/// Activate a specific config profile
/// This also starts/stops platform adapters based on the profile's active_platform_ids.
async fn activate_config_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigProfileDto>, (StatusCode, Json<Value>)> {
    let mut profiles = state.config_profiles.write().await;

    if let Some(profile) = profiles.get(&id) {
        // Check if the profile is enabled before activating
        if !profile.enable {
            drop(profiles);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error": "Cannot activate a disabled config profile. Enable it first."}),
                ),
            ));
        }
    }

    if profiles.contains_key(&id) {
        let now = Utc::now();
        // Set all profiles to inactive, then activate the target profile and update its timestamp
        for p in profiles.values_mut() {
            let is_target = p.id == id;
            p.is_active = is_target;
            if is_target {
                p.updated_at = now;
            }
        }

        let profile = profiles.get(&id).unwrap();
        let active_platform_ids = profile.active_platform_ids.clone();

        let dto = ConfigProfileDto {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            enable: profile.enable,
            is_active: profile.is_active,
            created_at: profile.created_at.to_rfc3339().to_string(),
            updated_at: profile.updated_at.to_rfc3339().to_string(),
            provider_id: profile.provider_id.clone(),
            persona_id: profile.persona_id.clone(),
            web_search_enabled: profile.web_search_enabled,
            computer_use_enabled: profile.computer_use_enabled,
            acp_enabled: profile.acp_enabled,
            active_skill_names: profile.active_skill_names.clone(),
            active_platform_ids: active_platform_ids.clone(),
            proxy_config: profile.proxy_config.clone(),
        };

        drop(profiles);
        state.auto_save().await;

        tracing::info!(profile_id = %id, profile_name = %dto.name, "Config profile activated");

        // Start/stop platform adapters based on the active profile's platform list
        activate_platforms_for_profile(&state, &active_platform_ids).await;

        Ok(Json(dto))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Config profile not found" })),
        ))
    }
}

/// Start platforms that should be active and stop platforms that should not.
/// This is called when a config profile is activated.
async fn activate_platforms_for_profile(state: &Arc<AppState>, _active_platform_ids: &[String]) {
    // Delegate to the unified sync method which reads the active profile
    // and reconciles running adapters accordingly.
    state.sync_platforms_with_active_profile().await;
}

/// Get the provider associated with a config profile
async fn get_config_profile_provider(
    State(state): State<Arc<AppState>>,
    Path(profile_id): Path<String>,
) -> Json<ConfigProfileProviderResponse> {
    let profiles = state.config_profiles.read().await;
    let providers = state.providers.read().await;
    let active_provider_id = state.active_provider_id.read().await;

    let provider = profiles
        .get(&profile_id)
        .and_then(|p| p.provider_id.as_ref())
        .and_then(|provider_id| providers.get(provider_id))
        .map(|p| stored_provider_to_dto(p, active_provider_id.as_deref()));

    Json(ConfigProfileProviderResponse { provider })
}

/// Get the persona associated with a config profile
async fn get_config_profile_persona(
    State(state): State<Arc<AppState>>,
    Path(profile_id): Path<String>,
) -> Json<ConfigProfilePersonaResponse> {
    let profiles = state.config_profiles.read().await;
    let personas = state.personas.read().await;

    let persona = profiles
        .get(&profile_id)
        .and_then(|p| p.persona_id.as_ref())
        .and_then(|persona_id| personas.get(persona_id))
        .map(|p| PersonaDto {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            prompt: p.prompt.clone(),
        });

    Json(ConfigProfilePersonaResponse { persona })
}

/// WebSocket日志推送处理器
async fn ws_logs_handler(mut socket: WebSocket, state: Arc<AppState>) {
    // 订阅日志广播
    let mut log_rx = state.log_manager.subscribe();

    // 注意：历史日志由前端通过 HTTP API 获取，WebSocket 只推送新日志
    // 这样避免历史日志重复发送

    // 持续接收新日志并推送
    while let Ok(log) = log_rx.recv().await {
        if let Ok(log_json) = serde_json::to_string(&log) {
            if socket
                .send(axum::extract::ws::Message::Text(Utf8Bytes::from(log_json)))
                .await
                .is_err()
            {
                break;
            }
        }
    }
}

// ─── Conversation Handlers ──────────────────────────────────────

/// List all conversations with optional filters
async fn list_conversations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<ListConversationsRequest>,
) -> Result<Json<Vec<ConversationDto>>, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    let chat_type = if let Some(ref ct) = query.chat_type {
        match ct.to_lowercase().as_str() {
            "group" => Some(crate::conversation::models::ChatType::Group),
            "private" => Some(crate::conversation::models::ChatType::Private),
            _ => None,
        }
    } else {
        None
    };

    let filter = if query.bot_name.is_some() || query.chat_type.is_some() || query.keyword.is_some()
    {
        Some(crate::conversation::models::ConversationFilter {
            bot_name: query.bot_name.clone(),
            chat_type,
            keyword: query.keyword.clone(),
        })
    } else {
        None
    };

    match db.list_conversations(filter).await {
        Ok(conversations) => {
            let dtos: Vec<ConversationDto> = conversations
                .iter()
                .map(|c| ConversationDto {
                    id: c.id.clone(),
                    bot_name: c.bot_name.clone(),
                    chat_type: match c.chat_type {
                        crate::conversation::models::ChatType::Group => "group".to_string(),
                        crate::conversation::models::ChatType::Private => "private".to_string(),
                    },
                    chat_id: c.chat_id.clone(),
                    title: c.title.clone(),
                    created_at: c.created_at.to_rfc3339().to_string(),
                    updated_at: c.updated_at.to_rfc3339().to_string(),
                })
                .collect();
            Ok(Json(dtos))
        }
        Err(e) => {
            error!(error = %e, "Failed to list conversations");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to list conversations: {}", e) })),
            ))
        }
    }
}

/// Create a new conversation
async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateConversationRequestDto>,
) -> Result<Json<ConversationDto>, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    let chat_type = match req.chat_type.to_lowercase().as_str() {
        "group" => crate::conversation::models::ChatType::Group,
        "private" => crate::conversation::models::ChatType::Private,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid chat_type. Must be 'group' or 'private'", })),
            ));
        }
    };

    let create_req = crate::conversation::models::CreateConversationRequest {
        bot_name: req.bot_name,
        chat_type,
        chat_id: req.chat_id,
        title: req.title,
    };

    match db.create_conversation(create_req).await {
        Ok(conversation) => {
            let dto = ConversationDto {
                id: conversation.id.clone(),
                bot_name: conversation.bot_name.clone(),
                chat_type: match conversation.chat_type {
                    crate::conversation::models::ChatType::Group => "group".to_string(),
                    crate::conversation::models::ChatType::Private => "private".to_string(),
                },
                chat_id: conversation.chat_id.clone(),
                title: conversation.title.clone(),
                created_at: conversation.created_at.to_rfc3339().to_string(),
                updated_at: conversation.updated_at.to_rfc3339().to_string(),
            };
            debug!("Created conversation with id: {}", dto.id);
            Ok(Json(dto))
        }
        Err(e) => {
            error!(error = %e, "Failed to create conversation");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to create conversation: {}", e) })),
            ))
        }
    }
}

/// Get a specific conversation by ID
async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConversationDto>, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    match db.get_conversation_by_id(&id).await {
        Ok(conversation) => {
            let dto = ConversationDto {
                id: conversation.id.clone(),
                bot_name: conversation.bot_name.clone(),
                chat_type: match conversation.chat_type {
                    crate::conversation::models::ChatType::Group => "group".to_string(),
                    crate::conversation::models::ChatType::Private => "private".to_string(),
                },
                chat_id: conversation.chat_id.clone(),
                title: conversation.title.clone(),
                created_at: conversation.created_at.to_rfc3339().to_string(),
                updated_at: conversation.updated_at.to_rfc3339().to_string(),
            };
            Ok(Json(dto))
        }
        Err(e) => {
            error!(error = %e, conversation_id = %id, "Failed to get conversation");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to get conversation: {}", e) })),
            ))
        }
    }
}

/// Delete a conversation
async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    match db.delete_conversation(&id).await {
        Ok(_) => {
            debug!("Deleted conversation with id: {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!(error = %e, conversation_id = %id, "Failed to delete conversation");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to delete conversation: {}", e) })),
            ))
        }
    }
}

/// Add a message to a conversation
async fn add_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<AddMessageRequestDto>,
) -> Result<Json<MessageDto>, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    // Use conversation_id from path parameter
    let convo_id = conversation_id.clone(); // Clone for error logging
    let add_req = crate::conversation::models::AddMessageRequest {
        conversation_id,
        role: req.role,
        content: req.content,
    };

    match db.add_message(add_req).await {
        Ok(message) => {
            let dto = MessageDto {
                id: message.id.clone(),
                conversation_id: message.conversation_id.clone(),
                role: message.role.clone(),
                content: message.content.clone(),
                created_at: message.created_at.to_rfc3339().to_string(),
            };
            debug!(
                "Added message with id: {} to conversation: {}",
                dto.id, dto.conversation_id
            );
            Ok(Json(dto))
        }
        Err(e) => {
            error!(error = %e, conversation_id = %convo_id, "Failed to add message");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to add message: {}", e) })),
            ))
        }
    }
}

/// Get all messages in a conversation
async fn get_conversation_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<MessageDto>>, (StatusCode, Json<Value>)> {
    let conversation_db = state.conversation_db.read().await;
    let db = conversation_db.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "Conversation database not initialized" })),
    ))?;

    match db.get_conversation_messages(&conversation_id).await {
        Ok(messages) => {
            let dtos: Vec<MessageDto> = messages
                .iter()
                .map(|m| MessageDto {
                    id: m.id.clone(),
                    conversation_id: m.conversation_id.clone(),
                    role: m.role.clone(),
                    content: m.content.clone(),
                    created_at: m.created_at.to_rfc3339().to_string(),
                })
                .collect();
            Ok(Json(dtos))
        }
        Err(e) => {
            error!(error = %e, conversation_id = %conversation_id, "Failed to get conversation messages");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to get conversation messages: {}", e) })),
            ))
        }
    }
}

// ─── MCP Server Configuration ─────────────────────────────────────

/// List all MCP server configurations
async fn list_mcp_servers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<McpServerConfig>>, (StatusCode, Json<Value>)> {
    let mcp_config = state.mcp_config.read().await;
    let mcp_config = mcp_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.list_servers().await {
        Ok(servers) => Ok(Json(servers)),
        Err(e) => {
            error!(error = %e, "Failed to list MCP servers");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to list MCP servers: {}", e) })),
            ))
        }
    }
}

/// Get a specific MCP server configuration
async fn get_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<McpServerConfig>, (StatusCode, Json<Value>)> {
    let mcp_config = state.mcp_config.read().await;
    let mcp_config = mcp_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.get_server(&id).await {
        Ok(Some(server)) => Ok(Json(server)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("MCP server not found: {}", id) })),
        )),
        Err(e) => {
            error!(error = %e, %id, "Failed to get MCP server");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to get MCP server: {}", e) })),
            ))
        }
    }
}

/// Create a new MCP server configuration
async fn create_mcp_server(
    State(state): State<Arc<AppState>>,
    Json(req): Json<crate::api::models::CreateMcpServerRequest>,
) -> Result<Json<McpServerConfig>, (StatusCode, Json<Value>)> {
    let server = McpServerConfig {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        transport_type: req.transport_type,
        transport_config: req.transport_config,
        enabled: Some(req.enabled),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let mut mcp_config = state.mcp_config.write().await;
    let mcp_config = mcp_config.as_mut().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.create_server(&server).await {
        Ok(_) => {
            debug!("Created MCP server: {}", server.name);
            Ok(Json(server))
        }
        Err(e) => {
            error!(error = %e, "Failed to create MCP server");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to create MCP server: {}", e) })),
            ))
        }
    }
}

/// Update an existing MCP server configuration
async fn update_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<crate::api::models::UpdateMcpServerRequest>,
) -> Result<Json<McpServerConfig>, (StatusCode, Json<Value>)> {
    // Get existing server first
    let existing = {
        let mcp_config = state.mcp_config.read().await;
        let mcp_config = mcp_config.as_ref().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "MCP config not initialized" })),
        ))?;
        mcp_config.get_server(&id).await.map_err(|e| {
            error!(error = %e, %id, "Failed to get MCP server for update");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to get MCP server: {}", e) })),
            )
        })?
    };

    let existing = existing.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("MCP server not found: {}", id) })),
    ))?;

    // Merge: only update fields that are provided
    let server = McpServerConfig {
        id: id.clone(),
        name: req.name.unwrap_or(existing.name),
        transport_type: req.transport_type.unwrap_or(existing.transport_type),
        transport_config: req.transport_config.unwrap_or(existing.transport_config),
        enabled: req.enabled.map(Some).unwrap_or(existing.enabled),
        created_at: existing.created_at,
        updated_at: chrono::Utc::now(),
    };

    let mut mcp_config = state.mcp_config.write().await;
    let mcp_config = mcp_config.as_mut().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.update_server(&server).await {
        Ok(_) => {
            debug!("Updated MCP server: {}", server.name);
            Ok(Json(server))
        }
        Err(e) => {
            error!(error = %e, %id, "Failed to update MCP server");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to update MCP server: {}", e) })),
            ))
        }
    }
}

/// Delete an MCP server configuration
async fn delete_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let mut mcp_config = state.mcp_config.write().await;
    let mcp_config = mcp_config.as_mut().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.delete_server(&id).await {
        Ok(_) => {
            debug!("Deleted MCP server: {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!(error = %e, %id, "Failed to delete MCP server");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to delete MCP server: {}", e) })),
            ))
        }
    }
}

/// Toggle MCP server enabled/disabled status
async fn toggle_mcp_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<McpServerConfig>, (StatusCode, Json<Value>)> {
    // First get the current state
    let mcp_config = state.mcp_config.read().await;
    let mcp_config = mcp_config.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    let server = mcp_config.get_server(&id).await.map_err(|e| {
        error!(error = %e, %id, "Failed to get MCP server");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to get MCP server: {}", e) })),
        )
    })?;

    let server = server.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({ "error": format!("MCP server not found: {}", id) })),
    ))?;

    let new_enabled = !server.enabled.unwrap_or(true);
    let _ = mcp_config;

    // Update the enabled state
    let mut mcp_config = state.mcp_config.write().await;
    let mcp_config = mcp_config.as_mut().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MCP config not initialized" })),
    ))?;
    match mcp_config.set_enabled(&id, new_enabled).await {
        Ok(_) => {
            debug!("Toggled MCP server: {} - enabled: {}", id, new_enabled);
            // Fetch the updated server
            match mcp_config.get_server(&id).await {
                Ok(Some(updated_server)) => Ok(Json(updated_server)),
                _ => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Failed to fetch updated server" })),
                )),
            }
        }
        Err(e) => {
            error!(error = %e, %id, "Failed to toggle MCP server");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to toggle MCP server: {}", e) })),
            ))
        }
    }
}

// ─── Platform Handlers ───────────────────────────────────────────

use crate::api::models::{CreatePlatformRequest, PlatformInstanceDto, UpdatePlatformRequest};
use crate::platform::manager::PlatformInstanceConfig;
use axum::response::IntoResponse;

async fn list_platforms(State(state): State<Arc<AppState>>) -> Json<Vec<PlatformInstanceDto>> {
    let configs = state.platform_configs.read().await;
    let statuses = state.platform_statuses_async().await;
    let list: Vec<PlatformInstanceDto> = configs
        .iter()
        .map(|c| PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            config: c.extra.clone(),
            status: statuses
                .get(&c.id)
                .cloned()
                .unwrap_or_else(|| "stopped".to_string()),
        })
        .collect();
    Json(list)
}

async fn get_platform(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let configs = state.platform_configs.read().await;
    let statuses = state.platform_statuses_async().await;
    match configs.iter().find(|c| c.id == id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            config: c.extra.clone(),
            status: statuses
                .get(&c.id)
                .cloned()
                .unwrap_or_else(|| "stopped".to_string()),
        })
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Platform not found"})),
        )
            .into_response(),
    }
}

async fn create_platform(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePlatformRequest>,
) -> Response {
    // Validate the platform type
    match req.platform_type.as_str() {
        "dingtalk" | "discord" => {}
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Unknown platform type: {}", other)})),
            )
                .into_response();
        }
    }

    let mut configs = state.platform_configs.write().await;

    // Check for duplicate ID
    if configs.iter().any(|c| c.id == req.id) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "Platform with this ID already exists"})),
        )
            .into_response();
    }

    let platform_id = req.id.clone();
    let new_config = PlatformInstanceConfig {
        platform_type: req.platform_type.clone(),
        id: platform_id.clone(),
        extra: req.config,
    };

    configs.push(new_config);

    // Save to file (release write lock first)
    drop(configs);
    state.save_platforms_config().await.ok();

    // Return the created platform (adapter will be started by config profile activation)
    let statuses = state.platform_statuses_async().await;
    let configs = state.platform_configs.read().await;
    match configs.iter().find(|c| c.id == platform_id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            config: c.extra.clone(),
            status: statuses
                .get(&c.id)
                .cloned()
                .unwrap_or_else(|| "stopped".to_string()),
        })
        .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create platform"})),
        )
            .into_response(),
    }
}

async fn update_platform(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePlatformRequest>,
) -> Response {
    let mut configs = state.platform_configs.write().await;

    let index = match configs.iter().position(|c| c.id == id) {
        Some(i) => i,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Platform not found"})),
            )
                .into_response();
        }
    };

    if let Some(ref new_id) = req.id {
        // Check for duplicate ID if changing
        if new_id != &id && configs.iter().any(|c| c.id == *new_id) {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "Platform with this ID already exists"})),
            )
                .into_response();
        }
    }

    let config = &mut configs[index];

    // Track the old ID for adapter management
    let old_id = config.id.clone();

    if let Some(new_id) = req.id {
        config.id = new_id;
    }
    if let Some(pt) = req.platform_type {
        config.platform_type = pt;
    }
    if let Some(extra) = req.config {
        config.extra = extra;
    }

    // Get the updated ID for adapter management
    let new_id = config.id.clone();

    drop(configs);

    // Save config to file
    state.save_platforms_config().await.ok();

    // If ID changed, remove old adapter (adapter lifecycle is managed by config profile)
    {
        let mut pm = state.platform_manager.write().await;
        if old_id != new_id && pm.is_running(&old_id) {
            if let Err(e) = pm.remove_platform(&old_id).await {
                tracing::warn!("Failed to stop old platform adapter {}: {}", old_id, e);
            }
        }
    }

    let statuses = state.platform_statuses_async().await;
    let configs = state.platform_configs.read().await;
    match configs.iter().find(|c| c.id == new_id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            config: c.extra.clone(),
            status: statuses
                .get(&c.id)
                .cloned()
                .unwrap_or_else(|| "stopped".to_string()),
        })
        .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Platform not found after update"})),
        )
            .into_response(),
    }
}

async fn delete_platform(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    // Stop the platform adapter first
    {
        let mut pm = state.platform_manager.write().await;
        if pm.is_running(&id) {
            if let Err(e) = pm.remove_platform(&id).await {
                tracing::warn!("Failed to stop platform adapter {}: {}", id, e);
            }
        }
    }

    let mut configs = state.platform_configs.write().await;
    let len_before = configs.len();
    configs.retain(|c| c.id != id);
    if configs.len() == len_before {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Platform not found"})),
        )
            .into_response();
    }
    drop(configs);
    state.save_platforms_config().await.ok();
    StatusCode::NO_CONTENT.into_response()
}

// ─── System endpoints ──────────────────────────────────────────

/// Restart the entire server process by re-executing the current binary.
async fn restart_system(State(state): State<Arc<AppState>>) -> Response {
    tracing::info!("System restart requested via API");

    // Save all configs before restarting
    if let Err(e) = state.save_to_file(&state.config_path).await {
        tracing::warn!("Failed to save config before restart: {}", e);
    }
    if let Err(e) = state.save_platforms_config().await {
        tracing::warn!("Failed to save platforms config before restart: {}", e);
    }

    // Shutdown all platform adapters gracefully
    {
        let mut pm = state.platform_manager.write().await;
        pm.shutdown_all().await;
    }

    // Spawn the restart in a separate task so we can respond first
    tokio::spawn(async move {
        // Give the HTTP response time to be sent
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        tracing::info!("Restarting Ruri server...");

        // Re-execute the current binary
        let current_exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                tracing::error!("Failed to get current executable path: {}", e);
                return;
            }
        };

        let args: Vec<String> = std::env::args().skip(1).collect();

        match tokio::process::Command::new(current_exe)
            .args(&args)
            .spawn()
        {
            Ok(_) => {
                tracing::info!("New server instance started, shutting down current instance");
                // Exit the current process
                std::process::exit(0);
            }
            Err(e) => {
                tracing::error!("Failed to restart server: {}", e);
            }
        }
    });

    Json(json!({"message": "Server is restarting"})).into_response()
}

/// Restart a platform adapter by stopping and starting it.
///
/// This is useful for applying configuration changes without restarting
/// the entire server. The adapter is stopped, then started again using
/// the current config profile's settings (including proxy).
async fn restart_platform(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    // Find the platform config
    let platform_config = {
        let configs = state.platform_configs.read().await;
        configs.iter().find(|c| c.id == id).cloned()
    };

    let Some(config) = platform_config else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Platform not found"})),
        )
            .into_response();
    };

    // Check that the platform is in the active profile
    let is_active = {
        let profiles = state.config_profiles.read().await;
        profiles
            .values()
            .find(|p| p.is_active && p.enable)
            .map(|p| p.active_platform_ids.contains(&id))
            .unwrap_or(false)
    };

    if !is_active {
        return (
            StatusCode::FAILED_DEPENDENCY,
            Json(json!({"error": "Platform is not active in the current config profile"})),
        )
            .into_response();
    }

    // Get proxy config from active profile
    let proxy_config = {
        let profiles = state.config_profiles.read().await;
        profiles
            .values()
            .find(|p| p.is_active && p.enable)
            .and_then(|p| {
                if p.proxy_config.is_configured() {
                    Some(p.proxy_config.clone())
                } else {
                    None
                }
            })
    };

    // Inject proxy_url into config (respecting rules mode)
    let mut config_with_proxy = config.clone();
    if let Some(ref proxy) = proxy_config {
        let platform_host = match config_with_proxy.platform_type.as_str() {
            "discord" => "discord.gg",
            "dingtalk" => "dingtalk.com",
            other => other,
        };

        if proxy.should_proxy(platform_host) {
            if let Some(obj) = config_with_proxy.extra.as_object_mut() {
                obj.insert(
                    "proxy_url".to_string(),
                    serde_json::Value::String(proxy.url.clone()),
                );
            }
        } else if let Some(obj) = config_with_proxy.extra.as_object_mut() {
            obj.remove("proxy_url");
        }
    } else if let Some(obj) = config_with_proxy.extra.as_object_mut() {
        obj.remove("proxy_url");
    }

    // Restart the adapter
    {
        let mut pm = state.platform_manager.write().await;
        if let Err(e) = pm.restart_platform(config_with_proxy).await {
            tracing::error!(platform_id = %id, error = %e, "Failed to restart platform");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to restart platform: {}", e)})),
            )
                .into_response();
        }
    }

    tracing::info!(platform_id = %id, "Platform restarted via API");

    // Return the updated platform status
    let statuses = state.platform_statuses_async().await;
    let configs = state.platform_configs.read().await;
    match configs.iter().find(|c| c.id == id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            config: c.extra.clone(),
            status: statuses
                .get(&c.id)
                .cloned()
                .unwrap_or_else(|| "stopped".to_string()),
        })
        .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Platform not found after restart"})),
        )
            .into_response(),
    }
}
