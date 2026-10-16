//! Authenticated Rust game transport with state-driven native gameplay commands.
//! Browser is only an authentication owner until the server welcomes Rust.
//!
//! Credentials are captured only from the existing, authenticated Firefox BiDi
//! session, held in memory, and never printed or persisted by this module.
use serde_json::Value;
use crate::native_twitch::{TwitchSession, TwitchSnapshot};
use crate::native_gameplay::{GameCommand, Gameplay};
use crate::native_hunts::{HuntIntel, HuntView};
use crate::native_restock::{Restock, RestockStatus};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client_tls, http::header, Message};
use tungstenite::client::IntoClientRequest;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const READ_TIMEOUT: Duration = Duration::from_millis(250);
const WELCOME_TIMEOUT: Duration = Duration::from_secs(15);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(25);
const CONNECTION_STALE_AFTER: Duration = Duration::from_secs(90);

#[derive(Clone, Debug)]
pub struct NativeSnapshot {
    pub phase: String,
    pub last_received: Option<Instant>,
    pub trainer_level: Option<u32>,
    pub trainer_xp: Option<u64>,
    pub gold: Option<u64>,
    pub orbs: Option<u64>,
    pub hunt_slug: Option<String>,
    pub frames: u64,
    pub battle_kills: u64,
    pub captures: u64,
    pub native_auto_catch: bool,
    pub native_capture_queue: usize,
    pub native_throws: u64,
    pub native_restock: RestockStatus,
    pub twitch_live: Vec<String>,
    pub twitch_watching: Vec<String>,
    pub twitch_bonus_pct: f32,
    pub error: Option<String>,
}
impl Default for NativeSnapshot {
    fn default() -> Self {
        Self {
            phase: "Connecting".into(), last_received: None,
            trainer_level: None, trainer_xp: None, gold: None, orbs: None,
            hunt_slug: None, frames: 0, battle_kills: 0, captures: 0, error: None,
            native_auto_catch: false, native_capture_queue: 0, native_throws: 0,
            native_restock: RestockStatus::default(),
            twitch_live: Vec::new(), twitch_watching: Vec::new(), twitch_bonus_pct: 0.0,
        }
    }
}

struct Credentials {
    url: String,
    host: String,
    port: u16,
    hello: String,
    cookies: String,
}
impl Credentials {
    fn parse(value: &Value) -> Result<Self, String> {
        let bootstrap = value.get("bootstrap")
            .ok_or_else(|| "Missing Firefox session bootstrap".to_string())?;
        let url = bootstrap.get("url").and_then(Value::as_str)
            .ok_or_else(|| "Missing WebSocket URL".to_string())?;
        let (host, port) = websocket_authority(url)?;
        let hello = bootstrap.get("hello").and_then(Value::as_str)
            .filter(|hello| !hello.is_empty() && hello.len() <= 32768)
            .ok_or_else(|| "Missing game hello".to_string())?;
        if serde_json::from_str::<Value>(hello).ok()
            .and_then(|v| v.get("t").and_then(Value::as_str).map(str::to_owned))
            .as_deref() != Some("hello")
        {
            return Err("The captured frame is not a game hello".into());
        }
        // BiDi storage.getCookies also returns cookies for unrelated domains.
        // Restrict Cookie to exactly those the browser could send to this WSS host.
        let mut cookie_parts = Vec::new();
        if let Some(entries) = value.get("cookies").and_then(Value::as_array) {
            for cookie in entries {
                let Some(domain) = cookie.get("domain").and_then(Value::as_str) else { continue };
                if !cookie_domain_matches(domain, &host) { continue; }
                let Some(name) = cookie.get("name").and_then(Value::as_str) else { continue };
                let Some(encoded) = cookie.get("value") else { continue };
                // Firefox BiDi represents cookie data as BytesValue. Do not
                // guess at base64 encoding or accidentally send literal JSON.
                let value = if encoded.get("type").and_then(Value::as_str) == Some("string") {
                    encoded.get("value").and_then(Value::as_str)
                } else {
                    encoded.as_str()
                };
                let Some(value) = value else { continue };
                if name.is_empty() || name.bytes().any(|b| b <= 32 || b >= 127 || b == b';' || b == b'=') ||
                    value.chars().any(|ch| matches!(ch, '\r' | '\n' | ';')) { continue; }
                cookie_parts.push(format!("{name}={value}"));
            }
        }
        Ok(Self { url: url.to_owned(), host, port,
            hello: hello.to_owned(), cookies: cookie_parts.join("; ") })
    }
}

