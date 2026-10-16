//! Opt-in experiment: keep a PokéIdle game WebSocket alive in Rust after Firefox
//! closes. Independently implemented from observed game protocol. Not a drop-in
//! replacement for browser userscripts (Auto Catch, Twitch, Hunt Atlas, etc.).
//!
//! Credentials are captured only from the existing, authenticated Firefox BiDi
//! session, held in memory, and never printed or persisted by this module.
use serde_json::Value;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client_tls, http::header, Message};
use tungstenite::client::IntoClientRequest;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const READ_TIMEOUT: Duration = Duration::from_secs(2);
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
    pub error: Option<String>,
}
impl Default for NativeSnapshot {
    fn default() -> Self {
        Self {
            phase: "Connecting".into(), last_received: None,
            trainer_level: None, trainer_xp: None, gold: None, orbs: None,
            hunt_slug: None, frames: 0, battle_kills: 0, captures: 0, error: None,
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

pub struct NativeSession {
    snapshot: Arc<Mutex<NativeSnapshot>>,
    stop: Arc<AtomicBool>,
}
impl NativeSession {
    pub fn start(bootstrap: Value) -> Result<Self, String> {
        let credentials = Credentials::parse(&bootstrap)?;
        let snapshot = Arc::new(Mutex::new(NativeSnapshot::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new().name("moth-native-game".to_owned())
            .spawn(move || session_loop(credentials, worker_snapshot, worker_stop))
            .map_err(|error| format!("Could not spawn native session: {error}"))?;
        Ok(Self { snapshot, stop })
    }
    pub fn snapshot(&self) -> NativeSnapshot {
        self.snapshot.lock().map(|status| status.clone()).unwrap_or_default()
    }
    pub fn stop(&self) { self.stop.store(true, Ordering::Relaxed); }
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
        .map_err(|error| format!("Game TLS/WebSocket handshake failed: {error}"))?;
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

fn observe(snapshot: &Arc<Mutex<NativeSnapshot>>, text: &str) {
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
        }
    });
}

fn session_loop(credentials: Credentials,
    snapshot: Arc<Mutex<NativeSnapshot>>, stop: Arc<AtomicBool>) {
    let mut ever_connected = false;
    let mut reconnect_failures = 0usize;
    while !stop.load(Ordering::Relaxed) {
        match open_session(&credentials) {
            Ok((mut socket, welcome)) => {
                observe(&snapshot, &welcome);
                ever_connected = true;
                reconnect_failures = 0;
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
                    if last_ping.elapsed() >= HEARTBEAT_INTERVAL {
                        if socket.send(Message::Ping(Vec::new().into())).is_err() { break; }
                        last_ping = Instant::now();
                    }
                    match socket.read() {
                        Ok(Message::Text(text)) => observe(&snapshot, text.as_ref()),
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
                    update(&snapshot, |s| s.phase = "Reconnecting".into());
                }
            }
            Err(error) => {
                reconnect_failures += 1;
                let failed = !ever_connected || reconnect_failures >= 5;
                update(&snapshot, |s| {
                    s.phase = if failed { "Failed" } else { "Reconnecting" }.into();
                    s.error = Some(error);
                });
                if failed { break; }
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
        observe(&status, r#"{"t":"batalha","ev":[{"k":"morte"},{"k":"bola","sucesso":true}]}"#);
        let observed = status.lock().unwrap().clone();
        assert_eq!(observed.battle_kills, 1);
        assert_eq!(observed.captures, 1);
        assert_eq!(observed.frames, 1);
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
