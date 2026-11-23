//! Hunt Atlas scenario models ported from Hunt Atlas 1.9.25.
//! These are explicitly *conditional*, not verified current server probabilities.
//! Keep acquisition out of this module and leave unknown inputs unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallKind { Ultra, Beast }
impl BallKind {
    pub fn id(self)->u64 {match self {Self::Ultra=>4,Self::Beast=>5}}
    pub fn wiki_efficiency(self)->f64 {match self {Self::Ultra=>4.0,Self::Beast=>8.0}}
    pub fn label(self)->&'static str {match self {Self::Ultra=>"Ultra",Self::Beast=>"Beast"}}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Modifiers {
    pub capture_boost:bool, pub shiny_boost:bool,
    pub kick_capture:bool, pub kick_shiny:bool,
    pub catch_event_pct:f64, pub shiny_event_pct:f64,
    /// Authoritative timed Twitch/Kick factors from the current game state.
    pub live_capture_multiplier:f64, pub live_shiny_multiplier:f64,
    /// Per-species Prey Capture Boost multiplier (`1.0`..`1.10`, outside Safari/Outland).
    pub prey_capture_multiplier:f64,
    pub lure:bool,
    pub loot:f64, pub guild:f64, pub drop_event:f64, pub kick_drop:f64,
}
impl Default for Modifiers {
    fn default()->Self {
        Self {capture_boost:false,shiny_boost:false,kick_capture:false,
            kick_shiny:false,catch_event_pct:0.0,shiny_event_pct:0.0,
            live_capture_multiplier:1.0,live_shiny_multiplier:1.0,
            prey_capture_multiplier:1.0,
            lure:false,loot:1.0,guild:1.0,drop_event:1.0,kick_drop:1.0}
    }
}
impl Modifiers {
    pub fn with_prey_capture(&self, multiplier: f64) -> Self {
        let mut next = self.clone();
        next.prey_capture_multiplier = if multiplier.is_finite() && multiplier >= 1.0 {
            multiplier
        } else {
            1.0
        };
        next
    }
    pub fn valid(&self)->bool {
        [self.catch_event_pct,self.shiny_event_pct].iter()
            .all(|v|v.is_finite()&&(0.0..=1000.0).contains(v))
        && [self.live_capture_multiplier,self.live_shiny_multiplier,
            self.prey_capture_multiplier,
            self.loot,self.guild,self.drop_event,self.kick_drop].iter()
            .all(|v|v.is_finite()&&(1.0..=1000.0).contains(v))
    }
    pub fn catch_factor(&self)->f64 {
        self.live_capture_multiplier
        *self.prey_capture_multiplier
        *(if self.capture_boost {2.0}else{1.0})
        *(1.0+self.catch_event_pct/100.0)
        *(if self.kick_capture {1.15}else{1.0})
    }
    pub fn shiny_factor(&self)->f64 {
        self.live_shiny_multiplier
        *(if self.shiny_boost {1.5}else{1.0})
        *(1.0+self.shiny_event_pct/100.0)
        *(if self.kick_shiny {1.15}else{1.0})
    }
    pub fn drop_factor(&self)->f64 {self.loot*self.guild*self.drop_event*self.kick_drop}
}
/// Missing source fields remain None; upstream rarity is derived from the
/// separate client creature catalog (NOT guide price or market sales).
pub fn catalog_rarity(hunt_level:Option<u32>,sell:Option<f64>,npc:Option<f64>)->Option<f64>{
    let high=||hunt_level.filter(|n|*n>0)
        .map(|n| (12000.0*(n as f64/150.0).powf(0.6)).round());
    if hunt_level.is_some_and(|n|n>150){return high()}
    if let Some(p)=sell.filter(|v|v.is_finite()&&*v>0.0) {
        if p<=500_000.0 {return Some(p)}
        return high();
    }
    if let Some(p)=npc.filter(|v|v.is_finite()&&*v>0.0&&*v<=500_000.0) {
        return Some(p);
    }
    None
}
pub fn wiki_capture_chance(rarity:f64,efficiency:f64,hp_fraction:f64)->Option<f64>{
    if !rarity.is_finite()||rarity<=0.0||
        !efficiency.is_finite()||efficiency<=0.0||
        !hp_fraction.is_finite()||!(0.0..=1.0).contains(&hp_fraction) {return None}
    let r=rarity.max(100.0).log10()-1.0;
    let factor=1.0+r.powi(3)/17.0;
    Some((0.0075*efficiency/factor*(1.4-hp_fraction)).clamp(0.003,0.10))
}
/// v1.314.2: ordinary hunts use evolutionary-stage factors, not NPC value.
/// The legacy price-based path is available only for explicitly separate
/// scenarios passing a rarity. Native ordinary-hunt callers pass no rarity,
/// so missing stage data produce an unknown chance, not the retired odds.
pub fn capture_chance_for_stage(stage:Option<u32>,rarity:Option<f64>,
    efficiency:f64,hp_fraction:f64)->Option<f64>{
    if !efficiency.is_finite()||efficiency<=0.0||
        !hp_fraction.is_finite()||!(0.0..=1.0).contains(&hp_fraction){return None}
    let factor=match stage {
        Some(20)=>2.8669, Some(40)=>3.5745,
        Some(_)=>4.4681,
        None=>return wiki_capture_chance(rarity?,efficiency,hp_fraction),
    };
    Some((0.0075*efficiency/factor*(1.4-hp_fraction)).clamp(0.003,0.10))
}
pub fn at_least_one(p:f64,rolls:f64)->Option<f64>{
    if !p.is_finite()||!(0.0..=1.0).contains(&p)
        ||!rolls.is_finite()||rolls<0.0{return None}
    if p==0.0||rolls==0.0{return Some(0.0)}
    if p==1.0 {return Some(1.0)}
    Some(-((rolls*(-p).ln_1p()).exp_m1()))
}
#[derive(Clone,Debug)]
pub struct CaptureForecast {
    pub chance:f64,
    // Keep the complete, test-covered forecast data available for the
    // detailed Atlas interface, even when its compact view omits these values.
    #[allow(dead_code)]
    pub accepted_h:f64,
    pub catches_h:f64,
    #[allow(dead_code)]
    pub shiny_seen_h:Option<f64>,
    pub shiny_caught_h:Option<f64>,
    pub shiny_caught_24h:Option<f64>,
    #[allow(dead_code)]
    pub balls_per_capture:f64,
    #[allow(dead_code)]
    pub accepted_is_ceiling:bool,
}
/// If accepted fraction is missing, 100% targetability is an optimistic CEILING,
/// not a measured player setting. Outland's shiny eligibility is unknown.
pub fn capture_scenario_with_stage(
    stage:Option<u32>,rarity:Option<f64>,efficiency:f64,
    kill_h:f64,spawn_share:f64,accepted_fraction:Option<f64>,
    outland:bool,mods:&Modifiers
)->Option<CaptureForecast>{
    if !mods.valid()||!kill_h.is_finite()||kill_h<0.0
        ||!spawn_share.is_finite()||!(0.0..=1.0).contains(&spawn_share)
        ||accepted_fraction.is_some_and(|v|!v.is_finite()||!(0.0..=1.0).contains(&v))
    {return None}
    let chance=capture_chance_for_stage(stage,rarity,efficiency,0.0)?;
    let chance=(chance*mods.catch_factor()).min(0.10);
    let accepted=kill_h*spawn_share*accepted_fraction.unwrap_or(1.0);
    let catches_h=accepted*chance;
    let shiny=if outland{None}else{Some(
        ((if mods.lure{1.0/12000.0}else{1.0/24000.0})
            *mods.shiny_factor()).min(1.0))};
    let shiny_seen_h=shiny.map(|p|accepted*p);
    let shiny_caught_h=shiny.map(|p|catches_h*p);
    let shiny_caught_24h=shiny.and_then(|p|at_least_one(chance*p,accepted*24.0));
    Some(CaptureForecast {chance,accepted_h:accepted,catches_h,shiny_seen_h,
        shiny_caught_h,shiny_caught_24h,balls_per_capture:1.0/chance,
        accepted_is_ceiling:accepted_fraction.is_none()})
}
// Upstream shared/outland-tiers.mjs (pokeidle-upstream v1.426.2).
// The game now exposes 40 selectable tiers, with IDs 101..140.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlandTier {
    pub id:u8,
    pub number:u8,
    pub required_level:u32,
    pub multiplier:f64,
}
pub const TIER_LEVELS:[u32;40]=[
    150,500,750,1_000,1_250,1_500,1_800,2_000,2_250,2_500,
    3_000,3_250,3_500,4_000,5_000,6_000,7_000,8_000,10_000,11_000,
    12_000,14_000,15_000,17_000,19_000,21_000,24_000,28_000,32_000,36_000,
    38_000,40_000,41_000,43_000,45_000,48_000,52_000,58_000,60_000,65_000,
];
pub const TIER_MULTIPLIERS:[f64;40]=[
    1.0,1.1,1.15,1.25,1.3,1.5,1.65,1.85,2.0,2.15,
    2.3,2.45,2.6,2.75,3.0,3.25,3.5,3.75,4.0,4.25,
    4.5,5.0,5.25,6.0,6.5,7.0,7.5,8.0,8.5,9.0,
    9.5,10.0,10.5,11.0,12.0,13.0,14.0,16.0,17.0,20.0,
];
// Legacy server builds exposed only eight settings. Preserve their unlock
// levels when translating old tier IDs to the current catalog.
pub const LEGACY_TIER_LEVELS:[u32;8]=[150,1_000,2_000,4_000,8_000,16_000,24_000,36_000];

