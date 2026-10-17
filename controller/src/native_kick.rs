//! Read-only Kick livestream monitoring via the documented Kick Developer API.
//! This worker never claims a viewer, reports a watch event, opens an HLS
//! connection, sends chat, or interprets an API response as earned rewards.
//!
//! API tokens are supplied explicitly at runtime. They are never persisted,
//! logged, included in URLs, or copied to the game/Twitch credential cache.
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const API_URL: &str = "https://api.kick.com/public/v1/channels";
const POLL_INTERVAL: Duration = Duration::from_secs(90);
const MAX_CHANNELS: usize = 12;
const MAX_TOKEN_LEN: usize = 8192;

#[derive(Clone, Debug)]
pub struct ChannelStatus {
    pub slug: String,
    pub live: Option<bool>,
    pub title: String,
    pub viewers: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct KickSnapshot {
    pub phase: String,
    pub channels: Vec<ChannelStatus>,
    pub last_checked: Option<Instant>,
    pub error: Option<String>,
}
impl Default for KickSnapshot {
    fn default() -> Self {
        Self {
            phase: "Connecting".to_owned(),
            channels: Vec::new(),
            last_checked: None,
            error: None,
        }
    }
}

pub struct KickMonitor {
    snapshot: Arc<Mutex<KickSnapshot>>,
    stop: Arc<AtomicBool>,
}
impl KickMonitor {
    pub fn start(token: String, input: &str) -> Result<Self, String> {
        let slugs = normalize_channels(input)?;
        let token = token.trim().to_owned();
        if token.is_empty() || token.len() > MAX_TOKEN_LEN
            || token.bytes().any(|c| c.is_ascii_control())
        {
            return Err("A valid Kick Developer API bearer token is required".into());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = Arc::new(Mutex::new(KickSnapshot {
            channels: slugs.iter().map(|slug| ChannelStatus {
                slug: slug.clone(),
                live: None,
                title: String::new(),
                viewers: None,
            }).collect(),
            ..KickSnapshot::default()
        }));
        let worker_state = Arc::clone(&snapshot);
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new()
            .name("moth-kick-monitor".into())
            .spawn(move || run(token, slugs, worker_state, worker_stop))
            .map_err(|_| "Could not start Kick monitoring worker".to_owned())?;
        Ok(Self { snapshot, stop })
    }

    pub fn snapshot(&self) -> KickSnapshot {
        self.snapshot.lock().map(|state| state.clone()).unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for KickMonitor {
    fn drop(&mut self) { self.stop(); }
}

/// Accept comma-, whitespace- or newline-separated slugs/URLs.
pub fn normalize_channels(input: &str) -> Result<Vec<String>, String> {
    let mut channels = BTreeSet::new();
    for part in input.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let part = part.trim().trim_end_matches('/');
        if part.is_empty() { continue; }
        let slug = part
            .strip_prefix("https://kick.com/")
            .or_else(|| part.strip_prefix("https://www.kick.com/"))
            .unwrap_or(part)
            .to_ascii_lowercase();
        if slug.is_empty() || slug.len() > 25 || !slug.bytes().all(|b|
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        {
            return Err("Invalid Kick channel: use channel names or kick.com URLs".into());
        }
        channels.insert(slug);
        if channels.len() > MAX_CHANNELS {
            return Err("Maximum 12 Kick channels per account".into());
        }
    }
    if channels.is_empty() {
        return Err("Add at least one Kick channel".into());
    }
    Ok(channels.into_iter().collect())
}

fn read_channels(client: &reqwest::blocking::Client, token: &str,
                 slugs: &[String]) -> Result<Vec<ChannelStatus>, String> {
    let mut request = client.get(API_URL).bearer_auth(token);
    for slug in slugs {
        request = request.query(&[("slug", slug)]);
    }
    let response = request.send().map_err(|_| "Kick API connection failed".to_owned())?;
    let code = response.status().as_u16();
    match code {
        200 => {},
        401 | 403 => return Err("Kick API authentication rejected (401/403)".into()),
        429 => return Err("Kick API rate limit reached (429)".into()),
        _ => return Err(format!("Kick API request failed (HTTP {code})")),
    }
    let value: Value = response.json()
        .map_err(|_| "Kick API returned invalid JSON".to_owned())?;
    parse_channels(&value, slugs)
}

fn parse_channels(body: &Value, slugs: &[String]) -> Result<Vec<ChannelStatus>, String> {
    let rows = body.get("data").and_then(Value::as_array)
        .ok_or_else(|| "Kick API response has no channel list".to_owned())?;
    Ok(slugs.iter().map(|slug| {
        let row = rows.iter().find(|row| {
            row.get("slug").and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(slug))
        });
        let stream = row.and_then(|row| row.get("stream"));
        ChannelStatus {
            slug: slug.clone(),
            live: row.map(|_| stream
                .and_then(|s| s.get("is_live"))
                .and_then(Value::as_bool)
                .unwrap_or(false)),
            title: row.and_then(|row| row.get("stream_title"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .chars().take(140).collect(),
            viewers: stream.and_then(|s| s.get("viewer_count")).and_then(Value::as_u64),
        }
    }).collect())
}

fn run(token: String, slugs: Vec<String>, snapshot: Arc<Mutex<KickSnapshot>>,
       stop: Arc<AtomicBool>) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(6))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            if let Ok(mut s) = snapshot.lock() {
                s.phase = "Error".into();
                s.error = Some("Could not initialize Kick HTTP client".into());
            }
            return;
        }
    };
    let mut consecutive_errors: u32 = 0;
    while !stop.load(Ordering::Relaxed) {
        let outcome = read_channels(&client, &token, &slugs);
        let needs_login = matches!(outcome.as_ref(), Err(error) if error.contains("401/403"));
        if let Ok(mut s) = snapshot.lock() {
            s.last_checked = Some(Instant::now());
            match outcome {
                Ok(channels) => {
                    s.channels = channels;
                    s.phase = "Online".into();
                    s.error = None;
                    consecutive_errors = 0;
                }
                Err(error) => {
                    s.phase = if needs_login { "NeedsLogin" } else { "Reconnecting" }.into();
                    s.error = Some(error);
                    // Preserve the most recent channel state, but mark it stale.
                    for channel in &mut s.channels {
                        channel.live = None;
                        channel.viewers = None;
                    }
                    consecutive_errors = consecutive_errors.saturating_add(1);
                }
            }
        }
        if needs_login { break; }
        // Back off gently on transient errors; never spin or issue rapid retries.
        let seconds = if consecutive_errors == 0 { POLL_INTERVAL.as_secs() }
            else { (15_u64.saturating_mul(2_u64.pow(consecutive_errors.min(3)))).min(180) };
        for _ in 0..seconds {
            if stop.load(Ordering::Relaxed) { break; }
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn channel_input_normalizes_and_deduplicates() {
        assert_eq!(normalize_channels("https://kick.com/SomeName, somename\nOther_2").unwrap(),
                   vec!["other_2", "somename"]);
        assert!(normalize_channels("https://evil.example/channel").is_err());
        assert!(normalize_channels("../../secret").is_err());
        assert!(normalize_channels("").is_err());
        assert!(normalize_channels(&vec!["a"; 13].join(",")).is_ok()); // duplicate only
        assert!(normalize_channels(&(0..13).map(|n| format!("ch{n}")).collect::<Vec<_>>().join(",")).is_err());
    }
    #[test]
    fn documented_channel_shape_and_unknown_slug() {
        let slugs = vec!["live".to_owned(), "offline".to_owned(), "unknown".to_owned()];
        let data = json!({"data": [
            {"slug":"live","stream":{"is_live":true,"viewer_count":99},"stream_title":"Hi"},
            {"slug":"offline","stream":{"is_live":false}}
        ]});
        let result = parse_channels(&data, &slugs).unwrap();
        assert_eq!(result[0].live, Some(true));
        assert_eq!(result[0].viewers, Some(99));
        assert_eq!(result[1].live, Some(false));
        assert_eq!(result[2].live, None);
        assert!(parse_channels(&json!({}), &slugs).is_err());
    }
}
