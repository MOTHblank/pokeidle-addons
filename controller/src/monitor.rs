use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tungstenite::{connect, Message, WebSocket};

#[derive(Clone, Debug)]
pub struct Health {
    pub state: String,
    pub url: String,
    pub title: String,
    pub game_ready: bool,
    pub last_error: Option<String>,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            state: "Starting".to_string(),
            url: String::new(),
            title: String::new(),
            game_ready: false,
            last_error: None,
        }
    }
}

impl Health {
    pub fn summary(&self) -> String {
        match self.state.as_str() {
            "Running" => format!(
                "{} · {} · {}",
                if self.game_ready { "game UI OK" } else { "game UI waiting" },
                if self.title.is_empty() { "no title" } else { &self.title },
                if self.url.is_empty() { "no URL" } else { &self.url }
            ),
            _ => self.last_error.clone().unwrap_or_else(|| self.state.clone()),
        }
    }
}

#[derive(Clone)]
pub struct MonitorHandle {
    health: Arc<Mutex<Health>>,
}

impl MonitorHandle {
    pub fn start(port: u16) -> Self {
        let health = Arc::new(Mutex::new(Health::default()));
        let shared = Arc::clone(&health);

        thread::spawn(move || monitor_loop(port, shared));

        Self { health }
    }

    pub fn health(&self) -> Health {
        self.health.lock().map(|h| h.clone()).unwrap_or_default()
    }
}

fn monitor_loop(port: u16, health: Arc<Mutex<Health>>) {
    loop {
        match probe_browser(port) {
            Ok(probe) => {
                if let Ok(mut current) = health.lock() {
                    current.state = "Running".to_string();
                    current.url = probe.url;
                    current.title = probe.title;
                    current.game_ready = probe.game_ready;
                    current.last_error = None;
                }
            }
            Err(error) => {
                if let Ok(mut current) = health.lock() {
                    current.state = "Connecting".to_string();
                    current.last_error = Some(error);
                    current.game_ready = false;
                }
            }
        }

        thread::sleep(Duration::from_secs(5));
    }
}

struct Probe {
    url: String,
    title: String,
    game_ready: bool,
}

fn probe_browser(port: u16) -> Result<Probe, String> {
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
        .ok_or_else(|| "Firefox returned no browsing context".to_string())?;

    let expression = r#"JSON.stringify({
        url: location.href,
        title: document.title,
        gameReady: !!document.getElementById('btn-bolsa')
    })"#;

    let result = send_and_wait(
        &mut socket,
        3,
        json!({
            "id": 3,
            "method": "script.evaluate",
            "params": {
                "expression": expression,
                "target": { "context": context },
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

    Ok(Probe {
        url: page.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        title: page.get("title").and_then(Value::as_str).unwrap_or_default().to_string(),
        game_ready: page.get("gameReady").and_then(Value::as_bool).unwrap_or(false),
    })
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

        let value: Value =
            serde_json::from_str(text.as_ref()).map_err(|error| format!("invalid BiDi JSON: {error}"))?;

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
