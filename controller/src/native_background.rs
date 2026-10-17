//! Authenticated Rust game transport with state-driven native gameplay commands.
//! Browser is only an authentication owner until the server welcomes Rust.
//!
//! Credentials are captured only from the existing, authenticated Firefox BiDi
//! session, held in memory, and never printed or persisted by this module.
use serde_json::Value;
use crate::native_twitch::{TwitchSession, TwitchSnapshot};
use crate::native_gameplay::{GameCommand, Gameplay};
use crate::native_capture_config::CatchSettings;
use crate::native_gameplay::GameplayStatus;
use crate::native_hunts::{HuntIntel, HuntView};
use crate::native_history::{HistoryRecorder, HistorySnapshot, HistoryForecast};
use crate::config::GameProfile;
use crate::native_restock::{Restock, RestockStatus};
use crate::native_automation::{AutomationWriter, AutomationStatus};
use crate::native_market::{MarketBuyer, MarketRules, MarketStatus};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client_tls, http::header, Message};
use tungstenite::client::IntoClientRequest;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
// A 250 ms blocking read imposed a hidden floor on every rule-buy dispatch.
// 60 ms permits Watch's 120 ms pacing while keeping the native worker blocking
// (not spinning) when the authenticated game has no market activity.
const READ_TIMEOUT: Duration = Duration::from_millis(60);
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
    pub native_capture_status: GameplayStatus,
    pub native_restock: RestockStatus,
    pub native_automation: AutomationStatus,
    pub native_market: MarketStatus,
    pub pending_pokemon_sale: Option<u64>,
    pub confirmed_pokemon_sales: u64,
    pub pokemon_sale_error: Option<String>,
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
            native_capture_status: GameplayStatus::default(),
            native_restock: RestockStatus::default(),
            native_automation: AutomationStatus::default(),
            native_market: MarketStatus::default(),
            pending_pokemon_sale: None, confirmed_pokemon_sales: 0,
            pokemon_sale_error: None,
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
    profile: GameProfile,
    twitch: TwitchSession,
    stop: Arc<AtomicBool>,
    command_tx: mpsc::SyncSender<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    capture_config: Arc<Mutex<CatchSettings>>,
    market_rules: Arc<Mutex<MarketRules>>,
    session_reset: Arc<AtomicBool>,
    hunts: Arc<Mutex<HuntView>>,
    history: Arc<Mutex<HistorySnapshot>>,
    forecast_tx:mpsc::SyncSender<HistoryForecast>,
}
impl NativeSession {
    /// Establish *both* verified WebSockets before handing ownership away
    /// from Firefox. This is blocking and MUST run on a background thread.
    pub fn start_verified(profile: GameProfile, bootstrap: Value) -> Result<Self, String> {
        let credentials = Credentials::parse(&bootstrap)?;
        let initial = open_session(&credentials)?;
        let twitch = TwitchSession::start_verified(bootstrap.get("twitch")
            .ok_or_else(|| "Twitch IRC bootstrap required".to_owned())?)?;
        Self::spawn(profile, credentials, twitch, Some(initial), bootstrap.get("autocatch").cloned())
    }
    fn spawn(profile: GameProfile, credentials: Credentials, twitch: TwitchSession,
        initial: Option<(GameSocket, String)>, settings: Option<Value>) -> Result<Self, String> {
        let desired = twitch.desired_handle();
        let gameplay = Gameplay::new(settings.as_ref());
        let restock = Restock::new(settings.as_ref());
        let rules=MarketRules::load(profile).unwrap_or_else(|error| {
            crate::logging::warn(&format!("Market rule settings unavailable; disabled: {error}"));
            MarketRules::default()
        });
        let market_rules=Arc::new(Mutex::new(rules));
        let worker_market_rules=Arc::clone(&market_rules);
        let capture_config=Arc::new(Mutex::new(CatchSettings::from_value(settings.as_ref())));
        let worker_capture_config=Arc::clone(&capture_config);
        let session_reset=Arc::new(AtomicBool::new(false));
        let worker_session_reset=Arc::clone(&session_reset);
        let hunt_intel = HuntIntel::new();
        let nick = serde_json::from_str::<Value>(&credentials.hello).ok()
            .and_then(|hello| hello.get("nick").and_then(Value::as_str).map(str::to_owned))
            .filter(|name| !name.is_empty()).unwrap_or_else(||profile.name().to_owned());
        let history_recorder = HistoryRecorder::new(profile, nick);
        let history = Arc::clone(&history_recorder.shared);
        let hunts = Arc::new(Mutex::new(hunt_intel.view.clone()));
        let worker_hunts = Arc::clone(&hunts);
        let restock_enabled = Arc::new(AtomicBool::new(restock.status.enabled));
        let worker_restock_enabled = Arc::clone(&restock_enabled);
        let auto_catch = Arc::new(AtomicBool::new(gameplay.status.auto_catch));
        let worker_auto_catch = Arc::clone(&auto_catch);
        let (command_tx, command_rx) = mpsc::sync_channel(64);
        let (forecast_tx,forecast_rx)=mpsc::sync_channel(8);
        let snapshot = Arc::new(Mutex::new(NativeSnapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new().name("moth-native-game".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session_loop(profile, credentials, Arc::clone(&worker_snapshot),
                        Arc::clone(&worker_stop), desired, initial,
                        gameplay, restock, command_rx, worker_auto_catch,
                        worker_restock_enabled, worker_capture_config,
                        worker_market_rules, worker_session_reset, hunt_intel, worker_hunts,
                        history_recorder,forecast_rx);
                }));
                if result.is_err() {
                    update(&worker_snapshot, |state| {
                        state.phase = "Failed".into();
                        state.error = Some("PokéIdle connection stopped unexpectedly. See the controller log.".into());
                    });
                    worker_stop.store(true, Ordering::Relaxed);
                }
            })
            .map_err(|error| format!("Could not start the PokéIdle connection: {error}"))?;
        Ok(Self {
            snapshot, profile, twitch, stop, command_tx, auto_catch, restock_enabled,
            capture_config, market_rules, session_reset, hunts, history, forecast_tx
        })
    }
    pub fn set_market_rules(&self, rules: MarketRules) -> Result<(), String> {
        rules.save(self.profile)?;
        let mut live=self.market_rules.lock().map_err(|_|"Market settings unavailable".to_owned())?;
        *live=rules;
        Ok(())
    }
    pub fn send_command(&self, command: GameCommand) -> Result<(), String> {
        if let GameCommand::OutlandTier(tier)=&command{
            let view=self.hunt_snapshot();
            if view.outland_tier.is_none(){
                return Err("Waiting for the server to report the current Outland tier".into());
            }
            if view.pending_outland_tier.is_some(){
                return Err("An Outland tier change is already in progress".into());
            }
            if view.outland_tier==Some(*tier){
                return Err("The requested Outland tier is already active".into());
            }
            if crate::native_atlas_models::unlocked_tier(*tier,view.trainer_level).is_none(){
                return Err("The requested Outland tier is not unlocked at this trainer level".into());
            }
        }
        if let GameCommand::Hunt(slug) = &command {
            let current = self.hunt_snapshot();
            if !current.catalog_ready || !current.hunts.iter().any(|hunt| hunt.slug == *slug) {
                return Err("Requested hunt is not in the server-provided unlocked hunt list".into());
            }
        }
        if !command.is_valid() {
            return Err("This game action is not available".into());
        }
        if let GameCommand::SellPokemon(id) = &command {
            let view=self.hunt_snapshot();
            let pokemon=view.pokemon_roster.iter().find(|p|p.id==*id)
                .ok_or_else(|| "Pokémon is not present in the authoritative roster".to_owned())?;
            if view.active_pokemon.as_ref().is_some_and(|p|p.id==*id) {
                return Err("Cannot sell the active Pokémon".into());
            }
            if pokemon.shiny {
                return Err("Shiny Pokémon cannot be sold automatically".into());
            }
            if self.snapshot().pending_pokemon_sale.is_some() {
                return Err("Wait for the previous Pokémon sale to be confirmed".into());
            }
        }
        if self.snapshot().phase != "Online" {
            return Err("Sign in to PokéIdle before using this feature".into());
        }
        self.command_tx.try_send(command)
            .map_err(|_| "PokéIdle is busy or disconnected. Try again shortly.".to_owned())
    }
    pub fn reset_hunt_session(&self) {
        self.session_reset.store(true,Ordering::Relaxed);
    }
    pub fn hunt_snapshot(&self) -> HuntView {
        self.hunts.lock().map(|view| view.clone()).unwrap_or_default()
    }
    pub fn history_snapshot(&self) -> HistorySnapshot {
        self.history.lock().map(|data|data.clone()).unwrap_or_default()
    }
    /// Nonblocking delivery to the running worker. Never write to disk on egui.
    pub fn archive_hunt_forecast(&self,hunt:&str,
        trainer_xp_h:f64,pokemon_xp_h:f64,kills_h:f64,calibrated:bool)->bool{
        if self.snapshot().phase!="Online"{return false}
        let Some(sample)=HistoryForecast::now(hunt,trainer_xp_h,
            pokemon_xp_h,kills_h,calibrated)else{return false};
        self.forecast_tx.try_send(sample).is_ok()
    }
    pub fn capture_settings(&self)->CatchSettings {
        self.capture_config.lock().map(|g|g.clone()).unwrap_or_default()
    }
    pub fn set_capture_settings(&self, settings:CatchSettings)->Result<(),String> {
        settings.validate()?;
        let mut current=self.capture_config.lock()
            .map_err(|_|"Capture preferences currently unavailable".to_owned())?;
        *current=settings.clone();
        self.auto_catch.store(settings.enabled,Ordering::Relaxed);
        self.restock_enabled.store(settings.restock_enabled,Ordering::Relaxed);
        Ok(())
    }
    pub fn set_auto_catch(&self, enabled: bool) {
        self.auto_catch.store(enabled, Ordering::Relaxed);
        if let Ok(mut cfg)=self.capture_config.lock() {cfg.enabled=enabled;}
    }
    pub fn set_restock_enabled(&self, enabled: bool) {
        self.restock_enabled.store(enabled, Ordering::Relaxed);
        if let Ok(mut cfg)=self.capture_config.lock() {cfg.restock_enabled=enabled;}
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

// Return to low-latency reads only AFTER TLS, WebSocket upgrade and the
// server's authenticated welcome are all complete.
#[allow(unreachable_patterns)] // TLS feature sets expose different stream variants.
fn set_live_read_timeout(socket: &mut GameSocket) -> Result<(), String> {
    let result = match socket.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(tcp) =>
            tcp.set_read_timeout(Some(READ_TIMEOUT)),
        tungstenite::stream::MaybeTlsStream::Rustls(tls) =>
            tls.get_mut().set_read_timeout(Some(READ_TIMEOUT)),
        _ => return Err("Unsupported native WebSocket TLS stream".into()),
    };
    result.map_err(|error| format!("Could not configure live PokéIdle updates: {error}"))
}

