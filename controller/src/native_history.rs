//! Bounded, restart-persistent native battle history, independent of userscripts.
//! Only derived gameplay counters are saved; never raw frames, auth, cookies,
//! WebSocket URLs or mutable credentials.
use crate::config::{data_root, GameProfile};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_DAYS: u64 = 90;
const SAVE_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
const FINAL_WRITE_TIMEOUT: Duration = Duration::from_secs(2);
// The game worker never queues snapshots or serialized JSON. A single
// generation counter represents all pending writes; bursts coalesce into
// the latest live snapshot and cannot accumulate unbounded memory.
#[derive(Default)]
struct WriteState {
    requested:u64,
    completed:u64,
    pending:bool,
    stopping:bool,
    last_error:Option<String>,
}
#[derive(Default)]
struct WriteSignal {
    state:Mutex<WriteState>,
    wake:Condvar,
    handle:Mutex<Option<thread::JoinHandle<()>>>,
    // Keep the history file owned until the detached writer exits.
    lease:Mutex<Option<Arc<HistoryLease>>>,
}
impl WriteSignal {
    fn request(&self)->Result<u64,String> {
        let mut state=self.state.lock()
            .map_err(|_|"History save coordinator unavailable".to_owned())?;
        if state.stopping {return Err("History writer already stopped".into())}
        state.requested=state.requested.saturating_add(1);
        state.pending=true;
        let sequence=state.requested;
        self.wake.notify_one();
        Ok(sequence)
    }
    #[cfg(test)]
    fn wait_written(&self,sequence:u64,timeout:Duration)->Result<(),String> {
        let state=self.state.lock()
            .map_err(|_|"History save coordinator unavailable".to_owned())?;
        let (state,_)=self.wake.wait_timeout_while(state,timeout,
            |state|state.completed<sequence)
            .map_err(|_|"History save coordinator unavailable".to_owned())?;
        if state.completed<sequence {
            return Err("History final write was not acknowledged".into());
        }
        state.last_error.clone().map_or(Ok(()),Err)
    }
    // Check infrequently from the socket worker; failed writes are retried
    // even if no new battle events arrive. Never stack requests during I/O.
    #[cfg(test)]
    fn in_flight(&self)->Option<u64> {
        self.state.lock().ok().and_then(|state|
            (state.requested>state.completed).then_some(state.requested))
    }
    fn retry_due(&self)->bool {
        self.state.lock().is_ok_and(|state|
            state.last_error.is_some() && state.completed==state.requested &&
            !state.pending && !state.stopping)
    }
    fn stop(&self) {
        if let Ok(mut state)=self.state.lock() {state.stopping=true;}
        self.wake.notify_all();
    }
    // A stopped socket worker must not release its profile to a replacement
    // recorder while its previous writer can still rename the same file.
    #[cfg(test)]
    fn stop_and_join(&self) {
        self.stop();
        if let Ok(mut worker)=self.handle.lock() {
            if let Some(handle)=worker.take() {
                if handle.join().is_err() {
                    crate::logging::warn("Native history writer panicked during shutdown");
                }
            }
        }
    }
}

// Only one recorder at a time may own a profile's history file, including
// the time spent reading it and draining the previous writer. Stop remains
// nonblocking for the UI; the replacement session waits on its startup thread.
#[derive(Default)]
struct HistoryGate {
    active:Mutex<bool>,
    available:Condvar,
}
struct HistoryLease {
    gate:Arc<HistoryGate>,
}
impl HistoryGate {
    fn acquire(self:&Arc<Self>)->HistoryLease {
        let mut active=self.active.lock().unwrap_or_else(|error|error.into_inner());
        while *active {
            active=self.available.wait(active).unwrap_or_else(|error|error.into_inner());
        }
        *active=true;
        HistoryLease{gate:Arc::clone(self)}
    }
}
impl Drop for HistoryLease {
    fn drop(&mut self) {
        let mut active=self.gate.active.lock().unwrap_or_else(|error|error.into_inner());
        *active=false;
        self.gate.available.notify_one();
    }
}
fn profile_history_gate(profile:GameProfile)->Arc<HistoryGate> {
    static GATES:OnceLock<[Arc<HistoryGate>;4]>=OnceLock::new();
    Arc::clone(&GATES.get_or_init(||
        std::array::from_fn(|_|Arc::new(HistoryGate::default())))[profile.index()])
}

