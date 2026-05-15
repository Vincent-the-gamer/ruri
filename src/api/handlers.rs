use axum::extract::ws::Utf8Bytes;
use axum::{
    Json, Router,
    extract::{
        Path, Query, Request, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::{IntoResponse, Response, sse::Event},
    routing::{delete, get, patch, post},
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{Cursor, Read};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error};
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
    // Build protected API routes with authentication middleware
    let protected_routes = Router::new()
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
        // Built-in commands
        .route("/api/commands", get(list_builtin_commands))
        .route("/api/commands/{name}/admin", patch(toggle_command_admin))
        // Chat
        .route("/api/chat", post(send_chat_message))
        .route("/api/chat/stream", post(stream_chat_message))
        .route(
            "/api/chat/history",
            get(get_chat_history).delete(clear_chat_history),
        )
        // Chat stop
        .route("/api/chat/stop", post(stop_chat_generation))
        // Agent status
        .route("/api/agent/status", get(get_status))
        // Persona library (reusable templates — not active/global config)
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
            "/api/config-profiles/{id}/deactivate",
            post(deactivate_config_profile),
        )
        .route(
            "/api/config-profiles/{id}/provider",
            get(get_config_profile_provider),
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
        .route(
            "/api/platforms/{id}/weixin-qr-login",
            post(weixin_qr_login_start),
        )
        .route(
            "/api/platforms/{id}/weixin-qr-status",
            get(weixin_qr_login_status),
        )
        // System
        .route("/api/system/restart", post(restart_system))
        // Knowledge base
        .route(
            "/api/knowledge-bases",
            get(list_knowledge_bases).post(create_knowledge_base),
        )
        .route(
            "/api/knowledge-bases/{id}",
            get(get_knowledge_base)
                .put(update_knowledge_base)
                .delete(delete_knowledge_base),
        )
        .route(
            "/api/knowledge-bases/{kb_id}/documents",
            get(list_kb_documents).post(upload_kb_document),
        )
        .route(
            "/api/knowledge-bases/{kb_id}/documents/{doc_id}",
            delete(delete_kb_document),
        )
        .route(
            "/api/knowledge-bases/{kb_id}/search",
            post(search_knowledge_base),
        )
        // Debug session (WebUI chat independent configuration)
        .route(
            "/api/debug-session",
            get(get_debug_session).put(update_debug_session),
        )
        // Apply authentication middleware to all protected routes
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_auth,
        ))
        .with_state(state.clone());

    // Merge auth routes (login, logout, etc.) with protected routes
    let auth_router = crate::auth::create_auth_router(state.clone());

    Router::new().merge(protected_routes).merge(auth_router)
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
                supports_multimodal: false,
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
    tracing::info!(
        message_len = req.message.len(),
        provider_id = ?req.provider_id,
        user_id = ?req.user_id,
        session_id = ?req.session_id,
        source = "webui",
        "Received non-streaming chat request"
    );

    // ── Command dispatch ─────────────────────────────────────────
    // Try to dispatch as a built-in command. If a known command matched,
    // `dispatch` returns `Some(result)` and we skip the LLM. Otherwise
    // (no prefix, prefix-only, or unrecognized command) we fall through
    // to the agent / LLM.
    {
        // Resolve per-context prefix and enabled_commands from the debug session
        // (WebUI chat is independent of config profiles).
        let (ctx_prefix, ctx_enabled_commands) = {
            let debug = state.debug_session.read().await;
            (debug.command_prefix.clone(), debug.enabled_commands.clone())
        };
        let dispatcher = state.command_dispatcher.read().await;
        let user_id = req.user_id.clone().unwrap_or_default();
        let session_id = req.session_id.clone().unwrap_or_default();
        let cmd_ctx = crate::command::CommandContext {
            raw_message: req.message.clone(),
            command_name: String::new(),
            args: String::new(),
            prefix: ctx_prefix,
            enabled_commands: ctx_enabled_commands,
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            platform_id: "webui".to_string(),
            self_id: "ruri".to_string(),
            message_type: crate::platform::types::MessageType::FriendMessage,
            group_id: String::new(),
            state: state.clone(),
        };

        if let Some(result) = dispatcher.dispatch(cmd_ctx).await {
            // A known command was matched — return its result directly
            let message_dto = ChatMessageDto {
                role: "assistant".to_string(),
                content: serde_json::Value::String(result.reply),
                tool_calls: None,
                tool_call_id: None,
            };
            return Ok(Json(ChatResponseDto {
                message: message_dto,
                tool_results: None,
                usage: None,
            }));
        }
    }

    // ── Agent processing ─────────────────────────────────────────
    // Build agent using Debug Session configuration (WebUI chat is independent)
    let agent_result = state
        .build_agent_with_context_extended(
            req.user_id.as_deref(),
            req.session_id.as_deref(),
            req.provider_id.as_deref(),
            true, // use_debug_session: true - WebUI chat uses its own config
            None, // profile_id: None - let debug session resolve itself
        )
        .await;
    let mut agent =
        agent_result.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))))?;

    // Knowledge Base skill is already loaded from the debug session context
    // via build_agent_with_context_extended. Do not add another one here to
    // ensure strict context isolation — the WebUI chat uses whatever knowledge
    // bases the debug session configuration has selected.

    // Apply Function Calling parameters from the request
    if req.tool_choice.is_some() {
        agent.set_tool_choice(req.tool_choice.clone());
    }
    if req.parallel_tool_calls.is_some() {
        agent.set_parallel_tool_calls(req.parallel_tool_calls);
    }

    // Register a cancellation token for this session so /stop can cancel it
    let session_key = req
        .session_id
        .clone()
        .unwrap_or_else(|| "webui".to_string());
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel_token.clone();
    {
        let mut tasks = state.running_agent_tasks.write().await;
        tasks.insert(session_key.clone(), cancel_token);
    }

    // Build user message (text-only or multimodal with images/files)
    let user_msg = if req.images.is_empty() && req.files.is_empty() {
        crate::types::ChatMessage::user(&req.message)
    } else {
        let mut parts: Vec<crate::types::ContentPart> = Vec::new();

        // Add images
        for url in &req.images {
            if url.starts_with("data:") {
                // Parse data URL: data:{media_type};base64,{data}
                if let Some((media_type, data)) = parse_data_url(url) {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Image,
                        text: None,
                        image_url: None,
                        image_data: Some(crate::types::ImageData { data, media_type }),
                    });
                } else {
                    // Fallback: if we can't parse the data URL, send it as image_url
                    // (some providers like OpenAI accept data URLs in the url field)
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::ImageUrl,
                        text: None,
                        image_url: Some(crate::types::ImageUrl {
                            url: url.clone(),
                            detail: None,
                        }),
                        image_data: None,
                    });
                }
            } else {
                // Regular HTTP(S) URL
                parts.push(crate::types::ContentPart {
                    part_type: crate::types::ContentPartType::ImageUrl,
                    text: None,
                    image_url: Some(crate::types::ImageUrl {
                        url: url.clone(),
                        detail: None,
                    }),
                    image_data: None,
                });
            }
        }

        // Extract text from attached files and add as text or image parts
        for file in &req.files {
            if file.mime_type.starts_with("image/") && file.content.starts_with("data:") {
                // Image file: convert to Image part
                if let Some((media_type, data)) = parse_data_url(&file.content) {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Image,
                        text: None,
                        image_url: None,
                        image_data: Some(crate::types::ImageData { data, media_type }),
                    });
                } else {
                    // Fallback: treat as image_url with data URL
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::ImageUrl,
                        text: None,
                        image_url: Some(crate::types::ImageUrl {
                            url: file.content.clone(),
                            detail: None,
                        }),
                        image_data: None,
                    });
                }
            } else {
                // Non-image file or plain text: extract text content
                let file_text =
                    extract_attached_file_text(&file.name, &file.mime_type, &file.content);
                if let Some(text) = file_text {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Text,
                        text: Some(format!("--- File: {} ---\n{}", file.name, text)),
                        image_url: None,
                        image_data: None,
                    });
                }
            }
        }

        // Add the user's text message as the last part
        parts.push(crate::types::ContentPart {
            part_type: crate::types::ContentPartType::Text,
            text: Some(req.message.clone()),
            image_url: None,
            image_data: None,
        });

        crate::types::ChatMessage {
            role: crate::types::MessageRole::User,
            content: Some(crate::types::MessageContent::Parts(parts)),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    };

    // Send message with cancellation support
    let response = tokio::select! {
        result = agent.chat_with_message(user_msg) => {
            // Remove the cancellation token when done
            {
                let mut tasks = state.running_agent_tasks.write().await;
                tasks.remove(&session_key);
            }
            result
        }
        _ = cancel_clone.cancelled() => {
            tracing::info!(
                session_id = %session_key,
                "WebUI agent task was cancelled via /stop"
            );
            return Err((
                StatusCode::OK,
                Json(json!({ "error": "任务已停止", "stopped": true })),
            ));
        }
    }
    .map_err(|e| {
        // Use custom_error_message: request override > config profile > raw error
        let custom_msg = req.custom_error_message.clone().or_else(|| {
            // SAFETY: We are in a synchronous closure (or_else), but this runs inside
            // a Tokio runtime. Use try_read to avoid blocking the runtime.
            // If the lock is contended, we simply fall through to the default error message.
            state.config_profiles.try_read().ok().and_then(|profiles| {
                profiles
                    .values()
                    .filter(|p| p.is_active && p.enable)
                    .find_map(|p| p.custom_error_message.clone())
            })
        });
        let error_msg = custom_msg.unwrap_or_else(|| e.to_string());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error_msg })),
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
        .and_then(|c| c.as_text_full())
        .unwrap_or_default();

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

