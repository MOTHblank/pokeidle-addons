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
pub struct HuntInfo {
    pub slug: String,
    pub name: String,
    pub level: u32,
    pub species: Vec<String>,
    pub xp_per_hour: u64,
}

#[derive(Clone, Debug)]
pub struct MarketListing {
    pub id: u64,
    pub item_id: u64,
    pub name: String,
    pub currency: String,
    pub price: u64,
    pub quantity: u64,
    pub seller: String,
    pub retained_until: u64,
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
    pub tabs: Vec<TabInfo>,
    pub hunts: Vec<HuntInfo>,
    pub market_listings: Vec<MarketListing>,
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
            active_pokemon: String::new(),
            fallen_count: 0,
            economy_mode: false,
            addon_ok: 0,
            addon_total: 5,
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
            tabs: Vec::new(),
            hunts: Vec::new(),
            market_listings: Vec::new(),
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

    pub fn details(&self) -> String {
        let addon = if self.addon_missing.is_empty() {
            format!("Addons {}/{} ✓", self.addon_ok, self.addon_total)
        } else {
            format!(
                "Addons {}/{} · missing: {}",
                self.addon_ok,
                self.addon_total,
                self.addon_missing.join(", ")
            )
        };

        let mut parts = vec![addon];

        if self.twitch_tabs > 0 {
            parts.push(format!(
                "Twitch LR {}/{}",
                self.twitch_low_resource_ok,
                self.twitch_tabs
            ));
        }

        if !self.performance_scene_runs.is_empty() {
            parts.push(format!("Scene {}", self.performance_scene_runs));
        }

        if !self.performance_fps.is_empty() && self.performance_fps != "—" {
            parts.push(format!("FPS {}", self.performance_fps));
        }

        if self.economy_mode {
            parts.push("Economy".to_string());
        }

        parts.join(" · ")
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
    player_level: u32,
    player_xp: String,
    gold: u64,
    orbs: u64,
    fallen_count: u32,

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
    xp_bonuses: Vec<String>,
    hunts: Vec<HuntInfo>,
    market_listings: Vec<MarketListing>,
}

struct Probe {
    url: String,
    title: String,
    game_ready: bool,
    logged_in: bool,
    activity: String,
    hunt: String,
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
    tabs: Vec<TabInfo>,
    hunts: Vec<HuntInfo>,
    market_listings: Vec<MarketListing>,
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
                        current.stream_bonus = probe.stream_bonus;
                        current.tabs = probe.tabs;
                        current.hunts = probe.hunts;
                        current.market_listings = probe.market_listings;
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
        if (loggedIn) {
            activity =
                huntSelected || visible('#golpes-painel')
                    ? 'Hunting'
                    : 'Center';
        }

        const addonChecks = [
            ['Auto Catch+', exists('#moth-ac-panel') || exists('#moth-ac-toggle')],
            ['Performance+', exists('#moth-performance-panel') || exists('#moth-performance-trigger')],
            ['Stream Scanner', exists('#moth-scan-live-streams')],
            ['Upstream Scraper', exists('#moth-upstream-scraper') || exists('#moth-upstream-scraper-button')],
            ['Controller Bridge', !!window.__mothControllerBridgeV1]
        ];

