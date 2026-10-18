//! Low-memory HLS media transport. Downloads the smallest advertised
//! rendition, then discards segment bytes without decoding or caching them.
//! A successful download is evidence of *network media transfer*, not proof
//! that Kick credited a view or that somebody watched the content.
use reqwest::blocking::Client;
use reqwest::{Url, redirect::Policy};
use std::collections::VecDeque;
use std::io::{Read, sink};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const MAX_PLAYLIST_BYTES: usize = 512 * 1024;
const MAX_SEGMENT_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SEEN: usize = 128;
const RETRY_SECS: u64 = 5;

#[derive(Default, Clone, Debug)]
pub struct MediaStatus {
    pub bytes: u64,
    pub segments: u64,
    pub bitrate: Option<u64>,
    pub last_segment: Option<Instant>,
    pub error: Option<String>,
}
pub struct MediaRunner {
    pub state: Arc<Mutex<MediaStatus>>,
    stop: Arc<AtomicBool>,
}
impl MediaRunner {
    pub fn start(source: String, account_stop: Arc<AtomicBool>) -> Result<Self, String> {
        let url = media_url(&source)?;
        let state = Arc::new(Mutex::new(MediaStatus::default()));
        let local_stop = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&local_stop);
        thread::Builder::new().name("moth-kick-hls".into())
            .spawn(move || run(url, account_stop, worker_stop, worker_state))
            .map_err(|_| "Could not start Kick media worker".to_owned())?;
        Ok(Self { state, stop: local_stop })
    }
    pub fn snapshot(&self) -> MediaStatus {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }
    pub fn active(&self) -> bool {
        self.snapshot().last_segment.is_some_and(|t| t.elapsed() < Duration::from_secs(25))
    }
}
impl Drop for MediaRunner {
    fn drop(&mut self) { self.stop.store(true, Ordering::Relaxed); }
}

