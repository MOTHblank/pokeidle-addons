//! Read-only Hunt Atlas reference: fetched only when Hunts tab opens.
//! Never execute public JavaScript or send credentials to the reference host.
//! The account's own welcome.hunts is authoritative for unlocked destinations.
use crate::native_hunts::{HuntEntry, HuntView, PokemonInfo, base_xp, base_gold};
use serde_json::Value;
use std::{collections::BTreeMap, io::Read, time::Duration};

const URL: &str = "https://guiapokeidlehardtocapture.site/data.js?v=32";
const HOST: &str = "guiapokeidlehardtocapture.site";
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ReferenceSpecies {
    pub id: u64,
    pub name: String,
    pub types: Vec<String>,
    pub base: Vec<f64>,
    pub moves: Vec<ReferenceMove>,
    pub drops: Vec<(f64, f64, f64, f64)>,
}
#[derive(Clone, Debug)]
pub struct ReferenceMove {
    pub move_type: String,
    pub special: bool,
    pub power: f64,
    pub cooldown: f64,
    pub min_level: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Reference {
    pub species: BTreeMap<u64, ReferenceSpecies>,
    pub chart: BTreeMap<String, BTreeMap<String, f64>>,
}
#[derive(Clone, Debug)]
pub struct Forecast {
    pub kills_h: f64,
    pub trainer_xp_h: f64,
    pub pokemon_xp_h: f64,
    pub gold_h: f64,
    pub seconds_per_kill: f64,
    pub coverage: f64,
    pub calibrated: bool,
}

pub fn parse(script: &str) -> Result<Reference, String> {
    let (_, data) = script.split_once("window.PI_DATA =")
        .ok_or_else(|| "Unexpected public hunt guide reference format".to_owned())?;
    let value: Value = serde_json::from_str(data.trim().trim_end_matches(';').trim())
        .map_err(|_| "Public guide reference is not valid JSON".to_owned())?;
    let all = value.get("especies").and_then(Value::as_array)
        .ok_or_else(|| "Guide has no species catalog".to_owned())?;
    let types = value.get("tipos").and_then(Value::as_object)
        .ok_or_else(|| "Guide has no type chart".to_owned())?;
    let mut result = Reference::default();
    for (attack, coverage) in types {
        if let Some(coverage) = coverage.as_object() {
            let mut row = BTreeMap::new();
            for (defend, effectiveness) in coverage {
                if let Some(n) = effectiveness.as_f64().filter(|n|n.is_finite() && *n >= 0.0) {
                    row.insert(defend.clone(),n);
                }
            }
            result.chart.insert(attack.clone(),row);
        }
    }
    for p in all {
        let Some(id) = p.get("id").and_then(Value::as_u64) else { continue };
        let Some(name) = p.get("n").and_then(Value::as_str) else { continue };
        let Some(types) = p.get("t").and_then(Value::as_array) else { continue };
        let Some(base) = p.get("b").and_then(Value::as_array) else { continue };
        if base.len() < 6 { continue }
        let base = base.iter().filter_map(Value::as_f64).collect::<Vec<_>>();
        if base.len() < 6 { continue }
        let mut moves = Vec::new();
        if let Some(entries) = p.get("m").and_then(Value::as_array) {
            for entry in entries {
                let Some(m) = entry.as_array().filter(|v|v.len()>=6) else { continue };
                let (Some(move_type),Some(power),Some(cooldown),Some(min_level)) =
                    (m[1].as_str(),m[3].as_f64(),m[4].as_f64(),m[5].as_u64()) else {continue};
                if power < 0.0 || cooldown <= 0.0 || !power.is_finite() {continue}
                moves.push(ReferenceMove {
                    move_type:move_type.to_owned(),
                    special:m[2].as_u64().unwrap_or(0)!=0,
                    power,cooldown,min_level:min_level.min(u32::MAX as u64) as u32
                });
            }
        }
        let mut drops=Vec::new();
        if let Some(loot)=p.get("l").and_then(Value::as_array) {
            for entry in loot {
                let Some(v)=entry.as_array().filter(|v|v.len()>=5) else {continue};
                if let (Some(chance),Some(low),Some(high),Some(price)) =
                    (v[1].as_f64(),v[2].as_f64(),v[3].as_f64(),v[4].as_f64()) {
                    drops.push((chance,low,high,price));
                }
            }
        }
        result.species.insert(id,ReferenceSpecies {
            id,name:name.to_owned(),
            types:types.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
            base,moves,drops
        });
    }
    if result.species.is_empty() || result.chart.is_empty() {
        return Err("Incomplete public hunt reference".into());
    }
    Ok(result)
}
pub fn load() -> Result<Reference, String> {
    crate::native_tls::ensure()?;
    let client=reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(18))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 2 || attempt.url().host_str()!=Some(HOST) {
                attempt.stop()
            } else { attempt.follow() }
        }))
        .build().map_err(|_| "Could not initialize guide downloader")?;
    let mut response=client.get(URL)
        .send().map_err(|e|format!("Guide unavailable: {e}"))?
        .error_for_status().map_err(|e|format!("Guide returned error: {e}"))?;
    if response.content_length().is_some_and(|n|n>MAX_BYTES) {
        return Err("Guide reference is larger than the allowed 4 MB".into());
    }
    let mut bytes=Vec::new();
    response.take(MAX_BYTES+1).read_to_end(&mut bytes)
        .map_err(|_|"Could not read guide reference".to_owned())?;
    if bytes.len() as u64 > MAX_BYTES {return Err("Guide reference exceeded size limit".into())}
    let script=std::str::from_utf8(&bytes)
        .map_err(|_|"Guide reference is not UTF-8".to_owned())?;
    parse(script)
}
fn effect(chart:&Reference, attack:&str, target:&ReferenceSpecies) -> f64 {
    target.types.iter().map(|defend|chart.chart.get(attack)
        .and_then(|row|row.get(defend)).copied().unwrap_or(1.0))
        .product()
}
fn damage(level:u32,power:f64,atk:f64,def:f64,stab:bool,multiplier:f64) -> f64 {
    let adjusted=if multiplier>1.0 {1.0+(multiplier-1.0)*1.5}
        else if multiplier<1.0 {multiplier/1.5} else {1.0};
    (((2.0*level as f64/5.0+2.0)*power*atk.max(1.0)/def.max(1.0))/50.0+2.0)
        *if stab{1.5}else{1.0}*adjusted*0.925
}
fn pokemon_stats(mon:&PokemonInfo, meta:&ReferenceSpecies)->Option<[f64;6]> {
    if mon.level==0||meta.base.len()<6 {return None}
    let quality=mon.quality.filter(|q|*q>0.0)?;
    let potency=mon.potency.filter(|n|(1..=5).contains(n))? as usize;
    let bonus=[0.0,0.05,0.1,0.25,1.0][potency-1];
    let exponent=[0.95,0.8,0.8,0.8,0.8,0.95];
    let mut stats=[0.0;6];
    for i in 0..6 {
        let iv=mon.ivs[i]? as f64;
        stats[i]=((meta.base[i]+2.0*iv)*mon.level as f64/100.0
            *quality.powf(exponent[i])*(1.0+bonus)
            *if mon.shiny{3.0}else{1.0}).round();
    }
    Some(stats)
}
fn dps(mon:&PokemonInfo, mon_meta:&ReferenceSpecies, stats:&[f64;6],
       wild:&ReferenceSpecies, level:u32, refdata:&Reference)->f64 {
    let iv_speed=mon.ivs[5].unwrap_or(16).clamp(1,32) as f64;
    let mut dps=0.0;
    let mut attacks=0.0;
    let def=(wild.base[2]+32.0)*level as f64/100.0;
    let tackle=damage(mon.level,30.0,stats[1],def,
        mon_meta.types.iter().any(|t|t=="NORMAL"),effect(refdata,"NORMAL",wild));
    for ability in mon_meta.moves.iter().filter(|m|m.min_level<=mon.level) {
        let attack=stats[if ability.special {3} else {1}];
        let defense=(wild.base[if ability.special{4}else{2}]+32.0)*level as f64/100.0;
        let cooldown=(ability.cooldown-iv_speed*10.0).max(400.0)/1000.0;
        let stab=mon_meta.types.iter().any(|t|t==&ability.move_type);
        dps+=damage(mon.level,ability.power,attack,defense,stab,
            effect(refdata,&ability.move_type,wild))/cooldown;
        attacks+=1.0/cooldown;
    }
    dps+tackle*((1.0-attacks*0.9).max(0.0)/2.0)
}
fn raw_kills(hunt:&HuntEntry, mon:&PokemonInfo, meta:&ReferenceSpecies,
             refdata:&Reference,stats:&[f64;6])->Option<(f64,f64,f64)> {
    let total=hunt.species.iter().map(|p|p.weight).sum::<f64>();
    if total<=0.0||hunt.level==0{return None}
    let mut secs=0.0;let mut loot=0.0;let mut coverage=0.0;
    for spawn in &hunt.species {
        let Some(wild)=refdata.species.get(&spawn.id) else {continue};
        let hp=(((wild.base[0]+32.0)*hunt.level as f64/100.0)*60.0).max(24.0);
        let kill=(hp/dps(mon,meta,stats,wild,hunt.level,refdata).max(1.0)).max(0.9);
        let ratio=spawn.weight.max(0.0)/total;
        secs+=ratio*kill;
        loot+=ratio*wild.drops.iter().map(|(p,lo,hi,price)|
            p/100.0*(lo+hi)/2.0*price).sum::<f64>();
        coverage+=ratio;
    }
    if coverage<0.6 {return None}
    let sec=(secs/coverage+2.8).max(0.9);
    let max_kills=3600.0/if hunt.level<20{4.2}else{3.2};
    Some(((3600.0/sec).min(max_kills),loot/coverage,coverage))
}
fn bonus(view:&HuntView, trainer:bool)->f64 {
    let b=&view.bonuses;
    // Same multiplicative structure as the community calculator, but mark
    // it partial when VIP/boost timing is not known from server clock.
    (if b.vip_active{1.5}else{1.0})
        *(1.0+b.guild_pct/100.0)
        *(if b.guild_boost_active{1.1}else{1.0})
        *(1.0+b.twitch_pct/100.0)
        *(1.0+if trainer{b.trainer_event_pct}else{b.pokemon_event_pct}/100.0)
}
pub fn estimate(view:&HuntView,hunt:&HuntEntry,reference:&Reference)->Option<Forecast> {
    let mon=view.active_pokemon.as_ref()?;
    let mon_meta=reference.species.get(&mon.species_id?)?;
    let stats=pokemon_stats(mon,mon_meta)?;
    let (base_kills,loot,coverage)=raw_kills(hunt,mon,mon_meta,reference,&stats)?;
    // The raw stat/move approximation is optimistic for many setups.
    // Calibrate against the current *observed* hunt's kills/h where possible;
    // never double-count Pokémon and trainer XP streams.
    let observed=&view.measurements;
    let current_prediction=view.hunt_slug.as_deref()
        .and_then(|slug|view.hunts.iter().find(|h|h.slug==slug))
        .and_then(|h|raw_kills(h,mon,mon_meta,reference,&stats));
    let calibration=if observed.seconds>=30.0 && observed.kills>=5 {
        current_prediction.and_then(|(k,_,_)|
            if k>0.0 {Some((observed.kills_h/k).clamp(0.1,2.0))}else{None})
    }else{None};
    let kills_h=(base_kills*calibration.unwrap_or(1.0))
        .min(3600.0/if hunt.level<20{4.2}else{3.2});
    Some(Forecast {
        kills_h,
        trainer_xp_h:base_xp(hunt.level)*kills_h*bonus(view,true),
        pokemon_xp_h:base_xp(hunt.level)*kills_h*bonus(view,false),
        gold_h:(base_gold(hunt.level)+loot)*kills_h,
        seconds_per_kill:if kills_h>0.0{3600.0/kills_h}else{0.0},
        coverage,
        calibrated:calibration.is_some()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_json_only_and_rejects_executable_script(){
        let text=r#"window.PI_DATA = {"tipos":{"NORMAL":{"GHOST":0.0}},"especies":[{"id":1,"n":"A","t":["NORMAL"],"b":[1,2,3,4,5,6],"m":[]}],"hunts":[]};"#;
        assert_eq!(parse(text).unwrap().species.len(),1);
        assert!(parse("window.PI_DATA = alert(document.cookie);").is_err());
        assert!(parse("alert('hi');").is_err());
    }
    #[test]
    fn xp_calculation_keeps_separate_channels(){
        let mut view=HuntView::default();
        view.bonuses.trainer_event_pct=100.0;
        view.bonuses.pokemon_event_pct=0.0;
        assert_eq!(bonus(&view,true),2.0);
        assert_eq!(bonus(&view,false),1.0);
    }
}