pub fn default_outland_tiers()->Vec<OutlandTier>{
    TIER_LEVELS.iter().zip(TIER_MULTIPLIERS.iter()).enumerate()
        .map(|(index,(&required_level,&multiplier))|OutlandTier{
            id:101+index as u8,number:index as u8+1,required_level,multiplier
        }).collect()
}
pub fn valid_outland_tier_id(tier:u64)->bool{
    (1..=8).contains(&tier)||(101..=140).contains(&tier)
}
/// Map a persisted pre-v1.426 tier to the newest tier at or below its old
/// unlock level, matching upstream outlandTier()/fichaDoDegrauAntigo().
pub fn canonical_outland_tier(tier:u8)->Option<u8>{
    match tier {
        101..=140=>Some(tier),
        1..=8=>{
            let level=LEGACY_TIER_LEVELS[usize::from(tier-1)];
            let index=TIER_LEVELS.partition_point(|candidate|*candidate<=level)
                .saturating_sub(1);
            Some(101+index as u8)
        }
        _=>None,
    }
}
// Upstream v1.398.1, shared/ovo-shiny.mjs: 32 level bands through Paldea.
// A fragment is only possible at level 5,000+, outside Outland.
pub const SHINY_EGG_LEVELS:[u32;32]=[
    5000,5400,5900,6400,6950,8000,8600,
    10000,11250,12750,14350,16000,20000,21200,
    25000,26150,27500,28950,30450,35150,
    40000,42800,46150,49650,53350,64950,
    75000,84650,95950,108000,120550,160300
];
pub fn shiny_egg_chance(level:u32,outland:bool)->f64{
    if outland {return 0.0}
    let steps=SHINY_EGG_LEVELS.partition_point(|threshold|level>=*threshold);
    if steps==0 {0.0}else{1.0/80_000.0+(steps-1) as f64*0.000_000_5}
}
pub const BOSS_TOKEN_ID:u64=70000;
pub const BOSS_TOKEN_CHANCE_DEFAULT:f64=0.000_1;
pub const KEY_FRAGMENT_ID:u64=70011;
pub const SHINY_STONE_FRAGMENT_ID:u64=70012;
pub const BICYCLE_FRAGMENT_ID:u64=70013;
pub const OUTLAND_FRAGMENT_CHANCE_DEFAULT:f64=0.000_004;
pub const SACRIFICE_BONE_ID:u64=70018;
pub const SACRIFICE_BONE_CHANCE:f64=0.000_004;
pub const SACRIFICE_BONE_POINTS:u32=100;

