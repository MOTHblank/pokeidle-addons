//! Authenticated Rust game transport with state-driven native gameplay commands.
//! Browser is only an authentication owner until the server welcomes Rust.
//!
//! Credentials are captured only from the existing, authenticated Firefox BiDi
//! session, held in memory, and never printed or persisted by this module.
use serde_json::Value;
use crate::native_gameplay::{GameCommand, Gameplay};
use crate::native_capture_config::CatchSettings;
use crate::native_gameplay::GameplayStatus;
use crate::native_hunts::{HuntIntel, HuntView};
use crate::native_hunt_navigation::HuntNavigator;
use crate::native_boss::{BossManager, BossView};
use crate::native_history::{HistoryRecorder, HistorySnapshot, HistoryForecast};
use crate::native_atlas::{self, Reference as AtlasReference};
use crate::config::GameProfile;
use crate::native_restock::{Restock, RestockStatus};
use crate::native_automation::{AutomationWriter, AutomationStatus};
use crate::native_pvp::{PvpAutomation, PvpAction, PvpStatus};
use crate::native_market::{MarketBuyer, MarketRules, MarketStatus};
use crate::native_twitch_miner::{TwitchMinerControl, TwitchMinerSession};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
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
// Historical models are optional. Reference data is loaded only after a user
// opens Hunt Atlas, and just one current-hunt simulation runs per interval.
const BACKGROUND_FORECAST_INTERVAL: Duration = Duration::from_secs(15 * 60);

// A stopped NativeSession has only set its cancellation flag; its socket and
// journal workers may still be closing. Keep their handles through window
// teardown so a final journal flush is not cut short by process exit.
static GAME_WORKERS: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>> = OnceLock::new();
fn track_game_worker(worker: thread::JoinHandle<()>) {
    let registry = GAME_WORKERS.get_or_init(|| Mutex::new(Vec::new()));
    let mut handles = registry.lock().unwrap_or_else(|e| e.into_inner());
    let mut index = 0;
    while index < handles.len() {
        if handles[index].is_finished() {
            let _ = handles.swap_remove(index).join();
        } else {
            index += 1;
        }
    }
    handles.push(worker);
}
pub fn finish_game_workers_after_window() {
    if let Some(registry) = GAME_WORKERS.get() {
        let handles = std::mem::take(
            &mut *registry.lock().unwrap_or_else(|e| e.into_inner()));
        for handle in handles {
            if handle.join().is_err() {
                crate::logging::warn("Native game worker panicked during shutdown");
            }
        }
    }
}

/// Minimal queue metrics for the opt-in Diagnostics page. Never clone the
/// larger market, roster, or gameplay snapshots just to read queue lengths.
#[derive(Clone, Debug)]
pub struct NativeDiagnosticsSnapshot {
    pub phase: String,
    pub capture_queued: usize,
    pub automation_queued: usize,
    pub automation_pending: bool,
    pub market_pending: bool,
    pub restock_pending: bool,
}

impl NativeDiagnosticsSnapshot {
    fn from_state(state: &NativeSnapshot) -> Self {
        Self {
            phase: state.phase.clone(),
            capture_queued: state.native_capture_queue,
            automation_queued: state.native_automation.queued,
            automation_pending: state.native_automation.pending_field.is_some(),
            market_pending: state.native_market.pending.is_some(),
            restock_pending: state.native_restock.pending,
        }
    }
}

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
    pub native_pvp: PvpStatus,
    pub native_market: MarketStatus,
    pub pending_pokemon_sale: Option<u64>,
    pub confirmed_pokemon_sales: u64,
    pub pokemon_sale_error: Option<String>,
    /// Twitch is game state only. No IRC/chat connection is owned by Moth.
    pub twitch_linked: Option<bool>,
    pub twitch_login: Option<String>,
    pub twitch_bonus_pct: f32,
    pub twitch_xp_until: u64,
    pub twitch_capture_until: u64,
    pub twitch_shiny_until: u64,
    /// Live official Kick channels from the authenticated PokéIdle game state.
    /// None = the server has not supplied the roster; Some(empty) = no one live.
    pub kick_official_live: Option<Vec<String>>,
    /// Full official roster including offline creators, as reported by the game.
    pub kick_official_roster: Option<Vec<crate::native_kick::OfficialKickChannel>>,
    pub kick_roster_revision: u64,
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
            native_pvp: PvpStatus::default(),
            native_market: MarketStatus::default(),
            pending_pokemon_sale: None, confirmed_pokemon_sales: 0,
            pokemon_sale_error: None,
            twitch_linked: None, twitch_login: None, twitch_bonus_pct: 0.0,
            twitch_xp_until: 0, twitch_capture_until: 0, twitch_shiny_until: 0,
            kick_official_live: None, kick_official_roster: None,
            kick_roster_revision: 0,
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
    Credentials::parse(bootstrap).map(|_| ())
}

type GameSocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

// Connection-state transitions notify the controller without a polling loop
// or a dependency on the egui display refresh cadence.
type PhaseWake = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;
fn notify_phase_change(wake: &PhaseWake) {
    let callback = wake.lock().ok().and_then(|slot| slot.clone());
    if let Some(callback) = callback { callback(); }
}

/// Cloneable control-plane handle: encrypted settings writes can update the
/// live game worker without waiting for an egui frame, even in the tray.
/// This contains no socket ownership and does not run gameplay on the UI.
#[derive(Clone)]
pub struct CaptureSettingsUpdater {
    config: Arc<Mutex<CatchSettings>>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
}
impl CaptureSettingsUpdater {
    pub fn apply(&self, settings: CatchSettings) -> Result<(), String> {
        settings.validate()?;
        let mut current = self.config.lock()
            .map_err(|_| "Capture preferences currently unavailable".to_owned())?;
        *current = settings.clone();
        self.auto_catch.store(settings.enabled, Ordering::Relaxed);
        self.restock_enabled.store(settings.restock_enabled, Ordering::Relaxed);
        Ok(())
    }
}

