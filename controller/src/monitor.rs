use crate::kick::KickStream;
use crate::logging;
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

type BrowserSocket = WebSocket<MaybeTlsStream<TcpStream>>;

// The inexpensive page heartbeat does not enumerate tabs or serialize game state.
// Detailed game data and the cross-tab inventory have separate cadences.
pub(crate) const MONITOR_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(45);
pub(crate) const MONITOR_POLL_INTERVAL: Duration = Duration::from_secs(3 * 60);
const MONITOR_STARTUP_POLL_INTERVAL: Duration = Duration::from_secs(45);
pub(crate) const MONITOR_TAB_INTERVAL: Duration = Duration::from_secs(10 * 60);
const MONITOR_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const TELEMETRY_FRESHNESS_LIMIT: Duration = Duration::from_secs(5 * 60);
const HEARTBEAT_FRESHNESS_LIMIT: Duration = Duration::from_secs(2 * 60);
const PROFILE_STAGGER: Duration = Duration::from_secs(12);

#[derive(Clone, Debug)]
pub struct TabInfo {
    pub kind: String,
    pub title: String,
    pub url: String,
    pub low_resource: bool,
    /// Count requested by the managed Twitch popout URL, not a count of tabs.
    pub twitch_channels: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Health {
    pub state: String,
    pub url: String,
    pub title: String,
    pub game_ready: bool,
    pub logged_in: bool,
    pub activity: String,
    pub hunt: String,
    pub hunt_slug: String,
    pub hunt_atlas: Value,
    pub active_pokemon: String,
    pub fallen_count: u32,
    pub economy_mode: bool,
    pub addon_ok: u8,
    pub addon_total: u8,
    pub addon_missing: Vec<String>,
    pub addon_versions: BTreeMap<String, String>,
    pub twitch_tabs: u8,
    pub twitch_low_resource_ok: u8,
    pub performance_enabled: Option<bool>,
    pub performance_fps: String,
    pub performance_scene_runs: String,
    pub pokemon_level: String,
    pub pokemon_xp: String,
    pub ball_stock: Vec<String>,
    pub autocatch_on: bool,
    pub autocatch_captures: u32,
    pub autocatch_balls_used: u32,
    pub autocatch_rate: String,
    pub autocatch_restock: String,
    pub stream_scan_status: String,
    pub stream_scan_live: u32,
    pub stream_scan_opened: u32,
    /// Scanner Twitch channel count; None if the installed scanner is too old.
    pub stream_scan_twitch_channels: Option<u32>,
    pub xp_bonuses: Vec<String>,
    pub player_level: u32,
    pub player_xp: String,
    /// Cumulative trainer XP from Controller Bridge; absent when unverified.
    pub player_total_xp: Option<u64>,
    /// Cumulative XP remaining to reach the next trainer level.
    pub player_xp_to_next: Option<u64>,
    pub gold: u64,
    pub orbs: u64,
    pub stream_bonus: String,
    pub stream_bonus_pct: f32,
    pub stream_watching: Vec<String>,
    pub stream_live_bonus: Vec<String>,
    pub stream_missing: Vec<String>,
    pub xp_sources: Vec<String>,
    pub bridge_connected: bool,
    /// The bridge snapshot was actually readable, regardless of game socket state.
    pub bridge_snapshot_ok: bool,
    pub last_game_message_ms: u64,
    pub tabs: Vec<TabInfo>,
    pub kick_streams: Vec<KickStream>,
    pub kick_state_available: bool,
    pub last_error: Option<String>,
    /// Time of last successful telemetry sample (kept on transient disconnects).
    pub last_successful_probe_at: Option<Instant>,
    /// A lightweight live-page check, independent from expensive game samples.
    pub last_heartbeat_at: Option<Instant>,
    /// Last full inventory of Firefox tabs (may be older than game stats).
    pub last_tabs_at: Option<Instant>,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            state: "Starting".to_string(),
            url: String::new(),
            title: String::new(),
            game_ready: false,
            logged_in: false,
            activity: "Starting".to_string(),
            hunt: String::new(),
            hunt_slug: String::new(),
            hunt_atlas: Value::Null,
            active_pokemon: String::new(),
            fallen_count: 0,
            economy_mode: false,
            addon_ok: 0,
            addon_total: 7,
            addon_missing: Vec::new(),
            addon_versions: BTreeMap::new(),
            twitch_tabs: 0,
            twitch_low_resource_ok: 0,
            performance_enabled: None,
            performance_fps: String::new(),
            performance_scene_runs: String::new(),
            pokemon_level: String::new(),
            pokemon_xp: String::new(),
            ball_stock: Vec::new(),
            autocatch_on: false,
            autocatch_captures: 0,
            autocatch_balls_used: 0,
            autocatch_rate: String::new(),
            autocatch_restock: String::new(),
            stream_scan_status: String::new(),
            stream_scan_live: 0,
            stream_scan_opened: 0,
            stream_scan_twitch_channels: None,
            xp_bonuses: Vec::new(),
            player_level: 0,
            player_xp: String::new(),
            player_total_xp: None,
            player_xp_to_next: None,
            gold: 0,
            orbs: 0,
            stream_bonus: String::new(),
            stream_bonus_pct: 0.0,
            stream_watching: Vec::new(),
            stream_live_bonus: Vec::new(),
            stream_missing: Vec::new(),
            xp_sources: Vec::new(),
            bridge_connected: false,
            bridge_snapshot_ok: false,
            last_game_message_ms: 0,
            tabs: Vec::new(),
            kick_streams: Vec::new(),
            kick_state_available: false,
            last_error: None,
            last_successful_probe_at: None,
            last_heartbeat_at: None,
            last_tabs_at: None,
        }
    }
}

impl Health {
    pub fn telemetry_age(&self) -> Option<Duration> {
        self.last_successful_probe_at.map(|at| at.elapsed())
    }

    pub fn telemetry_is_fresh(&self) -> bool {
        self.state == "Running"
            && self.telemetry_age().is_some_and(|age| age < TELEMETRY_FRESHNESS_LIMIT)
    }

    pub fn heartbeat_is_fresh(&self) -> bool {
        self.state == "Running"
            && self.last_heartbeat_at
                .is_some_and(|at| at.elapsed() < HEARTBEAT_FRESHNESS_LIMIT)
    }

    pub fn tab_inventory_age(&self) -> Option<Duration> {
        self.last_tabs_at.map(|at| at.elapsed())
    }

    pub fn summary(&self) -> String {
        if self.state != "Running" {
            return self
                .last_error
                .clone()
                .unwrap_or_else(|| self.state.clone());
        }

        match self.activity.as_str() {
            "Hunting" => {
                let mut parts = Vec::new();

                if !self.active_pokemon.is_empty() {
                    parts.push(self.active_pokemon.clone());
                }

                if !self.hunt.is_empty() {
                    parts.push(self.hunt.clone());
                }

                if self.fallen_count > 0 {
                    parts.push(format!("{} fallen", self.fallen_count));
                }

                if parts.is_empty() {
                    "Hunting".to_string()
                } else {
                    format!("Hunting · {}", parts.join(" · "))
                }
            }
            "Center" => "Online · Center".to_string(),
            "Login" => "Waiting for login".to_string(),
            other => other.to_string(),
        }
    }

}

#[derive(Clone)]
pub struct MonitorHandle {
    health: Arc<Mutex<Health>>,
    stop: Arc<AtomicBool>,
    commands: Arc<Mutex<CommandMailbox>>,
    close_request: Arc<Mutex<Option<mpsc::Sender<Result<(), String>>>>>,
}

