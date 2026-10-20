//! Native Hunt Atlas combat and field-throughput model adapted from Hunt for PokéIdle 1.0.3.
//! Pure, deterministic forecasts: no writes to game state, sessions or server hunt statistics.
use super::{damage, effect, Reference, ReferenceSpecies};
use crate::native_hunts::{HuntEntry, PokemonInfo};

const PLAYER_GAP_MS: f64 = 900.0;
const TACKLE_COOLDOWN_MS: f64 = 2000.0;
const TACKLE_POWER: f64 = 30.0;
const RETARGET_S: f64 = 1.0;
const MIN_CYCLE_S: f64 = 1.8;
const WAVE_GAP_S: f64 = 2.8;
const WAVE_JITTER_S: f64 = 2.6;
const WILD_TILE_S: f64 = 1.9;
const PLAYER_TILE_S: f64 = 0.26;
const SEARCH_FRACTION: f64 = 0.31;
const DEFAULT_DISTANCE: f64 = 20.6;
const MAX_WAVE_SPAWNS: usize = 16;
const MAX_AOE_ENGAGED: f64 = 15.0;
const WILD_DAMAGE_MULT: f64 = 1.8;
const WILD_ATTACK_GAP_S: f64 = 1.4;
const RESPAWN_S: f64 = 3.0;

#[derive(Clone, Debug)]
pub(super) struct CombatPace {
    pub kills_h: f64,
    pub seconds_per_kill: f64,
    pub coverage: f64,
    pub deaths_h: f64,
    pub one_hit: bool,
    pub aoe: bool,
    pub engaged: f64,
    pub worst_hit_fraction: f64,
    pub incoming_damage_per_kill: f64,
}
#[derive(Clone, Copy)]
struct Attack { damage: f64, cooldown_s: f64 }
#[derive(Clone, Copy)]
struct Threat { dps: f64, mean_hit: f64, worst_hit: f64 }

fn speed_iv(mon: &PokemonInfo) -> f64 {
    mon.ivs[5].unwrap_or(1).clamp(1, 32) as f64
}
fn iv_cooldown_ms(base_ms: f64, mon: &PokemonInfo) -> f64 {
    (base_ms - speed_iv(mon) * 10.0).max(400.0)
}
/// Exact shared/cooldown-golpes.mjs skill cooldown: base species Speed
/// determines up to 7% reduction, scaled by the Pokémon's Speed IV.
fn skill_cooldown_ms(base_ms:f64, base_speed:f64, mon:&PokemonInfo)->f64 {
    let potential=if base_speed<=120.0 {
        base_speed.max(0.0)/20.0
    }else{6.0+(base_speed-120.0)/40.0}.min(7.0);
    (base_ms*(1.0-potential*speed_iv(mon)/3200.0)).round()
}
fn wild_stat(base: f64, level: u32, iv: f64) -> f64 {
    (base + 2.0 * iv) * level as f64 / 100.0
}
fn effective_tackle_type<'a>(types: &'a [String], target: &ReferenceSpecies, r: &Reference) -> &'a str {
    let first = types.first().map(String::as_str).unwrap_or("NORMAL");
    if effect(r, first, target) == 0.0 {
        types.get(1).map(String::as_str).unwrap_or(first)
    } else { first }
}
fn move_damage(
    r: &Reference, attacker_level: u32, power: f64,
    attack: f64, defense: f64, kind: &str, attacker_types: &[String],
    target: &ReferenceSpecies,
) -> f64 {
    damage(attacker_level, power.max(15.0), attack, defense,
        attacker_types.iter().any(|t|t == kind), effect(r, kind, target))
}
fn incoming(
    r: &Reference, foe: &ReferenceSpecies, level: u32,
    player: &ReferenceSpecies, stats: &[f64; 6],
) -> Threat {
    let mut blows = Vec::new();
    for mv in foe.moves.iter().filter(|m| m.min_level <= level) {
        let atk = wild_stat(foe.base[if mv.special {3} else {1}], level, 16.0);
        let def = stats[if mv.special {4} else {2}];
        blows.push(move_damage(r, level, mv.power, atk, def,
            &mv.move_type, &foe.types, player));
    }
    let tackle_type = effective_tackle_type(&foe.types, player, r);
    let inv = move_damage(r, level, TACKLE_POWER,
        wild_stat(foe.base[1], level, 16.0), stats[2],
        tackle_type, &foe.types, player);
    let mean = if blows.is_empty() { inv } else {
        blows.iter().sum::<f64>() / blows.len() as f64 * 0.92 + inv * 0.08
    };
    let max = blows.iter().copied().fold(inv, f64::max);
    Threat {
        dps: mean * WILD_DAMAGE_MULT / WILD_ATTACK_GAP_S,
        mean_hit: mean * WILD_DAMAGE_MULT,
        worst_hit: max / 0.925 * WILD_DAMAGE_MULT,
    }
}

