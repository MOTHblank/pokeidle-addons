//! Read-only Hunt Atlas reference: fetched only when Hunts tab opens.
//! Never execute public JavaScript or send credentials to the reference host.
//! The account's own welcome.hunts is authoritative for unlocked destinations.
use crate::native_hunts::{HuntEntry, HuntSpecies, HuntView, PokemonInfo, base_xp, gross_gold_per_kill};
use serde_json::Value;
use std::{collections::{BTreeMap,BTreeSet}, io::Read, sync::OnceLock, time::Duration};

const URL: &str = "https://guiapokeidlehardtocapture.site/data.js?v=32";
const HOST: &str = "guiapokeidlehardtocapture.site";
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ReferenceSpecies {
    pub name: String,
    pub types: Vec<String>,
    pub base: Vec<f64>,
    pub moves: Vec<ReferenceMove>,
    pub drops: Vec<ReferenceDrop>,
}
#[derive(Clone, Debug)]
pub struct ReferenceDrop {
    pub item_id: Option<u64>,
    /// The public guide often identifies loot by a name, not by a numeric ID.
    pub item_name: Option<String>,
    pub chance_pct: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub guide_price: f64,
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
    /// Public-guide destinations, INCLUDING locked ones. Never infer game
    /// unlocks from these; only the authenticated welcome.hunts may do that.
    pub hunts: Vec<HuntEntry>,
    /// Public client-catalog derived capture rarity (not guide loot prices).
    pub capture_rarities: BTreeMap<u64,f64>,
    /// Upstream evolutionary-stage capture cap (20/40/100), inferred from
    /// current public creature evolution chains after all catalogs merge.
    pub capture_stages: BTreeMap<u64,u32>,
    capture_catalog_ids: BTreeSet<u64>,
    capture_evolutions: BTreeMap<u64,u64>,
    pub npc_sell_prices: BTreeMap<u64,f64>,
    /// Static catalog sellValue; not an observed player-to-player market average.
    pub catalog_sell_values: BTreeMap<u64,f64>,
}
/// Lendários, míticos e Megas seguem contratos próprios: não devem
/// receber chances de captura normais, mesmo que apareçam em um catálogo
/// antigo de hunts ou em uma previsão de spawn.
pub fn special_capture_species(id:u64)->bool{
    static IDS:OnceLock<(BTreeSet<u64>,BTreeMap<u64,u64>,BTreeSet<u64>)>=OnceLock::new();
    let (special,forms,megas)=IDS.get_or_init(||{
        let special=include_str!("../assets/visual/upstream-special-dex.tsv")
            .lines().filter_map(|line|line.parse::<u64>().ok())
            .collect::<BTreeSet<_>>();
        let forms=include_str!("../assets/visual/upstream-form-base.tsv")
            .lines().filter_map(|line|{
                let (id,dex)=line.split_once('\t')?;
                Some((id.parse::<u64>().ok()?,dex.parse::<u64>().ok()?))
            }).collect::<BTreeMap<_,_>>();
        let megas=include_str!("../assets/visual/upstream-megas.tsv")
            .lines().filter_map(|line|line.split('\t').next()?.parse::<u64>().ok())
            .collect::<BTreeSet<_>>();
        (special,forms,megas)
    });
    if megas.contains(&id){return true}
    let dex=if id>=13000{id%1000}else{forms.get(&id).copied().unwrap_or(id)};
    special.contains(&dex)
}
/// Ordinary hunts use stages exclusively. An incomplete stage catalog
/// must produce an unknown chance, NEVER a price-based chance from old rules.
/// Non-hunt special encounters have separate models outside Hunt Atlas.
pub fn capture_inputs(reference:&Reference,id:u64)->(Option<u32>,Option<f64>){
    if special_capture_species(id){return (None,None)}
    (reference.capture_stages.get(&id).copied(),None)
}
#[derive(Clone, Debug)]
pub struct Forecast {
    pub kills_h: f64,
    pub trainer_xp_h: f64,
    pub pokemon_xp_h: f64,
    pub gold_h: f64,
    /// Historical NPC field retained for compatibility; no NPC drops are modeled.
    pub combat_gold_h: f64,
    pub npc_loot_gold_h: f64,
    pub trainer_xp_per_kill: f64,
    pub pokemon_xp_per_kill: f64,
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
        // Legacy guide NPC drops are no longer awarded by the game.
        let drops=Vec::new();
        result.species.insert(id,ReferenceSpecies {
            name:name.to_owned(),
            types:types.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
            base,moves,drops
        });
    }
    if let Some(hunts)=value.get("hunts").and_then(Value::as_array) {
        let mut dedup=std::collections::BTreeSet::new();
        for raw in hunts.iter().take(10_000) {
            let (Some(slug),Some(name),Some(level))=(
                raw.get("s").and_then(Value::as_str),
                raw.get("n").and_then(Value::as_str),
                raw.get("nv").and_then(Value::as_u64)
            ) else {continue};
            if slug.is_empty() || slug.len()>128 || !slug.bytes()
                .all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_')
                || name.is_empty() || name.len()>160
                || level==0 || level>u32::MAX as u64 || !dedup.insert(slug.to_owned())
            {continue}
            let species=raw.get("e").and_then(Value::as_array)
                .map(|list|list.iter().filter_map(|row|{
                    let pair=row.as_array().filter(|a|a.len()>=2)?;
                    let id=pair[0].as_u64()?;
                    let weight=pair[1].as_f64()?;
                    if id==0 || !weight.is_finite() || weight<=0.0 {return None}
                    Some(HuntSpecies{id,weight})
                }).collect::<Vec<_>>()).unwrap_or_default();
            if species.is_empty(){continue}
            result.hunts.push(HuntEntry {
                slug:slug.to_owned(),name:name.to_owned(),
                area:raw.get("a").and_then(Value::as_str).unwrap_or("").to_owned(),
                region:String::new(),level:level as u32,
                species,total_spawns:0,looktype:None,
            });
        }
    }
    if result.species.is_empty() || result.chart.is_empty() {
        return Err("Incomplete public hunt reference".into());
    }
    Ok(result)
}
/// Fallback from the upstream game's elemental effectiveness contract.
/// The public guide is helpful for locked destination previews but must not
/// be a single point of failure for authenticated live Hunt Atlas data.
fn built_in_type_chart()->BTreeMap<String,BTreeMap<String,f64>>{
    const TYPES:[&str;18]=[
        "NORMAL","FIRE","WATER","ELECTRIC","GRASS","ICE","FIGHTING",
        "POISON","GROUND","FLYING","PSYCHIC","BUG","ROCK","GHOST",
        "DRAGON","DARK","STEEL","FAIRY"
    ];
    let mut chart=TYPES.iter().map(|kind|
        ((*kind).to_owned(),BTreeMap::new()))
        .collect::<BTreeMap<_,_>>();
    for line in include_str!("../assets/visual/upstream-type-chart.tsv").lines(){
        if line.starts_with('#'){continue}
        let mut fields=line.split('\t');
        let (Some(attack),Some(defend),Some(raw))=
            (fields.next(),fields.next(),fields.next())else{continue};
        let Ok(value)=raw.parse::<f64>()else{continue};
        if !value.is_finite()||value<0.0 || !chart.contains_key(defend){continue}
        if let Some(row)=chart.get_mut(attack){row.insert(defend.to_owned(),value);}
    }
    chart
}
pub fn load() -> Result<Reference, String> {
    crate::native_tls::ensure()?;
    let client=reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 2 || attempt.url().host_str()!=Some(HOST) {
                attempt.stop()
            } else { attempt.follow() }
        }))
        .build().map_err(|_| "Could not initialize guide downloader")?;
    let guide=(||->Result<Reference,String>{
        let response=client.get(URL)
            .send().map_err(|e|format!("Guide unavailable: {e}"))?
            .error_for_status().map_err(|e|format!("Guide returned error: {e}"))?;
        if response.content_length().is_some_and(|n|n>MAX_BYTES){
            return Err("Guide reference is larger than the allowed 4 MB".into())
        }
        let mut bytes=Vec::new();
        response.take(MAX_BYTES+1).read_to_end(&mut bytes)
            .map_err(|_|"Could not read guide reference".to_owned())?;
        if bytes.len() as u64>MAX_BYTES{
            return Err("Guide reference exceeded size limit".into())
        }
        let script=std::str::from_utf8(&bytes)
            .map_err(|_|"Guide reference is not UTF-8".to_owned())?;
        parse(script)
    })();
    let mut reference=match guide {
        Ok(guide)=>guide,
        Err(error)=>{
            crate::logging::warn(&format!(
                "Hunt Atlas guide unavailable; using game catalogs: {error}"));
            Reference::default()
        }
    };
    if reference.chart.is_empty(){reference.chart=built_in_type_chart();}
    // Only public static assets. Never send BiDi credentials or game cookies.
    // Missing optional assets do NOT prevent Atlas from loading.
    let assets=reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(7))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            if attempt.previous().len()>1 ||
                attempt.url().host_str()!=Some("pokeidle.io"){
                attempt.stop()
            }else{attempt.follow()}
        }))
        .build().map_err(|_|"Could not initialize static asset downloader")?;
    for name in ["creatures.json","creatures-novos.json",
        "creatures-outland-novos.json","creatures-formas.json",
        "creatures-audit-overrides.json"]{
        let uri=format!("https://pokeidle.io/assets/{name}");
        let Ok(response)=assets.get(&uri).send()
            .and_then(reqwest::blocking::Response::error_for_status)
            else{continue};
        if response.content_length().is_some_and(|length|length>5_242_880){
            continue;
        }
        let mut body=Vec::new();
        if response.take(5_242_881).read_to_end(&mut body).is_err()
            ||body.len()>5_242_880 {continue}
        if let Ok(catalog)=serde_json::from_slice::<Value>(&body){
            merge_creature_catalog(&mut reference,&catalog);
        }
    }
    if reference.species.is_empty(){
        return Err("Neither the guide nor the game's public creature catalogs could be loaded".into())
    }
    finalize_capture_stages(&mut reference);
    merge_upstream_megas(&mut reference);
    Ok(reference)
}
/// A creature's player-market listing must never become NPC/capture rarity.
/// The identical merge order is used by the userscript (base, new, Outland,
/// overrides); absent override fields preserve earlier valid values.
pub fn merge_creature_catalog(reference:&mut Reference,data:&Value){
    let rows=data.get("creatures").or_else(||data.get("overrides"))
        .and_then(Value::as_array);
    let Some(rows)=rows else{return};
    for raw in rows.iter().take(30_000){
        let Some(raw_id)=raw.get("pokeId").and_then(Value::as_u64)
            else{continue};
        let id=if (10501..10548).contains(&raw_id){raw_id-8500}
            else{raw_id};
        let level=raw.get("huntLevel").and_then(Value::as_u64)
            .filter(|v|*v<=u32::MAX as u64).map(|v|v as u32);
        let positive=|key:&str|raw.get(key).and_then(Value::as_f64)
            .filter(|n|n.is_finite()&&*n>0.0);
        let sell=positive("sellValue");
        let npc=positive("priceNpc");
        if let Some(price)=sell {reference.catalog_sell_values.insert(id,price);}
        if let Some(price)=npc.or(sell) {
            reference.npc_sell_prices.insert(id,price);
        }
        if let Some(rarity)=crate::native_atlas_models::catalog_rarity(level,sell,npc){
            reference.capture_rarities.insert(id,rarity);
        }
        // The game's capture stage is determined by evolutionary links, not
        // by market price or hunt level. Hold the edges until every catalog
        // (including regional forms) has been merged.
        if let Some(name)=raw.get("name").and_then(Value::as_str)
            .filter(|s|!s.is_empty()) {
            reference.capture_catalog_ids.insert(id);
            if let Some(next)=raw.get("evolvesToId").and_then(Value::as_u64)
                .filter(|v|*v>0) {
                let next=if (10501..10548).contains(&next){next-8500}else{next};
                reference.capture_evolutions.insert(id,next);
            }
            let types=["type1","type2"].iter().filter_map(|key|
                raw.get(*key).and_then(Value::as_str)
                    .filter(|s|!s.is_empty())).map(str::to_owned)
                .collect::<Vec<_>>();
            let base=["baseHp","baseAtk","baseDef","baseSpAtk","baseSpDef","baseSpeed"]
                .iter().filter_map(|key|raw.get(*key).and_then(Value::as_f64))
                .collect::<Vec<_>>();
            if types.len()>0 && base.len()==6 && base.iter().all(|n|n.is_finite()&&*n>0.0) {
                let mut moves=raw.get("attacks").and_then(Value::as_array)
                    .map(|arr|arr.iter().filter_map(|attack|{
                        let move_type=attack.get("type")?.as_str()?;
                        let power=attack.get("power")?.as_f64()?;
                        let cooldown=attack.get("cooldownMs")?.as_f64()?;
                        if !power.is_finite()||power<0.0||!cooldown.is_finite()||cooldown<=0.0{return None}
                        Some(ReferenceMove{
                            move_type:move_type.to_owned(),
                            special:attack.get("category").and_then(Value::as_str)
                                ==Some("SPECIAL"),
                            power,cooldown,
                            min_level:attack.get("learnLevel").and_then(Value::as_u64)
                                .unwrap_or(1).min(u32::MAX as u64) as u32,
                        })
                    }).collect::<Vec<_>>()).unwrap_or_default();
                if moves.is_empty() {
                    if let Some(existing)=reference.species.get(&id) {
                        moves=existing.moves.clone();
                    }
                }
                reference.species.insert(id,ReferenceSpecies{
                    name:name.to_owned(),types,base,moves,drops:Vec::new(),
                });
            }
        }
    }
}

