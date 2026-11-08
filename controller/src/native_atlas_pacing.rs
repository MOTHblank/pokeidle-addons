//! Static pacing data vendored from /hunt for PokeIdle 1.0.17.
//! The source dataset was generated from the PokeIdle 1.409.0 simulator/export.
//! Hunt Atlas treats its per-map one-shot pacing as authoritative until replaced
//! by a controller-owned pacing dataset.

use std::{collections::BTreeMap, sync::OnceLock};

pub const SOURCE_EXTENSION_VERSION: &str = "1.0.17";
pub const SOURCE_GAME_VERSION: &str = "1.409.0";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PacingRange {
    pub p10: f64,
    pub median: f64,
    pub p90: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapPacing {
    /// Seconds per kill for the one-shot, on-foot profile.
    pub s: f64,
    /// Seconds per kill for the one-shot AoE profile.
    pub s_aoe: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HuntPacing {
    pub map_id: String,
    pub capture_ceiling: u32,
}

#[derive(Debug)]
pub struct PacingDataset {
    pub origin: String,
    pub source: String,
    pub game_version: String,
    pub generated_at: String,
    pub fallback: PacingRange,
    pub aoe_gain: f64,
    pub maps: BTreeMap<String, MapPacing>,
    pub hunts: BTreeMap<String, HuntPacing>,
}

pub const MAP_RELATIVE_UNCERTAINTY: f64 = 0.09;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacingSource {
    HuntSlugMapping,
    LearnedCampoInit,
    GlobalFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPacing {
    pub source: PacingSource,
    pub map_id: Option<String>,
    pub base_seconds_per_kill: f64,
    pub aoe_seconds_per_kill: f64,
    pub uncertainty: PacingRange,
    pub aoe_uncertainty: PacingRange,
}

const RAW: &str = include_str!("../assets/hunt-atlas/ritmo-v1409.tsv");

fn parse() -> PacingDataset {
    let mut origin=String::new();
    let mut source=String::new();
    let mut game_version=String::new();
    let mut generated_at=String::new();
    let mut p10=None;
    let mut median=None;
    let mut p90=None;
    let mut aoe_gain=None;
    let mut maps=BTreeMap::new();
    let mut hunts=BTreeMap::new();

    for line in RAW.lines().filter(|line|!line.is_empty()&&!line.starts_with('#')) {
        let parts=line.split('\t').collect::<Vec<_>>();
        match parts.first().copied() {
            Some("meta") if parts.len()>=3 => match parts[1] {
                "origin" => origin=parts[2].to_owned(),
                "source" => source=parts[2].to_owned(),
                "game_version" => game_version=parts[2].to_owned(),
                "generated_at" => generated_at=parts[2].to_owned(),
                "fallback_p10" => p10=parts[2].parse::<f64>().ok(),
                "fallback_median" => median=parts[2].parse::<f64>().ok(),
                "fallback_p90" => p90=parts[2].parse::<f64>().ok(),
                "aoe_gain" => aoe_gain=parts[2].parse::<f64>().ok(),
                _ => {},
            },
            Some("map") if parts.len()==4 => {
                if let (Ok(s),Ok(s_aoe))=(parts[2].parse::<f64>(),parts[3].parse::<f64>()) {
                    maps.insert(parts[1].to_owned(),MapPacing{s,s_aoe});
                }
            },
            Some("hunt") if parts.len()==4 => {
                if let Ok(capture_ceiling)=parts[3].parse::<u32>() {
                    hunts.insert(parts[1].to_owned(),HuntPacing {
                        map_id:parts[2].to_owned(),capture_ceiling,
                    });
                }
            },
            _ => {},
        }
    }

    PacingDataset {
        origin,source,game_version,generated_at,
        fallback:PacingRange {
            p10:p10.expect("ritmo dataset missing fallback p10"),
            median:median.expect("ritmo dataset missing fallback median"),
            p90:p90.expect("ritmo dataset missing fallback p90"),
        },
        aoe_gain:aoe_gain.expect("ritmo dataset missing AoE gain"),
        maps,hunts,
    }
}

pub fn dataset() -> &'static PacingDataset {
    static DATA:OnceLock<PacingDataset>=OnceLock::new();
    DATA.get_or_init(parse)
}

fn known_map(source:PacingSource,map_id:&str,map:MapPacing)->ResolvedPacing {
    let r=MAP_RELATIVE_UNCERTAINTY;
    ResolvedPacing {
        source,
        map_id:Some(map_id.to_owned()),
        base_seconds_per_kill:map.s,
        aoe_seconds_per_kill:map.s_aoe,
        uncertainty:PacingRange {
            p10:map.s*(1.0-r), median:map.s, p90:map.s*(1.0+r),
        },
        aoe_uncertainty:PacingRange {
            p10:map.s_aoe*(1.0-r), median:map.s_aoe, p90:map.s_aoe*(1.0+r),
        },
    }
}

pub fn resolve(slug:&str, learned_map:Option<&str>) -> ResolvedPacing {
    let data=dataset();
    let static_map=data.hunts.get(slug).map(|hunt|hunt.map_id.as_str());
    if let Some(map_id)=static_map {
        if let Some(map)=data.maps.get(map_id) {
            return known_map(PacingSource::HuntSlugMapping,map_id,*map);
        }
    }
    if let Some(map_id)=learned_map.filter(|map|!map.is_empty()) {
        if let Some(map)=data.maps.get(map_id) {
            return known_map(PacingSource::LearnedCampoInit,map_id,*map);
        }
    }
    let base=data.fallback.median;
    let aoe=base/data.aoe_gain;
    let r=MAP_RELATIVE_UNCERTAINTY;
    ResolvedPacing {
        source:PacingSource::GlobalFallback,
        map_id:learned_map.filter(|map|!map.is_empty()).or(static_map).map(str::to_owned),
        base_seconds_per_kill:base,
        aoe_seconds_per_kill:aoe,
        uncertainty:PacingRange {
            p10:data.fallback.p10*(1.0-r), median:base,
            p90:data.fallback.p90*(1.0+r),
        },
        aoe_uncertainty:PacingRange {
            p10:data.fallback.p10/data.aoe_gain*(1.0-r), median:aoe,
            p90:data.fallback.p90/data.aoe_gain*(1.0+r),
        },
    }
}

pub fn capture_ceiling(slug:&str)->Option<u32> {
    dataset().hunts.get(slug).map(|hunt|hunt.capture_ceiling)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendors_complete_hunt_1_0_17_pacing_dataset() {
        let data=dataset();
        assert_eq!(data.game_version,SOURCE_GAME_VERSION);
        assert_eq!(SOURCE_EXTENSION_VERSION,"1.0.17");
        assert_eq!(data.source,"jogo");
        assert_eq!(data.generated_at,"2026-11-07T12:26:24.998Z");
        assert_eq!(data.maps.len(),500);
        assert_eq!(data.hunts.len(),1043);
        assert!(data.hunts.values().all(|hunt|data.maps.contains_key(&hunt.map_id)));
        assert_eq!(data.fallback,PacingRange{p10:3.27,median:3.406,p90:3.55});
        assert!((data.aoe_gain-1.096).abs()<1e-12);
        assert!(!data.origin.is_empty());
    }

    #[test]
    fn resolves_hunt_slug_to_map_and_exact_map_pacing() {
        let pace=resolve("caterpie",None);
        assert_eq!(pace.source,PacingSource::HuntSlugMapping);
        assert_eq!(pace.map_id.as_deref(),Some("heracross"));
        assert!((pace.base_seconds_per_kill-3.422).abs()<1e-12);
        assert!((pace.aoe_seconds_per_kill-3.12).abs()<1e-12);
        assert_eq!(pace.uncertainty.median,pace.base_seconds_per_kill);
        assert_eq!(capture_ceiling("caterpie"),Some(20));
    }

    #[test]
    fn learned_campo_map_is_second_priority_and_shared_maps_share_pacing() {
        let learned=resolve("__new_hunt__",Some("gerado_normal_01"));
        assert_eq!(learned.source,PacingSource::LearnedCampoInit);
        assert_eq!(learned.map_id.as_deref(),Some("gerado_normal_01"));
        let aipom=resolve("aipom",None);
        let glameow=resolve("glameow_sinnoh",None);
        assert_eq!(aipom.map_id,glameow.map_id);
        assert_eq!(aipom.base_seconds_per_kill,glameow.base_seconds_per_kill);
        assert_eq!(aipom.aoe_seconds_per_kill,glameow.aoe_seconds_per_kill);

        let static_wins=resolve("caterpie",Some("gerado_normal_01"));
        assert_eq!(static_wins.source,PacingSource::HuntSlugMapping);
        assert_eq!(static_wins.map_id.as_deref(),Some("heracross"));
    }

    #[test]
    fn unknown_hunt_uses_dataset_fallback_not_an_unrelated_map() {
        let pace=resolve("__unknown_hunt__",None);
        assert_eq!(pace.source,PacingSource::GlobalFallback);
        assert_eq!(pace.map_id,None);
        assert!((pace.base_seconds_per_kill-3.406).abs()<1e-12);
        assert!((pace.aoe_seconds_per_kill-(3.406/1.096)).abs()<1e-12);
        assert!((pace.uncertainty.p10-3.27*0.91).abs()<1e-12);
        assert!((pace.uncertainty.p90-3.55*1.09).abs()<1e-12);
    }
}
