//! Browserless Kick per-account supervisor.
//! Channel metadata, Kick viewer WebSocket, HLS bytes and channel-points
//! balances are distinct signals. Only an observed increase of a real
//! authenticated point balance is reported as points earned.
use crate::native_kick_viewer::{self, WatchTelemetry};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CHANNELS: usize = 12;
const MAX_TOKEN_LEN: usize = 8192;
const OFFLINE_POLL: Duration = Duration::from_secs(90);

#[derive(Clone, Debug)]
pub struct ChannelStatus {
    pub slug: String,
    pub live: Option<bool>,
    pub title: String,
    pub viewers: Option<u64>,
    pub watch: WatchTelemetry,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub struct KickSnapshot {
    pub phase: String,
    pub channels: Vec<ChannelStatus>,
    pub last_checked: Option<Instant>,
    pub error: Option<String>,
}
impl Default for KickSnapshot {
    fn default() -> Self {
        Self {
            phase: "Connecting".into(),
            channels: Vec::new(),
            last_checked: None,
            error: None,
        }
    }
}
pub struct KickMonitor {
    snapshot: Arc<Mutex<KickSnapshot>>,
    observed: Vec<(String, Arc<Mutex<WatchTelemetry>>)>,
    stop: Arc<AtomicBool>,
}
impl KickMonitor {
    pub fn start(token: String, input: &str) -> Result<Self, String> {
        let slugs = normalize_channels(input)?;
        // A Kick website account token is distinct from a Developer API
        // access token. Never log or persist it, including failed requests.
        let trimmed = token.trim();
        let bearer = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed).trim();
        if bearer.is_empty() || bearer.len() > MAX_TOKEN_LEN ||
            bearer.bytes().any(|byte| byte.is_ascii_control() || byte == b' ')
        {
            return Err("A Kick account Bearer token is required".into());
        }
        let snapshot = Arc::new(Mutex::new(KickSnapshot {
            channels: slugs.iter().map(|slug| ChannelStatus {
                slug: slug.clone(), live: None, title: String::new(),
                viewers: None, watch: WatchTelemetry::default(), error: None,
            }).collect(),
            ..KickSnapshot::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        // One isolated supervisor per channel. Stagger connections to avoid
        // a simultaneous burst from all four controller account profiles.
        let mut observed = Vec::with_capacity(slugs.len());
        for (offset, slug) in slugs.into_iter().enumerate() {
            let state = Arc::new(Mutex::new(WatchTelemetry::default()));
            let child_state = Arc::clone(&state);
            let child_snapshot = Arc::clone(&snapshot);
            let child_stop = Arc::clone(&stop);
            let child_token = bearer.to_owned();
            let child_slug = slug.clone();
            let outcome = thread::Builder::new()
                .name("moth-kick-channel".into())
                .spawn(move || {
                    for _ in 0..offset.saturating_mul(7) {
                        if child_stop.load(Ordering::Relaxed) { return; }
                        thread::sleep(Duration::from_secs(1));
                    }
                    channel_loop(child_token, child_slug, child_state,
                        child_snapshot, child_stop)
                });
            if outcome.is_err() {
                stop.store(true, Ordering::Relaxed);
                return Err("Could not start Kick channel worker".into());
            }
            observed.push((slug, state));
        }
        Ok(Self { snapshot, observed, stop })
    }
    pub fn snapshot(&self) -> KickSnapshot {
        let mut state = self.snapshot.lock().map(|s| s.clone()).unwrap_or_default();
        for channel in &mut state.channels {
            if let Some((_, observed)) = self.observed.iter().find(|(slug, _)| slug == &channel.slug) {
                if let Ok(watching) = observed.lock() {
                    channel.watch = watching.clone();
                }
            }
        }
        state
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for KickMonitor {
    fn drop(&mut self) { self.stop(); }
}
fn update_channel(snapshot: &Arc<Mutex<KickSnapshot>>, slug: &str,
                  action: impl FnOnce(&mut ChannelStatus)) {
    if let Ok(mut state) = snapshot.lock() {
        state.last_checked = Some(Instant::now());
        if let Some(channel) = state.channels.iter_mut().find(|item| item.slug == slug) {
            action(channel);
        }
        if state.channels.iter().any(|item| item.error.is_none() && item.live.is_some()) {
            state.phase = "Online".into();
            state.error = None;
        } else {
            state.phase = "Reconnecting".into();
        }
    }
}
fn pause(stop: &AtomicBool, duration: Duration) {
    for _ in 0..duration.as_secs() {
        if stop.load(Ordering::Relaxed) { break; }
        thread::sleep(Duration::from_secs(1));
    }
}
fn channel_loop(token: String, slug: String, telemetry: Arc<Mutex<WatchTelemetry>>,
                snapshot: Arc<Mutex<KickSnapshot>>, stop: Arc<AtomicBool>) {
    let client = match native_kick_viewer::new_client(&token) {
        Ok(client) => client,
        Err(error) => {
            update_channel(&snapshot, &slug, |c| c.error = Some(error));
            return;
        }
    };
    let mut failures = 0_u32;
    while !stop.load(Ordering::Relaxed) {
        match native_kick_viewer::channel_meta(&client, &slug) {
            Ok(meta) => {
                failures = 0;
                update_channel(&snapshot, &slug, |c| {
                    c.live = Some(meta.is_live);
                    c.title = meta.title.clone();
                    c.viewers = meta.viewers;
                    c.error = None;
                });
                if !meta.is_live {
                    if let Ok(mut state) = telemetry.lock() {
                        state.ws_connected = false;
                        state.handshake_received = false;
                    }
                    // Check a real balance even if a channel is offline, so
                    // previously earned points are not misrepresented as zero.
                    if let Ok(balance) = native_kick_viewer::point_balance(&client, &slug) {
                        if let Ok(mut state) = telemetry.lock() {
                            native_kick_viewer::observe_points(&mut state, balance);
                        }
                    }
                    pause(&stop, OFFLINE_POLL);
                    continue;
                }
                let result = native_kick_viewer::run_viewer(
                    &client, &slug, &meta, &telemetry, Arc::clone(&stop));
                if let Ok(mut state) = telemetry.lock() {
                    state.ws_connected = false;
                    state.handshake_received = false;
                    if let Err(error) = &result {
                        state.error = Some(error.clone());
                    }
                }
                if result.is_err() {
                    failures = failures.saturating_add(1);
                }
                // Reacquire signed HLS URL and short-lived viewer token. No
                // reused stale stream ID, and no fast recursive reconnect.
                let delay = if failures == 0 { 3 } else {
                    (5_u64.saturating_mul(2_u64.pow(failures.min(4)))).min(120)
                };
                pause(&stop, Duration::from_secs(delay));
            },
            Err(error) => {
                let login_expired = error.contains("401/403");
                update_channel(&snapshot, &slug, |c| {
                    c.error = Some(error);
                    c.live = None;
                    c.viewers = None;
                });
                if login_expired { break; }
                failures = failures.saturating_add(1);
                pause(&stop, Duration::from_secs(
                    (10_u64.saturating_mul(2_u64.pow(failures.min(4)))).min(180)));
            }
        }
    }
}
/// Official Kick streamers reported by PokéIdle (not general Kick search).
/// `None` means roster data was not delivered; `Some([])` means all offline.
/// Preserve all valid live names here; active session limits are applied later.
pub fn pokeidle_live_officials(state: &serde_json::Value) -> Option<Vec<String>> {
    let rows = state.get("kick")?.get("oficiais")?.as_array()?;
    let mut names = BTreeSet::new();
    for row in rows {
        if row.get("aoVivo").and_then(serde_json::Value::as_bool) != Some(true) {
            continue;
        }
        let Some(raw) = row.get("slug").or_else(|| row.get("login"))
            .and_then(serde_json::Value::as_str) else { continue; };
        // The game supplies names, not arbitrary URLs. Whitelist valid
        // channel names before passing them to Kick requests or UI.
        let slug = raw.trim().to_ascii_lowercase();
        if !slug.is_empty() && slug.len() <= 25 && slug.bytes().all(|b|
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') {
            names.insert(slug);
        }
    }
    Some(names.into_iter().collect())
}

/// Keep only validated public channel slugs (and official Kick links).
pub fn normalize_channels(input: &str) -> Result<Vec<String>, String> {
    let mut channels = BTreeSet::new();
    for part in input.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let part = part.trim().trim_end_matches('/');
        if part.is_empty() { continue; }
        let slug = part.strip_prefix("https://kick.com/")
            .or_else(|| part.strip_prefix("https://www.kick.com/"))
            .unwrap_or(part).to_ascii_lowercase();
        if slug.is_empty() || slug.len() > 25 || !slug.bytes().all(|b|
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        {
            return Err("Invalid Kick channel: use channel names or kick.com URLs".into());
        }
        channels.insert(slug);
        if channels.len() > MAX_CHANNELS { return Err("Maximum 12 Kick channels per account".into()); }
    }
    if channels.is_empty() { return Err("Add at least one Kick channel".into()); }
    Ok(channels.into_iter().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channels_validate_and_deduplicate() {
        assert_eq!(normalize_channels("https://kick.com/SomeName, somename\nOther_2").unwrap(),
            vec!["other_2", "somename"]);
        assert!(normalize_channels("https://evil.example/channel").is_err());
        assert!(normalize_channels("../../secret").is_err());
        assert!(normalize_channels("").is_err());
        assert!(normalize_channels(&(0..13).map(|i| format!("ch{i}"))
            .collect::<Vec<_>>().join(",")).is_err());
    }
    #[test]
    fn official_pokeidle_kick_roster_is_live_only_and_deduplicated() {
        use serde_json::json;
        assert_eq!(pokeidle_live_officials(&json!({})), None);
        assert_eq!(pokeidle_live_officials(&json!({"kick":{"oficiais":[]}})),
            Some(Vec::new()));
        let state = json!({"kick":{"oficiais":[
            {"slug":"KickLive","aoVivo":true},
            {"login":"kicklive","aoVivo":true},
            {"login":"not_live","aoVivo":false},
            {"slug":"https://evil.invalid/path","aoVivo":true},
            {"slug":"Another_One","aoVivo":true}
        ]}});
        assert_eq!(pokeidle_live_officials(&state),
            Some(vec!["another_one".into(), "kicklive".into()]));
    }
    #[test]
    fn never_count_point_balance_as_earned_without_delta() {
        let mut status=WatchTelemetry::default();
        native_kick_viewer::observe_points(&mut status,Some(500));
        assert_eq!(status.points_gained,0);
        native_kick_viewer::observe_points(&mut status,Some(510));
        assert_eq!(status.points_gained,10);
    }
}