/// Stream a chat message response using Server-Sent Events (SSE).
///
/// This endpoint sends incremental `ContentDelta` events as the model
/// generates tokens, giving the WebUI a real-time typing effect.
/// The platform-facing `/api/chat` endpoint continues to use
/// non-streaming responses.
async fn stream_chat_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequestDto>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    use crate::agent::runner::AgentStreamer;

    tracing::info!(
        message_len = req.message.len(),
        provider_id = ?req.provider_id,
        user_id = ?req.user_id,
        session_id = ?req.session_id,
        source = "webui-stream",
        "Received streaming chat request"
    );

    // Store user message text for early use
    let user_message_text = req.message.clone();

    // ── Command dispatch ────────────────────────────────────────
    // Try to dispatch as a built-in command. If a known command matched,
    // `dispatch` returns `Some(result)` and we skip the LLM. Otherwise
    // (no prefix, prefix-only, or unrecognized command) we fall through
    // to the agent / LLM.
    {
        // Resolve per-context prefix and enabled_commands from the debug session
        // (WebUI chat is independent of config profiles).
        let (ctx_prefix, ctx_enabled_commands) = {
            let debug = state.debug_session.read().await;
            (debug.command_prefix.clone(), debug.enabled_commands.clone())
        };
        let dispatcher = state.command_dispatcher.read().await;
        let user_id = req.user_id.clone().unwrap_or_default();
        let session_id = req.session_id.clone().unwrap_or_default();
        let cmd_ctx = crate::command::CommandContext {
            raw_message: req.message.clone(),
            command_name: String::new(),
            args: String::new(),
            prefix: ctx_prefix,
            enabled_commands: ctx_enabled_commands,
            session_id: session_id.clone(),
            user_id: user_id.clone(),
            platform_id: "webui".to_string(),
            self_id: "ruri".to_string(),
            message_type: crate::platform::types::MessageType::FriendMessage,
            group_id: String::new(),
            state: state.clone(),
        };

        if let Some(result) = dispatcher.dispatch(cmd_ctx).await {
            // A known command was matched — return its result as an SSE stream
            let reply = result.reply;
            let stream = async_stream::stream! {
                let event = crate::types::StreamEvent::ContentDelta { delta: reply };
                yield Ok::<Event, std::convert::Infallible>(Event::default().data(serde_json::to_string(&event).unwrap()));
                let done_event = crate::types::StreamEvent::Done { usage: None };
                yield Ok(Event::default().data(serde_json::to_string(&done_event).unwrap()));
            };
            return Ok(axum::response::Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::default())
                .into_response());
        }
    }

    // ── Agent processing ─────────────────────────────────────────
    // Build agent using Debug Session configuration (WebUI chat is independent)
    let agent_result = state
        .build_agent_with_context_extended(
            req.user_id.as_deref(),
            req.session_id.as_deref(),
            req.provider_id.as_deref(),
            true, // use_debug_session: true - WebUI chat uses its own config
            None, // profile_id: None - let debug session resolve itself
        )
        .await;
    let mut agent =
        agent_result.map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))))?;

    // Knowledge Base skill is already loaded from the debug session context
    // via build_agent_with_context_extended. Do not add another one here to
    // ensure strict context isolation — the WebUI chat uses whatever knowledge
    // bases the debug session configuration has selected.

    // Apply Function Calling parameters
    if req.tool_choice.is_some() {
        agent.set_tool_choice(req.tool_choice.clone());
    }
    if req.parallel_tool_calls.is_some() {
        agent.set_parallel_tool_calls(req.parallel_tool_calls);
    }

    // Register cancellation token
    let session_key = req
        .session_id
        .clone()
        .unwrap_or_else(|| "webui".to_string());
    let cancel_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tasks = state.running_agent_tasks.write().await;
        tasks.insert(session_key.clone(), cancel_token.clone());
    }

    // Build user message
    let user_msg = if req.images.is_empty() && req.files.is_empty() {
        crate::types::ChatMessage::user(&req.message)
    } else {
        // Reuse the same message building logic as send_chat_message
        let mut parts: Vec<crate::types::ContentPart> = Vec::new();
        for url in &req.images {
            if url.starts_with("data:") {
                if let Some((media_type, data)) = parse_data_url(url) {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Image,
                        text: None,
                        image_url: None,
                        image_data: Some(crate::types::ImageData { data, media_type }),
                    });
                } else {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::ImageUrl,
                        text: None,
                        image_url: Some(crate::types::ImageUrl {
                            url: url.clone(),
                            detail: None,
                        }),
                        image_data: None,
                    });
                }
            } else {
                parts.push(crate::types::ContentPart {
                    part_type: crate::types::ContentPartType::ImageUrl,
                    text: None,
                    image_url: Some(crate::types::ImageUrl {
                        url: url.clone(),
                        detail: None,
                    }),
                    image_data: None,
                });
            }
        }
        for file in &req.files {
            if file.mime_type.starts_with("image/") && file.content.starts_with("data:") {
                if let Some((media_type, data)) = parse_data_url(&file.content) {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Image,
                        text: None,
                        image_url: None,
                        image_data: Some(crate::types::ImageData { data, media_type }),
                    });
                } else {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::ImageUrl,
                        text: None,
                        image_url: Some(crate::types::ImageUrl {
                            url: file.content.clone(),
                            detail: None,
                        }),
                        image_data: None,
                    });
                }
            } else {
                let file_text =
                    extract_attached_file_text(&file.name, &file.mime_type, &file.content);
                if let Some(text) = file_text {
                    parts.push(crate::types::ContentPart {
                        part_type: crate::types::ContentPartType::Text,
                        text: Some(format!("--- File: {} ---\n{}", file.name, text)),
                        image_url: None,
                        image_data: None,
                    });
                }
            }
        }
        parts.push(crate::types::ContentPart {
            part_type: crate::types::ContentPartType::Text,
            text: Some(req.message.clone()),
            image_url: None,
            image_data: None,
        });
        crate::types::ChatMessage {
            role: crate::types::MessageRole::User,
            content: Some(crate::types::MessageContent::Parts(parts)),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    };

    // For DB persistence, user_message_text is already set above
    let state_clone = state.clone();
    let cancel_clone = cancel_token.clone();

    // Create the streaming agent
    let streamer = AgentStreamer::new(agent, user_msg);
    let event_stream = streamer.into_stream();

    // Convert StreamEvents to SSE Events, and persist messages to DB on completion
    let sse_stream = async_stream::stream! {
        let mut full_content = String::new();
        let mut stream = event_stream;

        // Closure to persist user and assistant messages to the conversation database.
        // Should be called in every termination path (Done, Error, Cancelled, None).
        let persist_to_db = |full_content: &str| {
            let state = state_clone.clone();
            let user_text = user_message_text.clone();
            let content = full_content.to_string();
            async move {
                let conv_db = state.conversation_db.read().await;
                if let Some(db) = conv_db.as_ref() {
                    let conversation_id = state.ensure_chat_conversation().await.ok();
                    if let Some(conv_id) = conversation_id {
                        if let Err(e) = db
                            .add_message(crate::conversation::models::AddMessageRequest {
                                conversation_id: conv_id.clone(),
                                role: "user".to_string(),
                                content: user_text,
                            })
                            .await
                        {
                            tracing::error!("Failed to add user message to database: {}", e);
                        }
                        if !content.is_empty() {
                            if let Err(e) = db
                                .add_message(crate::conversation::models::AddMessageRequest {
                                    conversation_id: conv_id,
                                    role: "assistant".to_string(),
                                    content,
                                })
                                .await
                            {
                                tracing::error!("Failed to add assistant message to database: {}", e);
                            }
                        }
                    }
                }
            }
        };

        // Handle cancellation
        loop {
            tokio::select! {
                event_result = stream.next() => {
                    match event_result {
                        Some(Ok(event)) => {
                            // Track content for DB persistence
                            if let crate::types::StreamEvent::ContentDelta { delta } = &event {
                                full_content.push_str(delta);
                            }

                            // Convert to SSE event
                            let data = serde_json::to_string(&event).unwrap_or_default();
                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(data));

                            // On Done, persist messages to DB
                            if let crate::types::StreamEvent::Done { .. } = &event {
                                persist_to_db(&full_content).await;
                                // Remove the cancellation token
                                {
                                    let mut tasks = state_clone.running_agent_tasks.write().await;
                                    tasks.remove(&session_key);
                                }
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            // Provider error
                            let custom_msg = req.custom_error_message.clone().or_else(|| {
                                // SAFETY: We are in a synchronous closure (or_else) inside a Tokio runtime.
                                // Use try_read to avoid blocking the runtime.
                                // If the lock is contended, we simply fall through to the default error message.
                                state_clone.config_profiles.try_read().ok().and_then(|profiles| {
                                    profiles
                                        .values()
                                        .filter(|p| p.is_active && p.enable)
                                        .find_map(|p| p.custom_error_message.clone())
                                })
                            });
                            let error_msg = custom_msg.unwrap_or_else(|| e.to_string());
                            let error_event = crate::types::StreamEvent::Error { error: error_msg };
                            let data = serde_json::to_string(&error_event).unwrap_or_default();
                            yield Ok(Event::default().data(data));
                            // Persist partial content to DB before breaking
                            persist_to_db(&full_content).await;
                            // Remove cancellation token
                            {
                                let mut tasks = state_clone.running_agent_tasks.write().await;
                                tasks.remove(&session_key);
                            }
                            break;
                        }
                        None => {
                            // Stream ended without Done – persist any partial content
                            persist_to_db(&full_content).await;
                            // Remove cancellation token
                            {
                                let mut tasks = state_clone.running_agent_tasks.write().await;
                                tasks.remove(&session_key);
                            }
                            break;
                        }
                    }
                }
                _ = cancel_clone.cancelled() => {
                    tracing::info!(
                        session_id = %session_key,
                        "WebUI streaming agent task was cancelled via /stop"
                    );
                    let stopped_event = crate::types::StreamEvent::Error {
                        error: "任务已停止".to_string(),
                    };
                    let data = serde_json::to_string(&stopped_event).unwrap_or_default();
                    yield Ok(Event::default().data(data));
                    // Persist partial content to DB before breaking
                    persist_to_db(&full_content).await;
                    // Remove cancellation token
                    {
                        let mut tasks = state_clone.running_agent_tasks.write().await;
                        tasks.remove(&session_key);
                    }
                    break;
                }
            }
        }
    };

    Ok(axum::response::Sse::new(sse_stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response())
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
                        content: serde_json::Value::String(m.content.clone()),
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

// ─── Chat Stop Handler ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StopChatRequest {
    pub session_id: Option<String>,
}