// One writer per active native session, parked on a condvar when idle.
// The writer owns cloning, JSON construction, serialization, and disk I/O;
// it only briefly locks the live snapshot to obtain a stable copy.
fn start_history_writer(
    shared:Arc<Mutex<HistorySnapshot>>, path:PathBuf, nick:String
)->Result<Arc<WriteSignal>,String> {
    let signal=Arc::new(WriteSignal::default());
    let worker_signal=Arc::clone(&signal);
    let handle=thread::Builder::new().name("moth-native-history".into())
        .spawn(move || {
            // Report severe snapshot-copy contention once per writer session,
            // not on every five-second save.
            let mut reported_slow_copy=false;
            loop {
            let sequence={
                let Ok(mut state)=worker_signal.state.lock() else{return};
                while !state.pending {
                    if state.stopping {return}
                    state=match worker_signal.wake.wait(state) {
                        Ok(next)=>next,Err(_)=>return,
                    };
                }
                state.pending=false;
                state.requested
            };
            let result=match shared.lock() {
                Ok(snapshot)=>{
                    let started=Instant::now();
                    let copy=snapshot.clone();
                    drop(snapshot);
                    if !reported_slow_copy && started.elapsed()>Duration::from_millis(50) {
                        reported_slow_copy=true;
                        crate::logging::warn(
                            "Native history snapshot copy held the game-history lock over 50 ms");
                    }
                    // JSON creation and serialization never run on the game
                    // socket worker or under the live-history mutex.
                    serialize_history(&copy,&nick)
                        .and_then(|raw|write_atomic(&path,&raw))
                }
                Err(_)=>Err("History snapshot unavailable".into()),
            };
            if let Ok(mut snapshot)=shared.lock() {
                snapshot.save_error=result.as_ref().err().cloned();
            }
            if let Err(error)=&result {
                crate::logging::warn(&format!("Native history persistence: {error}"));
            }
            if let Ok(mut state)=worker_signal.state.lock() {
                state.completed=sequence;
                state.last_error=result.err();
                worker_signal.wake.notify_all();
            } else {return}
            }
        })
        .map_err(|error|format!("Could not start history writer: {error}"))?;
    *signal.handle.lock().map_err(|_|"History writer handle unavailable".to_owned())?=Some(handle);
    Ok(signal)
}
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    pub day: u64,
    pub hunt: String,
    pub kills: u64,
    pub captures: u64,
    pub shiny_seen: u64,
    pub shiny_captured: u64,
    pub trainer_xp: u64,
    pub pokemon_xp: u64,
    /// Only time between consecutive server-confirmed kills, with same-hunt
    /// attribution and gaps capped at 120 seconds. Excludes idle/offline time.
    pub observed_seconds:u64,
    pub timed_kills:u64,
    pub timed_trainer_xp:u64,
    pub timed_pokemon_xp:u64,
    pub timed_trainer_receipts:u64,
    pub timed_pokemon_receipts:u64,
    pub combat_gold: u64,
    pub auto_sale_gold: u64,
    pub balls_used: BTreeMap<u64,u64>,
    pub drops: BTreeMap<String,u64>,
}
impl HistoryDay {
    fn from_value(value: &Value) -> Option<Self> {
        let day = value.get("day")?.as_u64()?;
        let hunt = value.get("hunt")?.as_str()?.to_owned();
        let n = |key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
        let balls_used = value.get("ballsUsed").and_then(Value::as_object)
            .map(|map| map.iter().filter_map(|(key,val)|
                Some((key.parse::<u64>().ok()?,val.as_u64()?))).collect())
            .unwrap_or_default();
        let drops = value.get("drops").and_then(Value::as_object)
            .map(|map| map.iter().filter_map(|(key,val)|
                Some((key.clone(),val.as_u64()?))).collect())
            .unwrap_or_default();
        Some(Self {day,hunt,kills:n("kills"),captures:n("captures"),
            shiny_seen:n("shinySeen"),shiny_captured:n("shinyCaptured"),
            trainer_xp:n("trainerXp"),pokemon_xp:n("pokemonXp"),
            observed_seconds:n("observedSeconds"),
            timed_kills:n("timedKills"),
            timed_trainer_xp:n("timedTrainerXp"),timed_pokemon_xp:n("timedPokemonXp"),
            timed_trainer_receipts:n("timedTrainerReceipts"),
            timed_pokemon_receipts:n("timedPokemonReceipts"),
            combat_gold:n("combatGold"),auto_sale_gold:n("autoSaleGold"),
            balls_used,drops})
    }
    fn from_event(&mut self, event: &Value) -> bool {
        let num=|field:&str|event.get(field).and_then(Value::as_u64).unwrap_or(0);
        match event.get("k").and_then(Value::as_str) {
            Some("morte") if event.get("quem").and_then(Value::as_str)==Some("selvagem")=>{
                self.kills=self.kills.saturating_add(1);
                self.trainer_xp=self.trainer_xp.saturating_add(num("xpTreinador"));
                self.pokemon_xp=self.pokemon_xp.saturating_add(num("xpPokemon"));
                self.combat_gold=self.combat_gold.saturating_add(num("ouro"));
                self.auto_sale_gold=self.auto_sale_gold.saturating_add(num("ouroVendaAuto"));
                if let Some(drops)=event.get("drops").and_then(Value::as_array) {
                    for item in drops {
                        let Some(name)=item.get("nome").and_then(Value::as_str)
                            .map(str::trim).filter(|s|!s.is_empty()) else{continue};
                        let count=item.get("ganho").and_then(Value::as_u64).filter(|n|*n>0)
                            .or_else(||item.get("qtd").and_then(Value::as_u64)).unwrap_or(0);
                        if count>0 {
                            let count_ref=self.drops.entry(name.to_owned()).or_default();
                            *count_ref=count_ref.saturating_add(count);
                        }
                    }
                }
                true
            }
            Some("bola")=>{
                if let Some(id)=event.get("ballId").and_then(Value::as_u64) {
                    let used=self.balls_used.entry(id).or_default();
                    *used=used.saturating_add(1);
                }
                let shiny=event.get("shiny").and_then(Value::as_bool)==Some(true);
                if shiny {self.shiny_seen=self.shiny_seen.saturating_add(1);}
                if event.get("sucesso").and_then(Value::as_bool)==Some(true) {
                    self.captures=self.captures.saturating_add(1);
                    if shiny {self.shiny_captured=self.shiny_captured.saturating_add(1);}
                }
                true
            }
            Some("fugiu") if event.get("shiny").and_then(Value::as_bool)==Some(true)=>{
                self.shiny_seen=self.shiny_seen.saturating_add(1);true
            }
            _ => false
        }
    }

