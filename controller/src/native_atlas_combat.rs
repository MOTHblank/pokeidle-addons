//! Native Hunt Atlas combat model; map pacing is vendored from /hunt for PokéIdle 1.0.17.
//! Pure, deterministic forecasts: no writes to game state, sessions or server hunt statistics.
use super::{damage, effect, Reference, ReferenceSpecies};
use crate::native_hunts::{HuntEntry, PokemonInfo};
use crate::native_atlas_pacing;

const PLAYER_GAP_MS: f64 = 900.0;
const TACKLE_COOLDOWN_MS: f64 = 2000.0;
const TACKLE_POWER: f64 = 30.0;
const RETARGET_S: f64 = 1.0;
const MIN_CYCLE_S: f64 = 1.8;
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
    let potential=(if base_speed<=120.0 {
        base_speed.max(0.0)/20.0
    }else{6.0+(base_speed-120.0)/40.0}).min(7.0);
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
fn seasonal_damage_multiplier(seasonal:Option<&str>,attacker:&[String],defender:&[String])->f64 {
    let Some(kind)=seasonal else{return 1.0};
    let outgoing=if attacker.iter().any(|t|t==kind){2.0}else{1.0};
    let incoming=if defender.iter().any(|t|t==kind){4.0}else{1.0};
    outgoing/incoming
}
fn move_damage(
    r: &Reference, attacker_level: u32, power: f64,
    attack: f64, defense: f64, kind: &str, attacker_types: &[String],
    target: &ReferenceSpecies, extra_multiplier:f64,
) -> f64 {
    damage(attacker_level, power.max(15.0), attack, defense,
        attacker_types.iter().any(|t|t == kind), effect(r, kind, target))*extra_multiplier
}
fn incoming(
    r: &Reference, foe: &ReferenceSpecies, level: u32,
    player: &ReferenceSpecies, stats: &[f64; 6], seasonal:Option<&str>,
) -> Threat {
    let mut blows = Vec::new();
    for mv in foe.moves.iter().filter(|m| m.min_level <= level) {
        let atk = wild_stat(foe.base[if mv.special {3} else {1}], level, 16.0);
        let def = stats[if mv.special {4} else {2}];
        blows.push(move_damage(r, level, mv.power, atk, def,
            &mv.move_type, &foe.types, player,
            seasonal_damage_multiplier(seasonal,&foe.types,&player.types)));
    }
    let tackle_type = effective_tackle_type(&foe.types, player, r);
    let inv = move_damage(r, level, TACKLE_POWER,
        wild_stat(foe.base[1], level, 16.0), stats[2],
        tackle_type, &foe.types, player,
        seasonal_damage_multiplier(seasonal,&foe.types,&player.types));
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
fn normalized_bicycle_factor(factor:f64)->f64 {
    if factor.is_finite()&&factor>1.0 {factor} else {1.0}
}
fn movement_component_s(base_pace_s:f64,bicycle_factor:f64)->f64 {
    // ritmo.js is an on-foot one-shot total pace. Preserve the combat floor
    // and apply bicycles only to the movement portion above that floor.
    (base_pace_s-MIN_CYCLE_S).max(0.0)/normalized_bicycle_factor(bicycle_factor)
}
fn paced_per_kill_s(base_pace_s:f64,cycle_s:f64,bicycle_factor:f64)->f64 {
    MIN_CYCLE_S+movement_component_s(base_pace_s,bicycle_factor)
        +(cycle_s-MIN_CYCLE_S).max(0.0)
}
fn single_target_kills(base_pace_s:f64,cycle_s:f64,bicycle_factor:f64)->f64 {
    3600.0/paced_per_kill_s(base_pace_s,cycle_s,bicycle_factor).max(0.001)
}
fn aoe_kills(base_pace_s:f64,cycle_s:f64,ttk:f64,bicycle_factor:f64)->f64 {
    let paced=3600.0/paced_per_kill_s(base_pace_s,cycle_s,bicycle_factor).max(0.001);
    paced.min(3600.0*MAX_AOE_ENGAGED/ttk.max(MIN_CYCLE_S))
}

/// Pure forecast, no observed-kill calibration: comparable full-wave conditions
/// for both current and alternative hunts. Observations remain separate in UI.
pub(super) fn simulate(
    hunt: &HuntEntry, mon: &PokemonInfo, player: &ReferenceSpecies,
    r: &Reference, stats: &[f64;6],
    aoe_enabled: bool, seasonal:Option<&str>, gym_multiplier:f64,
    bicycle_factor:f64, learned_map:Option<&str>,
) -> Option<CombatPace> {
    if hunt.level==0 || player.base.len()<6{return None}
    let pacing=native_atlas_pacing::resolve(&hunt.slug,learned_map);
    // The vendored map pace is the authoritative one-shot downtime baseline.
    // Passing it through the attack scheduler also lets cooldowns advance during
    // the same map-specific between-kill time used by the final throughput.
    let downtime=pacing.base_seconds_per_kill;
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
                damage:move_damage(r,mon.level,mv.power,atk,def,&mv.move_type,&player.types,foe,
                    seasonal_damage_multiplier(seasonal,&player.types,&foe.types)*gym_multiplier),
                cooldown_s:skill_cooldown_ms(mv.cooldown,player.base[5],mon)/1000.0,
            });
        }
        // Z-Move is a permanent 2800-power SPECIAL TM with a 60s base cooldown.
        if let Some(element)=mon.tm_elemental.as_deref().filter(|kind|
            player.types.iter().any(|t|t.as_str()==*kind)) {
            attacks.push(Attack{
                damage:move_damage(r,mon.level,2800.0,stats[3],
                    wild_stat(foe.base[4],hunt.level,16.0),
                    element,&player.types,foe,
                    seasonal_damage_multiplier(seasonal,&player.types,&foe.types)*gym_multiplier),
                cooldown_s:skill_cooldown_ms(60_000.0,player.base[5],mon)/1000.0,
            });
        }
        let tackle_type=effective_tackle_type(&player.types,foe,r);
        let basic=Attack {
            damage:move_damage(r,mon.level,TACKLE_POWER,stats[1],
                wild_stat(foe.base[2],hunt.level,16.0),
                tackle_type,&player.types,foe,
                seasonal_damage_multiplier(seasonal,&player.types,&foe.types)*gym_multiplier),
            cooldown_s:iv_cooldown_ms(TACKLE_COOLDOWN_MS,mon)/1000.0,
        };
        let mut hp=[0.0;32];
        for (i,value) in hp.iter_mut().enumerate() {
            let iv=(i+1) as f64;
            *value=(wild_stat(foe.base[0],hunt.level,iv)*12.0).round()
                .max(24.0)*5.0;
        }
        let time=mean_ttk(&attacks,basic,gap_s,&hp,downtime)?;
        let threat=incoming(r,foe,hunt.level,player,stats,seasonal);
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
    // ritmo.js gives the one-shot, on-foot total pace per map. Combat slower
    // than the one-shot floor is added on top; bicycles divide only the
    // movement component inferred from that authoritative baseline.
    let solo_kills_h=single_target_kills(pacing.base_seconds_per_kill,cycle,bicycle_factor);
    let mut kills_h=solo_kills_h;
    let aoe=aoe_enabled;
    let mut engaged=1.0;
    if aoe_enabled {
        // Only an explicit per-account Atlas checkbox activates this mode.
        // Never infer it from a live monster's partially populated TM flags.
        // The AoE scenario is never modeled as slower than single-target.
        kills_h=kills_h.max(aoe_kills(pacing.aoe_seconds_per_kill,cycle,ttk,bicycle_factor));
        engaged=(1.0+kills_h*ttk/3600.0).min(MAX_AOE_ENGAGED);
    }
    let own_hp=mon.max_hp.filter(|hp|*hp>0).map(|hp|hp as f64)
        .unwrap_or_else(||(stats[0]*12.0).round().max(24.0));
    let fatal_hit=hit_max+(engaged-1.0)*hit_mean;
    let one_hit=fatal_hit>=own_hp;
    let incoming_damage_per_kill=incoming_dps*engaged*ttk;
    // The extension's default is automatic potions. Non-lethal damage is healed.
    let kills_per_life=if one_hit {own_hp/fatal_hit.max(1e-6)} else {999.0};
    let recovery=RESPAWN_S+movement_component_s(pacing.base_seconds_per_kill,bicycle_factor);
    let alive_s=kills_per_life*3600.0/kills_h.max(1e-6);
    let uptime=alive_s/(alive_s+recovery);
    let deaths_h=if kills_per_life>=999.0 {0.0}else{3600.0/(alive_s+recovery)};
    kills_h*=uptime;
    if aoe_enabled {
        let solo_life=if hit_max>=own_hp {own_hp/hit_max.max(1e-6)} else {999.0};
        let solo_alive_s=solo_life*3600.0/solo_kills_h.max(1e-6);
        let solo_uptime=solo_alive_s/(solo_alive_s+recovery);
        kills_h=kills_h.max(solo_kills_h*solo_uptime);
    }
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
    fn seasonal_day_matches_upstream_attack_defense_hp_rule() {
        let fire=vec!["FIRE".to_owned()];
        let grass=vec!["GRASS".to_owned()];
        assert_eq!(seasonal_damage_multiplier(Some("FIRE"),&fire,&grass),2.0);
        assert_eq!(seasonal_damage_multiplier(Some("GRASS"),&fire,&grass),0.25);
        assert_eq!(seasonal_damage_multiplier(Some("FIRE"),&fire,&fire),0.5);
        assert_eq!(seasonal_damage_multiplier(None,&fire,&grass),1.0);
    }
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
    fn one_shot_foot_pace_matches_vendored_map_dataset() {
        let pace=native_atlas_pacing::resolve("caterpie",None);
        let kills=single_target_kills(pace.s,MIN_CYCLE_S,1.0);
        assert!((3600.0/kills-3.422).abs()<1e-12);
        let aoe=aoe_kills(pace.s_aoe,MIN_CYCLE_S,0.01,1.0);
        assert!((3600.0/aoe-3.12).abs()<1e-12);
    }
    #[test]
    fn unknown_hunt_uses_vendored_global_median() {
        let pace=native_atlas_pacing::resolve("__unknown_hunt__",None);
        assert_eq!(pace.source,native_atlas_pacing::PacingSource::GlobalFallback);
        let kills=single_target_kills(pace.s,MIN_CYCLE_S,1.0);
        assert!((3600.0/kills-3.406).abs()<1e-12);
    }
    #[test]
    fn learned_map_upgrades_unknown_hunt_to_shared_map_pacing() {
        let pace=native_atlas_pacing::resolve("__new_hunt__",Some("gerado_normal_01"));
        assert_eq!(pace.source,native_atlas_pacing::PacingSource::LearnedCampoInit);
        let aipom=native_atlas_pacing::resolve("aipom",None);
        assert_eq!(pace.map_id,aipom.map_id);
        assert_eq!(pace.base_seconds_per_kill,aipom.base_seconds_per_kill);
        assert_eq!(pace.uncertainty,aipom.uncertainty);
    }

    #[test]
    fn bicycle_factor_divides_only_map_movement_component() {
        let pace=native_atlas_pacing::resolve("caterpie",None);
        assert!((movement_component_s(pace.s,1.0)-(pace.s-MIN_CYCLE_S)).abs()<1e-12);
        assert!((movement_component_s(pace.s,2.0)-(pace.s-MIN_CYCLE_S)/2.0).abs()<1e-12);
        assert!((movement_component_s(pace.s,f64::NAN)-(pace.s-MIN_CYCLE_S)).abs()<1e-12);
        let foot=single_target_kills(pace.s,2.0,1.0);
        let legendary=single_target_kills(pace.s,2.0,2.0);
        assert!(legendary>foot);
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
