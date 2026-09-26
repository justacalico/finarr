//! Login/logout/me + password changes.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::{self, password, AuthUser};
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, display_name, role, disabled
           FROM users WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_optional(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    // Verify against a fixed dummy hash for unknown users so response time
    // doesn't reveal whether the account exists.
    static DUMMY: &str =
        "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let (row, hash): (Option<sqlx::sqlite::SqliteRow>, String) = match row {
        Some(r) => {
            let hash = r.get::<String, _>("password_hash");
            (Some(r), hash)
        }
        None => (None, DUMMY.to_string()),
    };
    if !password::verify_password(&body.password, &hash) || row.is_none() {
        return Err(ApiError::new(
            axum::http::StatusCode::UNAUTHORIZED,
            "invalid username or password",
        ));
    }
    let row = row.unwrap();
    if row.get::<i64, _>("disabled") != 0 {
        return Err(ApiError::forbidden("account disabled"));
    }
    let user_id: i64 = row.get("id");
    sqlx::query(
        "UPDATE users SET last_login_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?",
    )
    .bind(user_id)
    .execute(state.db.pool())
    .await
    .ok();
    let token = auth::create_session(&state.db, user_id, "web session", "session", Some(90))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "token": token,
        "user": {
            "id": user_id,
            "username": row.get::<String, _>("username"),
            "display_name": row.get::<String, _>("display_name"),
            "role": row.get::<String, _>("role"),
        }
    })))
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(str::to_string)
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if let Some(token) = bearer(&headers) {
        auth::revoke_token(&state.db, &token)
            .await
            .map_err(ApiError::internal)?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn me(user: AuthUser) -> Json<Value> {
    Json(json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "role": user.role,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PasswordBody {
    pub current: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<PasswordBody>,
) -> Result<Json<Value>, ApiError> {
    if body.new_password.len() < 8 {
        return Err(ApiError::bad_request(
            "password must be at least 8 characters",
        ));
    }
    let row = sqlx::query("SELECT password_hash FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    let hash: String = row.get("password_hash");
    if !password::verify_password(&body.current, &hash) {
        return Err(ApiError::bad_request("current password is wrong"));
    }
    let new_hash = password::hash_password(&body.new_password).map_err(ApiError::internal)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_hash)
        .bind(user.id)
        .execute(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    // A compromised-password rotation has to kill the sessions it created.
    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND kind = 'session'")
        .bind(user.id)
        .execute(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}