    #[cfg(test)]
    fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("HistoryDay must serialize in tests")
    }
}
/// Model estimate captured when Hunt Atlas was actually open and authenticated.
#[derive(Clone,Debug,serde::Serialize)]
pub struct HistoryForecast {
    pub day:u64,pub hour:u64,pub hunt:String,
    #[serde(rename = "trainerH")]
    pub trainer_xp_h:f64,
    #[serde(rename = "pokemonH")]
    pub pokemon_xp_h:f64,
    #[serde(rename = "killsH")]
    pub kills_h:f64,
    pub calibrated:bool,
}
impl HistoryForecast {
    pub fn now(hunt:&str,trainer_xp_h:f64,pokemon_xp_h:f64,
        kills_h:f64,calibrated:bool)->Option<Self>{
        if hunt.is_empty()||hunt.len()>128
            ||[trainer_xp_h,pokemon_xp_h,kills_h].iter()
                .any(|v|!v.is_finite()||*v<0.0){return None}
        let hour=SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs()/3600;
        Some(Self{day:hour/24,hour,hunt:hunt.into(),
            trainer_xp_h,pokemon_xp_h,kills_h,calibrated})
    }
    fn from_value(raw:&Value)->Option<Self>{
        let hunt=raw.get("hunt")?.as_str()?.to_owned();
        if hunt.is_empty()||hunt.len()>128{return None}
        let day=raw.get("day")?.as_u64()?;
        let hour=raw.get("hour")?.as_u64()?;
        if hour/24!=day{return None}
        let trainer_xp_h=raw.get("trainerH")?.as_f64()?;
        let pokemon_xp_h=raw.get("pokemonH")?.as_f64()?;
        let kills_h=raw.get("killsH")?.as_f64()?;
        if [trainer_xp_h,pokemon_xp_h,kills_h].iter()
            .any(|n|!n.is_finite()||*n<0.0){return None}
        Some(Self{day,hour,hunt,trainer_xp_h,pokemon_xp_h,kills_h,
            calibrated:raw.get("calibrated").and_then(Value::as_bool).unwrap_or(false)})
    }

    #[cfg(test)]
    fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("HistoryForecast must serialize in tests")
    }
}
// Borrow the derived history instead of cloning every row into serde_json::Value.
// This preserves the v1 on-disk field names without another JSON tree.
#[derive(serde::Serialize)]
struct HistoryDocument<'a> {
    version: u8,
    nick: &'a str,
    lifetime: &'a HistoryDay,
    days: &'a [HistoryDay],
    forecasts: &'a [HistoryForecast],
}
fn serialize_history(snapshot:&HistorySnapshot,nick:&str)->Result<String,String>{
    serde_json::to_string(&HistoryDocument{
        version:1,nick,lifetime:&snapshot.lifetime,
        days:&snapshot.days,forecasts:&snapshot.forecasts
    }).map_err(|error|format!("History serialization: {error}"))
}
#[derive(Clone,Debug,Default)]
pub struct HistorySnapshot {
    pub owner: String,
    pub days: Vec<HistoryDay>,
    pub forecasts:Vec<HistoryForecast>,
    /// Never pruned: lifetime gameplay totals survive daily detail retention.
    pub lifetime: HistoryDay,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
}
impl HistorySnapshot {
    fn from_value(value: &Value, nick: &str) -> Result<Self,String> {
        if value.get("version").and_then(Value::as_u64)!=Some(1)
            || value.get("nick").and_then(Value::as_str)!=Some(nick) {
            return Err("History version or account identity mismatch".into());
        }
        let Some(days)=value.get("days").and_then(Value::as_array) else {
            return Err("History data missing days".into());
        };
        if days.len()>10000{return Err("History contains too many rows".into());}
        let mut records=BTreeMap::new();
        for raw in days {
            let Some(day)=HistoryDay::from_value(raw) else{
                return Err("History data contains an invalid day".into())
            };
            if day.hunt.len()>128 {return Err("History hunt label too long".into());}
            records.insert((day.day,day.hunt.clone()),day);
        }
        let days: Vec<HistoryDay>=records.into_values().collect();
        let lifetime=value.get("lifetime").and_then(HistoryDay::from_value)
            .unwrap_or_else(||{
                let mut total=HistoryDay::default();
                for day in &days { total.accumulate(day); }
                total
            });
        let forecasts=value.get("forecasts").and_then(Value::as_array)
            .filter(|rows|rows.len()<=5000)
            .map(|rows|rows.iter().filter_map(HistoryForecast::from_value)
                .collect::<Vec<_>>()).unwrap_or_default();
        Ok(Self{owner:nick.to_owned(),days,lifetime,forecasts,..Default::default()})
    }
    fn retain_recent(&mut self, now_day:u64) {
        self.days.retain(|day|
            day.day<=now_day && now_day.saturating_sub(day.day)<MAX_DAYS);
        self.forecasts.retain(|item|item.day<=now_day &&
            now_day.saturating_sub(item.day)<MAX_DAYS);
    }
    pub fn totals(&self) -> HistoryDay {
        self.lifetime.clone()
    }

