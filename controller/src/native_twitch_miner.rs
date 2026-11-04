//! Controller adapter for the optional native Twitch channel-points miner.
//!
//! The miner is deliberately independent from PokéIdle authentication: a
//! missing or expired Twitch login never prevents the game worker from running.

use moth_twitch::{Credentials, Miner, MinerConfig, MinerControl, MinerSnapshot};
use serde_json::Value;

#[derive(Clone, Default)]
pub struct TwitchMinerControl {
    inner: Option<MinerControl>,
}

impl TwitchMinerControl {
    pub fn set_channels(&self, channels: Vec<String>) {
        if let Some(inner) = self.inner.as_ref() {
            inner.set_channels(channels);
        }
    }
}

pub struct TwitchMinerSession {
    miner: Option<Miner>,
}

impl TwitchMinerSession {
    pub fn start_optional(bootstrap: Option<&Value>) -> Self {
        let Some(token) = bootstrap
            .and_then(|value| value.get("token"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
        else {
            return Self { miner: None };
        };

        let miner = Credentials::token_only(token)
            .and_then(|credentials| Miner::start(credentials, MinerConfig::default()))
            .map_err(|error| {
                crate::logging::warn(&format!(
                    "Optional Twitch miner could not start; PokéIdle is unaffected: {error}"
                ));
                error
            })
            .ok();

        Self { miner }
    }

    pub fn control(&self) -> TwitchMinerControl {
        TwitchMinerControl {
            inner: self.miner.as_ref().map(Miner::control),
        }
    }

    pub fn snapshot(&self) -> Option<MinerSnapshot> {
        self.miner.as_ref().map(Miner::snapshot)
    }

    pub fn stop(&self) {
        if let Some(miner) = self.miner.as_ref() {
            miner.stop();
        }
    }
}

impl Drop for TwitchMinerSession {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn official_live_channels(payload: &Value) -> Option<Vec<String>> {
    let twitch = payload.get("estado")?.get("twitch")?;
    let official = twitch.get("oficiais")?.as_array()?;
    let live = twitch.get("lives")?.as_array()?;

    let mut eligible = official.iter().filter(|entry| {
        entry.get("bonus").and_then(Value::as_bool).unwrap_or(true)
    }).filter_map(|entry| {
        entry.get("login")
            .or_else(|| entry.get("nome"))
            .and_then(Value::as_str)
            .and_then(moth_twitch::normalize_login)
    }).collect::<std::collections::BTreeSet<_>>();

    let mut channels = live.iter().filter_map(|entry| {
        let login = entry.as_str().or_else(|| {
            entry.get("login")
                .or_else(|| entry.get("nome"))
                .and_then(Value::as_str)
        })?;
        moth_twitch::normalize_login(login)
    }).filter(|login| eligible.contains(login))
      .collect::<Vec<_>>();

    channels.sort_unstable();
    channels.dedup();
    eligible.clear();
    Some(channels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn official_live_roster_selects_only_bonus_channels() {
        let frame = json!({"estado":{"twitch":{
            "oficiais":[
                {"login":"One","bonus":true},
                {"login":"Two","bonus":false},
                {"login":"Three","bonus":true}
            ],
            "lives":[{"login":"ONE"},{"login":"two"}]
        }}});
        assert_eq!(official_live_channels(&frame).unwrap(), vec!["one"]);
    }

    #[test]
    fn missing_roster_does_not_clear_unknown_targets() {
        assert!(official_live_channels(&json!({"estado":{"twitch":{"vinculado":true}}})).is_none());
    }
}
