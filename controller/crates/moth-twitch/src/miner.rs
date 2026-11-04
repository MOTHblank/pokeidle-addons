use crate::gql::{GqlError, TwitchGql, ViewerIdentity};
use crate::model::{
    normalize_channels, Credentials, MinerConfig, MinerEvent, MinerPhase, MinerSnapshot,
    StreamerState,
};
use crate::watch::WatchClient;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type PhaseWake = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;

#[derive(Clone)]
pub struct MinerControl {
    desired: Arc<Mutex<Vec<String>>>,
    expected_login: Arc<Mutex<Option<String>>>,
    wake: SyncSender<()>,
}

impl MinerControl {
    pub fn set_channels(&self, channels: Vec<String>) {
        let channels = normalize_channels(channels);
        let changed = self.desired.lock()
            .map(|mut current| {
                if *current == channels {
                    false
                } else {
                    *current = channels;
                    true
                }
            })
            .unwrap_or(false);
        if changed {
            let _ = self.wake.try_send(());
        }
    }

    pub fn set_expected_login(&self, login: Option<String>) {
        let login = login.and_then(|login| crate::model::normalize_login(&login));
        let changed = self.expected_login.lock().map(|mut current| {
            if *current == login {
                false
            } else {
                *current = login;
                true
            }
        }).unwrap_or(false);
        if changed {
            let _ = self.wake.try_send(());
        }
    }
}

pub struct Miner {
    snapshot: Arc<Mutex<MinerSnapshot>>,
    events: Arc<Mutex<VecDeque<MinerEvent>>>,
    control: MinerControl,
    stop: Arc<AtomicBool>,
    phase_wake: PhaseWake,
}

impl Miner {
    pub fn start(credentials: Credentials, config: MinerConfig) -> Result<Self, String> {
        let worker_name = credentials.username.as_deref().unwrap_or("account").to_owned();
        let gql = TwitchGql::new(credentials)?;
        let snapshot = Arc::new(Mutex::new(MinerSnapshot::default()));
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let desired = Arc::new(Mutex::new(Vec::new()));
        let expected_login = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let phase_wake: PhaseWake = Arc::new(Mutex::new(None));
        let (wake_tx, wake_rx) = mpsc::sync_channel(1);

        let worker_snapshot = Arc::clone(&snapshot);
        let worker_events = Arc::clone(&events);
        let worker_desired = Arc::clone(&desired);
        let worker_expected_login = Arc::clone(&expected_login);
        let worker_stop = Arc::clone(&stop);
        let worker_phase_wake = Arc::clone(&phase_wake);
        thread::Builder::new()
            .name(format!("moth-twitch-{worker_name}"))
            .spawn(move || {
                run(
                    gql,
                    config,
                    worker_snapshot,
                    worker_events,
                    worker_desired,
                    worker_expected_login,
                    worker_stop,
                    worker_phase_wake,
                    wake_rx,
                )
            })
            .map_err(|error| format!("Could not start Twitch miner: {error}"))?;

        Ok(Self {
            snapshot,
            events,
            control: MinerControl { desired, expected_login, wake: wake_tx },
            stop,
            phase_wake,
        })
    }

    pub fn control(&self) -> MinerControl {
        self.control.clone()
    }

    pub fn snapshot(&self) -> MinerSnapshot {
        self.snapshot.lock().map(|snapshot| snapshot.clone()).unwrap_or_else(|_| {
            let mut snapshot = MinerSnapshot::default();
            snapshot.phase = MinerPhase::Failed;
            snapshot.error = Some("Twitch miner state is unavailable".into());
            snapshot
        })
    }

