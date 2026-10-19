//! Native stateful gameplay, implemented independently from public protocol behavior.
use serde_json::{json, Value};
use crate::native_capture_config::CatchSettings;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum GameCommand {
    Throw { ball_id: u64, slot: u64 },
    Hunt(String),
    /// Game-server Outland drop tier (independent of hunt location).
    OutlandTier(u8),
    Center,
    AutoSet(serde_json::Map<String, Value>),
    SetAutomation { field: String, value: Value },
    // Kept for the guarded Pokémon-sale command path and future UI controls.
    #[allow(dead_code)]
    SellPokemon(u64),
    ShopBuy { kind: String, id: u64, qty: u64 },
}
impl GameCommand {
    pub fn wire(&self) -> Value {
        match self {
            Self::Throw { ball_id, slot } => json!({"t":"ball.throw","ballId":ball_id,"slot":slot}),
            Self::Hunt(slug) => json!({"t":"hunt.select","slug":slug}),
            Self::OutlandTier(tier) => json!({"t":"outland.tier","tier":tier}),
            Self::Center => json!({"t":"centro.ir"}),
            Self::AutoSet(fields) => {
                let mut fields=fields.clone();
                fields.insert("t".into(),Value::String("auto.set".into()));
                Value::Object(fields)
            },
            // Internal intent only: the game socket replaces it with a full
            // server-based AutoSet payload. Never transmit this shape.
            Self::SetAutomation { .. } => Value::Null,
            Self::SellPokemon(id) => json!({"t":"shop.sellPokemon","pokemonId":id}),
            Self::ShopBuy {kind,id,qty} => json!({"t":"shop.buy","kind":kind,"id":id,"qty":qty}),
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Throw{ball_id,..} => *ball_id>0,
            Self::Hunt(slug) => !slug.is_empty() && slug.len()<=128 &&
                slug.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            Self::OutlandTier(tier) => (1..=8).contains(tier),
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
// The game exposes 30-second corpse lifetimes. Never use a late result as
// evidence that an old slot should be revived: a new "morte" starts a new life.
const CORPSE_LIFETIME:Duration=Duration::from_secs(30);
// A 600-ms native cadence serves dense AoE hunts. Bound unacknowledged
// commands, without resetting the game connection when receipts are missing.
// This is a chosen application cadence, NOT a verified server cooldown.
const MAX_PENDING_CAPTURE_THROWS:usize=3;
const CAPTURE_RESULT_TIMEOUT:Duration=Duration::from_secs(8);
const MIN_THROW_INTERVAL_MS:u64=600;
#[derive(Clone)]
struct Corpse {
    slot:u64, at:Instant, last:Option<Instant>, pending:bool,
    /// The pending throw is not reflected by estado.balls or a bola receipt.
    /// This flag belongs to the target lifecycle; never retain old reservations
    /// after that target is removed.
    unreflected:bool,
    // A missing receipt is a capture-level warning, never a socket failure.
    timeout_reported:bool,
    attempts:u32, last_ball:Option<u64>,
}
#[derive(Clone,Debug,Default)]
pub struct GameplayStatus {
    pub auto_catch:bool, pub queue:usize, pub throws:u64,
    /// Confirmed server success receipts. Unlike native-attributed catches,
    /// these do not require an outstanding corpse slot to still be tracked.
    pub server_catches:u64, pub server_failures:u64,
    pub catches:u64, pub failures:u64, pub retries:u64,
    pub capture_resyncs:u64,
    pub unmatched_successes:u64,
    /// Awaiting a battle capture result, even if inventory already reflected
    /// the ball cost. This must not be conflated with reserved ball stock.
    pub pending_results:u64, pub unreflected_throws:u64,
    pub at_center:bool, pub error:Option<String>,
    /// Enabled is a preference; ready requires server inventory and a usable ball.
    pub capture_ready:bool, pub blocked_reason:Option<String>,
    pub warning:Option<String>, pub deaths_without_slot:u64,
    pub last_server_notice:Option<String>,
    pub last_action:String,
    pub used_by_ball:BTreeMap<u64,u64>,
    pub caught_by_ball:BTreeMap<u64,u64>,
    pub server_caught_by_ball:BTreeMap<u64,u64>,
}
pub struct Gameplay {
    pub status:GameplayStatus,
    corpses:VecDeque<Corpse>,
    // Latest authoritative inventory, adjusted for confirmed ball receipts
    // received between server stock snapshots. There is no cumulative local
    // debit ledger: reservations exist only on active pending corpses.
    balls:BTreeMap<u64,u64>,
    ball_inventory_seen:bool,
    catalog:BTreeMap<u64,String>,
    priorities:Vec<String>,
    reserves:BTreeMap<String,u64>,
    interval:Duration,
    restock_ball_id:u64, restock_enabled:bool,
    last_death_untracked:bool,
    last_throw:Option<Instant>,
    pub server_auto:serde_json::Map<String,Value>,
}
impl Gameplay {
    pub fn new(settings: Option<&Value>) -> Self {
        let config=CatchSettings::from_value(settings);
        Self {
            status:GameplayStatus { auto_catch:config.enabled, ..Default::default() },
            corpses:VecDeque::new(),balls:BTreeMap::new(),ball_inventory_seen:false,
            catalog:BTreeMap::new(),priorities:config.priorities,
            reserves:config.reserves,
            interval:Duration::from_millis(config.interval_ms.max(MIN_THROW_INTERVAL_MS)),
            restock_ball_id:config.ball_id,restock_enabled:config.restock_enabled,
            last_death_untracked:false,last_throw:None,server_auto:serde_json::Map::new()
        }
    }
    pub fn configure(&mut self, config:&CatchSettings) {
        self.status.auto_catch=config.enabled;
        self.priorities=config.priorities.clone();
        self.reserves=config.reserves.clone();
        self.interval=Duration::from_millis(config.interval_ms.clamp(600,3000));
        self.restock_ball_id=config.ball_id;
        self.restock_enabled=config.restock_enabled;
        self.refresh_readiness();
    }
    pub fn on_disconnect(&mut self) {
        self.corpses.clear();
        self.balls.clear();
        self.ball_inventory_seen=false;
        self.catalog.clear();
        self.last_throw=None;
        self.status.queue=0;
        self.status.deaths_without_slot=0;
        self.status.last_server_notice=None;
        self.last_death_untracked=false;
        self.status.at_center=true;
        self.status.last_action="Waiting for authenticated game state".into();
        self.refresh_readiness();
    }
    fn expire_corpses(&mut self,now:Instant) {
        self.corpses.retain(|c|
            now.saturating_duration_since(c.at)<CORPSE_LIFETIME);
        self.status.queue=self.corpses.len();
    }
    fn pending_for_ball(&self,id:u64)->u64 {
        self.corpses.iter().filter(|corpse|
            corpse.pending && corpse.unreflected &&
            corpse.last_ball==Some(id)).count() as u64
    }
    fn pending_throws(&self)->usize {
        self.corpses.iter().filter(|corpse|corpse.pending).count()
    }
    fn refresh_ball_inventory(&mut self,stock:&serde_json::Map<String,Value>) {
        let fresh=stock.iter().filter_map(|(id,amount)|
            Some((id.parse::<u64>().ok()?,amount.as_u64()?))
        ).collect::<BTreeMap<_,_>>();
        // A decrease in authoritative stock accounts for outstanding sends.
        // Release only the corresponding pending reservations; a stale/equal
        // patch must NOT acknowledge them. Unrelated corpses never acquire a
        // permanent reservation.
        for (id,previous) in &self.balls {
            let current=fresh.get(id).copied().unwrap_or(0);
            let mut confirmed=previous.saturating_sub(current);
            if confirmed==0 {continue}
            for corpse in &mut self.corpses {
                if confirmed==0 {break}
                if corpse.pending && corpse.unreflected && corpse.last_ball==Some(*id) {
                    corpse.unreflected=false;
                    confirmed-=1;
                }
            }
        }
        self.balls=fresh;
        self.ball_inventory_seen=true;
        self.refresh_readiness();
    }
    fn available(&self,id:u64)->u64 {
        let stock=self.balls.get(&id).copied().unwrap_or(0);
        // The configured restock ball is replenishable. Don't subtract
        // speculative local sends from its usable stock: an outdated
        // reservation should never block low-stock Auto Catch. Still require
        // a positive actual server stock and respect explicit ball reserves.
        if self.restock_enabled && self.restock_ball_id==id {
            stock
        } else {
            stock.saturating_sub(self.pending_for_ball(id))
        }
    }
    fn ball_matches(name:&str,priority:&str)->bool {
        name.contains(priority) || (priority=="poke" && name.contains("poké"))
    }
    /// The game and PokeIdle Manager use stable inventory ball IDs. A server
    /// inventory object is authoritative for quantity; a descriptive shop
    /// catalog is optional and must not prevent native capture.
    fn known_ball_id(priority:&str)->Option<u64> {
        match priority {
            "poke"=>Some(1),"great"=>Some(2),"super"=>Some(3),
            "ultra"=>Some(4),"beast"=>Some(5),
            _=>None
        }
    }
    fn ball_id_for_priority(&self,priority:&str,require_available:bool)->Option<u64> {
        let known=Self::known_ball_id(priority)?;
        let reserve=self.reserves.get(priority).copied().unwrap_or(0);
        let allowed=|id:u64|self.catalog.get(&id)
            .is_none_or(|name|!name.contains("master")) &&
            (!require_available || self.available(id)>reserve);
        self.catalog.iter()
            .find(|(id,name)|Self::ball_matches(name,priority) && allowed(**id))
            .map(|(id,_)|*id)
            .or_else(||self.balls.contains_key(&known).then_some(known)
                .filter(|id|allowed(*id)))
    }
    fn refresh_readiness(&mut self) {
        self.status.pending_results=self.pending_throws() as u64;
        self.status.unreflected_throws=self.corpses.iter()
            .filter(|corpse|corpse.pending && corpse.unreflected).count() as u64;
        let mismatch=self.restock_enabled && self.restock_ball_id>0 &&
            !self.priorities.iter().any(|p|
                Self::known_ball_id(p)==Some(self.restock_ball_id) ||
                self.catalog.get(&self.restock_ball_id)
                    .is_some_and(|name|Self::ball_matches(name,p)));
        self.status.warning=if mismatch {
            Some("Restock buys a ball that Auto Catch does not use".to_owned())
        } else {None};
        let reason=if !self.status.auto_catch {
            Some("Auto Catch disabled")
        } else if self.status.at_center {
            Some("At center or waiting for authenticated hunt state")
        } else if self.priorities.is_empty() {
            Some("No capture ball selected")
        } else if !self.ball_inventory_seen {
            Some("Waiting for server Poké Ball inventory")
        } else if self.last_death_untracked && self.corpses.is_empty() {
            Some("Wild death events lack capture slots; cannot target throws")
        } else if self.pending_throws()>=MAX_PENDING_CAPTURE_THROWS {
            Some("Waiting for in-flight capture results")
        } else if !self.priorities.iter().any(|priority|
            self.ball_id_for_priority(priority,true).is_some()) {
            if self.priorities.iter().any(|priority|
                self.ball_id_for_priority(priority,false).is_some_and(|id|
                    self.balls.get(&id).copied().unwrap_or(0)>
                        self.reserves.get(priority).copied().unwrap_or(0))) {
                Some("Selected Poké Balls reserved; waiting for server consumption confirmation")
            } else {
                Some("No selected Poké Balls above reserve; check restock and ball priorities")
            }
        } else {None};
        self.status.capture_ready=reason.is_none();
        self.status.blocked_reason=reason.map(str::to_owned);
    }
    #[cfg(test)]
    pub fn ingest(&mut self, raw:&str, now:Instant) {
        let Ok(frame)=serde_json::from_str::<Value>(raw) else {return};
        self.ingest_value(&frame, now);
    }
    pub fn ingest_value(&mut self, frame:&Value, now:Instant) {
        let kind=frame.get("t").and_then(Value::as_str);
        if kind==Some("welcome") {
            self.on_disconnect();
            self.status.at_center=false;
            self.status.error=None;
            self.server_auto.clear();
        }
        if matches!(kind,Some("welcome"|"estado")) {
            let catalog=frame.get("catalogoBolas").or_else(||
                frame.get("estado").and_then(|v|v.get("catalogoBolas")));
            if let Some(entries)=catalog.and_then(Value::as_array) {
                for entry in entries {
                    if let (Some(id),Some(name))=(
                        entry.get("id").and_then(Value::as_u64),
                        entry.get("nome").or_else(||entry.get("name")).and_then(Value::as_str)
                    ) {
                        self.catalog.insert(id,name.to_lowercase());
                    }
                }
            }
            if let Some(state)=frame.get("estado") {
                // A full server corpse list can recover deaths whose receipts
                // omit a slot. Never guess a target slot.
                if let Some(downed)=state.get("caidos").and_then(Value::as_array) {
                    let slots=downed.iter().filter_map(|entry|
                        entry.get("slot").and_then(Value::as_u64)
                            .or_else(||entry.as_u64())).collect::<Vec<_>>();
                    self.corpses.retain(|corpse|slots.contains(&corpse.slot));
                    if !slots.is_empty(){ self.last_death_untracked=false; }
                    for slot in slots {
                        if !self.corpses.iter().any(|corpse|corpse.slot==slot) {
                            self.corpses.push_back(Corpse {
                                slot,at:now,last:None,pending:false,
                                unreflected:false,timeout_reported:false,attempts:0,last_ball:None
                            });
                        }
                    }
                }
                if let Some(stock)=state.get("balls").and_then(Value::as_object) {
                    self.refresh_ball_inventory(stock);
                }
                if let Some(auto)=state.get("automation").and_then(Value::as_object) {
                    for (key,value) in auto {
                        self.server_auto.insert(key.clone(),value.clone());
                    }
                }
                if let Some(center)=state.get("noCentro").and_then(Value::as_bool) {
                    self.status.at_center=center;
                    if center {self.corpses.clear();}
                }
            }
        }
        if kind==Some("batalha") {
            if let Some(events)=frame.get("ev").and_then(Value::as_array) {
                for event in events {
                    let slot=event.get("slot").and_then(Value::as_u64);
                    match event.get("k").and_then(Value::as_str) {
                        Some("aviso")=>{
                            if let Some(message)=event.get("msg").and_then(Value::as_str) {
                                let lower=message.to_lowercase();
                                if ["bola","ball","captur","throw","slot"].iter()
                                    .any(|word|lower.contains(word)) {
                                    self.status.last_server_notice=Some(
                                        message.chars().take(150).collect()
                                    );
                                }
                            }
                        }
                        Some("morte") if event.get("quem").and_then(Value::as_str)==Some("selvagem")=>{
                            if let Some(slot)=slot {
                                self.last_death_untracked=false;
                                self.corpses.retain(|v|v.slot!=slot);
                                self.corpses.push_back(Corpse {
                                    slot,at:now,last:None,pending:false,
                                    unreflected:false,timeout_reported:false,attempts:0,last_ball:None
                                });
                            } else {
                                self.last_death_untracked=true;
                                self.status.deaths_without_slot=
                                    self.status.deaths_without_slot.saturating_add(1);
                            }
                        }
                        Some("bola")=>{
                            if let Some(success)=event.get("sucesso").and_then(Value::as_bool) {
                                // VIP/manual results must not reset the NATIVE
                                // send timer or starve this capture worker.
                                // The server receipt is authoritative regardless
                                // of slot correlation. A slot may have expired
                                // or the reply may omit it altogether.
                                let response_ball=event.get("ballId").and_then(Value::as_u64);
                                if success {
                                    self.status.server_catches=self.status.server_catches.saturating_add(1);
                                    if let Some(id)=response_ball {
                                        *self.status.server_caught_by_ball.entry(id).or_default()+=1;
                                    }
                                } else {
                                    self.status.server_failures=self.status.server_failures.saturating_add(1);
                                }
                                // Missing slots may be correlated only when
                                // exactly one outstanding local throw is eligible.
                                // Never assign an ambiguous receipt to a corpse.
                                let matched=slot.and_then(|slot|
                                    self.corpses.iter().position(|c|c.slot==slot))
                                    .or_else(||{
                                        if slot.is_some(){return None}
                                        let mut eligible=self.corpses.iter().enumerate()
                                            .filter(|(_,c)|c.pending && c.attempts>0 &&
                                                response_ball.is_none_or(|id|c.last_ball==Some(id)))
                                            .map(|(idx,_)|idx);
                                        let first=eligible.next()?;
                                        if eligible.next().is_some(){None}else{Some(first)}
                                    });
                                if let Some(idx)=matched {
                                    let own=self.corpses[idx].attempts>0;
                                    let pending=self.corpses[idx].pending;
                                    let expected_ball=self.corpses[idx].last_ball;
                                    // A receipt consumes one ball only when
                                    // the server has not already reflected it
                                    // in its authoritative inventory snapshot.
                                    if own && pending && self.corpses[idx].unreflected &&
                                        (response_ball.is_none() || response_ball==expected_ball) {
                                        if let Some(id)=response_ball.or(expected_ball) {
                                            if let Some(stock)=self.balls.get_mut(&id) {
                                                *stock=stock.saturating_sub(1);
                                            }
                                        }
                                        self.corpses[idx].unreflected=false;
                                    }
                                    if success {
                                        if own {
                                            self.status.catches=self.status.catches.saturating_add(1);
                                            if let Some(id)=response_ball.or(expected_ball) {
                                                *self.status.caught_by_ball.entry(id).or_default()+=1;
                                            }
                                            self.status.last_action="Pokémon captured".into();
                                            self.status.error=None;
                                        }
                                        self.corpses.remove(idx);
                                    } else if own && (response_ball.is_none() ||
                                        response_ball==expected_ball) {
                                        if self.corpses[idx].pending {
                                            self.corpses[idx].pending=false;
                                            self.corpses[idx].unreflected=false;
                                            self.status.failures=self.status.failures.saturating_add(1);
                                            self.status.last_action="The Pokémon resisted; retrying".into();
                                            self.status.error=None;
                                        }
                                    }
                                } else if success {
                                    self.status.unmatched_successes=
                                        self.status.unmatched_successes.saturating_add(1);
                                }
                            }
                        }
                        Some("centro")=>{
                            self.corpses.clear();
                            self.status.at_center=true;
                        }
                        Some("hunt")=>{
                            self.corpses.clear();
                            self.status.at_center=false;
                        }
                        _=>{}
                    }
                }
            }
        }
        self.expire_corpses(now);
        self.refresh_readiness();
    }
    /// A missing capture receipt does not imply a dead game connection.
    /// Keep pending throws and their reserved inventory until the actual result,
    /// authoritative slot removal or corpse expiration, without replaying them.
    pub fn has_unreported_capture_timeout(&self, now:Instant)->bool {
        self.status.auto_catch && self.corpses.iter().any(|corpse|
            corpse.pending && !corpse.timeout_reported &&
            corpse.last.is_some_and(|last|
                now.saturating_duration_since(last)>=CAPTURE_RESULT_TIMEOUT))
    }
    pub fn report_capture_timeouts(&mut self, now:Instant)->usize {
        let mut newly_timed_out=0usize;
        for corpse in &mut self.corpses {
            if corpse.pending && !corpse.timeout_reported &&
                corpse.last.is_some_and(|last|
                    now.saturating_duration_since(last)>=CAPTURE_RESULT_TIMEOUT) {
                corpse.timeout_reported=true;
                newly_timed_out+=1;
            }
        }
        if newly_timed_out>0 {
            self.status.capture_resyncs=self.status.capture_resyncs
                .saturating_add(newly_timed_out as u64);
            self.status.error=Some(
                "Native capture receipt overdue; waiting without disconnecting".into()
            );
            self.status.last_action="Waiting for delayed capture results".into();
        }
        newly_timed_out
    }
    pub fn next_capture(&mut self,now:Instant)->Option<GameCommand> {
        self.expire_corpses(now);
        self.refresh_readiness();
        if !self.status.capture_ready ||
            self.pending_throws()>=MAX_PENDING_CAPTURE_THROWS ||
            self.last_throw.is_some_and(|v|
                now.saturating_duration_since(v)<self.interval) {
            return None;
        }
        // Prefer the primary ball down to its reserve, then the fallback.
        // Use server inventory counts and the same stable ball IDs used by
        // the Manager; do not require optional shop metadata.
        let selected=self.priorities.iter()
            .find_map(|priority|self.ball_id_for_priority(priority,true));
        let Some(id)=selected else {
            if !self.corpses.is_empty() {
                self.status.last_action="Waiting for allowed balls above reserves".into();
            }
            return None;
        };
        let next=self.corpses.iter().enumerate()
            .filter(|(_,c)|!c.pending && c.last.is_none_or(|at|
                now.saturating_duration_since(at)>=self.interval))
            // Oldest corpse expiry wins; among equally old targets, serve
            // unattempted AoE bodies before retrying a previous failure.
            .min_by_key(|(_,c)|(c.at,c.attempts>0,c.last.unwrap_or(c.at)))
            .map(|(idx,_)|idx);
        let Some(idx)=next else {return None};
        let corpse=&mut self.corpses[idx];
        let retry=corpse.attempts>0;
        if retry {
            self.status.retries=self.status.retries.saturating_add(1);
        }
        corpse.last=Some(now);
        corpse.last_ball=Some(id);
        corpse.pending=true;
        corpse.timeout_reported=false;
        corpse.unreflected=true;
        corpse.attempts=corpse.attempts.saturating_add(1);
        self.last_throw=Some(now);
        self.status.throws=self.status.throws.saturating_add(1);
        *self.status.used_by_ball.entry(id).or_default()+=1;
        self.status.last_action=if retry {
            "Trying another Poké Ball".to_string()
        } else {"Throwing a Poké Ball".to_string()};
        let command=GameCommand::Throw{ball_id:id,slot:corpse.slot};
        self.refresh_readiness();
        Some(command)
    }
    pub fn failed(&mut self,command:&GameCommand,reason:&str) {
        self.status.error=Some(reason.to_owned());
        if let GameCommand::Throw{ball_id,slot}=command {
            if let Some(c)=self.corpses.iter_mut().find(|c|c.slot==*slot) {
                c.pending=false;
                c.unreflected=false;
                if c.attempts>0 {c.attempts-=1;}
                // A failed socket send is not a server throw.
            }
            self.status.throws=self.status.throws.saturating_sub(1);
            if let Some(v)=self.status.used_by_ball.get_mut(ball_id) {
                *v=v.saturating_sub(1);
            }
            self.refresh_readiness();
        }
    }
}
#[cfg(test)]
mod protocol_command_tests {
    use super::*;
    #[test]
    fn outland_tier_uses_server_protocol_and_rejects_invalid_tiers() {
        for tier in 1..=8 {
            let cmd=GameCommand::OutlandTier(tier);
            assert!(cmd.is_valid());
            assert_eq!(cmd.wire(),json!({"t":"outland.tier","tier":tier}));
        }
        assert!(!GameCommand::OutlandTier(0).is_valid());
        assert!(!GameCommand::OutlandTier(9).is_valid());
    }
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
 fn server_cooldown_prefers_older_expiring_corpse() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],"intervalMs":100
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":20}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":11}]}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":22}]}"#,
        at+Duration::from_secs(5));
    assert_eq!(g.next_capture(at+Duration::from_secs(5)).unwrap().wire()["slot"],11);
    assert!(g.next_capture(at+Duration::from_millis(5599)).is_none());
    // A second distinct AoE corpse is served at 600ms even if the first
    // receipt is still pending.
    assert_eq!(g.next_capture(at+Duration::from_millis(5600)).unwrap().wire()["slot"],22);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":11,
        "ballId":4,"sucesso":true}]}"#,at+Duration::from_millis(5610));
    assert!(g.next_capture(at+Duration::from_millis(6200)).is_none());
    assert_eq!(g.status.throws,2);
    assert_eq!(g.status.catches,1);
 }
 #[test]
 fn retry_after_failed_receipt_honors_server_cooldown() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],"intervalMs":100
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":5}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":7}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire()["slot"],7);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":7,"ballId":4,
        "sucesso":false}]}"#,at+Duration::from_millis(20));
    assert_eq!(g.status.failures,1);
    // The 600 ms cadence is measured from the prior throw, not the
    // receipt (which arrived 20 ms after that throw).
    assert!(g.next_capture(at+Duration::from_millis(599)).is_none());
    assert_eq!(g.next_capture(at+Duration::from_millis(600)).unwrap().wire()["slot"],7);
    assert_eq!(g.status.throws,2);
    assert_eq!(g.status.retries,1);
 }
 #[test]
 fn unanswered_throw_does_not_spam_retries_or_exhaust_stock() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"]
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":5}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":9}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert!(g.next_capture(at+Duration::from_millis(1200)).is_none());
    assert!(g.next_capture(at+Duration::from_secs(7)).is_none());
    assert_eq!(g.status.throws,1);
    assert_eq!(g.status.unreflected_throws,1);
    assert_eq!(g.status.retries,0);
    assert!(!g.has_unreported_capture_timeout(at+Duration::from_secs(7)));
    assert!(g.has_unreported_capture_timeout(at+Duration::from_secs(8)));
 }
 #[test]
 fn stale_inventory_patches_do_not_spend_ball_reserve() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],"reserve":{"ultra":2}
    })));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"balls":{"4":3}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":7},
        {"k":"morte","quem":"selvagem","slot":8}
    ]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire()["slot"],7);
    assert_eq!(g.available(4),2);
    // Duplicate/stale stock is not authoritative evidence the reserved
    // ball has returned to inventory.
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":3}}}"#,
        at+Duration::from_millis(50));
    assert_eq!(g.available(4),2);
    assert!(g.next_capture(at+Duration::from_millis(100)).is_none());
    // Receipt + server stock down by one reconciles the reservation once.
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":2}}}"#,
        at+Duration::from_millis(150));
    assert_eq!(g.available(4),2);
    assert_eq!(g.pending_for_ball(4),0);
 }
 #[test]
 fn duplicate_and_manual_capture_receipts_are_not_ours() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"]
    })));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"balls":{"4":10}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":1},
        {"k":"morte","quem":"selvagem","slot":2}
    ]}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":2,"sucesso":true}]}"#,at);
    assert_eq!(g.status.catches,0);
    let cmd=g.next_capture(at+Duration::from_millis(1200)).unwrap();
    assert_eq!(cmd.wire()["slot"],1);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"bola","slot":1,"ballId":4,"sucesso":true},
        {"k":"bola","slot":1,"ballId":4,"sucesso":true},
        {"k":"bola","slot":1,"ballId":4,"sucesso":false}
    ]}"#,at);
    assert_eq!(g.status.throws,1);
    assert_eq!(g.status.catches,1);
    assert_eq!(g.status.failures,0);
    assert_eq!(g.status.queue,0);
 }
 #[test]
 fn expiring_corpse_and_disconnect_never_replay_stale_slot() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"]
    })));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"balls":{"4":10}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":9}]}"#,at);
    assert!(g.next_capture(at+Duration::from_secs(30)).is_none());
    assert_eq!(g.status.queue,0);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":10}]}"#,
        at+Duration::from_secs(31));
    assert!(g.next_capture(at+Duration::from_secs(31)).is_some());
    g.on_disconnect();
    assert_eq!(g.status.queue,0);
    assert!(g.next_capture(at+Duration::from_secs(32)).is_none());
 }
 #[test]
 fn zero_is_a_valid_server_capture_slot() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["poke"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":1,"nome":"Poké Ball"}],
        "estado":{"noCentro":false,"balls":{"1":5}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":0}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire(),
        json!({"t":"ball.throw","ballId":1,"slot":0}));
    assert!(GameCommand::Throw{ball_id:1,slot:0}.is_valid());
 }
 #[test]
 fn full_corpse_state_recovers_death_without_event_slot() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":8}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem"}]}"#,at);
    assert_eq!(g.status.deaths_without_slot,1);
    assert!(!g.status.capture_ready);
    assert!(g.next_capture(at).is_none());
    g.ingest(r#"{"t":"estado","estado":{"caidos":[{"slot":7}]}}"#,
        at+Duration::from_millis(20));
    assert!(g.status.capture_ready);
    assert_eq!(g.next_capture(at+Duration::from_millis(20)).unwrap().wire()["slot"],7);
 }
 #[test]
 fn replenished_inventory_releases_old_provisional_spend() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":2}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":9}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert_eq!(g.available(4),1);
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":100}}}"#,
        at+Duration::from_millis(1));
    // Non-restocked balls still protect the single unacknowledged spend.
    assert_eq!(g.available(4),99);
    assert!(g.status.capture_ready);
 }
 #[test]
 fn growing_corpse_queue_cannot_create_unbounded_unconfirmed_throws() {
    // Reproduces Account 4: changing downed targets with frozen throw count.
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],"reserve":{"ultra":0}
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":2}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":1},
        {"k":"morte","quem":"selvagem","slot":2}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert!(g.next_capture(at+Duration::from_millis(100)).is_none());
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":3}]}"#,
        at+Duration::from_millis(200));
    assert_eq!(g.status.queue,3);
    assert_eq!(g.status.throws,1);
    assert_eq!(g.status.unreflected_throws,1);
    assert!(g.has_unreported_capture_timeout(at+Duration::from_secs(8)));
 }
 #[test]
 fn missing_capture_receipt_does_not_reset_account_or_rethrow_same_slot() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"]
    })));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":5}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":12}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert!(g.has_unreported_capture_timeout(at+Duration::from_secs(8)));
    assert_eq!(g.report_capture_timeouts(at+Duration::from_secs(8)),1);
    assert!(!g.has_unreported_capture_timeout(at+Duration::from_secs(9)));
    assert_eq!(g.report_capture_timeouts(at+Duration::from_secs(9)),0);
    assert_eq!(g.status.capture_resyncs,1);
    assert_eq!(g.available(4),4);
    assert_eq!(g.pending_throws(),1);
    assert!(g.next_capture(at+Duration::from_secs(9)).is_none());
    // An actual receipt reconciles it while the session stays connected.
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":12,
        "ballId":4,"sucesso":true}]}"#,at+Duration::from_secs(10));
    assert_eq!(g.status.catches,1);
    assert_eq!(g.status.queue,0);
    assert_eq!(g.available(4),4);
 }
 #[test]
 fn inventory_confirmation_is_not_a_capture_result() {
    let now=Instant::now();
    let mut capture=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"]
    })));
    capture.ingest(r#"{"t":"welcome","catalogoBolas":[
        {"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":5}}}"#,now);
    capture.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":22}]}"#,now);
    assert!(capture.next_capture(now).is_some());
    assert_eq!(capture.status.pending_results,1);
    assert_eq!(capture.status.unreflected_throws,1);
    // Stock may update without a bola event. It is NOT proof of a catch.
    capture.ingest(r#"{"t":"estado","estado":{"balls":{"4":4}}}"#,
        now+Duration::from_secs(1));
    assert_eq!(capture.status.unreflected_throws,0);
    assert_eq!(capture.status.pending_results,1);
    capture.ingest(r#"{"t":"batalha","ev":[
        {"k":"bola","slot":22,"ballId":4,"sucesso":true}]}"#,
        now+Duration::from_secs(2));
    assert_eq!(capture.status.pending_results,0);
    assert_eq!(capture.status.catches,1);
 }
 #[test]
 fn restock_mismatch_and_empty_selected_balls_are_reported() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"],
        "restock":{"enabled":true,"selectedBallId":1}})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":1,"nome":"Poké Ball"},
        {"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"1":100,"4":0}}}"#,at);
    assert!(!g.status.capture_ready);
    assert!(g.status.blocked_reason.as_deref().unwrap().contains("No selected"));
    assert!(g.status.warning.as_deref().unwrap().contains("does not use"));
    g.ingest(r#"{"t":"estado","estado":{"balls":{"1":100,"4":5}}}"#,
        at+Duration::from_secs(1));
    assert!(g.status.capture_ready);
 }
 #[test]
 fn server_captures_count_without_slot_and_without_native_attribution() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":100}}}"#,at);
    // This is a valid server success but not a proven native capture.
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","ballId":4,"sucesso":true}]}"#,at);
    assert_eq!(g.status.server_catches,1);
    assert_eq!(g.status.catches,0);
    assert_eq!(g.status.server_caught_by_ball[&4],1);
    assert_eq!(g.status.unmatched_successes,1);
 }
 #[test]
 fn missing_slot_can_match_exactly_one_native_pending_throw() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":100}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":8}]}"#,at);
    assert!(g.next_capture(at).is_some());
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","ballId":4,"sucesso":true}]}"#,
        at+Duration::from_millis(30));
    assert_eq!(g.status.server_catches,1);
    assert_eq!(g.status.catches,1);
    assert_eq!(g.status.queue,0);
    assert_eq!(g.status.unmatched_successes,0);
 }
 #[test]
 fn matching_ball_result_reconciles_local_spend_without_stock_patch() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],
        "estado":{"noCentro":false,"balls":{"4":3}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":11}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert_eq!(g.available(4),2);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":11,"ballId":4,
        "sucesso":false}]}"#,at+Duration::from_millis(15));
    assert_eq!(g.balls[&4],2);
    assert_eq!(g.available(4),2);
    assert_eq!(g.status.unreflected_throws,0);
    assert_eq!(g.status.server_failures,1);
    // A delayed authoritative inventory matches the confirmed consumption.
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":2}}}"#,
        at+Duration::from_millis(50));
    assert_eq!(g.available(4),2);
 }
 #[test]
 fn ambiguous_missing_slot_receipt_never_guesses_a_target() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":100}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":5},
        {"k":"morte","quem":"selvagem","slot":6}]}"#,at);
    assert!(g.next_capture(at).is_some());
    // Simulate two compatible older in-flight replies from the wire, even
    // though the safer new worker never intentionally sends them together.
    let other=g.corpses.iter_mut().find(|c|c.slot==6).unwrap();
    other.pending=true;
    other.attempts=1;
    other.last_ball=Some(4);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","ballId":4,"sucesso":true}]}"#,
        at+Duration::from_millis(110));
    assert_eq!(g.status.server_catches,1);
    assert_eq!(g.status.catches,0);
    assert_eq!(g.status.unmatched_successes,1);
    assert_eq!(g.status.queue,2);
 }
 #[test]
 fn known_empty_inventory_is_not_mislabeled_as_missing_inventory() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{}}}"#,at);
    assert!(g.ball_inventory_seen);
    assert!(g.status.warning.is_none());
    assert_eq!(g.status.blocked_reason.as_deref(),
        Some("No selected Poké Balls above reserve; check restock and ball priorities"));
 }
 #[test]
 fn configured_ultra_ball_uses_authoritative_stock_without_optional_catalog() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":3}}}"#,at);
    assert!(g.catalog.is_empty());
    assert!(g.status.capture_ready);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":7}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire(),
        json!({"t":"ball.throw","ballId":4,"slot":7}));
 }
 #[test]
 fn restock_with_10_to_40_balls_keeps_600ms_aoe_cadence_without_debit_deadlock() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"intervalMs":600,"ballPriority":["ultra"],
        "restock":{"enabled":true,"selectedBallId":4,"ballAt":10,"ballTo":40}
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":10}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":1},
        {"k":"morte","quem":"selvagem","slot":2},
        {"k":"morte","quem":"selvagem","slot":3},
        {"k":"morte","quem":"selvagem","slot":4}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert_eq!(g.next_capture(at+Duration::from_millis(600)).unwrap().wire()["slot"],2);
    assert_eq!(g.next_capture(at+Duration::from_millis(1200)).unwrap().wire()["slot"],3);
    assert_eq!(g.status.throws,3);
    assert_eq!(g.pending_throws(),MAX_PENDING_CAPTURE_THROWS);
    assert!(g.next_capture(at+Duration::from_millis(1800)).is_none());
    assert!(!g.status.capture_ready);
    assert_eq!(g.status.blocked_reason.as_deref(),
        Some("Waiting for in-flight capture results"));
    assert_eq!(g.available(4),10); // Pending stock doesn't override restock.
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":1,"ballId":4,
        "sucesso":false}]}"#,at+Duration::from_millis(1900));
    assert_eq!(g.pending_throws(),2);
    assert_eq!(g.available(4),9); // Confirmed spend is real, not a reservation.
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":40}}}"#,
        at+Duration::from_millis(1950));
    assert_eq!(g.available(4),40);
    assert_eq!(g.next_capture(at+Duration::from_millis(1950)).unwrap().wire()["slot"],4);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"bola","slot":4,"ballId":4,
        "sucesso":true}]}"#,at+Duration::from_millis(2000));
    assert!(g.status.server_catches>0);
 }
 #[test]
 fn restock_does_not_invent_balls_when_server_says_zero() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],
        "restock":{"enabled":true,"selectedBallId":4,"ballAt":10,"ballTo":40}
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":0}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":1}]}"#,at);
    assert!(!g.status.capture_ready);
    assert!(g.next_capture(at).is_none());
    g.ingest(r#"{"t":"estado","estado":{"balls":{"4":30}}}"#,
        at+Duration::from_millis(30));
    assert!(g.status.capture_ready);
    assert_eq!(g.next_capture(at+Duration::from_millis(30)).unwrap().wire()["ballId"],4);
 }
 #[test]
 fn bounded_pending_window_never_reserves_more_than_small_nonrestocked_stock() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({
        "enabled":true,"ballPriority":["ultra"],"reserve":{"ultra":0}
    })));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":2}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":1},
        {"k":"morte","quem":"selvagem","slot":2},
        {"k":"morte","quem":"selvagem","slot":3}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert!(g.next_capture(at+Duration::from_millis(600)).is_some());
    assert!(g.next_capture(at+Duration::from_millis(1200)).is_none());
    assert_eq!(g.status.throws,2);
    assert_eq!(g.status.unreflected_throws,2);
    assert!(g.has_unreported_capture_timeout(at+Duration::from_secs(8)));
 }
 #[test]
 fn unrelated_vip_ball_receipts_do_not_reset_native_send_timer() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":20}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"morte","quem":"selvagem","slot":1},
        {"k":"morte","quem":"selvagem","slot":2}]}"#,at);
    assert!(g.next_capture(at).is_some());
    g.ingest(r#"{"t":"batalha","ev":[
        {"k":"bola","slot":99,"ballId":4,"sucesso":true}]}"#,
        at+Duration::from_millis(500));
    assert_eq!(g.status.server_catches,1);
    assert_eq!(g.status.catches,0);
    assert_eq!(g.next_capture(at+Duration::from_millis(600)).unwrap().wire()["slot"],2);
 }
 #[test]
 fn expired_pending_target_releases_its_ball_reservation() {
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,
        "ballPriority":["ultra"],"reserve":{"ultra":1}})));
    g.ingest(r#"{"t":"welcome","estado":{"noCentro":false,"balls":{"4":2}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":3}]}"#,at);
    assert!(g.next_capture(at).is_some());
    assert_eq!(g.available(4),1);
    g.expire_corpses(at+Duration::from_secs(30));
    g.refresh_readiness();
    assert_eq!(g.pending_for_ball(4),0);
    assert_eq!(g.status.unreflected_throws,0);
    assert_eq!(g.available(4),2);
 }
 #[test]
 fn native_capture_requires_enabled_mode_inventory_and_server_corpse(){
    let at=Instant::now();
    let mut g=Gameplay::new(Some(&json!({"enabled":true,"ballPriority":["ultra"]})));
    g.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"nome":"Ultra Ball"}],"estado":{"balls":{"4":3}}}"#,at);
    g.ingest(r#"{"t":"batalha","ev":[{"k":"morte","quem":"selvagem","slot":7}]}"#,at);
    assert_eq!(g.next_capture(at).unwrap().wire(),json!({"t":"ball.throw","ballId":4,"slot":7}));
    assert!(g.next_capture(at).is_none());
 }
}