#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct OutlandRareChances {
    pub boss_token:Option<f64>,
    pub key_fragment:Option<f64>,
    pub shiny_stone_fragment:Option<f64>,
    pub bicycle_fragment:Option<f64>,
}
impl OutlandRareChances {
    pub fn base_for(&self,id:u64)->f64{
        let override_chance=match id {
            BOSS_TOKEN_ID=>self.boss_token,
            KEY_FRAGMENT_ID=>self.key_fragment,
            SHINY_STONE_FRAGMENT_ID=>self.shiny_stone_fragment,
            BICYCLE_FRAGMENT_ID=>self.bicycle_fragment,
            _=>None,
        }.filter(|p|p.is_finite()&&(0.0..=1.0).contains(p));
        override_chance.unwrap_or(match id {
            BOSS_TOKEN_ID=>BOSS_TOKEN_CHANCE_DEFAULT,
            KEY_FRAGMENT_ID|SHINY_STONE_FRAGMENT_ID|BICYCLE_FRAGMENT_ID=>OUTLAND_FRAGMENT_CHANCE_DEFAULT,
            SACRIFICE_BONE_ID=>SACRIFICE_BONE_CHANCE,
            _=>0.0,
        })
    }
}

#[derive(Clone,Copy,Debug)]
pub struct RareKind {
    pub id:u64,pub name:&'static str,#[allow(dead_code)] pub base:f64,pub craft:u32,pub altar_points:u32,
}
pub const OUTLAND_RARES:[RareKind;5]=[
    RareKind{id:BOSS_TOKEN_ID,name:"Bronze Boss Token",base:BOSS_TOKEN_CHANCE_DEFAULT,craft:1,altar_points:0},
    RareKind{id:KEY_FRAGMENT_ID,name:"Key Fragment",base:OUTLAND_FRAGMENT_CHANCE_DEFAULT,craft:10,altar_points:0},
    RareKind{id:SHINY_STONE_FRAGMENT_ID,name:"Shiny Stone Fragment",base:OUTLAND_FRAGMENT_CHANCE_DEFAULT,craft:10,altar_points:0},
    RareKind{id:BICYCLE_FRAGMENT_ID,name:"Bicycle Fragment",base:OUTLAND_FRAGMENT_CHANCE_DEFAULT,craft:10,altar_points:0},
    RareKind{id:SACRIFICE_BONE_ID,name:"Sacrifice Bone",base:SACRIFICE_BONE_CHANCE,
        craft:0,altar_points:SACRIFICE_BONE_POINTS},
];
#[derive(Clone,Debug)]
pub struct RareForecast {
    pub id:u64,pub name:&'static str,pub per_kill:f64,pub units_h:f64,
    pub mean_hours:f64,pub craft_hours:f64,pub chance_24h:f64,pub altar_points_h:f64,
}
fn rare_row(kind:RareKind,chance:f64,kills_h:f64,mods:&Modifiers)->Option<RareForecast>{
    if !kills_h.is_finite()||kills_h<0.0||!mods.valid(){return None}
    let p=(chance*mods.drop_factor()).min(1.0);
    if p<=0.0{return None}
    let units_h=kills_h*p;
    Some(RareForecast{id:kind.id,name:kind.name,per_kill:p,units_h,
        mean_hours:if units_h>0.0{1.0/units_h}else{f64::INFINITY},
        craft_hours:if units_h>0.0&&kind.craft>0{kind.craft as f64/units_h}else{0.0},
        chance_24h:at_least_one(p,24.0*kills_h)?,
        altar_points_h:units_h*kind.altar_points as f64,})
}
/// Exact server-provided Outland gate when available; fallback is handled
/// by the caller's HuntView. Never present locked-tier rare yields.
#[cfg(test)]
pub fn rare_scenarios_with_gate(
    outland:bool,hunt_level:u32,gate:Option<(u32,f64)>,
    trainer_level:Option<u32>,kills_h:f64,mods:&Modifiers
)->Vec<RareForecast>{
    rare_scenarios_with_gate_and_token_chance(
        outland,hunt_level,gate,trainer_level,kills_h,None,mods)
}
/// Prefer the live server value for Bronze Boss Token when it is available.
/// Upstream app.js defaults chanceBossToken to 0.0001 and multiplies it only
/// by the selected Outland tier factor.
#[cfg(test)]
pub fn rare_scenarios_with_gate_and_token_chance(
    outland:bool,hunt_level:u32,gate:Option<(u32,f64)>,
    trainer_level:Option<u32>,kills_h:f64,boss_token_chance:Option<f64>,
    mods:&Modifiers
)->Vec<RareForecast>{
    rare_scenarios_with_gate_and_chances(
        outland,hunt_level,gate,trainer_level,kills_h,
        OutlandRareChances{boss_token:boss_token_chance,..Default::default()},
        mods,
    )
}
pub fn rare_scenarios_with_gate_and_chances(
    outland:bool,hunt_level:u32,gate:Option<(u32,f64)>,
    trainer_level:Option<u32>,kills_h:f64,chances:OutlandRareChances,
    mods:&Modifiers
)->Vec<RareForecast>{
    if !mods.valid()||!kills_h.is_finite()||kills_h<0.0{return vec![]}
    if outland{
        let Some((required,mult))=gate else{return vec![]};
        if trainer_level.is_none_or(|n|n<required) ||
            !mult.is_finite()||mult<=0.0{return vec![]}
        OUTLAND_RARES.iter().filter_map(|kind|{
            let base=chances.base_for(kind.id);
            rare_row(*kind,base*mult,kills_h,mods)
        }).collect()
    }else{
        let chance=shiny_egg_chance(hunt_level,false);
        rare_row(RareKind{id:70017,name:"Shiny Egg Fragment",base:chance,craft:10,altar_points:0},
            chance,kills_h,mods).into_iter().collect()
    }
}
/// Historical XP/h requires measured active durations. Values from old
/// history files without a reliable time denominator remain None.
pub fn retrospective_rate(rewards:u64,seconds:u64)->Option<f64>{
    if seconds<60 {return None}
    Some(rewards as f64*3600.0/seconds as f64)
}