/// Never fetch arbitrary hosts advertised by an untrusted playlist. HLS URLs
/// normally come from Amazon IVS, but may refer to a CDN in the same family.
fn media_url(source: &str) -> Result<Url, String> {
    let url = Url::parse(source).map_err(|_| "Invalid Kick HLS URL".to_owned())?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let trusted = ["live-video.net", "cloudfront.net", "akamaized.net", "kick.com"];
    if url.scheme() != "https" || url.port_or_known_default() != Some(443)
        || !trusted.iter().any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
    {
        return Err("Kick HLS URL host is not trusted".into());
    }
    Ok(url)
}
fn join_checked(base: &Url, relative: &str) -> Result<Url, String> {
    let candidate = base.join(relative)
        .map_err(|_| "Invalid Kick playlist reference".to_owned())?;
    media_url(candidate.as_str())
}
fn playlist_text(client: &Client, url: &Url) -> Result<String, String> {
    let mut response = client.get(url.clone()).send()
        .map_err(|_| "Kick HLS playlist download failed".to_owned())?
        .error_for_status()
        .map_err(|_| "Kick HLS playlist rejected the request".to_owned())?;
    let mut buffer = Vec::new();
    (&mut response).take(MAX_PLAYLIST_BYTES as u64 + 1)
        .read_to_end(&mut buffer)
        .map_err(|_| "Kick HLS playlist could not be read".to_owned())?;
    if buffer.len() > MAX_PLAYLIST_BYTES {
        return Err("Kick HLS playlist is too large".into());
    }
    let text = String::from_utf8(buffer)
        .map_err(|_| "Kick HLS playlist is not valid text".to_owned())?;
    if !text.starts_with("#EXTM3U") {
        return Err("Kick HLS source is not an M3U8 playlist".into());
    }
    Ok(text)
}
fn bitrate(line: &str) -> Option<u64> {
    line.split(',').filter_map(|field| field.split_once('='))
        .find(|(key, _)| key.trim().ends_with("BANDWIDTH"))
        .and_then(|(_, value)| value.trim().parse().ok())
}
/// Select lowest BANDWIDTH of all advertised variant playlists.
fn cheapest_variant(master: &str, base: &Url) -> Result<Option<(Url, u64)>, String> {
    let mut pending = None;
    let mut best: Option<(Url, u64)> = None;
    for line in master.lines().map(str::trim) {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            pending = Some(bitrate(line).unwrap_or(u64::MAX));
        } else if !line.is_empty() && !line.starts_with('#') {
            if let Some(rate) = pending.take() {
                let url = join_checked(base, line)?;
                if best.as_ref().is_none_or(|(_, old)| rate < *old) {
                    best = Some((url, rate));
                }
            }
        }
    }
    Ok(best)
}
fn segments(playlist: &str, base: &Url) -> Result<(Vec<(u64, Url)>, Duration), String> {
    let mut sequence = playlist.lines().find_map(|line| {
        line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:")
            .and_then(|s| s.trim().parse::<u64>().ok())
    }).unwrap_or(0);
    let seconds = playlist.lines().find_map(|line| {
        line.strip_prefix("#EXT-X-TARGETDURATION:")
            .and_then(|s| s.trim().parse::<u64>().ok())
    }).unwrap_or(4).clamp(2, 12);
    let mut result = Vec::new();
    for line in playlist.lines().map(str::trim) {
        // LL-HLS PART fragments are ignored when complete EXTINF segments exist.
        if line.is_empty() || line.starts_with('#') { continue; }
        result.push((sequence, join_checked(base, line)?));
        sequence = sequence.saturating_add(1);
    }
    Ok((result, Duration::from_secs(seconds / 2)))
}
fn pause(stop: &AtomicBool, account_stop: &AtomicBool, delay: Duration) {
    let rounds = (delay.as_millis() / 200).max(1) as usize;
    for _ in 0..rounds {
        if stop.load(Ordering::Relaxed) || account_stop.load(Ordering::Relaxed) { break; }
        thread::sleep(Duration::from_millis(200));
    }
}
fn run(master: Url, account_stop: Arc<AtomicBool>, stop: Arc<AtomicBool>,
       state: Arc<Mutex<MediaStatus>>) {
    let client = match Client::builder()
        .timeout(Duration::from_secs(18))
        .connect_timeout(Duration::from_secs(7))
        .redirect(Policy::none())
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(reqwest::header::ORIGIN,
                reqwest::header::HeaderValue::from_static("https://kick.com"));
            headers
        }).build()
    {
        Ok(value) => value,
        Err(_) => return,
    };
    let mut variant: Option<Url> = None;
    let mut known = VecDeque::<(u64, String)>::new();
    let mut failures = 0_u32;
    while !stop.load(Ordering::Relaxed) && !account_stop.load(Ordering::Relaxed) {
        let outcome = (|| -> Result<Duration, String> {
            let chosen = if let Some(url) = &variant {
                url.clone()
            } else {
                let body = playlist_text(&client, &master)?;
                if let Some((url, rate)) = cheapest_variant(&body, &master)? {
                    if let Ok(mut status) = state.lock() {
                        status.bitrate = (rate != u64::MAX).then_some(rate);
                    }
                    variant = Some(url.clone());
                    url
                } else {
                    variant = Some(master.clone());
                    master.clone()
                }
            };
            let playlist = playlist_text(&client, &chosen)?;
            let (entries, delay) = segments(&playlist, &chosen)?;
            if entries.is_empty() { return Err("Kick media playlist has no segments".into()); }
            let first_pass = known.is_empty();
            // Join near the live edge rather than downloading a large backlog.
            let selected = if first_pass {
                &entries[entries.len().saturating_sub(2)..]
            } else {
                &entries[..]
            };
            for (sequence, url) in selected {
                if stop.load(Ordering::Relaxed) || account_stop.load(Ordering::Relaxed) { break; }
                let key = (*sequence, url.as_str().to_owned());
                if known.contains(&key) { continue; }
                let mut response = client.get(url.clone()).send()
                    .map_err(|_| "Kick HLS segment request failed".to_owned())?
                    .error_for_status()
                    .map_err(|_| "Kick HLS segment rejected the request".to_owned())?;
                // std::io::sink() discards the bytes; it does not cache video.
                let bytes = std::io::copy(&mut (&mut response).take(MAX_SEGMENT_BYTES), &mut sink())
                    .map_err(|_| "Kick HLS segment transfer failed".to_owned())?;
                known.push_back(key);
                while known.len() > MAX_SEEN { known.pop_front(); }
                if let Ok(mut status) = state.lock() {
                    status.bytes = status.bytes.saturating_add(bytes);
                    status.segments = status.segments.saturating_add(1);
                    status.last_segment = Some(Instant::now());
                    status.error = None;
                }
            }
            Ok(delay)
        })();
        match outcome {
            Ok(delay) => {
                failures = 0;
                pause(&stop, &account_stop, delay);
            },
            Err(error) => {
                if let Ok(mut status) = state.lock() { status.error = Some(error); }
                failures = failures.saturating_add(1);
                // Expired signed manifest URLs are renewed by the channel worker.
                if failures >= 4 { break; }
                variant = None;
                pause(&stop, &account_stop, Duration::from_secs(RETRY_SECS));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lowest_bitrate_is_selected() {
        let base = media_url("https://edge.us-west-2.playback.live-video.net/live/index.m3u8").unwrap();
        let chosen = cheapest_variant("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=3000000\nhi.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=150000\nlow.m3u8\n", &base).unwrap().unwrap();
        assert!(chosen.0.path().ends_with("/live/low.m3u8"));
        assert_eq!(chosen.1, 150000);
    }
    #[test]
    fn reject_untrusted_playlist_redirects() {
        assert!(media_url("http://localhost/a.m3u8").is_err());
        assert!(media_url("https://evil.com/a.m3u8").is_err());
        let base = media_url("https://edge.playback.live-video.net/live/low.m3u8").unwrap();
        assert!(join_checked(&base, "http://127.0.0.1:8080/x").is_err());
    }
    #[test]
    fn playlist_sequence_is_not_confused_with_tags() {
        let base = media_url("https://edge.playback.live-video.net/live/low.m3u8").unwrap();
        let (parts, delay) = segments("#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:21\n#EXTINF:6.0,\na.ts\n#EXTINF:6.0,\nb.ts\n", &base).unwrap();
        assert_eq!(parts.len(),2);
        assert_eq!(parts[0].0,21);
        assert_eq!(parts[1].0,22);
        assert_eq!(delay,Duration::from_secs(3));
    }
}