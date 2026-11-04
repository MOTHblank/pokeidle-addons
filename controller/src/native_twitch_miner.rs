//! Controller adapter for the optional native Twitch channel-points miner.
//!
//! The miner is deliberately independent from PokéIdle authentication: a
//! missing or expired Twitch login never prevents the game worker from running.

use moth_twitch::{
    Credentials, Miner, MinerConfig, MinerControl, MinerEvent, MinerEventSink, MinerSnapshot,
};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

#[derive(Default)]
struct TwitchControlState {
    inner: Option<MinerControl>,
    channels: Vec<String>,
    live_channels: Vec<String>,
    favorites: Vec<String>,
    expected_login: Option<String>,
}

#[derive(Clone, Default)]
pub struct TwitchMinerControl {
    state: Arc<Mutex<TwitchControlState>>,
}

impl TwitchMinerControl {
    fn attach(&self, control: Option<MinerControl>) {
        let replay=self.state.lock().ok().map(|mut state| {
            state.inner=control.clone();
            control.map(|inner|(
                inner,
                state.channels.clone(),
                state.live_channels.clone(),
                state.favorites.clone(),
                state.expected_login.clone(),
            ))
        }).flatten();
        if let Some((inner,channels,live,favorites,expected_login))=replay {
            inner.set_channels(channels);
            inner.set_live_channels(live);
            inner.set_favorites(favorites);
            inner.set_expected_login(expected_login);
        }
    }

    pub fn set_channels(&self, channels: Vec<String>) {
        let inner=self.state.lock().ok().map(|mut state| {
            state.channels=channels.clone();
            state.inner.clone()
        }).flatten();
        if let Some(inner)=inner {inner.set_channels(channels);}
    }

    pub fn set_live_channels(&self, channels: Vec<String>) {
        let inner=self.state.lock().ok().map(|mut state| {
            state.live_channels=channels.clone();
            state.inner.clone()
        }).flatten();
        if let Some(inner)=inner {inner.set_live_channels(channels);}
    }

    pub fn set_favorites(&self, channels: Vec<String>) {
        let inner=self.state.lock().ok().map(|mut state| {
            state.favorites=channels.clone();
            state.inner.clone()
        }).flatten();
        if let Some(inner)=inner {inner.set_favorites(channels);}
    }

    pub fn set_expected_login(&self, login: Option<String>) {
        let inner=self.state.lock().ok().map(|mut state| {
            state.expected_login=login.clone();
            state.inner.clone()
        }).flatten();
        if let Some(inner)=inner {inner.set_expected_login(login);}
    }
}

type PhaseWake = Arc<dyn Fn() + Send + Sync>;
type TwitchLogMessage = (String, MinerEvent);

static TWITCH_LOG_SENDER: OnceLock<SyncSender<TwitchLogMessage>> = OnceLock::new();

fn log_miner_event(account:&str,event:MinerEvent) {
    let prefix=format!("{account} Twitch");
    match event {
        MinerEvent::Connected {login,reference_client} => crate::logging::info(&format!(
            "{prefix} authenticated as {login} ({})",
            if reference_client {"reference device session"} else {"Firefox compatibility session"}
        )),
        MinerEvent::Stopped => crate::logging::info(&format!(
            "{prefix} miner stopped"
        )),
        MinerEvent::RosterUpdated {channels} => crate::logging::info(&format!(
            "{prefix} official channel roster loaded: {channels} channel(s)"
        )),
        MinerEvent::PubSubConnected => crate::logging::info(&format!(
            "{prefix} channel-points events connected"
        )),
        MinerEvent::StreamOnline {channel,balance} => {
            let suffix=balance.map(|points|format!(" ({points} points)")).unwrap_or_default();
            crate::logging::info(&format!("{prefix} online: {channel}{suffix}"));
        }
        MinerEvent::StreamOffline {channel} => crate::logging::info(&format!(
            "{prefix} offline: {channel}"
        )),
        MinerEvent::WatchSlotStarted {channel,slot,active} => crate::logging::info(&format!(
            "{prefix} watch slot {slot}/2 selected: {channel} ({active}/2 active)"
        )),
        MinerEvent::WatchActive {channel} => crate::logging::info(&format!(
            "{prefix} watch active: {channel}; first minute-watched signal accepted"
        )),
        MinerEvent::WatchSlotStopped {channel,reason,active} => crate::logging::info(&format!(
            "{prefix} watch slot released: {channel} ({reason}; {active}/2 active)"
        )),
        MinerEvent::PointsGained {channel,amount,balance} => crate::logging::info(&format!(
            "{prefix} +{amount} watch points: {channel} (balance {balance})"
        )),
        MinerEvent::BonusClaimed {channel} => crate::logging::info(&format!(
            "{prefix} bonus chest claimed: {channel}"
        )),
        MinerEvent::StreakConfirmed {channel,reason} => crate::logging::info(&format!(
            "{prefix} streak confirmed: {channel} ({reason})"
        )),
        MinerEvent::Warning(message) => crate::logging::warn(&format!(
            "{prefix}: {message}"
        )),
    }
}

