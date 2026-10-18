//! Kick's website viewer session protocol (separate from Twitch IRC and
//! Kick's public Developer API). This is an undocumented upstream interface;
//! any endpoint can change without notice. Credentials remain memory-only.
use crate::native_kick_hls::{MediaRunner, MediaStatus};
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tungstenite::{client_tls, client::IntoClientRequest, http::header, Message};

const KICK_WS_HOST: &str = "websockets.kick.com";
const TOKEN_ENDPOINT: &str = "https://websockets.kick.com/viewer/v1/token";
const SESSION_SECONDS: u64 = 240;
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const HANDSHAKE_GAP: Duration = Duration::from_secs(30);
const EVENT_GAP: Duration = Duration::from_secs(12);
const POINTS_GAP: Duration = Duration::from_secs(120);

#[derive(Clone, Debug)]
pub struct ChannelMeta {
    pub channel_id: u64,
    pub user_id: u64,
    pub stream_id: u64,
    pub playback_url: String,
    pub title: String,
    pub viewers: Option<u64>,
    pub is_live: bool,
}
#[derive(Clone, Debug, Default)]
pub struct WatchTelemetry {
    pub ws_connected: bool,
    pub handshake_received: bool,
    pub events_sent: u64,
    pub media: MediaStatus,
    pub points: Option<u64>,
    pub points_gained: u64,
    pub last_points: Option<Instant>,
    pub error: Option<String>,
}
pub fn new_client(token: &str) -> Result<Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::ORIGIN,
        reqwest::header::HeaderValue::from_static("https://kick.com"));
    headers.insert(reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert(reqwest::header::REFERER,
        reqwest::header::HeaderValue::from_static("https://kick.com/"));
    headers.insert(reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:130.0) Gecko/20100101 Firefox/130.0"
        ));
    let authorization = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| "Invalid Kick account credential".to_owned())?;
    headers.insert(reqwest::header::AUTHORIZATION, authorization);
    Client::builder().timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(6))
        .default_headers(headers)
        .build().map_err(|_| "Could not initialize Kick account HTTP client".to_owned())
}
fn get_value(client: &Client, url: &str) -> Result<Value, String> {
    let result = client.get(url).send()
        .map_err(|_| "Could not connect to Kick account API".to_owned())?;
    match result.status().as_u16() {
        200 => result.json().map_err(|_| "Kick API returned an invalid response".to_owned()),
        401 | 403 => Err("Kick account authorization expired or denied (401/403)".to_owned()),
        404 => Err("Kick channel/endpoint not found (404)".to_owned()),
        429 => Err("Kick API rate-limited this account (429)".to_owned()),
        code => Err(format!("Kick account API failed (HTTP {code})")),
    }
}
pub fn channel_meta(client: &Client, slug: &str) -> Result<ChannelMeta, String> {
    let data = get_value(client, &format!("https://kick.com/api/v2/channels/{slug}"))?;
    parse_meta(&data)
}
pub fn parse_meta(value: &Value) -> Result<ChannelMeta, String> {
    let data = value.get("data").filter(|v| v.is_object()).unwrap_or(value);
    let channel_id = data.get("id").and_then(Value::as_u64)
        .ok_or_else(|| "Kick channel ID unavailable".to_owned())?;
    let user_id = data.get("user_id").and_then(Value::as_u64)
        .or_else(|| data.pointer("/user/id").and_then(Value::as_u64))
        .ok_or_else(|| "Kick channel user ID unavailable".to_owned())?;
    let stream = data.get("livestream").filter(|v| v.is_object());
    let stream_id = stream.and_then(|v| v.get("id")).and_then(Value::as_u64).unwrap_or(0);
    // Kick's v2 channel endpoint may return a populated livestream object
    // without an is_live field. The existence of a current stream ID means
    // live unless Kick explicitly says false; absence is not false.
    let is_live = stream_id > 0 && stream.and_then(|v| v.get("is_live"))
        .and_then(Value::as_bool).unwrap_or(true);
    let playback_url = data.get("playback_url").and_then(Value::as_str)
        .unwrap_or_default().to_owned();
    Ok(ChannelMeta {
        channel_id, user_id, stream_id, playback_url,
        title: stream.and_then(|v| v.get("session_title")).and_then(Value::as_str)
            .unwrap_or_default().chars().take(140).collect(),
        viewers: stream.and_then(|v| v.get("viewer_count")).and_then(Value::as_u64),
        is_live,
    })
}
fn parse_points(value: &Value) -> Option<u64> {
    ["/data/points", "/points", "/data/user/points", "/user/points"]
        .into_iter()
        .find_map(|key| value.pointer(key).and_then(Value::as_u64))
}
pub fn point_balance(client: &Client, slug: &str) -> Result<Option<u64>, String> {
    let url = format!("https://kick.com/api/v2/channels/{slug}/points");
    let response = client.get(&url).send()
        .map_err(|_| "Could not fetch Kick point balance".to_owned())?;
    let status = response.status().as_u16();
    if status == 404 {
        // The channel profile fallback in some miners exposes the creator's
        // user object, not a proven balance for our authenticated viewer.
        // Never mistake those values for earned points.
        return Ok(None);
    }
    match status {
        200 => {
            let value: Value = response.json()
                .map_err(|_| "Invalid Kick points response".to_owned())?;
            Ok(parse_points(&value))
        },
        401 | 403 => Err("Kick points endpoint denied authorization (401/403)".to_owned()),
        429 => Err("Kick points request rate-limited (429)".to_owned()),
        code => Err(format!("Kick points request failed (HTTP {code})")),
    }
}
pub fn observe_points(status: &mut WatchTelemetry, balance: Option<u64>) {
    if let Some(current) = balance {
        if let Some(previous) = status.points {
            // Balances can decrease when points are redeemed. Count only
            // independently observed increases; never invent rewards.
            status.points_gained = status.points_gained.saturating_add(
                current.saturating_sub(previous));
        }
        status.points = Some(current);
        status.last_points = Some(Instant::now());
    }
}
fn viewer_token(client: &Client, meta: &ChannelMeta) -> Result<String, String> {
    let result = client.get(TOKEN_ENDPOINT)
        .header("X-Chatroom", meta.channel_id.to_string())
        .header("X-User-Id", meta.user_id.to_string())
        .send().map_err(|_| "Could not request Kick viewer token".to_owned())?;
    match result.status().as_u16() {
        200 => {},
        401 | 403 => return Err("Kick viewer token authorization denied (401/403)".into()),
        429 => return Err("Kick viewer token request rate-limited (429)".into()),
        code => return Err(format!("Kick viewer-token request failed (HTTP {code})")),
    }
    let value: Value = result.json()
        .map_err(|_| "Kick viewer token response invalid".to_owned())?;
    let token = ["/data/token", "/data/websocket_token", "/token", "/websocket_token"]
        .into_iter().find_map(|key| value.pointer(key).and_then(Value::as_str))
        .filter(|v| !v.is_empty() && v.len() <= 8192)
        .ok_or_else(|| "Kick viewer token missing".to_owned())?;
    Ok(token.to_owned())
}
type KickSocket = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>;
fn ws_connect(token: &str) -> Result<KickSocket, String> {
    crate::native_tls::ensure()?;
    let mut url = reqwest::Url::parse("wss://websockets.kick.com/viewer/v1/connect")
        .map_err(|_| "Invalid Kick WebSocket URL".to_owned())?;
    url.query_pairs_mut().append_pair("token", token);
    let mut socket = None;
    for addr in (KICK_WS_HOST, 443).to_socket_addrs()
        .map_err(|_| "Could not resolve Kick WebSocket".to_owned())?
    {
        if let Ok(tcp) = TcpStream::connect_timeout(&addr, Duration::from_secs(7)) {
            socket = Some(tcp);
            break;
        }
    }
    let tcp = socket.ok_or_else(|| "Could not connect to Kick WebSocket".to_owned())?;
    tcp.set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "Could not set Kick WebSocket timeout".to_owned())?;
    tcp.set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "Could not set Kick WebSocket write timeout".to_owned())?;
    let mut request = url.as_str().into_client_request()
        .map_err(|_| "Invalid Kick WebSocket handshake".to_owned())?;
    request.headers_mut().insert(header::ORIGIN,
        header::HeaderValue::from_static("https://kick.com"));
    let (socket, _) = client_tls(request, tcp)
        .map_err(|_| "Kick viewer WebSocket TLS handshake failed".to_owned())?;
    Ok(socket)
}
fn ws_send(socket: &mut KickSocket, value: Value) -> Result<(), String> {
    socket.send(Message::Text(value.to_string().into()))
        .map_err(|_| "Kick WebSocket could not send message".to_owned())
}
fn handshake(meta: &ChannelMeta) -> Value {
    json!({"type":"channel_handshake","data":{"message":{"channelId":meta.channel_id}}})
}
fn heartbeat() -> Value { json!({"type":"ping"}) }
fn watch_event(meta: &ChannelMeta) -> Value {
    json!({"type":"user_event","data":{"message":{
        "name":"tracking.user.watch.livestream",
        "channel_id":meta.channel_id,
        "livestream_id":meta.stream_id
    }}})
}
/// Session begins after the channel is verified live; it sends tracking only
/// while current HLS segments are arriving. "Connected" alone never counts
/// as successful point earning.
pub fn run_viewer(client: &Client, slug: &str, meta: &ChannelMeta,
                  status: &Arc<Mutex<WatchTelemetry>>,
                  stop: Arc<AtomicBool>) -> Result<(), String> {
    if !meta.is_live || meta.stream_id == 0 {
        return Err("Kick channel is not live".into());
    }
    let token = viewer_token(client, meta)?;
    let mut socket = ws_connect(&token)?;
    ws_send(&mut socket, handshake(meta))?;
    ws_send(&mut socket, heartbeat())?;
    let media = MediaRunner::start(meta.playback_url.clone(), Arc::clone(&stop))?;
    let started = Instant::now();
    let mut last_handshake = started;
    let mut last_event = started;
    let mut last_points = started - POINTS_GAP;
    if let Ok(mut state) = status.lock() {
        state.ws_connected = true;
        state.error = None;
    }
    while !stop.load(Ordering::Relaxed)
        && started.elapsed() < Duration::from_secs(SESSION_SECONDS)
    {
        if last_handshake.elapsed() >= HANDSHAKE_GAP {
            ws_send(&mut socket, handshake(meta))?;
            ws_send(&mut socket, heartbeat())?;
            last_handshake = Instant::now();
        }
        let media_snapshot = media.snapshot();
        if let Ok(mut state) = status.lock() {
            state.media = media_snapshot;
        }
        if last_event.elapsed() >= EVENT_GAP && media.active() {
            ws_send(&mut socket, watch_event(meta))?;
            if let Ok(mut state) = status.lock() {
                state.events_sent = state.events_sent.saturating_add(1);
            }
            last_event = Instant::now();
        }
        if last_points.elapsed() >= POINTS_GAP {
            match point_balance(client, slug) {
                Ok(balance) => {
                    if let Ok(mut state) = status.lock() {
                        observe_points(&mut state, balance);
                    }
                },
                Err(error) => {
                    if let Ok(mut state) = status.lock() {
                        state.error = Some(error);
                    }
                }
            }
            last_points = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(message)) => {
                let text = message.as_str();
                if text == "ping" { ws_send(&mut socket, json!({"type":"pong"}))?; }
                if let Ok(value) = serde_json::from_str::<Value>(text) {
                    match value.get("type").and_then(Value::as_str) {
                        Some("channel_handshake") => {
                            if let Ok(mut state) = status.lock() {
                                state.handshake_received = true;
                            }
                        },
                        Some("ping") => { ws_send(&mut socket, json!({"type":"pong"}))?; },
                        Some("error") => return Err("Kick viewer WebSocket returned an error".into()),
                        _ => {},
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                socket.send(Message::Pong(payload))
                    .map_err(|_| "Kick WebSocket pong failed".to_owned())?;
            },
            Ok(Message::Close(_)) => return Err("Kick viewer session closed".into()),
            Err(tungstenite::Error::Io(ref error))
                if matches!(error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {},
            Err(_) => return Err("Kick viewer WebSocket disconnected".into()),
            _ => {},
        }
        // Stop unsuccessful transport instead of reporting false active status.
        if started.elapsed() > Duration::from_secs(65)
            && !media.active() && media.snapshot().error.is_some()
        {
            return Err("Kick HLS transport stalled".into());
        }
    }
    if let Ok(mut state) = status.lock() {
        state.media = media.snapshot();
        state.ws_connected = false;
    }
    let _ = socket.close(None);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_live_stream_and_media_url() {
        let source=json!({"id":123,"user_id":456,"playback_url":"https://edge.playback.live-video.net/x.m3u8",
            "livestream":{"id":789,"is_live":true,"viewer_count":42,"session_title":"Hello"}});
        let meta=parse_meta(&source).unwrap();
        assert!(meta.is_live);
        assert_eq!(meta.stream_id,789);
        assert_eq!(meta.viewers,Some(42));
        assert_eq!(handshake(&meta)["data"]["message"]["channelId"],123);
        assert_eq!(watch_event(&meta)["data"]["message"]["livestream_id"],789);
    }
    #[test]
    fn detects_live_stream_when_kick_omits_is_live() {
        let metadata = json!({
            "id": 120, "user_id": 300, "playback_url": "https://example.com/live.m3u8",
            "livestream": {"id": 456, "session_title": "streaming", "viewer_count": 20}
        });
        let meta = parse_meta(&metadata).unwrap();
        assert!(meta.is_live);
        assert_eq!(meta.stream_id, 456);
        let explicit_offline = json!({"id":120, "user_id":300,
            "livestream":{"id":456,"is_live":false}});
        assert!(!parse_meta(&explicit_offline).unwrap().is_live);
        let without_stream = json!({"id":120, "user_id":300, "livestream":null});
        assert!(!parse_meta(&without_stream).unwrap().is_live);
    }
    #[test]
    fn points_delta_requires_numeric_confirmed_balance() {
        let mut status=WatchTelemetry::default();
        observe_points(&mut status,parse_points(&json!({"data":{"points":100}})));
        observe_points(&mut status,None);
        observe_points(&mut status,Some(125));
        observe_points(&mut status,Some(10));
        assert_eq!(status.points_gained,25);
        assert_eq!(status.points,Some(10));
    }
}