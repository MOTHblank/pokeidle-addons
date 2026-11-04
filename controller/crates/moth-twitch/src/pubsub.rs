use serde_json::{json, Value};
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Error as WsError, Message, WebSocket};

const PUBSUB_URL: &str = "wss://pubsub-edge.twitch.tv/v1";
const RECONNECT_DELAY: Duration = Duration::from_secs(10);
const PING_INTERVAL: Duration = Duration::from_secs(240);
const PONG_DEADLINE: Duration = Duration::from_secs(300);

#[derive(Clone, Debug)]
pub(crate) enum PubSubEvent {
    Connected,
    PointsEarned {
        channel_id: String,
        amount: u64,
        balance: Option<u64>,
        reason: String,
    },
    ClaimAvailable {
        channel_id: String,
        claim_id: String,
    },
    Warning(String),
}

pub(crate) fn spawn(
    oauth_token: String,
    user_id: String,
    stop: Arc<AtomicBool>,
    event_tx: Sender<PubSubEvent>,
    wake: SyncSender<()>,
) {
    let _ = thread::Builder::new()
        .name(format!("moth-twitch-pubsub-{user_id}"))
        .spawn(move || {
            while !stop.load(Ordering::Acquire) {
                if let Err(error) = run_connection(
                    &oauth_token, &user_id, &stop, &event_tx, &wake
                ) {
                    let _ = event_tx.send(PubSubEvent::Warning(error));
                    let _ = wake.try_send(());
                }
                for _ in 0..10 {
                    if stop.load(Ordering::Acquire) { return; }
                    thread::sleep(RECONNECT_DELAY / 10);
                }
            }
        });
}

fn configure_timeout(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>) {
    let timeout = Some(Duration::from_secs(1));
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => {
            let _ = stream.set_read_timeout(timeout);
            let _ = stream.set_write_timeout(timeout);
        }
        MaybeTlsStream::Rustls(stream) => {
            let _ = stream.sock.set_read_timeout(timeout);
            let _ = stream.sock.set_write_timeout(timeout);
        }
        _ => {}
    }
}

fn run_connection(
    oauth_token: &str,
    user_id: &str,
    stop: &AtomicBool,
    event_tx: &Sender<PubSubEvent>,
    wake: &SyncSender<()>,
) -> Result<(), String> {
    let (mut socket, _) = connect(PUBSUB_URL)
        .map_err(|error| format!("Twitch PubSub connection failed: {error}"))?;
    configure_timeout(&mut socket);
    let topic = format!("community-points-user-v1.{user_id}");
    let listen = json!({
        "type": "LISTEN",
        "nonce": format!("moth-{user_id}"),
        "data": {
            "topics": [topic],
            "auth_token": oauth_token,
        }
    });
    socket.send(Message::text(listen.to_string()))
        .map_err(|error| format!("Twitch PubSub subscribe failed: {error}"))?;

    let _ = event_tx.send(PubSubEvent::Connected);
    let _ = wake.try_send(());
    let mut last_pong = Instant::now();
    let mut last_ping = Instant::now();

    while !stop.load(Ordering::Acquire) {
        if last_ping.elapsed() >= PING_INTERVAL {
            socket.send(Message::text(r#"{"type":"PING"}"#))
                .map_err(|error| format!("Twitch PubSub ping failed: {error}"))?;
            last_ping = Instant::now();
        }
        if last_pong.elapsed() >= PONG_DEADLINE {
            return Err("Twitch PubSub heartbeat expired".into());
        }

        match socket.read() {
            Ok(message) => {
                let Ok(text) = message.to_text() else { continue };
                let envelope: Value = match serde_json::from_str(text) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                match envelope.get("type").and_then(Value::as_str)
                    .unwrap_or_default().to_ascii_uppercase().as_str()
                {
                    "PONG" => last_pong = Instant::now(),
                    "RECONNECT" => return Err("Twitch PubSub requested reconnect".into()),
                    "RESPONSE" => {
                        if let Some(error) = envelope.get("error").and_then(Value::as_str)
                            .filter(|error| !error.trim().is_empty())
                        {
                            return Err(format!("Twitch PubSub subscription rejected: {error}"));
                        }
                    }
                    "MESSAGE" => {
                        if let Some(event)=parse_message(&envelope) {
                            let _ = event_tx.send(event);
                            let _ = wake.try_send(());
                        }
                    }
                    _ => {}
                }
            }
            Err(WsError::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(WsError::ConnectionClosed | WsError::AlreadyClosed) =>
                return Err("Twitch PubSub connection closed".into()),
            Err(error) => return Err(format!("Twitch PubSub read failed: {error}")),
        }
    }
    let _ = socket.close(None);
    Ok(())
}

fn number(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| value.as_u64().or_else(|| {
        value.as_f64().filter(|v|v.is_finite() && *v>=0.0).map(|v|v as u64)
    }))
}

fn nested<'a>(value:&'a Value,path:&[&str])->Option<&'a Value> {
    let mut current=value;
    for key in path { current=current.get(*key)?; }
    Some(current)
}

fn channel_id(payload:&Value)->Option<String> {
    for path in [
        &["data","balance","channel_id"][..],
        &["data","claim","channel_id"][..],
        &["data","channel_id"][..],
    ] {
        if let Some(id)=nested(payload,path).and_then(Value::as_str)
            .filter(|id|!id.is_empty()) {
            return Some(id.to_owned());
        }
    }
    None
}

fn parse_message(envelope:&Value)->Option<PubSubEvent> {
    let raw=envelope.get("data")?.get("message")?.as_str()?;
    let payload:Value=serde_json::from_str(raw).ok()?;
    let kind=payload.get("type")?.as_str()?.to_ascii_lowercase();
    let channel_id=channel_id(&payload)?;
    match kind.as_str() {
        "points-earned" => {
            let gain=nested(&payload,&["data","point_gain"])?;
            let reason=gain.get("reason_code").and_then(Value::as_str)
                .unwrap_or_default().to_ascii_uppercase();
            let amount=number(gain.get("total_points")).unwrap_or(0);
            let balance=number(nested(&payload,&["data","balance","balance"]));
            Some(PubSubEvent::PointsEarned {
                channel_id, amount, balance, reason
            })
        }
        "claim-available" => {
            let claim_id=nested(&payload,&["data","claim","id"])
                .and_then(Value::as_str)?.to_owned();
            (!claim_id.is_empty()).then_some(PubSubEvent::ClaimAvailable {
                channel_id, claim_id
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(inner:Value)->Value {
        json!({"type":"MESSAGE","data":{"message":inner.to_string()}})
    }

    #[test]
    fn parses_watch_streak_gain() {
        let event=parse_message(&envelope(json!({
            "type":"points-earned",
            "data":{
                "point_gain":{"reason_code":"WATCH_STREAK","total_points":450},
                "balance":{"channel_id":"123","balance":1450}
            }
        }))).unwrap();
        match event {
            PubSubEvent::PointsEarned{channel_id,amount,balance,reason}=>{
                assert_eq!(channel_id,"123");
                assert_eq!(amount,450);
                assert_eq!(balance,Some(1450));
                assert_eq!(reason,"WATCH_STREAK");
            }
            _=>panic!("unexpected event"),
        }
    }

    #[test]
    fn parses_claim_available() {
        let event=parse_message(&envelope(json!({
            "type":"claim-available",
            "data":{"claim":{"id":"claim-1","channel_id":"456"}}
        }))).unwrap();
        assert!(matches!(event,PubSubEvent::ClaimAvailable{
            channel_id,claim_id
        } if channel_id=="456" && claim_id=="claim-1"));
    }
}
