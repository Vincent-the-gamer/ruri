use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::models::*;
use crate::api::state::AppState;

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