// Upstream shared/prey.mjs + shared/dia-sazonal.mjs: up to 2 daily Prey cards
// (`automation.prey`) grant per-species PvE modifiers on ordinary map hunts.
pub const PREY_MAX_EXTRAS: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreyBonus {
    Damage,
    Defense,
    Capture,
    Spawn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreyEffect {
    pub card: u8,
    pub species_id: u64,
    pub bonus: PreyBonus,
    pub stars: u8,
    pub value: u32,
}

/// Exact `chaveDoDiaSazonal(t)` from upstream `shared/dia-sazonal.mjs`:
/// `DESLOCAMENTO_MS = FUSO_BRASILIA_MS (3h) - ANTECIPACAO_SAZONAL_MS (6h) = -3h`,
/// so `t - DESLOCAMENTO_MS = t + 3h` flips the UTC date at 18:00 Brasília (21:00 UTC).
pub fn seasonal_day_key(ms: u64) -> String {
    const SHIFT_MS: u64 = 3 * 3_600_000;
    const DAY_MS: u64 = 86_400_000;
    let days = (ms.saturating_add(SHIFT_MS) / DAY_MS) as i64;
    // Howard Hinnant's civil-from-days algorithm (UTC proleptic Gregorian).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{year:04}-{m:02}-{d:02}")
}

fn prey_value(bonus: PreyBonus, stars: u8) -> u32 {
    if !(1..=5).contains(&stars) { return 0; }
    let s = u32::from(stars);
    match bonus {
        PreyBonus::Damage | PreyBonus::Defense => s * 10,
        PreyBonus::Capture => s * 2,
        PreyBonus::Spawn => s,
    }
}

fn json_positive_id(v: &serde_json::Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return (n > 0).then_some(n);
    }
    if let Some(s) = v.as_str() {
        if let Ok(n) = s.trim().parse::<u64>() {
            return (n > 0).then_some(n);
        }
    }
    None
}