/// Move cooldowns persist between consecutive kills. At each global attack interval,
/// choose the strongest ready move or basic Tackle. Wild HP is sampled across all 32 IVs.
fn mean_ttk(moves: &[Attack], tackle: Attack, global_gap_s: f64,
            hp: &[f64; 32], downtime_s: f64) -> Option<f64> {
    let mut priority = (0..moves.len()).filter(|&i|moves[i].damage>0.0)
        .collect::<Vec<_>>();
    priority.sort_by(|&a,&b|moves[b].damage.total_cmp(&moves[a].damage));
    let mut ready=vec![0.0f64;moves.len()];
    let mut tackle_ready=0.0f64;
    let mut t=0.0f64;
    let mut spent=0.0f64;
    for target_hp in hp {
        let start=t;
        let mut damage_done=0.0;
        let mut last=t;
        let mut attempts=0;
        while damage_done < *target_hp {
            attempts+=1;
            if attempts>4000 {return None}
            if let Some(&index)=priority.iter().find(|&&i|ready[i] <= t) {
                damage_done+=moves[index].damage;
                ready[index]=t+moves[index].cooldown_s;
                last=t;t+=global_gap_s;
            } else if tackle.damage>0.0 && tackle_ready<=t {
                damage_done+=tackle.damage;
                tackle_ready=t+tackle.cooldown_s;
                last=t;t+=global_gap_s;
            } else {
                let next=priority.iter().map(|&i|ready[i])
                    .fold(if tackle.damage>0.0 {tackle_ready} else {f64::INFINITY},f64::min);
                if !next.is_finite() {return None}
                t=t.max(next);
            }
        }
        spent+=last-start;
        t=last+downtime_s;
    }
    Some(spent/hp.len() as f64)
}
fn spawn_distances(hunt: &HuntEntry, measurements: Option<&Vec<f64>>) -> Vec<f64> {
    let weighted_points = hunt.species.iter().map(|p|p.weight.max(0.0)).sum::<f64>();
    let n = if hunt.total_spawns>0 {hunt.total_spawns as usize}
        else if weighted_points>0.0 {weighted_points.round() as usize}
        else {15};
    let n=n.clamp(1,MAX_WAVE_SPAWNS);
    match measurements.filter(|d|!d.is_empty()) {
        Some(samples)=>{
            let mut sorted=samples.iter().copied().filter(|d|d.is_finite()&&*d>=0.0)
                .collect::<Vec<_>>();
            if sorted.is_empty(){return vec![DEFAULT_DISTANCE;n]}
            sorted.sort_by(f64::total_cmp);
            (0..n).map(|i|{
                let index=((i as f64+0.5)/n as f64*sorted.len() as f64).floor() as usize;
                sorted[index.min(sorted.len()-1)]
            }).collect()
        },
        None=>vec![DEFAULT_DISTANCE;n],
    }
}
fn replenishment_cap(hunt: &HuntEntry, n: usize) -> f64 {
    if hunt.level<=150 {return f64::INFINITY}
    let sinnoh=hunt.region.eq_ignore_ascii_case("sinnoh")
        ||hunt.slug.ends_with("_sinnoh");
    n as f64*3600.0/if sinnoh {30.0} else {65.0}
}
fn single_target_kills(d: &[f64], cycle_s: f64, hunt: &HuntEntry) -> f64 {
    let n=d.len();
    let average=d.iter().sum::<f64>()/n as f64;
    let walk=average*PLAYER_TILE_S*SEARCH_FRACTION;
    let meeting=WILD_TILE_S*PLAYER_TILE_S/(WILD_TILE_S+PLAYER_TILE_S);
    let mut t=0.0f64;
    for x in d { t=(t+cycle_s+walk).max(x*meeting); }
    (3600.0/cycle_s)
        .min(n as f64*3600.0/(t+WAVE_GAP_S))
        .min(replenishment_cap(hunt,n))
}
fn aoe_kills(d:&[f64], ttk:f64, hunt:&HuntEntry)->f64 {
    let n=d.len();
    let rhythm=if n>1 {d[n-2]} else {d[0]};
    let arrival=(rhythm-5.0).max(0.0)*WILD_TILE_S;
    (n as f64*3600.0/(arrival+WAVE_GAP_S))
        .min(replenishment_cap(hunt,n))
        .min(3600.0*MAX_AOE_ENGAGED/ttk.max(MIN_CYCLE_S))
}

