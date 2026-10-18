//! Opt-in Kick login from the Moth-owned Firefox profile.
//! One temporary BiDi session, scoped to a newly launched Kick-only browser.
//! Browser login is not Kick Developer OAuth; the website's own authenticated
//! request carries the token required by our private Kick viewer transport.
use crate::config::{Config, GameProfile};
use crate::{firefox, native_auth};
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpStream};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::{thread, io};
use tungstenite::{client, stream::MaybeTlsStream, Message, WebSocket};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;
const TIMEOUT: Duration = Duration::from_secs(7);
const LOGIN_WINDOW: Duration = Duration::from_secs(360);

// Installed before Kick is navigated, in the actual page realm. Observe only
// first-party request headers; never intercept response bodies or log secrets.
const PRELOAD: &str = r#"() => {
    try {
        if (location.hostname !== 'kick.com' && location.hostname !== 'www.kick.com') return;
        if (window.__mothKickBrowserLogin) return;
        let token = '';
        const valid = value => typeof value === 'string' &&
            /^[0-9]{1,18}\|[A-Za-z0-9._~-]{16,8192}$/.test(value);
        const remember = value => {
            if (typeof value !== 'string') return;
            const found = value.trim().replace(/^Bearer\s+/i, '');
            if (valid(found)) token = found;
        };
        const inspect = headers => {
            try { if (headers) remember(new Headers(headers).get('authorization')); }
            catch (_) {}
        };
        const firstParty = value => {
            try {
                const url = new URL(value, location.href);
                return url.protocol === 'https:' &&
                    (url.hostname === 'kick.com' || url.hostname.endsWith('.kick.com'));
            } catch (_) { return false; }
        };
        Object.defineProperty(window, '__mothKickBrowserLogin', {
            value: Object.freeze({ token() { return token || null; } }),
            configurable: false
        });
        const original = window.fetch;
        if (typeof original === 'function') {
            window.fetch = function(input, options) {
                try {
                    const target = typeof input === 'string' ? input : input.url;
                    if (firstParty(target)) {
                        inspect(options?.headers);
                        if (input instanceof Request) inspect(input.headers);
                    }
                } catch (_) {}
                return Reflect.apply(original, this, arguments);
            };
        }
        const proto = window.XMLHttpRequest?.prototype;
        if (proto) {
            const requests = new WeakMap();
            const open = proto.open;
            const setHeader = proto.setRequestHeader;
            proto.open = function(method, url) {
                requests.set(this, firstParty(url));
                return Reflect.apply(open, this, arguments);
            };
            proto.setRequestHeader = function(name, value) {
                if (requests.get(this) && /^authorization$/i.test(String(name))) {
                    remember(value);
                }
                return Reflect.apply(setHeader, this, arguments);
            };
        }
    } catch (_) {}
}"#;

fn command(socket: &mut Socket, id: u64, method: &str, params: Value) -> Result<Value, String> {
    socket.send(Message::Text(json!({
        "id":id, "method":method, "params":params
    }).to_string().into())).map_err(|_| "Kick Firefox BiDi send failed".to_owned())?;
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        match socket.read() {
            Ok(Message::Text(text)) => {
                let Ok(reply) = serde_json::from_str::<Value>(&text) else { continue };
                if reply.get("id").and_then(Value::as_u64) != Some(id) { continue; }
                if reply.get("error").is_some() {
                    return Err(format!("Kick Firefox BiDi rejected {method}"));
                }
                return Ok(reply.get("result").cloned().unwrap_or(Value::Null));
            },
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(ref error)) if error.kind() == io::ErrorKind::WouldBlock
                || error.kind() == io::ErrorKind::TimedOut => continue,
            Err(_) => return Err("Kick Firefox BiDi disconnected".into()),
            _ => {}
        }
    }
    Err(format!("Kick Firefox BiDi timed out on {method}"))
}

