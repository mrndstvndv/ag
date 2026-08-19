//! OpenCode Zen gateway — port of ~/.pi/personal/extensions/opencode-free.ts
//!
//! https://opencode.ai/zen/v1 — OpenAI-compatible endpoint with free models,
//! anonymous access (key `"public"`). The `x-opencode-*` headers are required
//! for rate-limit bucketing; without them the gateway 429s immediately.

use genai::adapter::AdapterKind;
use genai::chat::{ChatOptions, ChatResponse};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, Headers, ModelIden, Result, ServiceTarget};
use std::collections::HashMap;
use std::io::Read;
use std::process::Command;

pub const BASE_URL: &str = "https://opencode.ai/zen/v1/";
pub const API_KEY: &str = "public"; // anonymous access for free models

/// Verified working as of 2026-08-19 (other extension models are stale/dead).
pub const MODELS: [&str; 2] = ["deepseek-v4-flash-free", "nemotron-3-ultra-free"];

pub use genai::chat::{ChatMessage, ChatRequest, ChatStreamEvent, ChatStreamResponse};

const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn time_hex() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    format!("{:012x}", millis * 0x1000 & 0xffff_ffff_ffff)
}

fn random_base62(len: usize) -> String {
    let mut f = std::fs::File::open("/dev/urandom").unwrap();
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).unwrap();
    buf.iter().map(|b| BASE62[*b as usize % 62] as char).collect()
}

fn project_id() -> String {
    let root_commit = Command::new("git")
        .args(["rev-list", "--max-parents=0", "--all"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 40);
    root_commit.unwrap_or_else(|| "global".to_string())
}

/// Fresh `x-opencode-*` headers. Session/request IDs are regenerated per call
/// so each request gets its own rate-limit bucket.
pub fn new_headers() -> Headers {
    let session_id = format!("ses_{}{}", time_hex(), random_base62(14));
    let request_id = format!("usr_{}{}", time_hex(), random_base62(14));
    let mut map = HashMap::new();
    map.insert("User-Agent".into(), "opencode/latest/0.0.0/cli".into());
    map.insert("HTTP-Referer".into(), "https://opencode.ai".into());
    map.insert("X-Title".into(), "opencode".into());
    map.insert("x-opencode-session".into(), session_id);
    map.insert("x-opencode-project".into(), project_id());
    map.insert("x-opencode-request".into(), request_id);
    map.insert("x-opencode-client".into(), "cli".into());
    Headers::from(map)
}

/// Service target pinned to the Zen gateway with anonymous auth.
pub fn new_service_target(model: &str) -> ServiceTarget {
    ServiceTarget {
        endpoint: Endpoint::from_static(BASE_URL),
        auth: AuthData::from_single(API_KEY),
        model: ModelIden::new(AdapterKind::OpenAI, model),
    }
}

pub fn client() -> Client {
    Client::default()
}

/// One-shot chat call against the Zen gateway with fresh headers.
pub async fn chat(model: &str, chat_req: ChatRequest) -> Result<ChatResponse> {
    let options = ChatOptions::default().with_extra_headers(new_headers());
    client()
        .exec_chat(new_service_target(model), chat_req, Some(&options))
        .await
}

/// Streaming chat call against the Zen gateway with fresh headers.
pub async fn chat_stream(model: &str, chat_req: ChatRequest) -> Result<ChatStreamResponse> {
    let options = ChatOptions::default().with_extra_headers(new_headers());
    client()
        .exec_chat_stream(new_service_target(model), chat_req, Some(&options))
        .await
}