/// Exact `efeitosDoPrey(automation, chaveDoDiaSazonal(t))` from `shared/prey.mjs`.
pub fn active_prey_effects(
    automation: Option<&serde_json::Value>,
    server_now_ms: Option<u64>,
) -> Vec<PreyEffect> {
    let Some(auto) = automation.and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let Some(prey) = auto.get("prey").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let now_ms = server_now_ms.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    });
    let expected_day = seasonal_day_key(now_ms);
    if prey.get("dia").and_then(serde_json::Value::as_str) != Some(expected_day.as_str()) {
        return Vec::new();
    }
    let card2_unlocked = auto
        .get("preyCartela2")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
        .is_some_and(|n| n.is_finite() && n > 0.0);
    let unlocked = if card2_unlocked { 2usize } else { 1usize };
    let Some(cards) = prey.get("cartelas").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (idx, raw) in cards.iter().take(unlocked).enumerate() {
        let Some(card) = raw.as_object() else { continue };
        let Some(raw_list) = card.get("lista").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let mut lista = Vec::new();
        let mut list_valid = !raw_list.is_empty() && raw_list.len() <= 9;
        for entry in raw_list {
            let Some(id) = json_positive_id(entry) else {
                list_valid = false;
                break;
            };
            if !lista.contains(&id) {
                lista.push(id);
            }
        }
        if !list_valid || lista.is_empty() || lista.len() > 9 {
            continue;
        }
        let Some(pk) = card.get("pk").and_then(json_positive_id) else {
            continue;
        };
        if !lista.contains(&pk) {
            continue;
        }
        let Some(bonus) = card.get("bonus").and_then(serde_json::Value::as_str).and_then(|b| match b {
            "dano" => Some(PreyBonus::Damage),
            "defesa" => Some(PreyBonus::Defense),
            "captura" => Some(PreyBonus::Capture),
            "spawn" => Some(PreyBonus::Spawn),
            _ => None,
        }) else {
            continue;
        };
        let Some(stars) = card
            .get("est")
            .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok())))
            .filter(|s| (1..=5).contains(s))
            .map(|s| s as u8)
        else {
            continue;
        };
        out.push(PreyEffect {
            card: idx as u8,
            species_id: pk,
            bonus,
            stars,
            value: prey_value(bonus, stars),
        });
    }
    out
}