fn websocket_authority(url: &str) -> Result<(String, u16), String> {
    let authority = url.strip_prefix("wss://")
        .ok_or_else(|| "Only encrypted wss:// endpoints are supported".to_owned())?
        .split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || authority.contains('[')
        || authority.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err("Invalid WebSocket authority".into());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>()
            .map_err(|_| "Invalid WebSocket port".to_owned())?),
        None => (authority, 443),
    };
    if !allowed_game_host(host) || port == 0 {
        return Err("Refusing native connection outside PokéIdle".into());
    }
    Ok((host.to_owned(), port))
}
fn allowed_game_host(host: &str) -> bool {
    host == "pokeidle.io" || host.ends_with(".pokeidle.io")
}
fn cookie_domain_matches(domain: &str, host: &str) -> bool {
    if let Some(domain) = domain.strip_prefix('.') {
        host == domain || host.ends_with(&format!(".{domain}"))
    } else {
        host == domain
    }
}

pub fn validate_bootstrap(bootstrap: &Value) -> Result<(), String> {
    Credentials::parse(bootstrap)?;
    let twitch = bootstrap.get("twitch")
        .ok_or_else(|| "Twitch IRC authentication missing from Firefox handoff".to_owned())?;
    crate::native_twitch::validate(twitch)
}

type GameSocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

pub struct NativeSession {
    snapshot: Arc<Mutex<NativeSnapshot>>,
    twitch: TwitchSession,
    stop: Arc<AtomicBool>,
    command_tx: mpsc::SyncSender<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    hunts: Arc<Mutex<HuntView>>,
}
impl NativeSession {
    pub fn start(bootstrap: Value) -> Result<Self, String> {
        let credentials = Credentials::parse(&bootstrap)?;
        let twitch = TwitchSession::start(bootstrap.get("twitch")
            .ok_or_else(|| "Twitch IRC bootstrap required".to_owned())?)?;
        Self::spawn(credentials, twitch, None, bootstrap.get("autocatch").cloned())
    }
    /// Establish *both* verified WebSockets before handing ownership away
    /// from Firefox. This is blocking and MUST run on a background thread.
    pub fn start_verified(bootstrap: Value) -> Result<Self, String> {
        let credentials = Credentials::parse(&bootstrap)?;
        let initial = open_session(&credentials)?;
        let twitch = TwitchSession::start_verified(bootstrap.get("twitch")
            .ok_or_else(|| "Twitch IRC bootstrap required".to_owned())?)?;
        Self::spawn(credentials, twitch, Some(initial), bootstrap.get("autocatch").cloned())
    }
    fn spawn(credentials: Credentials, twitch: TwitchSession,
        initial: Option<(GameSocket, String)>, settings: Option<Value>) -> Result<Self, String> {
        let desired = twitch.desired_handle();
        let gameplay = Gameplay::new(settings.as_ref());
        let restock = Restock::new(settings.as_ref());
        let hunt_intel = HuntIntel::new();
        let hunts = Arc::new(Mutex::new(hunt_intel.view.clone()));
        let worker_hunts = Arc::clone(&hunts);
        let restock_enabled = Arc::new(AtomicBool::new(restock.status.enabled));
        let worker_restock_enabled = Arc::clone(&restock_enabled);
        let auto_catch = Arc::new(AtomicBool::new(gameplay.status.auto_catch));
        let worker_auto_catch = Arc::clone(&auto_catch);
        let (command_tx, command_rx) = mpsc::sync_channel(64);
        let snapshot = Arc::new(Mutex::new(NativeSnapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new().name("moth-native-game".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session_loop(credentials, Arc::clone(&worker_snapshot),
                        Arc::clone(&worker_stop), desired, initial,
                        gameplay, restock, command_rx, worker_auto_catch,
                        worker_restock_enabled, hunt_intel, worker_hunts);
                }));
                if result.is_err() {
                    update(&worker_snapshot, |state| {
                        state.phase = "Failed".into();
                        state.error = Some("Native game worker panicked; inspect sanitized controller log".into());
                    });
                    worker_stop.store(true, Ordering::Relaxed);
                }
            })
            .map_err(|error| format!("Could not spawn native session: {error}"))?;
        Ok(Self { snapshot, twitch, stop, command_tx, auto_catch, restock_enabled, hunts })
    }
    pub fn send_command(&self, command: GameCommand) -> Result<(), String> {
        if let GameCommand::Hunt(slug) = &command {
            let current = self.hunt_snapshot();
            if !current.catalog_ready || !current.hunts.iter().any(|hunt| hunt.slug == *slug) {
                return Err("Requested hunt is not in the server-provided unlocked hunt list".into());
            }
        }
        if !command.is_valid() {
            return Err("Invalid native game command".into());
        }
        if self.snapshot().phase != "Online" {
            return Err("Native game is not authenticated".into());
        }
        self.command_tx.try_send(command)
            .map_err(|_| "Native command queue full or session stopped".to_owned())
    }
    pub fn hunt_snapshot(&self) -> HuntView {
        self.hunts.lock().map(|view| view.clone()).unwrap_or_default()
    }
    pub fn set_auto_catch(&self, enabled: bool) {
        self.auto_catch.store(enabled, Ordering::Relaxed);
    }
    pub fn set_restock_enabled(&self, enabled: bool) {
        self.restock_enabled.store(enabled, Ordering::Relaxed);
    }
    pub fn twitch_snapshot(&self) -> TwitchSnapshot {
        self.twitch.snapshot()
    }
    pub fn snapshot(&self) -> NativeSnapshot {
        self.snapshot.lock().map(|status| status.clone()).unwrap_or_default()
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.twitch.stop();
    }
}
impl Drop for NativeSession {
    fn drop(&mut self) { self.stop(); }
}

