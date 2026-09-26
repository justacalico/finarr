//! Push notifications: ntfy.sh, Discord webhooks, Gotify, or a generic
//! JSON webhook. Failures are logged and swallowed — notifications must
//! never break the pipeline.

use serde_json::json;

use crate::settings::NotificationSettings;

pub async fn send(
    http: &reqwest::Client,
    cfg: &NotificationSettings,
    event: &str,
    title: &str,
    body: &str,
) {
    if !cfg.enabled() || (!cfg.events.is_empty() && !cfg.events.iter().any(|e| e == event)) {
        return;
    }
    let kind = cfg.kind.as_str();
    let res = match kind {
        "ntfy" => {
            http.post(&cfg.url)
                .header("Title", title)
                .header("Priority", cfg.priority.max(1).min(5).to_string())
                .header("Tags", event)
                .body(body.to_string())
                .send()
                .await
        }
        "discord" => {
            http.post(&cfg.url)
                .json(&json!({
                    "username": "Finarr",
                    "embeds": [{
                        "title": title,
                        "description": body,
                        "color": 0x3b82f6u32,
                    }]
                }))
                .send()
                .await
        }
        "gotify" => {
            http.post(format!("{}/message", cfg.url.trim_end_matches('/')))
                .header("X-Gotify-Key", &cfg.token)
                .json(&json!({
                    "title": title,
                    "message": body,
                    "priority": cfg.priority,
                }))
                .send()
                .await
        }
        _ => {
            http.post(&cfg.url)
                .json(&json!({
                    "event": event,
                    "title": title,
                    "body": body,
                }))
                .send()
                .await
        }
    };
    match res {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => tracing::warn!("notification to {kind} failed: {}", r.status()),
        Err(e) => tracing::warn!("notification to {kind} failed: {e:#}"),
    }
}