fn open_session(credentials: &Credentials) -> Result<(GameSocket, String), String> {
    crate::native_tls::ensure()?;
    let addrs = (credentials.host.as_str(), credentials.port)
        .to_socket_addrs().map_err(|_| "Could not find the PokéIdle server. Check your connection.".to_owned())?;
    // Try IPv4 and IPv6 rather than failing when the first DNS record is
    // unreachable on this machine.
    let mut stream = None;
    for address in addrs {
        if let Ok(connected) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            stream = Some(connected);
            break;
        }
    }
    let stream = stream.ok_or_else(|| "Could not connect to PokéIdle. Check your internet connection.".to_owned())?;
    // TLS and the initial authenticated welcome are network round trips.
    // The 60 ms steady-state polling timeout must never apply to the
    // handshake: it can abort valid connections before the server welcomes us.
    stream.set_read_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|error| format!("PokéIdle handshake timeout setup failed: {error}"))?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|error| format!("Could not send data to PokéIdle: {error}"))?;
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
                format!("Secure connection to PokéIdle failed: {response}")
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
                    set_live_read_timeout(&mut socket)?;
                    return Ok((socket, text.to_string()));
                }
            }
            Ok(Message::Close(_)) => return Err("PokéIdle closed the connection during sign-in".into()),
            Err(tungstenite::Error::Io(ref error))
                if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(error) => return Err(format!("PokéIdle could not complete sign-in: {error}")),
            _ => {}
        }
    }
    Err("PokéIdle did not confirm the connection".into())
}