fn update(snapshot: &Arc<Mutex<NativeSnapshot>>, f: impl FnOnce(&mut NativeSnapshot)) {
    if let Ok(mut status) = snapshot.lock() { f(&mut status); }
}

fn open_session(credentials: &Credentials) -> Result<(
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    String,
), String> {
    crate::native_tls::ensure()?;
    let addrs = (credentials.host.as_str(), credentials.port)
        .to_socket_addrs().map_err(|_| "Could not resolve PokéIdle WebSocket".to_owned())?;
    // Try IPv4 and IPv6 rather than failing when the first DNS record is
    // unreachable on this machine.
    let mut stream = None;
    for address in addrs {
        if let Ok(connected) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            stream = Some(connected);
            break;
        }
    }
    let stream = stream.ok_or_else(|| "Could not connect to any PokéIdle WebSocket address".to_owned())?;
    stream.set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|error| format!("Native game read timeout: {error}"))?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|error| format!("Native game write timeout: {error}"))?;
    let mut request = credentials.url.as_str().into_client_request()
        .map_err(|_| "Invalid game handshake URL".to_owned())?;
    request.headers_mut().insert(header::ORIGIN,
        header::HeaderValue::from_static("https://pokeidle.io"));
    if !credentials.cookies.is_empty() {
        request.headers_mut().insert(header::COOKIE,
            header::HeaderValue::from_str(&credentials.cookies)
                .map_err(|_| "Invalid browser cookie header".to_owned())?);
    }
    let (mut socket, _) = client_tls(request, stream)
        .map_err(|error| {
            // Distinguish expired/invalid game authorization from transient
            // TCP/DNS/TLS outages. Only authentication loss may open Firefox.
            let response = error.to_string();
            if response.contains("401") || response.contains("403") {
                "AUTH_REQUIRED: PokéIdle refused native game authentication".to_owned()
            } else {
                format!("Game TLS/WebSocket handshake failed: {response}")
            }
        })?;
    socket.send(Message::Text(credentials.hello.clone().into()))
        .map_err(|error| format!("Could not send game hello: {error}"))?;
    let until = Instant::now() + WELCOME_TIMEOUT;
    while Instant::now() < until {
        match socket.read() {
            Ok(Message::Text(text)) => {
                if serde_json::from_str::<Value>(text.as_ref()).ok()
                    .and_then(|v| v.get("t").and_then(Value::as_str).map(str::to_owned))
                    .as_deref() == Some("welcome") {
                    // Receiving welcome confirms authentication. Keep the full
                    // initial state for the native read-only account dashboard.
                    return Ok((socket, text.to_string()));
                }
            }
            Ok(Message::Close(_)) => return Err("Server closed the native session before welcome".into()),
            Err(tungstenite::Error::Io(ref error))
                if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(error) => return Err(format!("Game rejected native hello: {error}")),
            _ => {}
        }
    }
    Err("Native game session did not receive welcome".into())
}