impl MonitorHandle {
    pub fn start(port: u16, headless: bool) -> Self {
        let health = Arc::new(Mutex::new(Health::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let commands = Arc::new(Mutex::new(CommandMailbox::default()));
        let close_request = Arc::new(Mutex::new(None));
        let close_worker = Arc::clone(&close_request);
        let shared = Arc::clone(&health);
        let stop_worker = Arc::clone(&stop);
        let command_queue = Arc::clone(&commands);

        thread::spawn(move || {
            monitor_loop(port, headless, shared, stop_worker, command_queue, close_worker);
        });

        Self { health, stop, commands, close_request }
    }

    /// A queued command is not reported as successful until Firefox acknowledges it.
    pub fn send(&self, payload: Value) -> Result<u64, String> {
        if self.stop.load(Ordering::Relaxed) {
            return Err("browser monitor is stopped".to_string());
        }
        validate_open_tab(&payload)?;
        let mut mailbox = self.commands.lock()
            .map_err(|_| "browser command mailbox is unavailable".to_string())?;
        if mailbox.pending.len() >= MAX_COMMAND_QUEUE {
            return Err("browser command queue is full".to_string());
        }
        mailbox.next_id += 1;
        let id = mailbox.next_id;
        mailbox.pending.push_back(QueuedCommand { id, payload, queued_at: Instant::now() });
        Ok(id)
    }

    pub fn take_command_feedback(&self) -> Option<CommandFeedback> {
        self.commands.lock().ok().and_then(|mut mailbox| mailbox.feedback.pop_front())
    }

    pub fn health(&self) -> Health {
        self.health.lock().map(|h| h.clone()).unwrap_or_default()
    }

    /// Ask Firefox to shut down through the existing BiDi session. Creating
    /// a second session just for browser.close can fail while this one is active.
    pub fn close_browser(&self, timeout: Duration) -> Result<(), String> {
        let (reply, received) = mpsc::channel();
        let mut pending = self.close_request.lock()
            .map_err(|_| "BiDi close request lock poisoned".to_string())?;
        if pending.is_some() {
            return Err("Firefox close is already pending".to_string());
        }
        *pending = Some(reply);
        drop(pending);

        received.recv_timeout(timeout)
            .map_err(|error| format!("BiDi browser.close did not respond: {error}"))?
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut pending) = self.close_request.lock() {
            pending.take();
        }
        if let Ok(mut mailbox) = self.commands.lock() {
            mailbox.pending.clear();
        }
    }
}

const MAX_COMMAND_QUEUE: usize = 16;
const COMMAND_MAX_WAIT: Duration = Duration::from_secs(45);

#[derive(Clone, Debug)]
pub struct CommandFeedback {
    pub id: u64,
    pub ok: bool,
    pub message: String,
}

struct QueuedCommand {
    id: u64,
    payload: Value,
    queued_at: Instant,
}

#[derive(Default)]
struct CommandMailbox {
    next_id: u64,
    pending: VecDeque<QueuedCommand>,
    feedback: VecDeque<CommandFeedback>,
}

fn validate_open_tab(payload: &Value) -> Result<&str, String> {
    if payload.get("t").and_then(Value::as_str) != Some("browser.openTab") {
        return Err("unsupported browser command".to_string());
    }
    let url = payload.get("url").and_then(Value::as_str)
        .map(str::trim).unwrap_or_default();
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| "tab URL must use http:// or https://".to_string())?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.chars().any(char::is_whitespace)
        || authority.starts_with(':') || authority.contains('@')
    {
        return Err("tab URL has an invalid host".to_string());
    }
    Ok(url)
}

struct BrowserSession {
    socket: BrowserSocket,
    next_id: u64,
    game_context: Option<String>,
    /// Top-level context id+URL fingerprint from the last full tab inspection.
    tab_signature: Vec<(String, String)>,
}

#[derive(Default)]
struct RuntimeProbe {
    bridge_connected: bool,
    bridge_snapshot_ok: bool,
    hunt_slug: String,
    hunt_atlas: Value,
    player_level: u32,
    player_xp: String,
    player_total_xp: Option<u64>,
    player_xp_to_next: Option<u64>,
    gold: u64,
    orbs: u64,
    fallen_count: u32,
    active_pokemon: String,

    pokemon_level: String,
    pokemon_xp: String,
    ball_stock: Vec<String>,
    autocatch_on: bool,
    autocatch_captures: u32,
    autocatch_balls_used: u32,
    autocatch_rate: String,
    autocatch_restock: String,
    stream_scan_status: String,
    stream_scan_live: u32,
    stream_scan_opened: u32,
    stream_scan_twitch_channels: Option<u32>,
    stream_bonus: String,
    stream_bonus_pct: f32,
    stream_watching: Vec<String>,
    stream_live_bonus: Vec<String>,
    stream_missing: Vec<String>,
    xp_sources: Vec<String>,
    last_game_message_ms: u64,
    xp_bonuses: Vec<String>,
    kick_streams: Vec<KickStream>,
    kick_state_available: bool,
}

struct Probe {
    url: String,
    title: String,
    game_ready: bool,
    logged_in: bool,
    activity: String,
    hunt: String,
    hunt_slug: String,
    active_pokemon: String,
    fallen_count: u32,
    economy_mode: bool,
    addon_ok: u8,
    addon_total: u8,
    addon_missing: Vec<String>,
    addon_versions: BTreeMap<String, String>,
    performance_enabled: Option<bool>,
    performance_fps: String,
    performance_scene_runs: String,
    pokemon_level: String,
    pokemon_xp: String,
    ball_stock: Vec<String>,
    autocatch_on: bool,
    autocatch_captures: u32,
    autocatch_balls_used: u32,
    autocatch_rate: String,
    autocatch_restock: String,
    stream_scan_status: String,
    stream_scan_live: u32,
    stream_scan_opened: u32,
    stream_scan_twitch_channels: Option<u32>,
    xp_bonuses: Vec<String>,
    player_level: u32,
    player_xp: String,
    player_total_xp: Option<u64>,
    player_xp_to_next: Option<u64>,
    gold: u64,
    orbs: u64,
    stream_bonus: String,
    stream_bonus_pct: f32,
    stream_watching: Vec<String>,
    stream_live_bonus: Vec<String>,
    stream_missing: Vec<String>,
    xp_sources: Vec<String>,
    bridge_connected: bool,
    bridge_snapshot_ok: bool,
    last_game_message_ms: u64,
    tabs: Option<Vec<TabInfo>>,
    kick_streams: Vec<KickStream>,
    kick_state_available: bool,
}

const MONITOR_ISSUE_LOG_INTERVAL: Duration = Duration::from_secs(10 * 60);

// Long-lived Firefox sessions can lose their BiDi connection without stopping
// gameplay. Retry normally, but avoid writing the same warning every five seconds.
#[derive(Default)]
struct MonitorIssueReporter {
    last_logged: Option<(String, Instant)>,
}

impl MonitorIssueReporter {
    fn should_log(&self, message: &str) -> bool {
        self.last_logged.as_ref().is_none_or(|(previous, at)| {
            previous != message || at.elapsed() >= MONITOR_ISSUE_LOG_INTERVAL
        })
    }

    fn report(&mut self, message: String) {
        if self.should_log(&message) {
            logging::warn(&message);
            self.last_logged = Some((message, Instant::now()));
        }
    }

    fn recovered(&mut self, port: u16) {
        if self.last_logged.take().is_some() {
            logging::info(&format!("BiDi telemetry resumed on port {}", port));
        }
    }
}