async fn stop_chat_generation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StopChatRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let session_key = req.session_id.unwrap_or_else(|| "webui".to_string());
    let mut tasks = state.running_agent_tasks.write().await;
    if let Some(cancel_token) = tasks.remove(&session_key) {
        cancel_token.cancel();
        tracing::info!(session_id = %session_key, "Chat generation stopped via /api/chat/stop");
        Ok(Json(json!({ "stopped": true, "session_id": session_key })))
    } else {
        tracing::debug!(session_id = %session_key, "No running task found for /api/chat/stop");
        Ok(Json(json!({ "stopped": false, "session_id": session_key })))
    }
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
                "gemini" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("gemini-2.0-flash"),
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
                "gemini" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("gemini-2.0-flash"),
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
        active_knowledge_base_ids: acp_config.active_knowledge_base_ids.clone(),
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

    if let Some(active_knowledge_base_ids) = req.active_knowledge_base_ids {
        acp_config.active_knowledge_base_ids = active_knowledge_base_ids;
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
                "gemini" => p.config_json["default_model"]
                    .as_str()
                    .unwrap_or("gemini-2.0-flash"),
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
        active_knowledge_base_ids: acp_config.active_knowledge_base_ids.clone(),
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
            "aio_sandbox" => crate::computer_use::ComputerUseRuntime::AioSandbox,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": format!("Invalid runtime: {}. Must be 'none', 'local', or 'aio_sandbox'", runtime)
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

    // Update command_admin_required if provided
    if let Some(command_admin_required) = req.command_admin_required {
        computer_use_config.command_admin_required = command_admin_required;
    }

    // Update aio_sandbox_config if provided
    if let Some(aio_sandbox_config_dto) = req.aio_sandbox_config {
        computer_use_config.aio_sandbox_config = Some(crate::computer_use::AioSandboxConfig {
            endpoint: aio_sandbox_config_dto.endpoint,
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

// ─── Persona Library Handlers ─────────────────────────────────────

/// List all persona templates in the library.
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

/// Get a specific persona template by ID.
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

/// Create a new persona template with an auto-generated UUID.
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
        "Persona template created"
    );

    state.auto_save().await;

    Ok(Json(PersonaDto {
        id,
        name: req.name,
        description: req.description,
        prompt: req.prompt,
    }))
}

/// Update an existing persona template by ID.
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

    tracing::info!(persona_id = %id, "Persona template updated");

    drop(personas);
    state.auto_save().await;

    Ok(Json(dto))
}

/// Delete a persona template by ID.
async fn delete_persona(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let mut personas = state.personas.write().await;
    if personas.remove(&id).is_some() {
        tracing::info!(persona_id = %id, "Persona template deleted");
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
            embedded_persona: p.embedded_persona.as_ref().map(EmbeddedPersonaDto::from),
            web_search_enabled: p.web_search_enabled,
            computer_use_enabled: p.computer_use_enabled,
            active_skill_names: p.active_skill_names.clone(),
            active_knowledge_base_ids: p.active_knowledge_base_ids.clone(),
            proxy_config: p.proxy_config.clone(),
            command_prefix: p.command_prefix.clone(),
            enabled_commands: p.enabled_commands.clone(),
            command_admin_required: p.command_admin_required.clone(),
            custom_error_message: p.custom_error_message.clone(),
            platform_ids: p.platform_ids.clone(),
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
            embedded_persona: p.embedded_persona.as_ref().map(EmbeddedPersonaDto::from),
            web_search_enabled: p.web_search_enabled,
            computer_use_enabled: p.computer_use_enabled,
            active_skill_names: p.active_skill_names.clone(),
            active_knowledge_base_ids: p.active_knowledge_base_ids.clone(),
            proxy_config: p.proxy_config.clone(),
            command_prefix: p.command_prefix.clone(),
            enabled_commands: p.enabled_commands.clone(),
            command_admin_required: p.command_admin_required.clone(),
            custom_error_message: p.custom_error_message.clone(),
            platform_ids: p.platform_ids.clone(),
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

    // Create the profile - make it active by default if enabled
    let is_active = req.enable;
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
        embedded_persona: req
            .embedded_persona
            .as_ref()
            .map(|dto| crate::api::state::EmbeddedPersona::from(dto)),
        embedded_providers: Vec::new(),
        active_embedded_provider: None,
        embedded_skills: Vec::new(),
        active_embedded_skill_names: Vec::new(),
        web_search_enabled: req.web_search_enabled,
        computer_use_enabled: req.computer_use_enabled,
        active_skill_names: req.active_skill_names.clone(),
        active_knowledge_base_ids: req.active_knowledge_base_ids.clone(),
        proxy_config: req.proxy_config.clone(),
        command_prefix: req.command_prefix.clone(),
        enabled_commands: req.enabled_commands.clone(),
        command_admin_required: req.command_admin_required.clone(),
        custom_error_message: req.custom_error_message.clone(),
        platform_ids: req.platform_ids.clone().unwrap_or_default(),
    };

    // Validate platform_ids: only one platform per profile allowed
    if let Some(ref platform_ids) = req.platform_ids {
        if platform_ids.len() > 1 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Only one platform can be selected per config profile to prevent conflicts",
                })),
            ));
        }
    }

    // Insert the profile
    let mut profiles = state.config_profiles.write().await;

    // Validate platform_ids: check no platform_id is already used by another profile
    if !req.platform_ids.as_ref().map_or(true, |ids| ids.is_empty()) {
        let existing_profile_platforms: Vec<String> = profiles
            .values()
            .flat_map(|p| p.platform_ids.iter().cloned())
            .collect();
        let conflicting: Vec<String> = req
            .platform_ids
            .as_ref()
            .unwrap()
            .iter()
            .filter(|pid| existing_profile_platforms.contains(pid))
            .cloned()
            .collect();
        if !conflicting.is_empty() {
            drop(profiles);
            return Err((
                StatusCode::CONFLICT,
                Json(json!({
                    "error": format!("Platform(s) already used by other profile: {}", conflicting.join(", ")),
                    "conflicting_platform_ids": conflicting,
                })),
            ));
        }
    }

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
        provider_id: req.provider_id.clone(),
        persona_id: req.persona_id.clone(),
        embedded_persona: req.embedded_persona.clone(),
        web_search_enabled: req.web_search_enabled,
        computer_use_enabled: req.computer_use_enabled,
        active_skill_names: req.active_skill_names,
        active_knowledge_base_ids: req.active_knowledge_base_ids,
        proxy_config: req.proxy_config,
        command_prefix: req.command_prefix,
        enabled_commands: req.enabled_commands,
        command_admin_required: req.command_admin_required.clone(),
        custom_error_message: req.custom_error_message.clone(),
        platform_ids: req.platform_ids.unwrap_or_default(),
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
    // Validate platform_ids before acquiring mutable borrow
    if let Some(ref platform_ids) = req.platform_ids {
        // Only one platform per profile allowed
        if platform_ids.len() > 1 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Only one platform can be selected per config profile to prevent conflicts",
                })),
            ));
        }

        let profiles = state.config_profiles.read().await;
        // Check that the profile exists first
        if !profiles.contains_key(&id) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Config profile not found" })),
            ));
        }
        let other_profile_platforms: Vec<String> = profiles
            .values()
            .filter(|p| p.id != id)
            .flat_map(|p| p.platform_ids.iter().cloned())
            .collect();
        let conflicting: Vec<String> = platform_ids
            .iter()
            .filter(|pid| other_profile_platforms.contains(pid))
            .cloned()
            .collect();
        if !conflicting.is_empty() {
            return Err((
                StatusCode::CONFLICT,
                Json(json!({
                    "error": format!("Platform(s) already used by other profile: {}", conflicting.join(", ")),
                    "conflicting_platform_ids": conflicting,
                })),
            ));
        }
        drop(profiles);
    }

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
            // If the active profile is being disabled, deactivate it so that
            // the system falls back to another enabled profile (or none).
            if !enable && profile.is_active {
                profile.is_active = false;
            }
        }
        if let Some(provider_id) = req.provider_id {
            tracing::info!(profile_id = %id, provider_id = ?provider_id, "Updating profile provider_id");
            profile.provider_id = provider_id;
        }
        if let Some(persona_id) = req.persona_id {
            tracing::info!(profile_id = %id, persona_id = ?persona_id, "Updating profile persona_id");
            profile.persona_id = persona_id;
        }
        if let Some(embedded_persona) = req.embedded_persona {
            tracing::info!(profile_id = %id, "Updating profile embedded_persona");
            profile.embedded_persona =
                embedded_persona.map(|dto| crate::api::state::EmbeddedPersona::from(&dto));
        }
        if let Some(web_search_enabled) = req.web_search_enabled {
            profile.web_search_enabled = web_search_enabled;
        }
        if let Some(computer_use_enabled) = req.computer_use_enabled {
            profile.computer_use_enabled = computer_use_enabled;
        }
        if let Some(active_skill_names) = req.active_skill_names {
            profile.active_skill_names = active_skill_names;
        }
        if let Some(active_knowledge_base_ids) = req.active_knowledge_base_ids {
            profile.active_knowledge_base_ids = active_knowledge_base_ids;
        }
        let enable_changed = req.enable.is_some();
        let proxy_changed = req.proxy_config.is_some();
        if let Some(proxy_config) = req.proxy_config {
            profile.proxy_config = proxy_config;
        }
        if let Some(command_prefix) = req.command_prefix {
            profile.command_prefix = command_prefix;
        }
        if let Some(enabled_commands) = req.enabled_commands {
            profile.enabled_commands = enabled_commands;
        }
        if let Some(command_admin_required) = req.command_admin_required {
            profile.command_admin_required = command_admin_required;
        }
        if let Some(custom_error_message) = req.custom_error_message {
            profile.custom_error_message = custom_error_message;
        }
        if let Some(platform_ids) = req.platform_ids {
            // Validation was already done above before acquiring mutable borrow
            profile.platform_ids = platform_ids;
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
            embedded_persona: profile
                .embedded_persona
                .as_ref()
                .map(EmbeddedPersonaDto::from),
            web_search_enabled: profile.web_search_enabled,
            computer_use_enabled: profile.computer_use_enabled,
            active_skill_names: profile.active_skill_names.clone(),
            active_knowledge_base_ids: profile.active_knowledge_base_ids.clone(),
            proxy_config: profile.proxy_config.clone(),
            command_prefix: profile.command_prefix.clone(),
            enabled_commands: profile.enabled_commands.clone(),
            command_admin_required: profile.command_admin_required.clone(),
            custom_error_message: profile.custom_error_message.clone(),
            platform_ids: profile.platform_ids.clone(),
        };

        let is_active = dto.is_active;
        let was_deactivated = enable_changed && !dto.enable && !dto.is_active;
        drop(profiles);
        state.auto_save().await;

        // Synchronize running platform adapters when:
        // - The active profile's proxy config changed, OR
        // - A profile was deactivated (was active, now disabled), so adapters
        //   that depended on its proxy need to be re-synced.
        if (is_active && (proxy_changed || enable_changed)) || was_deactivated {
            state.sync_platforms().await;
        }

        // Update command dispatcher state - merge from all active profiles
        if is_active {
            let profiles = state.config_profiles.read().await;
            let active_profiles: Vec<_> = profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .collect();
            if !active_profiles.is_empty() {
                // Merge: use union of enabled commands from all active profiles
                let mut merged_enabled_commands: Vec<String> = Vec::new();
                let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
                    std::collections::HashMap::new();
                // Use the first active profile's prefix as the effective prefix
                let effective_prefix = active_profiles[0].command_prefix.clone();

                for profile in &active_profiles {
                    for cmd in &profile.enabled_commands {
                        if !merged_enabled_commands.contains(cmd) {
                            merged_enabled_commands.push(cmd.clone());
                        }
                    }
                    for (cmd, admin_req) in &profile.command_admin_required {
                        merged_command_admin_required.insert(cmd.clone(), *admin_req);
                    }
                }

                let mut dispatcher = state.command_dispatcher.write().await;
                dispatcher.set_prefix(effective_prefix);
                dispatcher.set_enabled_commands(merged_enabled_commands);
                drop(dispatcher);
                let mut computer_use_config = state.computer_use_config.write().await;
                computer_use_config.command_admin_required = merged_command_admin_required;
            }
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
        // Just activate the target profile without deactivating others
        if let Some(profile) = profiles.get_mut(&id) {
            profile.is_active = true;
            profile.updated_at = now;
        }

        let profile = profiles.get(&id).unwrap();

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
            embedded_persona: profile
                .embedded_persona
                .as_ref()
                .map(EmbeddedPersonaDto::from),
            web_search_enabled: profile.web_search_enabled,
            computer_use_enabled: profile.computer_use_enabled,
            active_skill_names: profile.active_skill_names.clone(),
            active_knowledge_base_ids: profile.active_knowledge_base_ids.clone(),
            proxy_config: profile.proxy_config.clone(),
            command_prefix: profile.command_prefix.clone(),
            enabled_commands: profile.enabled_commands.clone(),
            command_admin_required: profile.command_admin_required.clone(),
            custom_error_message: profile.custom_error_message.clone(),
            platform_ids: profile.platform_ids.clone(),
        };

        drop(profiles);
        state.auto_save().await;

        tracing::info!(profile_id = %id, profile_name = %dto.name, "Config profile activated");

        // Synchronize platform adapters (proxy config may have changed)
        state.sync_platforms().await;

        // Update command dispatcher state - merge from all active profiles
        {
            let profiles = state.config_profiles.read().await;
            let active_profiles: Vec<_> = profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .collect();
            if !active_profiles.is_empty() {
                // Merge: use union of enabled commands from all active profiles
                let mut merged_enabled_commands: Vec<String> = Vec::new();
                let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
                    std::collections::HashMap::new();
                // Use the first active profile's prefix as the effective prefix
                let effective_prefix = active_profiles[0].command_prefix.clone();

                for profile in &active_profiles {
                    for cmd in &profile.enabled_commands {
                        if !merged_enabled_commands.contains(cmd) {
                            merged_enabled_commands.push(cmd.clone());
                        }
                    }
                    for (cmd, admin_req) in &profile.command_admin_required {
                        merged_command_admin_required.insert(cmd.clone(), *admin_req);
                    }
                }

                let mut dispatcher = state.command_dispatcher.write().await;
                dispatcher.set_prefix(effective_prefix);
                dispatcher.set_enabled_commands(merged_enabled_commands);
                drop(dispatcher);
                let mut computer_use_config = state.computer_use_config.write().await;
                computer_use_config.command_admin_required = merged_command_admin_required;
            }
        }

        Ok(Json(dto))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Config profile not found"})),
        ))
    }
}

