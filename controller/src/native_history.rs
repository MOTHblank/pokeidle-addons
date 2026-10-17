//! Bounded, restart-persistent native battle history, independent of userscripts.
//! Only derived gameplay counters are saved; never raw frames, auth, cookies,
//! WebSocket URLs, Twitch identities or mutable credentials.
use crate::config::{data_root, GameProfile};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_DAYS: u64 = 90;
const SAVE_INTERVAL: Duration = Duration::from_secs(5);
const FINAL_WRITE_TIMEOUT: Duration = Duration::from_secs(2);
type WriteTask = (PathBuf,String,Option<mpsc::Sender<()>>);
#[derive(Clone, Debug, Default)]
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
    fn to_value(&self) -> Value {
        json!({
            "day":self.day,"hunt":self.hunt,
            "kills":self.kills,"captures":self.captures,
            "shinySeen":self.shiny_seen,"shinyCaptured":self.shiny_captured,
            "trainerXp":self.trainer_xp,"pokemonXp":self.pokemon_xp,
            "observedSeconds":self.observed_seconds,
            "timedKills":self.timed_kills,
            "timedTrainerXp":self.timed_trainer_xp,
            "timedPokemonXp":self.timed_pokemon_xp,
            "timedTrainerReceipts":self.timed_trainer_receipts,
            "timedPokemonReceipts":self.timed_pokemon_receipts,
            "combatGold":self.combat_gold,"autoSaleGold":self.auto_sale_gold,
            "ballsUsed":self.balls_used,"drops":self.drops
        })
    }
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
}
/// Model estimate captured when Hunt Atlas was actually open and authenticated.
#[derive(Clone,Debug)]
pub struct HistoryForecast {
    pub day:u64,pub hour:u64,pub hunt:String,
    pub trainer_xp_h:f64,pub pokemon_xp_h:f64,pub kills_h:f64,
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
    fn to_value(&self)->Value{
        json!({"day":self.day,"hour":self.hour,"hunt":self.hunt,
            "trainerH":self.trainer_xp_h,"pokemonH":self.pokemon_xp_h,
            "killsH":self.kills_h,"calibrated":self.calibrated})
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
    fn to_value(&self, nick: &str) -> Value {
        json!({"version":1,"nick":nick,
            "lifetime":self.lifetime.to_value(),
            "days":self.days.iter().map(HistoryDay::to_value).collect::<Vec<_>>(),
            "forecasts":self.forecasts.iter().map(HistoryForecast::to_value)
                .collect::<Vec<_>>()})
    }
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
/// Firefox or Twitch. If the profile was reused, show its latest saved
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
    pub shared: Arc<Mutex<HistorySnapshot>>,
    snapshot: HistorySnapshot,
    nick: String,
    last_hunt: Option<String>,
    last_kill:Option<(String,Instant)>,
    dirty:bool,
    needs_publish:bool,
    last_publish:Instant,
    last_flush:Instant,
    writer:mpsc::Sender<WriteTask>,
    path:Option<PathBuf>,
}
impl HistoryRecorder {
    pub fn new(profile:GameProfile,nick:String)->Self {
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
        snapshot.retain_recent(unix_day());
        let shared=Arc::new(Mutex::new(snapshot.clone()));
        let (writer_tx,writer_rx)=mpsc::channel::<WriteTask>();
        let writer_shared=Arc::clone(&shared);
        thread::Builder::new().name("moth-native-history".into())
            .spawn(move ||{
                while let Ok((path,raw,ack))=writer_rx.recv(){
                    let result=write_atomic(&path,&raw);
                    if let Ok(mut snapshot)=writer_shared.lock() {
                        snapshot.save_error=result.as_ref().err().cloned();
                    }
                    if let Err(error)=result {
                        crate::logging::warn(&format!("Native history persistence: {error}"));
                    }
                    if let Some(ack)=ack {let _=ack.send(());}
                }
            }).ok();
        Self {shared,snapshot,nick,last_hunt:None,last_kill:None,
            dirty:false,needs_publish:false,
            last_publish:Instant::now(),last_flush:Instant::now(),
            writer:writer_tx,path}
    }
    pub fn record_forecast(&mut self,model:HistoryForecast){
        if model.day!=unix_day(){return}
        if let Some(previous)=self.snapshot.forecasts.iter_mut()
            .find(|row|row.hour==model.hour && row.hunt==model.hunt){
            *previous=model;
        }else{
            self.snapshot.forecasts.push(model);
        }
        // Capturing only one model per hour/hunt prevents redraw-generated
        // history inflation. The async history writer owns persistence.
        self.snapshot.retain_recent(unix_day());
        // Never grow the 90-day journal beyond what the loader accepts.
        if self.snapshot.forecasts.len()>5000 {
            self.snapshot.forecasts.sort_by_key(|row|row.hour);
            let extra=self.snapshot.forecasts.len()-5000;
            self.snapshot.forecasts.drain(0..extra);
        }
        self.dirty=true;
        self.needs_publish=true;
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
        let mut slug=self.last_hunt.clone()
            .or_else(||hunt.filter(|s|s.len()<=128).map(str::to_owned))
            .unwrap_or_else(||"unknown".into());
        let mut index=self.snapshot.days.iter().position(|row|
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
                    index=self.snapshot.days.iter().position(|row|
                        row.day==day && row.hunt==slug);
                }
                continue;
            }
            if !matches!(event.get("k").and_then(Value::as_str),
                Some("morte"|"bola"|"fugiu")) { continue; }
            let slot=*index.get_or_insert_with(||{
                self.snapshot.days.push(HistoryDay{day,hunt:slug.clone(),..Default::default()});
                self.snapshot.days.len()-1
            });
            if self.snapshot.days[slot].from_event(event) {
                self.snapshot.lifetime.from_event(event);
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
                            for target in [&mut self.snapshot.days[slot],&mut self.snapshot.lifetime]{
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
        if self.dirty{
            self.snapshot.retain_recent(day);
            self.needs_publish=true;
        }
    }
    pub fn maybe_flush(&mut self,force:bool) {
        if self.needs_publish &&
            (force || self.last_publish.elapsed()>=Duration::from_secs(1)) {
            if let Ok(mut shared)=self.shared.lock() {
                let mut snapshot=self.snapshot.clone();
                snapshot.save_error=shared.save_error.clone();
                *shared=snapshot;
                self.needs_publish=false;
            }
            self.last_publish=Instant::now();
        }
        if !force && (!self.dirty || self.last_flush.elapsed()<SAVE_INTERVAL){return}
        if let Some(path)=self.path.as_ref(){
            let raw=self.snapshot.to_value(&self.nick).to_string();
            let (ack_tx,ack_rx)=mpsc::channel();
            let ack=if force {Some(ack_tx)}else{None};
            if self.writer.send((path.clone(),raw,ack)).is_ok(){
                self.dirty=false;
                if force && ack_rx.recv_timeout(FINAL_WRITE_TIMEOUT).is_err() {
                    if let Ok(mut shared)=self.shared.lock() {
                        shared.save_error=Some("History final write was not acknowledged".into());
                    }
                }
            } else if let Ok(mut shared)=self.shared.lock() {
                shared.save_error=Some("History writer unavailable".into());
            }
        }
        self.last_flush=Instant::now();
    }
}
impl Drop for HistoryRecorder {
    fn drop(&mut self){self.maybe_flush(true);}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_hunt_updates_from_welcome_and_ordered_battle_events() {
        // Keep unit tests entirely in memory; never touch a user's
        // production history directory.
        let (writer,_receiver)=mpsc::channel();
        let snapshot=HistorySnapshot::default();
        let mut history=HistoryRecorder{
            shared:Arc::new(Mutex::new(snapshot.clone())),snapshot,
            nick:"Test".into(),last_hunt:None,last_kill:None,dirty:false,needs_publish:false,
            last_publish:Instant::now(),last_flush:Instant::now(),
            writer,path:None
        };
        history.ingest(r#"{"t":"welcome","estado":{"huntSlug":"route-a"}}"#,None);
        history.ingest(r#"{"t":"batalha","ev":[
            {"k":"morte","quem":"selvagem","xpTreinador":5},
            {"k":"hunt","slug":"route-b"},
            {"k":"morte","quem":"selvagem","xpTreinador":10}
        ]}"#,Some("route-b"));
        assert_eq!(history.snapshot.days.iter().find(|d|d.hunt=="route-a")
            .unwrap().trainer_xp,5);
        assert_eq!(history.snapshot.days.iter().find(|d|d.hunt=="route-b")
            .unwrap().trainer_xp,10);
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
