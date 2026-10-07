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
            addon_total: 6,
            addon_missing: Vec::new(),
            twitch_tabs: 0,
            twitch_low_resource_ok: 0,
            performance_fps: String::new(),
            performance_scene_runs: String::new(),
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
}

impl MonitorHandle {
    pub fn start(port: u16) -> Self {
        let health = Arc::new(Mutex::new(Health::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&health);
        let stop_worker = Arc::clone(&stop);

        thread::spawn(move || monitor_loop(port, shared, stop_worker));

        Self { health, stop }
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
}

fn monitor_loop(
    port: u16,
    health: Arc<Mutex<Health>>,
    stop: Arc<AtomicBool>,
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

    let tree = send_and_wait(
        &mut socket,
        2,
        json!({
            "id": 2,
            "method": "browsingContext.getTree",
            "params": {}
        }),
    )?;

    let context = tree
        .get("result")
        .and_then(|v| v.get("contexts"))
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .and_then(|v| v.get("context"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Firefox returned no browsing context".to_string())?
        .to_string();

    Ok(BrowserSession {
        socket,
        next_id: 3,
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
            collect_contexts(context, &mut contexts);
        }
    }

    let game = contexts
        .iter()
        .find(|c| {
            c.url.starts_with("https://pokeidle.io/app")
                || c.url.starts_with("https://www.pokeidle.io/app")
        })
        .ok_or_else(|| "Firefox has no PokéIdle app context".to_string())?;

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
            ['Hunt Atlas', exists('#moth-hunt-atlas-button') || exists('#moth-hunt-atlas-drawer')],
            ['Moth Watch', exists('#moth-market-watch-tab') || exists('#moth-market-watch-panel')],
            ['Stream Scanner', exists('#moth-scan-live-streams')],
            ['Upstream Scraper', exists('#moth-upstream-scraper') || exists('#moth-upstream-scraper-button')]
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
            performanceSceneRuns: text('#mpp-scene-runs')
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

    let page: Value =
        serde_json::from_str(raw).map_err(|error| format!("invalid page probe: {error}"))?;

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

    let addon_ok = 6u8.saturating_sub(addon_missing.len() as u8);

    let mut twitch_tabs = 0u8;
    let mut twitch_low_resource_ok = 0u8;

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

    Ok(Probe {
        url: page.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        title: page.get("title").and_then(Value::as_str).unwrap_or_default().to_string(),
        game_ready: page.get("gameReady").and_then(Value::as_bool).unwrap_or(false),
        logged_in: page.get("loggedIn").and_then(Value::as_bool).unwrap_or(false),
        activity: page.get("activity").and_then(Value::as_str).unwrap_or("Unknown").to_string(),
        hunt: page.get("hunt").and_then(Value::as_str).unwrap_or_default().to_string(),
        active_pokemon: page.get("activePokemon").and_then(Value::as_str).unwrap_or_default().to_string(),
        fallen_count: page.get("fallenCount").and_then(Value::as_u64).unwrap_or(0) as u32,
        economy_mode: page.get("economyMode").and_then(Value::as_bool).unwrap_or(false),
        addon_ok,
        addon_total: 6,
        addon_missing,
        twitch_tabs,
        twitch_low_resource_ok,
        performance_fps: page.get("performanceFps").and_then(Value::as_str).unwrap_or_default().to_string(),
        performance_scene_runs: page.get("performanceSceneRuns").and_then(Value::as_str).unwrap_or_default().to_string(),
    })
}

struct ContextInfo {
    id: String,
    url: String,
}

fn collect_contexts(value: &Value, out: &mut Vec<ContextInfo>) {
    let Some(id) = value.get("context").and_then(Value::as_str) else {
        return;
    };

    out.push(ContextInfo {
        id: id.to_string(),
        url: value.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
    });

    if let Some(children) = value.get("children").and_then(Value::as_array) {
        for child in children {
            collect_contexts(child, out);
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

        return Ok(value);
    }
}
