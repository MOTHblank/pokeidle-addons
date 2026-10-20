//! Native Auto Catch configuration, compatible with the legacy v4 settings.
//! The worker and UI share this validated structure; browser localStorage is
//! an import source only, not the owner once Rust has authenticated.
use serde_json::{json, Value};
use std::collections::BTreeMap;

const BALLS: [&str; 5] = ["beast", "ultra", "super", "great", "poke"];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatchSettings {
    pub enabled: bool,
    pub priorities: Vec<String>,
    /// Milliseconds between sends, matching Auto Catch+ intervalMs.
    pub interval_ms: u64,
    pub reserves: BTreeMap<String,u64>,
    pub restock_enabled: bool,
    pub gold_reserve: u64,
    pub ball_id: u64,
    pub potion_id: u64,
    pub revive_id: u64,
    pub ball_at: u64,
    pub ball_to: u64,
    pub potion_at: u64,
    pub potion_to: u64,
    pub revive_at: u64,
    pub revive_to: u64,
}
impl Default for CatchSettings {
    fn default() -> Self {
        Self {
            enabled:false, priorities:vec!["ultra".into()], interval_ms:100,
            reserves:BALLS.iter().map(|key|(key.to_string(),0)).collect(),
            restock_enabled:false,gold_reserve:0,
            ball_id:4,potion_id:200,revive_id:0,
            ball_at:50,ball_to:500,potion_at:10,potion_to:50,
            revive_at:1,revive_to:5
        }
    }
}
impl CatchSettings {
    pub fn from_value(value:Option<&Value>)->Self {
        let mut cfg=Self::default();
        let Some(v)=value else {return cfg};
        cfg.enabled=v.get("enabled").and_then(Value::as_bool).unwrap_or(false);
        // The previous native build persisted its 1200-ms default. Upgrade
        // that legacy value once; a v2-tagged user-selected 1200 ms remains valid.
        let old_interval=v.get("intervalMs").and_then(Value::as_u64).unwrap_or(100);
        let old_interval=if old_interval==1200 &&
            v.get("captureTimingV2").and_then(Value::as_bool)!=Some(true) {
            600
        } else {old_interval};
        cfg.interval_ms=if old_interval==600 && v.get("captureTimingV3").and_then(Value::as_bool)!=Some(true) {100} else {old_interval.clamp(100,3000)};
        if let Some(list)=v.get("ballPriority").and_then(Value::as_array) {
            cfg.priorities=list.iter().filter_map(Value::as_str)
                .map(str::to_lowercase).filter(|p|BALLS.contains(&p.as_str()))
                .collect::<Vec<_>>();
            cfg.priorities.dedup();
            cfg.priorities.truncate(2);
        }
        if let Some(reserve)=v.get("reserve") {
            for key in BALLS {
                if let Some(n)=reserve.get(key).and_then(Value::as_u64) {
                    cfg.reserves.insert(key.to_owned(),n);
                }
            }
        }
        if let Some(restock)=v.get("restock") {
            let num=|key:&str,old:u64|restock.get(key).and_then(Value::as_u64).unwrap_or(old);
            cfg.restock_enabled=restock.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            cfg.gold_reserve=num("goldReserve",cfg.gold_reserve);
            cfg.ball_id=num("selectedBallId",cfg.ball_id);
            cfg.potion_id=num("selectedPotionId",cfg.potion_id);
            cfg.revive_id=num("selectedReviveId",cfg.revive_id);
            cfg.ball_at=num("ballAt",cfg.ball_at);
            cfg.ball_to=num("ballTo",cfg.ball_to);
            cfg.potion_at=num("potionAt",cfg.potion_at);
            cfg.potion_to=num("potionTo",cfg.potion_to);
            cfg.revive_at=num("reviveAt",cfg.revive_at);
            cfg.revive_to=num("reviveTo",cfg.revive_to);
        }
        cfg
    }
    pub fn validate(&self)->Result<(),String> {
        if !(100..=3000).contains(&self.interval_ms) {
            return Err("Auto Catch interval must be 100–3000 ms".into());
        }
        if self.priorities.len()>2 || self.priorities.iter().any(|p|!BALLS.contains(&p.as_str())) ||
            (self.priorities.len()==2 && self.priorities[0]==self.priorities[1]) {
            return Err("Choose at most two different supported balls; Master Balls are not automated".into());
        }
        if self.enabled && self.priorities.is_empty() {
            return Err("Select at least one Auto Catch ball before enabling".into());
        }
        if self.priorities.iter().any(|p|self.reserves.get(p).copied().unwrap_or(0)>1_000_000) {
            return Err("Ball reserve exceeds one million".into());
        }
        for (id,low,target,name) in [
            (self.ball_id,self.ball_at,self.ball_to,"ball"),
            (self.potion_id,self.potion_at,self.potion_to,"potion"),
            (self.revive_id,self.revive_at,self.revive_to,"revive")
        ] {
            if id > 1_000_000 || low > 1_000_000 || target > 1_000_000 {
                return Err(format!("Invalid {name} ID or stock threshold"));
            }
            if self.restock_enabled && id>0 && (target<=low || target==0) {
                return Err(format!("{name} target must be greater than the low-stock threshold"));
            }
        }
        if self.restock_enabled && self.ball_id==0 && self.potion_id==0 && self.revive_id==0 {
            return Err("Select at least one restock item".into());
        }
        Ok(())
    }
    pub fn as_value(&self)->Value {
        let mut reserve=serde_json::Map::new();
        for (k,v) in &self.reserves {reserve.insert(k.clone(),json!(v));}
        json!({
            "enabled":self.enabled,
            "ballPriority":self.priorities,
            "intervalMs":self.interval_ms,
            "captureTimingV2":true,
            "captureTimingV3":true,
            "reserve":reserve,
            "restock":{
                "enabled":self.restock_enabled,
                "goldReserve":self.gold_reserve,
                "selectedBallId":self.ball_id,
                "selectedPotionId":self.potion_id,
                "selectedReviveId":if self.revive_id==0 {Value::Null}else{json!(self.revive_id)},
                "ballAt":self.ball_at,"ballTo":self.ball_to,
                "potionAt":self.potion_at,"potionTo":self.potion_to,
                "reviveAt":self.revive_at,"reviveTo":self.revive_to
            }
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ultra_ball_is_default_without_overwriting_saved_preferences() {
        let defaults=CatchSettings::default();
        assert_eq!(defaults.priorities,vec!["ultra".to_owned()]);
        assert_eq!(defaults.ball_id,4);
        let partial=CatchSettings::from_value(Some(&json!({
            "enabled":true,"restock":{"enabled":true}
        })));
        assert_eq!(partial.priorities,vec!["ultra".to_owned()]);
        assert_eq!(partial.ball_id,4);
        let customized=CatchSettings::from_value(Some(&json!({
            "enabled":true,"ballPriority":["beast","great"],
            "restock":{"selectedBallId":2}
        })));
        assert_eq!(customized.priorities,vec!["beast".to_owned(),"great".to_owned()]);
        assert_eq!(customized.ball_id,2);
    }
    #[test]
    fn legacy_settings_round_trip_and_reserve_survives() {
        let old=json!({"enabled":true,"ballPriority":["beast","ultra"],
            "reserve":{"beast":5},"restock":{"enabled":true,
                "selectedBallId":4,"ballAt":15,"ballTo":300,"goldReserve":75}});
        let cfg=CatchSettings::from_value(Some(&old));
        assert_eq!(cfg.reserves["beast"],5);
        assert_eq!(cfg.potion_id,200);
        cfg.validate().unwrap();
        assert_eq!(CatchSettings::from_value(Some(&cfg.as_value())),cfg);
    }
    #[test]
    fn interval_round_trip_and_bounds() {
        let config=CatchSettings::from_value(Some(&json!({
            "enabled":true,"intervalMs":1500,"ballPriority":["ultra"]
        })));
        assert_eq!(config.interval_ms,1500);
        assert_eq!(config.as_value()["intervalMs"],1500);
        let mut invalid=config.clone();
        invalid.interval_ms=0;
        assert!(invalid.validate().is_err());
        invalid.interval_ms=3001;
        assert!(invalid.validate().is_err());
        assert_eq!(CatchSettings::from_value(Some(&json!({"intervalMs":20}))).interval_ms,100);
    }
    #[test]
    fn legacy_native_default_upgrades_to_100ms_but_explicit_new_interval_is_preserved() {
        let old=CatchSettings::from_value(Some(&json!({
            "enabled":true,"intervalMs":1200
        })));
        assert_eq!(old.interval_ms,100);
        let new=CatchSettings::from_value(Some(&json!({
            "enabled":true,"intervalMs":1200,"captureTimingV2":true
        })));
        assert_eq!(new.interval_ms,1200);
        assert_eq!(CatchSettings::from_value(Some(&new.as_value())),new);
        assert_eq!(CatchSettings::default().interval_ms,100);
    }
    #[test]
    fn golden_potion_selection_cannot_overwrite_revive_selection() {
        let mut cfg = CatchSettings::default();
        cfg.restock_enabled = true;
        cfg.potion_id = 70070;
        cfg.revive_id = 206;
        cfg.validate().unwrap();
        let saved = cfg.as_value();
        assert_eq!(saved["restock"]["selectedPotionId"], 70070);
        assert_eq!(saved["restock"]["selectedReviveId"], 206);
        assert_eq!(CatchSettings::from_value(Some(&saved)), cfg);
        cfg.revive_id = 0;
        let saved = cfg.as_value();
        assert_eq!(saved["restock"]["selectedPotionId"], 70070);
        assert!(saved["restock"]["selectedReviveId"].is_null());
        assert_eq!(CatchSettings::from_value(Some(&saved)), cfg);
    }

    #[test]
    fn no_master_ball_and_invalid_restock_targets() {
        let mut cfg=CatchSettings::default();
        cfg.priorities=vec!["master".into()];
        assert!(cfg.validate().is_err());
        cfg.priorities=vec!["ultra".into()];
        cfg.restock_enabled=true;
        cfg.ball_to=1;
        assert!(cfg.validate().is_err());
    }
}