pub struct NativeSession {
    snapshot: Arc<Mutex<NativeSnapshot>>,
    created_at: Instant,
    profile: GameProfile,
    stop: Arc<AtomicBool>,
    command_tx: mpsc::SyncSender<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    pvp_preference: Arc<AtomicU8>,
    pvp_toggle_epoch: Arc<AtomicU64>,
    capture_config: Arc<Mutex<CatchSettings>>,
    market_rules: Arc<Mutex<Arc<MarketRules>>>,
    market_rules_revision: Arc<AtomicU64>,
    session_reset: Arc<AtomicBool>,
    hunts: Arc<Mutex<HuntView>>,
    boss: Arc<Mutex<BossView>>,
    history: Arc<Mutex<HistorySnapshot>>,
    forecast_tx:mpsc::SyncSender<HistoryForecast>,
    background_forecast_reference: Arc<Mutex<Option<(Arc<AtlasReference>, bool)>>>,
    phase_wake:PhaseWake,
    kick_roster_publisher:Arc<Mutex<Option<crate::native_kick::KickRosterPublisher>>>,
    twitch_miner:TwitchMinerSession,
}
impl NativeSession {
    /// Establish the verified game WebSocket before handing ownership away
    /// from Firefox. This is blocking and MUST run on a background thread.
    pub fn start_verified(profile: GameProfile, bootstrap: Value) -> Result<Self, String> {
        let credentials = Credentials::parse(&bootstrap)?;
        let initial = open_session(&credentials)?;
        let twitch_miner = TwitchMinerSession::start_optional(
            bootstrap.get("twitch").filter(|value| !value.is_null())
        );
        Self::spawn(profile, credentials, twitch_miner, Some(initial),
            bootstrap.get("autocatch").cloned())
    }
    fn spawn(profile: GameProfile, credentials: Credentials, twitch_miner:TwitchMinerSession,
        initial: Option<(GameSocket, Value)>, settings: Option<Value>) -> Result<Self, String> {
        let gameplay = Gameplay::new(settings.as_ref());
        let restock = Restock::new(settings.as_ref());
        let rules=MarketRules::load(profile).unwrap_or_else(|error| {
            crate::logging::warn(&format!("Market rule settings unavailable; disabled: {error}"));
            MarketRules::default()
        });
        // Worker and editor share immutable rule data; a 60 ms dispatcher
        // must not retain a second full allocation of the same rule set.
        let market_rules=Arc::new(Mutex::new(Arc::new(rules)));
        let worker_market_rules=Arc::clone(&market_rules);
        let market_rules_revision=Arc::new(AtomicU64::new(0));
        let worker_rules_revision=Arc::clone(&market_rules_revision);
        let capture_config=Arc::new(Mutex::new(CatchSettings::from_value(settings.as_ref())));
        let worker_capture_config=Arc::clone(&capture_config);
        let session_reset=Arc::new(AtomicBool::new(false));
        let worker_session_reset=Arc::clone(&session_reset);
        let hunt_intel = HuntIntel::new();
        let nick = serde_json::from_str::<Value>(&credentials.hello).ok()
            .and_then(|hello| hello.get("nick").and_then(Value::as_str).map(str::to_owned))
            .filter(|name| !name.is_empty()).unwrap_or_else(||profile.name().to_owned());
        let history_recorder = HistoryRecorder::new(profile, nick.clone());
        let history = Arc::clone(&history_recorder.shared);
        let boss_manager = BossManager::new(profile, &nick);
        let boss = Arc::new(Mutex::new(boss_manager.view.clone()));
        let worker_boss = Arc::clone(&boss);
        let hunts = Arc::new(Mutex::new(hunt_intel.view.clone()));
        let worker_hunts = Arc::clone(&hunts);
        let restock_enabled = Arc::new(AtomicBool::new(restock.status.enabled));
        let worker_restock_enabled = Arc::clone(&restock_enabled);
        let auto_catch = Arc::new(AtomicBool::new(gameplay.status.auto_catch));
        let worker_auto_catch = Arc::clone(&auto_catch);
        // Inherit the VIP setting by default; explicit on/off persists per account.
        let saved_pvp = crate::config::Config::load_accounts().ok()
            .and_then(|accounts| accounts[profile.index()].auto_pvp);
        let pvp_preference = Arc::new(AtomicU8::new(match saved_pvp {
            None => 0, Some(false) => 1, Some(true) => 2,
        }));
        let worker_pvp_preference = Arc::clone(&pvp_preference);
        let pvp_toggle_epoch = Arc::new(AtomicU64::new(0));
        let worker_pvp_toggle_epoch = Arc::clone(&pvp_toggle_epoch);
        let (command_tx, command_rx) = mpsc::sync_channel(64);
        let (forecast_tx,forecast_rx)=mpsc::sync_channel(8);
        let background_forecast_reference=Arc::new(Mutex::new(None));
        let worker_forecast_reference=Arc::clone(&background_forecast_reference);
        let background_forecast_tx=forecast_tx.clone();
        let snapshot = Arc::new(Mutex::new(NativeSnapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_stop = Arc::clone(&stop);
        let phase_wake:PhaseWake=Arc::new(Mutex::new(None));
        let worker_phase_wake=Arc::clone(&phase_wake);
        let kick_roster_publisher=Arc::new(Mutex::new(None));
        let worker_kick_roster_publisher=Arc::clone(&kick_roster_publisher);
        let twitch_miner_control=twitch_miner.control();
        let worker = thread::Builder::new().name("moth-native-game".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session_loop(profile, credentials, Arc::clone(&worker_snapshot),
                        Arc::clone(&worker_stop), twitch_miner_control, initial,
                        gameplay, restock, command_rx, worker_auto_catch,
                        worker_restock_enabled, worker_pvp_preference,
                        worker_pvp_toggle_epoch, worker_capture_config,
                        worker_market_rules, worker_rules_revision, worker_session_reset,
                        Arc::clone(&worker_phase_wake), worker_kick_roster_publisher,
                        hunt_intel, worker_hunts, boss_manager, worker_boss,
                        history_recorder,forecast_rx,worker_forecast_reference,
                        background_forecast_tx);
                }));
                if result.is_err() {
                    update(&worker_snapshot, |state| {
                        state.phase = "Failed".into();
                        state.error = Some("PokéIdle connection stopped unexpectedly. See the controller log.".into());
                    });
                    notify_phase_change(&worker_phase_wake);
                    worker_stop.store(true, Ordering::Relaxed);
                }
            })
            .map_err(|error| format!("Could not start the PokéIdle connection: {error}"))?;
        track_game_worker(worker);
        Ok(Self {
            snapshot, created_at: Instant::now(), profile, stop, command_tx, auto_catch, restock_enabled,
            pvp_preference, pvp_toggle_epoch, capture_config, market_rules, market_rules_revision, session_reset, hunts, boss, history, forecast_tx,
            background_forecast_reference, phase_wake, kick_roster_publisher, twitch_miner
        })
    }
    /// Changes take effect on the socket worker even when egui is minimized.
    pub fn set_auto_pvp(&self, preference: Option<bool>) {
        self.pvp_preference.store(match preference {
            None => 0, Some(false) => 1, Some(true) => 2,
        }, Ordering::Release);
        self.pvp_toggle_epoch.fetch_add(1, Ordering::AcqRel);
    }
    pub fn set_market_rules(&self, rules: MarketRules) -> Result<(), String> {
        rules.save(self.profile)?;
        let mut live=self.market_rules.lock().map_err(|_|"Market settings unavailable".to_owned())?;
        *live=Arc::new(rules);
        self.market_rules_revision.fetch_add(1, Ordering::Release);
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
            if view.unlocked_outland_tier(*tier).is_none(){
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
    pub fn boss_snapshot(&self) -> BossView {
        self.boss.lock().map(|view| view.clone()).unwrap_or_default()
    }
    pub fn history_snapshot(&self) -> HistorySnapshot {
        self.history.lock().map(|data|data.clone()).unwrap_or_default()
    }
    /// Reuse the Atlas reference only after the user has opened the utility.
    /// Never fetch new assets, start timers or simulate every unlocked hunt
    /// for accounts that only use the main Moth Controller features.
    pub fn enable_background_forecasts(&self, reference:Arc<AtlasReference>, aoe:bool) {
        if let Ok(mut current)=self.background_forecast_reference.lock() {
            *current=Some((reference,aoe));
        }
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
    pub fn capture_settings_updater(&self) -> CaptureSettingsUpdater {
        CaptureSettingsUpdater {
            config: Arc::clone(&self.capture_config),
            auto_catch: Arc::clone(&self.auto_catch),
            restock_enabled: Arc::clone(&self.restock_enabled),
        }
    }
    pub fn snapshot(&self) -> NativeSnapshot {
        self.snapshot.lock().map(|status| status.clone()).unwrap_or_default()
    }

    /// On-demand and nonblocking: if the game worker owns its snapshot,
    /// Diagnostics skips this tick rather than stalling automation or the UI.
    pub fn diagnostics_snapshot(&self) -> Option<NativeDiagnosticsSnapshot> {
        let state = self.snapshot.try_lock().ok()?;
        Some(NativeDiagnosticsSnapshot::from_state(&state))
    }
    /// Allocation-free roster change token. A new session timestamp prevents
    /// old revision numbers from hiding a freshly authenticated game session.
    pub fn official_kick_roster_stamp(&self) -> Option<(Instant,u64)> {
        let state=self.snapshot.lock().ok()?;
        if state.phase!="Online" ||
            state.last_received.is_none_or(|at|at.elapsed()>Duration::from_secs(90)) ||
            state.kick_official_roster.is_none() {return None}
        Some((self.created_at,state.kick_roster_revision))
    }
    pub fn official_kick_roster(&self) -> Option<Vec<crate::native_kick::OfficialKickChannel>> {
        let state = self.snapshot.lock().ok()?;
        if state.phase != "Online" ||
            state.last_received.is_none_or(|at| at.elapsed() > Duration::from_secs(90)) {
            return None;
        }
        state.kick_official_roster.clone()
    }
    /// Register a changed-roster publisher without adding reads or polling.
    /// Send the current roster once so a Kick monitor started after PokéIdle
    /// login immediately catches up with the already-observed server state.
    pub fn set_kick_roster_publisher(&self,publisher:crate::native_kick::KickRosterPublisher) {
        if let Ok(mut current)=self.kick_roster_publisher.lock() {
            *current=Some(publisher.clone());
        }
        // A verified native handoff may install the publisher between
        // observing welcome and publishing phase=Online. The just-received
        // roster is already authoritative even in that narrow transition.
        let ready=self.snapshot.lock().ok().and_then(|state| {
            let recent=state.last_received.is_some_and(|at|
                at.elapsed()<Duration::from_secs(90));
            (recent && matches!(state.phase.as_str(),"Online"|"Connecting"))
                .then(||state.kick_official_roster.clone()).flatten()
        });
        if let Some(roster)=ready {publisher.publish(roster);}
    }
    pub fn clear_kick_roster_publisher(&self) {
        if let Ok(mut current)=self.kick_roster_publisher.lock() {
            *current=None;
        }
    }
    /// Called after the authenticated session is adopted into its account.
    /// Game state changes can request immediate recovery.
    pub fn set_phase_waker(&self, wake:Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot)=self.phase_wake.lock() {
            *slot=Some(Arc::clone(&wake));
        }
        self.twitch_miner.set_phase_waker(Arc::clone(&wake));
        // A worker can expire while its initial handoff is still in flight.
        if self.snapshot().phase=="NeedsLogin" { wake(); }
    }
    pub fn twitch_miner_snapshot(&self) -> Option<moth_twitch::MinerSnapshot> {
        self.twitch_miner.snapshot()
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.twitch_miner.stop();
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

fn open_session(credentials: &Credentials) -> Result<(GameSocket, Value), String> {
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
                if let Ok(frame) = serde_json::from_str::<Value>(text.as_ref()) {
                    if frame.get("t").and_then(Value::as_str) == Some("welcome") {
                        // Preserve the parsed welcome across the verified handoff.
                        // The game worker must not parse this large frame twice.
                        set_live_read_timeout(&mut socket)?;
                        return Ok((socket, frame));
                    }
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

#[cfg(test)]
fn observe(snapshot: &Arc<Mutex<NativeSnapshot>>, text: &str) {
    let Ok(payload) = serde_json::from_str::<Value>(text) else { return };
    observe_value(snapshot, &payload);
}
fn observe_value(snapshot: &Arc<Mutex<NativeSnapshot>>, payload: &Value) -> bool {
    let mut changed_kick_roster=false;
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
            // Keep PokéIdle's own Kick official roster authoritative. The
            // optional value distinguishes unavailable data from an empty list.
            if let Some(roster) = crate::native_kick::pokeidle_official_roster(state) {
                if status.kick_official_roster.as_ref()!=Some(&roster) {
                    status.kick_official_live=Some(roster.iter().filter(|entry|entry.live)
                        .map(|entry|entry.slug.clone()).collect());
                    // The observer only returns a flag. Publishing clones
                    // the roster only if a Kick account is actually running.
                    changed_kick_roster=true;
                    status.kick_official_roster=Some(roster);
                    status.kick_roster_revision=status.kick_roster_revision.wrapping_add(1);
                }
            }
            if let Some(twitch) = state.get("twitch") {
                status.twitch_linked=twitch.get("vinculado").and_then(Value::as_bool)
                    .or_else(||Some(twitch.get("login").and_then(Value::as_str).is_some()));
                if let Some(login)=twitch.get("login").and_then(Value::as_str) {
                    status.twitch_login=Some(login.to_owned());
                }
                if let Some(pct)=twitch.get("pct").and_then(Value::as_f64) {
                    status.twitch_bonus_pct=if pct.is_finite(){pct.max(0.0) as f32}else{0.0};
                }
                if let Some(bonus)=twitch.get("bonus") {
                    if let Some(raw)=bonus.get("xp") {
                        status.twitch_xp_until=raw.as_u64().unwrap_or(0);
                    }
                    if let Some(raw)=bonus.get("captura") {
                        status.twitch_capture_until=raw.as_u64().unwrap_or(0);
                    }
                    if let Some(raw)=bonus.get("shiny") {
                        status.twitch_shiny_until=raw.as_u64().unwrap_or(0);
                    }
                }
            }
        }
    });
    changed_kick_roster
}

fn sync_twitch_miner_targets(control:&TwitchMinerControl,payload:&Value) {
    if let Some(channels)=crate::native_twitch_miner::official_live_channels(payload) {
        control.set_channels(channels);
    }
}

fn publish_kick_roster(
    publisher:&Arc<Mutex<Option<crate::native_kick::KickRosterPublisher>>>,
    snapshot:&Arc<Mutex<NativeSnapshot>>,
    changed:bool
) {
    if !changed {return}
    // No Kick login means zero roster clones, channel messages or additional
    // locks beyond this rare changed-roster notification.
    let sender=publisher.lock().ok().and_then(|current|current.clone());
    let Some(sender)=sender else{return};
    if let Some(roster)=snapshot.lock().ok()
        .and_then(|current|current.kick_official_roster.clone()) {
        sender.publish(roster);
    }
}

// A forecast is only meaningful while the server confirms an ordinary hunt,
// a complete catalog and a selected Pokémon. Never model a Center or a
// temporary/no-lead screen as a zero-throughput combat session.
fn current_background_hunt(view:&HuntView)->Option<crate::native_hunts::HuntEntry> {
    if !view.catalog_ready || view.in_center || view.active_pokemon.is_none() {
        return None;
    }
    let slug=view.hunt_slug.as_deref()?;
    view.hunts.iter().find(|hunt|hunt.slug==slug).cloned()
}

fn session_loop(profile: GameProfile, credentials: Credentials,
    snapshot: Arc<Mutex<NativeSnapshot>>, stop: Arc<AtomicBool>,
    twitch_miner:TwitchMinerControl, first_session: Option<(GameSocket, Value)>,
    mut gameplay: Gameplay, mut restock: Restock,
    command_rx: mpsc::Receiver<GameCommand>,
    auto_catch: Arc<AtomicBool>,
    restock_enabled: Arc<AtomicBool>,
    pvp_preference: Arc<AtomicU8>,
    pvp_toggle_epoch: Arc<AtomicU64>,
    capture_config: Arc<Mutex<CatchSettings>>,
    market_rules: Arc<Mutex<Arc<MarketRules>>>,
    market_rules_revision: Arc<AtomicU64>,
    session_reset: Arc<AtomicBool>,
    phase_wake:PhaseWake,
    kick_roster_publisher:Arc<Mutex<Option<crate::native_kick::KickRosterPublisher>>>,
    mut hunt_intel: HuntIntel,
    hunts: Arc<Mutex<HuntView>>,
    mut boss_manager: BossManager,
    boss: Arc<Mutex<BossView>>,
    mut history_recorder: HistoryRecorder,
    forecast_rx:mpsc::Receiver<HistoryForecast>,
    background_forecast_reference:Arc<Mutex<Option<(Arc<AtlasReference>,bool)>>>,
    background_forecast_tx:mpsc::SyncSender<HistoryForecast>) {
    let mut rejected_welcomes = 0usize;
    let mut first_session = first_session;
    let mut automation_writer = AutomationWriter::new();
    let mut pvp = PvpAutomation::default();
    let mut seen_pvp_toggle_epoch = 0u64;
    let mut market = MarketBuyer::new(profile);
    let mut pending_sale: Option<(u64, Instant)> = None;
    let mut next_background_forecast=Instant::now()+BACKGROUND_FORECAST_INTERVAL;
    // Metadata is downloaded without cookies/credentials, and only if a
    // configured healing-item rule lacks a server shop price. Never block the
    // game WebSocket reader with HTTP requests.
    let (metadata_tx, metadata_rx) = mpsc::channel::<Result<crate::native_items::ItemCatalog, String>>();
    let mut item_metadata: Option<crate::native_items::ItemCatalog> = None;
    let mut last_metadata_attempt: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        match first_session.take().map(Ok).unwrap_or_else(|| open_session(&credentials)) {
            Ok((mut socket, frame)) => {
                crate::logging::info("Native PokéIdle WSS received authenticated welcome");
                // The handshake already parsed this frame; all consumers borrow it.
                let received_at = Instant::now();
                let roster_changed=observe_value(&snapshot, &frame);
                sync_twitch_miner_targets(&twitch_miner,&frame);
                publish_kick_roster(&kick_roster_publisher,&snapshot,roster_changed);
                gameplay.ingest_value(&frame, received_at);
                restock.ingest_value(&frame, received_at);
                let mut market_cfg=market_rules.lock()
                    .map(|v|Arc::clone(&v))
                    .unwrap_or_else(|_|Arc::new(MarketRules::default()));
                let mut last_rules_revision=market_rules_revision.load(Ordering::Acquire);
                market.ingest_value(&frame, received_at, &market_cfg);
                if let Some(catalog) = item_metadata.as_ref() {
                    restock.merge_item_metadata(catalog);
                }
                hunt_intel.ingest_value(&frame, received_at);
                pvp.on_frame(&frame, received_at);
                boss_manager.ingest(&frame);
                let mut navigator = HuntNavigator::default();
                navigator.observe(&frame, &hunt_intel.view, received_at);
                boss_manager.recover_on_welcome(&hunt_intel.view);
                boss_manager.publish(&boss);
                history_recorder.ingest_value(&frame,hunt_intel.view.hunt_slug.as_deref());
                if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                let mut last_hunt_publish = Instant::now();
                let mut last_pvp_tick = Instant::now() - Duration::from_secs(1);
                let mut last_status_publish = Instant::now();
                let mut last_snatch_status_publish = Instant::now() - Duration::from_secs(5);
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
                notify_phase_change(&phase_wake);
                // A reconnect can confirm exactly the previous roster.
                // Replay it once after welcome so a new Kick subscriber does
                // not depend on the next live/offline status change.
                let subscriber=kick_roster_publisher.lock().ok()
                    .and_then(|publisher|publisher.clone());
                if let Some(subscriber)=subscriber {
                    if let Some(roster)=snapshot.lock().ok()
                        .and_then(|state|state.kick_official_roster.clone()) {
                        subscriber.publish(roster);
                    }
                }
                let mut last_ping = Instant::now();
                // Delayed capture receipts are diagnostic, not connection failures.
                // Summarize them at most once every five minutes per game socket.
                let mut delayed_capture_receipts = 0usize;
                let mut last_capture_delay_summary = Instant::now();
                loop {
                    if stop.load(Ordering::Relaxed) {
                        let _ = socket.close(None);
                        break;
                    }
                    if snapshot.lock().ok().and_then(|s| s.last_received)
                        .is_some_and(|last| last.elapsed() > CONNECTION_STALE_AFTER) {
                        crate::logging::warn(&format!("{}: reconnecting game socket after 90s without server frames or pong",profile.label()));
                        break;
                    }
                    // Every mutating action has one owner: this game socket.
                    // Never enqueue commands against an older/disconnected
                    // browser or replay unconfirmed writes after reconnect.
                    let mut send_failed = false;
                    while let Ok(command) = command_rx.try_recv() {
                        if let GameCommand::BossStart {key,repeat} = &command {
                            match boss_manager.start(key, *repeat,
                                &hunt_intel.view, &mut navigator, Instant::now()) {
                                Ok(()) => {
                                    if hunt_intel.view.in_center { hunt_intel.cancel_hunt_intent(); }
                                    else { hunt_intel.queue_center(); }
                                }
                                Err(error) => {
                                    boss_manager.view.warning = Some(error.clone());
                                    crate::logging::warn(&format!("Boss manager: {error}"));
                                }
                            }
                            boss_manager.publish(&boss);
                            continue;
                        }
                        if matches!(command, GameCommand::BossStop | GameCommand::BossLeave) {
                            let actions = if matches!(command, GameCommand::BossLeave) {
                                boss_manager.leave(&mut navigator)
                            } else {
                                boss_manager.stop(&mut navigator, &hunt_intel.view,
                                    Instant::now()).into_iter().collect()
                            };
                            for action in actions {
                                if socket.send(Message::Text(action.wire().to_string().into())).is_err() {
                                    send_failed = true;
                                    break;
                                }
                            }
                            if !boss_manager.active() { hunt_intel.cancel_hunt_intent(); }
                            boss_manager.publish(&boss);
                            if send_failed { break; }
                            continue;
                        }
                        if boss_manager.active() && matches!(command,
                            GameCommand::Hunt(_) | GameCommand::Center | GameCommand::GiveUp) {
                            boss_manager.view.warning = Some(
                                "Finish or cancel the boss trip before choosing another destination".into());
                            boss_manager.publish(&boss);
                            continue;
                        }
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
                        if let GameCommand::Hunt(slug) = &command {
                            // Store the intention. The navigator sends on the first
                            // server-confirmed open gate and retries during that opening.
                            if !hunt_intel.view.in_center &&
                                hunt_intel.view.hunt_slug.as_deref() == Some(slug.as_str()) {
                                navigator.cancel();
                                hunt_intel.cancel_hunt_intent();
                            } else if let Err(error) = hunt_intel.queue_hunt(slug) {
                                crate::logging::warn(&format!("Native hunt selection rejected: {error}"));
                            } else {
                                navigator.select(slug.clone(), Instant::now());
                            }
                            continue;
                        }
                        if matches!(command, GameCommand::Center) {
                            // Normal Center uses the same timed gate as a hunt
                            // change, without ever forcing a surrender.
                            if hunt_intel.view.in_center {
                                navigator.cancel();
                                hunt_intel.cancel_hunt_intent();
                            } else {
                                hunt_intel.queue_center();
                                navigator.center(Instant::now());
                            }
                            continue;
                        }
                        if matches!(command, GameCommand::GiveUp) {
                            // Explicit, player-confirmed action only. Never
                            // resume a queued destination after surrender.
                            navigator.cancel();
                            hunt_intel.cancel_hunt_intent();
                            if hunt_intel.view.in_center { continue; }
                        }
                        let payload = command.wire().to_string();
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
                    let boss_phase = boss_manager.view.phase.clone();
                    if let Some(command) = boss_manager.next_action(
                        &hunt_intel.view, &mut navigator, Instant::now()) {
                        if socket.send(Message::Text(command.wire().to_string().into())).is_err() {
                            boss_manager.view.warning = Some("Boss command failed to send".into());
                            boss_manager.publish(&boss);
                            break;
                        }
                    }
                    if boss_manager.view.phase != boss_phase {
                        boss_manager.publish(&boss);
                    }
                    // 60 ms worker wake, rather than a one-shot select followed
                    // by a 20-second timeout, catches narrow 3-second openings.
                    if let Some(command) = navigator.next_action(&hunt_intel.view, Instant::now()) {
                        let mut allowed = true;
                        if let GameCommand::Hunt(slug) = &command {
                            if let Err(error) = hunt_intel.request_hunt(slug, Instant::now()) {
                                crate::logging::warn(&format!("Native hunt selection rejected: {error}"));
                                navigator.cancel();
                                hunt_intel.cancel_hunt_intent();
                                allowed = false;
                            }
                        }
                        if allowed && socket.send(Message::Text(command.wire().to_string().into())).is_err() {
                            if matches!(command, GameCommand::Hunt(_)) {
                                hunt_intel.command_failed("Hunt selection transport failed");
                            }
                            break;
                        }
                    }
                    // Give Up is NEVER issued by navigation; prolonged combat
                    // only produces a visible warning while safe retries continue.
                    hunt_intel.view.pending_center = navigator.center_pending();
                    hunt_intel.view.navigation_warning =
                        navigator.blocked(&hunt_intel.view, Instant::now())
                        .then(|| crate::native_hunt_navigation::BLOCKED_NOTICE.to_owned());
                    if pending_sale.as_ref().is_some_and(|(_,at)|
                        at.elapsed()>Duration::from_secs(20)) {
                        pending_sale=None;
                        update(&snapshot, |s| {
                            s.pending_pokemon_sale=None;
                            s.pokemon_sale_error=Some("Pokémon sale not confirmed within 20 seconds".into());
                        });
                    }
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
                    // Match the game's built-in Potion and Revive preferences
                    // to the corresponding selected restock items, when enabled.
                    // Neither the game automation toggles nor HP threshold change.
                    automation_writer.sync_restock_potion(
                        (last_config.restock_enabled && last_config.potion_id>0)
                            .then_some(last_config.potion_id),
                        hunt_intel.view.automation.as_ref(),
                    );
                    automation_writer.sync_restock_revive(
                        (last_config.restock_enabled && last_config.revive_id>0)
                            .then_some(last_config.revive_id),
                        hunt_intel.view.automation.as_ref(),
                    );
                    // Only queue management; neither matchmaking path touches hunts.
                    if last_pvp_tick.elapsed() >= Duration::from_secs(1) {
                        let toggle_epoch = pvp_toggle_epoch.load(Ordering::Acquire);
                        if toggle_epoch != seen_pvp_toggle_epoch {
                            seen_pvp_toggle_epoch = toggle_epoch;
                            pvp.on_user_toggle();
                        }
                        let pref = match pvp_preference.load(Ordering::Acquire) {
                            1 => Some(false), 2 => Some(true), _ => None,
                        };
                        let vip = hunt_intel.view.bonuses.vip_active;
                        let vip_auto = hunt_intel.view.automation.as_ref()
                            .and_then(|a| a.get("pvpAutoFila")).and_then(Value::as_bool);
                        // Do not treat an unconfirmed VIP status as non-VIP:
                        // matchmaking must wait until the server reports it.
                        if hunt_intel.view.bonuses.vip_known {
                            if let Some(action) = pvp.next(pref, vip, vip_auto,
                                hunt_intel.view.trainer_level, Instant::now()) {
                                match action {
                                    PvpAction::SetVipAuto(wanted) => {
                                        if let Err(error) = automation_writer.queue(
                                            "pvpAutoFila".into(), Value::Bool(wanted)) {
                                            crate::logging::warn(&format!(
                                                "Could not update VIP Auto PvP preference: {error}"));
                                        }
                                    }
                                    other => if let Some(wire) = other.wire() {
                                        if socket.send(Message::Text(wire.to_string().into())).is_err() {
                                            pvp.send_failed(other);
                                            send_failed = true;
                                        } else if matches!(other, PvpAction::Join | PvpAction::Leave) {
                                            let activity = if other == PvpAction::Join {
                                                "requested matchmaking"
                                            } else { "requested queue cancellation" };
                                            crate::logging::info(&format!(
                                                "{}: Auto PvP {activity}", profile.label()));
                                        }
                                    },
                                }
                            }
                        }
                        last_pvp_tick = Instant::now();
                    }
                    if send_failed { break; }
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
                    // Read market rules only when the editor publishes a new revision.
                    // The 60 ms Snatch loop never needs to clone unchanged rules.
                    let revision=market_rules_revision.load(Ordering::Acquire);
                    if revision != last_rules_revision {
                        if let Ok(rules)=market_rules.lock() {
                            market_cfg=Arc::clone(&rules);
                            last_rules_revision=revision;
                        }
                    }
                    // Rule buys share this authenticated game socket.
                    // Snatch owns the four-second release lottery. Send up to
                    // eight authorized maximum-quantity attempts per wake,
                    // prioritizing each queued listing's first attempt. Exact
                    // market.anuncio verifications never wait behind ordinary
                    // rule-scanning, and share the same authenticated socket.
                    let snatch_started=Instant::now();
                    let snatch_payloads=market.snatch_actions(&market_cfg,Instant::now());
                    let mut snatch_send_failed=false;
                    let mut snatch_buys_sent=false;
                    for payload in snatch_payloads {
                        let buying=payload.get("t").and_then(Value::as_str)==Some("market.comprar");
                        if socket.send(Message::Text(payload.to_string().into())).is_err() {
                            market.send_failed();
                            snatch_send_failed=true;
                            break;
                        }
                        snatch_buys_sent|=buying;
                    }
                    if snatch_buys_sent {
                        // Real pre-dispatch batch cost, including the single
                        // durable journal commit and all actual socket sends.
                        market.record_dispatch_latency(true,snatch_started.elapsed());
                    }
                    if snatch_send_failed {break;}
                    let rule_started=Instant::now();
                    if let Some(payload)=market.next(&market_cfg,Instant::now()) {
                        let buying=payload.get("t").and_then(Value::as_str)==Some("market.comprar");
                        let failed=socket.send(Message::Text(payload.to_string().into())).is_err();
                        if buying {market.record_dispatch_latency(false,rule_started.elapsed());}
                        if failed {
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
                    // Boss combat deliberately does not gate Restock. Bosses consume
                    // the same potion/revive inventory as hunts, while the server's
                    // Auto-Potion/Auto-Revive settings stay synchronized above.
                    if let Some(command) = restock.next(Instant::now()) {
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
                    // Missing post-purchase inventory must pause *restock*,
                    // not disconnect this account's otherwise healthy socket.
                    // Restock keeps the unresolved order guarded until a fresh
                    // authoritative stock snapshot arrives; never buy twice
                    // based on a guessed inventory count.
                    // A delayed Poké Ball receipt is NOT proof the authenticated
                    // game socket is unhealthy. Keep receiving hunts, account
                    // balances and other automation events. Gameplay holds the
                    // pending slot and ball reservation until server reconciliation
                    // or corpse expiration, so the throw cannot be blindly replayed.
                    let capture_now=Instant::now();
                    if gameplay.has_unreported_capture_timeout(capture_now) {
                        delayed_capture_receipts=delayed_capture_receipts.saturating_add(
                            gameplay.report_capture_timeouts(capture_now)
                        );
                    }
                    if capture_now.duration_since(last_capture_delay_summary)
                        >= Duration::from_secs(300) {
                        if delayed_capture_receipts > 0 {
                            crate::logging::info(&format!(
                                "{}: {} delayed native capture receipt(s) in the last five minutes; game connection unaffected",
                                profile.label(), delayed_capture_receipts
                            ));
                        }
                        delayed_capture_receipts=0;
                        last_capture_delay_summary=capture_now;
                    }
                    while let Ok(sample)=forecast_rx.try_recv(){
                        history_recorder.record_forecast(sample);
                    }
                    history_recorder.maybe_flush(false);

                    if last_hunt_publish.elapsed() >= Duration::from_secs(1) {
                        if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                        last_hunt_publish = Instant::now();
                        // Run the optional history sampler within the
                        // already existing one-second hunt refresh, not the
                        // 60 ms purchase/capture dispatch hot path.
                        // The market's release-time buying window always
                        // takes priority over optional Atlas modeling.
                        if Instant::now() >= next_background_forecast
                            && !market.snatch_critical(&market_cfg) {
                            next_background_forecast=Instant::now()+BACKGROUND_FORECAST_INTERVAL;
                            let reference=background_forecast_reference.lock().ok()
                                .and_then(|current|current.clone());
                            if let Some((reference,aoe))=reference {
                                let view=&hunt_intel.view;
                                if let Some(hunt)=current_background_hunt(view) {
                                    let view=view.clone();
                                    let tx=background_forecast_tx.clone();
                                    let _=thread::Builder::new()
                                        .name("moth-current-hunt-forecast".into())
                                        .spawn(move||{
                                            let Some(model)=native_atlas::estimate_with_aoe(
                                                &view,&hunt,&reference,aoe) else {return};
                                            let Some(record)=HistoryForecast::now(
                                                &hunt.slug,model.trainer_xp_h,
                                                model.pokemon_xp_h,model.kills_h,
                                                model.calibrated) else {return};
                                            // Drop a full queue rather than
                                            // blocking gameplay or market buys.
                                            let _=tx.try_send(record);
                                        });
                                }
                            }
                        }
                    }
                    // These snapshots are for egui, not for purchasing or capturing.
                    // Do not rebuild rule rows and clone market state 16 times/sec.
                    if last_status_publish.elapsed() >= Duration::from_secs(1) {
                        // Snatch display rows are nonessential telemetry. Never
                        // build them inside the 60 ms purchase dispatcher or
                        // during an active release-time buying burst.
                        if last_snatch_status_publish.elapsed() >= Duration::from_secs(5)
                            && !market.snatch_critical(&market_cfg) {
                            market.publish_snatch_status(&market_cfg,Instant::now());
                            last_snatch_status_publish=Instant::now();
                        }
                        restock.publish_status();
                        // Aggregate latency statistics are emitted only after
                        // the critical Snatch window, once every five minutes.
                        market.maybe_report_latency(&market_cfg);
                        update(&snapshot, |s| {
                            s.native_automation = automation_writer.status.clone();
                            let pref = match pvp_preference.load(Ordering::Acquire) {
                                1 => Some(false), 2 => Some(true), _ => None,
                            };
                            let vip_auto = hunt_intel.view.automation.as_ref()
                                .and_then(|a| a.get("pvpAutoFila")).and_then(Value::as_bool);
                            s.native_pvp = pvp.status(pref, hunt_intel.view.bonuses.vip_active,
                                vip_auto, hunt_intel.view.trainer_level, Instant::now());
                            s.native_auto_catch = gameplay.status.auto_catch;
                            s.native_capture_queue = gameplay.status.queue;
                            s.native_throws = gameplay.status.throws;
                            s.native_capture_status = gameplay.status.clone();
                            s.native_restock = restock.status.clone();
                            s.native_market = market.status.clone();
                        });
                        last_status_publish = Instant::now();
                    }
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
                                market.ingest_value(&frame, received_at, &market_cfg);
                                // Submit a qualified freshly discovered bargain immediately.
                                // Waiting for the following 60 ms read timeout (plus
                                // hunt/history work) loses release-time races.
                                if frame.get("t").and_then(Value::as_str)==Some("market") {
                                    if let Some(buy)=market.next_ready_buy(&market_cfg,received_at) {
                                        let failed=socket.send(Message::Text(buy.to_string().into())).is_err();
                                        // Includes market frame processing,
                                        // journal persistence and socket send.
                                        market.record_dispatch_latency(false,received_at.elapsed());
                                        if failed {
                                            market.send_failed();
                                            break;
                                        }
                                    }
                                }
                                hunt_intel.ingest_value(&frame, received_at);
                                pvp.on_frame(&frame, received_at);
                                boss_manager.ingest(&frame);
                                if frame.get("t").and_then(Value::as_str) == Some("welcome") ||
                                    frame.get("estado").and_then(|s| s.get("boss")).is_some() ||
                                    frame.get("ev").and_then(Value::as_array).is_some_and(|ev|
                                        ev.iter().any(|e| e.get("k").and_then(Value::as_str)
                                            .is_some_and(|k| k.starts_with("boss")))) {
                                    boss_manager.publish(&boss);
                                }
                                navigator.observe(&frame, &hunt_intel.view, received_at);
                                hunt_intel.view.pending_center = navigator.center_pending();
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
                                if restock.status.confirmed > confirmed_before {
                                    crate::logging::info(&format!(
                                        "Native restock: confirmed and reconciled {}",
                                        restock.status.last_order.as_deref().unwrap_or("purchase")
                                    ));
                                }
                                let roster_changed=observe_value(&snapshot, &frame);
                sync_twitch_miner_targets(&twitch_miner,&frame);
                                publish_kick_roster(&kick_roster_publisher,&snapshot,roster_changed);
                            }
                        },
                        Ok(Message::Pong(_)) | Ok(Message::Ping(_)) => {
                            update(&snapshot, |s| s.last_received = Some(Instant::now()));
                        }
                        Ok(Message::Close(frame)) => {
                            crate::logging::warn(&format!("{}: game server closed WebSocket: {:?}",profile.label(),frame.as_ref().map(|f|f.code)));
                            break;
                        },
                        Err(tungstenite::Error::Io(ref error))
                            if matches!(error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                        Err(error) => {
                            crate::logging::warn(&format!("{}: game WebSocket read failed: {}",profile.label(),error));
                            break;
                        },
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
                    pvp.on_reconnect();
                    pending_sale=None;
                    hunt_intel.on_disconnect();
                    boss_manager.on_disconnect(&navigator, &hunt_intel.view);
                    boss_manager.publish(&boss);
                    if let Ok(mut view) = hunts.lock() { *view = hunt_intel.snapshot(); }
                    update(&snapshot, |s| {
                        s.phase = "Reconnecting".into();
                        s.native_restock = restock.status.clone();
                        s.native_automation = automation_writer.status.clone();
                        s.native_pvp = PvpStatus::default();
                        s.native_market = market.status.clone();
                        s.native_auto_catch = gameplay.status.auto_catch;
                        s.native_capture_queue = gameplay.status.queue;
                        s.native_capture_status = gameplay.status.clone();
                        s.pending_pokemon_sale=None;
                        s.pokemon_sale_error=Some("Connection lost before Pokémon sale confirmation".into());
                    });
                    notify_phase_change(&phase_wake);
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
                notify_phase_change(&phase_wake);
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
        notify_phase_change(&phase_wake);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn state_waker_fires_without_waiting_for_a_view_refresh() {
        let calls=Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count=Arc::clone(&calls);
        let wake:PhaseWake=Arc::new(Mutex::new(Some(Arc::new(move || {
            count.fetch_add(1,Ordering::Relaxed);
        }))));
        notify_phase_change(&wake);
        assert_eq!(calls.load(Ordering::Relaxed),1);
    }
    #[test]
    fn rejects_non_game_and_unencrypted_endpoints() {
        assert!(websocket_authority("ws://pokeidle.io/ws").is_err());
        assert!(websocket_authority("wss://pokeidle.io.evil.test/ws").is_err());
        assert!(websocket_authority("wss://someone@pokeidle.io/ws").is_err());
        assert!(websocket_authority("wss://ws.pokeidle.io/ws").is_ok());
    }
    #[test]
    fn lazy_background_history_does_not_require_startup_downloads() {
        assert!(BACKGROUND_FORECAST_INTERVAL>=Duration::from_secs(15*60));
        let reference:Arc<Mutex<Option<(Arc<AtlasReference>,bool)>>> =
            Arc::new(Mutex::new(None));
        assert!(reference.lock().unwrap().is_none());
    }
    #[test]
    fn current_hunt_forecasts_skip_center_and_incomplete_teams() {
        let mut view=HuntView::default();
        view.hunt_slug=Some("training".into());
        view.hunts=Arc::new(vec![crate::native_hunts::HuntEntry {
            slug:"training".into(), ..Default::default()
        }]);
        view.catalog_ready=true;
        assert!(current_background_hunt(&view).is_none());
        view.active_pokemon=Some(crate::native_hunts::PokemonInfo::default());
        assert_eq!(current_background_hunt(&view).unwrap().slug,"training");
        view.in_center=true;
        assert!(current_background_hunt(&view).is_none());
    }
    #[test]
    fn compact_diagnostics_reads_only_queue_metadata() {
        let mut state = NativeSnapshot::default();
        state.phase = "Online".into();
        state.native_capture_queue = 7;
        state.native_automation.queued = 3;
        state.native_automation.pending_field = Some("potionIds".into());
        state.native_market.pending = Some("order".into());
        state.native_restock.pending = true;
        let compact = NativeDiagnosticsSnapshot::from_state(&state);
        assert_eq!(compact.phase, "Online");
        assert_eq!(compact.capture_queued, 7);
        assert_eq!(compact.automation_queued, 3);
        assert!(compact.automation_pending);
        assert!(compact.market_pending);
        assert!(compact.restock_pending);
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
        observe(&status, r#"{"t":"batalha","ev":[
            {"k":"morte","quem":"selvagem"},
            {"k":"morte","quem":"jogador"},
            {"k":"bola","sucesso":true}
        ]}"#);
        let observed = status.lock().unwrap().clone();
        assert_eq!(observed.battle_kills, 1);
        assert_eq!(observed.captures, 1);
        assert_eq!(observed.frames, 1);
    }
    #[test]
    fn unchanged_official_kick_roster_does_not_advance_ui_revision() {
        let snapshot=Arc::new(Mutex::new(NativeSnapshot::default()));
        let frame=r#"{"t":"estado","estado":{"kick":{"oficiais":[
            {"slug":"sample","aoVivo":true}]}}}"#;
        let first_event=serde_json::from_str::<Value>(frame).unwrap();
        assert!(observe_value(&snapshot,&first_event));
        let first=snapshot.lock().unwrap().kick_roster_revision;
        assert!(first>0);
        assert!(!observe_value(&snapshot,&first_event));
        assert_eq!(snapshot.lock().unwrap().kick_roster_revision,first);
        let offline=serde_json::json!({"t":"estado","estado":{"kick":{"oficiais":[
            {"slug":"sample","aoVivo":false}]}}});
        assert!(observe_value(&snapshot,&offline));
        assert_eq!(snapshot.lock().unwrap().kick_roster_revision,first+1);
    }
    #[test]
    fn twitch_redemption_state_comes_from_game_without_chat_tracking() {
        let status = Arc::new(Mutex::new(NativeSnapshot::default()));
        observe(&status, r#"{"t":"welcome","estado":{"twitch":{
            "vinculado":true,"login":"trainer","pct":15,
            "bonus":{"xp":200,"captura":300,"shiny":400}
        }}}"#);
        let observed=status.lock().unwrap().clone();
        assert_eq!(observed.twitch_linked,Some(true));
        assert_eq!(observed.twitch_login.as_deref(),Some("trainer"));
        assert_eq!(observed.twitch_bonus_pct,15.0);
        assert_eq!(observed.twitch_xp_until,200);
        assert_eq!(observed.twitch_capture_until,300);
        assert_eq!(observed.twitch_shiny_until,400);
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

// Validate the control path without constructing a window, repaint timer, or
// authenticated socket. The game loop observes the shared settings directly.
#[cfg(test)]
mod background_capture_settings_tests {
    use super::*;
    #[test]
    fn saved_settings_reach_worker_with_no_ui_frames() {
        let config = Arc::new(Mutex::new(CatchSettings::default()));
        let auto_catch = Arc::new(AtomicBool::new(false));
        let restock_enabled = Arc::new(AtomicBool::new(false));
        let updater = CaptureSettingsUpdater {
            config: Arc::clone(&config),
            auto_catch: Arc::clone(&auto_catch),
            restock_enabled: Arc::clone(&restock_enabled),
        };
        let mut desired = CatchSettings::default();
        desired.enabled = true;
        desired.restock_enabled = true;
        let expected = desired.clone();
        std::thread::spawn(move || updater.apply(desired))
            .join().expect("settings worker panicked").expect("valid settings");
        assert_eq!(*config.lock().unwrap(), expected);
        assert!(auto_catch.load(Ordering::Relaxed));
        assert!(restock_enabled.load(Ordering::Relaxed));
    }
}
