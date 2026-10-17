//! Browserless Twitch IRC for the native-first controller.
//! Credentials are observed from Firefox's *existing authenticated IRC socket*
//! during bootstrap. No access token or PASS frame is logged or persisted.
use std::collections::BTreeSet;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client_tls, client::IntoClientRequest, http::header, Message};

const IRC_HOST: &str = "irc-ws.chat.twitch.tv";
const IRC_URL: &str = "wss://irc-ws.chat.twitch.tv:443";
const AUTH_TIMEOUT: Duration = Duration::from_secs(15);
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const JOIN_GAP: Duration = Duration::from_millis(600);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const MAX_ROOMS: usize = 100;

#[derive(Clone, Debug)]
pub struct TwitchSnapshot {
    pub phase: String,
    pub desired: usize,
    pub joined: usize,
    pub last_received: Option<Instant>,
    pub error: Option<String>,
}
impl Default for TwitchSnapshot {
    fn default() -> Self {
        Self { phase: "Connecting".into(), desired: 0, joined: 0,
            last_received: None, error: None }
    }
}

struct IrcIdentity {
    nick: String,
    pass: String,
    cap: String,
}
impl IrcIdentity {
    fn from_browser(value: &serde_json::Value) -> Result<Self, String> {
        let url = value.get("url").and_then(serde_json::Value::as_str).unwrap_or("");
        if url != IRC_URL && url != "wss://irc-ws.chat.twitch.tv" &&
            url != "wss://irc-ws.chat.twitch.tv:443/" &&
            url != "wss://irc-ws.chat.twitch.tv/" {
            return Err("Native Twitch handoff refused a non-IRC Twitch endpoint".into());
        }
        let pass = value.get("pass").and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Twitch IRC PASS wasn't captured".to_owned())?;
        let nick = value.get("nick").and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Twitch IRC NICK wasn't captured".to_owned())?;
        // Twitch's web IRC client can omit its CAP request on reconnect.
        // An empty captured capability string is not an authentication error.
        let cap = value.get("cap").and_then(serde_json::Value::as_str)
            .filter(|cap| !cap.trim().is_empty())
            .unwrap_or("CAP REQ :twitch.tv/tags twitch.tv/commands");
        if !pass.starts_with("PASS oauth:") || pass.len() > 2048 ||
            !pass[11..].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err("Captured Twitch IRC authentication is not OAuth PASS".into());
        }
        let Some(username) = nick.strip_prefix("NICK ") else {
            return Err("Captured Twitch IRC NICK is invalid".into());
        };
        if !valid_channel(username) || username.len() > 25 {
            return Err("Invalid Twitch username".into());
        }
        if !cap.starts_with("CAP REQ :") || cap.len() > 256 ||
            !cap.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b' ' | b':' | b'/' | b'_' | b'-' | b'.')) {
            return Err("Invalid Twitch IRC capabilities".into());
        }
        Ok(Self { nick: nick.into(), pass: pass.into(), cap: cap.into() })
    }
}
pub fn validate(value: &serde_json::Value) -> Result<(), String> {
    IrcIdentity::from_browser(value).map(|_| ())
}
fn valid_channel(value: &str) -> bool {
    !value.is_empty() && value.len() <= 25 &&
        value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn normalize_channels(channels: Vec<String>) -> Vec<String> {
    channels.into_iter().map(|channel| channel.trim_start_matches('#').to_ascii_lowercase())
        .filter(|channel| valid_channel(channel))
        .collect::<BTreeSet<_>>().into_iter().take(MAX_ROOMS).collect()
}
fn write_irc(socket: &mut tungstenite::WebSocket<
    tungstenite::stream::MaybeTlsStream<TcpStream>>, message: &str) -> Result<(), String> {
    socket.send(Message::Text(format!("{message}\r\n").into()))
        .map_err(|_| "Could not send data to Twitch".to_owned())
}
fn connect(identity: &IrcIdentity) -> Result<tungstenite::WebSocket<
    tungstenite::stream::MaybeTlsStream<TcpStream>>, String> {
    crate::native_tls::ensure()?;
    let addrs = (IRC_HOST, 443).to_socket_addrs()
        .map_err(|_| "Could not find Twitch. Check your internet connection.".to_owned())?;
    let mut tcp = None;
    for address in addrs {
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_secs(7)) {
            tcp = Some(stream);
            break;
        }
    }
    let stream = tcp.ok_or_else(|| "Could not connect to Twitch. Check your internet connection.".to_owned())?;
    stream.set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "Could not set up the Twitch connection".to_owned())?;
    stream.set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "Could not set up Twitch messages".to_owned())?;
    let mut request = IRC_URL.into_client_request()
        .map_err(|_| "Invalid Twitch IRC URL".to_owned())?;
    request.headers_mut().insert(header::ORIGIN,
        header::HeaderValue::from_static("https://www.twitch.tv"));
    let (mut socket, _) = client_tls(request, stream)
        .map_err(|_| "Secure connection to Twitch failed".to_owned())?;
    write_irc(&mut socket, &identity.cap)?;
    write_irc(&mut socket, &identity.pass)?;
    write_irc(&mut socket, &identity.nick)?;
    let deadline = Instant::now() + AUTH_TIMEOUT;
    while Instant::now() < deadline {
        match socket.read() {
            Ok(Message::Text(raw)) => {
                for line in raw.split("\r\n") {
                    if line.starts_with("PING ") {
                        let response = line.replacen("PING ", "PONG ", 1);
                        write_irc(&mut socket, &response)?;
                    }
                    if line.contains(" 001 ") { return Ok(socket); }
                    if line.contains("Login authentication failed") ||
                        line.contains("Improperly formatted auth") {
                        return Err("Twitch rejected the captured OAuth authentication".into());
                    }
                }
            }
            Ok(Message::Close(_)) => return Err("Twitch closed the connection during sign-in".into()),
            Err(tungstenite::Error::Io(ref error))
                if matches!(error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) => {}
            Err(_) => return Err("Twitch disconnected during sign-in".into()),
            _ => {}
        }
    }
    Err("Twitch sign-in timed out".into())
}

type IrcSocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;

pub struct TwitchSession {
    state: Arc<Mutex<TwitchSnapshot>>,
    desired: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
}
impl TwitchSession {
    /// Server welcome is mandatory before we close the authentication
    /// browser. Used by handoff and protected-cache cold starts.
    pub fn start_verified(browser_bootstrap: &serde_json::Value) -> Result<Self, String> {
        let identity = IrcIdentity::from_browser(browser_bootstrap)?;
        let socket = connect(&identity)?;
        Self::spawn(identity, Some(socket))
    }
    fn spawn(identity: IrcIdentity, first_socket: Option<IrcSocket>) -> Result<Self, String> {
        let state = Arc::new(Mutex::new(TwitchSnapshot::default()));
        let desired = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_desired = Arc::clone(&desired);
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new().name("moth-native-twitch".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(identity, Arc::clone(&worker_state), worker_desired,
                        Arc::clone(&worker_stop), first_socket);
                }));
                if result.is_err() {
                    if let Ok(mut state) = worker_state.lock() {
                        state.phase = "Failed".into();
                        state.error = Some("Twitch connection stopped unexpectedly. See the controller log.".into());
                        state.joined = 0;
                    }
                    worker_stop.store(true, Ordering::Relaxed);
                }
            })
            .map_err(|_| "Could not start the Twitch connection".to_owned())?;
        Ok(Self { state, desired, stop })
    }
    pub fn snapshot(&self) -> TwitchSnapshot {
        self.state.lock().map(|state| state.clone()).unwrap_or_default()
    }
    pub fn desired_handle(&self) -> Arc<Mutex<Vec<String>>> {
        Arc::clone(&self.desired)
    }
    pub fn stop(&self) { self.stop.store(true, Ordering::Relaxed); }
}
impl Drop for TwitchSession {
    fn drop(&mut self) { self.stop(); }
}

