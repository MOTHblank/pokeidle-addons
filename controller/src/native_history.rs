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
#[derive(Clone,Debug,Default)]
pub struct HistorySnapshot {
    pub owner: String,
    pub days: Vec<HistoryDay>,
    /// Never pruned: lifetime gameplay totals survive daily detail retention.
    pub lifetime: HistoryDay,
    pub load_error: Option<String>,
    pub save_error: Option<String>,
}
impl HistorySnapshot {
    fn to_value(&self, nick: &str) -> Value {
        json!({"version":1,"nick":nick,
            "lifetime":self.lifetime.to_value(),
            "days":self.days.iter().map(HistoryDay::to_value).collect::<Vec<_>>()})
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
        Ok(Self{owner:nick.to_owned(),days,lifetime,..Default::default()})
    }
    fn retain_recent(&mut self, now_day:u64) {
        self.days.retain(|day|
            day.day<=now_day && now_day.saturating_sub(day.day)<MAX_DAYS);
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
        Self {shared,snapshot,nick,dirty:false,needs_publish:false,
            last_publish:Instant::now(),last_flush:Instant::now(),
            writer:writer_tx,path}
    }
    pub fn ingest(&mut self,raw:&str,hunt:Option<&str>) {
        let Ok(frame)=serde_json::from_str::<Value>(raw) else{return};
        if frame.get("t").and_then(Value::as_str)!=Some("batalha"){return}
        let Some(events)=frame.get("ev").and_then(Value::as_array) else{return};
        let day=unix_day();
        let hunt=hunt.unwrap_or("unknown");
        let slug=if hunt.len()<=128 {hunt}else{"unknown"};
        let mut index=self.snapshot.days.iter().position(|row|
            row.day==day && row.hunt==slug);
        for event in events {
            // Only recognized gameplay events allocate history rows.
            if !matches!(event.get("k").and_then(Value::as_str),
                Some("morte"|"bola"|"fugiu")) { continue; }
            let slot=*index.get_or_insert_with(||{
                self.snapshot.days.push(HistoryDay{day,hunt:slug.into(),..Default::default()});
                self.snapshot.days.len()-1
            });
            if self.snapshot.days[slot].from_event(event) {
                // The same server receipt contributes exactly once to
                // lifetime statistics and its daily hunt breakdown.
                self.snapshot.lifetime.from_event(event);
                self.dirty=true;
            }
        }
        if self.dirty{
            self.snapshot.retain_recent(day);
            // A single battle can emit many frames per second. Only clone
            // the bounded history into the UI at one-second cadence.
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
