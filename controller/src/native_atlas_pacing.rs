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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedPacing<'a> {
    pub map_id: Option<&'a str>,
    pub s: f64,
    pub s_aoe: f64,
    pub fallback: bool,
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

pub fn resolve(slug:&str) -> ResolvedPacing<'static> {
    let data=dataset();
    let hunt=data.hunts.get(slug);
    if let Some(hunt)=hunt {
        if let Some(map)=data.maps.get(&hunt.map_id) {
            return ResolvedPacing {
                map_id:Some(hunt.map_id.as_str()),
                s:map.s,
                s_aoe:map.s_aoe,
                fallback:false,
            };
        }
    }
    let s=data.fallback.median;
    ResolvedPacing {
        map_id:hunt.map(|hunt|hunt.map_id.as_str()),
        s,
        s_aoe:s/data.aoe_gain,
        fallback:true,
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
        assert_eq!(data.maps.len(),500);
        assert_eq!(data.hunts.len(),1043);
        assert_eq!(data.fallback,PacingRange{p10:3.27,median:3.406,p90:3.55});
        assert!((data.aoe_gain-1.096).abs()<1e-12);
        assert!(!data.origin.is_empty());
        assert!(!data.generated_at.is_empty());
    }

    #[test]
    fn resolves_hunt_slug_to_map_and_exact_map_pacing() {
        let pace=resolve("caterpie");
        assert_eq!(pace.map_id,Some("heracross"));
        assert!((pace.s-3.422).abs()<1e-12);
        assert!((pace.s_aoe-3.12).abs()<1e-12);
        assert!(!pace.fallback);
        assert_eq!(capture_ceiling("caterpie"),Some(20));
    }

    #[test]
    fn unknown_hunt_uses_dataset_fallback_not_an_unrelated_map() {
        let pace=resolve("__unknown_hunt__");
        assert_eq!(pace.map_id,None);
        assert!(pace.fallback);
        assert!((pace.s-3.406).abs()<1e-12);
        assert!((pace.s_aoe-(3.406/1.096)).abs()<1e-12);
    }
}