fn monitor_loop(
    port: u16,
    headless: bool,
    health: Arc<Mutex<Health>>,
    stop: Arc<AtomicBool>,
    commands: Arc<Mutex<CommandMailbox>>,
    close_request: Arc<Mutex<Option<mpsc::Sender<Result<(), String>>>>>,
) {
    let mut session: Option<BrowserSession> = None;
    let mut issues = MonitorIssueReporter::default();
    let mut next_full = Instant::now();
    let mut next_heartbeat = Instant::now();
    let mut next_tabs = Instant::now();

    loop {
        expire_pending_commands(&commands);
        if stop.load(Ordering::Relaxed) {
            logging::info(&format!("BiDi monitor on port {} stopping", port));
            end_session(&mut session);
            break;
        }

        if session.is_none() {
            match open_session(port) {
                Ok(browser_session) => {
                    logging::info(&format!("BiDi session established on port {}", port));
                    session = Some(browser_session);
                    let already_sampled = health.lock().map(|current|
                        current.last_successful_probe_at.is_some()).unwrap_or(false);
                    next_full = Instant::now() + stagger_delay(port, already_sampled);
                    next_heartbeat = next_full;
                    next_tabs = next_full;
                    if let Ok(mut current) = health.lock() {
                        current.state = "Connecting".to_string();
                        current.last_error = None;
                    }
                }
                Err(error) => {
                    issues.report(format!(
                        "BiDi session setup on port {} failed: {}", port, error
                    ));
                    if let Ok(mut current) = health.lock() {
                        current.state = "Connecting".to_string();
                        current.last_error = Some(error);
                        current.game_ready = false;
                        current.logged_in = false;
                    }
                }
            }
        }

        if stop.load(Ordering::Relaxed) {
            break;
        }
        let reply = close_request.lock().ok().and_then(|mut pending| pending.take());
        if let Some(reply) = reply {
            let result = match session.as_mut() {
                Some(browser_session) => {
                    let id = browser_session.next_id;
                    browser_session.next_id += 1;
                    send_and_wait(
                        &mut browser_session.socket,
                        id,
                        json!({ "id": id, "method": "browser.close", "params": {} }),
                    ).map(|_| ())
                }
                None => Err("Firefox BiDi session is not connected".to_string()),
            };
            let closed = result.is_ok();
            let _ = reply.send(result);
            if closed {
                logging::info(&format!("Firefox browser.close accepted on port {}", port));
                break;
            }
        }

        let mut wait = MONITOR_RETRY_INTERVAL;
        if let Some(browser_session) = session.as_mut() {
            if flush_commands(browser_session, &commands) {
                // An explicit controller tab change deserves an immediate update.
                next_full = Instant::now();
                next_tabs = next_full;
            }

            let now = Instant::now();
            if now >= next_full {
                match probe_page(browser_session, headless, now >= next_tabs) {
                    Ok(probe) => {
                        let sample_at = Instant::now();
                        let interval = if probe.game_ready && probe.logged_in {
                            MONITOR_POLL_INTERVAL
                        } else {
                            MONITOR_STARTUP_POLL_INTERVAL
                        };
                        let inspected_tabs = probe.tabs.is_some();
                        issues.recovered(port);
                        if let Ok(mut current) = health.lock() {
                        current.state = "Running".to_string();
                        current.url = probe.url;
                        current.title = probe.title;
                        current.game_ready = probe.game_ready;
                        current.logged_in = probe.logged_in;
                        current.activity = probe.activity;
                        current.hunt = probe.hunt;
                        current.hunt_slug = probe.hunt_slug;
            current.hunt_atlas = probe.hunt_atlas;
                        current.active_pokemon = probe.active_pokemon;
                        current.fallen_count = probe.fallen_count;
                        current.economy_mode = probe.economy_mode;
                        current.addon_ok = probe.addon_ok;
                        current.addon_total = probe.addon_total;
                        current.addon_missing = probe.addon_missing;
                        current.addon_versions = probe.addon_versions;
                        // Keep the prior tab inventory until an actual inventory refresh.
                        current.performance_enabled = probe.performance_enabled;
                        current.performance_fps = probe.performance_fps;
                        current.performance_scene_runs = probe.performance_scene_runs;
                        current.pokemon_level = probe.pokemon_level;
                        current.pokemon_xp = probe.pokemon_xp;
                        current.ball_stock = probe.ball_stock;
                        current.autocatch_on = probe.autocatch_on;
                        current.autocatch_captures = probe.autocatch_captures;
                        current.autocatch_balls_used = probe.autocatch_balls_used;
                        current.autocatch_rate = probe.autocatch_rate;
                        current.autocatch_restock = probe.autocatch_restock;
                        current.stream_scan_status = probe.stream_scan_status;
                        current.stream_scan_live = probe.stream_scan_live;
                        current.stream_scan_opened = probe.stream_scan_opened;
                        current.stream_scan_twitch_channels = probe.stream_scan_twitch_channels;
                        current.xp_bonuses = probe.xp_bonuses;
                        current.player_level = probe.player_level;
                        current.player_xp = probe.player_xp;
                        current.player_total_xp = probe.player_total_xp;
                        current.player_xp_to_next = probe.player_xp_to_next;
                        current.gold = probe.gold;
                        current.orbs = probe.orbs;
                        current.stream_bonus = probe.stream_bonus.clone();
                        current.stream_bonus_pct = probe.stream_bonus_pct;
                        current.stream_watching = probe.stream_watching;
                        current.stream_live_bonus = probe.stream_live_bonus;
                        current.stream_missing = probe.stream_missing;
                        current.xp_sources = probe.xp_sources;
                        current.bridge_connected = probe.bridge_connected;
                        current.bridge_snapshot_ok = probe.bridge_snapshot_ok;
                        current.last_game_message_ms = probe.last_game_message_ms;
                        if let Some(tabs) = probe.tabs {
                            current.twitch_tabs = tabs.iter().filter(|t| t.kind == "Twitch").count() as u8;
                            current.twitch_low_resource_ok = tabs.iter()
                                .filter(|t| t.kind == "Twitch" && t.low_resource).count() as u8;
                            current.tabs = tabs;
                            current.last_tabs_at = Some(sample_at);
                        }
                        current.kick_streams = probe.kick_streams;
                        current.kick_state_available = probe.kick_state_available;
                        current.last_error = None;
                        current.last_successful_probe_at = Some(sample_at);
                        current.last_heartbeat_at = Some(sample_at);
                        }
                        next_full = sample_at + interval;
                        next_heartbeat = sample_at + MONITOR_HEARTBEAT_INTERVAL;
                        if inspected_tabs {
                            next_tabs = sample_at + MONITOR_TAB_INTERVAL;
                        }
                    }
                    Err(error) => {
                        issues.report(format!(
                            "BiDi page probe on port {} failed: {}", port, error
                        ));
                        if let Ok(mut current) = health.lock() {
                            current.state = "Reconnecting".to_string();
                            current.last_error = Some(error);
                            current.game_ready = false;
                            current.logged_in = false;
                        }
                        end_session(&mut session);
                    }
                }
            } else if now >= next_heartbeat {
                match probe_heartbeat(browser_session) {
                    Ok(heartbeat) => {
                        let checked_at = Instant::now();
                        if let Ok(mut current) = health.lock() {
                            let changed_login = current.logged_in != heartbeat.logged_in;
                            current.last_heartbeat_at = Some(checked_at);
                            current.logged_in = heartbeat.logged_in;
                            current.game_ready = heartbeat.logged_in;
                            current.state = "Running".to_string();
                            if changed_login {
                                // New login/logout should refresh full metrics promptly.
                                next_full = checked_at;
                            }
                        }
                        issues.recovered(port);
                    }
                    Err(_error) => {
                        // Navigation can invalidate an old game context. Resolve
                        // it via a full context lookup before reporting failure.
                        next_full = Instant::now();
                    }
                }
                next_heartbeat = Instant::now() + MONITOR_HEARTBEAT_INTERVAL;
            }

            if session.is_some() {
                wait = next_full.min(next_heartbeat)
                    .saturating_duration_since(Instant::now())
                    .max(Duration::from_millis(500));
            }
        }

        // Keep browser.close and manual tab commands responsive without
        // using BiDi polling to drive the GUI repaint cadence.
        for _ in 0..(wait.as_millis() / 500) {
            if stop.load(Ordering::Relaxed)
                || close_request.lock().map(|pending| pending.is_some()).unwrap_or(false)
                || (session.is_some() && commands.lock()
                    .map(|queue| !queue.pending.is_empty()).unwrap_or(false))
            {
                break;
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

/// One tiny read of the already-known game context. No context enumeration,
/// userscript snapshot, page-wide scanning or Firefox tab evaluations.
#[derive(Clone, Copy)]
struct Heartbeat {
    logged_in: bool,
}

fn probe_heartbeat(session: &mut BrowserSession) -> Result<Heartbeat, String> {
    let context = session.game_context.as_deref()
        .ok_or_else(|| "game context not discovered yet".to_string())?;
    let id = session.next_id;
    session.next_id += 1;
    let result = send_and_wait(
        &mut session.socket,
        id,
        json!({
            "id": id,
            "method": "script.evaluate",
            "params": {
                "expression": "JSON.stringify({ loggedIn: (() => { const app = document.querySelector('#app'); return !!app && !app.classList.contains('hidden'); })() })",
                "target": { "context": context },
                "awaitPromise": false
            }
        }),
    )?;
    let raw = result.get("result")
        .and_then(|v| v.get("result"))
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| "no game heartbeat result".to_string())?;
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| format!("invalid game heartbeat: {error}"))?;
    Ok(Heartbeat {
        logged_in: value.get("loggedIn").and_then(Value::as_bool)
            .ok_or_else(|| "game heartbeat missing login state".to_string())?,
    })
}

fn should_inspect_tabs(
    page_loading: bool,
    force: bool,
    current: &[(String, String)],
    previous: &[(String, String)],
) -> bool {
    !page_loading && (force || current != previous)
}

fn stagger_delay(port: u16, previous_sample: bool) -> Duration {
    // Initial acquisition is staggered across Game1..Game4 to avoid
    // simultaneous expensive snapshots. Reconnects are never delayed.
    if previous_sample {
        Duration::ZERO
    } else {
        PROFILE_STAGGER * u32::from(port.saturating_sub(27701) % 4)
    }
}

fn open_session(port: u16) -> Result<BrowserSession, String> {
    let (mut socket, _) = connect(format!("ws://127.0.0.1:{port}/session"))
        .map_err(|error| error.to_string())?;
    // Bounded I/O prevents an unanswered read from freezing monitoring forever.
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream.set_read_timeout(Some(Duration::from_secs(4)))
            .map_err(|error| format!("BiDi read timeout setup failed: {error}"))?;
        stream.set_write_timeout(Some(Duration::from_secs(4)))
            .map_err(|error| format!("BiDi write timeout setup failed: {error}"))?;
    }

    send_and_wait(
        &mut socket,
        1,
        json!({
            "id": 1,
            "method": "session.new",
            "params": { "capabilities": {} }
        }),
    )?;

    Ok(BrowserSession {
        socket,
        next_id: 2,
        game_context: None,
        tab_signature: Vec::new(),
    })
}

fn probe_page(session: &mut BrowserSession, headless: bool, force_tabs: bool) -> Result<Probe, String> {
    let tree_id = session.next_id;
    session.next_id += 1;

    let tree = send_and_wait(
        &mut session.socket,
        tree_id,
        json!({
            "id": tree_id,
            "method": "browsingContext.getTree",
            "params": { "maxDepth": 10 }
        }),
    )?;

    let mut contexts = Vec::new();
    if let Some(list) = tree.get("result").and_then(|v| v.get("contexts")).and_then(Value::as_array) {
        for context in list {
            collect_contexts(context, &mut contexts, 0);
        }
    }

    let game = contexts
        .iter()
        .find(|c| {
            c.url.starts_with("https://pokeidle.io/app")
                || c.url.starts_with("https://www.pokeidle.io/app")
        })
        .ok_or_else(|| "Firefox has no PokéIdle app context".to_string())?
        .clone();

    session.game_context = Some(game.id.clone());

    let game_id = session.next_id;
    session.next_id += 1;

    let expression = r#"(() => {
        const html = document.documentElement;
        const forcePerformance = __MOTH_HEADLESS__;
        if (html && html.hasAttribute('data-moth-performance-force-on') !== forcePerformance) {
            html.toggleAttribute('data-moth-performance-force-on', forcePerformance);
            document.dispatchEvent(new Event('moth-performance-policy-change'));
        }
        const performanceState = html?.getAttribute('data-moth-performance-active')
            ?? document.querySelector('#moth-performance-trigger')?.getAttribute('aria-pressed');
        const performanceEnabled = performanceState === '1' || performanceState === 'true'
            ? true : performanceState === '0' || performanceState === 'false' ? false : null;

        const text = (selector) => {
            const el = document.querySelector(selector);
            return el ? (el.textContent || '').replace(/\s+/g, ' ').trim() : '';
        };
        const exists = (selector) => !!document.querySelector(selector);
        const visible = (selector) => {
            const el = document.querySelector(selector);
            return !!el && !el.classList.contains('hidden') && !el.hidden;
        };

        const app = document.querySelector('#app');
        const loggedIn = !!app && !app.classList.contains('hidden');
        const loading = visible('#carregando');
        const loadingPercent = text('#carga-pct');
        const loadingStage = text('#carga-etapa');
        const ativo = document.querySelector('#ativo-card');
        const huntText = text('#hud-hunt');
        const playerLevel = text('#tr-level');
        const playerXp = text('#tr-xp-txt');

        const activePokemon =
            ativo && !ativo.classList.contains('vazio')
                ? text('#ativo-card').slice(0, 48)
                : '';

        const huntSelected =
            huntText.length > 0 &&
            !/^escolha um mapa\s*[→›-]?\s*$/i.test(huntText);

        let activity = 'Login';
        if (loading && !loggedIn) {
            activity = 'Loading';
        } else if (loggedIn) {
            activity =
                huntSelected || visible('#golpes-painel')
                    ? 'Hunting'
                    : 'Center';
        }

        // Count only scripts reporting a version in this profile, rather than
        // treating every available userscript as mandatory.
        const readiness = {
            'autocatch': exists('#moth-ac-panel') || exists('#moth-ac-toggle'),
            'performance': exists('#moth-performance-panel') || exists('#moth-performance-trigger'),
            'stream-auto-open': exists('#moth-scan-live-streams'),
            'controller-bridge': (() => {
                const bridge = window.__mothControllerBridgeV1;
                return !!bridge && Number(bridge.version) >= 9
                    && typeof bridge.snapshot === 'function'
                    && typeof bridge.gameSnapshot === 'function'
                    && typeof bridge.streamStateSnapshot === 'function'
                    && typeof bridge.socket === 'function';
            })(),
            'twitch-low-resource': exists('#moth-twitch-low-resource-addon-ready') ||
                exists('#moth-twitch-low-resource-css'),
            'hunt-atlas': !!window.__mothHuntAtlasControllerV1,
            'market-bot': !!window.__mothMarketWatchControllerV1 ||
                document.documentElement?.dataset?.mothWatchReady === '1'
        };
        const displayNames = {
            'autocatch': 'Auto Catch+', 'performance': 'Performance+',
            'stream-auto-open': 'Stream Scanner', 'controller-bridge': 'Controller Bridge',
            'twitch-low-resource': 'Twitch Low Resource', 'hunt-atlas': 'Hunt Atlas',
            'market-bot': 'Moth Watch'
        };
        const prefix = 'data-moth-addon-version-';
        const addonChecks = [...(document.documentElement?.attributes || [])]
            .filter(attr => attr.name.startsWith(prefix))
            .map(attr => {
                const key = attr.name.slice(prefix.length);
                const version = String(attr.value || '');
                return { key, name: displayNames[key] || key, version,
                    ok: Object.hasOwn(readiness, key) ? !!readiness[key] : !!version };
            }).filter(entry => entry.key && entry.version);

        return JSON.stringify({
            url: location.href,
            title: document.title,
            gameReady: loggedIn,
            loggedIn,
            activity,
            hunt: loading && !loggedIn
                ? [loadingPercent ? loadingPercent + '%' : '', loadingStage].filter(Boolean).join(' · ')
                : huntSelected ? huntText.slice(0, 80) : '',
            activePokemon,
            fallenCount: document.querySelector('#caidos-lista')?.children.length || 0,
            economyMode: document.documentElement.classList.contains('modo-economia'),
            addons: addonChecks.filter(entry => entry.ok).map(entry => entry.name),
            addonTotal: addonChecks.length,
            addonMissing: addonChecks.filter(entry => !entry.ok).map(entry => entry.name),
            addonVersions: Object.fromEntries(addonChecks.filter(entry => entry.ok)
                .map(entry => [entry.key + '.user.js', entry.version])),
            performanceEnabled,
            performanceFps: text('#mpp-browser-fps'),
            performanceSceneRuns: text('#mpp-scene-runs'),
            playerLevel,
            playerXp,
            streamBonus: (() => {
                const sources = [
                    text('#tr-ativos'),
                    text('#evento-texto'),
                    text('#evento-faixa')
                ];
                return sources.find(value =>
                    /\+\s*15\s*%/i.test(value) &&
                    /\b(?:XP|EXP|experi)/i.test(value)
                ) || '';
            })()
        });
    })()"#;

    let expression = expression.replace("__MOTH_HEADLESS__", if headless { "true" } else { "false" });

    let result = send_and_wait(
        &mut session.socket,
        game_id,
        json!({
            "id": game_id,
            "method": "script.evaluate",
            "params": {
                "expression": expression,
                "target": { "context": game.id },
                "awaitPromise": false
            }
        }),
    )?;

    let raw = result
        .get("result")
        .and_then(|v| v.get("result"))
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Firefox returned no page probe value".to_string())?;

    let page: Value = serde_json::from_str(raw)
        .map_err(|error| format!("invalid page probe: {error}"))?;

    let addon_missing = page
        .get("addonMissing")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Older scripts have no marker; never infer their installed version.
    let addon_versions: BTreeMap<String, String> = page
        .get("addonVersions")
        .and_then(Value::as_object)
        .map(|entries| entries.iter().filter_map(|(file, version)| {
            let version = version.as_str()?;
            if version.len() > 40 || !version.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-')) {
                return None;
            }
            Some((file.clone(), version.to_string()))
        }).collect())
        .unwrap_or_default();

    let addon_total = page.get("addonTotal").and_then(Value::as_u64)
        .unwrap_or(0).min(u8::MAX as u64) as u8;
    let addon_ok = addon_total.saturating_sub(addon_missing.len().min(u8::MAX as usize) as u8);

    // The upstream game intentionally performs several large downloads and CPU-heavy
    // catalog transforms before the login screen appears. Do not compete with that boot
    // pipeline by evaluating every tab and serializing the full controller bridge snapshot
    // while the loading overlay is visible.
    let page_loading = page
        .get("activity")
        .and_then(Value::as_str)
        == Some("Loading");

    // getTree is needed to locate the game context, but per-tab script.evaluate
    // is only needed when top-level tabs change or the slow refresh expires.
    let signature: Vec<(String, String)> = contexts.iter()
        .filter(|context| context.depth == 0)
        .map(|context| (context.id.clone(), context.url.clone()))
        .collect();
    let tabs = if should_inspect_tabs(page_loading, force_tabs, &signature, &session.tab_signature) {
        let inspected = build_tab_infos(&contexts, &mut session.socket, &mut session.next_id);
        session.tab_signature = signature;
        Some(inspected)
    } else {
        None
    };

    let runtime = if page_loading {
        RuntimeProbe::default()
    } else {
        probe_runtime_details(session, &game.id).unwrap_or_else(|error| {
            logging::warn(&format!(
                "controller bridge runtime probe skipped on port {}: {}",
                session.game_context.as_deref().unwrap_or("unknown"),
                error
            ));
            RuntimeProbe::default()
        })
    };

    Ok(Probe {
        url: page.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        title: page.get("title").and_then(Value::as_str).unwrap_or_default().to_string(),
        game_ready: page.get("gameReady").and_then(Value::as_bool).unwrap_or(false),
        logged_in: page.get("loggedIn").and_then(Value::as_bool).unwrap_or(false),
        activity: page.get("activity").and_then(Value::as_str).unwrap_or("Unknown").to_string(),
        hunt: page.get("hunt").and_then(Value::as_str).unwrap_or_default().to_string(),
        hunt_slug: runtime.hunt_slug.clone(),
        hunt_atlas: runtime.hunt_atlas.clone(),
        active_pokemon: if runtime.active_pokemon.is_empty() {
            page.get("activePokemon")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        } else if runtime.pokemon_level.is_empty() {
            runtime.active_pokemon.clone()
        } else {
            format!("{} · Lv {}", runtime.active_pokemon, runtime.pokemon_level)
        },
        fallen_count: if runtime.bridge_connected {
            runtime.fallen_count
        } else {
            page.get("fallenCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32
        },
        economy_mode: page.get("economyMode").and_then(Value::as_bool).unwrap_or(false),
        addon_ok,
        addon_total,
        addon_missing,
        addon_versions,
        performance_enabled: page.get("performanceEnabled").and_then(Value::as_bool),
        performance_fps: page.get("performanceFps").and_then(Value::as_str).unwrap_or_default().to_string(),
        performance_scene_runs: page.get("performanceSceneRuns").and_then(Value::as_str).unwrap_or_default().to_string(),
        pokemon_level: runtime.pokemon_level,
        pokemon_xp: runtime.pokemon_xp,
        ball_stock: runtime.ball_stock,
        autocatch_on: runtime.autocatch_on,
        autocatch_captures: runtime.autocatch_captures,
        autocatch_balls_used: runtime.autocatch_balls_used,
        autocatch_rate: runtime.autocatch_rate,
        autocatch_restock: runtime.autocatch_restock,
        stream_scan_status: runtime.stream_scan_status,
        stream_scan_live: runtime.stream_scan_live,
        stream_scan_opened: runtime.stream_scan_opened,
        stream_scan_twitch_channels: runtime.stream_scan_twitch_channels,
        xp_bonuses: runtime.xp_bonuses,
        player_level: page.get("playerLevel").and_then(Value::as_str)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(runtime.player_level),
        player_xp: if page.get("playerXp").and_then(Value::as_str).unwrap_or_default().is_empty() {
            runtime.player_xp
        } else {
            page.get("playerXp").and_then(Value::as_str).unwrap_or_default().to_string()
        },
        player_total_xp: runtime.player_total_xp,
        player_xp_to_next: runtime.player_xp_to_next,
        gold: runtime.gold,
        orbs: runtime.orbs,
        stream_bonus: if runtime.stream_bonus.is_empty() {
            page.get("streamBonus").and_then(Value::as_str).unwrap_or_default().to_string()
        } else {
            runtime.stream_bonus.clone()
        },
        stream_bonus_pct: runtime.stream_bonus_pct,
        stream_watching: runtime.stream_watching,
        stream_live_bonus: runtime.stream_live_bonus,
        stream_missing: runtime.stream_missing,
        xp_sources: runtime.xp_sources,
        bridge_connected: runtime.bridge_connected,
        bridge_snapshot_ok: runtime.bridge_snapshot_ok,
        last_game_message_ms: runtime.last_game_message_ms,
        tabs,
        kick_streams: runtime.kick_streams,
        kick_state_available: runtime.kick_state_available,
    })
}
/// The bridge exposes cumulative XP and next-level threshold. Never infer a
/// level threshold from a text progress bar or assume it exists when missing.
fn remaining_trainer_xp(state: &Value) -> Option<u64> {
    let current = state.get("xp").and_then(Value::as_u64)?;
    let next = state.get("xpProximo").and_then(Value::as_u64)?;
    (next > current).then_some(next - current)
}

