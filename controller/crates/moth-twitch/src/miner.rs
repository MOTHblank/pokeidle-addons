use crate::gql::{GqlError, TwitchGql, ViewerIdentity};
use crate::model::{
    normalize_channels, Credentials, MinerConfig, MinerEvent, MinerPhase, MinerSnapshot,
    StreamerState,
};
use crate::pubsub::{self, PubSubEvent};
use crate::watch::WatchClient;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

type PhaseWake = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;
const WATCH_SLOTS: usize = 2;
const STREAK_PRIORITY_BASE: Duration = Duration::from_secs(7 * 60);
const STREAK_PRIORITY_EXTENDED: Duration = Duration::from_secs(20 * 60);
const LONG_SESSION: Duration = Duration::from_secs(10 * 60 * 60);
const MIN_POLL_INTERVAL: Duration = Duration::from_secs(60);
const LIVE_STATUS_INTERVAL: Duration = Duration::from_secs(60);
const OFFLINE_STATUS_INTERVAL: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub struct MinerControl {
    desired: Arc<Mutex<Vec<String>>>,
    live: Arc<Mutex<Vec<String>>>,
    expected_login: Arc<Mutex<Option<String>>>,
    favorites: Arc<Mutex<Vec<String>>>,
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

    pub fn set_live_channels(&self, channels: Vec<String>) {
        let channels=normalize_channels(channels);
        let changed=self.live.lock().map(|mut current|{
            if *current==channels {false} else {*current=channels;true}
        }).unwrap_or(false);
        if changed {let _=self.wake.try_send(());}
    }

    /// Favorites affect selection order, never the fixed two watch slots.
    pub fn set_favorites(&self, channels: Vec<String>) {
        let mut unique = Vec::new();
        for raw in channels.into_iter().take(128) {
            if let Some(login)=crate::model::normalize_login(&raw) {
                if !unique.contains(&login) { unique.push(login); }
            }
        }
        let changed=self.favorites.lock().map(|mut current|{
            if *current==unique {false} else {*current=unique;true}
        }).unwrap_or(false);
        if changed {let _=self.wake.try_send(());}
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
        let live = Arc::new(Mutex::new(Vec::new()));
        let expected_login = Arc::new(Mutex::new(None));
        let favorites = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let phase_wake: PhaseWake = Arc::new(Mutex::new(None));
        let (wake_tx, wake_rx) = mpsc::sync_channel(1);

        let worker_snapshot = Arc::clone(&snapshot);
        let worker_events = Arc::clone(&events);
        let worker_desired = Arc::clone(&desired);
        let worker_live = Arc::clone(&live);
        let worker_expected_login = Arc::clone(&expected_login);
        let worker_favorites = Arc::clone(&favorites);
        let worker_stop = Arc::clone(&stop);
        let worker_phase_wake = Arc::clone(&phase_wake);
        let worker_wake = wake_tx.clone();
        thread::Builder::new()
            .name(format!("moth-twitch-{worker_name}"))
            .spawn(move || {
                run(
                    gql,
                    config,
                    worker_snapshot,
                    worker_events,
                    worker_desired,
                    worker_live,
                    worker_expected_login,
                    worker_favorites,
                    worker_stop,
                    worker_phase_wake,
                    worker_wake,
                    wake_rx,
                )
            })
            .map_err(|error| format!("Could not start Twitch miner: {error}"))?;

        Ok(Self {
            snapshot,
            events,
            control: MinerControl { desired, live, expected_login, favorites, wake: wake_tx },
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
    points_gained: u64,
    last_watch: Option<Instant>,
    last_media_activity: Option<Instant>,
    last_watch_attempt: Option<Instant>,
    watch_failures: u8,
    watch_reward_count: u8,
    last_points_attempt: Option<Instant>,
    last_points_checked: Option<Instant>,
    last_claim_id: Option<String>,
    broadcast_id: Option<String>,
    stream_started: Option<String>,
    game_id: Option<String>,
    game_name: Option<String>,
    live: Option<bool>,
    last_live_checked: Option<Instant>,
    stream_checked: Option<Instant>,
    streak_checked: Option<Instant>,
    streak_confirmed: Option<bool>,
    streak_length: Option<u32>,
    watch_attempts: u32,
    streak_priority_exhausted: bool,
    last_error: Option<String>,
}

/// Two persistent watch slots. A slot keeps its broadcast identity so an
/// offline/restarted stream never inherits the prior stream's streak window.
struct WatchSlot {
    login: String,
    broadcast_id: String,
    entered: Instant,
    streak_limit: Duration,
}

fn streak_pending(channel:&ChannelCache)->bool {
    channel.streak_confirmed!=Some(true) && !channel.streak_priority_exhausted
}

fn watch_backed_off(channel:&ChannelCache,now:Instant)->bool {
    if channel.watch_failures<3 {return false;}
    let steps=u64::from(channel.watch_failures.saturating_sub(2));
    let seconds=(steps*5*60).min(30*60);
    channel.last_watch_attempt.is_some_and(|attempt|
        now.saturating_duration_since(attempt)<Duration::from_secs(seconds))
}

fn effective_live_channels(
    channels:&[String], live_hint:&[String], cache:&HashMap<String,ChannelCache>,
)->Vec<String> {
    channels.iter().filter(|login| {
        cache.get(*login).and_then(|channel|channel.live)
            .unwrap_or_else(||live_hint.binary_search(*login).is_ok())
    }).cloned().collect()
}

fn clear_stream_state(channel:&mut ChannelCache) {
    channel.broadcast_id=None;
    channel.stream_started=None;
    channel.game_id=None;
    channel.game_name=None;
    channel.stream_checked=None;
    channel.streak_confirmed=None;
    channel.streak_checked=None;
    channel.watch_attempts=0;
    channel.watch_reward_count=0;
    channel.watch_failures=0;
    channel.last_watch_attempt=None;
    channel.streak_priority_exhausted=false;
    channel.last_watch=None;
    channel.last_media_activity=None;
}

/// Hold two candidate broadcasts through the reference miner's streak
/// priority window (7 minutes normally, 20 after 10 hours), then rotate. Do not fill temporary gaps
/// between two telemetry POSTs with extra streams: two slots means two streams.
fn refresh_watch_slots(
    slots:&mut Vec<WatchSlot>, live:&[String],
    cache:&mut HashMap<String,ChannelCache>,favorites:&[String],
    now:Instant, streak_limit:Duration,
) {
    for slot in slots.iter() {
        if now.saturating_duration_since(slot.entered)>=slot.streak_limit {
            if let Some(channel)=cache.get_mut(&slot.login) {
                // Like the reference miner, stop prioritizing an unconfirmed
                // streak after its watch window without claiming success.
                if channel.streak_confirmed!=Some(true) {
                    channel.streak_priority_exhausted=true;
                }
            }
        }
    }
    let waiting_streak=live.iter().any(|name|
        !slots.iter().any(|slot|&slot.login==name)
        && cache.get(name).is_some_and(streak_pending));
    slots.retain(|slot| {
        let Some(channel)=cache.get(&slot.login) else {return false};
        if !live.binary_search(&slot.login).is_ok()
            || channel.broadcast_id.as_deref()!=Some(slot.broadcast_id.as_str()) {
            return false;
        }
        if watch_backed_off(channel,now) {return false;}
        let elapsed=now.saturating_duration_since(slot.entered);
        if streak_pending(channel) && elapsed<slot.streak_limit {
            return true;
        }
        !waiting_streak && elapsed<Duration::from_secs(600)
    });
    let mut candidates=live.iter().filter(|name|cache.get(*name)
        .is_some_and(|channel| {
            channel.broadcast_id.is_some() && !watch_backed_off(channel,now)
        })).collect::<Vec<_>>();
    candidates.sort_by(|a,b|watch_priority(a,b,cache,favorites));
    for name in candidates {
        if slots.len()>=WATCH_SLOTS {break;}
        if slots.iter().any(|slot|slot.login==name.as_str()) {continue;}
        let Some(broadcast_id)=cache.get(name).and_then(|c|c.broadcast_id.clone())
            else {continue};
        slots.push(WatchSlot {
            login:name.clone(),broadcast_id,entered:now,streak_limit
        });
    }
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

fn confirm_streak(
    login:&str,
    channel:&mut ChannelCache,
    reason:&str,
    snapshot:&Arc<Mutex<MinerSnapshot>>,
    events:&Arc<Mutex<VecDeque<MinerEvent>>>,
) {
    if channel.streak_confirmed==Some(true) {return;}
    channel.streak_confirmed=Some(true);
    channel.streak_priority_exhausted=false;
    channel.streak_checked=Some(Instant::now());
    push_event(events,MinerEvent::StreakConfirmed {
        channel:login.to_owned(),reason:reason.to_owned()
    });
    if let Ok(mut current)=snapshot.lock() {
        current.last_received=Some(Instant::now());
    }
}

fn apply_pubsub_events(
    gql:&TwitchGql,
    config:&MinerConfig,
    pending:&mut VecDeque<PubSubEvent>,
    cache:&mut HashMap<String,ChannelCache>,
    snapshot:&Arc<Mutex<MinerSnapshot>>,
    events:&Arc<Mutex<VecDeque<MinerEvent>>>,
) {
    let count=pending.len();
    for _ in 0..count {
        let Some(event)=pending.pop_front() else {break};
        match event {
            PubSubEvent::Connected=>{
                if let Ok(mut current)=snapshot.lock() {
                    current.pubsub_connected=true;
                    current.pubsub_error=None;
                }
            }
            PubSubEvent::Warning(error)=>{
                if let Ok(mut current)=snapshot.lock() {
                    current.pubsub_connected=false;
                    current.pubsub_error=Some(error.clone());
                }
                push_event(events,MinerEvent::Warning(error));
            }
            PubSubEvent::PointsEarned{channel_id,amount,balance,reason}=>{
                let Some((login,channel))=cache.iter_mut()
                    .find(|(_,channel)|channel.channel_id.as_deref()==Some(channel_id.as_str()))
                    .map(|(login,channel)|(login.clone(),channel))
                else {
                    pending.push_back(PubSubEvent::PointsEarned {
                        channel_id,amount,balance,reason
                    });
                    continue;
                };
                let before=channel.balance;
                let next=match before {
                    Some(current) if amount>0=>current.saturating_add(amount),
                    Some(current)=>balance.unwrap_or(current),
                    None=>balance.unwrap_or(amount),
                };
                channel.balance=Some(next);
                channel.last_points_checked=Some(Instant::now());
                channel.last_error=None;
                let gained=if amount>0 {
                    amount
                } else {
                    before.and_then(|old|next.checked_sub(old)).unwrap_or(0)
                };
                let moth_watch_active =
                    channel.watch_attempts>0 || channel.last_watch.is_some();
                let watch_reward=matches!(reason.as_str(),"WATCH"|"WATCH_STREAK");
                if gained>0 && moth_watch_active && watch_reward {
                    channel.points_gained=channel.points_gained.saturating_add(gained);
                    if let Ok(mut current)=snapshot.lock() {
                        current.points_gained=current.points_gained.saturating_add(gained);
                        current.last_received=Some(Instant::now());
                    }
                    push_event(events,MinerEvent::PointsGained {
                        channel:login.clone(),amount:gained,balance:next
                    });
                }
                if moth_watch_active {
                    match reason.as_str() {
                        "WATCH_STREAK"=>confirm_streak(
                            &login,channel,"PubSub WATCH_STREAK",snapshot,events
                        ),
                        "WATCH"=>{
                            channel.watch_reward_count=
                                channel.watch_reward_count.saturating_add(1);
                            // Reference parity: a second ordinary WATCH reward
                            // proves Twitch processed enough watch activity to
                            // resolve the current streak even if RewardList lags.
                            if channel.watch_reward_count>=2 {
                                confirm_streak(
                                    &login,channel,"PubSub WATCH x2",snapshot,events
                                );
                            }
                        }
                        _=>{}
                    }
                }
                update_streamer(snapshot,&login,channel,None);
            }
            PubSubEvent::ClaimAvailable{channel_id,claim_id}=>{
                let Some((login,channel))=cache.iter_mut()
                    .find(|(_,channel)|channel.channel_id.as_deref()==Some(channel_id.as_str()))
                    .map(|(login,channel)|(login.clone(),channel))
                else {
                    pending.push_back(PubSubEvent::ClaimAvailable {
                        channel_id,claim_id
                    });
                    continue;
                };
                if !config.claim_bonuses
                    || channel.last_claim_id.as_deref()==Some(claim_id.as_str()) {
                    continue;
                }
                match gql.claim_bonus(&channel_id,&claim_id) {
                    Ok(())=>{
                        channel.last_claim_id=Some(claim_id);
                        channel.last_error=None;
                        if let Ok(mut current)=snapshot.lock() {
                            current.bonuses_claimed=
                                current.bonuses_claimed.saturating_add(1);
                            current.last_received=Some(Instant::now());
                        }
                        push_event(events,MinerEvent::BonusClaimed{channel:login});
                    }
                    Err(error)=>{
                        channel.last_error=Some(error.user_message());
                        push_event(events,MinerEvent::Warning(format!(
                            "{}: {}",login,error.user_message()
                        )));
                    }
                }
            }
        }
    }
    while pending.len()>128 {pending.pop_front();}
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
    live: Arc<Mutex<Vec<String>>>,
    expected_login: Arc<Mutex<Option<String>>>,
    favorites: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    phase_wake: PhaseWake,
    wake: SyncSender<()>,
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
    let mut watch_slots: Vec<WatchSlot> = Vec::new();
    let started_at=Instant::now();
    let (pubsub_tx,pubsub_rx)=mpsc::channel();
    let mut pubsub_started=false;
    let mut pending_pubsub=VecDeque::new();
    let interval = config.poll_interval.max(MIN_POLL_INTERVAL);

    while !stop.load(Ordering::Acquire) {
        if identity.is_none() {
            match gql.authenticate() {
                Ok(viewer) => {
                    watch.set_session(gql.oauth_token(),&viewer.user_id);
                    if let Ok(mut current)=snapshot.lock() {
                        current.reference_client=gql.reference_client();
                    }
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

        pending_pubsub.extend(pubsub_rx.try_iter());
        apply_pubsub_events(
            &gql,&config,&mut pending_pubsub,&mut cache,&snapshot,&events
        );

        let channels = desired.lock().map(|channels| channels.clone()).unwrap_or_default();
        cache.retain(|channel, _| channels.binary_search(channel).is_ok());
        if let Ok(mut current) = snapshot.lock() {
            current.desired = channels.len();
            if channels.is_empty() {
                current.active = 0;
            }
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
                let streak_limit=if started_at.elapsed()>LONG_SESSION {
                    STREAK_PRIORITY_EXTENDED
                } else {
                    STREAK_PRIORITY_BASE
                };
                let auth_failed = poll_channels(
                    &gql,
                    &mut watch,
                    viewer,
                    &config,
                    &channels,
                    &live.lock().map(|items|items.clone()).unwrap_or_default(),
                    &favorites.lock().map(|items|items.clone()).unwrap_or_default(),
                    &mut cache,
                    &mut watch_slots,
                    streak_limit,
                    &stop,
                    &snapshot,
                    &events,
                    &phase_wake,
                );
                if !pubsub_started && !auth_failed {
                    pubsub::spawn(
                        gql.oauth_token().to_owned(),viewer.user_id.clone(),
                        Arc::clone(&stop),pubsub_tx.clone(),wake.clone()
                    );
                    pubsub_started=true;
                }
                pending_pubsub.extend(pubsub_rx.try_iter());
                apply_pubsub_events(
                    &gql,&config,&mut pending_pubsub,&mut cache,&snapshot,&events
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
    live_hint: &[String],
    favorites: &[String],
    cache: &mut HashMap<String, ChannelCache>,
    watch_slots: &mut Vec<WatchSlot>,
    streak_limit: Duration,
    stop: &AtomicBool,
    snapshot: &Arc<Mutex<MinerSnapshot>>,
    events: &Arc<Mutex<VecDeque<MinerEvent>>>,
    phase_wake: &PhaseWake,
) -> bool {
    let mut first_error: Option<GqlError> = None;
    let mut auth_failed = false;
    // Balance checks are paced separately from the two watch slots. PokéIdle's
    // live roster is only a startup hint; Twitch becomes authoritative as soon
    // as a channel receives a direct live-status result.
    let live_now=effective_live_channels(channels,live_hint,cache);
    let mut ordered=channels.iter().collect::<Vec<_>>();
    ordered.sort_by(|a,b| {
        let live_a=live_now.binary_search(a).is_ok();
        let live_b=live_now.binary_search(b).is_ok();
        live_b.cmp(&live_a)
            .then_with(||cache.get(*a).and_then(|c|c.last_points_attempt)
                .cmp(&cache.get(*b).and_then(|c|c.last_points_attempt)))
            .then_with(||a.cmp(b))
    });
    let mut checked_this_tick=0usize;
    for login in ordered {
        if stop.load(Ordering::Acquire) {return false;}
        let channel = cache.entry(login.clone()).or_default();
        let interval=if live_now.binary_search(login).is_ok() {
            Duration::from_secs(45)
        } else {
            Duration::from_secs(180)
        };
        if channel.channel_id.is_none() {
            match gql.lookup_channel_id(login) {
                Ok(id) => channel.channel_id = Some(id),
                Err(error) => {
                    auth_failed |= error.is_auth();
                    channel.last_error=Some(error.user_message());
                    update_streamer(snapshot, login, channel, Some(error.user_message()));
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                    continue;
                }
            }
        }
        let due=channel.last_points_attempt.is_none_or(|at|at.elapsed()>=interval);
        if !due || checked_this_tick>=4 {
            update_streamer(snapshot,login,channel,None);
            continue;
        }
        checked_this_tick+=1;
        channel.last_points_attempt=Some(Instant::now());

        match gql.points_context(login) {
            Ok(context) => {
                channel.last_error=None;
                channel.last_points_checked=Some(Instant::now());
                let _prior = channel.balance.replace(context.balance);
                // Balance polling is deliberately not counted as mined points.
                // A higher balance may come from a pending bonus claim, another
                // Twitch client, or delayed account state. Only PubSub WATCH /
                // WATCH_STREAK events after Moth watch activity are attributed
                // to this miner.

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
                                    channel.last_error=Some(error.user_message());
                                    if first_error.is_none() {
                                        first_error = Some(error);
                                    }
                                }
                            }
                        }
                    }
                }

                // Balance checks are independent from the scarce two watch slots.
                // Live channel metadata and streak milestones are processed below.

                update_streamer(snapshot, login, channel, None);
                if let Ok(mut current) = snapshot.lock() {
                    current.last_received = Some(Instant::now());
                }
            }
            Err(error) => {
                auth_failed |= error.is_auth();
                channel.last_error=Some(error.user_message());
                update_streamer(snapshot, login, channel, Some(error.user_message()));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    let active=cache.values().filter(|channel|channel.last_points_checked
        .is_some_and(|at|at.elapsed()<Duration::from_secs(300))
        && channel.balance.is_some()).count();

    // Mirror the reference miner's lightweight WithIsStreamLiveQuery instead
    // of trusting PokéIdle's legacy estado.twitch.lives roster. The latter can
    // legitimately be absent/empty now that Twitch rewards are redemption-based.
    // Live status is intentionally a complete roster sweep. It is lightweight,
    // independent from points/streak polling, and must not be truncated just
    // because many official channels are online at once.
    for login in channels {
        if stop.load(Ordering::Acquire) {return false;}
        let Some(channel)=cache.get_mut(login) else {continue};
        let Some(channel_id)=channel.channel_id.clone() else {continue};
        let interval=if channel.live==Some(true) {
            LIVE_STATUS_INTERVAL
        } else {
            OFFLINE_STATUS_INTERVAL
        };
        if channel.last_live_checked
            .is_some_and(|at|at.elapsed()<interval) {
            continue;
        }
        channel.last_live_checked=Some(Instant::now());
        match gql.is_stream_live(&channel_id) {
            Ok(Some(live_stream)) => {
                channel.live=Some(true);
                if channel.stream_started.is_none() {
                    channel.stream_started=live_stream.started_at;
                }
                channel.last_error=None;
            }
            Ok(None) => {
                channel.live=Some(false);
                clear_stream_state(channel);
            }
            Err(error) => {
                auth_failed |= error.is_auth();
                channel.last_error=Some(error.user_message());
                if first_error.is_none() {first_error=Some(error);}
            }
        }
        update_streamer(snapshot,login,channel,None);
    }
    let live=effective_live_channels(channels,live_hint,cache);
    // Inspect every live official stream on a bounded cadence. A fresh
    // broadcast always resets prior streak evidence and watch attempts.
    // Limit metadata/reward-list work per tick to prevent large rosters from
    // delaying minute-watched credit and point claims.
    let mut metadata_checks=0usize;
    let mut streak_checks=0usize;
    for login in &live {
        if stop.load(Ordering::Acquire) {return false;}
        let Some(channel)=cache.get_mut(login) else {continue};
        let Some(id)=channel.channel_id.clone() else {continue};
        if metadata_checks<4 && channel.stream_checked.is_none_or(|at|
            at.elapsed() >= Duration::from_secs(120)) {
            metadata_checks+=1;
            channel.stream_checked=Some(Instant::now());
            match gql.stream_info(login,channel.stream_started.as_deref()) {
                Ok(Some(stream)) => {
                    let fresh_broadcast=
                        channel.broadcast_id.as_deref()!=Some(stream.broadcast_id.as_str());
                    channel.game_id=stream.game_id;
                    channel.game_name=stream.game_name;
                    if fresh_broadcast {
                        channel.broadcast_id=Some(stream.broadcast_id);
                        channel.stream_started=Some(stream.started_at);
                        channel.streak_confirmed=None;
                        channel.streak_checked=None;
                        channel.watch_attempts=0;
                        channel.watch_reward_count=0;
                        channel.watch_failures=0;
                        channel.last_watch_attempt=None;
                        channel.streak_priority_exhausted=false;
                        channel.last_watch=None;
                        channel.last_media_activity=None;
                    }
                }
                Ok(None) => {
                    clear_stream_state(channel);
                }
                Err(error) => {
                    auth_failed |= error.is_auth();
                    channel.last_error=Some(error.user_message());
                    if first_error.is_none() {first_error=Some(error);}
                }
            }
        }
        if streak_checks<4 && channel.broadcast_id.is_some()
            && channel.streak_checked.is_none_or(|at|
                at.elapsed() >= Duration::from_secs(120)) {
            streak_checks+=1;
            channel.streak_checked=Some(Instant::now());
            match gql.streak(&id) {
                Ok(milestone) => {
                    channel.streak_length=milestone.length;
                    let detected=match (
                        channel.stream_started.as_deref(),
                        milestone.achieved_at.as_deref()
                    ) {
                        (Some(started),Some(achieved)) =>
                            crate::streak::milestone_for_current_broadcast(started,achieved),
                        (Some(_),None) if milestone.length.is_some() => Some(false),
                        _ => None,
                    };
                    if detected==Some(true) {
                        confirm_streak(login,channel,"RewardList",snapshot,events);
                    } else if channel.streak_confirmed!=Some(true) {
                        channel.streak_confirmed=detected;
                    }
                }
                Err(error) => {
                    auth_failed |= error.is_auth();
                    channel.last_error=Some(error.user_message());
                    if first_error.is_none(){first_error=Some(error);}
                }
            }
        }
        update_streamer(snapshot,login,channel,None);
    }

    // Watch selection persists across polling iterations. Do not create
    // fresh temporary watch sessions while selected streams are between
    // their minute-watched requests.
    refresh_watch_slots(
        watch_slots,&live,cache,favorites,Instant::now(),streak_limit
    );
    for slot in watch_slots.iter() {
        if stop.load(Ordering::Acquire) {return false;}
        let Some(channel)=cache.get_mut(&slot.login) else {continue};
        if channel.last_watch_attempt.is_some_and(|at|
            at.elapsed()<Duration::from_secs(15)) {continue;}
        let Some(channel_id)=channel.channel_id.as_deref() else {continue};
        channel.last_watch_attempt=Some(Instant::now());
        match watch.send_minute_watched(
            &slot.login,channel_id,&slot.broadcast_id,&viewer.user_id,
            channel.game_id.as_deref(),channel.game_name.as_deref(),
            || gql.playback_access(&slot.login).map_err(|error|error.user_message())
        ) {
            Ok(ack)=>{
                channel.last_watch=Some(Instant::now());
                channel.watch_failures=0;
                channel.watch_attempts=channel.watch_attempts.saturating_add(1);
                if let Some(media_error)=ack.media_error {
                    push_event(events,MinerEvent::Warning(format!(
                        "{} media activity: {media_error}",slot.login
                    )));
                } else {
                    channel.last_media_activity=Some(Instant::now());
                }
                if let Ok(mut current)=snapshot.lock(){
                    current.watch_signals=current.watch_signals.saturating_add(1);
                }
            }
            Err(error)=>{
                channel.watch_failures=channel.watch_failures.saturating_add(1);
                channel.last_error=Some(error.clone());
                push_event(events,MinerEvent::Warning(format!("{}: {error}",slot.login)));
            }
        }
        update_streamer(snapshot,&slot.login,channel,None);
    }
    if let Ok(mut current) = snapshot.lock() {
        current.active = active;
        for streamer in &mut current.streamers {
            streamer.live = live.binary_search(&streamer.login).is_ok();
            streamer.watching = watch_slots.iter()
                .any(|slot|slot.login==streamer.login);
        }
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

/// Fair streak coverage: channels that still need this broadcast's
/// milestone are ordered by oldest watch attempt, before favorites. After
/// their initial streak window, favorites determine ordinary farming order.
fn watch_priority(
    a:&String,b:&String,cache:&HashMap<String,ChannelCache>,favorites:&[String]
)->std::cmp::Ordering {
    let left=&cache[a];
    let right=&cache[b];
    let pending=streak_pending;
    let l_pending=pending(left);
    let r_pending=pending(right);
    let favorite=|login:&String|
        favorites.iter().position(|item|item==login).unwrap_or(usize::MAX);
    r_pending.cmp(&l_pending)
        .then_with(|| if l_pending && r_pending {
            left.last_watch_attempt.cmp(&right.last_watch_attempt)
        } else {
            favorite(a).cmp(&favorite(b))
        })
        .then_with(||left.last_watch_attempt.cmp(&right.last_watch_attempt))
        .then_with(||favorite(a).cmp(&favorite(b)))
        .then_with(||a.cmp(b))
}

#[cfg(test)]
mod priority_tests {
    use super::*;
    #[test]
    fn pending_streaks_are_fair_before_favorites() {
        let now=Instant::now();
        let mut cache=HashMap::new();
        cache.insert("favorite".into(),ChannelCache {
            last_watch_attempt:Some(now), ..Default::default()
        });
        cache.insert("unwatched".into(),ChannelCache::default());
        let favorites=vec!["favorite".to_owned()];
        assert_eq!(watch_priority(&"unwatched".into(),&"favorite".into(),
            &cache,&favorites),std::cmp::Ordering::Less);
        cache.get_mut("unwatched").unwrap().streak_confirmed=Some(true);
        assert_eq!(watch_priority(&"favorite".into(),&"unwatched".into(),
            &cache,&favorites),std::cmp::Ordering::Less);
    }
    #[test]
    fn holds_two_streams_until_one_streak_is_confirmed() {
        let start=Instant::now();
        let live=vec!["alpha".to_owned(),"beta".to_owned(),"gamma".to_owned()];
        let mut cache=HashMap::new();
        for name in &live {
            cache.insert(name.clone(),ChannelCache {
                broadcast_id:Some(format!("{name}-broadcast")),
                ..Default::default()
            });
        }
        let mut slots=Vec::new();
        refresh_watch_slots(&mut slots,&live,&mut cache,&[],start,STREAK_PRIORITY_BASE);
        assert_eq!(slots.len(),2);
        let initial=slots.iter().map(|s|s.login.clone()).collect::<Vec<_>>();
        assert_eq!(initial,vec!["alpha","beta"]);
        refresh_watch_slots(&mut slots,&live,&mut cache,&[],
            start+Duration::from_secs(90),STREAK_PRIORITY_BASE);
        assert_eq!(slots.iter().map(|s|s.login.clone()).collect::<Vec<_>>(),
            initial,"a polling cycle must not rotate unfinished streaks");
        cache.get_mut("alpha").unwrap().streak_confirmed=Some(true);
        refresh_watch_slots(&mut slots,&live,&mut cache,&[],
            start+Duration::from_secs(180),STREAK_PRIORITY_BASE);
        assert_eq!(slots.len(),2);
        assert!(slots.iter().any(|slot|slot.login=="beta"));
        assert!(slots.iter().any(|slot|slot.login=="gamma"));
        assert!(!slots.iter().any(|slot|slot.login=="alpha"));
    }

    #[test]
    fn stale_streak_window_releases_slot_without_claiming_success() {
        let start=Instant::now();
        let live=vec!["alpha".to_owned(),"beta".to_owned(),"gamma".to_owned()];
        let mut cache=HashMap::new();
        for name in &live {
            cache.insert(name.clone(),ChannelCache {
                broadcast_id:Some(format!("{name}-broadcast")),
                ..Default::default()
            });
        }
        let mut slots=Vec::new();
        refresh_watch_slots(&mut slots,&live,&mut cache,&[],start,STREAK_PRIORITY_BASE);
        refresh_watch_slots(&mut slots,&live,&mut cache,&[],
            start+STREAK_PRIORITY_BASE+Duration::from_secs(1),STREAK_PRIORITY_BASE);
        assert!(slots.iter().any(|slot|slot.login=="gamma"));
        assert_eq!(cache.get("alpha").unwrap().streak_confirmed,None);
        assert!(cache.get("alpha").unwrap().streak_priority_exhausted);
    }

    #[test]
    fn pubsub_watch_rewards_confirm_streak_like_reference() {
        let gql=TwitchGql::new(Credentials::token_only("token").unwrap()).unwrap();
        let config=MinerConfig::default();
        let snapshot=Arc::new(Mutex::new(MinerSnapshot::default()));
        let events=Arc::new(Mutex::new(VecDeque::new()));
        let mut cache=HashMap::new();
        cache.insert("alpha".to_owned(),ChannelCache {
            channel_id:Some("123".to_owned()),
            streak_confirmed:Some(false),
            watch_attempts:1,
            last_watch:Some(Instant::now()),
            ..Default::default()
        });
        let mut pending=VecDeque::from([
            PubSubEvent::PointsEarned {
                channel_id:"123".into(),amount:10,balance:Some(110),
                reason:"WATCH".into()
            },
            PubSubEvent::PointsEarned {
                channel_id:"123".into(),amount:10,balance:Some(120),
                reason:"WATCH".into()
            },
        ]);
        apply_pubsub_events(
            &gql,&config,&mut pending,&mut cache,&snapshot,&events
        );
        let channel=&cache["alpha"];
        assert_eq!(channel.watch_reward_count,2);
        assert_eq!(channel.streak_confirmed,Some(true));
        assert_eq!(channel.balance,Some(120));
        assert_eq!(channel.points_gained,20);
        let current=snapshot.lock().unwrap();
        assert_eq!(current.points_gained,20);
        assert_eq!(
            current.streamers.iter().find(|streamer|streamer.login=="alpha")
                .map(|streamer|streamer.points_gained),
            Some(20)
        );
    }

    #[test]
    fn unassociated_pubsub_gain_updates_balance_but_not_moth_points() {
        let gql=TwitchGql::new(Credentials::token_only("token").unwrap()).unwrap();
        let config=MinerConfig::default();
        let snapshot=Arc::new(Mutex::new(MinerSnapshot::default()));
        let events=Arc::new(Mutex::new(VecDeque::new()));
        let mut cache=HashMap::new();
        cache.insert("alpha".to_owned(),ChannelCache {
            channel_id:Some("123".to_owned()),
            balance:Some(100),
            ..Default::default()
        });
        let mut pending=VecDeque::from([PubSubEvent::PointsEarned {
            channel_id:"123".into(),amount:100,balance:Some(200),
            reason:"WATCH".into()
        }]);
        apply_pubsub_events(
            &gql,&config,&mut pending,&mut cache,&snapshot,&events
        );
        assert_eq!(cache["alpha"].balance,Some(200));
        assert_eq!(cache["alpha"].points_gained,0);
        assert_eq!(snapshot.lock().unwrap().points_gained,0);
    }

    #[test]
    fn pubsub_watch_streak_confirms_immediately() {
        let gql=TwitchGql::new(Credentials::token_only("token").unwrap()).unwrap();
        let config=MinerConfig::default();
        let snapshot=Arc::new(Mutex::new(MinerSnapshot::default()));
        let events=Arc::new(Mutex::new(VecDeque::new()));
        let mut cache=HashMap::new();
        cache.insert("alpha".to_owned(),ChannelCache {
            channel_id:Some("123".to_owned()),
            streak_confirmed:Some(false),
            watch_attempts:1,
            last_watch:Some(Instant::now()),
            ..Default::default()
        });
        let mut pending=VecDeque::from([PubSubEvent::PointsEarned {
            channel_id:"123".into(),amount:450,balance:Some(1450),
            reason:"WATCH_STREAK".into()
        }]);
        apply_pubsub_events(
            &gql,&config,&mut pending,&mut cache,&snapshot,&events
        );
        assert_eq!(cache["alpha"].streak_confirmed,Some(true));
    }

    #[test]
    fn watch_backoff_grows_from_five_to_thirty_minutes() {
        let now=Instant::now();
        let channel=ChannelCache {
            watch_failures:3,last_watch_attempt:Some(now),..Default::default()
        };
        assert!(watch_backed_off(&channel,now+Duration::from_secs(4*60+59)));
        assert!(!watch_backed_off(&channel,now+Duration::from_secs(5*60)));
        let channel=ChannelCache {
            watch_failures:8,last_watch_attempt:Some(now),..Default::default()
        };
        assert!(watch_backed_off(&channel,now+Duration::from_secs(29*60)));
        assert!(!watch_backed_off(&channel,now+Duration::from_secs(30*60)));
    }

    #[test]
    fn completed_streaks_follow_favorite_order() {
        let mut cache=HashMap::new();
        for name in ["star","other"] {
            cache.insert(name.to_owned(),ChannelCache {
                streak_confirmed:Some(true), ..Default::default()
            });
        }
        assert_eq!(watch_priority(&"star".into(),&"other".into(),&cache,
            &["star".into()]),std::cmp::Ordering::Less);
    }

    #[test]
    fn twitch_live_result_overrides_legacy_live_hint() {
        let channels=vec!["live".into(),"offline".into(),"unknown".into()];
        let hints=vec!["offline".into(),"unknown".into()];
        let mut cache=HashMap::new();
        cache.insert("live".into(),ChannelCache {live:Some(true),..Default::default()});
        cache.insert("offline".into(),ChannelCache {live:Some(false),..Default::default()});
        assert_eq!(effective_live_channels(&channels,&hints,&cache),
            vec!["live".to_owned(),"unknown".to_owned()]);
    }
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
            state.points_gained = cache.points_gained;
            state.last_watch_signal = cache.last_watch;
            state.last_media_activity = cache.last_media_activity;
            state.streak_confirmed=cache.streak_confirmed;
            state.streak_length=cache.streak_length;
            state.streak_checked=cache.streak_checked;
            state.streak_watch_attempts=cache.watch_attempts;
            state.streak_priority_exhausted=cache.streak_priority_exhausted;
            state.last_update = cache.last_points_checked;
            state.error = error.or_else(||cache.last_error.clone());
        }
        None => current.streamers.push(StreamerState {
            login: login.to_owned(),
            channel_id: cache.channel_id.clone(),
            points: cache.balance,
            points_gained: cache.points_gained,
            last_watch_signal: cache.last_watch,
            last_media_activity:cache.last_media_activity,
            streak_confirmed:cache.streak_confirmed,
            streak_length:cache.streak_length,
            streak_checked:cache.streak_checked,
            streak_watch_attempts:cache.watch_attempts,
            streak_priority_exhausted:cache.streak_priority_exhausted,
            live:false,
            watching:false,
            last_update: cache.last_points_checked,
            error:error.or_else(||cache.last_error.clone()),
        }),
    }
}