pub fn prey_damage_multiplier(effects: &[PreyEffect], species_id: u64) -> f64 {
    let mut m = 1.0;
    for e in effects {
        if e.bonus == PreyBonus::Damage && e.species_id == species_id {
            m *= 1.0 + e.value as f64 / 100.0;
        }
    }
    m
}

pub fn prey_defense_multiplier(effects: &[PreyEffect], species_id: u64) -> f64 {
    let mut m = 1.0;
    for e in effects {
        if e.bonus == PreyBonus::Defense && e.species_id == species_id {
            m *= 1.0 - e.value as f64 / 100.0;
        }
    }
    m
}

pub fn prey_capture_multiplier(effects: &[PreyEffect], species_id: u64) -> f64 {
    let mut m = 1.0;
    for e in effects {
        if e.bonus == PreyBonus::Capture && e.species_id == species_id {
            m *= 1.0 + e.value as f64 / 100.0;
        }
    }
    m
}

pub fn prey_extra_spawns(effects: &[PreyEffect], species_id: u64) -> u32 {
    let mut total = 0u32;
    for e in effects {
        if e.bonus == PreyBonus::Spawn && e.species_id == species_id {
            total = total.saturating_add(e.value);
        }
    }
    total.min(PREY_MAX_EXTRAS)
}

