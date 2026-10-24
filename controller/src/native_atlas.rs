//! Native Hunt Atlas reference: public game creature catalogs plus the bundled
//! upstream type chart; loaded only when Hunt Atlas opens.
//! No third-party guide request. Authenticated hunts remain server-authoritative.
use crate::native_hunts::{HuntEntry, HuntView, PokemonInfo, base_xp, gross_gold_per_kill};
#[cfg(test)]
use crate::native_hunts::HuntSpecies;
#[path = "native_atlas_combat.rs"]
mod combat;
use serde_json::Value;
use std::{collections::{BTreeMap,BTreeSet}, io::Read, sync::OnceLock, time::Duration};

#[derive(Clone, Debug)]
pub struct ReferenceSpecies {
    pub name: String,
    pub types: Vec<String>,
    pub base: Vec<f64>,
    pub moves: Vec<ReferenceMove>,
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
    /// Public game-catalog derived capture rarity (not market listing prices).
    pub capture_rarities: BTreeMap<u64,f64>,
    /// Upstream evolutionary-stage capture cap (20/40/100), inferred from
    /// current public creature evolution chains after all catalogs merge.
    pub capture_stages: BTreeMap<u64,u32>,
    capture_catalog_ids: BTreeSet<u64>,
    capture_evolutions: BTreeMap<u64,u64>,
    pub npc_sell_prices: BTreeMap<u64,f64>,
    /// Static catalog sellValue; not an observed player-to-player market average.
    pub catalog_sell_values: BTreeMap<u64,f64>,
    /// Untouched catalog hunt level, independent of the wild hunt level.
    pub catalog_hunt_levels: BTreeMap<u64,u32>,
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
/// NPC MKT buyback, not a player-to-player RMT listing.
///
/// v1.357.3 caps the species base at ouroDoNivel(2500) = 64,907,
/// and the captured Pokémon level multiplier at level 100.
/// The reference is quality 1.0, non-shiny, without an individual IV price.
pub fn mkt_sale_estimate(
    reference:&Reference,id:u64,wild_level:u32,lowest_hunt_level:u32,
)->Option<f64>{
    if wild_level==0{return None}
    let raw_sell=reference.catalog_sell_values.get(&id).copied();
    let raw_npc=reference.npc_sell_prices.get(&id).copied();
    let catalog_level=reference.catalog_hunt_levels.get(&id).copied();
    if raw_sell.is_none()&&raw_npc.is_none()&&catalog_level.is_none(){return None}
    let level=catalog_level.unwrap_or(wild_level).max(1);
    let ouro_do_nivel=|n:u32|(12_000.0*(n.max(1) as f64/150.0).powf(0.6)).round();
    let cap=ouro_do_nivel(2500);
    // aplicarEconomiaRegiaoAlta replaces the old mirror value above 150.
    let mut base=if level>150{ouro_do_nivel(level)}else{
        raw_sell.filter(|v|*v<=500_000.0)
            .or(raw_npc.filter(|v|*v<=500_000.0))
            .unwrap_or_else(||ouro_do_nivel(level))
    };
    // elevarAoNivelDaHunt only promotes a Kanto-scale catalog species
    // when ALL its authentic hunts are above the level-150 boundary.
    if level<=150&&lowest_hunt_level>150{
        base=base.max(ouro_do_nivel(lowest_hunt_level));
    }
    base=match id{
        2027|2050=>60_000.0,
        2006=>32_000.0,
        212|3212=>88_000.0,
        _=>base,
    }.min(cap);
    // nivelDeCaptura preserves wild levels <=100; higher ones use
    // 20/40/100 evolutionary-stage caps. Unknown stages stay unknown.
    let captured_level=if wild_level<=100{wild_level}
        else{reference.capture_stages.get(&id).copied()?.min(100)};
    let value=(base*(1.0+captured_level as f64/50.0)).floor().max(1.0);
    value.is_finite().then_some(value)
}
#[derive(Clone, Debug)]
pub struct Forecast {
    pub kills_h: f64,
    pub trainer_xp_h: f64,
    pub pokemon_xp_h: f64,
    pub gold_h: f64,
    /// Historical NPC field retained for compatibility; no NPC drops are modeled.
    pub combat_gold_h: f64,
    pub trainer_xp_per_kill: f64,
    pub pokemon_xp_per_kill: f64,
    pub seconds_per_kill: f64,
    pub coverage: f64,
    pub calibrated: bool,
    pub deaths_h: f64,
    pub one_hit: bool,
    pub aoe: bool,
    pub engaged: f64,
    pub worst_hit_fraction:f64,
    pub incoming_damage_per_kill:f64,
}

/// The type chart is bundled from upstream; no download or guide is required.
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
    let mut reference=Reference::default();
    reference.chart=built_in_type_chart();
    // Public game-hosted static assets only. No credentials or game cookies.
    // Missing individual asset files do not prevent loading the remaining ones.
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
        return Err("Game creature catalogs are unavailable".into())
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
        if let Some(level)=level.filter(|n|*n>0){
            reference.catalog_hunt_levels.insert(id,level);
        }
        if let Some(price)=sell {reference.catalog_sell_values.insert(id,price);}
        if let Some(price)=npc {reference.npc_sell_prices.insert(id,price);}
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
                    name:name.to_owned(),types,base,moves,
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
            name:columns[2].to_owned(),types,base:stats,moves,
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
    if let Some(stats)=mon.battle_stats.filter(|stats|
        stats.iter().all(|value|value.is_finite()&&*value>0.0)) {
        return Some(stats);
    }
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
fn bonus(view:&HuntView, trainer:bool)->f64 {
    let b=&view.bonuses;
    // Same multiplicative structure as the community calculator, but mark
    // it partial when VIP/boost timing is not known from server clock.
    (if b.vip_active{1.5}else{1.0})
        *(1.0+b.guild_pct/100.0)
        *(if b.guild_boost_active{1.1}else{1.0})
        *(1.0+b.twitch_pct/100.0)
        *(1.0+b.kick_xp_pct/100.0)
        *(if trainer {b.shop_trainer_xp_multiplier.max(1.0)}
          else {b.shop_pokemon_xp_multiplier.max(1.0)})
        *b.delayed_xp_multiplier.max(1.0)
        *(1.0+(if trainer{b.trainer_event_pct}else{b.pokemon_event_pct})/100.0)
}
/// The legacy entry point intentionally models a single-target Pokémon.
/// Live UI must pass the user's explicit AoE preference to estimate_with_aoe.
#[cfg(test)]
pub fn estimate(view:&HuntView,hunt:&HuntEntry,reference:&Reference)->Option<Forecast> {
    estimate_with_aoe(view,hunt,reference,false)
}
pub fn estimate_with_aoe(
    view:&HuntView,hunt:&HuntEntry,reference:&Reference,aoe_enabled:bool,
)->Option<Forecast> {
    let mon=view.active_pokemon.as_ref()?;
    let player=reference.species.get(&mon.species_id?)?;
    let stats=pokemon_stats(mon,player)?;
    let geometry=view.geo_distances.get(&hunt.slug);
    let pace=combat::simulate(hunt,mon,player,reference,&stats,geometry,aoe_enabled)?;
    let kills_h=pace.kills_h;
    let loot_pct=(if view.bonuses.guild_known {view.bonuses.guild_pct} else {0.0})
        + view.bonuses.extra_loot_pct;
    let combat_gold_h=gross_gold_per_kill(hunt.level,loot_pct)*kills_h;
    Some(Forecast {
        kills_h,
        trainer_xp_h:base_xp(hunt.level)*kills_h*bonus(view,true),
        pokemon_xp_h:base_xp(hunt.level)*kills_h*bonus(view,false),
        gold_h:combat_gold_h,
        combat_gold_h,
        trainer_xp_per_kill:base_xp(hunt.level)*bonus(view,true),
        pokemon_xp_per_kill:base_xp(hunt.level)*bonus(view,false),
        seconds_per_kill:pace.seconds_per_kill,
        coverage:pace.coverage,
        calibrated:false, // model is independent of observed current-hunt pace
        deaths_h:pace.deaths_h,
        one_hit:pace.one_hit,
        aoe:pace.aoe,
        engaged:pace.engaged,
        worst_hit_fraction:pace.worst_hit_fraction,
        incoming_damage_per_kill:pace.incoming_damage_per_kill,
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
    fn game_catalog_supplies_species_types_stats_and_learned_moves() {
        let mut reference=Reference::default();
        reference.chart=built_in_type_chart();
        merge_creature_catalog(&mut reference,&serde_json::json!({
            "creatures":[{"pokeId":25,"name":"Pikachu","type1":"ELECTRIC",
                "baseHp":35,"baseAtk":55,"baseDef":40,"baseSpAtk":50,
                "baseSpDef":50,"baseSpeed":90,
                "attacks":[{"type":"ELECTRIC","power":90.0,
                    "cooldownMs":2300,"category":"SPECIAL","learnLevel":15}]
            }]
        }));
        let mon=&reference.species[&25];
        assert_eq!(mon.name,"Pikachu");
        assert_eq!(mon.types,vec!["ELECTRIC"]);
        assert_eq!(mon.base,vec![35.0,55.0,40.0,50.0,50.0,90.0]);
        assert_eq!(mon.moves.len(),1);
        assert!(mon.moves[0].special);
        assert_eq!(mon.moves[0].min_level,15);
        assert_eq!(mon.moves[0].cooldown,2300.0);
        assert_eq!(reference.chart["ELECTRIC"]["GROUND"],0.0);
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
            moves:Vec::new(),
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
    fn upstream_v1357_3_mega_reference_uses_latest_types_stats_and_second_megas() {
        // These compact rows supplement the live creature JSON for Megas.
        // Every new upstream Mega and the changed Aggron typing must survive
        // the base-species lookup used by the native battle simulator.
        let mut reference=Reference::default();
        let base=ReferenceSpecies{
            name:"Base".into(),types:vec!["NORMAL".into()],
            base:vec![100.0;6],moves:Vec::new(),
        };
        for id in [26,306,373,445,500,531,952] {
            reference.species.insert(id,base.clone());
        }
        merge_upstream_megas(&mut reference);
        for (id,types,stats) in [
            (3373,vec!["DRAGON","FLYING"],vec![95.0,145.0,130.0,120.0,90.0,120.0]),
            (3500,vec!["FIRE","FIGHTING"],vec![110.0,148.0,75.0,110.0,110.0,75.0]),
            (3531,vec!["NORMAL","FAIRY"],vec![103.0,60.0,126.0,80.0,126.0,50.0]),
            (3952,vec!["GRASS","FIRE"],vec![65.0,138.0,85.0,138.0,85.0,75.0]),
            (9026,vec!["ELECTRIC"],vec![60.0,135.0,95.0,90.0,95.0,110.0]),
            (9445,vec!["DRAGON"],vec![108.0,130.0,85.0,141.0,85.0,151.0]),
        ] {
            let mega=reference.species.get(&id)
                .unwrap_or_else(||panic!("Missing upstream Mega {id}"));
            assert_eq!(mega.types,types,"Mega {id} types");
            assert_eq!(mega.base,stats,"Mega {id} stats");
            assert_eq!(mega.moves.len(),1,"Mega {id} signature attack");
            assert_eq!(mega.moves[0].power,600.0);
            assert!(special_capture_species(id),"Mega {id} must not use ordinary capture odds");
        }
        let aggron=&reference.species[&3306];
        assert_eq!(aggron.types,vec!["STEEL"]);
        assert_eq!(aggron.base[2],230.0);
        assert_eq!(include_str!("../assets/visual/upstream-megas.tsv").lines()
            .filter(|line|line.starts_with(|c:char|c.is_ascii_digit())).count(),93);
    }
    #[test]
    fn upstream_v1357_3_forms_map_to_base_dex_and_exclude_special_captures() {
        let mut form_dex=BTreeMap::<u64,u64>::new();
        for line in include_str!("../assets/visual/upstream-form-base.tsv").lines() {
            let Some((id,dex))=line.split_once('\t') else{continue};
            if let (Ok(id),Ok(dex))=(id.parse::<u64>(),dex.parse::<u64>()){
                assert!(form_dex.insert(id,dex).is_none(),"Duplicate form {id}");
            }
        }
        assert_eq!(form_dex.len(),97);
        for (id,dex) in [
            (4053,53),(8413,413),(9413,413),(8483,483),(8484,484),
            (8492,492),(8555,555),(9555,555),(9646,646),
        ] {
            assert_eq!(form_dex.get(&id),Some(&dex),"Form {id} must use its national dex");
        }
        let mut reference=Reference::default();
        for id in [8483,8484,8492,9646] {
            assert!(special_capture_species(id),"Legendary/mythical form {id}");
            reference.capture_stages.insert(id,100);
            assert_eq!(capture_inputs(&reference,id),(None,None));
        }
        for id in [4053,8413,9413,8555,9555] {
            assert!(!special_capture_species(id),"Ordinary form {id}");
            reference.capture_stages.insert(id,100);
            assert_eq!(capture_inputs(&reference,id),(Some(100),None));
        }
    }
    #[test]
    fn upstream_v1357_3_mkt_caps_base_and_captured_level(){
        let mut reference=Reference::default();
        for (id,level) in [(1,25),(2,25),(3,25),(212,100),
            (2006,150),(555,5000),(556,160300)]{
            reference.catalog_hunt_levels.insert(id,level);
        }
        for id in [1,2,3]{reference.catalog_sell_values.insert(id,500.0);}
        for id in [555,556]{reference.catalog_sell_values.insert(id,3000.0);}
        for (id,stage) in [(1,20),(2,40),(3,100),
            (212,100),(2006,100),(555,20),(556,100)]{
            reference.capture_stages.insert(id,stage);
        }
        // Kanto low-level catches retain the wild level.
        assert_eq!(mkt_sale_estimate(&reference,1,30,30),Some(800.0));
        // The old high-region catalog raw value is not the MKT price.
        assert_eq!(mkt_sale_estimate(&reference,555,5000,5000),Some(90_869.0));
        assert_eq!(mkt_sale_estimate(&reference,556,160300,160300),Some(194_721.0));
        assert_eq!(mkt_sale_estimate(&reference,212,120,120),Some(194_721.0));
        assert_eq!(mkt_sale_estimate(&reference,2006,150,150),Some(96_000.0));
        assert_eq!(mkt_sale_estimate(&reference,2,120,120),Some(900.0));
        assert_eq!(mkt_sale_estimate(&reference,3,120,120),Some(1500.0));
        reference.capture_stages.remove(&556);
        assert_eq!(mkt_sale_estimate(&reference,556,160300,160300),None);
        assert_eq!(mkt_sale_estimate(&Reference::default(),9,500,500),None);
    }
    #[test]
    fn upstream_v1357_3_mkt_uses_lowest_hunt_for_legacy_species(){
        let mut reference=Reference::default();
        reference.catalog_hunt_levels.insert(99,100);
        reference.catalog_sell_values.insert(99,3000.0);
        reference.capture_stages.insert(99,100);
        // The species' only hunt is high-level: its base is promoted.
        let price=mkt_sale_estimate(&reference,99,2000,2000).unwrap();
        let expected=((12_000.0_f64*(2000.0_f64/150.0).powf(0.6))
            .round().min(64_907.0)*3.0).floor();
        assert_eq!(price,expected);
        // Having any low-level hunt prevents that promotion.
        assert_eq!(mkt_sale_estimate(&reference,99,2000,100),Some(9000.0));
    }
    #[test]
    fn forecast_uses_simulation_instead_of_current_hunt_pace_without_double_counting_xp() {
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
            }]
        };
        let mut data=Reference::default();
        data.chart.insert("NORMAL".into(),
            [("NORMAL".into(),1.0)].into_iter().collect());
        data.species.insert(1,base(1,"Attacker"));
        data.species.insert(2,base(2,"Target"));
        let forecast=estimate(&view,&hunt,&data).unwrap();
        assert!(!forecast.calibrated);
        assert!(forecast.kills_h>0.0 && forecast.kills_h.is_finite());
        assert_eq!(forecast.trainer_xp_h,forecast.pokemon_xp_h);
        // Detection is not consent to apply the AoE simulation.
        view.active_pokemon.as_mut().unwrap().tm_aoe=true;
        let unchecked=estimate_with_aoe(&view,&hunt,&data,false).unwrap();
        assert!(!unchecked.aoe);
        // Checked mode is explicitly modeled even if the server omits its flag.
        view.active_pokemon.as_mut().unwrap().tm_aoe=false;
        let checked=estimate_with_aoe(&view,&hunt,&data,true).unwrap();
        assert!(checked.aoe);
        assert!(checked.kills_h>=unchecked.kills_h);
        view.bonuses.trainer_event_pct=100.0;
        let forecast=estimate(&view,&hunt,&data).unwrap();
        assert!(forecast.trainer_xp_h>forecast.pokemon_xp_h);
    }
    #[test]
    fn vip_and_timed_guild_xp_boosts_stack_for_both_xp_streams(){
        let unboosted=HuntView::default();
        let mut boosted=unboosted.clone();
        boosted.bonuses.vip_active=true;
        boosted.bonuses.guild_boost_active=true;
        for trainer in [true,false] {
            let base=bonus(&unboosted,trainer);
            assert!((bonus(&boosted,trainer)/base-1.65).abs()<1e-10);
        }
        boosted.bonuses.vip_active=false;
        assert!((bonus(&boosted,true)-1.1).abs()<1e-10);
    }
    #[test]
    fn forecast_gold_comes_only_from_combat_and_xp_stays_separate(){
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
                }]
            });
        }
        let result=estimate(&view,&hunt,&guide).unwrap();
        assert!((result.gold_h-result.combat_gold_h).abs()<0.001);
        assert!(result.trainer_xp_per_kill>result.pokemon_xp_per_kill);
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