/// Deactivate a specific config profile
async fn deactivate_config_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigProfileDto>, (StatusCode, Json<Value>)> {
    let mut profiles = state.config_profiles.write().await;

    if let Some(profile) = profiles.get_mut(&id) {
        profile.is_active = false;
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
            embedded_persona: profile
                .embedded_persona
                .as_ref()
                .map(EmbeddedPersonaDto::from),
            web_search_enabled: profile.web_search_enabled,
            computer_use_enabled: profile.computer_use_enabled,
            active_skill_names: profile.active_skill_names.clone(),
            active_knowledge_base_ids: profile.active_knowledge_base_ids.clone(),
            proxy_config: profile.proxy_config.clone(),
            command_prefix: profile.command_prefix.clone(),
            enabled_commands: profile.enabled_commands.clone(),
            command_admin_required: profile.command_admin_required.clone(),
            custom_error_message: profile.custom_error_message.clone(),
            platform_ids: profile.platform_ids.clone(),
        };

        drop(profiles);
        state.auto_save().await;

        // Synchronize platform adapters (proxy config may have changed)
        state.sync_platforms().await;

        // Update command dispatcher state - merge from all remaining active profiles
        {
            let profiles = state.config_profiles.read().await;
            let active_profiles: Vec<_> = profiles
                .values()
                .filter(|p| p.is_active && p.enable)
                .collect();
            if !active_profiles.is_empty() {
                // Merge: use union of enabled commands from all active profiles
                let mut merged_enabled_commands: Vec<String> = Vec::new();
                let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
                    std::collections::HashMap::new();
                // Use the first active profile's prefix as the effective prefix
                let effective_prefix = active_profiles[0].command_prefix.clone();

                for profile in &active_profiles {
                    for cmd in &profile.enabled_commands {
                        if !merged_enabled_commands.contains(cmd) {
                            merged_enabled_commands.push(cmd.clone());
                        }
                    }
                    for (cmd, admin_req) in &profile.command_admin_required {
                        merged_command_admin_required.insert(cmd.clone(), *admin_req);
                    }
                }

                let mut dispatcher = state.command_dispatcher.write().await;
                dispatcher.set_prefix(effective_prefix);
                dispatcher.set_enabled_commands(merged_enabled_commands);
                drop(dispatcher);
                let mut computer_use_config = state.computer_use_config.write().await;
                computer_use_config.command_admin_required = merged_command_admin_required;
            } else {
                // No active profiles left - clear the command dispatcher
                let mut dispatcher = state.command_dispatcher.write().await;
                dispatcher.set_enabled_commands(Vec::new());
            }
        }

        tracing::info!(profile_id = %id, "Config profile deactivated");

        Ok(Json(dto))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Config profile not found"})),
        ))
    }
}

