use crate::kick::KickStream;
use crate::logging;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

type BrowserSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Debug)]
pub struct TabInfo {
    pub kind: String,
    pub title: String,
    pub url: String,
    pub low_resource: bool,
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
    pub active_pokemon: String,
    pub fallen_count: u32,
    pub economy_mode: bool,
    pub addon_ok: u8,
    pub addon_total: u8,
    pub addon_missing: Vec<String>,
    pub twitch_tabs: u8,
    pub twitch_low_resource_ok: u8,
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
    pub xp_bonuses: Vec<String>,
    pub player_level: u32,
    pub player_xp: String,
    pub gold: u64,
    pub orbs: u64,
    pub stream_bonus: String,
    pub stream_bonus_last: String,
    pub stream_bonus_pct: f32,
    pub stream_watching: Vec<String>,
    pub stream_live_bonus: Vec<String>,
    pub stream_missing: Vec<String>,
    pub xp_sources: Vec<String>,
    pub bridge_connected: bool,
    pub last_game_message_ms: u64,
    pub tabs: Vec<TabInfo>,
    pub kick_streams: Vec<KickStream>,
    pub kick_state_available: bool,
    pub last_error: Option<String>,
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
            active_pokemon: String::new(),
            fallen_count: 0,
            economy_mode: false,
            addon_ok: 0,
            addon_total: 7,
            addon_missing: Vec::new(),
            twitch_tabs: 0,
            twitch_low_resource_ok: 0,
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
            xp_bonuses: Vec::new(),
            player_level: 0,
            player_xp: String::new(),
            gold: 0,
            orbs: 0,
            stream_bonus: String::new(),
            stream_bonus_last: String::new(),
            stream_bonus_pct: 0.0,
            stream_watching: Vec::new(),
            stream_live_bonus: Vec::new(),
            stream_missing: Vec::new(),
            xp_sources: Vec::new(),
            bridge_connected: false,
            last_game_message_ms: 0,
            tabs: Vec::new(),
            kick_streams: Vec::new(),
            kick_state_available: false,
            last_error: None,
        }
    }
}

impl Health {
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
    commands: Arc<Mutex<Vec<Value>>>,
}

