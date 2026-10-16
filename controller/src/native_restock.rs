//! Server-priced replenishment of configured Auto Catch ball stock.
use crate::native_gameplay::GameCommand;
use serde_json::Value;
use std::time::{Duration,Instant};
#[derive(Clone,Debug)]
struct Order { id:u64, at:Instant, ack:bool }
pub struct Restock {
    enabled:bool,id:u64,low:u64,target:u64,reserve:u64,
    price:Option<u64>,amount:Option<u64>,gold:Option<u64>,
    pending:Option<Order>,cooldown:Option<Instant>,
}
impl Restock {
    pub fn new(settings:Option<&Value>)->Self {
        let restock=settings.and_then(|v|v.get("restock"));
        let num=|key:&str|restock.and_then(|v|v.get(key))
            .and_then(Value::as_u64).unwrap_or(0);
        Self {
            enabled:settings.and_then(|v|v.get("enabled"))
                .and_then(Value::as_bool)==Some(true) &&
                restock.and_then(|v|v.get("enabled"))
                .and_then(Value::as_bool)==Some(true),
            id:num("selectedBallId"),low:num("ballAt"),target:num("ballTo"),
            reserve:num("goldReserve"),price:None,amount:None,gold:None,
            pending:None,cooldown:None
        }
    }
    pub fn ingest(&mut self,raw:&str,now:Instant) {
        let Ok(v)=serde_json::from_str::<Value>(raw) else{return};
        if v.get("t").and_then(Value::as_str)==Some("welcome") {
            self.pending=None;
            if let Some(catalog)=v.get("catalogoBolas").and_then(Value::as_array) {
                for entry in catalog {
                    if entry.get("id").and_then(Value::as_u64)!=Some(self.id) {continue;}
                    if entry.get("compravel").and_then(Value::as_bool)==Some(false) {continue;}
                    self.price=entry.get("priceGold").and_then(Value::as_u64).filter(|v|*v>0);
                }
            }
        }
        if let Some(state)=v.get("estado") {
            if let Some(gold)=state.get("gold").and_then(Value::as_u64){self.gold=Some(gold);}
            if let Some(balls)=state.get("balls").and_then(Value::as_object){
                self.amount=Some(balls.get(&self.id.to_string())
                    .and_then(Value::as_u64).unwrap_or(0));
                if self.pending.as_ref().is_some_and(|p|p.ack) {
                    self.pending=None;self.cooldown=Some(now+Duration::from_secs(2));
                }
            }
        }
        if let Some(events)=v.get("ev").and_then(Value::as_array) {
            if events.iter().any(|event|event.get("k").and_then(Value::as_str)==Some("compra")) {
                if let Some(p)=self.pending.as_mut(){p.ack=true;}
            }
        }
    }
    pub fn next(&mut self,now:Instant,catch_enabled:bool)->Option<GameCommand> {
        if !catch_enabled||!self.enabled||self.id==0||self.target<=self.low{return None;}
        if let Some(p)=&self.pending {
            if now.saturating_duration_since(p.at)<Duration::from_secs(10){return None;}
            self.pending=None;
            self.cooldown=Some(now+Duration::from_secs(30));
        }
        if self.cooldown.is_some_and(|at|now<at){return None;}
        let price=self.price?;
        let amount=self.amount?;
        let gold=self.gold?;
        if amount>self.low||amount>=self.target||price==0{return None;}
        let qty=(self.target-amount).min(5000)
            .min(gold.saturating_sub(self.reserve)/price);
        if qty==0{return None;}
        self.pending=Some(Order{id:self.id,at:now,ack:false});
        Some(GameCommand::ShopBuy{kind:"ball".into(),id:self.id,qty})
    }
    pub fn failed(&mut self) {
        self.pending=None;self.cooldown=Some(Instant::now()+Duration::from_secs(30));
    }
}
#[cfg(test)]
mod tests {
    use super::*;use serde_json::json;
    #[test]
    fn respects_gold_reserve_and_single_in_flight_purchase() {
        let at=Instant::now();
        let mut r=Restock::new(Some(&json!({"enabled":true,"restock":{
            "enabled":true,"selectedBallId":4,"ballAt":5,"ballTo":20,"goldReserve":100
        }})));
        r.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"priceGold":10,"compravel":true}],"estado":{"gold":190,"balls":{"4":3}}}"#,at);
        assert_eq!(r.next(at,true).unwrap().wire(),json!({"t":"shop.buy","kind":"ball","id":4,"qty":9}));
        assert!(r.next(at,true).is_none());
    }
}
