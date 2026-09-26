//! Token auth: bearer sessions and long-lived API keys.

pub mod password;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::db::Db;
use crate::error::ApiError;

/// Random session/API token. The raw value is only ever shown once at
/// creation; the DB keeps a SHA-256 digest.
pub fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

async fn lookup_token(db: &Db, token: &str) -> Result<Option<AuthUser>, sqlx::Error> {
    let digest = hash_token(token);
    let row = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.role, u.disabled, s.id AS sid,
                s.expires_at
           FROM sessions s JOIN users u ON u.id = s.user_id
          WHERE s.token_hash = ?",
    )
    .bind(&digest)
    .fetch_optional(db.pool())
    .await?;
    let Some(row) = row else { return Ok(None) };
    if row.get::<i64, _>("disabled") != 0 {
        return Ok(None);
    }
    if let Some(exp) = row
        .try_get::<Option<String>, _>("expires_at")
        .ok()
        .flatten()
    {
        if exp
            <= chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()
        {
            return Ok(None);
        }
    }
    let sid: i64 = row.get("sid");
    let _ = sqlx::query(
        "UPDATE sessions SET last_seen_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?",
    )
    .bind(sid)
    .execute(db.pool())
    .await;
    Ok(Some(AuthUser {
        id: row.get("id"),
        username: row.get("username"),
        display_name: row.get("display_name"),
        role: row.get("role"),
    }))
}

fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(hv) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        let v = hv.to_str().unwrap_or("");
        if let Some(t) = v
            .strip_prefix("Bearer ")
            .or_else(|| v.strip_prefix("bearer "))
        {
            return Some(t.trim().to_string());
        }
    }
    if let Some(hv) = parts.headers.get("x-api-key") {
        return Some(hv.to_str().unwrap_or("").trim().to_string());
    }
    if let Some(q) = parts.uri.query() {
        for pair in q.split('&') {
            if let Some(v) = pair.strip_prefix("apikey=") {
                return Some(urlencoding::decode(v).ok()?.into_owned());
            }
        }
    }
    None
}

impl FromRequestParts<std::sync::Arc<crate::AppState>> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &std::sync::Arc<crate::AppState>,
    ) -> Result<Self, Self::Rejection> {
        if state.config.dev_mode {
            return Ok(AuthUser {
                id: 0,
                username: "local".into(),
                display_name: "Local".into(),
                role: "admin".into(),
            });
        }
        let token = extract_token(parts).ok_or_else(ApiError::unauthorized)?;
        match lookup_token(&state.db, &token).await {
            Ok(Some(u)) => Ok(u),
            Ok(None) => Err(ApiError::unauthorized()),
            Err(e) => Err(ApiError::internal(e)),
        }
    }
}

/// Requires the `admin` role.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl FromRequestParts<std::sync::Arc<crate::AppState>> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &std::sync::Arc<crate::AppState>,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_admin() {
            return Err(ApiError::forbidden("admin role required"));
        }
        Ok(Self(user))
    }
}

/// Create a session row and return the raw token.
pub async fn create_session(
    db: &Db,
    user_id: i64,
    name: &str,
    kind: &str,
    days_valid: Option<i64>,
) -> anyhow::Result<String> {
    let token = generate_token();
    let expires = days_valid.map(|d| {
        (chrono::Utc::now() + chrono::Duration::days(d))
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    });
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, name, kind, expires_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(hash_token(&token))
    .bind(user_id)
    .bind(name)
    .bind(kind)
    .bind(expires)
    .execute(db.pool())
    .await?;
    Ok(token)
}

pub async fn revoke_token(db: &Db, token: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(hash_token(token))
        .execute(db.pool())
        .await?;
    Ok(())
}