impl MonitorHandle {
    pub fn start(port: u16) -> Self {
        let health = Arc::new(Mutex::new(Health::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let commands = Arc::new(Mutex::new(Vec::new()));
        let shared = Arc::clone(&health);
        let stop_worker = Arc::clone(&stop);
        let command_queue = Arc::clone(&commands);

        thread::spawn(move || {
            monitor_loop(port, shared, stop_worker, command_queue);
        });

        Self { health, stop, commands }
    }

    pub fn send(&self, payload: Value) {
        if let Ok(mut commands) = self.commands.lock() {
            commands.push(payload);
        }
    }

    pub fn health(&self) -> Health {
        self.health.lock().map(|h| h.clone()).unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

struct BrowserSession {
    socket: BrowserSocket,
    next_id: u64,
    game_context: Option<String>,
}

#[derive(Default)]
struct RuntimeProbe {
    bridge_connected: bool,
    hunt_slug: String,
    player_level: u32,
    player_xp: String,
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
    stream_bonus: String,
    stream_bonus_last: String,
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
    twitch_tabs: u8,
    twitch_low_resource_ok: u8,
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
    xp_bonuses: Vec<String>,
    player_level: u32,
    player_xp: String,
    gold: u64,
    orbs: u64,
    stream_bonus: String,
    stream_bonus_last: String,
    stream_bonus_pct: f32,
    stream_watching: Vec<String>,
    stream_live_bonus: Vec<String>,
    stream_missing: Vec<String>,
    xp_sources: Vec<String>,
    bridge_connected: bool,
    last_game_message_ms: u64,
    tabs: Vec<TabInfo>,
    kick_streams: Vec<KickStream>,
    kick_state_available: bool,
}

fn monitor_loop(
    port: u16,
    health: Arc<Mutex<Health>>,
    stop: Arc<AtomicBool>,
    commands: Arc<Mutex<Vec<Value>>>,
) {
    let mut session: Option<BrowserSession> = None;

    loop {
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

                    if let Ok(mut current) = health.lock() {
                        current.state = "Running".to_string();
                        current.last_error = None;
                    }
                }
                Err(error) => {
                    logging::warn(&format!(
                        "BiDi session setup on port {} failed: {}",
                        port, error
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

        if let Some(browser_session) = session.as_mut() {
            flush_commands(browser_session, &commands);

            match probe_page(browser_session) {
                Ok(probe) => {
                    if let Ok(mut current) = health.lock() {
                        current.state = "Running".to_string();
                        current.url = probe.url;
                        current.title = probe.title;
                        current.game_ready = probe.game_ready;
                        current.logged_in = probe.logged_in;
                        current.activity = probe.activity;
                        current.hunt = probe.hunt;
                        current.hunt_slug = probe.hunt_slug;
                        current.active_pokemon = probe.active_pokemon;
                        current.fallen_count = probe.fallen_count;
                        current.economy_mode = probe.economy_mode;
                        current.addon_ok = probe.addon_ok;
                        current.addon_total = probe.addon_total;
                        current.addon_missing = probe.addon_missing;
                        current.twitch_tabs = probe.twitch_tabs;
                        current.twitch_low_resource_ok = probe.twitch_low_resource_ok;
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
                        current.xp_bonuses = probe.xp_bonuses;
                        current.player_level = probe.player_level;
                        current.player_xp = probe.player_xp;
                        current.gold = probe.gold;
                        current.orbs = probe.orbs;
                        current.stream_bonus = probe.stream_bonus.clone();
                        if !probe.stream_bonus.is_empty() {
                            current.stream_bonus_last = probe.stream_bonus.clone();
                        }
                        current.stream_bonus_pct = probe.stream_bonus_pct;
                        current.stream_watching = probe.stream_watching;
                        current.stream_live_bonus = probe.stream_live_bonus;
                        current.stream_missing = probe.stream_missing;
                        current.xp_sources = probe.xp_sources;
                        current.bridge_connected = probe.bridge_connected;
                        current.last_game_message_ms = probe.last_game_message_ms;
                        current.tabs = probe.tabs;
                        current.kick_streams = probe.kick_streams;
                        current.kick_state_available = probe.kick_state_available;
                        current.last_error = None;
                    }
                }
                Err(error) => {
                    logging::warn(&format!(
                        "BiDi page probe on port {} failed: {}",
                        port, error
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
        }

        for _ in 0..50 {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn open_session(port: u16) -> Result<BrowserSession, String> {
    logging::info(&format!(
        "connecting to Firefox BiDi on 127.0.0.1:{}",
        port
    ));

    let (mut socket, _) = connect(format!("ws://127.0.0.1:{port}/session"))
        .map_err(|error| error.to_string())?;

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
    })
}

fn probe_page(session: &mut BrowserSession) -> Result<Probe, String> {
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

        const addonChecks = [
            ['Auto Catch+', exists('#moth-ac-panel') || exists('#moth-ac-toggle')],
            ['Performance+', exists('#moth-performance-panel') || exists('#moth-performance-trigger')],
            ['Stream Scanner', exists('#moth-scan-live-streams')],
            ['Controller Bridge', (() => {
                const bridge = window.__mothControllerBridgeV1;
                return !!bridge
                    && Number(bridge.version) >= 4
                    && typeof bridge.snapshot === 'function'
                    && typeof bridge.gameSnapshot === 'function'
                    && typeof bridge.socket === 'function';
            })()],
            ['Twitch Low Resource', exists('#moth-twitch-low-resource-addon-ready') || exists('#moth-twitch-low-resource-css')],
            ['Hunt Atlas', !!window.__mothHuntAtlasControllerV1],
            ['Moth Watch', !!window.__mothMarketWatchControllerV1 || document.documentElement?.dataset?.mothWatchReady === '1']
        ];

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
            addons: addonChecks.filter(([, ok]) => ok).map(([name]) => name),
            addonMissing: addonChecks.filter(([, ok]) => !ok).map(([name]) => name),
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

    let addon_ok = 7u8.saturating_sub(addon_missing.len() as u8);

    // The upstream game intentionally performs several large downloads and CPU-heavy
    // catalog transforms before the login screen appears. Do not compete with that boot
    // pipeline by evaluating every tab and serializing the full controller bridge snapshot
    // while the loading overlay is visible.
    let page_loading = page
        .get("activity")
        .and_then(Value::as_str)
        == Some("Loading");

    let tabs = if page_loading {
        Vec::new()
    } else {
        build_tab_infos(&contexts, &mut session.socket, &mut session.next_id)
    };

    let twitch_tabs = tabs
        .iter()
        .filter(|tab| tab.kind == "Twitch")
        .count() as u8;

    let twitch_low_resource_ok = tabs
        .iter()
        .filter(|tab| tab.kind == "Twitch" && tab.low_resource)
        .count() as u8;

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
        addon_total: 7,
        addon_missing,
        twitch_tabs,
        twitch_low_resource_ok,
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
        xp_bonuses: runtime.xp_bonuses,
        player_level: page.get("playerLevel").and_then(Value::as_str)
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(runtime.player_level),
        player_xp: if page.get("playerXp").and_then(Value::as_str).unwrap_or_default().is_empty() {
            runtime.player_xp
        } else {
            page.get("playerXp").and_then(Value::as_str).unwrap_or_default().to_string()
        },
        gold: runtime.gold,
        orbs: runtime.orbs,
        stream_bonus: if runtime.stream_bonus.is_empty() {
            page.get("streamBonus").and_then(Value::as_str).unwrap_or_default().to_string()
        } else {
            runtime.stream_bonus.clone()
        },
        stream_bonus_last: if runtime.stream_bonus_last.is_empty() {
            page.get("streamBonus").and_then(Value::as_str).unwrap_or_default().to_string()
        } else {
            runtime.stream_bonus_last.clone()
        },
        stream_bonus_pct: runtime.stream_bonus_pct,
        stream_watching: runtime.stream_watching,
        stream_live_bonus: runtime.stream_live_bonus,
        stream_missing: runtime.stream_missing,
        xp_sources: runtime.xp_sources,
        bridge_connected: runtime.bridge_connected,
        last_game_message_ms: runtime.last_game_message_ms,
        tabs,
        kick_streams: runtime.kick_streams,
        kick_state_available: runtime.kick_state_available,
    })
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
                    "expression": "JSON.stringify({ title: document.title || '', lowResource: !!document.querySelector('#moth-twitch-low-resource-css, #moth-twitch-low-resource-chat-css') })",
                    "target": { "context": context.id },
                    "awaitPromise": false
                }
            }),
        );

        let (title, low_resource) = result
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
            ))
            .unwrap_or_default();

        tabs.push(TabInfo {
            kind: kind.to_string(),
            title: if title.is_empty() { context.url.clone() } else { title },
            url: context.url.clone(),
            low_resource: kind == "Twitch" && low_resource,
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
                    let name = item.get("name").and_then(Value::as_str)?.trim();
                    let url = item.get("url").and_then(Value::as_str)?.trim();
                    if name.is_empty() || url.is_empty() {
                        return None;
                    }
                    Some(KickStream {
                        name: name.to_string(),
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

    let stream_bonus_last = state
        .get("lastStreamBonus")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

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
            "{} · chat not open: {}",
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

    if !stream_bonus_last.is_empty()
        && !xp_sources.iter().any(|item| item == &stream_bonus_last)
    {
        xp_sources.push(format!("Last seen: {}", stream_bonus_last));
    }

    for bonus in [stream_bonus.clone(), stream_bonus_last.clone()] {
        if !bonus.is_empty() && !xp_bonuses.iter().any(|item| item == &bonus) {
            xp_bonuses.push(bonus);
        }
    }

    Ok(RuntimeProbe {
        bridge_connected,
        hunt_slug,
        player_level,
        player_xp,
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
        stream_bonus,
        stream_bonus_last,
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

fn flush_commands(
    session: &mut BrowserSession,
    commands: &Arc<Mutex<Vec<Value>>>,
) {
    let pending = match commands.lock() {
        Ok(mut commands) => std::mem::take(&mut *commands),
        Err(_) => Vec::new(),
    };

    for payload in pending {
        if payload.get("t").and_then(Value::as_str) != Some("browser.openTab") {
            logging::warn("unsupported controller command dropped");
            continue;
        }

        let url = payload
            .get("url")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();

        if url.is_empty() || !(url.starts_with("https://") || url.starts_with("http://")) {
            logging::warn("custom browser tab rejected: URL must use http:// or https://");
            continue;
        }

        if let Err(error) = open_custom_tab(session, url) {
            logging::warn(&format!("custom browser tab failed: {}", error));
        }
    }
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