    #[cfg(test)]
    fn to_value(&self, nick: &str) -> Value {
        let encoded=serialize_history(self,nick).expect("HistorySnapshot must serialize in tests");
        serde_json::from_str(&encoded).expect("HistorySnapshot test JSON must parse")
    }
}
impl HistoryDay {
    fn accumulate(&mut self,other:&HistoryDay) {
        let total=self;
        let day=other;
        {
            for (dst,src) in [
                (&mut total.kills,day.kills),
                (&mut total.captures,day.captures),
                (&mut total.shiny_seen,day.shiny_seen),
                (&mut total.shiny_captured,day.shiny_captured),
                (&mut total.trainer_xp,day.trainer_xp),
                (&mut total.pokemon_xp,day.pokemon_xp),
                (&mut total.observed_seconds,day.observed_seconds),
                (&mut total.timed_kills,day.timed_kills),
                (&mut total.timed_trainer_xp,day.timed_trainer_xp),
                (&mut total.timed_pokemon_xp,day.timed_pokemon_xp),
                (&mut total.timed_trainer_receipts,day.timed_trainer_receipts),
                (&mut total.timed_pokemon_receipts,day.timed_pokemon_receipts),
                (&mut total.combat_gold,day.combat_gold),
                (&mut total.auto_sale_gold,day.auto_sale_gold)
            ] {*dst=dst.saturating_add(src);}
            for (id,qty) in &day.balls_used {
                let value=total.balls_used.entry(*id).or_default();
                *value=value.saturating_add(*qty);
            }
            for (item,qty) in &day.drops {
                let value=total.drops.entry(item.clone()).or_default();
                *value=value.saturating_add(*qty);
            }
        }
    }
}
fn unix_day() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs()/86_400
}
fn history_path(profile:GameProfile,nick:&str)->Result<PathBuf,String> {
    // FNV-1a is a stable filename namespace, not a security primitive.
    // File contents also verify identity before any old totals are loaded.
    let mut hash=0xcbf29ce484222325u64;
    let normalized_nick=nick.to_lowercase();
    for b in normalized_nick.as_bytes() {
        hash^=*b as u64;hash=hash.wrapping_mul(0x100000001b3);
    }
    Ok(data_root()?.join("History").join(
        format!("{}-{hash:016x}.json",profile.name())))
}
fn write_atomic(path:&PathBuf,content:&str)->Result<(),String>{
    if let Some(parent)=path.parent(){
        fs::create_dir_all(parent).map_err(|error|format!("History directory: {error}"))?;
    }
    let tmp=path.with_extension("json.tmp");
    fs::write(&tmp,content).map_err(|error|format!("History staging: {error}"))?;
    fs::rename(&tmp,path).map_err(|error|format!("History save: {error}"))
}
/// Read-only history for stopped accounts. Never starts a game socket,
/// Firefox. If the profile was reused, show its latest saved
/// identity rather than mixing unrelated accounts' totals.
pub fn load_offline(profile:GameProfile)->HistorySnapshot {
    let mut fallback=HistorySnapshot::default();
    let folder=match data_root() {
        Ok(root)=>root.join("History"),
        Err(error)=>{fallback.load_error=Some(error);return fallback;}
    };
    let Ok(entries)=fs::read_dir(folder) else{return fallback};
    let prefix=format!("{}-",profile.name());
    let mut files=entries.filter_map(Result::ok)
        .filter(|entry|{
            let name=entry.file_name();
            let Some(name)=name.to_str() else{return false};
            name.starts_with(&prefix) && name.ends_with(".json")
        })
        .filter_map(|entry|{
            let meta=entry.metadata().ok()?;
            if meta.len()>8*1024*1024 {return None;}
            let modified=meta.modified().ok()?;
            Some((modified,entry.path()))
        }).collect::<Vec<_>>();
    files.sort_by(|a,b|b.0.cmp(&a.0));
    let Some((_,path))=files.first() else{return fallback};
    let raw=match fs::read_to_string(path) {
        Ok(raw)=>raw,
        Err(error)=>{
            fallback.load_error=Some(format!("Saved history read: {error}"));
            return fallback;
        }
    };
    let parsed=serde_json::from_str::<Value>(&raw)
        .map_err(|_|"Saved history JSON malformed".to_owned())
        .and_then(|json|{
            let Some(nick)=json.get("nick").and_then(Value::as_str) else {
                return Err("Saved history has no account identity".into())
            };
            HistorySnapshot::from_value(&json,nick)
        });
    match parsed {
        Ok(mut snapshot)=>{snapshot.retain_recent(unix_day());snapshot},
        Err(error)=>{fallback.load_error=Some(error);fallback}
    }
}

