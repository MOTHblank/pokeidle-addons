//! Experimental low-bandwidth Kick HLS transport (Twitch-style).
//! Polls the lowest-bitrate M3U8 playlist and probes new segments using HTTP
//! HEAD (0 bytes of video payload), falling back to a 1-byte Range GET or
//! playlist-sequence tracking if the CDN rejects HEAD.
//! Selected only when the user enables the experimental Kick lite-mode switch.
use crate::native_kick_hls::{AccountBandwidth, MediaStatus};
use crate::native_kick_pool;
use reqwest::blocking::Client;
use reqwest::{Url, redirect::Policy};
use std::collections::VecDeque;
use std::io::{Read, sink};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MAX_PLAYLIST_BYTES: usize = 512 * 1024;
const MAX_RANGE_PROBE_BYTES: u64 = 64;
const MAX_SEEN: usize = 128;
const RETRY_SECS: u64 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeMode {
    Head,
    RangeByte,
    PlaylistOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeOutcome {
    Accepted { bytes: u64, next_mode: ProbeMode },
    RetryWith(ProbeMode),
    Rejected,
}

fn classify_head_status(status: u16) -> ProbeOutcome {
    match status {
        200 => ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::Head },
        403 | 405 => ProbeOutcome::RetryWith(ProbeMode::RangeByte),
        _ => ProbeOutcome::Rejected,
    }
}

fn classify_range_status(status: u16) -> ProbeOutcome {
    match status {
        206 => ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::RangeByte },
        200 | 403 | 416 => ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::PlaylistOnly },
        _ => ProbeOutcome::Rejected,
    }
}

struct FinishGuard(Arc<AtomicBool>);
impl Drop for FinishGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub struct LiteMediaRunner {
    pub state: Arc<Mutex<MediaStatus>>,
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

impl LiteMediaRunner {
    pub fn start(
        source: String,
        account_stop: Arc<AtomicBool>,
        bandwidth: Arc<AccountBandwidth>,
    ) -> Result<Self, String> {
        let url = media_url(&source)?;
        let state = Arc::new(Mutex::new(MediaStatus::default()));
        let local_stop = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let worker_state = Arc::clone(&state);
        let worker_stop = Arc::clone(&local_stop);
        let worker_finished = Arc::clone(&finished);
        native_kick_pool::submit_hls(Arc::clone(&local_stop), move || {
            run(url, account_stop, worker_stop, worker_finished, worker_state, bandwidth)
        })?;
        Ok(Self { state, stop: local_stop, finished })
    }

    pub fn snapshot(&self) -> MediaStatus {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn active(&self) -> bool {
        self.snapshot().last_segment.is_some_and(|t| t.elapsed() < Duration::from_secs(25))
    }

    pub fn finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

impl Drop for LiteMediaRunner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        native_kick_pool::discard_canceled_hls();
    }
}

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

fn playlist_text(
    client: &Client,
    url: &Url,
    bandwidth: &AccountBandwidth,
) -> Result<String, String> {
    let mut response = client.get(url.clone()).send()
        .map_err(|_| "Kick HLS playlist download failed".to_owned())?
        .error_for_status()
        .map_err(|_| "Kick HLS playlist rejected the request".to_owned())?;
    let mut buffer = Vec::new();
    let result = (&mut response).take(MAX_PLAYLIST_BYTES as u64 + 1)
        .read_to_end(&mut buffer);
    bandwidth.record(buffer.len() as u64);
    result.map_err(|_| "Kick HLS playlist could not be read".to_owned())?;
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
        if line.is_empty() || line.starts_with('#') { continue; }
        result.push((sequence, join_checked(base, line)?));
        sequence = sequence.saturating_add(1);
    }
    Ok((result, Duration::from_secs(seconds / 2)))
}

