//! In-memory log ring buffer so the Web UI can show recent server logs.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

const CAPACITY: usize = 2000;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

static BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

pub fn recent(limit: usize) -> Vec<LogEntry> {
    let buf = buffer().lock().unwrap();
    buf.iter().rev().take(limit).cloned().collect()
}

fn push(entry: LogEntry) {
    let mut buf = buffer().lock().unwrap();
    if buf.len() >= CAPACITY {
        buf.pop_front();
    }
    buf.push_back(entry);
}

struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else if self.message.is_empty() {
            let _ = write!(self.message, "{}={value:?} ", field.name());
        }
    }
}

/// tracing Layer that mirrors every event into the ring buffer.
pub struct BufferLayer;

impl<S: Subscriber> Layer<S> for BufferLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        push(LogEntry {
            ts: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            message: visitor.message,
        });
    }
}

/// Install the standard formatter plus the capture layer.
pub fn init() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "finarr=info,tower_http=warn,librqbit=warn".into()),
        )
        .finish()
        .with(BufferLayer)
        .try_init();
}