fn xp_progress(value: &Value, fallback: &str) -> String {
    let current = value.get("xp").and_then(Value::as_u64).unwrap_or(0);
    let floor = value.get("xpNivel").and_then(Value::as_u64).unwrap_or(0);
    let next = value.get("xpProximo").and_then(Value::as_u64).unwrap_or(0);

    if next > floor && current >= floor {
        return format!("{}/{}", current.saturating_sub(floor), next.saturating_sub(floor));
    }

    fallback.to_string()
}


fn trim_pct(value: f32) -> String {
    if value.fract().abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}


fn build_tab_infos(
    contexts: &[ContextInfo],
    socket: &mut BrowserSocket,
    next_id: &mut u64,
) -> Vec<TabInfo> {
    let mut tabs = Vec::new();

    for context in contexts.iter().filter(|context| context.depth == 0) {
        let lower = context.url.to_ascii_lowercase();
        let kind = if lower.starts_with("https://pokeidle.io/")
            || lower.starts_with("https://www.pokeidle.io/")
        {
            "PokéIdle"
        } else if lower.contains("twitch.tv") {
            "Twitch"
        } else if lower.contains("kick.com") {
            "KICK"
        } else if lower.starts_with("about:")
            || lower.starts_with("moz-extension:")
        {
            "Browser"
        } else {
            "Web"
        };

        let id = *next_id;
        *next_id += 1;

        let result = send_and_wait(
            socket,
            id,
            json!({
                "id": id,
                "method": "script.evaluate",
                "params": {
                    "expression": "JSON.stringify({ title: document.title || '', lowResource: !!document.querySelector('#moth-twitch-low-resource-css, #moth-twitch-low-resource-chat-css'), twitchChannels: (() => { const value = new URLSearchParams(location.search).get('moth-channels'); return value === null ? null : new Set(value.split(',').filter(Boolean)).size; })() })",
                    "target": { "context": context.id },
                    "awaitPromise": false
                }
            }),
        );

        let (title, low_resource, twitch_channels) = result
            .ok()
            .and_then(|value| {
                value.get("result")
                    .and_then(|v| v.get("result"))
                    .and_then(|v| v.get("value"))
                    .and_then(Value::as_str)
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            })
            .map(|value| (
                value.get("title").and_then(Value::as_str).unwrap_or_default().to_string(),
                value.get("lowResource").and_then(Value::as_bool).unwrap_or(false),
                if kind == "Twitch" {
                    value.get("twitchChannels").and_then(Value::as_u64).map(|n| n.min(u32::MAX as u64) as u32)
                } else { None },
            ))
            .unwrap_or_default();

        tabs.push(TabInfo {
            kind: kind.to_string(),
            title: if title.is_empty() { context.url.clone() } else { title },
            url: context.url.clone(),
            low_resource: kind == "Twitch" && low_resource,
            twitch_channels,
        });
    }

    tabs
}

