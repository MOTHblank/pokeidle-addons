//! Browserless Kick per-account supervisor.
//! Channel metadata, Kick viewer WebSocket, HLS bytes and channel-points
//! balances are distinct signals. Only an observed increase of a real
//! authenticated point balance is reported as points earned.
use crate::native_kick_viewer::{self, WatchTelemetry};
use std::collections::{BTreeMap, BTreeSet};
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
// A separate cancellation flag per channel allows PokéIdle's live roster
// to change without restarting unrelated viewer/HLS sessions or losing the
// observed point-balance baseline for streams that stay online.
struct KickWorker {
    slug: String,
    telemetry: Arc<Mutex<WatchTelemetry>>,
    stop: Arc<AtomicBool>,
}
#[derive(Clone, Debug, Default)]
pub struct OfflinePoints {
    pub balance: Option<u64>,
    pub updated_at: Option<Instant>,
    pub error: Option<String>,
}

/// A single HTTP poller handles official creators not currently monitored
/// by the stream supervisors, without extra WebSockets or media downloads.
struct OfflinePointsWorker {
    roster: Arc<Mutex<Vec<String>>>,
    balances: Arc<Mutex<BTreeMap<String, OfflinePoints>>>,
    stop: Arc<AtomicBool>,
}
pub struct KickMonitor {
    snapshot: Arc<Mutex<KickSnapshot>>,
    observed: Vec<KickWorker>,
    token: String,
    offline_points: Option<OfflinePointsWorker>,
}
impl KickMonitor {
    pub fn start(token: String, input: &str) -> Result<Self, String> {
        // An empty official roster is valid: keep the session ready for the
        // next PokéIdle live-state update, without creating a false channel.
        let slugs = if input.trim().is_empty() {
            Vec::new()
        } else {
            normalize_channels(input)?
        };
        let trimmed = token.trim();
        let bearer = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed).trim();
        if bearer.is_empty() || bearer.len() > MAX_TOKEN_LEN ||
            bearer.bytes().any(|byte| byte.is_ascii_control() || byte == b' ')
        {
            return Err("A Kick account Bearer token is required".into());
        }
        let mut monitor = Self {
            snapshot: Arc::new(Mutex::new(KickSnapshot::default())),
            observed: Vec::new(),
            token: bearer.to_owned(),
            offline_points: None,
        };
        monitor.sync_channels(&slugs)?;
        Ok(monitor)
    }

    /// Reconcile active channels without interrupting unchanged streams.
    /// Empty means no official live streams (unless manually selected).
    /// Do not persist discovered channels as permanent manual preferences.
    pub fn sync_channels(&mut self, slugs: &[String]) -> Result<(), String> {
        let normalized = if slugs.is_empty() {
            Vec::new()
        } else {
            normalize_channels(&slugs.join(","))?
        };
        let desired: BTreeSet<_> = normalized.iter().cloned().collect();
        for worker in &self.observed {
            if !desired.contains(&worker.slug) {
                worker.stop.store(true, Ordering::Relaxed);
            }
        }
        self.observed.retain(|w| desired.contains(&w.slug));
        if let Ok(mut state) = self.snapshot.lock() {
            state.channels.retain(|channel| desired.contains(&channel.slug));
        }
        for (offset, slug) in normalized.iter().enumerate() {
            if self.observed.iter().any(|worker| worker.slug == *slug) {
                continue;
            }
            let telemetry = Arc::new(Mutex::new(WatchTelemetry::default()));
            let worker_stop = Arc::new(AtomicBool::new(false));
            if let Ok(mut state) = self.snapshot.lock() {
                state.channels.push(ChannelStatus {
                    slug: slug.clone(), live: None, title: String::new(),
                    viewers: None, watch: WatchTelemetry::default(), error: None,
                });
                state.channels.sort_by(|a,b| a.slug.cmp(&b.slug));
            }
            let child_state = Arc::clone(&telemetry);
            let child_snapshot = Arc::clone(&self.snapshot);
            let child_stop = Arc::clone(&worker_stop);
            let child_token = self.token.clone();
            let child_slug = slug.clone();
            let result = thread::Builder::new()
                .name("moth-kick-channel".into())
                .spawn(move || {
                    // A short stagger avoids bursts, without delaying already
                    // active channels when the official list changes.
                    for _ in 0..offset.min(5).saturating_mul(2) {
                        if child_stop.load(Ordering::Relaxed) { return; }
                        thread::sleep(Duration::from_secs(1));
                    }
                    channel_loop(child_token, child_slug, child_state,
                        child_snapshot, child_stop)
                });
            if result.is_err() {
                worker_stop.store(true, Ordering::Relaxed);
                if let Ok(mut state) = self.snapshot.lock() {
                    state.channels.retain(|item| item.slug != *slug);
                }
                return Err("Could not start Kick channel worker".into());
            }
            self.observed.push(KickWorker {
                slug: slug.clone(), telemetry, stop: worker_stop,
            });
        }
        Ok(())
    }
    /// Check authenticated balances for official creators not handled by a
    /// regular channel supervisor (offline or beyond the streaming limit).
    /// The worker sleeps between polls and never creates media connections.
    pub fn sync_offline_officials(&mut self, names: &[String]) -> Result<(), String> {
        let mut pending: Vec<String> = names.iter().filter(|name|
            !self.observed.iter().any(|worker| worker.slug == name.as_str())
        ).cloned().collect();
        pending.sort();
        pending.dedup();
        if self.offline_points.is_none() {
            if pending.is_empty() { return Ok(()); }
            let roster = Arc::new(Mutex::new(pending.clone()));
            let balances = Arc::new(Mutex::new(BTreeMap::<String, OfflinePoints>::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let roster_worker = Arc::clone(&roster);
            let balances_worker = Arc::clone(&balances);
            let stop_worker = Arc::clone(&stop);
            let token = self.token.clone();
            thread::Builder::new().name("moth-kick-offline-points".into()).spawn(move || {
                let Ok(client) = native_kick_viewer::new_client(&token) else { return };
                let mut previous = Vec::<String>::new();
                let mut last_poll = Instant::now() - Duration::from_secs(130);
                while !stop_worker.load(Ordering::Relaxed) {
                    let current = roster_worker.lock().map(|roster| roster.clone()).unwrap_or_default();
                    if current != previous || last_poll.elapsed() >= Duration::from_secs(120) {
                        previous = current.clone();
                        last_poll = Instant::now();
                        if let Ok(mut results) = balances_worker.lock() {
                            results.retain(|slug, _| current.contains(slug));
                        }
                        for slug in &current {
                            if stop_worker.load(Ordering::Relaxed) { break; }
                            let outcome = native_kick_viewer::point_balance(&client, slug);
                            if let Ok(mut results) = balances_worker.lock() {
                                let info = results.entry(slug.clone()).or_default();
                                match outcome {
                                    Ok(Some(balance)) => {
                                        info.balance = Some(balance);
                                        info.updated_at = Some(Instant::now());
                                        info.error = None;
                                    }
                                    Ok(None) => info.error = None,
                                    Err(error) => info.error = Some(error),
                                }
                            }
                            // Avoid bursts when many official creators are offline.
                            thread::sleep(Duration::from_millis(350));
                        }
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }).map_err(|_| "Could not start Kick offline balance polling".to_owned())?;
            self.offline_points = Some(OfflinePointsWorker { roster, balances, stop });
        } else if let Some(worker) = &self.offline_points {
            if let Ok(mut current) = worker.roster.lock() {
                if *current != pending { *current = pending; }
            }
        }
        Ok(())
    }
    pub fn offline_point_balances(&self) -> BTreeMap<String, OfflinePoints> {
        self.offline_points.as_ref()
            .and_then(|worker| worker.balances.lock().ok().map(|balances| balances.clone()))
            .unwrap_or_default()
    }
    pub fn snapshot(&self) -> KickSnapshot {
        let mut state = self.snapshot.lock().map(|s| s.clone()).unwrap_or_default();
        for channel in &mut state.channels {
            if let Some(worker) = self.observed.iter()
                .find(|worker| worker.slug == channel.slug)
            {
                if let Ok(watching) = worker.telemetry.lock() {
                    channel.watch = watching.clone();
                }
            }
        }
        state
    }
    pub fn stop(&self) {
        if let Some(worker) = &self.offline_points {
            worker.stop.store(true, Ordering::Relaxed);
        }
        for worker in &self.observed {
            worker.stop.store(true, Ordering::Relaxed);
        }
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
            // Make failed public-channel lookups visible at account level.
            // Previously errors existed only inside collapsed per-channel
            // details, so users saw an endless 0/N and "waiting" message.
            state.error = state.channels.iter()
                .find_map(|item| item.error.clone());
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
    let public_client = match native_kick_viewer::new_public_client() {
        Ok(client) => client,
        Err(error) => {
            update_channel(&snapshot, &slug, |c| c.error = Some(error));
            return;
        }
    };
    let mut failures = 0_u32;
    while !stop.load(Ordering::Relaxed) {
        match native_kick_viewer::channel_meta(&public_client, &slug) {
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
                    let balance_result = native_kick_viewer::point_balance(&client, &slug);
                    if let Ok(mut state) = telemetry.lock() {
                        match balance_result {
                            Ok(balance) => native_kick_viewer::observe_points(&mut state, balance),
                            Err(error) => state.error = Some(error),
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
                // Public metadata requests cannot prove that an account's
                // saved login has expired. A 401/403 can also be a network
                // challenge, so retain errors and retry with backoff.
                update_channel(&snapshot, &slug, |c| {
                    c.error = Some(error);
                    c.live = None;
                    c.viewers = None;
                });
                failures = failures.saturating_add(1);
                pause(&stop, Duration::from_secs(
                    (10_u64.saturating_mul(2_u64.pow(failures.min(4)))).min(180)));
            }
        }
    }
}
/// An official Kick creator and current live status from PokéIdle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficialKickChannel {
    pub slug: String,
    pub live: bool,
}

/// Preserve *all* official creators, not just live ones. Inactive creators
/// remain visible in the Kick channel list without starting media sessions.
pub fn pokeidle_official_roster(state: &serde_json::Value) -> Option<Vec<OfficialKickChannel>> {
    let rows = state.get("kick")?.get("oficiais")?.as_array()?;
    let mut channels = std::collections::BTreeMap::<String, bool>::new();
    for row in rows {
        let raw = row.get("slug").and_then(serde_json::Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .or_else(|| row.get("login").and_then(serde_json::Value::as_str));
        let Some(raw) = raw else { continue };
        let slug = raw.trim().to_ascii_lowercase();
        if slug.is_empty() || slug.len() > 25 || !slug.bytes().all(|b|
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-') {
            continue;
        }
        let is_live = row.get("aoVivo").and_then(serde_json::Value::as_bool) == Some(true);
        channels.entry(slug).and_modify(|live| *live |= is_live)
            .or_insert(is_live);
    }
    Some(channels.into_iter().map(|(slug, live)|
        OfficialKickChannel { slug, live }).collect())
}

/// Only official creators currently broadcasting should start Kick HLS streams.
pub fn pokeidle_live_officials(state: &serde_json::Value) -> Option<Vec<String>> {
    Some(pokeidle_official_roster(state)?.into_iter()
        .filter(|channel| channel.live).map(|channel| channel.slug).collect())
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
    fn can_start_waiting_for_official_kick_lives() {
        assert!(normalize_channels("").is_err()); // empty manual input
        let monitor = KickMonitor::start("local-test-token".into(), "").unwrap();
        assert!(monitor.snapshot().channels.is_empty());
        monitor.stop();
    }
    #[test]
    fn official_roster_retains_offline_creators_and_merges_duplicates() {
        use serde_json::json;
        let state = json!({"kick":{"oficiais":[
            {"slug":"Offline_One","aoVivo":false},
            {"slug":"LIVE","aoVivo":false},
            {"login":"live","aoVivo":true},
            {"slug":"Invalid/Name","aoVivo":true}
        ]}});
        assert_eq!(pokeidle_official_roster(&state), Some(vec![
            OfficialKickChannel { slug:"live".into(),live:true },
            OfficialKickChannel { slug:"offline_one".into(),live:false },
        ]));
        assert_eq!(pokeidle_live_officials(&state),Some(vec!["live".into()]));
        assert_eq!(pokeidle_official_roster(&json!({})), None);
        assert_eq!(pokeidle_official_roster(&json!({"kick":{"oficiais":[]}})),Some(vec![]));
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
            {"slug":"Another_One","aoVivo":true},
            {"slug":null,"login":"Fallback","aoVivo":true}
        ]}});
        assert_eq!(pokeidle_live_officials(&state),
            Some(vec!["another_one".into(), "fallback".into(), "kicklive".into()]));
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