pub struct HistoryRecorder {
    // A single authoritative snapshot; socket updates mutate it in place.
    // No full-history clone is needed to publish current counters to the UI.
    pub shared: Arc<Mutex<HistorySnapshot>>,
    last_hunt: Option<String>,
    last_kill:Option<(String,Instant)>,
    last_pruned_day:u64,
    dirty:bool,
    last_flush:Instant,
    // Report actual socket-worker waits for this mutex, not the time the
    // background writer spent cloning. Throttle warnings per session.
    last_lock_warning:Option<Instant>,
    writer:Option<Arc<WriteSignal>>,
    _lease:Arc<HistoryLease>,
}
impl HistoryRecorder {
    pub fn new(profile:GameProfile,nick:String)->Self {
        let lease=Arc::new(profile_history_gate(profile).acquire());
        let path=history_path(profile,&nick);
        let mut snapshot=HistorySnapshot::default();
        let path=match path {
            Ok(path)=>{
                match fs::read_to_string(&path) {
                    Ok(raw)=>{
                        match serde_json::from_str::<Value>(&raw)
                            .map_err(|_|"History JSON malformed".to_owned())
                            .and_then(|value|HistorySnapshot::from_value(&value,&nick)){
                            Ok(data)=>snapshot=data,
                            Err(error)=>snapshot.load_error=Some(error)
                        }
                    }
                    Err(error) if error.kind()==std::io::ErrorKind::NotFound => {}
                    Err(error)=>snapshot.load_error=Some(format!("History read: {error}"))
                }
                Some(path)
            },
            Err(error)=>{snapshot.load_error=Some(error);None}
        };
        snapshot.owner=nick.clone();
        let today=unix_day();
        snapshot.retain_recent(today);
        let shared=Arc::new(Mutex::new(snapshot));
        let writer=path.and_then(|path| {
            match start_history_writer(Arc::clone(&shared),path,nick.clone()) {
                Ok(writer)=>{
                    *writer.lease.lock().unwrap_or_else(|e|e.into_inner())=
                        Some(Arc::clone(&lease));
                    Some(writer)
                },
                Err(error)=>{
                    if let Ok(mut snapshot)=shared.lock() {
                        snapshot.save_error=Some(error);
                    }
                    None
                }
            }
        });
        Self {shared,last_hunt:None,last_kill:None,last_pruned_day:today,
            dirty:false,last_flush:Instant::now(),
            last_lock_warning:None,writer,_lease:lease}
    }
    fn report_lock_wait(&mut self,waited:Duration) {
        if waited>=Duration::from_millis(5) &&
            self.last_lock_warning.is_none_or(|at|
                at.elapsed()>=Duration::from_secs(60)) {
            self.last_lock_warning=Some(Instant::now());
            crate::logging::warn(&format!(
                "Native history socket worker waited {} ms for snapshot lock",
                waited.as_millis()));
        }
    }
    pub fn record_forecast(&mut self,model:HistoryForecast){
        let day=model.day;
        if day!=unix_day(){return}
        let started=Instant::now();
        let Ok(mut snapshot)=self.shared.lock() else{return};
        let waited=started.elapsed();
        if let Some(previous)=snapshot.forecasts.iter_mut()
            .find(|row|row.hour==model.hour && row.hunt==model.hunt){
            *previous=model;
        }else{
            snapshot.forecasts.push(model);
        }
        // Capturing only one model per hour/hunt prevents redraw-generated
        // history inflation. The async history writer owns persistence.
        if self.last_pruned_day!=day {
            snapshot.retain_recent(day);
            self.last_pruned_day=day;
        }
        // Never grow the 90-day journal beyond what the loader accepts.
        if snapshot.forecasts.len()>5000 {
            snapshot.forecasts.sort_by_key(|row|row.hour);
            let extra=snapshot.forecasts.len()-5000;
            snapshot.forecasts.drain(0..extra);
        }
        self.dirty=true;
        drop(snapshot);
        self.report_lock_wait(waited);
    }
    #[cfg(test)]
    pub fn ingest(&mut self,raw:&str,hunt:Option<&str>) {
        let Ok(frame)=serde_json::from_str::<Value>(raw) else{return};
        self.ingest_value(&frame, hunt);
    }
    pub fn ingest_value(&mut self, frame:&Value, hunt:Option<&str>) {
        let kind=frame.get("t").and_then(Value::as_str);
        if kind==Some("welcome") { self.last_kill=None; }
        if matches!(kind,Some("welcome"|"estado")) {
            if let Some(slug)=frame.get("estado").and_then(|state|state.get("huntSlug"))
                .and_then(Value::as_str).filter(|s|!s.is_empty() && s.len()<=128){
                if self.last_hunt.as_deref()!=Some(slug){
                    self.last_kill=None;
                }
                self.last_hunt=Some(slug.to_owned());
            }
            return;
        }
        if kind!=Some("batalha"){return}
        let Some(events)=frame.get("ev").and_then(Value::as_array) else{return};
        let day=unix_day();
        let started=Instant::now();
        let Ok(mut guard)=self.shared.lock() else{return};
        let waited=started.elapsed();
        // Work through a normal &mut reference so distinct fields (daily
        // counters and lifetime totals) can be updated together safely.
        let snapshot:&mut HistorySnapshot=&mut *guard;
        let mut slug=self.last_hunt.clone()
            .or_else(||hunt.filter(|s|s.len()<=128).map(str::to_owned))
            .unwrap_or_else(||"unknown".into());
        let mut index=snapshot.days.iter().position(|row|
            row.day==day && row.hunt==slug);
        for event in events {
            // Attribute server events to the hunt active at that exact point
            // in a batch; the final HuntIntel state may already be switched.
            if event.get("k").and_then(Value::as_str)==Some("hunt") {
                if let Some(next)=event.get("slug").and_then(Value::as_str)
                    .filter(|s|!s.is_empty() && s.len()<=128) {
                    slug=next.to_owned();
                    self.last_kill=None;
                    self.last_hunt=Some(slug.clone());
                    index=snapshot.days.iter().position(|row|
                        row.day==day && row.hunt==slug);
                }
                continue;
            }
            if !matches!(event.get("k").and_then(Value::as_str),
                Some("morte"|"bola"|"fugiu")) { continue; }
            let slot=*index.get_or_insert_with(||{
                snapshot.days.push(HistoryDay{day,hunt:slug.clone(),..Default::default()});
                snapshot.days.len()-1
            });
            if snapshot.days[slot].from_event(event) {
                snapshot.lifetime.from_event(event);
                if event.get("k").and_then(Value::as_str)==Some("morte")
                    && event.get("quem").and_then(Value::as_str)==Some("selvagem"){
                    // Only count elapsed monotonic time since an earlier kill of
                    // this same hunt. Do not turn 8h of AFK into 8h of combat.
                    let now=Instant::now();
                    if let Some((previous,at))=self.last_kill.as_ref(){
                        let seconds=now.saturating_duration_since(*at).as_secs();
                        if previous==&slug && (1..=120).contains(&seconds){
                            let xp=event.get("xpTreinador").and_then(Value::as_u64);
                            let pk=event.get("xpPokemon").and_then(Value::as_u64);
                            for target in [&mut snapshot.days[slot],&mut snapshot.lifetime]{
                                target.observed_seconds=target.observed_seconds.saturating_add(seconds);
                                target.timed_kills=target.timed_kills.saturating_add(1);
                                if let Some(v)=xp{
                                    target.timed_trainer_xp=target.timed_trainer_xp.saturating_add(v);
                                    target.timed_trainer_receipts=target.timed_trainer_receipts.saturating_add(1);
                                }
                                if let Some(v)=pk{
                                    target.timed_pokemon_xp=target.timed_pokemon_xp.saturating_add(v);
                                    target.timed_pokemon_receipts=target.timed_pokemon_receipts.saturating_add(1);
                                }
                            }
                        }
                    }
                    self.last_kill=Some((slug.clone(),now));
                }
                self.dirty=true;
            }
        }
        self.last_hunt=Some(slug);
        // Pruning scans the retained journal: run once per UTC day, not on
        // every battle receipt while the game socket is time-sensitive.
        if self.last_pruned_day!=day {
            snapshot.retain_recent(day);
            self.last_pruned_day=day;
        }
        drop(guard);
        self.report_lock_wait(waited);
    }
    pub fn maybe_flush(&mut self,force:bool) {
        if !force && self.last_flush.elapsed()<SAVE_INTERVAL {return}
        let retry=self.writer.as_ref().is_some_and(|writer|writer.retry_due());
        if !self.dirty && !retry {
            // Disconnect/Stop never queues an unchanged full document.
            // Outstanding writes finish on the detached history writer.
            self.last_flush=Instant::now();
            return;
        }
        if let Some(writer)=self.writer.as_ref() {
            match writer.request() {
                Ok(_sequence)=>{
                    // Even a forced final flush is only a handoff. The socket
                    // worker must never wait for a filesystem acknowledgement.
                    // The detached writer retains this profile's file lease
                    // until its last requested generation has been written.
                    self.dirty=false;
                }
                Err(error)=>{
                    if let Ok(mut snapshot)=self.shared.try_lock() {
                        snapshot.save_error=Some(error);
                    }
                }
            }
        }
        self.last_flush=Instant::now();
    }
}
impl Drop for HistoryRecorder {
    fn drop(&mut self){
        self.maybe_flush(true);
        // Do not join on the time-sensitive game socket worker. The writer
        // keeps a separate profile lease until its final I/O has finished.
        if let Some(writer)=self.writer.as_ref(){writer.stop();}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_hunt_updates_from_welcome_and_ordered_battle_events() {
        // Keep unit tests entirely in memory; never touch a user's
        // production history directory.
        let mut history=HistoryRecorder{
            shared:Arc::new(Mutex::new(HistorySnapshot::default())),
            last_hunt:None,last_kill:None,last_pruned_day:unix_day(),dirty:false,
            last_flush:Instant::now(),last_lock_warning:None,writer:None,
            _lease:Arc::new(Arc::new(HistoryGate::default()).acquire())
        };
        history.ingest(r#"{"t":"welcome","estado":{"huntSlug":"route-a"}}"#,None);
        history.ingest(r#"{"t":"batalha","ev":[
            {"k":"morte","quem":"selvagem","xpTreinador":5},
            {"k":"hunt","slug":"route-b"},
            {"k":"morte","quem":"selvagem","xpTreinador":10}
        ]}"#,Some("route-b"));
        let snapshot=history.shared.lock().unwrap();
        assert_eq!(snapshot.days.iter().find(|d|d.hunt=="route-a")
            .unwrap().trainer_xp,5);
        assert_eq!(snapshot.days.iter().find(|d|d.hunt=="route-b")
            .unwrap().trainer_xp,10);
    }
    #[test]
    fn replacement_history_recorder_waits_for_previous_owner() {
        let gate=Arc::new(HistoryGate::default());
        let first=gate.acquire();
        let (ready_tx,ready_rx)=std::sync::mpsc::channel();
        let (acquired_tx,acquired_rx)=std::sync::mpsc::channel();
        let next=Arc::clone(&gate);
        let replacement=thread::spawn(move || {
            ready_tx.send(()).unwrap();
            let _lease=next.acquire();
            acquired_tx.send(()).unwrap();
        });
        ready_rx.recv().unwrap();
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "replacement must not read while old session owns the file");
        drop(first);
        acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        replacement.join().unwrap();
    }
    #[test]
    fn delayed_writer_drains_before_replacement_reads_history() {
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(
            format!("moth-history-handoff-{}-{id}.json",std::process::id()));
        let gate=Arc::new(HistoryGate::default());
        let old_lease=gate.acquire();
        let shared=Arc::new(Mutex::new(HistorySnapshot {
            owner:"Test".into(),..Default::default()
        }));
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .unwrap();
        // Prevent the in-flight writer from copying its snapshot until after
        // the replacement has tried to acquire the same profile's lease.
        let mut locked=shared.lock().unwrap();
        locked.lifetime.kills=42;
        writer.request().unwrap();
        let next_gate=Arc::clone(&gate);
        let next_path=path.clone();
        let (read_tx,read_rx)=std::sync::mpsc::channel();
        let replacement=thread::spawn(move || {
            let _lease=next_gate.acquire();
            read_tx.send(fs::read_to_string(next_path).unwrap()).unwrap();
        });
        let stopping=thread::spawn(move || {
            writer.stop_and_join();
            drop(old_lease);
        });
        assert!(read_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(locked);
        stopping.join().unwrap();
        let raw=read_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        replacement.join().unwrap();
        let recovered=HistorySnapshot::from_value(
            &serde_json::from_str::<Value>(&raw).unwrap(),"Test").unwrap();
        assert_eq!(recovered.lifetime.kills,42);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn dropping_a_recorder_is_bounded_without_releasing_the_writer_lease() {
        use std::sync::mpsc;
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(format!(
            "moth-history-detached-writer-{}-{id}.json",std::process::id()));
        let gate=Arc::new(HistoryGate::default());
        let lease=Arc::new(gate.acquire());
        let shared=Arc::new(Mutex::new(HistorySnapshot {
            owner:"Test".into(),..Default::default()
        }));
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .unwrap();
        *writer.lease.lock().unwrap()=Some(Arc::clone(&lease));
        let mut locked=shared.lock().unwrap();
        locked.lifetime.kills=23;
        writer.request().unwrap();
        let recorder=HistoryRecorder {
            shared:Arc::clone(&shared),last_hunt:None,last_kill:None,
            last_pruned_day:unix_day(),dirty:false,last_flush:Instant::now(),
            last_lock_warning:None,writer:Some(Arc::clone(&writer)),_lease:lease,
        };
        let (dropped_tx,dropped_rx)=mpsc::channel();
        let dropping=thread::spawn(move || {
            drop(recorder);
            dropped_tx.send(()).unwrap();
        });
        dropped_rx.recv_timeout(Duration::from_millis(250))
            .expect("recorder Drop must not wait on the blocked history writer");
        dropping.join().unwrap();
        let (acquired_tx,acquired_rx)=mpsc::channel();
        let next_gate=Arc::clone(&gate);
        let next=thread::spawn(move || {
            let _lease=next_gate.acquire();
            acquired_tx.send(()).unwrap();
        });
        drop(writer);
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "an unfinished writer must continue to own the history file");
        drop(locked);
        acquired_rx.recv_timeout(Duration::from_secs(2))
            .expect("replacement must resume after the writer finishes");
        next.join().unwrap();
        let raw=fs::read_to_string(&path).unwrap();
        let saved=HistorySnapshot::from_value(
            &serde_json::from_str::<Value>(&raw).unwrap(),"Test").unwrap();
        assert_eq!(saved.lifetime.kills,23);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn pending_history_saves_coalesce_without_queuing_full_snapshots() {
        let signal=WriteSignal::default();
        for _ in 0..1_000 {signal.request().unwrap();}
        let state=signal.state.lock().unwrap();
        assert!(state.pending);
        assert_eq!(state.requested,1_000);
        assert_eq!(state.completed,0);
        // There is only one pending flag, not a queue of 1,000 documents.
    }
    #[test]
    fn final_history_save_writes_latest_counters_before_acknowledgement() {
        // Real writer exercising the persisted legacy format, but only in a
        // unique temporary test path (never a player's data directory).
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(
            format!("moth-history-writer-test-{}-{id}.json",std::process::id()));
        let shared=Arc::new(Mutex::new(HistorySnapshot {
            owner:"Test".into(),..Default::default()
        }));
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .expect("start temporary history writer");
        // Keep the live snapshot locked while requests flood in. The
        // writer can only observe the newest state after the guard releases.
        let mut snapshot=shared.lock().unwrap();
        snapshot.lifetime.kills=1;
        for _ in 0..100 {writer.request().unwrap();}
        snapshot.lifetime.kills=17;
        let final_generation=writer.request().unwrap();
        drop(snapshot);
        writer.wait_written(final_generation,FINAL_WRITE_TIMEOUT)
            .expect("final snapshot acknowledged");
        let raw=fs::read_to_string(&path).unwrap();
        let recovered=HistorySnapshot::from_value(
            &serde_json::from_str::<Value>(&raw).unwrap(),"Test").unwrap();
        assert_eq!(recovered.lifetime.kills,17);
        writer.stop_and_join();
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn forced_dirty_flush_hands_off_without_waiting_for_the_writer() {
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(format!(
            "moth-history-nonblocking-flush-{}-{id}.json",std::process::id()));
        let shared=Arc::new(Mutex::new(HistorySnapshot{
            owner:"Test".into(),..Default::default()
        }));
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .unwrap();
        let mut held=shared.lock().unwrap();
        held.lifetime.kills=31;
        let mut recorder=HistoryRecorder {
            shared:Arc::clone(&shared),last_hunt:None,last_kill:None,
            last_pruned_day:unix_day(),dirty:true,
            last_flush:Instant::now()-SAVE_INTERVAL,
            last_lock_warning:None,writer:Some(Arc::clone(&writer)),
            _lease:Arc::new(Arc::new(HistoryGate::default()).acquire()),
        };
        let started=Instant::now();
        recorder.maybe_flush(true);
        assert!(started.elapsed()<Duration::from_millis(250),
            "force must only signal the writer, not wait for the locked snapshot");
        assert_eq!(writer.in_flight(),Some(1));
        drop(held);
        writer.wait_written(1,FINAL_WRITE_TIMEOUT).unwrap();
        drop(recorder);
        writer.stop_and_join();
        let raw=fs::read_to_string(&path).unwrap();
        let recovered=HistorySnapshot::from_value(
            &serde_json::from_str::<Value>(&raw).unwrap(),"Test").unwrap();
        assert_eq!(recovered.lifetime.kills,31);
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn forced_clean_flush_does_not_queue_duplicate_history_snapshots() {
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(
            format!("moth-history-forced-flush-{}-{id}.json",std::process::id()));
        let shared=Arc::new(Mutex::new(HistorySnapshot{
            owner:"Test".into(),..Default::default()
        }));
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .unwrap();
        let mut recorder=HistoryRecorder {
            shared,last_hunt:None,last_kill:None,last_pruned_day:unix_day(),
            dirty:false,last_flush:Instant::now()-SAVE_INTERVAL,
            last_lock_warning:None,writer:Some(Arc::clone(&writer)),
            _lease:Arc::new(Arc::new(HistoryGate::default()).acquire()),
        };
        recorder.maybe_flush(true);
        assert_eq!(writer.state.lock().unwrap().requested,0,
            "an untouched recorder should not create a history file");
        recorder.dirty=true;
        recorder.maybe_flush(true);
        recorder.maybe_flush(true);
        assert_eq!(writer.state.lock().unwrap().requested,1,
            "disconnect must not duplicate the already queued final save");
        writer.wait_written(1,FINAL_WRITE_TIMEOUT).unwrap();
        drop(recorder);
        assert_eq!(writer.state.lock().unwrap().requested,1);
        writer.stop_and_join();
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn failed_history_write_retries_without_new_battle_receipts() {
        let id=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let blocked=std::env::temp_dir().join(
            format!("moth-history-retry-{}-{id}",std::process::id()));
        let path=blocked.join("history.json");
        fs::write(&blocked,"not a directory").unwrap();
        let shared=Arc::new(Mutex::new(HistorySnapshot {
            owner:"Test".into(),..Default::default()
        }));
        shared.lock().unwrap().lifetime.kills=17;
        let writer=start_history_writer(Arc::clone(&shared),path.clone(),"Test".into())
            .expect("start temporary history writer");
        let mut recorder=HistoryRecorder {
            shared,last_hunt:None,last_kill:None,last_pruned_day:unix_day(),
            dirty:true,last_flush:Instant::now()-SAVE_INTERVAL,
            last_lock_warning:None,writer:Some(Arc::clone(&writer)),
            _lease:Arc::new(Arc::new(HistoryGate::default()).acquire())
        };
        recorder.maybe_flush(false);
        let initial=writer.state.lock().unwrap().requested;
        assert!(writer.wait_written(initial,FINAL_WRITE_TIMEOUT).is_err());
        assert!(!recorder.dirty);
        assert!(writer.retry_due());
        fs::remove_file(&blocked).unwrap();
        fs::create_dir(&blocked).unwrap();
        recorder.last_flush=Instant::now()-SAVE_INTERVAL;
        recorder.maybe_flush(false);
        let retry=writer.state.lock().unwrap().requested;
        assert!(retry>initial,"failed write must schedule a retry with no new events");
        writer.wait_written(retry,FINAL_WRITE_TIMEOUT).unwrap();
        assert!(!writer.retry_due());
        drop(recorder);
        let saved=fs::read_to_string(&path).unwrap();
        let recovered=HistorySnapshot::from_value(
            &serde_json::from_str::<Value>(&saved).unwrap(),"Test").unwrap();
        assert_eq!(recovered.lifetime.kills,17);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&blocked).unwrap();
    }
    #[test]
    fn background_writer_serialization_matches_existing_disk_schema() {
        let mut snapshot=HistorySnapshot::default();
        snapshot.days.push(HistoryDay{
            day:123,hunt:"route-a".into(),kills:8,captures:2,
            balls_used:BTreeMap::from([(4,3)]),
            drops:BTreeMap::from([("Heart Stone".into(),1)]),
            ..Default::default()
        });
        snapshot.forecasts.push(HistoryForecast{
            day:123,hour:123*24,hunt:"route-a".into(),
            trainer_xp_h:1200.0,pokemon_xp_h:800.0,
            kills_h:50.0,calibrated:true
        });
        let encoded=serialize_history(&snapshot,"Trainer").unwrap();
        let decoded:Value=serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded,snapshot.to_value("Trainer"));
    }
    #[test]
    fn roundtrip_daily_gameplay_totals_excluding_raw_tokens() {
        let day=unix_day();
        let mut snapshot=HistorySnapshot::default();
        let mut sample=HistoryDay{day,hunt:"route-1".into(),..Default::default()};
        assert!(sample.from_event(&json!({"k":"morte","quem":"selvagem",
            "xpTreinador":10,"xpPokemon":20,"ouro":3,
            "drops":[{"nome":"Heart Stone","qtd":2}],"token":"secret"})));
        sample.from_event(&json!({"k":"bola","ballId":4,"sucesso":true,"shiny":true}));
        snapshot.lifetime=sample.clone();
        snapshot.days.push(sample);
        let raw=snapshot.to_value("Trainer").to_string();
        assert!(!raw.contains("secret"));
        let recovered=HistorySnapshot::from_value(&serde_json::from_str::<Value>(&raw).unwrap(),"Trainer").unwrap();
        assert_eq!(recovered.totals().trainer_xp,10);
        assert_eq!(recovered.totals().pokemon_xp,20);
        assert_eq!(recovered.totals().captures,1);
        assert_eq!(recovered.totals().drops["Heart Stone"],2);
        assert!(HistorySnapshot::from_value(&serde_json::from_str::<Value>(&raw).unwrap(),"Other").is_err());
    }
    #[test]
    fn archived_forecast_roundtrips_without_changing_legacy_history_version(){
        let row=HistoryForecast{
            day:123,hour:123*24,hunt:"route-a".into(),
            trainer_xp_h:2000.0,pokemon_xp_h:900.0,
            kills_h:120.0,calibrated:true,
        };
        let back=HistoryForecast::from_value(&row.to_value()).unwrap();
        assert_eq!(back.trainer_xp_h,2000.0);
        assert!(back.calibrated);
        let mut history=HistorySnapshot::default();
        history.forecasts.push(row);
        let revived=HistorySnapshot::from_value(&history.to_value("player"),"player").unwrap();
        assert_eq!(revived.forecasts.len(),1);
    }
    #[test]
    fn old_history_with_no_time_observations_does_not_claim_a_rate(){
        let old=serde_json::json!({"day":20261016,"hunt":"route-a",
            "kills":40,"trainerXp":120000,"pokemonXp":90000});
        let parsed=HistoryDay::from_value(&old).unwrap();
        assert_eq!(parsed.observed_seconds,0);
        assert_eq!(crate::native_atlas_models::retrospective_rate(
            parsed.timed_trainer_xp,parsed.observed_seconds),None);
    }
    #[test]
    fn observed_combat_duration_survives_history_serialization(){
        let row=HistoryDay{day:123,hunt:"route-a".into(),
            observed_seconds:3600,timed_kills:100,
            timed_trainer_xp:13000,timed_pokemon_xp:9000,
            timed_trainer_receipts:100,timed_pokemon_receipts:100,
            ..Default::default()};
        let decoded=HistoryDay::from_value(&row.to_value()).unwrap();
        assert_eq!(decoded.observed_seconds,3600);
        assert_eq!(decoded.timed_pokemon_xp,9000);
    }
    #[test]
    fn retained_lifetime_totals_outlive_daily_detail_pruning() {
        let mut snapshot=HistorySnapshot::default();
        snapshot.lifetime.kills=999;
        snapshot.days.push(HistoryDay{day:10,hunt:"old".into(),kills:999,
            ..Default::default()});
        snapshot.retain_recent(200);
        assert!(snapshot.days.is_empty());
        assert_eq!(snapshot.totals().kills,999);
        let raw=snapshot.to_value("Trainer");
        let recovered=HistorySnapshot::from_value(&raw,"Trainer").unwrap();
        assert_eq!(recovered.totals().kills,999);
    }
    #[test]
    fn retains_only_recent_history() {
        let mut s=HistorySnapshot::default();
        s.days.push(HistoryDay{day:100,hunt:"old".into(),..Default::default()});
        s.days.push(HistoryDay{day:199,hunt:"new".into(),..Default::default()});
        s.retain_recent(200);
        assert_eq!(s.days.len(),1);
    }
}