fn probe_runtime_details(
    session: &mut BrowserSession,
    context_id: &str,
) -> Result<RuntimeProbe, String> {
    let id = session.next_id;
    session.next_id += 1;

    let expression = r#"(() => {
        const bridge = window.__mothControllerBridgeV1;
        if (!bridge || typeof bridge.snapshot !== 'function') {
            return JSON.stringify({ error: 'controller bridge is not installed' });
        }
        return JSON.stringify(bridge.snapshot());
    })()"#;

    let result = send_and_wait(
        &mut session.socket,
        id,
        json!({
            "id": id,
            "method": "script.evaluate",
            "params": {
                "expression": expression,
                "target": { "context": context_id },
                "awaitPromise": false
            }
        }),
    )?;

    let raw = result
        .get("result")
        .and_then(|v| v.get("result"))
        .and_then(|v| v.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Firefox returned no controller bridge snapshot".to_string())?;

    let snapshot: Value =
        serde_json::from_str(raw)
            .map_err(|error| format!("invalid controller bridge snapshot: {error}"))?;

    if let Some(error) = snapshot.get("error").and_then(Value::as_str) {
        return Err(error.to_string());
    }

    let stream = snapshot.get("stream").unwrap_or(&Value::Null);
    let kick_scanner = stream.get("kickScanner").unwrap_or(&Value::Null);
    let kick_state_available = kick_scanner
        .get("stateAvailable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let kick_streams = kick_scanner
        .get("live")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let url = item.get("url").and_then(Value::as_str)?.trim();
                    if url.is_empty() {
                        return None;
                    }
                    Some(KickStream {
                        url: url.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let state = snapshot.get("state").cloned().unwrap_or(Value::Null);
    let active = state.get("activePokemon").cloned().unwrap_or(Value::Null);

    let player_level =
        state.get("level").and_then(Value::as_u64).unwrap_or(0) as u32;

    let hunt_slug = state
        .get("huntSlug")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let bridge_connected = snapshot
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let last_game_message_ms = snapshot
        .get("lastMessageAt")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let player_xp = xp_progress(
        &state,
        snapshot
            .get("state")
            .and_then(|s| s.get("playerXp"))
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let fallen_count =
        state.get("fallen").and_then(Value::as_u64).unwrap_or(0) as u32;
    let gold = state.get("gold").and_then(Value::as_u64).unwrap_or(0);
    let orbs = state.get("orbs").and_then(Value::as_u64).unwrap_or(0);

    let active_pokemon = active
        .get("nome")
        .or_else(|| active.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let pokemon_level = active
        .get("level")
        .and_then(Value::as_u64)
        .map(|v| v.to_string())
        .unwrap_or_default();

    let pokemon_xp = xp_progress(
        &active,
        &state
            .get("activePokemonXp")
            .and_then(Value::as_u64)
            .map(|v| v.to_string())
            .unwrap_or_default(),
    );

    let mut ball_stock = Vec::new();
    if let Some(items) = state.get("domBalls").and_then(Value::as_array) {
        for item in items {
            let name = item.get("name").and_then(Value::as_str).unwrap_or("Ball");
            let count = item.get("count").and_then(Value::as_u64).unwrap_or(0);
            ball_stock.push(format!("{} {}", name, count));
        }
    }

    if ball_stock.is_empty() {
        if let Some(object) = state.get("balls").and_then(Value::as_object) {
            for (id, count) in object {
                ball_stock.push(format!("Ball {} {}", id, count));
            }
        }
    }

    let mut xp_bonuses = snapshot
        .get("state")
        .and_then(|s| s.get("xpBonuses"))
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();

    let twitch = state.get("twitch").cloned().unwrap_or(Value::Null);

    let stream_bonus_pct = twitch
        .get("pctAtual")
        .or_else(|| twitch.get("pct"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;

    let stream_watching = twitch
        .get("assistindoEm")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let official_bonus = twitch
        .get("oficiais")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter(|item| item.get("bonus").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|item| {
                    item.get("login")
                        .or_else(|| item.get("nome"))
                        .and_then(Value::as_str)
                        .map(|name| name.to_ascii_lowercase())
                })
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let live_names = twitch
        .get("lives")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(|item| {
                    let login = item.get("login").and_then(Value::as_str)?;
                    let login_key = login.to_ascii_lowercase();
                    if official_bonus.contains(&login_key) {
                        Some(login.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let stream_missing = live_names
        .iter()
        .filter(|name| {
            !stream_watching
                .iter()
                .any(|watching| watching.eq_ignore_ascii_case(name))
        })
        .cloned()
        .collect::<Vec<_>>();

    let stream_bonus = if !stream_watching.is_empty() && stream_bonus_pct > 0.0 {
        format!(
            "+{}% XP · watching {}",
            trim_pct(stream_bonus_pct),
            stream_watching.join(", ")
        )
    } else {
        String::new()
    };

    let mut xp_sources = Vec::new();

    if !stream_watching.is_empty() && stream_bonus_pct > 0.0 {
        xp_sources.push(stream_bonus.clone());
    } else if !stream_missing.is_empty() {
        let bonus_label = if stream_bonus_pct > 0.0 {
            format!("+{}% Twitch XP available", trim_pct(stream_bonus_pct))
        } else {
            "Twitch XP bonus available".to_string()
        };

        xp_sources.push(format!(
            "{} · bonus not registered for: {}",
            bonus_label,
            stream_missing.join(", ")
        ));
    }

    let event = state.get("evento").cloned().unwrap_or(Value::Null);
    let event_trainer = event
        .get("xpTreinadorPct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let event_pokemon = event
        .get("xpPokemonPct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    if event_trainer > 0.0 || event_pokemon > 0.0 {
        xp_sources.push(format!(
            "Event +{}% trainer / +{}% Pokémon XP",
            trim_pct(event_trainer as f32),
            trim_pct(event_pokemon as f32)
        ));
    }

    let guild_bonus = state
        .get("guildBonusPct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    if guild_bonus > 0.0 {
        xp_sources.push(format!("Guild +{}% XP", trim_pct(guild_bonus as f32)));
    }

    if state
        .get("loja")
        .and_then(|value| value.get("atrasados"))
        .is_some()
    {
        xp_sources.push("Delayed XP boost active".to_string());
    }

    if state
        .get("loja")
        .and_then(|value| value.get("boosts"))
        .and_then(Value::as_object)
        .map(|boosts| !boosts.is_empty())
        .unwrap_or(false)
    {
        xp_sources.push("Shop XP boost active".to_string());
    }

    if !stream_bonus.is_empty() && !xp_bonuses.iter().any(|item| item == &stream_bonus) {
        xp_bonuses.push(stream_bonus.clone());
    }

    Ok(RuntimeProbe {
        bridge_connected,
        bridge_snapshot_ok: true,
        hunt_slug,
        hunt_atlas: snapshot.get("huntAtlas").cloned().unwrap_or(Value::Null),
        player_level,
        player_xp,
        player_total_xp: state.get("xp").and_then(Value::as_u64),
        player_xp_to_next: remaining_trainer_xp(&state),
        gold,
        orbs,
        fallen_count,
        active_pokemon,
        pokemon_level,
        pokemon_xp,
        ball_stock,
        autocatch_on: state.get("autoCatchOn").and_then(Value::as_bool).unwrap_or(false),
        autocatch_captures: state.get("autoCatchCaptures").and_then(Value::as_u64).unwrap_or(0) as u32,
        autocatch_balls_used: state.get("autoCatchBallsUsed").and_then(Value::as_u64).unwrap_or(0) as u32,
        autocatch_rate: state.get("autoCatchRate").and_then(Value::as_str).unwrap_or_default().to_string(),
        autocatch_restock: state.get("autoCatchRestock").and_then(Value::as_str).unwrap_or_default().to_string(),
        stream_scan_status: state.get("streamScanStatus").and_then(Value::as_str).unwrap_or_default().to_string(),
        stream_scan_live: state.get("streamScanLive").and_then(Value::as_u64).unwrap_or(0) as u32,
        stream_scan_opened: state.get("streamScanOpened").and_then(Value::as_u64).unwrap_or(0) as u32,
        stream_scan_twitch_channels: state.get("streamScanTwitchChannels").and_then(Value::as_u64).map(|n| n.min(u32::MAX as u64) as u32),
        stream_bonus,
        stream_bonus_pct,
        stream_watching,
        stream_live_bonus: live_names,
        stream_missing,
        xp_sources,
        last_game_message_ms,
        xp_bonuses,
        kick_streams,
        kick_state_available,
    })
}

fn open_custom_tab(
    session: &mut BrowserSession,
    url: &str,
) -> Result<(), String> {
    let create_id = session.next_id;
    session.next_id += 1;

    let response = send_and_wait(
        &mut session.socket,
        create_id,
        json!({
            "id": create_id,
            "method": "browsingContext.create",
            "params": {
                "type": "tab"
            }
        }),
    )?;

    let context = response
        .get("result")
        .and_then(|value| value.get("context"))
        .and_then(Value::as_str)
        .ok_or_else(|| "custom tab creation returned no context".to_string())?
        .to_string();

    let navigate_id = session.next_id;
    session.next_id += 1;

    if let Err(error) = send_and_wait(
        &mut session.socket,
        navigate_id,
        json!({
            "id": navigate_id,
            "method": "browsingContext.navigate",
            "params": {
                "context": context,
                "url": url,
                "wait": "none"
            }
        }),
    ) {
        let close_id = session.next_id;
        session.next_id += 1;
        let _ = send_and_wait(
            &mut session.socket,
            close_id,
            json!({
                "id": close_id,
                "method": "browsingContext.close",
                "params": {
                    "context": context
                }
            }),
        );
        return Err(error);
    }

    logging::info(&format!("opened custom browser tab: {}", url));
    Ok(())
}

/// Discard old requests even if Firefox never becomes available; commands
/// should not unexpectedly open tabs several minutes after a user click.
fn expire_pending_commands(mailbox: &Arc<Mutex<CommandMailbox>>) {
    let Ok(mut queue) = mailbox.lock() else { return; };
    while queue.pending.front().is_some_and(|command| command.queued_at.elapsed() > COMMAND_MAX_WAIT) {
        let Some(command) = queue.pending.pop_front() else { break; };
        if queue.feedback.len() >= MAX_COMMAND_QUEUE {
            queue.feedback.pop_front();
        }
        queue.feedback.push_back(CommandFeedback {
            id: command.id,
            ok: false,
            message: "tab command expired while waiting for Firefox".to_string(),
        });
    }
}

fn flush_commands(
    session: &mut BrowserSession,
    mailbox: &Arc<Mutex<CommandMailbox>>,
) -> bool {
    let mut tabs_changed = false;
    loop {
        let command = match mailbox.lock() {
            Ok(mut queue) => queue.pending.pop_front(),
            Err(_) => {
                logging::error("browser command mailbox lock poisoned");
                return tabs_changed;
            }
        };
        let Some(command) = command else { break; };
        let outcome = if command.queued_at.elapsed() > COMMAND_MAX_WAIT {
            Err("tab command expired while waiting for Firefox".to_string())
        } else {
            match validate_open_tab(&command.payload) {
                Ok(url) => open_custom_tab(session, url),
                Err(error) => Err(error),
            }
        };
        let (ok, message) = match outcome {
            Ok(()) => {
                tabs_changed = true;
                (true, "tab opened".to_string())
            },
            Err(error) => {
                // No blind retry: Firefox might have created the tab before
                // losing the acknowledgement.
                logging::warn(&format!("browser command #{} failed: {}", command.id, error));
                (false, error)
            }
        };
        if let Ok(mut queue) = mailbox.lock() {
            if queue.feedback.len() >= MAX_COMMAND_QUEUE {
                queue.feedback.pop_front();
            }
            queue.feedback.push_back(CommandFeedback { id: command.id, ok, message });
        }
    }
    tabs_changed
}


#[derive(Clone)]
struct ContextInfo {
    id: String,
    url: String,
    depth: u8,
}

fn collect_contexts(value: &Value, out: &mut Vec<ContextInfo>, depth: u8) {
    let Some(id) = value.get("context").and_then(Value::as_str) else {
        return;
    };

    out.push(ContextInfo {
        id: id.to_string(),
        url: value.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        depth,
    });

    if let Some(children) = value.get("children").and_then(Value::as_array) {
        for child in children {
            collect_contexts(child, out, depth.saturating_add(1));
        }
    }
}

fn end_session(session: &mut Option<BrowserSession>) {
    let Some(mut session) = session.take() else {
        return;
    };

    let command_id = session.next_id;
    let _ = send_and_wait(
        &mut session.socket,
        command_id,
        json!({
            "id": command_id,
            "method": "session.end",
            "params": {}
        }),
    );

    let _ = session.socket.close(None);
}

fn send_and_wait<S>(
    socket: &mut WebSocket<S>,
    expected_id: u64,
    command: Value,
) -> Result<Value, String>
where
    S: Read + Write,
{
    socket
        .send(Message::Text(command.to_string().into()))
        .map_err(|error| error.to_string())?;

    loop {
        let message = socket.read().map_err(|error| error.to_string())?;

        let Message::Text(text) = message else {
            continue;
        };

        let value: Value = serde_json::from_str(text.as_ref())
            .map_err(|error| format!("invalid BiDi JSON: {error}"))?;

        if value.get("id").and_then(Value::as_u64) != Some(expected_id) {
            continue;
        }

        if value.get("type").and_then(Value::as_str) == Some("error") {
            return Err(value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Firefox BiDi command failed")
                .to_string());
        }

        if value
            .get("result")
            .and_then(|result| result.get("type"))
            .and_then(Value::as_str)
            == Some("exception")
        {
            let result = value.get("result").unwrap_or(&Value::Null);
            let details = result
                .get("exceptionDetails")
                .and_then(|details| details.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("script.evaluate raised a JavaScript exception");

            return Err(format!("script.evaluate exception: {details}"));
        }

        return Ok(value);
    }
}
#[cfg(test)]
mod command_tests {
    use super::{validate_open_tab, MAX_COMMAND_QUEUE};
    use serde_json::json;
    #[test]
    fn telemetry_is_not_current_until_a_probe_succeeds() {
        let mut health = super::Health::default();
        assert!(!health.telemetry_is_fresh());
        health.state = "Running".to_string();
        assert!(!health.telemetry_is_fresh());

        health.last_successful_probe_at = Some(std::time::Instant::now());
        assert!(health.telemetry_is_fresh());

        health.state = "Reconnecting".to_string();
        assert!(!health.telemetry_is_fresh());
        assert!(health.telemetry_age().is_some());
    }

    #[test]
    fn repeated_monitor_errors_are_rate_limited() {
        use super::{MonitorIssueReporter, MONITOR_ISSUE_LOG_INTERVAL};
        use std::time::Instant;

        let mut reporter = MonitorIssueReporter::default();
        assert!(reporter.should_log("connection failed"));
        reporter.last_logged = Some(("connection failed".to_string(), Instant::now()));
        assert!(!reporter.should_log("connection failed"));
        assert!(reporter.should_log("different failure"));
        reporter.last_logged = Some((
            "connection failed".to_string(),
            Instant::now() - MONITOR_ISSUE_LOG_INTERVAL,
        ));
        assert!(reporter.should_log("connection failed"));
    }

    #[test]
    fn freshness_window_covers_normal_poll_interval() {
        use super::{
            Health, MONITOR_POLL_INTERVAL, MONITOR_STARTUP_POLL_INTERVAL,
            TELEMETRY_FRESHNESS_LIMIT,
        };
        use std::time::{Duration, Instant};

        assert_eq!(MONITOR_POLL_INTERVAL, Duration::from_secs(3 * 60));
        assert_eq!(MONITOR_STARTUP_POLL_INTERVAL, Duration::from_secs(45));
        assert!(MONITOR_POLL_INTERVAL < TELEMETRY_FRESHNESS_LIMIT);
        let mut health = Health::default();
        health.state = "Running".to_string();
        health.last_successful_probe_at = Some(Instant::now() - Duration::from_secs(4 * 60));
        assert!(health.telemetry_is_fresh());

        health.last_successful_probe_at = Some(Instant::now() - Duration::from_secs(6 * 60));
        assert!(!health.telemetry_is_fresh());
    }

    #[test]
    fn tab_inventory_only_runs_on_change_or_slow_refresh() {
        use super::should_inspect_tabs;
        let a = vec![("tab1".to_string(), "https://pokeidle.io/app".to_string())];
        let b = vec![("tab2".to_string(), "https://www.twitch.tv".to_string())];
        assert!(!should_inspect_tabs(false, false, &a, &a));
        assert!(should_inspect_tabs(false, true, &a, &a));
        assert!(should_inspect_tabs(false, false, &b, &a));
        assert!(!should_inspect_tabs(true, true, &b, &a));
    }

    #[test]
    fn heartbeat_and_profile_stagger_are_independent_of_full_scans() {
        use super::{
            stagger_delay, Health, MONITOR_HEARTBEAT_INTERVAL, MONITOR_TAB_INTERVAL,
        };
        use std::time::{Duration, Instant};
        assert_eq!(MONITOR_HEARTBEAT_INTERVAL, Duration::from_secs(45));
        assert_eq!(MONITOR_TAB_INTERVAL, Duration::from_secs(10 * 60));
        assert_eq!(stagger_delay(27701, false), Duration::ZERO);
        assert_eq!(stagger_delay(27702, false), Duration::from_secs(12));
        assert_eq!(stagger_delay(27704, false), Duration::from_secs(36));
        assert_eq!(stagger_delay(27704, true), Duration::ZERO);
        let mut health = Health::default();
        health.state = "Running".to_string();
        health.last_heartbeat_at = Some(Instant::now());
        assert!(health.heartbeat_is_fresh());
        assert!(!health.telemetry_is_fresh()); // heartbeat is not a game snapshot
    }

    #[test]
    fn level_progress_uses_absolute_trainer_thresholds() {
        use super::remaining_trainer_xp;
        use serde_json::json;
        assert_eq!(remaining_trainer_xp(&json!({"xp": 1400, "xpProximo": 2000})), Some(600));
        assert_eq!(remaining_trainer_xp(&json!({"xp": 2000, "xpProximo": 2000})), None);
        assert_eq!(remaining_trainer_xp(&json!({"xp": 150, "xpProximo": 0})), None);
        assert_eq!(remaining_trainer_xp(&json!({"xp": 150})), None);
        assert_eq!(remaining_trainer_xp(&json!({"xp": "150", "xpProximo": 250})), None);
    }

    #[test]
    fn allow_http_and_https_tabs() {
        assert!(validate_open_tab(&json!({"t":"browser.openTab","url":"https://example.com"})).is_ok());
        assert!(validate_open_tab(&json!({"t":"browser.openTab","url":"http://localhost:3000"})).is_ok());
    }
    #[test]
    fn reject_invalid_tabs_and_unknown_commands() {
        assert!(validate_open_tab(&json!({"t":"browser.openTab","url":"file:///etc/passwd"})).is_err());
        assert!(validate_open_tab(&json!({"t":"browser.openTab","url":"https://"})).is_err());
        assert!(validate_open_tab(&json!({"t":"invalid","url":"https://example.com"})).is_err());
        assert_eq!(MAX_COMMAND_QUEUE, 16);
    }
}