    pub fn drain_events(&self) -> Vec<MinerEvent> {
        self.events.lock()
            .map(|mut events| events.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn set_phase_waker(&self, wake: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut current) = self.phase_wake.lock() {
            *current = Some(wake);
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.control.wake.try_send(());
    }
}

impl Drop for Miner {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Default)]
struct ChannelCache {
    channel_id: Option<String>,
    balance: Option<u64>,
    last_watch: Option<Instant>,
    last_claim_id: Option<String>,
}

fn notify(wake: &PhaseWake) {
    if let Some(callback) = wake.lock().ok().and_then(|wake| wake.clone()) {
        callback();
    }
}

fn push_event(events: &Arc<Mutex<VecDeque<MinerEvent>>>, event: MinerEvent) {
    if let Ok(mut events) = events.lock() {
        while events.len() >= 128 {
            events.pop_front();
        }
        events.push_back(event);
    }
}

fn set_phase(
    snapshot: &Arc<Mutex<MinerSnapshot>>,
    wake: &PhaseWake,
    phase: MinerPhase,
    error: Option<String>,
) {
    let changed = snapshot.lock().map(|mut current| {
        let changed = current.phase != phase || current.error != error;
        current.phase = phase;
        current.error = error;
        changed
    }).unwrap_or(false);
    if changed {
        notify(wake);
    }
}

fn run(
    mut gql: TwitchGql,
    config: MinerConfig,
    snapshot: Arc<Mutex<MinerSnapshot>>,
    events: Arc<Mutex<VecDeque<MinerEvent>>>,
    desired: Arc<Mutex<Vec<String>>>,
    expected_login: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
    phase_wake: PhaseWake,
    wake_rx: Receiver<()>,
) {
    let mut watch = match WatchClient::new() {
        Ok(watch) => watch,
        Err(error) => {
            set_phase(&snapshot, &phase_wake, MinerPhase::Failed, Some(error));
            return;
        }
    };
    let mut identity: Option<ViewerIdentity> = None;
    let mut cache: HashMap<String, ChannelCache> = HashMap::new();
    let interval = config.poll_interval.max(Duration::from_secs(10));

    while !stop.load(Ordering::Acquire) {
        if identity.is_none() {
            match gql.authenticate() {
                Ok(viewer) => {
                    identity = Some(viewer);
                    set_phase(&snapshot, &phase_wake, MinerPhase::Online, None);
                    push_event(&events, MinerEvent::Connected);
                }
                Err(error) if error.is_auth() => {
                    set_phase(
                        &snapshot,
                        &phase_wake,
                        MinerPhase::NeedsLogin,
                        Some(error.user_message()),
                    );
                }
                Err(error) => {
                    set_phase(
                        &snapshot,
                        &phase_wake,
                        MinerPhase::Reconnecting,
                        Some(error.user_message()),
                    );
                }
            }
        }

        let channels = desired.lock().map(|channels| channels.clone()).unwrap_or_default();
        cache.retain(|channel, _| channels.binary_search(channel).is_ok());
        if let Ok(mut current) = snapshot.lock() {
            current.desired = channels.len();
            current.streamers.retain(|streamer| channels.binary_search(&streamer.login).is_ok());
        }

        if let Some(viewer) = identity.as_ref() {
            let expected = expected_login.lock().ok().and_then(|login| login.clone());
            if let Some(expected)=expected.as_deref().filter(|expected| *expected != viewer.login.as_str()) {
                if let Ok(mut current)=snapshot.lock() {
                    current.active=0;
                }
                set_phase(
                    &snapshot,
                    &phase_wake,
                    MinerPhase::NeedsLogin,
                    Some(format!(
                        "Twitch is signed in as {}, but PokéIdle is linked to {}",
                        viewer.login, expected
                    )),
                );
            } else if !channels.is_empty() {
                let auth_failed = poll_channels(
                    &gql,
                    &mut watch,
                    viewer,
                    &config,
                    &channels,
                    &mut cache,
                    &snapshot,
                    &events,
                    &phase_wake,
                );
                if auth_failed {
                    identity = None;
                }
            } else {
                set_phase(&snapshot, &phase_wake, MinerPhase::Online, None);
            }
        }

        match wake_rx.recv_timeout(interval) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    set_phase(&snapshot, &phase_wake, MinerPhase::Stopped, None);
}

#[allow(clippy::too_many_arguments)]
fn poll_channels(
    gql: &TwitchGql,
    watch: &mut WatchClient,
    viewer: &ViewerIdentity,
    config: &MinerConfig,
    channels: &[String],
    cache: &mut HashMap<String, ChannelCache>,
    snapshot: &Arc<Mutex<MinerSnapshot>>,
    events: &Arc<Mutex<VecDeque<MinerEvent>>>,
    phase_wake: &PhaseWake,
) -> bool {
    let mut active = 0usize;
    let mut first_error: Option<GqlError> = None;
    let mut auth_failed = false;
    let mut watch_slots = config.max_watched_channels.min(channels.len());

    for login in channels {
        let channel = cache.entry(login.clone()).or_default();
        if channel.channel_id.is_none() {
            match gql.lookup_channel_id(login) {
                Ok(id) => channel.channel_id = Some(id),
                Err(error) => {
                    auth_failed |= error.is_auth();
                    update_streamer(snapshot, login, channel, Some(error.user_message()));
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                    continue;
                }
            }
        }

        match gql.points_context(login) {
            Ok(context) => {
                active += 1;
                let prior = channel.balance.replace(context.balance);
                if let Some(gained) = prior.and_then(|old| context.balance.checked_sub(old))
                    .filter(|gain| *gain > 0)
                {
                    if let Ok(mut current) = snapshot.lock() {
                        current.points_gained = current.points_gained.saturating_add(gained);
                    }
                    push_event(events, MinerEvent::PointsGained {
                        channel: login.clone(),
                        amount: gained,
                        balance: context.balance,
                    });
                }

                if config.claim_bonuses {
                    if let (Some(channel_id), Some(claim_id)) =
                        (channel.channel_id.as_deref(), context.available_claim_id.as_deref())
                    {
                        if channel.last_claim_id.as_deref() != Some(claim_id) {
                            match gql.claim_bonus(channel_id, claim_id) {
                                Ok(()) => {
                                    channel.last_claim_id = Some(claim_id.to_owned());
                                    if let Ok(mut current) = snapshot.lock() {
                                        current.bonuses_claimed =
                                            current.bonuses_claimed.saturating_add(1);
                                    }
                                    push_event(
                                        events,
                                        MinerEvent::BonusClaimed { channel: login.clone() },
                                    );
                                }
                                Err(error) => {
                                    auth_failed |= error.is_auth();
                                    if first_error.is_none() {
                                        first_error = Some(error);
                                    }
                                }
                            }
                        }
                    }
                }

                let watch_due = channel.last_watch
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(55));
                if watch_due && watch_slots > 0 {
                    match gql.stream_info(login) {
                        Ok(Some(stream)) => {
                            watch_slots -= 1;
                            if let Some(channel_id) = channel.channel_id.as_deref() {
                                match watch.send_minute_watched(
                                    login,
                                    channel_id,
                                    &stream.broadcast_id,
                                    &viewer.user_id,
                                ) {
                                    Ok(()) => channel.last_watch = Some(Instant::now()),
                                    Err(error) => {
                                        push_event(events, MinerEvent::Warning(format!(
                                            "{login}: {error}"
                                        )));
                                    }
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            auth_failed |= error.is_auth();
                            if first_error.is_none() {
                                first_error = Some(error);
                            }
                        }
                    }
                }

                update_streamer(snapshot, login, channel, None);
                if let Ok(mut current) = snapshot.lock() {
                    current.last_received = Some(Instant::now());
                }
            }
            Err(error) => {
                auth_failed |= error.is_auth();
                update_streamer(snapshot, login, channel, Some(error.user_message()));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if let Ok(mut current) = snapshot.lock() {
        current.active = active;
        current.streamers.sort_by(|left, right| left.login.cmp(&right.login));
    }

    match first_error {
        Some(error) if error.is_auth() => set_phase(
            snapshot,
            phase_wake,
            MinerPhase::NeedsLogin,
            Some(error.user_message()),
        ),
        Some(error) if active == 0 => set_phase(
            snapshot,
            phase_wake,
            MinerPhase::Reconnecting,
            Some(error.user_message()),
        ),
        Some(error) => set_phase(
            snapshot,
            phase_wake,
            MinerPhase::Online,
            Some(error.user_message()),
        ),
        None => set_phase(snapshot, phase_wake, MinerPhase::Online, None),
    }
    auth_failed
}

fn update_streamer(
    snapshot: &Arc<Mutex<MinerSnapshot>>,
    login: &str,
    cache: &ChannelCache,
    error: Option<String>,
) {
    let Ok(mut current) = snapshot.lock() else { return };
    let state = current.streamers.iter_mut().find(|streamer| streamer.login == login);
    match state {
        Some(state) => {
            state.channel_id.clone_from(&cache.channel_id);
            state.points = cache.balance;
            state.last_update = Some(Instant::now());
            state.error = error;
        }
        None => current.streamers.push(StreamerState {
            login: login.to_owned(),
            channel_id: cache.channel_id.clone(),
            points: cache.balance,
            last_update: Some(Instant::now()),
            error,
        }),
    }
}