/// List all built-in commands
async fn list_builtin_commands(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<crate::command::BuiltinCommandInfo>> {
    let dispatcher = state.command_dispatcher.read().await;
    let profiles = state.config_profiles.read().await;
    let active_profiles: Vec<_> = profiles
        .values()
        .filter(|p| p.is_active && p.enable)
        .collect();
    if active_profiles.is_empty() {
        let empty_admin_map: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();
        return Json(dispatcher.list_commands_info("/", &empty_admin_map, &Vec::new()));
    }
    // Merge: use union of enabled commands, merge admin requirements, first profile's prefix
    let prefix = active_profiles[0].command_prefix.as_str();
    let mut merged_enabled_commands: Vec<String> = Vec::new();
    let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();
    for profile in &active_profiles {
        for cmd in &profile.enabled_commands {
            if !merged_enabled_commands.contains(cmd) {
                merged_enabled_commands.push(cmd.clone());
            }
        }
        for (cmd, admin_req) in &profile.command_admin_required {
            merged_command_admin_required.insert(cmd.clone(), *admin_req);
        }
    }
    Json(dispatcher.list_commands_info(
        prefix,
        &merged_command_admin_required,
        &merged_enabled_commands,
    ))
}

/// Toggle the admin requirement for a specific built-in command.
/// Updates all active config profiles and re-syncs the dispatcher.
#[derive(Debug, Deserialize)]
struct ToggleCommandAdminRequest {
    /// true = admin required, false = open to all
    require_admin: bool,
}

async fn toggle_command_admin(
    State(state): State<Arc<AppState>>,
    Path(command_name): Path<String>,
    Json(req): Json<ToggleCommandAdminRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Validate the command exists
    {
        let dispatcher = state.command_dispatcher.read().await;
        if !dispatcher.commands().contains_key(command_name.as_str()) {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("Command '{}' not found", command_name)})),
            ));
        }
    }

    // Update all active config profiles
    let mut profiles = state.config_profiles.write().await;
    let active_profiles: Vec<_> = profiles
        .values_mut()
        .filter(|p| p.is_active && p.enable)
        .collect();

    if active_profiles.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                json!({"error": "No active config profile found. Please activate a profile first."}),
            ),
        ));
    }

    for profile in active_profiles {
        profile
            .command_admin_required
            .insert(command_name.clone(), req.require_admin);
        profile.updated_at = Utc::now();
    }
    drop(profiles);

    // Re-merge and sync dispatcher + computer_use_config
    {
        let profiles = state.config_profiles.read().await;
        let active_profiles: Vec<_> = profiles
            .values()
            .filter(|p| p.is_active && p.enable)
            .collect();

        let mut merged_enabled_commands: Vec<String> = Vec::new();
        let mut merged_command_admin_required: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();
        let effective_prefix = active_profiles[0].command_prefix.clone();

        for profile in &active_profiles {
            for cmd in &profile.enabled_commands {
                if !merged_enabled_commands.contains(cmd) {
                    merged_enabled_commands.push(cmd.clone());
                }
            }
            for (cmd, admin_req) in &profile.command_admin_required {
                merged_command_admin_required.insert(cmd.clone(), *admin_req);
            }
        }

        let mut dispatcher = state.command_dispatcher.write().await;
        dispatcher.set_prefix(effective_prefix);
        dispatcher.set_enabled_commands(merged_enabled_commands);
        drop(dispatcher);
        let mut computer_use_config = state.computer_use_config.write().await;
        computer_use_config.command_admin_required = merged_command_admin_required;
    }

    state.auto_save().await;

    tracing::info!(
        command = %command_name,
        require_admin = req.require_admin,
        "Toggled command admin requirement"
    );

    Ok(Json(json!({
        "command": command_name,
        "require_admin": req.require_admin,
    })))
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