fn connect(config: &Config, cancelled: &AtomicBool) -> Result<Socket, String> {
    let address = SocketAddr::from(([127,0,0,1], config.remote_debug_port));
    let deadline = Instant::now() + Duration::from_secs(18);
    while Instant::now() < deadline && !cancelled.load(Ordering::Relaxed) {
        if let Ok(tcp) = TcpStream::connect_timeout(&address, Duration::from_millis(800)) {
            let _ = tcp.set_read_timeout(Some(Duration::from_millis(800)));
            let _ = tcp.set_write_timeout(Some(TIMEOUT));
            if let Ok((socket, _)) = client(
                format!("ws://127.0.0.1:{}/session", config.remote_debug_port),
                MaybeTlsStream::Plain(tcp))
            { return Ok(socket); }
        }
        thread::sleep(Duration::from_millis(350));
    }
    Err("Could not establish Firefox BiDi for Kick sign-in".into())
}
fn capture(socket: &mut Socket, cancelled: &AtomicBool) -> Result<String, String> {
    command(socket, 1, "session.new", json!({"capabilities":{}}))?;
    command(socket, 2, "script.addPreloadScript",
        json!({"functionDeclaration": PRELOAD}))?;
    let tree = command(socket, 3, "browsingContext.getTree", json!({"maxDepth":1}))?;
    let ctx = tree.get("contexts").and_then(Value::as_array)
        .and_then(|list| list.first())
        .and_then(|value| value.get("context")).and_then(Value::as_str)
        .ok_or_else(|| "Kick sign-in browser has no tab".to_owned())?.to_owned();
    command(socket, 4, "browsingContext.navigate",
        json!({"context":ctx,"url":"https://kick.com/","wait":"none"}))?;
    let deadline = Instant::now() + LOGIN_WINDOW;
    let mut seq = 5u64;
    while Instant::now() < deadline {
        if cancelled.load(Ordering::Relaxed) { return Err("Kick login cancelled".into()); }
        let output = command(socket, seq, "script.evaluate", json!({
            "expression": "JSON.stringify(window.__mothKickBrowserLogin?.token() || null)",
            "target": {"context":ctx}, "awaitPromise":false
        }));
        seq += 1;
        if let Ok(result) = output {
            if let Some(raw) = result.get("result").and_then(|v| v.get("value")).and_then(Value::as_str) {
                if let Ok(Some(token)) = serde_json::from_str::<Option<String>>(raw) {
                    if token.len() <= 8192 && token.starts_with(|ch:char| ch.is_ascii_digit()) {
                        return Ok(token);
                    }
                }
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err("Kick login timed out. Open Kick and sign in, then retry.".into())
}
fn stop_owned_browser(socket: &mut Socket, child: &mut Child) {
    let _ = command(socket, 900_001, "browser.close", json!({}));
    for _ in 0..8 {
        if matches!(child.try_wait(), Ok(Some(_))) { return; }
        thread::sleep(Duration::from_millis(250));
    }
    let _ = child.kill();
    let _ = child.wait();
}
/// Safe, per-account UI state; never contains a credential.
#[derive(Clone, Debug)]
pub enum LoginStatus {
    Idle,
    Running,
    Done,
    Error(String),
}
struct LoginJob {
    status: LoginStatus,
    cancelled: Arc<AtomicBool>,
}
fn jobs() -> &'static Mutex<[LoginJob; 4]> {
    static JOBS: OnceLock<Mutex<[LoginJob; 4]>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(std::array::from_fn(|_| LoginJob {
        status: LoginStatus::Idle,
        cancelled: Arc::new(AtomicBool::new(false)),
    })))
}
pub fn status(profile: GameProfile) -> LoginStatus {
    jobs().lock().ok().map(|guard| guard[profile.index()].status.clone())
        .unwrap_or(LoginStatus::Error("Kick login state unavailable".into()))
}
pub fn cancel(profile: GameProfile) {
    if let Ok(guard) = jobs().lock() {
        guard[profile.index()].cancelled.store(true, Ordering::Relaxed);
    }
}
pub fn start(profile: GameProfile) -> Result<(), String> {
    let cancelled = {
        let mut guard = jobs().lock()
            .map_err(|_| "Kick login state unavailable".to_owned())?;
        let current = &mut guard[profile.index()];
        if matches!(current.status, LoginStatus::Running) {
            return Err("Kick login is already in progress".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        current.status = LoginStatus::Running;
        current.cancelled = Arc::clone(&cancelled);
        cancelled
    };
    let outcome = thread::Builder::new()
        .name(format!("moth-kick-login-{}", profile.name()))
        .spawn(move || {
            let result = login(profile, &cancelled);
            if let Ok(mut guard) = jobs().lock() {
                guard[profile.index()].status = match result {
                    Ok(()) => LoginStatus::Done,
                    Err(error) => LoginStatus::Error(error),
                };
            }
        });
    if let Err(error) = outcome {
        if let Ok(mut guard) = jobs().lock() {
            guard[profile.index()].status = LoginStatus::Idle;
        }
        return Err(format!("Could not start Kick Firefox login: {error}"));
    }
    Ok(())
}

pub fn login(profile: GameProfile, cancelled: &AtomicBool) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    if firefox::find_profile_process(&config)?.is_some() {
        return Err("This Firefox profile is already open. Complete native game handoff first.".into());
    }
    let mut child = firefox::launch_kick_login(&config)?;
    let result = (|| {
        let mut socket = connect(&config, cancelled)?;
        let result = capture(&mut socket, cancelled)
            .and_then(|token| native_auth::save_kick_token(profile, &token));
        stop_owned_browser(&mut socket, &mut child);
        result
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}