fn normalize_twitch_login(value: &str) -> Option<String> {
    let value = value.trim_start_matches('#').to_ascii_lowercase();
    if !value.is_empty() && value.len() <= 25 &&
        value.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') {
        Some(value)
    } else { None }
}

fn observe(snapshot: &Arc<Mutex<NativeSnapshot>>, text: &str,
    desired: &Arc<Mutex<Vec<String>>>) {
    let Ok(payload) = serde_json::from_str::<Value>(text) else { return };
    let state = payload.get("estado");
    update(snapshot, |status| {
        status.frames = status.frames.saturating_add(1);
        status.last_received = Some(Instant::now());
        if payload.get("t").and_then(Value::as_str) == Some("batalha") {
            if let Some(events) = payload.get("ev").and_then(Value::as_array) {
                for event in events {
                    match event.get("k").and_then(Value::as_str) {
                        Some("morte") => status.battle_kills = status.battle_kills.saturating_add(1),
                        Some("bola") if event.get("sucesso").and_then(Value::as_bool) == Some(true) =>
                            status.captures = status.captures.saturating_add(1),
                        _ => {}
                    }
                }
            }
        }
        if let Some(state) = state {
            if let Some(level) = state.get("level").and_then(Value::as_u64) {
                status.trainer_level = u32::try_from(level).ok();
            }
            if let Some(xp) = state.get("xp").and_then(Value::as_u64) {
                status.trainer_xp = Some(xp);
            }
            if let Some(gold) = state.get("gold").and_then(Value::as_u64) {
                status.gold = Some(gold);
            }
            if let Some(orbs) = state.get("orbs").and_then(Value::as_u64) {
                status.orbs = Some(orbs);
            }
            if let Some(slug) = state.get("huntSlug").and_then(Value::as_str) {
                status.hunt_slug = Some(slug.to_owned());
            }
            if let Some(twitch) = state.get("twitch") {
                if let Some(lives) = twitch.get("lives").and_then(Value::as_array) {
                    status.twitch_live = lives.iter()
                        .filter_map(|entry| {
                            let name = entry.get("login").or_else(|| entry.get("name"))
                                .and_then(Value::as_str)?;
                            normalize_twitch_login(name)
                        }).collect();
                }
                if let Some(watching) = twitch.get("assistindoEm").and_then(Value::as_array) {
                    status.twitch_watching = watching.iter()
                        .filter_map(Value::as_str).filter_map(normalize_twitch_login).collect();
                }
                if let Some(pct) = twitch.get("pctAtual").or_else(|| twitch.get("pct"))
                    .and_then(Value::as_f64) {
                    status.twitch_bonus_pct = pct as f32;
                }
            }
        }
    });
    // Use the same source of truth as stream-auto-open: game server live rooms
    // PLUS reward-active rooms, so a temporarily omitted stream is preserved.
    if let Ok(game) = snapshot.lock() {
        let mut names = game.twitch_live.clone();
        names.extend(game.twitch_watching.iter().cloned());
        names.sort_unstable();
        names.dedup();
        if let Ok(mut current) = desired.lock() {
            if *current != names { *current = names; }
        }
    }
}