/// WebSocket日志推送处理器
async fn ws_logs_handler(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut log_rx = state.log_manager.subscribe();
    let mut current_level_filter: Option<crate::logging::LogLevel> = None;

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    // Consume the first immediate tick
    ping_interval.tick().await;

    loop {
        tokio::select! {
            // 1. Broadcast log events
            log_result = log_rx.recv() => {
                match log_result {
                    Ok(log) => {
                        // Apply level filter if set
                        if let Some(ref filter) = current_level_filter {
                            if log.level < *filter {
                                continue;
                            }
                        }
                        if let Ok(log_json) = serde_json::to_string(&log) {
                            if ws_sender
                                .send(axum::extract::ws::Message::Text(Utf8Bytes::from(log_json)))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        // Client fell behind – log and continue rather than breaking
                        debug!("WebSocket log receiver lagged, skipped {} messages", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Channel closed, no more logs
                        break;
                    }
                }
            }

            // 2. Incoming messages from client (filter / get_since)
            maybe_msg = ws_receiver.next() => {
                match maybe_msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        // Try to parse as a client command
                        if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                            match cmd.get("type").and_then(|v| v.as_str()) {
                                Some("filter") => {
                                    if let Some(level_str) = cmd.get("level").and_then(|v| v.as_str()) {
                                        match serde_json::from_str::<crate::logging::LogLevel>(
                                            &format!("\"{}\"", level_str),
                                        ) {
                                            Ok(level) => {
                                                debug!("WebSocket log level filter set to {:?}", level);
                                                current_level_filter = Some(level);
                                            }
                                            Err(_) => {
                                                debug!("Invalid log level in filter: {}", level_str);
                                            }
                                        }
                                    }
                                }
                                Some("get_since") => {
                                    if let Some(ts) = cmd.get("timestamp").and_then(|v| v.as_u64()) {
                                        let logs = state.log_manager.get_logs_since(ts).await;
                                        for log in &logs {
                                            if let Ok(log_json) = serde_json::to_string(log) {
                                                if ws_sender
                                                    .send(axum::extract::ws::Message::Text(
                                                        Utf8Bytes::from(log_json),
                                                    ))
                                                    .await
                                                    .is_err()
                                                {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    debug!("Unknown WebSocket command type");
                                }
                            }
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(data))) => {
                        // Respond with pong (axum handles this automatically in most cases,
                        // but we handle it explicitly for safety)
                        let _ = ws_sender
                            .send(axum::extract::ws::Message::Pong(data))
                            .await;
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) => {
                        break;
                    }
                    Some(Err(e)) => {
                        debug!("WebSocket receive error: {}", e);
                        break;
                    }
                    None => {
                        // Stream ended – client disconnected
                        break;
                    }
                    _ => {
                        // Ignore Binary, Pong, etc.
                    }
                }
            }

            // 3. Periodic ping to keep connection alive
            _ = ping_interval.tick() => {
                if ws_sender
                    .send(axum::extract::ws::Message::Ping(Vec::new().into()))
                    .await
                    .is_err()
                {
                    break;
                }
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

async fn list_platforms(State(state): State<Arc<AppState>>) -> Json<Vec<PlatformInstanceDto>> {
    let configs = state.platform_configs.read().await;
    let statuses = state.platform_statuses_async().await;
    let list: Vec<PlatformInstanceDto> = configs
        .iter()
        .map(|c| PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            enable: c.enable,
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
            enable: c.enable,
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
        "dingtalk" | "discord" | "weixin_oc" | "onebot12" => {}
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
        enable: req.enable,
        extra: req.config,
    };

    configs.push(new_config);

    // Save to file (release write lock first)
    drop(configs);
    state.save_platforms_config().await.ok();

    // Ensure the adapter is started if enabled
    state.sync_platforms().await;

    // Return the created platform
    let statuses = state.platform_statuses_async().await;
    let configs = state.platform_configs.read().await;
    match configs.iter().find(|c| c.id == platform_id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            enable: c.enable,
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
    if let Some(enable) = req.enable {
        config.enable = enable;
    }
    if let Some(extra) = req.config {
        config.extra = extra;
    }

    // Get the updated ID for adapter management
    let new_id = config.id.clone();

    drop(configs);

    // Save config to file
    state.save_platforms_config().await.ok();

    // If ID changed, remove old adapter
    {
        let mut pm = state.platform_manager.write().await;
        if old_id != new_id && pm.is_running(&old_id) {
            if let Err(e) = pm.remove_platform(&old_id).await {
                tracing::warn!("Failed to stop old platform adapter {}: {}", old_id, e);
            }
        }
    }

    // Sync platform running state with enable state
    state.sync_platforms().await;

    let statuses = state.platform_statuses_async().await;
    let configs = state.platform_configs.read().await;
    match configs.iter().find(|c| c.id == new_id) {
        Some(c) => Json(PlatformInstanceDto {
            id: c.id.clone(),
            platform_type: c.platform_type.clone(),
            enable: c.enable,
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

    // Save main config first
    if let Err(e) = state.save_to_file(&state.config_path).await {
        tracing::warn!("Failed to save config before restart: {}", e);
    }

    // Shutdown all platform adapters gracefully first — this syncs
    // any updated credentials (e.g. from WeChat re-login) from the
    // API state into the adapter's config, so persist_config_hint()
    // will return the latest values.
    {
        let mut pm = state.platform_manager.write().await;
        pm.shutdown_all().await;
    }

    // Now persist any updated credentials from adapters
    state.persist_adapter_credentials().await;

    // Save platform configs (which now contain synced credentials)
    if let Err(e) = state.save_platforms_config().await {
        tracing::warn!("Failed to save platforms config before restart: {}", e);
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

    // Check that the platform is enabled
    let is_enabled = {
        let configs = state.platform_configs.read().await;
        configs
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.enable)
            .unwrap_or(false)
    };

    if !is_enabled {
        return (
            StatusCode::FAILED_DEPENDENCY,
            Json(json!({"error": "Platform is not enabled"})),
        )
            .into_response();
    }

    // Get proxy config from active profile
    let proxy_config = {
        let profiles = state.config_profiles.read().await;
        profiles
            .values()
            .filter(|p| p.is_active && p.enable)
            .find_map(|p| {
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
            enable: c.enable,
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

// ─── WeChat QR Login Handlers ──────────────────────────────────────

/// Start a WeChat QR code login session.
///
/// POST /api/platforms/:id/weixin-qr-login
async fn weixin_qr_login_start(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    use crate::platform::weixin_oc::api::WeixinApi;
    use crate::platform::weixin_oc::config::WeixinOcConfig;

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

    // Verify it's a weixin_oc platform
    if config.platform_type != "weixin_oc" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "QR login is only supported for weixin_oc platform type"})),
        )
            .into_response();
    }

    // Deserialize config into WeixinOcConfig
    let weixin_config: WeixinOcConfig = match serde_json::from_value(config.extra.clone()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid weixin_oc config: {}", e)})),
            )
                .into_response();
        }
    };

    // Create the API client and start QR login
    let api = match WeixinApi::new(weixin_config) {
        Ok(api) => api,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to create WeixinApi: {}", e)})),
            )
                .into_response();
        }
    };

    match api.qr_login_start().await {
        Ok(qr_resp) => {
            // Clean up any previous redirect host from a prior session
            {
                let mut configs = state.platform_configs.write().await;
                if let Some(cfg) = configs.iter_mut().find(|c| c.id == id) {
                    if let Some(obj) = cfg.extra.as_object_mut() {
                        obj.remove("_qr_redirect_host");
                    }
                }
            }

            // Generate a base64 PNG QR code image from the qrcode data string.
            // The qrcode_img_content from the API is a liteapp URL (a webpage, not an image),
            // so we generate a real QR code image from the qrcode field instead.
            let qrcode_img_b64 = {
                use qrcode::QrCode;
                match QrCode::new(&qr_resp.qrcode_img_content) {
                    Ok(code) => {
                        let png = code
                            .render::<qrcode::render::svg::Color>()
                            .min_dimensions(256, 256)
                            .build();
                        // Wrap SVG in a data URI
                        format!(
                            "data:image/svg+xml;base64,{}",
                            base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                png.as_bytes()
                            )
                        )
                    }
                    Err(e) => {
                        tracing::warn!("Failed to generate QR code image: {}, using raw URL", e);
                        qr_resp.qrcode_img_content.clone()
                    }
                }
            };

            Json(json!({
                "qrcode": qr_resp.qrcode,
                "qrcode_img_content": qrcode_img_b64,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("QR login start failed: {}", e)})),
        )
            .into_response(),
    }
}

/// Check WeChat QR code login status.
///
/// GET /api/platforms/:id/weixin-qr-status?qrcode=xxx
#[derive(Deserialize)]
struct WeixinQrStatusQuery {
    qrcode: String,
}

async fn weixin_qr_login_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<WeixinQrStatusQuery>,
) -> Response {
    use crate::platform::weixin_oc::api::WeixinApi;
    use crate::platform::weixin_oc::config::WeixinOcConfig;

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

    // Verify it's a weixin_oc platform
    if config.platform_type != "weixin_oc" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "QR login is only supported for weixin_oc platform type"})),
        )
            .into_response();
    }

    // Deserialize config into WeixinOcConfig
    let weixin_config: WeixinOcConfig = match serde_json::from_value(config.extra.clone()) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid weixin_oc config: {}", e)})),
            )
                .into_response();
        }
    };

    // Create the API client and poll QR status
    let api = match WeixinApi::new(weixin_config) {
        Ok(api) => api,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Failed to create WeixinApi: {}", e)})),
            )
                .into_response();
        }
    };

    // Apply any previously-stored IDC redirect host
    if let Some(redirect_host) = config
        .extra
        .get("_qr_redirect_host")
        .and_then(|v| v.as_str())
    {
        api.set_qr_redirect_url(redirect_host).await;
    }

    match api.qr_login_wait(&query.qrcode, 5000).await {
        Ok(status_resp) => {
            let mut status = status_resp.status.clone();

            // Handle scaned_but_redirect: switch to the redirect host and report as "scanned"
            // so the frontend keeps polling normally.
            if status == "scaned_but_redirect" {
                if let Some(ref host) = status_resp.redirect_host {
                    tracing::info!(
                        "IDC redirect for platform '{}', switching QR polling host to: {}",
                        id,
                        host
                    );
                    // Store the redirect host in the platform config so subsequent
                    // qr_login_wait calls use it.
                    {
                        let mut configs = state.platform_configs.write().await;
                        if let Some(cfg) = configs.iter_mut().find(|c| c.id == id) {
                            if let Some(obj) = cfg.extra.as_object_mut() {
                                obj.insert(
                                    "_qr_redirect_host".to_string(),
                                    serde_json::Value::String(host.clone()),
                                );
                            }
                        }
                    }
                }
                // From the frontend's perspective, this is equivalent to "scanned"
                status = "scanned".to_string();
            }

            // If confirmed, save token and account_id to the platform config
            if status == "confirmed" {
                if let (Some(token), Some(account_id)) =
                    (&status_resp.bot_token, &status_resp.ilink_bot_id)
                {
                    let mut configs = state.platform_configs.write().await;
                    if let Some(cfg) = configs.iter_mut().find(|c| c.id == id) {
                        if let Some(obj) = cfg.extra.as_object_mut() {
                            obj.insert(
                                "token".to_string(),
                                serde_json::Value::String(token.clone()),
                            );
                            obj.insert(
                                "account_id".to_string(),
                                serde_json::Value::String(account_id.clone()),
                            );
                            // Clean up temporary redirect host
                            obj.remove("_qr_redirect_host");
                        }
                    }
                    drop(configs);
                    state.save_platforms_config().await.ok();
                }
            }

            Json(json!({
                "status": status,
                "bot_token": status_resp.bot_token,
                "ilink_bot_id": status_resp.ilink_bot_id,
                "baseurl": status_resp.baseurl,
                "ilink_user_id": status_resp.ilink_user_id,
                "redirect_host": status_resp.redirect_host,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("QR status poll failed: {}", e)})),
        )
            .into_response(),
    }
}

// ─── Knowledge Base File Extraction Helpers ─────────────────────

/// Extract text content from an attached file in a chat message.
///
/// For text files the content is the raw text. For binary files (PDF, DOCX, XLSX, etc.)
/// the content is a base64 data-URL — we decode it and extract text.
/// Parse a data URL like `data:image/png;base64,iVBORw0KGgo...` into its
/// media type and base64 data components.
fn parse_data_url(data_url: &str) -> Option<(String, String)> {
    // Format: data:{media_type};base64,{data}
    if !data_url.starts_with("data:") {
        return None;
    }
    let rest = &data_url[5..]; // strip "data:"
    let semicolon = rest.find(';')?;
    let media_type = rest[..semicolon].to_string();
    let after_semicolon = &rest[semicolon + 1..];

    // Expect "base64," prefix
    if !after_semicolon.starts_with("base64,") {
        return None;
    }
    let data = after_semicolon["base64,".len()..].to_string();
    Some((media_type, data))
}