        return JSON.stringify({
            url: location.href,
            title: document.title,
            gameReady: loggedIn,
            loggedIn,
            activity,
            hunt: huntSelected ? huntText.slice(0, 80) : '',
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

    let addon_ok = 5u8.saturating_sub(addon_missing.len() as u8);

    let mut twitch_tabs = 0u8;
    let mut twitch_low_resource_ok = 0u8;
    let mut xp_bonuses = page
        .get("xpBonuses")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for context in &contexts {
        if !context.url.starts_with("https://www.twitch.tv/")
            && !context.url.starts_with("https://twitch.tv/")
            && !context.url.starts_with("https://player.twitch.tv/")
            && !context.url.starts_with("https://m.twitch.tv/")
        {
            continue;
        }

        twitch_tabs = twitch_tabs.saturating_add(1);

        let twitch_id = session.next_id;
        session.next_id += 1;

        let twitch_probe = send_and_wait(
            &mut session.socket,
            twitch_id,
            json!({
                "id": twitch_id,
                "method": "script.evaluate",
                "params": {
                    "expression": "JSON.stringify({ lowResource: !!document.querySelector('#moth-twitch-low-resource-css') })",
                    "target": { "context": context.id },
                    "awaitPromise": false
                }
            }),
        );

        if let Ok(value) = twitch_probe {
            let low_resource = value
                .get("result")
                .and_then(|v| v.get("result"))
                .and_then(|v| v.get("value"))
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .and_then(|v| v.get("lowResource").and_then(Value::as_bool))
                .unwrap_or(false);

            if low_resource {
                twitch_low_resource_ok = twitch_low_resource_ok.saturating_add(1);
            }
        }
    }

    let stream_scan_opened = page.get("streamScanOpened").and_then(Value::as_u64).unwrap_or(0) as u32;
    if stream_scan_opened > 0
        && twitch_tabs > 0
        && !xp_bonuses.iter().any(|value| value.to_ascii_lowercase().contains("twitch"))
    {
        xp_bonuses.push("Twitch stream · +15% XP (chat open)".to_string());
    }

    let runtime = probe_runtime_details(session, &game.id).unwrap_or_default();

    Ok(Probe {
        url: page.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        title: page.get("title").and_then(Value::as_str).unwrap_or_default().to_string(),
        game_ready: page.get("gameReady").and_then(Value::as_bool).unwrap_or(false),
        logged_in: page.get("loggedIn").and_then(Value::as_bool).unwrap_or(false),
        activity: page.get("activity").and_then(Value::as_str).unwrap_or("Unknown").to_string(),
        hunt: page.get("hunt").and_then(Value::as_str).unwrap_or_default().to_string(),
        active_pokemon: runtime.pokemon_level.is_empty()
            .then(|| page.get("activePokemon").and_then(Value::as_str).unwrap_or_default().to_string())
            .unwrap_or_else(|| {
                let bridge_name = runtime.pokemon_xp.clone();
                if bridge_name.is_empty() {
                    page.get("activePokemon").and_then(Value::as_str).unwrap_or_default().to_string()
                } else {
                    page.get("activePokemon").and_then(Value::as_str).unwrap_or_default().to_string()
                }
            }),
        fallen_count: runtime.fallen_count,
        economy_mode: page.get("economyMode").and_then(Value::as_bool).unwrap_or(false),
        addon_ok,
        addon_total: 6,
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
        player_xp: page.get("playerXp").and_then(Value::as_str).unwrap_or_default().to_string(),
        gold: runtime.gold,
        orbs: runtime.orbs,
        stream_bonus: if runtime.stream_bonus.is_empty() {
            page.get("streamBonus").and_then(Value::as_str).unwrap_or_default().to_string()
        } else {
            runtime.stream_bonus
        },
        tabs: build_tab_infos(&contexts, &mut session.socket, &mut session.next_id),
        hunts: runtime.hunts,
        market_listings: runtime.market_listings,
    })
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
                    "expression": "JSON.stringify({ title: document.title || '', lowResource: !!document.querySelector('#moth-twitch-low-resource-css') })",
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

    let state = snapshot.get("state").cloned().unwrap_or(Value::Null);
    let active = state.get("activePokemon").cloned().unwrap_or(Value::Null);

    let player_level =
        state.get("level").and_then(Value::as_u64).unwrap_or(0) as u32;
    let player_xp = snapshot
        .get("state")
        .and_then(|s| s.get("playerXp"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let fallen_count =
        state.get("fallen").and_then(Value::as_u64).unwrap_or(0) as u32;
    let gold = state.get("gold").and_then(Value::as_u64).unwrap_or(0);
    let orbs = state.get("orbs").and_then(Value::as_u64).unwrap_or(0);

    let pokemon_level = active
        .get("level")
        .and_then(Value::as_u64)
        .map(|v| v.to_string())
        .unwrap_or_default();

    let pokemon_xp = state
        .get("activePokemonXp")
        .and_then(Value::as_u64)
        .map(|v| v.to_string())
        .unwrap_or_default();

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

    let stream_bonus = state
        .get("visibleStreamBonus")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if !stream_bonus.is_empty()
        && !xp_bonuses.iter().any(|item| item == &stream_bonus)
    {
        xp_bonuses.push(stream_bonus.clone());
    }

    let current_hunt = state
        .get("huntSlug")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let battle_events = snapshot
        .get("battleEvents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut kill_times: std::collections::HashMap<String, Vec<u64>> =
        std::collections::HashMap::new();

    for entry in battle_events {
        if entry.get("event").and_then(|e| e.get("k")).and_then(Value::as_str) != Some("morte") {
            continue;
        }

        let hunt = entry
            .get("hunt")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if hunt.is_empty() {
            continue;
        }

        let at = entry.get("at").and_then(Value::as_u64).unwrap_or(0);
        kill_times.entry(hunt.to_string()).or_default().push(at);
    }

    let hunts = snapshot
        .get("hunts")
        .and_then(Value::as_array)
        .map(|values| {
            values.iter().map(|hunt| {
                let slug = hunt.get("slug").and_then(Value::as_str).unwrap_or_default().to_string();
                let name = hunt.get("name").and_then(Value::as_str).unwrap_or(&slug).to_string();
                let level = hunt.get("level").and_then(Value::as_u64).unwrap_or(0) as u32;
                let species = hunt.get("species").and_then(Value::as_array).map(|values| {
                    values.iter()
                        .filter_map(|species| species.get("name").and_then(Value::as_str).map(str::to_string))
                        .collect::<Vec<_>>()
                }).unwrap_or_default();

                let kills = kill_times.get(&slug).cloned().unwrap_or_default();
                let xp_per_hour = if kills.len() >= 2 {
                    let first = *kills.first().unwrap_or(&0);
                    let last = *kills.last().unwrap_or(&first);
                    let elapsed_ms = last.saturating_sub(first);
                    if elapsed_ms >= 1000 {
                        let kills_per_hour = (kills.len() as f64) * 3_600_000.0 / elapsed_ms as f64;
                        let base_xp = (0.6_f64 * (level as f64).powi(2) + 8.0).floor();
                        (kills_per_hour * base_xp).round() as u64
                    } else {
                        0
                    }
                } else {
                    0
                };

                HuntInfo {
                    slug,
                    name,
                    level,
                    species,
                    xp_per_hour,
                }
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let catalog = snapshot
        .get("catalog")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut names = std::collections::HashMap::new();
    for item in catalog {
        if let Some(id) = item.get("id").and_then(Value::as_u64) {
            let name = item.get("nome")
                .or_else(|| item.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("Item")
                .to_string();
            names.insert(id, name);
        }
    }

    let mut market_listings = Vec::new();
    if let Some(messages) = snapshot.get("market").and_then(Value::as_array) {
        for entry in messages {
            let Some(message) = entry.get("message") else { continue; };
            if message.get("aba").and_then(Value::as_str) != Some("item") {
                continue;
            }

            let item_id = message.get("itemId").and_then(Value::as_u64).unwrap_or(0);
            let currency = message.get("moeda").and_then(Value::as_str).unwrap_or("gold").to_string();
            let name = names.get(&item_id).cloned().unwrap_or_else(|| format!("Item {}", item_id));

            if let Some(lines) = message.get("linhas").and_then(Value::as_array) {
                for listing in lines {
                    let id = listing.get("id").and_then(Value::as_u64).unwrap_or(0);
                    let price = listing.get("preco").and_then(Value::as_u64).unwrap_or(0);
                    if id == 0 || price == 0 { continue; }

                    market_listings.push(MarketListing {
                        id,
                        item_id,
                        name: name.clone(),
                        currency: currency.clone(),
                        price,
                        quantity: listing.get("qtd").and_then(Value::as_u64).unwrap_or(1),
                        seller: listing.get("vendedor").and_then(Value::as_str).unwrap_or("—").to_string(),
                        retained_until: listing.get("compravelEm").and_then(Value::as_u64).unwrap_or(0),
                    });
                }
            }
        }
    }

    market_listings.sort_by_key(|listing| listing.price);
    market_listings.truncate(100);

    Ok(RuntimeProbe {
        player_level,
        player_xp,
        gold,
        orbs,
        fallen_count,
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
        xp_bonuses,
        hunts,
        market_listings,
    })
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
        let id = session.next_id;
        session.next_id += 1;

        let expression = "JSON.stringify(window.__mothControllerBridgeV1 ? window.__mothControllerBridgeV1.send(ARG) : {ok:false,error:'controller bridge missing'})";
        let arg = match serde_json::to_string(&payload) {
            Ok(value) => value,
            Err(error) => {
                logging::warn(&format!("failed to serialize controller command: {}", error));
                continue;
            }
        };

        let expression = format!(
            "(payload => {})({})",
            expression,
            arg
        );

        match send_and_wait(
            &mut session.socket,
            id,
            json!({
                "id": id,
                "method": "script.evaluate",
                "params": {
                    "expression": expression,
                    "target": {
                    "context": match session.game_context.as_deref() {
                        Some(context) => context,
                        None => {
                            logging::warn("controller command dropped: no PokéIdle context");
                            continue;
                        }
                    }
                },
                    "awaitPromise": false
                }
            }),
        ) {
            Ok(_) => {}
            Err(error) => {
                logging::warn(&format!("controller command failed: {}", error));
            }
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