fn probe_segment(
    client: &Client,
    url: &Url,
    mode: &mut ProbeMode,
    bandwidth: &AccountBandwidth,
) -> Result<u64, String> {
    loop {
        match *mode {
            ProbeMode::Head => {
                let response = client.head(url.clone()).send()
                    .map_err(|_| "Kick HLS segment request failed".to_owned())?;
                match classify_head_status(response.status().as_u16()) {
                    ProbeOutcome::Accepted { bytes, next_mode } => {
                        *mode = next_mode;
                        return Ok(bytes);
                    }
                    ProbeOutcome::RetryWith(next_mode) => {
                        *mode = next_mode;
                    }
                    ProbeOutcome::Rejected => {
                        return Err("Kick HLS segment rejected the request".into());
                    }
                }
            }
            ProbeMode::RangeByte => {
                let mut response = client.get(url.clone())
                    .header(reqwest::header::RANGE, "bytes=0-0")
                    .send()
                    .map_err(|_| "Kick HLS segment request failed".to_owned())?;
                let code = response.status().as_u16();
                match classify_range_status(code) {
                    ProbeOutcome::Accepted { next_mode, .. } => {
                        let bytes = if code == 206 {
                            let amount = std::io::copy(
                                &mut (&mut response).take(MAX_RANGE_PROBE_BYTES),
                                &mut sink(),
                            ).map_err(|_| "Kick HLS segment transfer failed".to_owned())?;
                            bandwidth.record(amount);
                            amount
                        } else {
                            // CDN ignored Range (200) or rejected partial read:
                            // drop without pulling video bytes and fall back to
                            // playlist-sequence liveness.
                            0
                        };
                        *mode = next_mode;
                        return Ok(bytes);
                    }
                    ProbeOutcome::RetryWith(next_mode) => {
                        *mode = next_mode;
                    }
                    ProbeOutcome::Rejected => {
                        return Err("Kick HLS segment rejected the request".into());
                    }
                }
            }
            ProbeMode::PlaylistOnly => {
                return Ok(0);
            }
        }
    }
}

fn pause(stop: &AtomicBool, account_stop: &AtomicBool, delay: Duration) {
    let rounds = (delay.as_millis() / 200).max(1) as usize;
    for _ in 0..rounds {
        if stop.load(Ordering::Relaxed) || account_stop.load(Ordering::Relaxed) { break; }
        thread::sleep(Duration::from_millis(200));
    }
}

fn run(
    master: Url,
    account_stop: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    state: Arc<Mutex<MediaStatus>>,
    bandwidth: Arc<AccountBandwidth>,
) {
    let _finish = FinishGuard(finished);
    let client = match Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(6))
        .redirect(Policy::none())
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ORIGIN,
                reqwest::header::HeaderValue::from_static("https://kick.com"),
            );
            headers
        })
        .build()
    {
        Ok(value) => value,
        Err(_) => return,
    };
    let mut variant: Option<Url> = None;
    let mut known = VecDeque::<(u64, String)>::new();
    let mut probe_mode = ProbeMode::Head;
    let mut failures = 0_u32;

    while !stop.load(Ordering::Relaxed) && !account_stop.load(Ordering::Relaxed) {
        let outcome = (|| -> Result<Duration, String> {
            let chosen = if let Some(url) = &variant {
                url.clone()
            } else {
                let body = playlist_text(&client, &master, &bandwidth)?;
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
            let playlist = playlist_text(&client, &chosen, &bandwidth)?;
            let (entries, delay) = segments(&playlist, &chosen)?;
            if entries.is_empty() {
                return Err("Kick media playlist has no segments".into());
            }
            let first_pass = known.is_empty();
            let selected = if first_pass {
                &entries[entries.len().saturating_sub(2)..]
            } else {
                &entries[..]
            };
            for (sequence, url) in selected {
                if stop.load(Ordering::Relaxed) || account_stop.load(Ordering::Relaxed) { break; }
                let key = (*sequence, url.as_str().to_owned());
                if known.contains(&key) { continue; }
                let bytes = probe_segment(&client, url, &mut probe_mode, &bandwidth)?;
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
            }
            Err(error) => {
                if let Ok(mut status) = state.lock() { status.error = Some(error); }
                failures = failures.saturating_add(1);
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
        let chosen = cheapest_variant(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=3000000\nhi.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=150000\nlow.m3u8\n",
            &base,
        ).unwrap().unwrap();
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
    fn probe_mode_transitions_handle_head_and_range_fallbacks() {
        assert_eq!(
            classify_head_status(200),
            ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::Head }
        );
        assert_eq!(classify_head_status(405), ProbeOutcome::RetryWith(ProbeMode::RangeByte));
        assert_eq!(classify_head_status(403), ProbeOutcome::RetryWith(ProbeMode::RangeByte));
        assert_eq!(classify_head_status(404), ProbeOutcome::Rejected);

        assert_eq!(
            classify_range_status(206),
            ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::RangeByte }
        );
        assert_eq!(
            classify_range_status(200),
            ProbeOutcome::Accepted { bytes: 0, next_mode: ProbeMode::PlaylistOnly }
        );
        assert_eq!(classify_range_status(500), ProbeOutcome::Rejected);
    }
}