fn run(identity: IrcIdentity, state: Arc<Mutex<TwitchSnapshot>>,
       desired: Arc<Mutex<Vec<String>>>, stop: Arc<AtomicBool>,
       first_socket: Option<IrcSocket>) {
    let mut first_socket = first_socket;
    while !stop.load(Ordering::Relaxed) {
        match first_socket.take().map(Ok).unwrap_or_else(|| connect(&identity)) {
            Ok(mut socket) => {
                crate::logging::info("Native Twitch IRC received authenticated welcome");
                if let Ok(mut s) = state.lock() {
                    s.phase = "Online".into();
                    s.error = None;
                    s.joined = 0;
                    s.last_received = Some(Instant::now());
                }
                let mut joined = BTreeSet::new();
                let mut next_join = Instant::now();
                let mut auth_rejected = false;
                let mut previous_desired: Vec<String> = Vec::new();
                let mut targets: Vec<String> = Vec::new();
                let mut target_set: BTreeSet<String> = BTreeSet::new();
                loop {
                    if stop.load(Ordering::Relaxed) { let _ = socket.close(None); break; }
                    // An unchanged server room list must not allocate a new
                    // vector, normalize names, and rebuild a set on every read.
                    let changed = desired.lock().ok().and_then(|current|
                        if *current != previous_desired {Some(current.clone())}
                        else {None});
                    if let Some(next) = changed {
                        previous_desired = next.clone();
                        targets = normalize_channels(next);
                        target_set = targets.iter().cloned().collect();
                    }
                    // Leave removed channels, but keep every unchanged room
                    // connected; never rebuild the socket on live-list deltas.
                    for previous in joined.difference(&target_set).cloned().collect::<Vec<_>>() {
                        {
                            if write_irc(&mut socket, &format!("PART #{previous}")).is_err() {
                                joined.clear();
                                break;
                            }
                            joined.remove(&previous);
                        }
                    }
                    if Instant::now() >= next_join {
                        if let Some(missing) = targets.iter().find(|c| !joined.contains(*c)) {
                            if write_irc(&mut socket, &format!("JOIN #{missing}")).is_err() {
                                break;
                            }
                            joined.insert(missing.clone());
                            next_join = Instant::now() + JOIN_GAP;
                        }
                    }
                    if let Ok(mut s) = state.lock() {
                        s.desired = targets.len();
                        s.joined = joined.len();
                    }
                    match socket.read() {
                        Ok(Message::Text(raw)) => {
                            if let Ok(mut s) = state.lock() { s.last_received = Some(Instant::now()); }
                            let mut rejected = false;
                            for line in raw.split("\r\n") {
                                if let Some(payload) = line.strip_prefix("PING ") {
                                    if write_irc(&mut socket, &format!("PONG {payload}")).is_err() {
                                        rejected = true; break;
                                    }
                                }
                                if line.contains("Login authentication failed") ||
                                   line.contains("Improperly formatted auth") {
                                    auth_rejected = true;
                                    rejected = true;
                                    break;
                                }
                            }
                            if rejected { break; }
                        }
                        Ok(Message::Ping(data)) => {
                            if socket.send(Message::Pong(data)).is_err() { break; }
                        }
                        Ok(Message::Pong(_)) => {
                            if let Ok(mut s) = state.lock() { s.last_received = Some(Instant::now()); }
                        }
                        Ok(Message::Close(_)) => break,
                        Err(tungstenite::Error::Io(ref error))
                            if matches!(error.kind(),
                                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) => {}
                        Err(_) => break,
                        _ => {}
                    }
                }
                if !stop.load(Ordering::Relaxed) {
                    if let Ok(mut s) = state.lock() {
                        s.phase = if auth_rejected { "NeedsLogin" } else { "Reconnecting" }.into();
                        s.joined = 0;
                        if auth_rejected {
                            s.error = Some("Your Twitch login expired. Sign in again.".into());
                        }
                    }
                    if auth_rejected { break; }
                }
            }
            Err(error) => {
                let auth_expired = error.contains("rejected the captured OAuth");
                if let Ok(mut s) = state.lock() {
                    s.phase = if auth_expired { "NeedsLogin" } else { "Reconnecting" }.into();
                    s.error = Some(error);
                    s.joined = 0;
                }
                // Network failures are handled within this native worker.
                // No browser is opened for transient IRC outages.
                if auth_expired { break; }
            }
        }
        let until = Instant::now() + RETRY_DELAY;
        while !stop.load(Ordering::Relaxed) && Instant::now() < until {
            thread::sleep(Duration::from_millis(200));
        }
    }
    if stop.load(Ordering::Relaxed) {
        if let Ok(mut s) = state.lock() { s.phase = "Stopped".into(); s.joined = 0; }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn filters_invalid_rooms_and_deduplicates() {
        let channels = normalize_channels(vec![
            "a".into(), "#A".into(), "b".into(),
            "x\r\nPART #everything".into(), "not-valid-!".into()
        ]);
        assert_eq!(channels, vec!["a", "b"]);
    }
    #[test]
    fn rejects_unauthenticated_and_unexpected_hosts() {
        let sample = json!({"url":"wss://irc-ws.chat.twitch.tv:443",
            "nick":"NICK myusername", "pass":"PASS oauth:secret",
            "cap":"CAP REQ :twitch.tv/tags twitch.tv/commands"});
        assert!(validate(&sample).is_ok());
        let without_cap = serde_json::json!({
            "url":"wss://irc-ws.chat.twitch.tv:443",
            "pass":"PASS oauth:secret", "nick":"NICK myusername", "cap":""
        });
        assert!(validate(&without_cap).is_ok());
        let native_capture = json!({"url":"wss://irc-ws.chat.twitch.tv:443","nick":"NICK myusername","pass":"PASS oauth:secret","cap":"CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership"});
        assert!(validate(&native_capture).is_ok());
        assert!(validate(&json!({"url":"wss://evil.tld","nick":"NICK user","pass":"PASS oauth:secret"})).is_err());
        assert!(validate(&json!({"url":"wss://irc-ws.chat.twitch.tv:443","nick":"NICK user","pass":"PASS SCHMOOPIIE"})).is_err());
    }
}
