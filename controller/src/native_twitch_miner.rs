//! Controller adapter for the optional native Twitch channel-points miner.
//!
//! The miner is deliberately independent from PokéIdle authentication: a
//! missing or expired Twitch login never prevents the game worker from running.

use moth_twitch::{Credentials, Miner, MinerConfig, MinerControl, MinerSnapshot};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct TwitchMinerControl {
    inner: Arc<Mutex<Option<MinerControl>>>,
}

impl TwitchMinerControl {
    fn attach(&self, control: Option<MinerControl>) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner = control;
        }
    }

    fn with(&self, f: impl FnOnce(&MinerControl)) {
        if let Ok(inner) = self.inner.lock() {
            if let Some(inner) = inner.as_ref() { f(inner); }
        }
    }

    pub fn set_channels(&self, channels: Vec<String>) {
        self.with(|inner| inner.set_channels(channels));
    }

    pub fn set_live_channels(&self, channels: Vec<String>) {
        self.with(|inner| inner.set_live_channels(channels));
    }

    pub fn set_favorites(&self, channels: Vec<String>) {
        self.with(|inner| inner.set_favorites(channels));
    }

    pub fn set_expected_login(&self, login: Option<String>) {
        self.with(|inner| inner.set_expected_login(login));
    }
}

pub struct TwitchMinerSession {
    miner: Mutex<Option<Miner>>,
    control: TwitchMinerControl,
}

impl TwitchMinerSession {
    pub fn start_optional(token: Option<&str>) -> Self {
        let control = TwitchMinerControl::default();
        let session = Self { miner: Mutex::new(None), control };
        if let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) {
            if let Err(error) = session.replace_token(token, None) {
                crate::logging::warn(&format!(
                    "Optional Twitch miner could not start; PokéIdle is unaffected: {error}"
                ));
            }
        }
        session
    }

    pub fn replace_token(
        &self,
        token: &str,
        wake: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Result<(), String> {
        let credentials = Credentials::token_only(token)
            .map_err(|error| error.to_string())?;
        let miner = Miner::start(credentials, MinerConfig::default())
            .map_err(|error| error.to_string())?;
        if let Some(wake) = wake {
            miner.set_phase_waker(wake);
        }
        let new_control = miner.control();
        let mut slot = self.miner.lock()
            .map_err(|_| "Twitch miner state unavailable".to_owned())?;
        if let Some(previous) = slot.take() {
            previous.stop();
        }
        *slot = Some(miner);
        self.control.attach(Some(new_control));
        Ok(())
    }

    pub fn clear_login(&self) {
        if let Ok(mut slot) = self.miner.lock() {
            if let Some(previous) = slot.take() { previous.stop(); }
        }
        self.control.attach(None);
    }

    pub fn set_favorites(&self, channels: Vec<String>) {
        self.control.set_favorites(channels);
    }

    pub fn control(&self) -> TwitchMinerControl {
        self.control.clone()
    }

    pub fn snapshot(&self) -> Option<MinerSnapshot> {
        self.miner.lock().ok()
            .and_then(|miner| miner.as_ref().map(Miner::snapshot))
    }

    pub fn set_phase_waker(&self, wake:Arc<dyn Fn() + Send + Sync>) {
        if let Ok(miner)=self.miner.lock() {
            if let Some(miner)=miner.as_ref() {
                miner.set_phase_waker(wake);
            }
        }
    }

    pub fn stop(&self) {
        self.clear_login();
    }
}

impl Drop for TwitchMinerSession {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Preserve the full official catalog while the miner only watches live channels.
pub fn official_channels(payload: &Value) -> Option<Vec<String>> {
    let official = payload.get("estado")?.get("twitch")?.get("oficiais")?.as_array()?;
    let mut channels = official.iter().filter(|entry|
        entry.get("bonus").and_then(Value::as_bool).unwrap_or(true)
    ).filter_map(|entry|{
        entry.as_str().or_else(||entry.get("login").or_else(||entry.get("nome"))
            .and_then(Value::as_str)).and_then(moth_twitch::normalize_login)
    }).collect::<Vec<_>>();
    channels.sort_unstable();
    channels.dedup();
    Some(channels)
}

pub fn official_live_channels(payload: &Value) -> Option<Vec<String>> {
    let twitch = payload.get("estado")?.get("twitch")?;
    let official = twitch.get("oficiais")?.as_array()?;
    let live = twitch.get("lives")?.as_array()?;

    let eligible = official.iter().filter(|entry| {
        entry.get("bonus").and_then(Value::as_bool).unwrap_or(true)
    }).filter_map(|entry| {
        entry.as_str().or_else(||entry.get("login")
            .or_else(|| entry.get("nome"))
            .and_then(Value::as_str))
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
    fn full_official_roster_includes_offline_channels() {
        let frame=json!({"estado":{"twitch":{"oficiais":[
            {"login":"offline"},{"login":"live","bonus":true},
            {"login":"excluded","bonus":false}
        ],"lives":[{"login":"live"}]}}});
        assert_eq!(official_channels(&frame).unwrap(),vec!["live","offline"]);
        assert_eq!(official_live_channels(&frame).unwrap(),vec!["live"]);
    }

    #[test]
    fn missing_roster_does_not_clear_unknown_targets() {
        assert!(official_live_channels(&json!({"estado":{"twitch":{"vinculado":true}}})).is_none());
    }
}
