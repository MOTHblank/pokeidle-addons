use crate::reduce::{MapReduction, ProfileStats, ReducedDataset};
use crate::hunt_pacing_model;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const BORROWED_TARGET: &str = include_str!("../../assets/hunt-atlas/ritmo-v1409.tsv");
const ENOUGH_WINDOWS: usize = 3;

#[derive(Clone, Debug)]
struct TargetCorpus {
    maps: BTreeSet<String>,
    paces: BTreeMap<String, (f64, f64)>,
    hunts: BTreeMap<String, (String, String)>,
}

pub fn write_reduced(path: &Path, reduced: &ReducedDataset) -> Result<(), String> {
    write_json(path, reduced)
}

pub fn read_reduced(path: &Path) -> Result<ReducedDataset, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))
}

pub fn write_dataset_json(path: &Path, reduced: &ReducedDataset) -> Result<(), String> {
    let dataset = dataset_json(reduced);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(&dataset).map_err(|error| error.to_string())?;
    fs::write(path, bytes)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

pub fn write_dataset_tsv(path: &Path, reduced: &ReducedDataset) -> Result<(), String> {
    let corpus = target_corpus();
    let fallback = fallback_values(reduced);
    let aoe_gain = aoe_gain(reduced);
    let mut out = String::new();
    out.push_str("# Moth live-observation pacing dataset. Only maps with measured ST1 and AOE are emitted.\n");
    out.push_str("# record formats: meta<TAB>key<TAB>value | map<TAB>map_id<TAB>s<TAB>s_aoe | hunt<TAB>slug<TAB>map_id<TAB>capture_ceiling\n");
    out.push_str("meta\torigin\tMoth empirical server-observed hunt pacing\n");
    out.push_str("meta\tsource\tmoth-live-observation\n");
    out.push_str(&format!("meta\tgame_version\t{}\n", clean_field(&reduced.game_version)));
    out.push_str(&format!("meta\tgenerated_at\t{}\n", reduced.generated_at_unix_ms));
    out.push_str(&format!("meta\tfallback_p10\t{:.6}\n", fallback.0));
    out.push_str(&format!("meta\tfallback_median\t{:.6}\n", fallback.1));
    out.push_str(&format!("meta\tfallback_p90\t{:.6}\n", fallback.2));
    out.push_str(&format!("meta\taoe_gain\t{:.6}\n", aoe_gain));

    let mut exported_maps = BTreeSet::new();
    for (map_id, map) in &reduced.maps {
        let Some(st1) = map.profiles.get("st1") else { continue };
        let Some(aoe) = map.profiles.get("aoe") else { continue };
        out.push_str(&format!(
            "map\t{}\t{:.6}\t{:.6}\n",
            clean_field(map_id),
            st1.median_seconds_per_kill,
            aoe.median_seconds_per_kill,
        ));
        exported_maps.insert(map_id.clone());
    }

    for (slug, map_id) in &reduced.hunts {
        if !exported_maps.contains(map_id) { continue; }
        let ceiling = corpus.hunts.get(slug)
            .map(|(_, ceiling)| ceiling.as_str()).unwrap_or("0");
        out.push_str(&format!(
            "hunt\t{}\t{}\t{}\n",
            clean_field(slug), clean_field(map_id), clean_field(ceiling)
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    fs::write(path, out)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

pub fn coverage_report(reduced: &ReducedDataset) -> String {
    let corpus = target_corpus();
    let measured_st1 = corpus.maps.iter()
        .filter(|map_id| profile(reduced, map_id, "st1").is_some())
        .count();
    let measured_aoe = corpus.maps.iter()
        .filter(|map_id| profile(reduced, map_id, "aoe").is_some())
        .count();
    let enough_st1 = corpus.maps.iter()
        .filter(|map_id| profile(reduced, map_id, "st1")
            .is_some_and(enough))
        .count();
    let enough_aoe = corpus.maps.iter()
        .filter(|map_id| profile(reduced, map_id, "aoe")
            .is_some_and(enough))
        .count();

    let need_st1 = corpus.maps.iter()
        .filter(|map_id| !profile(reduced, map_id, "st1").is_some_and(enough))
        .cloned().collect::<Vec<_>>();
    let need_aoe = corpus.maps.iter()
        .filter(|map_id| !profile(reduced, map_id, "aoe").is_some_and(enough))
        .cloned().collect::<Vec<_>>();

    let observed_expected_hunts = corpus.hunts.keys()
        .filter(|slug| reduced.hunts.contains_key(*slug)).count();
    let unobserved_hunts = corpus.hunts.keys()
        .filter(|slug| !reduced.hunts.contains_key(*slug)
            && !reduced.unknown_hunts.contains(*slug))
        .cloned().collect::<Vec<_>>();
    let comparisons = pacing_comparisons(reduced, &corpus);
    let shared_maps = observed_shared_maps(reduced);
    let bike_validation = bicycle_validation_lines(reduced);
    let deadtime_validation = deadtime_validation_lines(reduced);

    format!(
r#"Hunt pacing coverage
  expected pacing map IDs: {expected}
  maps with ST1 data:      {measured_st1}
  maps with enough ST1:    {enough_st1}
  maps with AOE data:      {measured_aoe}
  maps with enough AOE:    {enough_aoe}
  expected hunts observed: {observed_expected_hunts}/{expected_hunts}
  hunt identities not yet observed: {unobserved_count}
  conflicting observed identities:  {unknown}

Maps needing more ST1 samples ({need_st1_count}):
{need_st1_lines}

Maps needing more AOE samples ({need_aoe_count}):
{need_aoe_lines}

Target hunts whose map identity has not been observed ({unobserved_count}):
{unobserved_lines}

Hunts with conflicting observed map identity ({unknown_count}):
{unknown_lines}

Measured maps vs current /hunt target:
{comparison_lines}

Observed map IDs shared by multiple hunt slugs:
{shared_map_lines}

Bicycle validation against fixed + walking/factor:
{bike_validation_lines}

Dead-time validation against 0.83 / 1.41 / 2.8:
{deadtime_validation_lines}
"#,
        expected = corpus.maps.len(),
        expected_hunts = corpus.hunts.len(),
        unknown = reduced.unknown_hunts.len(),
        unobserved_count = unobserved_hunts.len(),
        need_st1_count = need_st1.len(),
        need_aoe_count = need_aoe.len(),
        need_st1_lines = format_lines(&need_st1),
        need_aoe_lines = format_lines(&need_aoe),
        unobserved_lines = format_lines(&unobserved_hunts),
        unknown_count = reduced.unknown_hunts.len(),
        unknown_lines = format_lines(&reduced.unknown_hunts),
        comparison_lines = if comparisons.is_empty() {
            "  (none)".into()
        } else { comparisons.join("\n") },
        shared_map_lines = if shared_maps.is_empty() {
            "  (none)".into()
        } else { shared_maps.join("\n") },
        bike_validation_lines = if bike_validation.is_empty() {
            "  (none)".into()
        } else { bike_validation.join("\n") },
        deadtime_validation_lines = if deadtime_validation.is_empty() {
            "  (none)".into()
        } else { deadtime_validation.join("\n") },
    )
}

pub fn dataset_json(reduced: &ReducedDataset) -> Value {
    let corpus = target_corpus();
    let fallback = fallback_values(reduced);
    let mut maps = serde_json::Map::new();

    for (map_id, map) in &reduced.maps {
        let st1 = map.profiles.get("st1");
        let aoe = map.profiles.get("aoe");
        if st1.is_none() && aoe.is_none() { continue; }
        maps.insert(map_id.clone(), json!({
            "s": st1.map(|stats| stats.median_seconds_per_kill),
            "sAoe": aoe.map(|stats| stats.median_seconds_per_kill),
            "sample_count": {
                "st1": st1.map(|stats| stats.samples).unwrap_or(0),
                "aoe": aoe.map(|stats| stats.samples).unwrap_or(0),
            },
            "kill_count": {
                "st1": st1.map(|stats| stats.kills).unwrap_or(0),
                "aoe": aoe.map(|stats| stats.kills).unwrap_or(0),
            },
            "uncertainty": {
                "st1": uncertainty(st1),
                "aoe": uncertainty(aoe),
            },
            "statistics": {
                "st1": stats_json(st1),
                "aoe": stats_json(aoe),
            },
        }));
    }

    json!({
        "metadata": {
            "source": "moth-live-observation",
            "game_version": reduced.game_version,
            "generated_at": reduced.generated_at_unix_ms,
            "profile_definition": {
                "st1": "on foot; single-target; deliberately configured one-shot; stable server-observed state",
                "aoe": "on foot; AOE enabled; deliberately configured one-shot; actual kill batches retained",
                "window": {
                    "warmup_kills": reduced.config.warmup_kills,
                    "min_measured_kills": reduced.config.min_measured_kills,
                    "max_packet_gap_ms": reduced.config.max_packet_gap_ms,
                    "aggregate": "median window seconds/kill; weighted mean and dispersion retained",
                    "outlier_rule": format!("outside physical bounds or > {} MAD from map/profile median when >=5 windows",
                        reduced.config.outlier_mad_multiplier),
                }
            },
            "target_corpus": {
                "source": "existing ritmo-v1409.tsv identifiers only",
                "expected_maps": corpus.maps.len(),
                "expected_hunts": corpus.hunts.len(),
            }
        },
        "maps": maps,
        "hunts": reduced.hunts,
        "fallback": {
            "p10": fallback.0,
            "median": fallback.1,
            "p90": fallback.2,
        },
        "coverage": {
            "unknown_hunts": reduced.unknown_hunts,
            "rejected_windows": reduced.rejected_windows,
        },
        "validation": {
            "bicycle": bicycle_validation_json(reduced),
            "deadtime": deadtime_validation_json(reduced),
            "model": {
                "walking_portion": hunt_pacing_model::WALKING_PORTION,
                "walking_floor_seconds": hunt_pacing_model::WALKING_FLOOR_S,
                "combat_interaction_tau_seconds": hunt_pacing_model::COMBAT_INTERACTION_TAU_S,
            }
        }
    })
}

fn bicycle_validation_lines(reduced: &ReducedDataset) -> Vec<String> {
    let mut lines = Vec::new();
    for (map_id, map) in &reduced.maps {
        let Some(st1) = map.profiles.get("st1") else { continue };
        let baseline = st1.median_seconds_per_kill;
        for (profile, stats) in &map.profiles {
            if !profile.starts_with("bike-validation:") || stats.windows.is_empty() {
                continue;
            }
            let factor = stats.windows[0].bicycle_factor;
            let observed = stats.median_seconds_per_kill;
            let predicted = hunt_pacing_model::total_cycle_s(baseline, 0.0, factor);
            let pct = relative_error_pct(observed, predicted);
            lines.push(format!(
                "  {map_id} {profile}: observed {observed:.4}s, predicted {predicted:.4}s ({pct:+.2}%)"
            ));
        }
    }
    lines
}

fn deadtime_validation_lines(reduced: &ReducedDataset) -> Vec<String> {
    let mut lines = Vec::new();
    for (map_id, map) in &reduced.maps {
        let Some(st1) = map.profiles.get("st1") else { continue };
        let baseline = st1.median_seconds_per_kill;
        for (profile, stats) in &map.profiles {
            if !profile.starts_with("deadtime-validation:") || stats.windows.is_empty() {
                continue;
            }
            let window = &stats.windows[0];
            let Some(combat_seconds) = window.validation_combat_seconds else {
                lines.push(format!(
                    "  {map_id} {profile}: missing declared combat seconds; measurement retained but model comparison skipped"
                ));
                continue;
            };
            let observed = stats.median_seconds_per_kill;
            let predicted = hunt_pacing_model::total_cycle_s(baseline, combat_seconds, 1.0);
            let pct = relative_error_pct(observed, predicted);
            let hits = window.declared_hits
                .map(|hits| hits.to_string()).unwrap_or_else(|| "?".into());
            lines.push(format!(
                "  {map_id} hits={hits} combat={combat_seconds:.3}s: observed {observed:.4}s, predicted {predicted:.4}s ({pct:+.2}%)"
            ));
        }
    }
    lines
}

fn bicycle_validation_json(reduced: &ReducedDataset) -> Value {
    let mut rows = Vec::new();
    for (map_id, map) in &reduced.maps {
        let Some(st1) = map.profiles.get("st1") else { continue };
        let baseline = st1.median_seconds_per_kill;
        for (profile, stats) in &map.profiles {
            if !profile.starts_with("bike-validation:") || stats.windows.is_empty() {
                continue;
            }
            let factor = stats.windows[0].bicycle_factor;
            let observed = stats.median_seconds_per_kill;
            let predicted = hunt_pacing_model::total_cycle_s(baseline, 0.0, factor);
            rows.push(json!({
                "map_id": map_id,
                "profile": profile,
                "factor": factor,
                "baseline_st1_seconds_per_kill": baseline,
                "observed_seconds_per_kill": observed,
                "predicted_seconds_per_kill": predicted,
                "relative_error_pct": relative_error_pct(observed, predicted),
                "samples": stats.samples,
                "kills": stats.kills,
            }));
        }
    }
    Value::Array(rows)
}

fn deadtime_validation_json(reduced: &ReducedDataset) -> Value {
    let mut rows = Vec::new();
    for (map_id, map) in &reduced.maps {
        let Some(st1) = map.profiles.get("st1") else { continue };
        let baseline = st1.median_seconds_per_kill;
        for (profile, stats) in &map.profiles {
            if !profile.starts_with("deadtime-validation:") || stats.windows.is_empty() {
                continue;
            }
            let window = &stats.windows[0];
            let predicted = window.validation_combat_seconds
                .map(|combat| hunt_pacing_model::total_cycle_s(baseline, combat, 1.0));
            rows.push(json!({
                "map_id": map_id,
                "profile": profile,
                "declared_hits": window.declared_hits,
                "combat_seconds": window.validation_combat_seconds,
                "baseline_st1_seconds_per_kill": baseline,
                "observed_seconds_per_kill": stats.median_seconds_per_kill,
                "predicted_seconds_per_kill": predicted,
                "relative_error_pct": predicted.map(|predicted|
                    relative_error_pct(stats.median_seconds_per_kill, predicted)),
                "samples": stats.samples,
                "kills": stats.kills,
            }));
        }
    }
    Value::Array(rows)
}

fn relative_error_pct(observed: f64, predicted: f64) -> f64 {
    if predicted > 0.0 && predicted.is_finite() {
        100.0 * (observed / predicted - 1.0)
    } else {
        0.0
    }
}

fn stats_json(stats: Option<&ProfileStats>) -> Value {
    let Some(stats) = stats else { return Value::Null };
    json!({
        "samples": stats.samples,
        "kills": stats.kills,
        "total_seconds": stats.total_seconds,
        "median_seconds_per_kill": stats.median_seconds_per_kill,
        "mean_seconds_per_kill": stats.mean_seconds_per_kill,
        "p10": stats.p10,
        "p90": stats.p90,
        "mad": stats.mad,
        "rejected_outlier_samples": stats.rejected_outlier_samples,
    })
}

fn uncertainty(stats: Option<&ProfileStats>) -> Value {
    let Some(stats) = stats else { return Value::Null };
    json!({
        "p10": stats.p10,
        "p90": stats.p90,
        "mad": stats.mad,
    })
}

fn enough(stats: &ProfileStats) -> bool {
    stats.samples >= ENOUGH_WINDOWS
}

fn profile<'a>(reduced: &'a ReducedDataset, map_id: &str, name: &str)
    -> Option<&'a ProfileStats>
{
    reduced.maps.get(map_id)?.profiles.get(name)
}

fn fallback_values(reduced: &ReducedDataset) -> (f64, f64, f64) {
    let values = reduced.maps.values()
        .filter_map(|map| map.profiles.get("st1"))
        .map(|stats| stats.median_seconds_per_kill)
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    if values.is_empty() { return (0.0, 0.0, 0.0); }
    (
        quantile(values.clone(), 0.10),
        quantile(values.clone(), 0.50),
        quantile(values, 0.90),
    )
}

fn aoe_gain(reduced: &ReducedDataset) -> f64 {
    let ratios = reduced.maps.values().filter_map(|map| {
        let st1 = map.profiles.get("st1")?.median_seconds_per_kill;
        let aoe = map.profiles.get("aoe")?.median_seconds_per_kill;
        (st1 > 0.0 && aoe > 0.0).then_some(st1 / aoe)
    }).collect::<Vec<_>>();
    if ratios.is_empty() { 1.0 } else { quantile(ratios, 0.5) }
}

fn quantile(mut values: Vec<f64>, q: f64) -> f64 {
    if values.is_empty() { return 0.0; }
    values.sort_by(|a, b| a.total_cmp(b));
    if values.len() == 1 { return values[0]; }
    let position = q.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper { return values[lower]; }
    let fraction = position - lower as f64;
    values[lower] + (values[upper] - values[lower]) * fraction
}

fn target_corpus() -> TargetCorpus {
    let mut maps = BTreeSet::new();
    let mut paces = BTreeMap::new();
    let mut hunts = BTreeMap::new();
    for line in BORROWED_TARGET.lines() {
        if line.starts_with('#') || line.trim().is_empty() { continue; }
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.first().copied() {
            Some("map") if fields.len() >= 4 => {
                maps.insert(fields[1].to_owned());
                if let (Ok(st1), Ok(aoe)) = (fields[2].parse::<f64>(), fields[3].parse::<f64>()) {
                    paces.insert(fields[1].to_owned(), (st1, aoe));
                }
            }
            Some("hunt") if fields.len() >= 4 => {
                hunts.insert(fields[1].to_owned(), (fields[2].to_owned(), fields[3].to_owned()));
            }
            _ => {}
        }
    }
    TargetCorpus { maps, paces, hunts }
}

fn pacing_comparisons(reduced: &ReducedDataset, corpus: &TargetCorpus) -> Vec<String> {
    let mut lines = Vec::new();
    for (map_id, map) in &reduced.maps {
        let Some((borrowed_st1, borrowed_aoe)) = corpus.paces.get(map_id).copied() else {
            continue;
        };
        let st1 = map.profiles.get("st1");
        let aoe = map.profiles.get("aoe");
        if st1.is_none() && aoe.is_none() { continue; }
        let st1_text = st1.map(|stats| {
            let moth = stats.median_seconds_per_kill;
            let pct = if borrowed_st1 > 0.0 { 100.0 * (moth / borrowed_st1 - 1.0) } else { 0.0 };
            format!("ST1 {moth:.4}s vs {borrowed_st1:.4}s ({pct:+.2}%)")
        }).unwrap_or_else(|| "ST1 —".into());
        let aoe_text = aoe.map(|stats| {
            let moth = stats.median_seconds_per_kill;
            let pct = if borrowed_aoe > 0.0 { 100.0 * (moth / borrowed_aoe - 1.0) } else { 0.0 };
            format!("AOE {moth:.4}s vs {borrowed_aoe:.4}s ({pct:+.2}%)")
        }).unwrap_or_else(|| "AOE —".into());
        lines.push(format!("  {map_id}: {st1_text}; {aoe_text}"));
    }
    lines
}

fn observed_shared_maps(reduced: &ReducedDataset) -> Vec<String> {
    let mut by_map = BTreeMap::<String, Vec<String>>::new();
    for (hunt, map) in &reduced.hunts {
        by_map.entry(map.clone()).or_default().push(hunt.clone());
    }
    by_map.into_iter()
        .filter(|(_, hunts)| hunts.len() > 1)
        .map(|(map, hunts)| format!("  {map}: {}", hunts.join(", ")))
        .collect()
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn clean_field(raw: &str) -> String {
    raw.replace('\t', " ").replace('\n', " ").replace('\r', " ")
}

fn format_lines(values: &[String]) -> String {
    if values.is_empty() {
        "  (none)".into()
    } else {
        values.iter().map(|value| format!("  {value}"))
            .collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_corpus_is_the_exact_borrowed_identifier_set() {
        let corpus = target_corpus();
        assert_eq!(corpus.maps.len(), 500);
        assert_eq!(corpus.hunts.len(), 1043);
        assert!(corpus.maps.contains("aipom"));
    }
}