/// Effective per-species spawn share in a hunt wave after applying Prey Extra
/// Spawns (`+1..+5` on top of the normal wave in ordinary map hunts).
pub fn effective_spawn_share(
    hunt: &crate::native_hunts::HuntEntry,
    species_id: u64,
    base_weight: f64,
    total_weight: f64,
    prey: &[PreyEffect],
) -> f64 {
    if !(base_weight > 0.0) || !(total_weight > 0.0) {
        return 0.0;
    }
    if hunt.area.eq_ignore_ascii_case("outland") || prey.is_empty() {
        return base_weight / total_weight;
    }
    let base_spawns = if hunt.total_spawns > 0 {
        hunt.total_spawns as f64
    } else {
        total_weight
    };
    let total_extra: f64 = hunt
        .species
        .iter()
        .filter(|s| s.weight > 0.0)
        .map(|s| prey_extra_spawns(prey, s.id) as f64)
        .sum();
    if total_extra <= 0.0 {
        return base_weight / total_weight;
    }
    let base_count = base_spawns * (base_weight / total_weight);
    let extra = prey_extra_spawns(prey, species_id) as f64;
    (base_count + extra) / (base_spawns + total_extra)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn ultra_beast_and_unknown_rarity(){
        let ultra=wiki_capture_chance(12000.0,4.0,0.0).unwrap();
        let beast=wiki_capture_chance(12000.0,8.0,0.0).unwrap();
        assert!(beast>ultra);
        assert!(wiki_capture_chance(0.0,4.0,0.0).is_none());
        assert_eq!(catalog_rarity(Some(200),Some(100.0),None),
            Some((12000.0_f64*(200.0_f64/150.0).powf(0.6)).round()));
    }
    #[test] fn live_stream_capture_and_shiny_factors_stack_with_scenarios(){
        let mut m=Modifiers::default();
        m.live_capture_multiplier=1.15*1.15;
        m.live_shiny_multiplier=1.15;
        assert!((m.catch_factor()-1.3225).abs()<1e-12);
        assert!((m.shiny_factor()-1.15).abs()<1e-12);
    }
    #[test] fn capture_boost_doubles_odds_before_10_percent_cap(){
        let mut boosted=Modifiers::default();
        boosted.capture_boost=true;
        assert_eq!(boosted.catch_factor(),2.0);
        let baseline=capture_scenario_with_stage(Some(40),None,8.0,1.0,1.0,
            None,false,&Modifiers::default()).unwrap().chance;
        let doubled=capture_scenario_with_stage(Some(40),None,8.0,1.0,1.0,
            None,false,&boosted).unwrap().chance;
        assert!((doubled-2.0*baseline).abs()<1e-12);
    }
    #[test] fn upstream_evolution_stage_capture_factors(){
        let ordinary=|tier,ball|capture_chance_for_stage(Some(tier),None,ball,0.0).unwrap();
        assert!((ordinary(20,8.0)-0.0293).abs()<0.00002);
        assert!((ordinary(40,8.0)-0.0235).abs()<0.00002);
        assert!((ordinary(100,8.0)-0.0188).abs()<0.00002);
        // The 4x ball uses half the 8x efficiency without rounding
        // the expected probabilities to four decimal places.
        for stage in [20, 40, 100] {
            assert!((ordinary(stage,4.0)*2.0-ordinary(stage,8.0)).abs()<1e-12);
        }
        assert_eq!(capture_chance_for_stage(None,None,4.0,0.0),None);
        assert_eq!(capture_chance_for_stage(None,Some(12000.0),4.0,0.0),
            wiki_capture_chance(12000.0,4.0,0.0));
        let mut boosted=Modifiers::default();
        boosted.capture_boost=true;
        boosted.kick_capture=true;
        boosted.catch_event_pct=1000.0;
        let capped=capture_scenario_with_stage(Some(20),None,8.0,100.0,1.0,
            None,false,&boosted).unwrap();
        assert_eq!(capped.chance,0.10);
        let m=Modifiers::default();
        let forecast=capture_scenario_with_stage(Some(20),None,8.0,100.0,1.0,
            None,false,&m).unwrap();
        assert!((forecast.chance-ordinary(20,8.0)).abs()<1e-12);
    }
    #[test] fn capture_bonuses_and_ceiling_provenance(){
        let m=Modifiers::default();
        let forecast=capture_scenario_with_stage(None,Some(12000.0),4.0,100.0,0.5,None,false,&m).unwrap();
        assert!(forecast.accepted_is_ceiling);
        assert_eq!(forecast.accepted_h,50.0);
        assert!(forecast.shiny_caught_h.unwrap()>0.0);
        assert!(capture_scenario_with_stage(None,None,4.0,100.0,0.5,None,false,&m).is_none());
        let outland=capture_scenario_with_stage(None,Some(12000.0),8.0,100.0,0.5,Some(0.3),true,&m).unwrap();
        assert_eq!(outland.shiny_caught_h,None);
        assert_eq!(outland.accepted_h,15.0);
    }
    #[test] fn server_outland_gate_affects_rare_yield(){
        let mods=Modifiers::default();
        assert!(rare_scenarios_with_gate(true,200,Some((2000,3.0)),
            Some(1999),300.0,&mods).is_empty());
        let server=rare_scenarios_with_gate(true,200,Some((2000,3.0)),
            Some(2000),300.0,&mods);
        let default=rare_scenarios_with_gate(true,200,Some((2000,1.5)),
            Some(2000),300.0,&mods);
        assert!((server[0].units_h/default[0].units_h-2.0).abs()<1e-12);
    }
    #[test] fn outland_tiers_egg_and_event_modifiers(){
        let m=Modifiers::default();
        assert_eq!(shiny_egg_chance(4999,false),0.0);
        assert!((shiny_egg_chance(5000,false)-1.0/80_000.0).abs()<1e-15);
        assert_eq!(shiny_egg_chance(5000,true),0.0);
        assert_eq!(shiny_egg_chance(24999,false),shiny_egg_chance(21200,false));
        assert!((shiny_egg_chance(25000,false)-(1.0/80_000.0+14.0*0.000_000_5)).abs()<1e-15);
        assert!((shiny_egg_chance(40000,false)-(1.0/80_000.0+20.0*0.000_000_5)).abs()<1e-15);
        assert!((shiny_egg_chance(75000,false)-(1.0/80_000.0+26.0*0.000_000_5)).abs()<1e-15);
        assert!((shiny_egg_chance(160300,false)-0.000028).abs()<1e-15);
        assert_eq!(shiny_egg_chance(160300,true),0.0);
        let base=rare_scenarios_with_gate(true,200,Some((150,1.0)),Some(200),500.0,&m);
        assert_eq!(base.len(),5);
        let token=base.iter().find(|row|row.id==BOSS_TOKEN_ID).unwrap();
        assert!((token.per_kill-BOSS_TOKEN_CHANCE_DEFAULT).abs()<1e-15);
        let server=rare_scenarios_with_gate_and_token_chance(true,200,Some((150,1.0)),
            Some(200),500.0,Some(0.0002),&m);
        let server_token=server.iter().find(|row|row.id==BOSS_TOKEN_ID).unwrap();
        assert!((server_token.per_kill-0.0002).abs()<1e-15);
        let bone=base.iter().find(|row|row.id==SACRIFICE_BONE_ID).unwrap();
        assert!((bone.per_kill-SACRIFICE_BONE_CHANCE).abs()<1e-15);
        assert!((bone.units_h-500.0*SACRIFICE_BONE_CHANCE).abs()<1e-12);
        assert!((bone.altar_points_h-bone.units_h*SACRIFICE_BONE_POINTS as f64).abs()<1e-12);
        assert_eq!(bone.craft_hours,0.0);
        let mut boosted=m.clone();boosted.loot=2.0;
        assert!(rare_scenarios_with_gate(true,200,Some((150,1.0)),Some(200),500.0,&boosted)[0].units_h>base[0].units_h);
        assert!(rare_scenarios_with_gate(true,200,Some((36000,8.0)),Some(200),500.0,&m).is_empty());
    }
    #[test] fn history_never_invents_rates(){
        assert_eq!(retrospective_rate(2000,0),None);
        assert_eq!(retrospective_rate(2000,3600),Some(2000.0));
        assert!((at_least_one(0.01,100.0).unwrap()-(1.0-0.99_f64.powf(100.0))).abs()<1e-12);
    }
    #[test] fn prey_day_key_flips_at_18h_brasilia_and_validates_cards(){
        // 2026-03-15 20:59:59 UTC (17:59:59 BRT) -> 2026-03-15
        let before_flip = 1_773_608_399_000u64;
        assert_eq!(seasonal_day_key(before_flip), "2026-03-15");
        // 2026-03-15 21:00:00 UTC (18:00:00 BRT) -> 2026-03-16
        assert_eq!(seasonal_day_key(before_flip + 1_000), "2026-03-16");

        let auto = serde_json::json!({
            "preyCartela2": 0,
            "prey": {
                "dia": "2026-03-15",
                "cartelas": [
                    {"lista": [25, 19], "pk": 25, "bonus": "captura", "est": 5, "n": [0, 0, 0]},
                    {"lista": [19, 16], "pk": 19, "bonus": "dano", "est": 4, "n": [0, 0, 0]}
                ]
            }
        });
        // Card 2 locked -> only card 0 active.
        let fx = active_prey_effects(Some(&auto), Some(before_flip));
        assert_eq!(fx.len(), 1);
        assert!((prey_capture_multiplier(&fx, 25) - 1.10).abs() < 1e-12);
        assert_eq!(prey_damage_multiplier(&fx, 19), 1.0);

        // Next seasonal day -> expired, no active cards.
        assert!(active_prey_effects(Some(&auto), Some(before_flip + 1_000)).is_empty());

        // Unlock card 2 -> both active.
        let mut auto2 = auto.clone();
        auto2["preyCartela2"] = serde_json::json!(1);
        let fx2 = active_prey_effects(Some(&auto2), Some(before_flip));
        assert_eq!(fx2.len(), 2);
        assert!((prey_damage_multiplier(&fx2, 19) - 1.40).abs() < 1e-12);

        // Prey capture boosts catch chance before 10% cap without altering shiny chance.
        let base_mods = Modifiers::default();
        let prey_mods = base_mods.with_prey_capture(prey_capture_multiplier(&fx2, 25));
        let base_cap = capture_scenario_with_stage(Some(20), None, 8.0, 100.0, 1.0, None, false, &base_mods).unwrap();
        let prey_cap = capture_scenario_with_stage(Some(20), None, 8.0, 100.0, 1.0, None, false, &prey_mods).unwrap();
        assert!((prey_cap.chance / base_cap.chance - 1.10).abs() < 1e-12);
        assert!((prey_cap.shiny_seen_h.unwrap() - base_cap.shiny_seen_h.unwrap()).abs() < 1e-12);
    }
}
