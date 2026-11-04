use std::time::{Duration, Instant};

pub const MAX_CHANNELS: usize = 32;

#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: Option<String>,
    pub oauth_token: String,
}

impl Credentials {
    pub fn new(username: impl Into<String>, oauth_token: impl Into<String>) -> Result<Self, String> {
        let username = normalize_login(&username.into())
            .ok_or_else(|| "Invalid Twitch username".to_owned())?;
        let oauth_token = oauth_token.into().trim().to_owned();
        if oauth_token.is_empty() || oauth_token.len() > 2048
            || oauth_token.bytes().any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err("Invalid Twitch OAuth token".into());
        }
        Ok(Self { username: Some(username), oauth_token })
    }

    pub fn token_only(oauth_token: impl Into<String>) -> Result<Self, String> {
        let oauth_token = oauth_token.into().trim().to_owned();
        if oauth_token.is_empty() || oauth_token.len() > 2048
            || oauth_token.bytes().any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err("Invalid Twitch OAuth token".into());
        }
        Ok(Self { username: None, oauth_token })
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
            poll_interval: Duration::from_secs(30),
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
    /// A successful telemetry HTTP acknowledgement, not confirmed reward credit.
    pub last_watch_signal: Option<Instant>,
    /// Confirmed only when RewardList's achievement timestamp belongs to
    /// the current live broadcast. None = not yet verified.
    pub streak_confirmed: Option<bool>,
    pub streak_length: Option<u32>,
    pub streak_checked: Option<Instant>,
    pub streak_watch_attempts: u32,
    pub live: bool,
    pub last_update: Option<Instant>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MinerSnapshot {
    pub phase: MinerPhase,
    pub desired: usize,
    pub active: usize,
    pub watch_signals: u64,
    pub points_gained: u64,
    pub bonuses_claimed: u64,
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
            last_received: None,
            error: None,
            streamers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MinerEvent {
    Connected,
    PointsGained { channel: String, amount: u64, balance: u64 },
    BonusClaimed { channel: String },
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
}
