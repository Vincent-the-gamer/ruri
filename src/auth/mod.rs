//! Authentication module for WebUI
//!
//! Provides session-based authentication with password hashing via Argon2.
//! Features:
//! - Login/logout endpoints
//! - Session management with tokens stored in cookies
//! - Password change with "must change on first login" flag
//! - Authentication middleware for protecting API routes

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Json;
use chrono;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tracing::info;

use uuid::Uuid;

use crate::api::state::AppState;
use crate::db;

// ─── Cookie helpers ──────────────────────────────────────────────

/// Extract session token from the Cookie header
fn get_session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            if let Some((key, value)) = cookie.split_once('=') {
                if key.trim() == "session_token" {
                    return Some(value.trim().to_string());
                }
            }
            None
        })
}

/// Create a Set-Cookie header value
fn set_cookie_header(name: &str, value: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax",
        name, value
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// Create an expired cookie to clear it
fn clear_cookie_header(name: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("{}=; Max-Age=0", name))
        .unwrap_or_else(|_| HeaderValue::from_static(""))
}

// ─── Models ──────────────────────────────────────────────────────

/// Request body for login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Request body for password change
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// Request body for username update
#[derive(Debug, Deserialize)]
pub struct UpdateUsernameRequest {
    pub new_username: String,
}

/// User info returned to the client (does NOT include sensitive data)
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub must_change_password: bool,
}

/// Login response with session token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// A stored user record from the database
#[derive(Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub must_change_password: bool,
}

// ─── Session Database Operations ─────────────────────────────────

/// Session validity period in hours (7 days).
const SESSION_EXPIRY_HOURS: i64 = 168;

/// Create a new session in the database and return the token.
async fn create_session(pool: &sqlx::SqlitePool, user_id: &str) -> Result<String, sqlx::Error> {
    let token = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(SESSION_EXPIRY_HOURS))
        .unwrap()
        .to_rfc3339();

    sqlx::query(
        r#"INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)"#,
    )
    .bind(&token)
    .bind(user_id)
    .bind(&now)
    .bind(&expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

/// Validate a session token and return the associated user ID if valid and not expired.
async fn validate_session(
    pool: &sqlx::SqlitePool,
    token: &str,
) -> Result<Option<String>, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    let row: Option<(String,)> =
        sqlx::query_as(r#"SELECT user_id FROM sessions WHERE token = ? AND expires_at > ?"#)
            .bind(token)
            .bind(&now)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|r| r.0))
}

/// Remove a session from the database.
async fn remove_session(pool: &sqlx::SqlitePool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query(r#"DELETE FROM sessions WHERE token = ?"#)
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

// ─── Database helpers ────────────────────────────────────────────

/// Fetch a user by username from the database.
pub async fn get_user_by_username(
    pool: &sqlx::SqlitePool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash, must_change_password
        FROM users
        WHERE username = ?
        "#,
    )
    .bind(username)
    .map(|r: sqlx::sqlite::SqliteRow| User {
        id: r.try_get(0).unwrap_or_default(),
        username: r.try_get(1).unwrap_or_default(),
        password_hash: r.try_get(2).unwrap_or_default(),
        must_change_password: r.try_get(3).unwrap_or(false),
    })
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

/// Update a user's password hash.
pub async fn update_user_password(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = ?, must_change_password = 0, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(password_hash)
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update a user's username.
pub async fn update_user_username(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    new_username: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE users
        SET username = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(new_username)
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_user_by_id(
    pool: &sqlx::SqlitePool,
    user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash, must_change_password
        FROM users
        WHERE id = ?
        "#,
    )
    .bind(user_id)
    .map(|r: sqlx::sqlite::SqliteRow| User {
        id: r.try_get(0).unwrap_or_default(),
        username: r.try_get(1).unwrap_or_default(),
        password_hash: r.try_get(2).unwrap_or_default(),
        must_change_password: r.try_get(3).unwrap_or(false),
    })
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

// ─── Error type ──────────────────────────────────────────────────

#[derive(Debug)]
pub struct AuthError {
    pub status: StatusCode,
    pub message: String,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

// ─── Handlers ────────────────────────────────────────────────────

/// POST /api/auth/login
pub async fn login(
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let pool = state.db_pool.read().await;
    let pool = pool.as_ref().ok_or(AuthError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "Database not available".to_string(),
    })?;

    let user = get_user_by_username(pool, &req.username)
        .await
        .map_err(|e| {
            tracing::error!("Database error during login: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid username or password".to_string(),
        })?;

    // Verify password
    if !db::verify_password(&req.password, &user.password_hash) {
        return Err(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid username or password".to_string(),
        });
    }

    // Create session
    let token = create_session(pool, &user.id).await.map_err(|e| {
        tracing::error!("Failed to create session: {}", e);
        AuthError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Internal server error".to_string(),
        }
    })?;

    info!(
        user_id = %user.id,
        username = %user.username,
        "User logged in successfully"
    );

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        must_change_password: user.must_change_password,
    };

    // Build response with Set-Cookie header
    let mut res = Response::new(Body::from(
        serde_json::to_string(&LoginResponse {
            token: token.clone(),
            user: user_info,
        })
        .unwrap_or_default(),
    ));

    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        set_cookie_header("session_token", &token),
    );
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok(res)
}

/// POST /api/auth/logout
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AuthError> {
    let token = get_session_token(&headers);

    if let Some(token) = token {
        let pool = state.db_pool.read().await;
        if let Some(pool) = pool.as_ref() {
            let _ = remove_session(pool, &token).await;
            info!("Session logged out");
        }
    }

    // Build response with cleared cookie
    let mut res = Response::new(Body::from(
        serde_json::to_string(&serde_json::json!({ "message": "Logged out successfully" }))
            .unwrap_or_default(),
    ));

    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        clear_cookie_header("session_token"),
    );
    res.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    Ok(res)
}

