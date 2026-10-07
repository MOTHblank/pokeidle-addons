use crate::logging;
use native_windows_gui as nwg;
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
                if self.game_ready {
                    "game UI OK"
                } else {
                    "game UI waiting"
                },
                if self.title.is_empty() {
                    "no title"
                } else {
                    &self.title
                },
                if self.url.is_empty() {
                    "no URL"
                } else {
                    &self.url
                }
            ),
            _ => self
                .last_error
                .clone()
                .unwrap_or_else(|| self.state.clone()),
        }
    }
}

#[derive(Clone)]
pub struct MonitorHandle {
    health: Arc<Mutex<Health>>,
    stop: Arc<AtomicBool>,
}

impl MonitorHandle {
    pub fn start(port: u16, notice_sender: nwg::NoticeSender) -> Self {
        let health = Arc::new(Mutex::new(Health::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::clone(&health);
        let stop_worker = Arc::clone(&stop);

        thread::spawn(move || monitor_loop(port, shared, stop_worker, notice_sender));

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
    context: String,
    next_id: u64,
}

fn monitor_loop(port: u16, health: Arc<Mutex<Health>>, stop: Arc<AtomicBool>, notice_sender: nwg::NoticeSender) {
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
                    notice_sender.notice();
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
                    }
                    notice_sender.notice();
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
                        current.last_error = None;
                    }
                    notice_sender.notice();
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
                    }
                    notice_sender.notice();

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

    Ok(BrowserSession { socket, context, next_id: 3 })
}

fn probe_page(session: &mut BrowserSession) -> Result<Probe, String> {
    let expression = r#"JSON.stringify({
        url: location.href,
        title: document.title,
        gameReady: !!document.getElementById('btn-bolsa')
    })"#;

    let command_id = session.next_id;
    session.next_id += 1;

    let result = send_and_wait(
        &mut session.socket,
        command_id,
        json!({
            "id": 3,
            "method": "script.evaluate",
            "params": {
                "expression": expression,
                "target": { "context": session.context },
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
        url: page
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: page
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        game_ready: page
            .get("gameReady")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn end_session(session: &mut Option<BrowserSession>) {
    let Some(mut session) = session.take() else {
        return;
    };

    let _ = send_and_wait(
        &mut session.socket,
        4,
        json!({
            "id": 4,
            "method": "session.end",
            "params": {}
        }),
    );

    let _ = session.socket.close(None);
}

struct Probe {
    url: String,
    title: String,
    game_ready: bool,
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