fn extract_attached_file_text(name: &str, mime_type: &str, content: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    // Text-based files: content is plain text
    if mime_type.starts_with("text/")
        || name_lower.ends_with(".txt")
        || name_lower.ends_with(".csv")
        || name_lower.ends_with(".md")
        || name_lower.ends_with(".markdown")
        || name_lower.ends_with(".json")
        || name_lower.ends_with(".xml")
        || name_lower.ends_with(".html")
        || name_lower.ends_with(".htm")
        || name_lower.ends_with(".yaml")
        || name_lower.ends_with(".yml")
        || name_lower.ends_with(".toml")
        || name_lower.ends_with(".ini")
        || name_lower.ends_with(".cfg")
        || name_lower.ends_with(".log")
        || name_lower.ends_with(".rs")
        || name_lower.ends_with(".py")
        || name_lower.ends_with(".js")
        || name_lower.ends_with(".ts")
        || name_lower.ends_with(".tsx")
        || name_lower.ends_with(".jsx")
        || name_lower.ends_with(".java")
        || name_lower.ends_with(".c")
        || name_lower.ends_with(".cpp")
        || name_lower.ends_with(".h")
        || name_lower.ends_with(".hpp")
        || name_lower.ends_with(".go")
        || name_lower.ends_with(".sh")
        || name_lower.ends_with(".bash")
        || name_lower.ends_with(".zsh")
        || name_lower.ends_with(".bat")
        || name_lower.ends_with(".ps1")
        || name_lower.ends_with(".sql")
        || name_lower.ends_with(".r")
        || name_lower.ends_with(".rb")
        || name_lower.ends_with(".php")
        || name_lower.ends_with(".swift")
        || name_lower.ends_with(".kt")
        || name_lower.ends_with(".scala")
        || name_lower.ends_with(".lua")
        || name_lower.ends_with(".pl")
        || name_lower.ends_with(".css")
        || name_lower.ends_with(".scss")
        || name_lower.ends_with(".less")
        || name_lower.ends_with(".sass")
        || name_lower.ends_with(".env")
        || name_lower.ends_with(".gitignore")
        || name_lower.ends_with(".dockerfile")
        || name_lower.ends_with(".makefile")
    {
        // Truncate very large text content
        let truncated = if content.len() > 100_000 {
            format!(
                "{}\n\n... (truncated, original size: {} bytes)",
                &content[..content.floor_char_boundary(100_000)],
                content.len()
            )
        } else {
            content.to_string()
        };
        return Some(truncated);
    }

    // Binary files: content is a base64 data-URL — decode and extract
    let data = if content.starts_with("data:") {
        // data-URL format: data:<mime>;base64,<payload>
        if let Some(idx) = content.find(",") {
            use base64::Engine;
            let payload = &content[idx + 1..];
            match base64::engine::general_purpose::STANDARD.decode(payload) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(file = %name, error = %e, "Failed to decode base64 data-URL");
                    return None;
                }
            }
        } else {
            tracing::warn!(file = %name, "Invalid data-URL format");
            return None;
        }
    } else {
        // Not a data-URL, skip
        return None;
    };

    // Extract based on file extension
    if name_lower.ends_with(".pdf") {
        match pdf_extract::extract_text_from_mem(&data) {
            Ok(text) => {
                let truncated = if text.len() > 100_000 {
                    format!(
                        "{}\n\n... (truncated)",
                        &text[..text.floor_char_boundary(100_000)]
                    )
                } else {
                    text
                };
                return Some(truncated);
            }
            Err(e) => {
                tracing::warn!(file = %name, error = %e, "Failed to extract text from PDF");
                return None;
            }
        }
    } else if name_lower.ends_with(".xlsx") || name_lower.ends_with(".xls") {
        match extract_excel_text(&data) {
            Ok(text) => {
                let truncated = if text.len() > 100_000 {
                    format!(
                        "{}\n\n... (truncated)",
                        &text[..text.floor_char_boundary(100_000)]
                    )
                } else {
                    text
                };
                return Some(truncated);
            }
            Err(e) => {
                tracing::warn!(file = %name, error = %e, "Failed to extract text from Excel");
                return None;
            }
        }
    } else if name_lower.ends_with(".docx") {
        match extract_docx_text(&data) {
            Ok(text) => {
                let truncated = if text.len() > 100_000 {
                    format!(
                        "{}\n\n... (truncated)",
                        &text[..text.floor_char_boundary(100_000)]
                    )
                } else {
                    text
                };
                return Some(truncated);
            }
            Err(e) => {
                tracing::warn!(file = %name, error = %e, "Failed to extract text from DOCX");
                return None;
            }
        }
    } else if name_lower.ends_with(".rtf") {
        // Basic RTF: just strip control words and return plain text
        let text = strip_rtf_text(std::str::from_utf8(&data).unwrap_or(""));
        if !text.is_empty() {
            return Some(text);
        }
    }

    // Unsupported binary format — skip silently
    None
}

/// Naive RTF text extraction: strips control words and returns the remaining text.
fn strip_rtf_text(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut in_control = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                in_control = true;
            }
            '{' | '}' => {
                // Group delimiters — skip
                in_control = false;
            }
            '\n' | '\r' => {
                in_control = false;
            }
            ' ' if in_control => {
                in_control = false;
            }
            _ if in_control => {
                // Part of a control word, skip
            }
            _ => {
                result.push(c);
            }
        }
    }
    result
}

/// Extract text content from an Excel file (xls or xlsx) using calamine.
fn extract_excel_text(data: &[u8]) -> Result<String, anyhow::Error> {
    use calamine::{Data, Reader, open_workbook_auto_from_rs};
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let mut workbook = open_workbook_auto_from_rs(cursor)?;

    let mut text = String::new();
    let sheets = workbook.sheet_names().to_vec();

    for sheet_name in &sheets {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&format!("--- Sheet: {} ---\n", sheet_name));

        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let mut row_count = 0u32;
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => {
                            // Show integers without decimal point
                            if *f == (*f as i64) as f64 {
                                (*f as i64).to_string()
                            } else {
                                f.to_string()
                            }
                        }
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => dt.to_string(),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("ERR:{:?}", e),
                        Data::Empty => String::new(),
                    })
                    .collect();
                text.push_str(&cells.join("\t"));
                text.push('\n');
                row_count += 1;
                // Limit to prevent extremely large extractions
                if row_count >= 10000 {
                    text.push_str("... (truncated after 10000 rows)\n");
                    break;
                }
            }
        }
    }

    Ok(text)
}