/// Pure forecast, no observed-kill calibration: comparable full-wave conditions
/// for both current and alternative hunts. Observations remain separate in UI.
pub(super) fn simulate(
    hunt: &HuntEntry, mon: &PokemonInfo, player: &ReferenceSpecies,
    r: &Reference, stats: &[f64;6], distances: Option<&Vec<f64>>,
) -> Option<CombatPace> {
    if hunt.level==0 || player.base.len()<6{return None}
    let d=spawn_distances(hunt,distances);
    let count=d.len();
    let downtime=WAVE_JITTER_S/count as f64+RETARGET_S;
    let gap_s=iv_cooldown_ms(PLAYER_GAP_MS,mon)/1000.0;
    let mut weight=0.0;
    let mut ttk=0.0;
    let mut hit_mean=0.0;
    let mut hit_max=0.0f64;
    let mut incoming_dps=0.0;
    for encounter in &hunt.species {
        let foe=match r.species.get(&encounter.id) {
            Some(s) if s.base.len()>=6 => s, _=>continue
        };
        let w=encounter.weight.max(0.0);
        if w<=0.0 {continue}
        let mut attacks=Vec::new();
        for mv in player.moves.iter().filter(|mv|mv.min_level<=mon.level) {
            let atk=stats[if mv.special {3}else{1}];
            let def=wild_stat(foe.base[if mv.special {4}else{2}],hunt.level,16.0);
            attacks.push(Attack {
                damage:move_damage(r,mon.level,mv.power,atk,def,&mv.move_type,&player.types,foe),
                cooldown_s:skill_cooldown_ms(mv.cooldown,player.base[5],mon)/1000.0,
            });
        }
        // Z-Move is a permanent 2800-power SPECIAL TM with a 60s base cooldown.
        if let Some(element)=mon.tm_elemental.as_deref().filter(|kind|
            player.types.iter().any(|t|t==*kind)) {
            attacks.push(Attack{
                damage:move_damage(r,mon.level,2800.0,stats[3],
                    wild_stat(foe.base[4],hunt.level,16.0),
                    element,&player.types,foe),
                cooldown_s:skill_cooldown_ms(60_000.0,player.base[5],mon)/1000.0,
            });
        }
        let tackle_type=effective_tackle_type(&player.types,foe,r);
        let basic=Attack {
            damage:move_damage(r,mon.level,TACKLE_POWER,stats[1],
                wild_stat(foe.base[2],hunt.level,16.0),
                tackle_type,&player.types,foe),
            cooldown_s:iv_cooldown_ms(TACKLE_COOLDOWN_MS,mon)/1000.0,
        };
        let mut hp=[0.0;32];
        for (i,value) in hp.iter_mut().enumerate() {
            let iv=(i+1) as f64;
            *value=(wild_stat(foe.base[0],hunt.level,iv)*12.0).round()
                .max(24.0)*5.0;
        }
        let time=mean_ttk(&attacks,basic,gap_s,&hp,downtime)?;
        let threat=incoming(r,foe,hunt.level,player,stats);
        weight+=w;
        ttk+=w*time;
        incoming_dps+=w*threat.dps;
        hit_mean+=w*threat.mean_hit;
        hit_max=hit_max.max(threat.worst_hit);
    }
    let total=hunt.species.iter().map(|x|x.weight.max(0.0)).sum::<f64>();
    if weight<=0.0||total<=0.0{return None}
    let coverage=weight/total;
    if coverage<0.6{return None}
    ttk/=weight;
    hit_mean/=weight;
    incoming_dps/=weight;
    let cycle=(ttk+RETARGET_S).max(MIN_CYCLE_S);
    let mut kills_h=single_target_kills(&d,cycle,hunt);
    let mut aoe=false;
    let mut engaged=1.0;
    if mon.tm_aoe {
        let area=aoe_kills(&d,ttk,hunt);
        if area>kills_h { kills_h=area;aoe=true; }
        engaged=(1.0+kills_h*ttk/3600.0).min(MAX_AOE_ENGAGED);
    }
    let own_hp=mon.max_hp.filter(|hp|*hp>0).map(|hp|hp as f64)
        .unwrap_or_else(||(stats[0]*12.0).round().max(24.0));
    let fatal_hit=hit_max+(engaged-1.0)*hit_mean;
    let one_hit=fatal_hit>=own_hp;
    let incoming_damage_per_kill=incoming_dps*engaged*ttk;
    // The extension's default is automatic potions. Non-lethal damage is healed.
    let kills_per_life=if one_hit {own_hp/fatal_hit.max(1e-6)} else {999.0};
    let recovery=RESPAWN_S+d.iter().sum::<f64>()/count as f64*PLAYER_TILE_S;
    let alive_s=kills_per_life*3600.0/kills_h.max(1e-6);
    let uptime=alive_s/(alive_s+recovery);
    let deaths_h=if kills_per_life>=999.0 {0.0}else{3600.0/(alive_s+recovery)};
    kills_h*=uptime;
    if !kills_h.is_finite()||!ttk.is_finite() {return None}
    Some(CombatPace {
        kills_h,seconds_per_kill:ttk,coverage,deaths_h,one_hit,aoe,engaged,
        worst_hit_fraction:fatal_hit/own_hp,
        incoming_damage_per_kill,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldowns_survive_kills_and_strongest_ready_attack_goes_first() {
        let moves=[Attack{damage:100.0,cooldown_s:4.0},
                   Attack{damage:50.0,cooldown_s:1.0}];
        let hp=[120.0;32];
        let t=mean_ttk(&moves,Attack{damage:10.0,cooldown_s:2.0},
            0.9,&hp,1.0).unwrap();
        assert!(t>0.0&&t<5.0);
    }
    #[test]
    fn aoe_is_spawn_limited_and_per_slot_respawn_applies() {
        let hunt=HuntEntry{level:1000,region:"sinnoh".into(),..Default::default()};
        let d=vec![20.6;16];
        assert!(aoe_kills(&d,0.01,&hunt)<=16.0*3600.0/30.0);
        assert!(aoe_kills(&d,10000.0,&hunt)<20.0);
    }
    #[test]
    fn player_speed_iv_reduces_global_gap_to_a_floor() {
        let mut mon=PokemonInfo::default();
        mon.ivs[5]=Some(32);
        assert_eq!(iv_cooldown_ms(900.0,&mon),580.0);
        assert_eq!(iv_cooldown_ms(500.0,&mon),400.0);
        assert_eq!(skill_cooldown_ms(60_000.0,130.0,&mon),56250.0);
    }
}