fn normalize_twitch_login(value: &str) -> Option<String> {
    let value = value.trim_start_matches('#').to_ascii_lowercase();
    if !value.is_empty() && value.len() <= 25 &&
        value.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') {
        Some(value)
    } else { None }
}

#[cfg(test)]
fn observe(snapshot: &Arc<Mutex<NativeSnapshot>>, text: &str,
    desired: &Arc<Mutex<Vec<String>>>) {
    let Ok(payload) = serde_json::from_str::<Value>(text) else { return };
    observe_value(snapshot, &payload, desired);
}
fn observe_value(snapshot: &Arc<Mutex<NativeSnapshot>>, payload: &Value,
    desired: &Arc<Mutex<Vec<String>>>) {
    let state = payload.get("estado");
    update(snapshot, |status| {
        status.frames = status.frames.saturating_add(1);
        status.last_received = Some(Instant::now());
        if payload.get("t").and_then(Value::as_str) == Some("batalha") {
            if let Some(events) = payload.get("ev").and_then(Value::as_array) {
                for event in events {
                    match event.get("k").and_then(Value::as_str) {
                        Some("morte") if event.get("quem").and_then(Value::as_str) == Some("selvagem") =>
                            status.battle_kills = status.battle_kills.saturating_add(1),
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

fn session_loop(profile: GameProfile, credentials: Credentials,
    snapshot: Arc<Mutex<NativeSnapshot>>, stop: Arc<AtomicBool>,
    desired: Arc<Mutex<Vec<String>>>, first_session: Option<(GameSocket, String)>,
    mut gameplay: Gameplay, mut restock: Restock,
    command_rx: mpsc::Receiver<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    capture_config: Arc<Mutex<CatchSettings>>,
    market_rules: Arc<Mutex<MarketRules>>,
    session_reset: Arc<AtomicBool>,
    mut hunt_intel: HuntIntel,
    hunts: Arc<Mutex<HuntView>>,
    mut history_recorder: HistoryRecorder,
    forecast_rx:mpsc::Receiver<HistoryForecast>) {
    let mut rejected_welcomes = 0usize;
    let mut first_session = first_session;
    let mut automation_writer = AutomationWriter::new();
    let mut market = MarketBuyer::new(profile);
    let mut pending_sale: Option<(u64, Instant)> = None;
    // Metadata is downloaded without cookies/credentials, and only if a
    // configured healing-item rule lacks a server shop price. Never block the
    // game WebSocket reader with HTTP requests.
    let (metadata_tx, metadata_rx) = mpsc::channel::<Result<crate::native_items::ItemCatalog, String>>();
    let mut item_metadata: Option<crate::native_items::ItemCatalog> = None;
    let mut last_metadata_attempt: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        match first_session.take().map(Ok).unwrap_or_else(|| open_session(&credentials)) {
            Ok((mut socket, welcome)) => {
                crate::logging::info("Native PokéIdle WSS received authenticated welcome");
                // A server frame has one parsed representation. All consumers
                // inspect borrowed values instead of deserializing it again.
                let Ok(frame) = serde_json::from_str::<Value>(&welcome) else { continue };
                let received_at = Instant::now();
                observe_value(&snapshot, &frame, &desired);
                gameplay.ingest_value(&frame, received_at);
                restock.ingest_value(&frame, received_at);
                let initial_market_cfg=market_rules.lock().map(|v|v.clone()).unwrap_or_default();
                market.ingest_value(&frame, received_at, &initial_market_cfg);
                if let Some(catalog) = item_metadata.as_ref() {
                    restock.merge_item_metadata(catalog);
                }
                hunt_intel.ingest_value(&frame, received_at);
                history_recorder.ingest_value(&frame,hunt_intel.view.hunt_slug.as_deref());
                if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                let mut last_hunt_publish = Instant::now();
                let mut last_config = capture_config.lock().map(|v|v.clone())
                    .unwrap_or_default();
                gameplay.configure(&last_config);
                restock.configure(&last_config);
                restock.publish_status();
                update(&snapshot, |s| {
                    s.native_restock = restock.status.clone();
                    s.native_automation = automation_writer.status.clone();
                    s.native_market = market.status.clone();
                    s.native_capture_status = gameplay.status.clone();
                });
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
                        if let GameCommand::SetAutomation {field,value} = &command {
                            if let Err(error) = automation_writer.queue(field.clone(),value.clone()) {
                                crate::logging::warn(&format!(
                                    "Native server automation update rejected: {error}"
                                ));
                            }
                            continue;
                        }
                        if let GameCommand::SellPokemon(id) = &command {
                            if pending_sale.is_some() {
                                crate::logging::warn("Native Pokémon sale already awaiting confirmation");
                                continue;
                            }
                            let current = hunt_intel.snapshot();
                            if current.active_pokemon.as_ref().is_some_and(|p|p.id==*id)
                                || !current.pokemon_roster.iter().any(|p|p.id==*id && !p.shiny) {
                                crate::logging::warn("Native Pokémon sale rejected after refreshed server state");
                                continue;
                            }
                        }
                        let payload = command.wire().to_string();
                        if let GameCommand::Hunt(slug) = &command {
                            if let Err(error) = hunt_intel.request_hunt(slug, Instant::now()) {
                                crate::logging::warn(&format!("Native hunt selection rejected: {error}"));
                                continue;
                            }
                        }
                        if let GameCommand::OutlandTier(tier)=&command{
                            // Revalidate against the latest server-derived level,
                            // not an earlier UI snapshot or guessed game tier.
                            if let Err(error)=hunt_intel.request_outland_tier(
                                *tier,Instant::now()){
                                crate::logging::warn(&format!(
                                    "Native Outland tier request rejected: {error}"
                                ));
                                hunt_intel.view.outland_tier_error=Some(error);
                                continue;
                            }
                        }
                        if socket.send(Message::Text(payload.into())).is_err() {
                            gameplay.failed(&command, "Could not send the action to PokéIdle");
                            if matches!(command, GameCommand::Hunt(_)) {
                                hunt_intel.command_failed("Hunt selection transport failed");
                            }
                            if matches!(command, GameCommand::OutlandTier(_)){
                                hunt_intel.outland_command_failed(
                                    "Outland tier command could not be sent"
                                );
                            }
                            send_failed = true; break;
                        }
                        if let GameCommand::SellPokemon(id) = command {
                            pending_sale = Some((id,Instant::now()));
                            update(&snapshot, |s| {
                                s.pending_pokemon_sale=Some(id);
                                s.pokemon_sale_error=None;
                            });
                        }
                    }
                    if send_failed { break; }
                    if pending_sale.as_ref().is_some_and(|(_,at)|
                        at.elapsed()>Duration::from_secs(20)) {
                        pending_sale=None;
                        update(&snapshot, |s| {
                            s.pending_pokemon_sale=None;
                            s.pokemon_sale_error=Some("Pokémon sale not confirmed within 20 seconds".into());
                        });
                    }
                    if let Some(fields) = automation_writer.next(
                        hunt_intel.view.automation.as_ref(),Instant::now()) {
                        if socket.send(Message::Text(
                            GameCommand::AutoSet(fields).wire().to_string().into()
                        )).is_err() {
                            automation_writer.send_failed();
                            send_failed=true;
                        }
                    }
                    if send_failed { break; }
                    if session_reset.swap(false,Ordering::Relaxed) {
                        hunt_intel.reset_session(Instant::now());
                    }
                    if let Ok(cfg) = capture_config.lock() {
                        if *cfg != last_config {
                            gameplay.configure(&cfg);
                            restock.configure(&cfg);
                            last_config=cfg.clone();
                        }
                    }
                    gameplay.status.auto_catch = auto_catch.load(Ordering::Relaxed);
                    restock.set_enabled(restock_enabled.load(Ordering::Relaxed));
                    while let Ok(result) = metadata_rx.try_recv() {
                        match result {
                            Ok(catalog) => {
                                crate::logging::info(&format!(
                                    "Native restock: public metadata available for {} item IDs",
                                    catalog.items.len()
                                ));
                                restock.merge_item_metadata(&catalog);
                                item_metadata = Some(catalog);
                            }
                            Err(error) => crate::logging::warn(&format!(
                                "Native restock: optional public item catalog unavailable: {error}"
                            )),
                        }
                    }
                    if restock.needs_item_metadata() {
                        if let Some(catalog) = item_metadata.as_ref() {
                            restock.merge_item_metadata(catalog);
                        } else if last_metadata_attempt.is_none_or(|at|
                            at.elapsed() >= Duration::from_secs(300)) {
                            last_metadata_attempt = Some(Instant::now());
                            let sender = metadata_tx.clone();
                            thread::spawn(move || {
                                let result = std::panic::catch_unwind(
                                    crate::native_items::load
                                ).unwrap_or_else(|_|Err(
                                    "Public item metadata loader panicked".into()
                                ));
                                let _ = sender.send(result);
                            });
                        }
                    }
                    // Rule buys share this authenticated game socket.
                    let market_cfg=market_rules.lock().map(|v|v.clone()).unwrap_or_default();
                    // Snatch owns the four-second release lottery. Send up to
                    // eight authorized maximum-quantity attempts per wake,
                    // prioritizing each queued listing's first attempt. Exact
                    // market.anuncio verifications never wait behind ordinary
                    // rule-scanning, and share the same authenticated socket.
                    let snatch_payloads=market.snatch_actions(&market_cfg,Instant::now());
                    let mut snatch_send_failed=false;
                    for payload in snatch_payloads {
                        if socket.send(Message::Text(payload.to_string().into())).is_err() {
                            market.send_failed();
                            snatch_send_failed=true;
                            break;
                        }
                    }
                    if snatch_send_failed {break;}
                    if let Some(payload)=market.next(&market_cfg,Instant::now()) {
                        if socket.send(Message::Text(payload.to_string().into())).is_err() {
                            market.send_failed();
                            break;
                        }
                    }
                    if let Some(command) = gameplay.next_capture(Instant::now()) {
                        if socket.send(Message::Text(command.wire().to_string().into())).is_err() {
                            gameplay.failed(&command, "Could not throw a Poké Ball");
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
                    while let Ok(sample)=forecast_rx.try_recv(){
                        history_recorder.record_forecast(sample);
                    }
                    history_recorder.maybe_flush(false);
                    if last_hunt_publish.elapsed() >= Duration::from_secs(1) {
                        if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                        last_hunt_publish = Instant::now();
                    }
                    restock.publish_status();
                    update(&snapshot, |s| {
                        s.native_automation = automation_writer.status.clone();
                        s.native_auto_catch = gameplay.status.auto_catch;
                        s.native_capture_queue = gameplay.status.queue;
                        s.native_throws = gameplay.status.throws;
                        s.native_capture_status = gameplay.status.clone();
                        s.native_restock = restock.status.clone();
                        s.native_market = market.status.clone();
                    });
                    if last_ping.elapsed() >= HEARTBEAT_INTERVAL {
                        if socket.send(Message::Ping(Vec::new().into())).is_err() { break; }
                        last_ping = Instant::now();
                    }
                    match socket.read() {
                        Ok(Message::Text(text)) => {
                            if let Ok(frame)=serde_json::from_str::<Value>(text.as_ref()) {
                                let received_at = Instant::now();
                                gameplay.ingest_value(&frame, received_at);
                                let confirmed_before = restock.status.confirmed;
                                restock.ingest_value(&frame, received_at);
                                let market_cfg=market_rules.lock().map(|v|v.clone()).unwrap_or_default();
                                market.ingest_value(&frame, received_at, &market_cfg);
                                hunt_intel.ingest_value(&frame, received_at);
                                history_recorder.ingest_value(&frame,hunt_intel.view.hunt_slug.as_deref());
                                if let Some(auto)=frame.get("estado")
                                    .and_then(|state|state.get("automation")) {
                                    automation_writer.observe(auto);
                                }
                                if let Some((id,_))=pending_sale {
                                    // A full authoritative roster missing the ID
                                    // confirms the sale. Manager also recognizes
                                    // an explicit battle sale receipt; require
                                    // either matching pokemonId or an exact
                                    // matching Pokémon name to avoid treating
                                    // unrelated automatic loot sales as success.
                                    let full_missing=frame.get("estado")
                                        .and_then(|state|state.get("pokemons"))
                                        .and_then(Value::as_array)
                                        .is_some_and(|full| !full.iter().any(|p|
                                            p.get("id").and_then(Value::as_u64)==Some(id)));
                                    let name=hunt_intel.view.pokemon_roster.iter()
                                        .find(|p|p.id==id).map(|p|p.name.as_str());
                                    let sale_receipt=frame.get("ev").and_then(Value::as_array)
                                        .is_some_and(|events|events.iter().any(|event|
                                            event.get("k").and_then(Value::as_str)==Some("venda")
                                            && (event.get("pokemonId").and_then(Value::as_u64)==Some(id)
                                                || name.is_some_and(|name|
                                                    event.get("nome").and_then(Value::as_str)==Some(name)))));
                                    if full_missing || sale_receipt {
                                        hunt_intel.confirm_pokemon_sale(id);
                                        if let Ok(mut view)=hunts.lock() {*view=hunt_intel.snapshot();}
                                        pending_sale=None;
                                        update(&snapshot, |s| {
                                            s.pending_pokemon_sale=None;
                                            s.confirmed_pokemon_sales=s.confirmed_pokemon_sales.saturating_add(1);
                                            s.pokemon_sale_error=None;
                                        });
                                    }
                                }
                            }
                                if restock.status.confirmed > confirmed_before {
                                    crate::logging::info(&format!(
                                        "Native restock: confirmed and reconciled {}",
                                        restock.status.last_order.as_deref().unwrap_or("purchase")
                                    ));
                                }
                                observe_value(&snapshot, &frame, &desired);
                            }
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
                    history_recorder.maybe_flush(true);
                    restock.on_disconnect(Instant::now());
                    market.on_disconnect();
                    gameplay.on_disconnect();
                    restock.publish_status();
                    automation_writer.on_disconnect();
                    pending_sale=None;
                    hunt_intel.on_disconnect();
                    if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                    update(&snapshot, |s| {
                        s.phase = "Reconnecting".into();
                        s.native_restock = restock.status.clone();
                        s.native_automation = automation_writer.status.clone();
                        s.native_market = market.status.clone();
                        s.native_auto_catch = gameplay.status.auto_catch;
                        s.native_capture_queue = gameplay.status.queue;
                        s.native_capture_status = gameplay.status.clone();
                        s.pending_pokemon_sale=None;
                        s.pokemon_sale_error=Some("Connection lost before Pokémon sale confirmation".into());
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
    fn connection_setup_has_time_for_server_handshake() {
        // A 60 ms WebSocket handshake timeout regressed otherwise valid
        // Firefox-to-Rust handoffs. Live reads can remain at 60 ms.
        assert!(CONNECT_TIMEOUT >= Duration::from_secs(2));
        assert!(CONNECT_TIMEOUT > READ_TIMEOUT);
        assert!(WELCOME_TIMEOUT > READ_TIMEOUT);
    }
    #[test]
    fn records_battle_events_without_requiring_browser_dom() {
        let status = Arc::new(Mutex::new(NativeSnapshot::default()));
        let desired = Arc::new(Mutex::new(Vec::new()));
        observe(&status, r#"{"t":"batalha","ev":[
            {"k":"morte","quem":"selvagem"},
            {"k":"morte","quem":"jogador"},
            {"k":"bola","sucesso":true}
        ]}"#, &desired);
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