fn session_loop(credentials: Credentials,
    snapshot: Arc<Mutex<NativeSnapshot>>, stop: Arc<AtomicBool>,
    desired: Arc<Mutex<Vec<String>>>, first_session: Option<(GameSocket, String)>,
    mut gameplay: Gameplay, mut restock: Restock,
    command_rx: mpsc::Receiver<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    mut hunt_intel: HuntIntel,
    hunts: Arc<Mutex<HuntView>>) {
    let mut rejected_welcomes = 0usize;
    let mut first_session = first_session;
    while !stop.load(Ordering::Relaxed) {
        match first_session.take().map(Ok).unwrap_or_else(|| open_session(&credentials)) {
            Ok((mut socket, welcome)) => {
                crate::logging::info("Native PokéIdle WSS received authenticated welcome");
                observe(&snapshot, &welcome, &desired);
                gameplay.ingest(&welcome, Instant::now());
                restock.ingest(&welcome, Instant::now());
                hunt_intel.ingest(&welcome, Instant::now());
                if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                let mut last_hunt_publish = Instant::now();
                update(&snapshot, |s| s.native_restock = restock.status.clone());
                rejected_welcomes = 0;
                update(&snapshot, |s| {
                    s.phase = "Online".into();
                    s.error = None;
                    s.last_received = Some(Instant::now());
                });
                let mut last_ping = Instant::now();
                loop {
                    if stop.load(Ordering::Relaxed) {
                        let _ = socket.close(None);
                        break;
                    }
                    if snapshot.lock().ok().and_then(|s| s.last_received)
                        .is_some_and(|last| last.elapsed() > CONNECTION_STALE_AFTER) { break; }
                    // Every mutating action has one owner: this game socket.
                    // Never enqueue commands against an older/disconnected
                    // browser or replay unconfirmed writes after reconnect.
                    let mut send_failed = false;
                    while let Ok(command) = command_rx.try_recv() {
                        let payload = command.wire().to_string();
                        if let GameCommand::Hunt(slug) = &command {
                            if let Err(error) = hunt_intel.request_hunt(slug, Instant::now()) {
                                crate::logging::warn(&format!("Native hunt selection rejected: {error}"));
                                continue;
                            }
                        }
                        if socket.send(Message::Text(payload.into())).is_err() {
                            gameplay.failed(&command, "native game command send failed");
                            if matches!(command, GameCommand::Hunt(_)) {
                                hunt_intel.command_failed("Hunt selection transport failed");
                            }
                            send_failed = true; break;
                        }
                    }
                    if send_failed { break; }
                    gameplay.status.auto_catch = auto_catch.load(Ordering::Relaxed);
                    restock.set_enabled(restock_enabled.load(Ordering::Relaxed));
                    if let Some(command) = gameplay.next_capture(Instant::now()) {
                        if socket.send(Message::Text(command.wire().to_string().into())).is_err() {
                            gameplay.failed(&command, "native auto capture send failed");
                            break;
                        }
                    }
                    if let Some(command) = restock.next(Instant::now(), gameplay.status.auto_catch) {
                        // Commands contain only category, product ID and
                        // quantity. Never log authentication or personal data.
                        if let GameCommand::ShopBuy { kind, id, qty } = &command {
                            crate::logging::info(&format!(
                                "Native restock: requesting {qty}x {kind} #{id}"
                            ));
                        }
                        if socket.send(Message::Text(command.wire().to_string().into())).is_err() {
                            restock.write_failed(Instant::now());
                            break;
                        }
                    }
                    if last_hunt_publish.elapsed() >= Duration::from_secs(1) {
                        if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                        last_hunt_publish = Instant::now();
                    }
                    update(&snapshot, |s| {
                        s.native_auto_catch = gameplay.status.auto_catch;
                        s.native_capture_queue = gameplay.status.queue;
                        s.native_throws = gameplay.status.throws;
                        s.native_restock = restock.status.clone();
                    });
                    if last_ping.elapsed() >= HEARTBEAT_INTERVAL {
                        if socket.send(Message::Ping(Vec::new().into())).is_err() { break; }
                        last_ping = Instant::now();
                    }
                    match socket.read() {
                        Ok(Message::Text(text)) => {
                            gameplay.ingest(text.as_ref(), Instant::now());
                            let confirmed_before = restock.status.confirmed;
                            restock.ingest(text.as_ref(), Instant::now());
                            hunt_intel.ingest(text.as_ref(), Instant::now());
                            if restock.status.confirmed > confirmed_before {
                                crate::logging::info(&format!(
                                    "Native restock: confirmed and reconciled {}",
                                    restock.status.last_order.as_deref().unwrap_or("purchase")
                                ));
                            }
                            observe(&snapshot, text.as_ref(), &desired);
                        },
                        Ok(Message::Pong(_)) | Ok(Message::Ping(_)) => {
                            update(&snapshot, |s| s.last_received = Some(Instant::now()));
                        }
                        Ok(Message::Close(_)) => break,
                        Err(tungstenite::Error::Io(ref error))
                            if matches!(error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                        Err(_) => break,
                        _ => {}
                    }
                }
                if !stop.load(Ordering::Relaxed) {
                    while command_rx.try_recv().is_ok() {}
                    restock.on_disconnect(Instant::now());
                    hunt_intel.on_disconnect();
                    if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                    update(&snapshot, |s| {
                        s.phase = "Reconnecting".into();
                        s.native_restock = restock.status.clone();
                    });
                }
            }
            Err(error) => {
                let expired = error.starts_with("AUTH_REQUIRED:");
                if error.contains("before welcome") ||
                   error.contains("did not receive welcome") {
                    rejected_welcomes += 1;
                } else {
                    rejected_welcomes = 0;
                }
                // Consecutive pre-welcome rejections after a valid handshake
                // may indicate that the user's Firefox login must be renewed.
                // Ordinary network failures NEVER launch Firefox.
                let needs_login = expired || rejected_welcomes >= 5;
                update(&snapshot, |s| {
                    s.phase = if needs_login { "NeedsLogin" } else { "Reconnecting" }.into();
                    s.error = Some(error);
                });
                if needs_login { break; }
            }
        }
        for _ in 0..10 {
            if stop.load(Ordering::Relaxed) { break; }
            thread::sleep(Duration::from_millis(500));
        }
    }
    if stop.load(Ordering::Relaxed) {
        update(&snapshot, |s| s.phase = "Stopped".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_non_game_and_unencrypted_endpoints() {
        assert!(websocket_authority("ws://pokeidle.io/ws").is_err());
        assert!(websocket_authority("wss://pokeidle.io.evil.test/ws").is_err());
        assert!(websocket_authority("wss://someone@pokeidle.io/ws").is_err());
        assert!(websocket_authority("wss://ws.pokeidle.io/ws").is_ok());
    }
    #[test]
    fn records_battle_events_without_requiring_browser_dom() {
        let status = Arc::new(Mutex::new(NativeSnapshot::default()));
        let desired = Arc::new(Mutex::new(Vec::new()));
        observe(&status, r#"{"t":"batalha","ev":[{"k":"morte"},{"k":"bola","sucesso":true}]}"#, &desired);
        let observed = status.lock().unwrap().clone();
        assert_eq!(observed.battle_kills, 1);
        assert_eq!(observed.captures, 1);
        assert_eq!(observed.frames, 1);
    }
    #[test]
    fn native_twitch_follows_lives_and_protects_rewarded_rooms() {
        let status = Arc::new(Mutex::new(NativeSnapshot::default()));
        let desired = Arc::new(Mutex::new(Vec::new()));
        observe(&status, r#"{"t":"welcome","estado":{"twitch":{"lives":[{"login":"ChannelA"}],"assistindoEm":["OldChannel"],"pctAtual":10}}}"#, &desired);
        assert_eq!(*desired.lock().unwrap(), vec!["channela", "oldchannel"]);
        observe(&status, r#"{"t":"estado","estado":{"twitch":{"lives":[]}}}"#, &desired);
        assert_eq!(*desired.lock().unwrap(), vec!["oldchannel"]);
        assert_eq!(status.lock().unwrap().twitch_bonus_pct, 10.0);
    }
    #[test]
    fn cookie_scoping_and_hello_validation() {
        let input = json!({
            "bootstrap": {"url": "wss://ws.pokeidle.io/game", "hello": r#"{"t":"hello","delta":1}"#},
            "cookies": [
                {"domain": ".pokeidle.io", "name": "session", "value": {"type":"string","value":"private"}},
                {"domain": "elsewhere.example", "name": "bad", "value": {"type":"string","value":"ignored"}},
                {"domain": ".pokeidle.io", "name": "bad;name", "value": {"type":"string","value":"ignored"}}
            ]
        });
        let parsed = Credentials::parse(&input).expect("valid bootstrap");
        assert_eq!(parsed.cookies, "session=private");
        assert!(Credentials::parse(&json!({"bootstrap":{"url":"wss://pokeidle.io/ws","hello":"{}"}})).is_err());
    }
}
