//! Minimal, anonymous installation heartbeat. No player, account or machine identifiers.
//! Endpoint is embedded only when the release builder configures MOTH_TELEMETRY_URL.
use serde_json::{json, Value};
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const INTERVAL: u64 = 24 * 60 * 60;
const RETRY_INTERVAL: u64 = 60 * 60;
const STATE_FILE: &str = "anonymous-usage.json";

#[derive(Clone, Debug)]
struct State {
    id: String,
    last_sent: u64,
}

fn new_id() -> String {
    // RandomState is seeded by the OS. This is a pseudonymous counter key,
    // NOT an authentication token or cryptographic security primitive.
    let a = std::collections::hash_map::RandomState::new();
    let b = std::collections::hash_map::RandomState::new();
    let mut first = a.build_hasher();
    first.write_u64(0x4d4f5448_55534147);
    let mut second = b.build_hasher();
    second.write_u64(0x455f4944_32303236);
    format!("{:016x}{:016x}", first.finish(), second.finish())
}

fn read_state(path: &Path) -> Result<Option<State>, String> {
    if !path.exists() { return Ok(None); }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let id = value.get("id").and_then(Value::as_str)
        .filter(|id| id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or("invalid anonymous usage id")?;
    let last_sent = value.get("last_sent").and_then(Value::as_u64).unwrap_or(0);
    Ok(Some(State { id: id.to_string(), last_sent }))
}

fn write_state(path: &Path, state: &State) -> Result<(), String> {
    let parent = path.parent().ok_or("missing usage state directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    // Windows cannot atomically rename over an existing destination.
    // Keep the file tiny; malformed partial writes fail closed on next start.
    std::fs::write(path, json!({"id": state.id, "last_sent": state.last_sent}).to_string())
        .map_err(|e| e.to_string())
}

fn now_secs() -> Option<u64> {
    SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn should_send(last_sent: u64, now: u64) -> bool {
    now.saturating_sub(last_sent) >= INTERVAL || last_sent > now
}

fn post_once(endpoint: &str, state: &State) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("MothController-usage")
        .build().map_err(|e| e.to_string())?;
    let response = client.post(endpoint).json(&json!({
        "installation_id": state.id,
        "version": env!("CARGO_PKG_VERSION"),
        "platform": if cfg!(windows) { "windows" } else if cfg!(target_os = "linux") { "linux" } else { "other" }
    })).send().map_err(|e| e.to_string())?;
    response.error_for_status().map_err(|e| e.to_string())?;
    Ok(())
}

fn run_once(endpoint: &str) {
    let Ok(options) = crate::preferences::AppPreferences::load() else { return };
    if !options.anonymous_usage_stats { return; }
    let Some(now) = now_secs() else { return };
    let Ok(root) = crate::config::data_root() else { return };
    let path = root.join(STATE_FILE);
    let mut state = match read_state(&path) {
        Ok(Some(state)) => state,
        Ok(None) => State { id: new_id(), last_sent: 0 },
        Err(_) => return, // Never silently change an existing installation identity.
    };
    if !should_send(state.last_sent, now) { return; }
    // Persist before sending, so retries cannot create new identities on write failures.
    if write_state(&path, &state).is_err() { return; }
    if post_once(endpoint, &state).is_ok() {
        state.last_sent = now;
        let _ = write_state(&path, &state);
    } else {
        // Back off on transport/server failures; never retry every UI repaint.
        state.last_sent = now.saturating_sub(INTERVAL - RETRY_INTERVAL);
        let _ = write_state(&path, &state);
    }
}

/// A single low-priority thread; the UI and game sessions never wait for telemetry.
/// Release builds without a configured HTTPS receiver create no thread or network call.
pub fn start() {
    let Some(endpoint) = option_env!("MOTH_TELEMETRY_URL")
        .filter(|url| url.starts_with("https://") && !url.chars().any(char::is_whitespace))
    else { return };
    let endpoint = endpoint.to_string();
    let _ = std::thread::Builder::new()
        .name("anonymous-usage".into())
        .spawn(move || loop {
            run_once(&endpoint);
            std::thread::sleep(Duration::from_secs(RETRY_INTERVAL));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_sends_more_than_once_per_day() {
        assert!(!should_send(1000, 1000 + INTERVAL - 1));
        assert!(should_send(1000, 1000 + INTERVAL));
        assert!(should_send(1000 + INTERVAL, 1000)); // recovered clock
    }
    #[test]
    fn generated_id_has_no_local_account_material() {
        let first = new_id();
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first, new_id());
    }
}