/// Apply the upstream teto-captura evolution-chain ladder after merging.
/// 3 links: 20/40/100; 2 links: 40/100; singleton/Outland: 100.
/// We require a public catalog identity; missing data keeps the older
/// rarity model rather than asserting an invented evolutionary stage.
pub fn finalize_capture_stages(reference:&mut Reference){
    let mut before:BTreeMap<u64,u32>=reference.capture_catalog_ids.iter()
        .map(|id|(*id,0)).collect();
    let mut after=before.clone();
    let mut links=reference.capture_evolutions.iter()
        .map(|(&from,&to)|(from,to)).collect::<Vec<_>>();
    // Include upstream branched evolutions (Eevee, Tyrogue, regional forms,
    // clone lines). evolvesToId alone does not describe these families.
    for line in include_str!("../assets/visual/upstream-evolution-branches.tsv").lines(){
        if line.starts_with('#'){continue}
        let mut values=line.split('\t');
        if let (Some(from),Some(to))=(values.next(),values.next()){
            if let (Ok(from),Ok(to))=(from.parse::<u64>(),to.parse::<u64>()){
                links.push((from,to));
            }
        }
    }
    // Cross-generation and repaired/truncated links are conditional upstream:
    // a real evolvesToId always wins over the fallback.
    for line in include_str!("../assets/visual/upstream-evolution-cross.tsv").lines(){
        if line.starts_with('#'){continue}
        let mut values=line.split('\t');
        if let (Some(from),Some(to))=(values.next(),values.next()){
            if let (Ok(from),Ok(to))=(from.parse::<u64>(),to.parse::<u64>()){
                if !reference.capture_evolutions.contains_key(&from){
                    links.push((from,to));
                }
            }
        }
    }
    for _ in 0..8 {
        let mut changed=false;
        for &(from,to) in &links {
            if !reference.capture_catalog_ids.contains(&from)
                ||!reference.capture_catalog_ids.contains(&to)||from==to{continue}
            let previous=before.get(&from).copied().unwrap_or(0);
            if previous+1>before.get(&to).copied().unwrap_or(0) {
                before.insert(to,previous+1);changed=true;
            }
            let next=after.get(&to).copied().unwrap_or(0);
            if next+1>after.get(&from).copied().unwrap_or(0) {
                after.insert(from,next+1);changed=true;
            }
        }
        if !changed {break}
    }
    reference.capture_stages.clear();
    for &id in &reference.capture_catalog_ids{
        let depth=before.get(&id).copied().unwrap_or(0).min(3);
        let forward=after.get(&id).copied().unwrap_or(0).min(3);
        let length=(depth+forward+1).min(4);
        let cap=match (length,depth) {
            (1,_)=>100, (2,0)=>40, (2,_)=>100,
            (3,0)=>20, (3,1)=>40, (3,_)=>100,
            (4,0)=>20, (4,1)=>40, (4,2)=>70, _=>100,
        };
        // Outland variants are intentionally unevolving, irrespective of
        // the national Pokémon whose art or types they may reuse.
        if !special_capture_species(id){
            reference.capture_stages.insert(id,
                if (2001..=2047).contains(&id){100}else{cap});
        }
    }
}
/// Tiny baked upstream metadata for Mega / second-Mega species. The combat
/// reference uses the exact game types and bases. The full move catalog is
/// unavailable as a static upstream JSON, so retain the base species moves
/// and include the upstream Mega signature (600 power / 30-second cooldown).
/// Game-reported combat measurements still calibrate active-hunt throughput.
fn merge_upstream_megas(reference:&mut Reference){
    for row in include_str!("../assets/visual/upstream-megas.tsv").lines(){
        if row.starts_with('#')||row.is_empty(){continue}
        let columns=row.split('\t').collect::<Vec<_>>();
        if columns.len()!=11{continue}
        let (Ok(id),Ok(base_id))=(columns[0].parse::<u64>(),
            columns[1].parse::<u64>())else{continue};
        let Some(base)=reference.species.get(&base_id).cloned()else{continue};
        let mut stats=Vec::with_capacity(6);
        for text in &columns[5..]{
            let Ok(n)=text.parse::<f64>()else{break};
            if !n.is_finite()||n<=0.0{break}
            stats.push(n);
        }
        if stats.len()!=6||columns[3].is_empty(){continue}
        let types=[columns[3],columns[4]].iter().filter(|s|!s.is_empty())
            .map(|s|(*s).to_owned()).collect::<Vec<_>>();
        let mut moves=base.moves;
        moves.push(ReferenceMove{
            move_type:columns[3].to_owned(),
            special:stats[3]>stats[1],
            power:600.0,cooldown:30_000.0,min_level:1,
        });
        reference.species.insert(id,ReferenceSpecies{
            name:columns[2].to_owned(),types,base:stats,moves,drops:Vec::new(),
        });
    }
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
    let mut secs=0.0;let mut coverage=0.0;
    for spawn in &hunt.species {
        let Some(wild)=refdata.species.get(&spawn.id) else {continue};
        let hp=(((wild.base[0]+32.0)*hunt.level as f64/100.0)*60.0).max(24.0);
        let kill=(hp/dps(mon,meta,stats,wild,hunt.level,refdata).max(1.0)).max(0.9);
        let ratio=spawn.weight.max(0.0)/total;
        secs+=ratio*kill;
        coverage+=ratio;
    }
    if coverage<0.6 {return None}
    let sec=(secs/coverage+2.8).max(0.9);
    let max_kills=3600.0/if hunt.level<20{4.2}else{3.2};
    Some(((3600.0/sec).min(max_kills),0.0,coverage))
}
fn bonus(view:&HuntView, trainer:bool)->f64 {
    let b=&view.bonuses;
    // Same multiplicative structure as the community calculator, but mark
    // it partial when VIP/boost timing is not known from server clock.
    (if b.vip_active{1.5}else{1.0})
        *(1.0+b.guild_pct/100.0)
        *(if b.guild_boost_active{1.1}else{1.0})
        *(1.0+b.twitch_pct/100.0)
        *(1.0+(if trainer{b.trainer_event_pct}else{b.pokemon_event_pct})/100.0)
}
pub fn estimate(view:&HuntView,hunt:&HuntEntry,reference:&Reference)->Option<Forecast> {
    let mon=view.active_pokemon.as_ref()?;
    let mon_meta=reference.species.get(&mon.species_id?)?;
    let stats=pokemon_stats(mon,mon_meta)?;
    let (base_kills,_,coverage)=raw_kills(hunt,mon,mon_meta,reference,&stats)?;
    // The raw stat/move approximation is optimistic for many setups.
    // Calibrate against the current *observed* hunt's kills/h where possible;
    // never double-count Pokémon and trainer XP streams.
    let observed=&view.measurements;
    let current_prediction=view.hunt_slug.as_deref()
        .and_then(|slug|view.hunts.iter().find(|h|h.slug==slug))
        .and_then(|h|raw_kills(h,mon,mon_meta,reference,&stats));
    let calibration=if observed.seconds>=30.0 && observed.kills>=5
        && observed.last_battle_age_secs.is_none_or(|age|age<=180) {
        current_prediction.and_then(|(k,_,_)|
            if k>0.0 && observed.kills_h.is_finite() && observed.kills_h>0.0 {
                Some((observed.kills_h/k).clamp(0.001,10.0))
            } else {None})
    }else{None};
    let kills_h=(base_kills*calibration.unwrap_or(1.0))
        .min(3600.0/if hunt.level<20{4.2}else{3.2});
    // Guild bonus is reported by estado and, per upstream, also boosts
    // converted loot Coins. Unreported loot-store/event boosts are not guessed.
    let loot_pct=if view.bonuses.guild_known {view.bonuses.guild_pct} else {0.0};
    let combat_gold_h=gross_gold_per_kill(hunt.level,loot_pct)*kills_h;
    Some(Forecast {
        kills_h,
        trainer_xp_h:base_xp(hunt.level)*kills_h*bonus(view,true),
        pokemon_xp_h:base_xp(hunt.level)*kills_h*bonus(view,false),
        gold_h:combat_gold_h,
        combat_gold_h,
        npc_loot_gold_h:0.0,
        trainer_xp_per_kill:base_xp(hunt.level)*bonus(view,true),
        pokemon_xp_per_kill:base_xp(hunt.level)*bonus(view,false),
        seconds_per_kill:if kills_h>0.0{3600.0/kills_h}else{0.0},
        coverage,
        calibrated:calibration.is_some()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_type_chart_survives_missing_third_party_guide(){
        let chart=built_in_type_chart();
        assert_eq!(chart.len(),18);
        assert_eq!(chart["GHOST"]["NORMAL"],0.0);
        assert_eq!(chart["DRAGON"]["DRAGON"],2.0);
        assert_eq!(chart["FIRE"]["GRASS"],2.0);
        assert_eq!(chart["ELECTRIC"]["GROUND"],0.0);
        // Neutral entries are intentionally implicit in the compact table.
        assert!(!chart["NORMAL"].contains_key("NORMAL"));
    }
    #[test]
    fn public_creature_capture_rarity_is_separate_from_market_and_guide_loot(){
        let mut reference=Reference::default();
        merge_creature_catalog(&mut reference,&serde_json::json!({
            "creatures":[
                {"pokeId":25,"huntLevel":100,"sellValue":8000,"priceNpc":4000},
                {"pokeId":10501,"huntLevel":300,"sellValue":4000000},
                {"pokeId":1,"priceNpc":null}
            ]}));
        assert_eq!(reference.capture_rarities[&25],8000.0);
        assert_eq!(reference.npc_sell_prices[&25],4000.0);
        assert_eq!(reference.catalog_sell_values[&25],8000.0);
        assert!(reference.capture_rarities.contains_key(&2001));
        assert!(!reference.capture_rarities.contains_key(&1));
    }
    #[test]
    fn catalog_merge_adds_regional_forms_and_preserves_identity(){
        let mut reference=Reference::default();
        merge_creature_catalog(&mut reference,&serde_json::json!({
            "creatures":[
                {"pokeId":4019,"name":"Alolan Rattata",
                 "type1":"DARK","type2":"NORMAL",
                 "baseHp":30,"baseAtk":56,"baseDef":35,"baseSpAtk":25,
                 "baseSpDef":35,"baseSpeed":72,"evolvesToId":4020},
                {"pokeId":4020,"name":"Alolan Raticate",
                 "type1":"DARK","type2":"NORMAL",
                 "baseHp":75,"baseAtk":71,"baseDef":70,"baseSpAtk":40,
                 "baseSpDef":80,"baseSpeed":77},
                {"pokeId":1,"name":"Bulbasaur","evolvesToId":2},
                {"pokeId":2,"name":"Ivysaur","evolvesToId":3},
                {"pokeId":3,"name":"Venusaur"},
                {"pokeId":2001,"name":"Outland","evolvesToId":2}
            ]}));
        finalize_capture_stages(&mut reference);
        assert_eq!(reference.capture_stages[&4019],40);
        assert_eq!(reference.capture_stages[&4020],100);
        assert_eq!(reference.capture_stages[&1],20);
        assert_eq!(reference.capture_stages[&2],40);
        assert_eq!(reference.capture_stages[&3],100);
        assert_eq!(reference.capture_stages[&2001],100);
        assert_eq!(reference.species[&4019].types,vec!["DARK","NORMAL"]);
        assert_eq!(reference.species[&4019].base[0],30.0);
    }
    #[test]
    fn branched_evolutions_keep_eevee_at_the_middle_stage(){
        let mut reference=Reference::default();
        merge_creature_catalog(&mut reference,&serde_json::json!({
            "creatures":[
                {"pokeId":133,"name":"Eevee"},
                {"pokeId":134,"name":"Vaporeon"},
                {"pokeId":135,"name":"Jolteon"}
            ]}));
        finalize_capture_stages(&mut reference);
        assert_eq!(reference.capture_stages[&133],40);
        assert_eq!(reference.capture_stages[&134],100);
        assert_eq!(reference.capture_stages[&135],100);
    }
    #[test]
    fn special_capture_exclusions_include_regional_forms_and_modern_legendaries(){
        for id in [144,150,772,773,386,8386,5144,1001,13386,9006,9150]{
            assert!(special_capture_species(id),"special species {id}");
        }
        for id in [1,19,4019,2001,1479,1000]{
            assert!(!special_capture_species(id),"ordinary species {id}");
        }
        let mut reference=Reference::default();
        reference.capture_rarities.insert(5144,15000.0);
        reference.capture_stages.insert(5144,100);
        assert_eq!(capture_inputs(&reference,5144),(None,None));
        assert_eq!(capture_inputs(&reference,4019),(None,None));
        reference.capture_rarities.insert(4019,999.0);
        assert_eq!(capture_inputs(&reference,4019),(None,None));
        reference.capture_stages.insert(4019,40);
        assert_eq!(capture_inputs(&reference,4019),(Some(40),None));
    }
    #[test]
    fn mega_combat_metadata_uses_distinct_types_and_bases(){
        let mut reference=Reference::default();
        reference.species.insert(6,ReferenceSpecies{
            name:"Charizard".into(),
            types:vec!["FIRE".into(),"FLYING".into()],
            base:vec![78.0,84.0,78.0,109.0,85.0,100.0],
            moves:Vec::new(),drops:Vec::new(),
        });
        merge_upstream_megas(&mut reference);
        let mega=reference.species.get(&9006).unwrap();
        assert_eq!(mega.types,vec!["FIRE","DRAGON"]);
        assert_eq!(mega.base[1],130.0);
        assert_eq!(mega.moves[0].power,600.0);
        assert_eq!(mega.moves[0].cooldown,30_000.0);
        assert!(!reference.capture_stages.contains_key(&9006));
    }
    #[test]
    fn parses_json_only_and_rejects_executable_script(){
        let text=r#"window.PI_DATA = {"tipos":{"NORMAL":{"GHOST":0.0}},"especies":[{"id":1,"n":"A","t":["NORMAL"],"b":[1,2,3,4,5,6],"m":[]}],"hunts":[]};"#;
        assert_eq!(parse(text).unwrap().species.len(),1);
        assert!(parse("window.PI_DATA = alert(document.cookie);").is_err());
        assert!(parse("alert('hi');").is_err());
    }
    #[test]
    fn forecast_calibrates_kills_in_current_hunt_instead_of_double_counting_xp() {
        let mut view=HuntView::default();
        let hunt=HuntEntry{
            slug:"route-a".into(),level:100,
            species:vec![crate::native_hunts::HuntSpecies{id:2,weight:100.0}],
            ..Default::default()
        };
        view.hunt_slug=Some("route-a".into());
        view.hunts=std::sync::Arc::new(vec![hunt.clone()]);
        view.measurements.seconds=60.0;
        view.measurements.kills=10;
        view.measurements.kills_h=600.0;
        view.active_pokemon=Some(PokemonInfo {
            id:42,species_id:Some(1),name:"Attacker".into(),level:150,
            potency:Some(3),quality:Some(1.2),ivs:[Some(25);6],
            ..Default::default()
        });
        let base = |_id,name:&str| ReferenceSpecies {
            name:name.into(),types:vec!["NORMAL".into()],
            base:vec![100.0;6],
            moves:vec![ReferenceMove{
                move_type:"NORMAL".into(),special:false,
                power:60.0,cooldown:1200.0,min_level:1
            }],
            drops:vec![]
        };
        let mut data=Reference::default();
        data.chart.insert("NORMAL".into(),
            [("NORMAL".into(),1.0)].into_iter().collect());
        data.species.insert(1,base(1,"Attacker"));
        data.species.insert(2,base(2,"Target"));
        let forecast=estimate(&view,&hunt,&data).unwrap();
        assert!(forecast.calibrated);
        assert!((forecast.kills_h-600.0).abs()<1.0);
        assert_eq!(forecast.trainer_xp_h,forecast.pokemon_xp_h);
        view.bonuses.trainer_event_pct=100.0;
        let forecast=estimate(&view,&hunt,&data).unwrap();
        assert!(forecast.trainer_xp_h>forecast.pokemon_xp_h);
    }
    #[test]
    fn forecast_economy_channels_sum_to_gross_gold_and_xp_stays_separate(){
        let mut view=HuntView::default();
        view.hunt_slug=Some("hunt-a".into());
        view.bonuses.trainer_event_pct=100.0;
        let hunt=HuntEntry {
            slug:"hunt-a".into(),name:"A".into(),level:100,
            species:vec![HuntSpecies{id:2,weight:100.0}],
            ..Default::default()
        };
        view.hunts=std::sync::Arc::new(vec![hunt.clone()]);
        view.active_pokemon=Some(PokemonInfo {
            id:1,species_id:Some(1),name:"Lead".into(),level:150,
            quality:Some(1.2),potency:Some(3),ivs:[Some(25);6],
            ..Default::default()
        });
        let mut guide=Reference::default();
        guide.chart.insert("NORMAL".into(),
            [("NORMAL".into(),1.0)].into_iter().collect());
        for id in [1,2]{
            guide.species.insert(id,ReferenceSpecies {
                name:format!("Pokemon {id}"),types:vec!["NORMAL".into()],
                base:vec![100.0;6],
                moves:vec![ReferenceMove {
                    move_type:"NORMAL".into(),special:false,
                    power:60.0,cooldown:1200.0,min_level:1,
                }],
                drops:vec![ReferenceDrop{
                    item_id:Some(20),item_name:None,chance_pct:10.0,
                    minimum:1.0,maximum:2.0,guide_price:100.0,
                }],
            });
        }
        let result=estimate(&view,&hunt,&guide).unwrap();
        assert!((result.gold_h-result.combat_gold_h-result.npc_loot_gold_h).abs()<0.001);
        assert!(result.trainer_xp_per_kill>result.pokemon_xp_per_kill);
    }
    #[test]
    fn obsolete_guide_npc_drops_are_not_loaded(){
        let script=r#"window.PI_DATA = {"tipos":{"NORMAL":{"NORMAL":1}},
            "especies":[{"id":1,"n":"A","t":["NORMAL"],"b":[1,2,3,4,5,6],
            "l":[["Moon Stone",2.5,1,3,400]]}]};"#;
        let guide=parse(script).unwrap();
        assert!(guide.species[&1].drops.is_empty());
    }
    #[test]
    fn guide_catalog_is_read_only_and_does_not_fake_unlocks(){
        let text=r#"window.PI_DATA = {"tipos":{"NORMAL":{"NORMAL":1}},
            "especies":[{"id":1,"n":"A","t":["NORMAL"],"b":[10,10,10,10,10,10]}],
            "hunts":[{"s":"forest-1","n":"Forest","a":"Kanto","nv":70,
                "e":[[1,75],[2,25]]},
                {"s":"bad slug","n":"Invalid","nv":30,"e":[[1,100]]}]};"#;
        let guide=parse(text).unwrap();
        assert_eq!(guide.hunts.len(),1);
        assert_eq!(guide.hunts[0].slug,"forest-1");
        assert_eq!(guide.hunts[0].species[0].weight,75.0);
        // Being in the public guide says nothing about authenticated access.
        let view=HuntView::default();
        assert!(!view.hunts.iter().any(|hunt|hunt.slug=="forest-1"));
    }
    #[test]
    fn xp_calculation_keeps_separate_channels(){
        let mut view=HuntView::default();
        view.bonuses.trainer_event_pct=100.0;
        view.bonuses.pokemon_event_pct=0.0;
        assert_eq!(bonus(&view,true),2.0);
        assert_eq!(bonus(&view,false),1.0);
    }
    #[test]
    fn obsolete_guide_drop_tables_do_not_reenter_native_models(){
        let script=r#"window.PI_DATA = {"tipos":{"NORMAL":{"NORMAL":1}},
            "especies":[{"id":4,"n":"Test","t":["NORMAL"],"b":[10,10,10,10,10,10],
            "l":[[42,2.5,1,3,100],[43,120,1,2,100]]}]};"#;
        let reference=parse(script).unwrap();
        let drops=&reference.species[&4].drops;
        assert!(drops.is_empty(), "Legacy NPC drops were retired upstream");
    }

}
