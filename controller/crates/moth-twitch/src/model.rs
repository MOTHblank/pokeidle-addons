use crate::client::decode_reference_credential;
use std::time::{Duration, Instant};

pub const MAX_CHANNELS: usize = 32;

#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: Option<String>,
    pub oauth_token: String,
    pub(crate) device_id: Option<String>,
}

impl Credentials {
    fn parse_credential(value:String)->Result<(String,Option<String>),String> {
        let (token,device_id)=decode_reference_credential(&value)?;
        if token.is_empty() || token.len() > 2048
            || token.bytes().any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err("Invalid Twitch OAuth token".into());
        }
        Ok((token.to_owned(),device_id.map(str::to_owned)))
    }

    pub fn new(username: impl Into<String>, credential: impl Into<String>) -> Result<Self, String> {
        let username = normalize_login(&username.into())
            .ok_or_else(|| "Invalid Twitch username".to_owned())?;
        let (oauth_token,device_id)=Self::parse_credential(credential.into())?;
        Ok(Self { username: Some(username), oauth_token, device_id })
    }

    pub fn token_only(credential: impl Into<String>) -> Result<Self, String> {
        let (oauth_token,device_id)=Self::parse_credential(credential.into())?;
        Ok(Self { username: None, oauth_token, device_id })
    }
}

#[derive(Clone, Debug)]
pub struct MinerConfig {
    pub poll_interval: Duration,
    pub claim_bonuses: bool,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(60),
            claim_bonuses: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinerPhase {
    Connecting,
    Online,
    Reconnecting,
    NeedsLogin,
    Failed,
    Stopped,
}

impl MinerPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connecting => "Connecting",
            Self::Online => "Online",
            Self::Reconnecting => "Reconnecting",
            Self::NeedsLogin => "NeedsLogin",
            Self::Failed => "Failed",
            Self::Stopped => "Stopped",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StreamerState {
    pub login: String,
    pub channel_id: Option<String>,
    pub points: Option<u64>,
    /// Twitch WATCH/WATCH_STREAK points attributed to Moth watch activity
    /// for this channel during the current miner session.
    pub points_gained: u64,
    /// A successful telemetry HTTP acknowledgement, not confirmed reward credit.
    pub last_watch_signal: Option<Instant>,
    /// Last time the HLS media playlist and listed segments were successfully touched.
    pub last_media_activity: Option<Instant>,
    /// Confirmed by Twitch point-gain events (WATCH_STREAK / WATCH fallback)
    /// or by a RewardList milestone belonging to the current broadcast.
    /// None = not yet verified.
    pub streak_confirmed: Option<bool>,
    pub streak_length: Option<u32>,
    pub streak_checked: Option<Instant>,
    pub streak_watch_attempts: u32,
    /// The reference-style 7/20 minute priority window elapsed without
    /// Twitch confirming this broadcast's streak.
    pub streak_priority_exhausted: bool,
    pub live: bool,
    /// This channel occupies one of the miner's two persistent watch slots.
    pub watching: bool,
    pub last_update: Option<Instant>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MinerSnapshot {
    pub phase: MinerPhase,
    pub desired: usize,
    pub active: usize,
    pub watch_signals: u64,
    /// Confirmed Twitch WATCH/WATCH_STREAK points attributable to Moth.
    pub points_gained: u64,
    pub bonuses_claimed: u64,
    pub reference_client: bool,
    pub pubsub_connected: bool,
    pub pubsub_error: Option<String>,
    pub last_received: Option<Instant>,
    pub error: Option<String>,
    pub streamers: Vec<StreamerState>,
}

impl Default for MinerSnapshot {
    fn default() -> Self {
        Self {
            phase: MinerPhase::Connecting,
            desired: 0,
            active: 0,
            watch_signals: 0,
            points_gained: 0,
            bonuses_claimed: 0,
            reference_client: false,
            pubsub_connected: false,
            pubsub_error: None,
            last_received: None,
            error: None,
            streamers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MinerEvent {
    Connected { login: String, reference_client: bool },
    Stopped,
    RosterUpdated { channels: usize },
    PubSubConnected,
    StreamOnline { channel: String, balance: Option<u64> },
    StreamOffline { channel: String },
    WatchSlotStarted { channel: String, slot: usize, active: usize },
    WatchActive { channel: String },
    WatchSlotStopped { channel: String, reason: String, active: usize },
    PointsGained { channel: String, amount: u64, balance: u64 },
    BonusClaimed { channel: String },
    StreakConfirmed { channel: String, reason: String },
    Warning(String),
}

pub fn normalize_login(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('#').to_ascii_lowercase();
    if value.is_empty() || value.len() > 25 {
        return None;
    }
    value.bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        .then_some(value)
}

pub(crate) fn normalize_channels(channels: Vec<String>) -> Vec<String> {
    let mut channels: Vec<_> = channels.into_iter()
        .filter_map(|channel| normalize_login(&channel))
        .collect();
    channels.sort_unstable();
    channels.dedup();
    channels.truncate(MAX_CHANNELS);
    channels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_bounds_channel_lists() {
        let channels = normalize_channels(vec![
            "#Channel_A".into(), "channel_a".into(), "bad-name".into(), "two2".into()
        ]);
        assert_eq!(channels, vec!["channel_a", "two2"]);
    }

    #[test]
    fn credentials_reject_whitespace_tokens() {
        assert!(Credentials::new("valid_name", "token").is_ok());
        assert!(Credentials::token_only("token").is_ok());
        assert!(Credentials::new("valid_name", "bad token").is_err());
        assert!(Credentials::new("Bad-Name", "token").is_err());
    }

    #[test]
    fn default_poll_interval_is_at_least_one_minute() {
        assert!(MinerConfig::default().poll_interval >= Duration::from_secs(60));
    }
}
