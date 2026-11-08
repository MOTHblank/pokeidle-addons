//! Native Hunt Atlas combat model; map pacing is vendored from /hunt for PokéIdle 1.0.17.
//! Pure, deterministic forecasts: no writes to game state, sessions or server hunt statistics.
use super::{damage, effect, Reference, ReferenceSpecies};
use crate::native_hunts::{HuntEntry, PokemonInfo};
use crate::native_atlas_pacing;

const PLAYER_GAP_MS: f64 = 900.0;
const TACKLE_COOLDOWN_MS: f64 = 2000.0;
const TACKLE_POWER: f64 = 30.0;
const MAX_AOE_ENGAGED: f64 = 15.0;

#[derive(Clone, Copy)]
struct PacingModelConstants {
    walking_portion: f64,
    walking_floor_s: f64,
    combat_interaction_tau_s: f64,
}
const PACING_MODEL: PacingModelConstants = PacingModelConstants {
    walking_portion: 0.83,
    walking_floor_s: 1.41,
    combat_interaction_tau_s: 2.8,
};
const WILD_DAMAGE_MULT: f64 = 1.8;
const WILD_ATTACK_GAP_S: f64 = 1.4;
const RESPAWN_S: f64 = 3.0;

#[derive(Clone, Debug)]
pub(super) struct CombatPace {
    pub kills_h: f64,
    pub seconds_per_kill: f64,
    pub combat_seconds_per_kill: f64,
    pub dead_time_seconds_per_kill: f64,
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

#[derive(Clone)]
struct CombatSample {
    weight: f64,
    attacks: Vec<Attack>,
    tackle: Attack,
    hp: [f64;32],
}

#[derive(Clone, Copy, Debug)]
struct PacedKill {
    dead_time_s: f64,
    combat_s: f64,
    total_s: f64,
}

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

/// Dead time is everything outside active combat: walking plus the map's
/// non-walking cadence. The vendored map `s` is the zero-combat, on-foot
/// baseline. As combat grows, enemies close distance while the player fights,
/// so only the walking portion decays toward its calibrated floor.
fn dead_time_s(map_baseline_s:f64,combat_s:f64,bicycle_factor:f64)->f64 {
    let baseline=map_baseline_s.max(0.0);
    let initial_walk=(PACING_MODEL.walking_portion*baseline).max(0.0);
    let reducible=(initial_walk-PACING_MODEL.walking_floor_s).max(0.0);
    let interaction=if combat_s.is_finite() {
        (-combat_s.max(0.0)/PACING_MODEL.combat_interaction_tau_s).exp()
    } else {0.0};
    let walk=initial_walk-reducible*(1.0-interaction);
    let fixed=(baseline-initial_walk).max(0.0);
    fixed+walk/normalized_bicycle_factor(bicycle_factor)
}

fn walking_dead_time_s(map_baseline_s:f64,combat_s:f64,bicycle_factor:f64)->f64 {
    let baseline=map_baseline_s.max(0.0);
    let initial_walk=(PACING_MODEL.walking_portion*baseline).max(0.0);
    let reducible=(initial_walk-PACING_MODEL.walking_floor_s).max(0.0);
    let interaction=if combat_s.is_finite() {
        (-combat_s.max(0.0)/PACING_MODEL.combat_interaction_tau_s).exp()
    } else {0.0};
    let walk=initial_walk-reducible*(1.0-interaction);
    walk/normalized_bicycle_factor(bicycle_factor)
}

fn weighted_ttk(samples:&[CombatSample],global_gap_s:f64,downtime_s:f64)->Option<f64> {
    let mut weighted=0.0;
    let mut total_weight=0.0;
    for sample in samples {
        let time=mean_ttk(&sample.attacks,sample.tackle,global_gap_s,&sample.hp,downtime_s)?;
        weighted+=sample.weight*time;
        total_weight+=sample.weight;
    }
    (total_weight>0.0).then_some(weighted/total_weight)
}

/// Resolve the feedback loop between dead time and combat cooldowns without
/// iterating the discontinuous attack scheduler directly. This mirrors /hunt:
/// simulate combat at the zero-combat map baseline and at the long-fight dead
/// time floor, interpolate combat between those endpoints, then solve the
/// dead-time curve against that interpolation.
fn paced_kill(
    samples:&[CombatSample],global_gap_s:f64,map_baseline_s:f64,bicycle_factor:f64,
)->Option<PacedKill> {
    let no_combat_dead=dead_time_s(map_baseline_s,0.0,bicycle_factor);
    let floor_dead=dead_time_s(map_baseline_s,f64::INFINITY,bicycle_factor);
    let combat_at_baseline=weighted_ttk(samples,global_gap_s,no_combat_dead)?;
    if !combat_at_baseline.is_finite(){return None}

    if combat_at_baseline<=0.02 || no_combat_dead-floor_dead<=0.02 {
        let dead=dead_time_s(map_baseline_s,combat_at_baseline,bicycle_factor);
        return Some(PacedKill {
            dead_time_s:dead,
            combat_s:combat_at_baseline,
            total_s:dead+combat_at_baseline,
        });
    }

    let combat_at_floor=weighted_ttk(samples,global_gap_s,floor_dead)?;
    if !combat_at_floor.is_finite(){return None}
    let span=(no_combat_dead-floor_dead).max(1e-9);
    let combat_with_dead=|dead:f64| {
        let t=((no_combat_dead-dead)/span).clamp(0.0,1.0);
        combat_at_baseline+t*(combat_at_floor-combat_at_baseline)
    };

    let mut lo=floor_dead;
    let mut hi=no_combat_dead;
    for _ in 0..30 {
        let mid=(lo+hi)/2.0;
        let combat=combat_with_dead(mid);
        if dead_time_s(map_baseline_s,combat,bicycle_factor)>=mid {
            lo=mid;
        } else {
            hi=mid;
        }
    }
    let solved_dead=(lo+hi)/2.0;
    let combat=combat_with_dead(solved_dead);
    let dead=dead_time_s(map_baseline_s,combat,bicycle_factor);
    Some(PacedKill {
        dead_time_s:dead,
        combat_s:combat,
        total_s:dead+combat,
    })
}

fn kills_per_hour(pace:PacedKill)->f64 {
    3600.0/pace.total_s.max(0.001)
}
fn aoe_kills_per_hour(pace:PacedKill)->f64 {
    kills_per_hour(pace)
        .min(3600.0*MAX_AOE_ENGAGED/pace.combat_s.max(0.001))
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
    let mut hit_mean=0.0;
    let mut hit_max=0.0f64;
    let mut incoming_dps=0.0;
    let mut samples=Vec::new();
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
        let threat=incoming(r,foe,hunt.level,player,stats,seasonal);
        weight+=w;
        incoming_dps+=w*threat.dps;
        hit_mean+=w*threat.mean_hit;
        hit_max=hit_max.max(threat.worst_hit);
        samples.push(CombatSample {weight:w,attacks,tackle:basic,hp});
    }
    let total=hunt.species.iter().map(|x|x.weight.max(0.0)).sum::<f64>();
    if weight<=0.0||total<=0.0{return None}
    let coverage=weight/total;
    if coverage<0.6{return None}
    hit_mean/=weight;
    incoming_dps/=weight;

