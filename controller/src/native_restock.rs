//! Server-priced replenishment of configured Auto Catch ball stock.
use crate::native_gameplay::GameCommand;
use serde_json::Value;
use std::time::{Duration,Instant};
use std::collections::BTreeMap;
#[derive(Clone,Debug)]
struct Order { kind:&'static str, id:u64, at:Instant, ack:bool }
struct Rule { kind:&'static str, id:u64, low:u64, target:u64 }
pub struct Restock {
    enabled:bool,rules:Vec<Rule>,reserve:u64,
    prices:BTreeMap<(&'static str,u64),u64>,stock:BTreeMap<(&'static str,u64),u64>,
    gold:Option<u64>,pending:Option<Order>,cooldown:Option<Instant>,
}
impl Restock {
    pub fn new(settings:Option<&Value>)->Self {
        let restock=settings.and_then(|v|v.get("restock"));
        let num=|key:&str|restock.and_then(|v|v.get(key))
            .and_then(Value::as_u64).unwrap_or(0);
        let mut rules=Vec::new();
        for (kind,id_key,low_key,to_key) in [
            ("ball","selectedBallId","ballAt","ballTo"),
            ("item","selectedPotionId","potionAt","potionTo"),
            ("item","selectedReviveId","reviveAt","reviveTo")
        ] {
            let id=num(id_key);
            if id>0 && num(to_key)>num(low_key) {
                rules.push(Rule{kind,id,low:num(low_key),target:num(to_key)});
            }
        }
        Self {
            enabled:settings.and_then(|v|v.get("enabled"))
                .and_then(Value::as_bool)==Some(true) &&
                restock.and_then(|v|v.get("enabled"))
                .and_then(Value::as_bool)==Some(true),
            rules,reserve:num("goldReserve"),prices:BTreeMap::new(),
            stock:BTreeMap::new(),gold:None,pending:None,cooldown:None
        }
    }
    pub fn ingest(&mut self,raw:&str,now:Instant) {
        let Ok(v)=serde_json::from_str::<Value>(raw) else{return};
        if v.get("t").and_then(Value::as_str)==Some("welcome") {
            self.pending=None;
            for (kind,field) in [("ball","catalogoBolas"),("item","itensCompraveis")] {
                if let Some(catalog)=v.get(field).and_then(Value::as_array) {
                    for item in catalog {
                        if item.get("compravel").and_then(Value::as_bool)==Some(false){continue;}
                        let id=item.get("itemId").or_else(||item.get("id")).and_then(Value::as_u64);
                        let price=item.get("priceGold").or_else(||item.get("preco"))
                            .or_else(||item.get("npcPrice")).and_then(Value::as_u64);
                        if let (Some(id),Some(price))=(id,price){
                            if price>0 {self.prices.insert((kind,id),price);}
                        }
                    }
                }
            }
        }
        if let Some(state)=v.get("estado") {
            if let Some(gold)=state.get("gold").and_then(Value::as_u64){self.gold=Some(gold);}
            for (kind,field) in [("ball","balls"),("item","items")] {
                if let Some(items)=state.get(field).and_then(Value::as_object) {
                    self.stock.retain(|(stored_kind,_),_|*stored_kind!=kind);
                    for (id,amount) in items {
                        if let (Ok(id),Some(amount))=(id.parse::<u64>(),amount.as_u64()){
                            self.stock.insert((kind,id),amount);
                        }
                    }
                    if self.pending.as_ref().is_some_and(|p|p.kind==kind && p.ack){
                        self.pending=None;
                        self.cooldown=Some(now+Duration::from_secs(2));
                    }
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
        if !catch_enabled||!self.enabled{return None;}
        if let Some(p)=&self.pending {
            if now.saturating_duration_since(p.at)<Duration::from_secs(10){return None;}
            self.pending=None;
            self.cooldown=Some(now+Duration::from_secs(30));
        }
        if self.cooldown.is_some_and(|at|now<at){return None;}
        let gold=self.gold?;
        for rule in &self.rules {
            let Some(&price)=self.prices.get(&(rule.kind,rule.id)) else {continue};
            let amount=self.stock.get(&(rule.kind,rule.id)).copied().unwrap_or(0);
            if amount>rule.low||amount>=rule.target||price==0{continue;}
            let qty=(rule.target-amount).min(5000)
                .min(gold.saturating_sub(self.reserve)/price);
            if qty==0{continue;}
            self.pending=Some(Order{kind:rule.kind,id:rule.id,at:now,ack:false});
            return Some(GameCommand::ShopBuy{kind:rule.kind.into(),id:rule.id,qty});
        }
        None
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