/// GET /api/auth/me
pub async fn get_current_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AuthError> {
    let token = get_session_token(&headers).ok_or(AuthError {
        status: StatusCode::UNAUTHORIZED,
        message: "Not authenticated".to_string(),
    })?;

    let pool = state.db_pool.read().await;
    let pool = pool.as_ref().ok_or(AuthError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "Database not available".to_string(),
    })?;

    let user_id = validate_session(pool, &token)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid session".to_string(),
        })?;

    let user = get_user_by_id(pool, &user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "User not found".to_string(),
        })?;

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        must_change_password: user.must_change_password,
    };

    Ok((StatusCode::OK, Json(user_info)))
}

/// POST /api/auth/change-password
pub async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let token = get_session_token(&headers).ok_or(AuthError {
        status: StatusCode::UNAUTHORIZED,
        message: "Not authenticated".to_string(),
    })?;

    let pool = state.db_pool.read().await;
    let pool = pool.as_ref().ok_or(AuthError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "Database not available".to_string(),
    })?;

    let user_id = validate_session(pool, &token)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid session".to_string(),
        })?;

    // Get current user
    let user = get_user_by_id(pool, &user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "User not found".to_string(),
        })?;

    // Verify old password
    if !db::verify_password(&req.old_password, &user.password_hash) {
        return Err(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Current password is incorrect".to_string(),
        });
    }

    // Hash new password
    let new_hash = db::hash_password(&req.new_password).map_err(|e| {
        tracing::error!("Failed to hash new password: {}", e);
        AuthError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Failed to update password".to_string(),
        }
    })?;

    // Update password in database
    update_user_password(pool, &user.id, &new_hash)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update password: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to update password".to_string(),
            }
        })?;

    info!(
        user_id = %user.id,
        "Password changed successfully"
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Password changed successfully" })),
    ))
}

/// POST /api/auth/update-username
pub async fn update_username(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateUsernameRequest>,
) -> Result<impl IntoResponse, AuthError> {
    let token = get_session_token(&headers).ok_or(AuthError {
        status: StatusCode::UNAUTHORIZED,
        message: "Not authenticated".to_string(),
    })?;

    let pool = state.db_pool.read().await;
    let pool = pool.as_ref().ok_or(AuthError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        message: "Database not available".to_string(),
    })?;

    let user_id = validate_session(pool, &token)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Internal server error".to_string(),
            }
        })?
        .ok_or(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid session".to_string(),
        })?;

    // Validate username is not empty
    if req.new_username.trim().is_empty() {
        return Err(AuthError {
            status: StatusCode::BAD_REQUEST,
            message: "Username cannot be empty".to_string(),
        });
    }

    // Check if the new username is already taken by another user
    let existing_user = get_user_by_username(pool, &req.new_username.trim())
        .await
        .map_err(|e| {
            tracing::error!("Failed to check username: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to check username".to_string(),
            }
        })?;

    if let Some(existing) = existing_user {
        if existing.id != user_id {
            return Err(AuthError {
                status: StatusCode::CONFLICT,
                message: "Username already exists".to_string(),
            });
        }
        // Username unchanged, treat as success
        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({ "message": "Username updated successfully" })),
        ));
    }

    // Update username in database
    update_user_username(pool, &user_id, &req.new_username.trim())
        .await
        .map_err(|e| {
            tracing::error!("Failed to update username: {}", e);
            AuthError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to update username".to_string(),
            }
        })?;

    info!(
        user_id = %user_id,
        new_username = %req.new_username,
        "Username updated successfully"
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "message": "Username updated successfully" })),
    ))
}

// ─── Middleware ──────────────────────────────────────────────────

/// Authentication middleware that checks for a valid session token.
/// Returns 401 Unauthorized if the session is invalid or missing.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, AuthError> {
    let token = get_session_token(request.headers());

    match token {
        Some(token) => {
            let pool = state.db_pool.read().await;
            let is_valid = if let Some(pool) = pool.as_ref() {
                validate_session(pool, &token)
                    .await
                    .unwrap_or_default()
                    .is_some()
            } else {
                false
            };

            if is_valid {
                // Session is valid, continue to handler
                let response = next.run(request).await;
                Ok(response)
            } else {
                Err(AuthError {
                    status: StatusCode::UNAUTHORIZED,
                    message: "Invalid session".to_string(),
                })
            }
        }
        None => Err(AuthError {
            status: StatusCode::UNAUTHORIZED,
            message: "Not authenticated".to_string(),
        }),
    }
}

// ─── Router Builder ──────────────────────────────────────────────

/// Build the auth router with public and protected routes.
pub fn create_auth_router(state: Arc<AppState>) -> axum::Router {
    use axum::routing::{get, post};

    // Public auth routes (no auth required)
    let public_routes = axum::Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(get_current_user))
        .with_state(state.clone());

    // Auth management routes (require authentication)
    let auth_routes = axum::Router::new()
        .route("/api/auth/change-password", post(change_password))
        .route("/api/auth/update-username", post(update_username))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state.clone());

    public_routes.merge(auth_routes)
}
