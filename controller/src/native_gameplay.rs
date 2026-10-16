//! Native stateful gameplay, implemented independently from public protocol behavior.
use serde_json::{json, Value};
use crate::native_capture_config::CatchSettings;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum GameCommand {
    Throw { ball_id: u64, slot: u64 },
    Hunt(String),
    Center,
    AutoSet(serde_json::Map<String, Value>),
    SetAutomation { field: String, value: Value },
    SellPokemon(u64),
    ShopBuy { kind: String, id: u64, qty: u64 },
}
impl GameCommand {
    pub fn wire(&self) -> Value {
        match self {
            Self::Throw { ball_id, slot } => json!({"t":"ball.throw","ballId":ball_id,"slot":slot}),
            Self::Hunt(slug) => json!({"t":"hunt.select","slug":slug}),
            Self::Center => json!({"t":"centro.ir"}),
            Self::AutoSet(fields) => {
                let mut fields=fields.clone();
                fields.insert("t".into(),Value::String("auto.set".into()));
                Value::Object(fields)
            },
            Self::SetAutomation {field,value} => json!({"t":"auto.set","field":field,"value":value}),
            Self::SellPokemon(id) => json!({"t":"shop.sellPokemon","pokemonId":id}),
            Self::ShopBuy {kind,id,qty} => json!({"t":"shop.buy","kind":kind,"id":id,"qty":qty}),
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Throw{ball_id,slot} => *ball_id>0 && *slot>0,
            Self::Hunt(slug) => !slug.is_empty() && slug.len()<=128 &&
                slug.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            Self::Center => true,
            Self::AutoSet(fields) => fields.len()<128 && !fields.contains_key("t"),
            Self::SetAutomation {field,value} =>
                crate::native_automation::AutomationWriter::valid(field,value),
            Self::SellPokemon(id) => *id > 0,
            Self::ShopBuy{kind,id,qty} =>
                (kind=="ball" || kind=="item") && *id>0 && (1..=5000).contains(qty),
        }
    }
}
#[derive(Clone)]
struct Corpse { slot:u64, at:Instant, last:Option<Instant>, pending:bool }
#[derive(Clone,Default)]
pub struct GameplayStatus {
    pub auto_catch:bool, pub queue:usize, pub throws:u64,
    pub catches:u64, pub at_center:bool, pub error:Option<String>,
}
pub struct Gameplay {
    pub status:GameplayStatus,
    corpses:VecDeque<Corpse>,
    balls:BTreeMap<u64,u64>,
    catalog:BTreeMap<u64,String>,
    priorities:Vec<String>,
    reserves:BTreeMap<String,u64>,
    last_throw:Option<Instant>,
    pub server_auto:serde_json::Map<String,Value>,
}
impl Gameplay {
    pub fn new(settings: Option<&Value>) -> Self {
        let config=CatchSettings::from_value(settings);
        Self {
            status:GameplayStatus {
                auto_catch:config.enabled,
                ..Default::default()
            },
            corpses:VecDeque::new(),balls:BTreeMap::new(),catalog:BTreeMap::new(),
            priorities:config.priorities,
            reserves:config.reserves,
            last_throw:None,server_auto:serde_json::Map::new()
        }
    }
    pub fn configure(&mut self, config:&CatchSettings) {
        self.status.auto_catch=config.enabled;
        self.priorities=config.priorities.clone();
        self.reserves=config.reserves.clone();
    }
    pub fn ingest(&mut self, raw:&str, now:Instant) {
        let Ok(frame)=serde_json::from_str::<Value>(raw) else {return};
        let kind=frame.get("t").and_then(Value::as_str);
        if kind==Some("welcome") {
            self.corpses.clear();
            if let Some(catalog)=frame.get("catalogoBolas").and_then(Value::as_array) {
                for entry in catalog {
                    if let (Some(id),Some(name))=(
                        entry.get("id").and_then(Value::as_u64),
                        entry.get("nome").or_else(||entry.get("name")).and_then(Value::as_str)
                    ){self.catalog.insert(id,name.to_lowercase());}
                }
            }
        }
        if matches!(kind,Some("welcome"|"estado")) {
            if let Some(state)=frame.get("estado") {
                if let Some(stock)=state.get("balls").and_then(Value::as_object) {
                    self.balls=stock.iter().filter_map(|(id,v)|
                        Some((id.parse::<u64>().ok()?,v.as_u64()?))).collect();
                }
                if let Some(auto)=state.get("automation").and_then(Value::as_object) {
                    for (key,value) in auto {self.server_auto.insert(key.clone(),value.clone());}
                }
                if let Some(center)=state.get("noCentro").and_then(Value::as_bool) {
                    self.status.at_center=center;
                }
            }
        }
        if kind==Some("batalha") {
            if let Some(events)=frame.get("ev").and_then(Value::as_array) {
                for event in events {
                    let slot=event.get("slot").and_then(Value::as_u64);
                    match event.get("k").and_then(Value::as_str) {
                        Some("morte") if event.get("quem").and_then(Value::as_str)==Some("selvagem")=>{
                            if let Some(slot)=slot {
                                self.corpses.retain(|v|v.slot!=slot);
                                self.corpses.push_back(Corpse{slot,at:now,last:None,pending:false});
                            }
                        }
                        Some("bola")=>{
                            if let Some(slot)=slot {
                                self.status.throws+=1;
                                if event.get("sucesso").and_then(Value::as_bool)==Some(true) {
                                    self.status.catches+=1;
                                    self.corpses.retain(|v|v.slot!=slot);
                                } else if let Some(c)=self.corpses.iter_mut().find(|c|c.slot==slot) {
                                    c.pending=false;c.last=Some(now);
                                }
                                self.last_throw=Some(now);
                            }
                        }
                        Some("centro"|"hunt")=>self.corpses.clear(),
                        _=>{}
                    }
                }
            }
        }
        self.corpses.retain(|v|now.duration_since(v.at)<Duration::from_secs(30));
        self.status.queue=self.corpses.len();
    }
    pub fn next_capture(&mut self,now:Instant)->Option<GameCommand> {
        if !self.status.auto_catch || self.status.at_center ||
            self.last_throw.is_some_and(|v|now.duration_since(v)<Duration::from_millis(1200)) {
            return None;
        }
        // Never infer a numeric ball ID or spend a Master Ball accidentally.
        let id=self.priorities.iter().find_map(|priority|
            self.catalog.iter().find(|(id,name)|
                !name.contains("master") &&
                (name.contains(priority) || (priority=="poke" && name.contains("poké"))) &&
                self.balls.get(id).copied().unwrap_or(0)>
                    self.reserves.get(priority).copied().unwrap_or(0))
                .map(|(id,_)|*id))?;
        let corpse=self.corpses.iter_mut().find(|v|
            (!v.pending || v.last.is_some_and(|at|now.duration_since(at)>=Duration::from_secs(3))) &&
            v.last.is_none_or(|at|now.duration_since(at)>=Duration::from_millis(1200)))?;
        corpse.last=Some(now);corpse.pending=true;
        self.last_throw=Some(now);
        Some(GameCommand::Throw{ball_id:id,slot:corpse.slot})
    }
    pub fn failed(&mut self,command:&GameCommand,reason:&str) {
        self.status.error=Some(reason.to_owned());
        if let GameCommand::Throw{slot,..}=command {
            if let Some(c)=self.corpses.iter_mut().find(|c|c.slot==*slot) {c.pending=false;}
        }
    }
}
#[cfg(test)]
mod protocol_command_tests {
    use super::*;
    #[test]
    fn sell_command_uses_managers_confirmed_wire_format() {
        assert_eq!(GameCommand::SellPokemon(42).wire(),
            json!({"t":"shop.sellPokemon","pokemonId":42}));
        assert!(!GameCommand::SellPokemon(0).is_valid());
        assert!(!GameCommand::SetAutomation {
            field:"unsupported".into(),value:json!(true)
        }.is_valid());
    }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn ball_reserve_is_enforced_with_priority_fallback(){
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,
        "ballPriority":["beast","ultra"],"reserve":{"beast":3,"ultra":1}})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[
        {"id":5,"nome":"Beast Ball"},{"id":4,"nome":"Ultra Ball"}],
        "estado":{"balls":{"5":3,"4":2}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":7}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire()["ballId"],4);
    g.configure(&CatchSettings::from_value(Some(&json!({
        "enabled":true,"ballPriority":["beast"],
        "reserve":{"beast":3}
    }))));
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":8}]}"#,at);
    assert!(g.next_capture(at+Duration::from_secs(2)).is_none());
 }
 #[test]
 fn native_capture_requires_enabled_mode_catalog_and_server_corpse(){
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],"estado":{"balls":{"4":3}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":7}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire(),json!({"t":"ball.throw","ballId":4,"slot":7}));
    assert!(g.next_capture(at).is_none());
 }
}
