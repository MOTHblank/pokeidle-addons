//! Native Twitch channel-points integration.
//!
//! Firefox can still provide the OAuth token already used by Twitch's web IRC
//! client, but IRC presence is no longer the automation mechanism. The token is
//! handed to the UI-agnostic `moth-twitch` worker, which tracks channel-point
//! balances and claims available community-points bonuses. PokéIdle itself does
//! not depend on Twitch authentication.

use moth_twitch::{Credentials, Miner, MinerConfig};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct TwitchStreamerSnapshot {
    pub login: String,
    pub points: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TwitchSnapshot {
    pub phase: String,
    pub desired: usize,
    pub joined: usize,
    pub last_received: Option<Instant>,
    pub error: Option<String>,
    pub points_gained: u64,
    pub bonuses_claimed: u64,
    pub streamers: Vec<TwitchStreamerSnapshot>,
}

impl Default for TwitchSnapshot {
    fn default() -> Self {
        Self {
            phase: "NeedsLogin".into(),
            desired: 0,
            joined: 0,
            last_received: None,
            error: None,
            points_gained: 0,
            bonuses_claimed: 0,
            streamers: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct TwitchControl {
    inner: Option<moth_twitch::MinerControl>,
    desired: Arc<Mutex<Vec<String>>>,
}

impl TwitchControl {
    pub fn set_channels(&self, channels: Vec<String>) {
        if let Ok(mut desired) = self.desired.lock() {
            *desired = channels.clone();
        }
        if let Some(inner) = self.inner.as_ref() {
            inner.set_channels(channels);
        }
    }

    #[cfg(test)]
    pub fn channels(&self) -> Vec<String> {
        self.desired.lock().map(|channels| channels.clone()).unwrap_or_default()
    }
}

pub struct TwitchSession {
    miner: Option<Miner>,
    fallback: TwitchSnapshot,
}

impl TwitchSession {
    /// Twitch is optional for the game connection. If Firefox has already
    /// exposed an authenticated Twitch token, start the miner; otherwise retain
    /// a cheap NeedsLogin snapshot and let PokéIdle continue normally.
    pub fn start_optional(browser_bootstrap: Option<&serde_json::Value>) -> Self {
        let credentials = browser_bootstrap.and_then(|value| {
            credentials_from_browser(value).map_err(|error| {
                crate::logging::warn(&format!(
                    "Twitch miner unavailable until Twitch sign-in: {error}"
                ));
                error
            }).ok()
        });

        match credentials {
            Some(credentials) => match Miner::start(credentials, MinerConfig::default()) {
                Ok(miner) => Self {
                    miner: Some(miner),
                    fallback: TwitchSnapshot::default(),
                },
                Err(error) => {
                    crate::logging::warn(&format!(
                        "Twitch miner could not start; PokéIdle will continue: {error}"
                    ));
                    Self {
                        miner: None,
                        fallback: TwitchSnapshot {
                            phase: "Failed".into(),
                            error: Some(error),
                            ..TwitchSnapshot::default()
                        },
                    }
                }
            },
            None => Self {
                miner: None,
                fallback: TwitchSnapshot::default(),
            },
        }
    }

    pub fn control(&self) -> TwitchControl {
        TwitchControl {
            inner: self.miner.as_ref().map(Miner::control),
            desired: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn snapshot(&self) -> TwitchSnapshot {
        let Some(miner) = self.miner.as_ref() else {
            return self.fallback.clone();
        };
        let state = miner.snapshot();
        TwitchSnapshot {
            phase: state.phase.as_str().into(),
            desired: state.desired,
            joined: state.active,
            last_received: state.last_received,
            error: state.error,
            points_gained: state.points_gained,
            bonuses_claimed: state.bonuses_claimed,
            streamers: state.streamers.into_iter().map(|streamer| TwitchStreamerSnapshot {
                login: streamer.login,
                points: streamer.points,
                error: streamer.error,
            }).collect(),
        }
    }

    pub fn set_phase_waker(&self, wake: Arc<dyn Fn() + Send + Sync>) {
        if let Some(miner) = self.miner.as_ref() {
            miner.set_phase_waker(wake);
        }
    }

    pub fn stop(&self) {
        if let Some(miner) = self.miner.as_ref() {
            miner.stop();
        }
    }
}

pub fn validate(value: &serde_json::Value) -> Result<(), String> {
    credentials_from_browser(value).map(|_| ())
}

fn credentials_from_browser(value: &serde_json::Value) -> Result<Credentials, String> {
    let pass = value.get("pass").and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Twitch OAuth authentication was not captured".to_owned())?;
    let nick = value.get("nick").and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Twitch username was not captured".to_owned())?;
    let token = pass.strip_prefix("PASS oauth:")
        .ok_or_else(|| "Captured Twitch authentication is not OAuth".to_owned())?;
    let username = nick.strip_prefix("NICK ")
        .ok_or_else(|| "Captured Twitch username is invalid".to_owned())?;
    Credentials::new(username, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn captured_irc_oauth_is_reused_without_irc_presence() {
        let credentials = credentials_from_browser(&json!({
            "url": "wss://irc-ws.chat.twitch.tv:443",
            "pass": "PASS oauth:abc_123",
            "nick": "NICK moth_user",
            "cap": "CAP REQ :twitch.tv/tags"
        })).unwrap();
        assert_eq!(credentials.username, "moth_user");
        assert_eq!(credentials.oauth_token, "abc_123");
    }

    #[test]
    fn twitch_is_not_required_for_native_game_startup() {
        let session = TwitchSession::start_optional(None);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.phase, "NeedsLogin");
        assert_eq!(snapshot.desired, 0);
    }
}