    let solo_pace=paced_kill(&samples,gap_s,pacing.base_seconds_per_kill,bicycle_factor)?;
    let solo_kills_h=kills_per_hour(solo_pace);
    let mut selected_pace=solo_pace;
    let mut selected_baseline=pacing.base_seconds_per_kill;
    let mut kills_h=solo_kills_h;
    let aoe=aoe_enabled;
    let mut engaged=1.0;
    if aoe_enabled {
        // AoE keeps its own authoritative one-shot map baseline. Combat still
        // interacts with dead time through the same curve; the engagement cap
        // remains a separate simultaneous-target ceiling.
        let aoe_pace=paced_kill(&samples,gap_s,pacing.aoe_seconds_per_kill,bicycle_factor)?;
        let candidate=aoe_kills_per_hour(aoe_pace);
        if candidate>kills_h {
            kills_h=candidate;
            selected_pace=aoe_pace;
            selected_baseline=pacing.aoe_seconds_per_kill;
        }
        engaged=(1.0+kills_h*selected_pace.combat_s/3600.0).min(MAX_AOE_ENGAGED);
    }
    let own_hp=mon.max_hp.filter(|hp|*hp>0).map(|hp|hp as f64)
        .unwrap_or_else(||(stats[0]*12.0).round().max(24.0));
    let fatal_hit=hit_max+(engaged-1.0)*hit_mean;
    let one_hit=fatal_hit>=own_hp;
    let incoming_damage_per_kill=incoming_dps*engaged*selected_pace.combat_s;
    // The extension's default is automatic potions. Non-lethal damage is healed.
    let kills_per_life=if one_hit {own_hp/fatal_hit.max(1e-6)} else {999.0};
    let recovery=RESPAWN_S+
        walking_dead_time_s(selected_baseline,selected_pace.combat_s,bicycle_factor);
    let alive_s=kills_per_life*3600.0/kills_h.max(1e-6);
    let uptime=alive_s/(alive_s+recovery);
    let deaths_h=if kills_per_life>=999.0 {0.0}else{3600.0/(alive_s+recovery)};
    kills_h*=uptime;
    if aoe_enabled {
        let solo_life=if hit_max>=own_hp {own_hp/hit_max.max(1e-6)} else {999.0};
        let solo_recovery=RESPAWN_S+
            walking_dead_time_s(pacing.base_seconds_per_kill,solo_pace.combat_s,bicycle_factor);
        let solo_alive_s=solo_life*3600.0/solo_kills_h.max(1e-6);
        let solo_uptime=solo_alive_s/(solo_alive_s+solo_recovery);
        kills_h=kills_h.max(solo_kills_h*solo_uptime);
    }
    if !kills_h.is_finite()||!selected_pace.total_s.is_finite() {return None}
    Some(CombatPace {
        kills_h,
        seconds_per_kill:selected_pace.total_s,
        combat_seconds_per_kill:selected_pace.combat_s,
        dead_time_seconds_per_kill:selected_pace.dead_time_s,
        coverage,deaths_h,one_hit,aoe,engaged,
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
    fn sample(damage:f64,hp:f64)->CombatSample {
        CombatSample {
            weight:1.0,
            attacks:vec![Attack{damage,cooldown_s:0.9}],
            tackle:Attack{damage:0.0,cooldown_s:2.0},
            hp:[hp;32],
        }
    }

    #[test]
    fn pacing_constants_match_initial_hunt_calibration() {
        assert!((PACING_MODEL.walking_portion-0.83).abs()<1e-12);
        assert!((PACING_MODEL.walking_floor_s-1.41).abs()<1e-12);
        assert!((PACING_MODEL.combat_interaction_tau_s-2.8).abs()<1e-12);
    }

    #[test]
    fn dead_time_curve_starts_at_map_baseline_and_decays_to_walking_floor() {
        let s=3.422;
        let reducible=(0.83*s-1.41).max(0.0);
        assert!((dead_time_s(s,0.0,1.0)-s).abs()<1e-12);
        let at_tau=dead_time_s(s,2.8,1.0);
        let expected=s-reducible*(1.0-(-1.0f64).exp());
        assert!((at_tau-expected).abs()<1e-12);
        assert!(dead_time_s(s,4.0,1.0)<dead_time_s(s,1.0,1.0));
        let floor=s-reducible;
        assert!((dead_time_s(s,f64::INFINITY,1.0)-floor).abs()<1e-12);
    }

    #[test]
    fn one_shot_foot_pace_matches_vendored_map_dataset() {
        let pace=native_atlas_pacing::resolve("caterpie",None);
        let one=[sample(1000.0,100.0)];
        let solo=paced_kill(&one,0.9,pace.base_seconds_per_kill,1.0).unwrap();
        assert!(solo.combat_s.abs()<1e-12);
        assert!((solo.dead_time_s-3.422).abs()<1e-12);
        assert!((solo.total_s-3.422).abs()<1e-12);
        assert!((3600.0/kills_per_hour(solo)-3.422).abs()<1e-12);
        let aoe=paced_kill(&one,0.9,pace.aoe_seconds_per_kill,1.0).unwrap();
        assert!((aoe.total_s-3.12).abs()<1e-12);
        assert!((3600.0/kills_per_hour(aoe)-3.12).abs()<1e-12);
    }

    #[test]
    fn longer_combat_reduces_dead_time_but_total_is_dead_time_plus_combat() {
        let s=3.422;
        let longer=[sample(60.0,180.0)];
        let pace=paced_kill(&longer,0.9,s,1.0).unwrap();
        assert!(pace.combat_s>0.0);
        assert!(pace.dead_time_s<s);
        assert!(pace.dead_time_s>dead_time_s(s,f64::INFINITY,1.0)-1e-9);
        assert!((pace.total_s-(pace.dead_time_s+pace.combat_s)).abs()<1e-12);
    }

    #[test]
    fn unknown_hunt_uses_vendored_global_median() {
        let pace=native_atlas_pacing::resolve("__unknown_hunt__",None);
        assert_eq!(pace.source,native_atlas_pacing::PacingSource::GlobalFallback);
        let one=[sample(1000.0,100.0)];
        let resolved=paced_kill(&one,0.9,pace.base_seconds_per_kill,1.0).unwrap();
        assert!((resolved.total_s-3.406).abs()<1e-12);
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
    fn bicycle_divides_only_the_dead_time_walking_component() {
        let s=3.422;
        let fixed=s*(1.0-PACING_MODEL.walking_portion);
        let initial_walk=s*PACING_MODEL.walking_portion;
        assert!((dead_time_s(s,0.0,2.0)-(fixed+initial_walk/2.0)).abs()<1e-12);
        let foot_floor=dead_time_s(s,f64::INFINITY,1.0);
        let bike_floor=dead_time_s(s,f64::INFINITY,2.0);
        assert!((foot_floor-(fixed+PACING_MODEL.walking_floor_s)).abs()<1e-12);
        assert!((bike_floor-(fixed+PACING_MODEL.walking_floor_s/2.0)).abs()<1e-12);
        assert!(bike_floor<foot_floor);
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