/// Extract text content from a DOCX file using docx-rs.
fn extract_docx_text(data: &[u8]) -> Result<String, anyhow::Error> {
    use docx_rs::{DocumentChild, ParagraphChild, RunChild, read_docx};

    let docx = read_docx(data)?;
    let mut text = String::new();

    for child in &docx.document.children {
        match child {
            DocumentChild::Paragraph(paragraph) => {
                let mut para_text = String::new();
                for pchild in &paragraph.children {
                    match pchild {
                        ParagraphChild::Run(run) => {
                            for rchild in &run.children {
                                if let RunChild::Text(t) = rchild {
                                    if !para_text.is_empty() {
                                        para_text.push(' ');
                                    }
                                    para_text.push_str(&t.text);
                                }
                            }
                        }
                        ParagraphChild::Hyperlink(hyperlink) => {
                            for pchild in &hyperlink.children {
                                if let ParagraphChild::Run(run) = pchild {
                                    for rchild in &run.children {
                                        if let RunChild::Text(t) = rchild {
                                            if !para_text.is_empty() {
                                                para_text.push(' ');
                                            }
                                            para_text.push_str(&t.text);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if !para_text.is_empty() {
                    text.push_str(&para_text);
                    text.push('\n');
                }
            }
            DocumentChild::Table(table) => {
                // Extract text from table cells
                if !text.is_empty() {
                    text.push('\n');
                }
                for row_child in &table.rows {
                    let mut row_texts = Vec::new();
                    let docx_rs::TableChild::TableRow(row) = row_child;
                    for cell_child in &row.cells {
                        let docx_rs::TableRowChild::TableCell(cell) = cell_child;
                        let mut cell_text = String::new();
                        for content in &cell.children {
                            if let docx_rs::TableCellContent::Paragraph(para) = content {
                                for pchild in &para.children {
                                    if let ParagraphChild::Run(run) = pchild {
                                        for rchild in &run.children {
                                            if let RunChild::Text(t) = rchild {
                                                if !cell_text.is_empty() {
                                                    cell_text.push(' ');
                                                }
                                                cell_text.push_str(&t.text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        row_texts.push(cell_text);
                    }
                    text.push_str(&row_texts.join("\t"));
                    text.push('\n');
                }
            }
            _ => {}
        }
    }

    Ok(text)
}

// ─── Knowledge Base ──────────────────────────────────────────────

async fn list_knowledge_bases(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<KnowledgeBaseDto>>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let kbs = service.list_knowledge_bases().await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(Json(kbs.into_iter().map(KnowledgeBaseDto::from).collect()))
}

async fn get_knowledge_base(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<KnowledgeBaseDto>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let kb = service.get_knowledge_base(&id).await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(Json(KnowledgeBaseDto::from(kb)))
}

async fn create_knowledge_base(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateKnowledgeBaseRequest>,
) -> Result<Json<KnowledgeBaseDto>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let kb_req = crate::knowledge::CreateKnowledgeBaseRequest {
        name: req.name,
        description: req.description,
        embedding_provider_config: req.embedding_provider_config.into(),
        rerank_provider_config: req.rerank_provider_config.map(Into::into),
        chunk_size: Some(req.chunk_size),
        chunk_overlap: Some(req.chunk_overlap),
    };
    let kb = service.create_knowledge_base(kb_req).await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(Json(KnowledgeBaseDto::from(kb)))
}

async fn update_knowledge_base(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateKnowledgeBaseRequestDto>,
) -> Result<Json<KnowledgeBaseDto>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let kb_req = crate::knowledge::UpdateKnowledgeBaseRequest {
        name: req.name,
        description: req.description,
        rerank_provider_config: req.rerank_provider_config.map(|opt| opt.map(Into::into)),
        chunk_size: req.chunk_size,
        chunk_overlap: req.chunk_overlap,
    };
    let kb = service
        .update_knowledge_base(&id, kb_req)
        .await
        .map_err(|e| {
            error!("{e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("{e:#}") })),
            )
        })?;
    Ok(Json(KnowledgeBaseDto::from(kb)))
}

async fn delete_knowledge_base(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    service.delete_knowledge_base(&id).await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_kb_documents(
    State(state): State<Arc<AppState>>,
    Path(kb_id): Path<String>,
) -> Result<Json<Vec<KbDocumentDto>>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let docs = service.list_documents(&kb_id).await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(Json(docs.into_iter().map(KbDocumentDto::from).collect()))
}

async fn upload_kb_document(
    State(state): State<Arc<AppState>>,
    Path(kb_id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Vec<KbDocumentDto>>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };

    let mut results = Vec::new();
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })? {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let data = field.bytes().await.map_err(|e| {
            error!("{e:#}");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("{e:#}") })),
            )
        })?;

        let filename_lower = filename.to_lowercase();
        let content = if filename_lower.ends_with(".pdf") {
            // PDF file – extract text content using pdf-extract
            tracing::info!(filename = %filename, "Extracting text from PDF file");
            match pdf_extract::extract_text_from_mem(&data) {
                Ok(text) => {
                    tracing::info!(
                        filename = %filename,
                        text_len = text.len(),
                        "PDF text extraction succeeded"
                    );
                    text
                }
                Err(e) => {
                    tracing::error!(
                        filename = %filename,
                        error = %e,
                        "Failed to extract text from PDF"
                    );
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!(
                            "Failed to extract text from PDF '{}': {}. The PDF may be an image-based scan (OCR required) or use an unsupported encoding.",
                            filename, e
                        ) })),
                    ));
                }
            }
        } else if filename_lower.ends_with(".xlsx") || filename_lower.ends_with(".xls") {
            // Excel file – extract text content using calamine
            tracing::info!(filename = %filename, "Extracting text from Excel file");
            match extract_excel_text(&data) {
                Ok(text) => {
                    tracing::info!(
                        filename = %filename,
                        text_len = text.len(),
                        "Excel text extraction succeeded"
                    );
                    text
                }
                Err(e) => {
                    tracing::error!(
                        filename = %filename,
                        error = %e,
                        "Failed to extract text from Excel file"
                    );
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!(
                            "Failed to extract text from Excel '{}': {}",
                            filename, e
                        ) })),
                    ));
                }
            }
        } else if filename_lower.ends_with(".docx") {
            // DOCX file – extract text content using docx-rs
            tracing::info!(filename = %filename, "Extracting text from DOCX file");
            match extract_docx_text(&data) {
                Ok(text) => {
                    tracing::info!(
                        filename = %filename,
                        text_len = text.len(),
                        "DOCX text extraction succeeded"
                    );
                    text
                }
                Err(e) => {
                    tracing::error!(
                        filename = %filename,
                        error = %e,
                        "Failed to extract text from DOCX file"
                    );
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!(
                            "Failed to extract text from DOCX '{}': {}",
                            filename, e
                        ) })),
                    ));
                }
            }
        } else {
            // Non-binary file – try UTF-8 decoding
            match String::from_utf8(data.to_vec()) {
                Ok(s) => s,
                Err(e) => {
                    // The file contains non-UTF-8 bytes – likely a binary file
                    // (PPT, ZIP, etc.) or text in a non-UTF-8 encoding (GBK, etc.).
                    let is_likely_binary = filename_lower.ends_with(".doc")
                        || filename_lower.ends_with(".ppt")
                        || filename_lower.ends_with(".pptx")
                        || filename_lower.ends_with(".zip")
                        || filename_lower.ends_with(".rar")
                        || filename_lower.ends_with(".7z");

                    if is_likely_binary {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            Json(json!({ "error": format!(
                            "File '{}' appears to be a binary format. Please upload plain text files (txt, md, csv), PDF, Excel (xls/xlsx), or DOCX files instead.",
                            filename
                        ) })),
                        ));
                    }

                    tracing::warn!(
                        filename = %filename,
                        invalid_utf8_bytes = e.utf8_error().valid_up_to(),
                        "File contains non-UTF-8 bytes, converting with lossy replacement"
                    );
                    String::from_utf8_lossy(&data).to_string()
                }
            }
        };

        let doc = service
            .upload_document(&kb_id, &filename, &content)
            .await
            .map_err(|e| {
                error!("{e:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("{e:#}") })),
                )
            })?;
        results.push(KbDocumentDto::from(doc));
    }
    Ok(Json(results))
}

async fn delete_kb_document(
    State(state): State<Arc<AppState>>,
    Path((_kb_id, doc_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    service.delete_document(&doc_id).await.map_err(|e| {
        error!("{e:#}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("{e:#}") })),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn search_knowledge_base(
    State(state): State<Arc<AppState>>,
    Path(kb_id): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<SearchResultDto>>, (StatusCode, Json<Value>)> {
    let kb_service = state.knowledge_base_service.read().await;
    let service = match kb_service.as_ref() {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Knowledge base service is not available" })),
            ));
        }
    };
    let results = service
        .search(&kb_id, &req.query, req.top_k, 0, None)
        .await
        .map_err(|e| {
            error!("{e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("{e:#}") })),
            )
        })?;
    Ok(Json(
        results.into_iter().map(SearchResultDto::from).collect(),
    ))
}

// ─── Debug Session Handlers ─────────────────────────────────────

/// Get the current debug session configuration
async fn get_debug_session(State(state): State<Arc<AppState>>) -> Json<DebugSessionDto> {
    let session = state.debug_session.read().await;

    let dto = DebugSessionDto {
        persona_id: session.persona_id.clone(),
        embedded_persona: session
            .embedded_persona
            .as_ref()
            .map(EmbeddedPersonaDto::from),
        providers: session.providers.iter().map(Into::into).collect(),
        active_provider: session.active_provider.clone(),
        provider_id: session.provider_id.clone(),
        temperature: session.temperature,
        max_tokens: session.max_tokens,
        custom_error_message: session.custom_error_message.clone(),
        knowledge_base_ids: session.knowledge_base_ids.clone(),
        skills: session.skills.iter().map(Into::into).collect(),
        active_skill_names: session.active_skill_names.clone(),
        command_prefix: session.command_prefix.clone(),
        enabled_commands: session.enabled_commands.clone(),
    };

    Json(dto)
}

/// Update the debug session configuration
async fn update_debug_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateDebugSessionRequest>,
) -> Result<Json<DebugSessionDto>, (StatusCode, Json<Value>)> {
    // Apply updates and build the DTO inside the write lock
    let dto = {
        let mut session = state.debug_session.write().await;

        if let Some(embedded_persona) = req.embedded_persona {
            session.embedded_persona =
                embedded_persona.map(|dto| crate::api::state::EmbeddedPersona::from(&dto));
        }
        if let Some(persona_id) = req.persona_id {
            session.persona_id = persona_id;
        }
        if let Some(providers) = req.providers {
            session.providers = providers
                .iter()
                .map(|p| crate::api::state::EmbeddedProvider::from(p))
                .collect();
        }
        if let Some(active_provider) = req.active_provider {
            session.active_provider = active_provider;
        }
        if let Some(provider_id) = req.provider_id {
            session.provider_id = provider_id;
        }
        if let Some(temperature) = req.temperature {
            session.temperature = temperature;
        }
        if let Some(max_tokens) = req.max_tokens {
            session.max_tokens = max_tokens;
        }
        if let Some(custom_error_message) = req.custom_error_message {
            session.custom_error_message = custom_error_message;
        }
        if let Some(knowledge_base_ids) = req.knowledge_base_ids {
            session.knowledge_base_ids = knowledge_base_ids;
        }
        if let Some(skills) = req.skills {
            session.skills = skills
                .iter()
                .map(|s| crate::api::state::EmbeddedSkill::from(s))
                .collect();
        }
        if let Some(active_skill_names) = req.active_skill_names {
            session.active_skill_names = active_skill_names;
        }
        if let Some(command_prefix) = req.command_prefix {
            session.command_prefix = command_prefix;
        }
        if let Some(enabled_commands) = req.enabled_commands {
            session.enabled_commands = enabled_commands;
        }

        // Build DTO while still holding the lock, then drop the lock
        let dto = DebugSessionDto {
            persona_id: session.persona_id.clone(),
            embedded_persona: session
                .embedded_persona
                .as_ref()
                .map(EmbeddedPersonaDto::from),
            providers: session
                .providers
                .iter()
                .map(|p| EmbeddedProviderDto::from(p))
                .collect(),
            active_provider: session.active_provider.clone(),
            provider_id: session.provider_id.clone(),
            temperature: session.temperature,
            max_tokens: session.max_tokens,
            custom_error_message: session.custom_error_message.clone(),
            knowledge_base_ids: session.knowledge_base_ids.clone(),
            skills: session
                .skills
                .iter()
                .map(|s| EmbeddedSkillDto::from(s))
                .collect(),
            active_skill_names: session.active_skill_names.clone(),
            command_prefix: session.command_prefix.clone(),
            enabled_commands: session.enabled_commands.clone(),
        };

        dto
    }; // write lock is dropped here

    // Now save to file — safe because the write lock has been released
    state.save_debug_session().await;

    tracing::info!("Debug session configuration updated");

    Ok(Json(dto))
}