fn twitch_log_sender()->SyncSender<TwitchLogMessage> {
    TWITCH_LOG_SENDER.get_or_init(|| {
        let (tx,rx)=mpsc::sync_channel::<TwitchLogMessage>(256);
        if let Err(error)=thread::Builder::new()
            .name("moth-twitch-logger".into())
            .spawn(move || {
                while let Ok((account,event))=rx.recv() {
                    log_miner_event(&account,event);
                }
            })
        {
            crate::logging::warn(&format!(
                "Twitch event logger could not start: {error}"
            ));
        }
        tx
    }).clone()
}

pub struct TwitchMinerSession {
    account_label: String,
    miner: Mutex<Option<Miner>>,
    control: TwitchMinerControl,
    phase_wake: Mutex<Option<PhaseWake>>,
    closed: AtomicBool,
}

impl TwitchMinerSession {
    pub fn start_optional(account_label:impl Into<String>,token: Option<&str>) -> Self {
        let control = TwitchMinerControl::default();
        let session = Self {
            account_label:account_label.into(),
            miner: Mutex::new(None),
            control,
            phase_wake: Mutex::new(None),
            closed: AtomicBool::new(false),
        };
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
        token:&str,
        wake:Option<PhaseWake>,
    )->Result<(),String> {
        self.replace_token_checked(token,wake,||true).and_then(|applied|
            if applied {Ok(())} else {Err("Twitch miner replacement was superseded".into())})
    }

    pub fn replace_token_checked(
        &self,
        token:&str,
        wake:Option<PhaseWake>,
        is_current:impl Fn()->bool,
    )->Result<bool,String> {
        if self.closed.load(Ordering::Acquire) || !is_current() {
            return Ok(false);
        }
        if let Some(wake)=wake {
            if let Ok(mut current)=self.phase_wake.lock() {
                *current=Some(wake);
            }
        }
        let credentials = Credentials::token_only(token)
            .map_err(|error| error.to_string())?;
        let log_sender=twitch_log_sender();
        let account_label=self.account_label.clone();
        let event_sink:MinerEventSink=Arc::new(move |event| {
            let _=log_sender.try_send((account_label.clone(),event));
        });
        let miner = Miner::start_with_sink(
            credentials, MinerConfig::default(), Some(event_sink)
        ).map_err(|error| error.to_string())?;
        if let Some(wake)=self.phase_wake.lock().ok().and_then(|current|current.clone()) {
            miner.set_phase_waker(wake);
        }
        let new_control = miner.control();
        if self.closed.load(Ordering::Acquire) || !is_current() {
            miner.stop();
            return Ok(false);
        }
        let mut slot = self.miner.lock()
            .map_err(|_| "Twitch miner state unavailable".to_owned())?;
        if self.closed.load(Ordering::Acquire) || !is_current() {
            miner.stop();
            return Ok(false);
        }
        if let Some(previous) = slot.take() {
            previous.stop();
        }
        *slot = Some(miner);
        drop(slot);
        self.control.attach(Some(new_control));
        Ok(true)
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

    pub fn set_phase_waker(&self, wake:PhaseWake) {
        if let Ok(mut current)=self.phase_wake.lock() {
            *current=Some(Arc::clone(&wake));
        }
        let active=self.miner.lock().ok().map(|miner| {
            if let Some(miner)=miner.as_ref() {
                miner.set_phase_waker(Arc::clone(&wake));
                true
            } else {
                false
            }
        }).unwrap_or(false);
        // Authentication can finish before ControllerApp adopts the native
        // session and installs its wake callback. Publish the already-current
        // state immediately instead of waiting for a later Twitch event.
        if active {wake();}
    }

    pub fn stop(&self) {
        self.closed.store(true,Ordering::Release);
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
