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
    pub lure:bool,
    pub loot:f64, pub guild:f64, pub drop_event:f64, pub kick_drop:f64,
}
impl Default for Modifiers {
    fn default()->Self {
        Self {capture_boost:false,shiny_boost:false,kick_capture:false,
            kick_shiny:false,catch_event_pct:0.0,shiny_event_pct:0.0,
            lure:false,loot:1.0,guild:1.0,drop_event:1.0,kick_drop:1.0}
    }
}
impl Modifiers {
    pub fn valid(&self)->bool {
        [self.catch_event_pct,self.shiny_event_pct].iter()
            .all(|v|v.is_finite()&&(0.0..=1000.0).contains(v))
        && [self.loot,self.guild,self.drop_event,self.kick_drop].iter()
            .all(|v|v.is_finite()&&(1.0..=1000.0).contains(v))
    }
    pub fn catch_factor(&self)->f64 {
        (if self.capture_boost {1.5}else{1.0})
        *(1.0+self.catch_event_pct/100.0)
        *(if self.kick_capture {1.15}else{1.0})
    }
    pub fn shiny_factor(&self)->f64 {
        (if self.shiny_boost {1.5}else{1.0})
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
/// Legendary / Mythical special encounters and missing stage data retain the
/// old economic formula. Values are the canonical upstream shared constants.
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
    let chance=(chance*mods.catch_factor()).min(1.0);
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
// Upstream shared/outland-tiers.mjs (v1.314.2).
pub const TIER_LEVELS:[u32;8]=[150,1000,2000,4000,8000,16000,24000,36000];
pub const TIER_MULTIPLIERS:[f64;8]=[1.0,1.25,1.5,2.0,3.0,4.0,6.0,8.0];
pub const SHINY_EGG_LEVELS:[u32;30]=[
    5000,5400,5900,6400,6950,8000,8600,10000,11250,12750,
    14350,16000,20000,21200,25000,27050,29450,32000,34650,43100,
    50000,58900,64000,69300,86150,100000,114800,123350,132200,160300
];
pub fn unlocked_tier(tier:u8,trainer_level:Option<u32>)->Option<f64>{
    if !(1..=8).contains(&tier){return None}
    let level=trainer_level?;
    if level<TIER_LEVELS[(tier-1) as usize]{return None}
    Some(TIER_MULTIPLIERS[(tier-1) as usize])
}
pub fn shiny_egg_chance(level:u32,outland:bool)->f64{
    if outland {return 0.0}
    let steps=SHINY_EGG_LEVELS.iter().filter(|threshold|level>=**threshold).count();
    if steps==0 {0.0}else{(375.0+(steps-1) as f64*15.0)/10_000_000.0}
}
#[derive(Clone,Copy,Debug)]
pub struct RareKind {pub id:u64,pub name:&'static str,pub base:f64,pub craft:u32}
pub const OUTLAND_RARES:[RareKind;4]=[
    RareKind{id:70000,name:"Bronze Boss Token",base:0.0005,craft:1},
    RareKind{id:70011,name:"Key Fragment",base:0.000005,craft:10},
    RareKind{id:70012,name:"Shiny Stone Fragment",base:0.000005,craft:10},
    RareKind{id:70013,name:"Bicycle Fragment",base:0.000005,craft:10},
];
#[derive(Clone,Debug)]
pub struct RareForecast {
    pub id:u64,pub name:&'static str,pub per_kill:f64,pub units_h:f64,
    pub mean_hours:f64,pub craft_hours:f64,pub chance_24h:f64,
}
fn rare_row(kind:RareKind,chance:f64,kills_h:f64,mods:&Modifiers)->Option<RareForecast>{
    if !kills_h.is_finite()||kills_h<0.0||!mods.valid(){return None}
    let p=(chance*mods.drop_factor()).min(1.0);
    if p<=0.0{return None}
    let units_h=kills_h*p;
    Some(RareForecast{id:kind.id,name:kind.name,per_kill:p,units_h,
        mean_hours:if units_h>0.0{1.0/units_h}else{f64::INFINITY},
        craft_hours:if units_h>0.0{kind.craft as f64/units_h}else{f64::INFINITY},
        chance_24h:at_least_one(p,24.0*kills_h)?,})
}
pub fn rare_scenarios(
    outland:bool,hunt_level:u32,tier:u8,trainer_level:Option<u32>,
    kills_h:f64,mods:&Modifiers
)->Vec<RareForecast>{
    if !mods.valid()||!kills_h.is_finite()||kills_h<0.0{return vec![]}
    if outland{
        let Some(mult)=unlocked_tier(tier,trainer_level) else{return vec![]};
        OUTLAND_RARES.iter().filter_map(|kind|
            rare_row(*kind,kind.base*mult,kills_h,mods)).collect()
    }else{
        let chance=shiny_egg_chance(hunt_level,false);
        rare_row(RareKind{id:70017,name:"Shiny Egg Fragment",base:chance,craft:10},
            chance,kills_h,mods).into_iter().collect()
    }
}
/// Expected vendor proceeds are reference economics; player market proceeds
/// require EXPLICIT user-supplied quote or verified completed-sale history.
pub fn market_scenario(units_h:f64,quote:f64)->Option<f64>{
    if !units_h.is_finite()||units_h<0.0||!quote.is_finite()||quote<=0.0{return None}
    Some(units_h*quote)
}
/// Historical XP/h requires measured active durations. Values from old
/// history files without a reliable time denominator remain None.
pub fn retrospective_rate(rewards:u64,seconds:u64)->Option<f64>{
    if seconds<60 {return None}
    Some(rewards as f64*3600.0/seconds as f64)
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
    #[test] fn upstream_evolution_stage_capture_factors(){
        let ordinary=|tier,ball|capture_chance_for_stage(Some(tier),None,ball,0.0).unwrap();
        assert!((ordinary(20,8.0)-0.0293).abs()<0.00002);
        assert!((ordinary(40,8.0)-0.0235).abs()<0.00002);
        assert!((ordinary(100,8.0)-0.0188).abs()<0.00002);
        assert!((ordinary(20,4.0)-0.0146).abs()<0.00002);
        assert!((ordinary(40,4.0)-0.0117).abs()<0.00002);
        assert!((ordinary(100,4.0)-0.0094).abs()<0.00002);
        assert_eq!(capture_chance_for_stage(None,None,4.0,0.0),None);
        assert_eq!(capture_chance_for_stage(None,Some(12000.0),4.0,0.0),
            wiki_capture_chance(12000.0,4.0,0.0));
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
    #[test] fn outland_tiers_egg_and_event_modifiers(){
        let m=Modifiers::default();
        assert_eq!(unlocked_tier(8,Some(16000)),None);
        assert_eq!(unlocked_tier(8,Some(25000)),None);
        assert_eq!(unlocked_tier(8,Some(36000)),Some(8.0));
        assert_eq!(shiny_egg_chance(4999,false),0.0);
        assert_eq!(shiny_egg_chance(5000,false),375.0/10_000_000.0);
        assert_eq!(shiny_egg_chance(5000,true),0.0);
        let base=rare_scenarios(true,200,1,Some(200),500.0,&m);
        assert_eq!(base.len(),4);
        let mut boosted=m.clone();boosted.loot=2.0;
        assert!(rare_scenarios(true,200,1,Some(200),500.0,&boosted)[0].units_h>base[0].units_h);
        assert!(rare_scenarios(true,200,8,Some(200),500.0,&m).is_empty());
    }
    #[test] fn market_and_history_never_invent_data(){
        assert_eq!(market_scenario(5.0,0.0),None);
        assert_eq!(market_scenario(5.0,100.0),Some(500.0));
        assert_eq!(retrospective_rate(2000,0),None);
        assert_eq!(retrospective_rate(2000,3600),Some(2000.0));
        assert!((at_least_one(0.01,100.0).unwrap()-(1.0-0.99_f64.powf(100.0))).abs()<1e-12);
    }
}
