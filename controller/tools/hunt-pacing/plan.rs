use crate::reduce::ReducedDataset;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const BORROWED_TARGET: &str = include_str!("../../assets/hunt-atlas/ritmo-v1409.tsv");
const ENOUGH_WINDOWS: usize = 3;

#[derive(Clone, Debug)]
struct Corpus {
    hunts: BTreeMap<String, String>,
    by_map: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationPlan {
    pub schema: u32,
    pub source: String,
    pub generated_from: String,
    pub jobs: Vec<CalibrationJob>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationJob {
    pub id: String,
    pub purpose: String,
    pub profile: String,
    pub hunt_slug: String,
    pub borrowed_map_hint: String,
    pub target_windows: usize,
    pub completed_windows: usize,
    pub notes: String,
    pub available_on_profiles: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub fn build(reduced: Option<&ReducedDataset>, limit: usize) -> CalibrationPlan {
    let corpus = corpus();
    let empty = empty_reduced();
    let baseline = reduced.unwrap_or(&empty);
    let mut jobs = Vec::new();
    let mut seen = BTreeSet::<(String, String)>::new();

    if let Some(reduced) = reduced {
        // First finish maps we have already proven live. Use the observed live
        // map identity; the borrowed map stays a hint only.
        for (map_id, map) in &reduced.maps {
            let hunt = reduced.hunts.iter()
                .find_map(|(hunt, live_map)| (live_map == map_id
                    && hunt_is_available(reduced, hunt)).then(|| hunt.clone()))
                .or_else(|| corpus.by_map.get(map_id).and_then(|hunts|
                    hunts.iter().find(|hunt| hunt_is_available(reduced, hunt)).cloned()));
            let Some(hunt) = hunt else { continue };
            if map.profiles.get("st1").map(|stats| stats.samples).unwrap_or(0) < ENOUGH_WINDOWS {
                add_job(
                    &mut jobs, &mut seen, reduced, &corpus, &hunt, "st1",
                    "finish-observed-map", "Complete ST1 reproducibility on an already observed live map.",
                    Some(map.profiles.get("st1").map(|stats| stats.samples).unwrap_or(0)),
                );
            }
            if map.profiles.get("aoe").map(|stats| stats.samples).unwrap_or(0) < ENOUGH_WINDOWS {
                add_job(
                    &mut jobs, &mut seen, reduced, &corpus, &hunt, "aoe",
                    "finish-observed-map", "Complete AoE pacing on an already observed live map.",
                    Some(map.profiles.get("aoe").map(|stats| stats.samples).unwrap_or(0)),
                );
            }
        }

        // Then explicitly test the premise that different hunts sharing one
        // live map converge to one map-level pace.
        for (map_id, map) in &reduced.maps {
            let Some(st1) = map.profiles.get("st1") else { continue };
            if st1.samples < ENOUGH_WINDOWS { continue; }
            let measured_hunts = st1.windows.iter()
                .filter_map(|window| window.hunt_slug.as_ref())
                .cloned().collect::<BTreeSet<_>>();
            let candidates = corpus.by_map.get(map_id).cloned().unwrap_or_default()
                .into_iter().filter(|hunt| hunt_is_available(reduced, hunt))
                .collect::<Vec<_>>();
            if measured_hunts.len() >= 2 || candidates.len() < 2 { continue; }
            if let Some(hunt) = candidates.into_iter()
                .find(|hunt| !measured_hunts.contains(hunt))
            {
                add_job(
                    &mut jobs, &mut seen, reduced, &corpus, &hunt, "st1",
                    "shared-map-convergence",
                    "Measure a second hunt that the borrowed corpus says shares this map. The live campo.init map ID decides whether it actually does.",
                    None,
                );
            }
        }
    }

    // Always keep a diverse pilot tail so an empty/new dataset can make
    // progress without a hand-written checklist.
    for (hunt, profile, purpose, notes) in pilot_candidates(&corpus, reduced) {
        add_job(
            &mut jobs, &mut seen,
            baseline,
            &corpus, &hunt, &profile, purpose, notes, None,
        );
    }

    jobs.truncate(limit.max(1));
    CalibrationPlan {
        schema: 1,
        source: "moth-hunt-pacing-planner".into(),
        generated_from: reduced.map(|r| format!(
            "reduced:{} maps:{} hunts:{}", r.game_version, r.maps.len(), r.hunts.len()
        )).unwrap_or_else(|| "target-corpus-pilot".into()),
        jobs,
    }
}

pub fn render(plan: &CalibrationPlan) -> String {
    let mut out = format!(
        "Hunt pacing calibration plan — {} job(s)\nSource: {}\n\n",
        plan.jobs.len(), plan.generated_from
    );
    for (index, job) in plan.jobs.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} — {} / {}\n   purpose: {}\n   borrowed map hint: {} (advisory only)\n   account profiles: {}\n   windows: {}/{}\n   {}\n",
            index + 1, job.id, job.hunt_slug, job.profile, job.purpose,
            job.borrowed_map_hint,
            if job.available_on_profiles.is_empty() { "unknown".into() }
            else { job.available_on_profiles.join(", ") },
            job.completed_windows, job.target_windows, job.notes
        ));
        for (key, value) in &job.env {
            out.push_str(&format!("   {key}={value}\n"));
        }
        out.push('\n');
    }
    out
}

pub fn write_json(path: &Path, plan: &CalibrationPlan) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(plan).map_err(|error| error.to_string())?;
    fs::write(path, bytes)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn add_job(
    jobs: &mut Vec<CalibrationJob>,
    seen: &mut BTreeSet<(String, String)>,
    reduced: &ReducedDataset,
    corpus: &Corpus,
    hunt: &str,
    profile: &str,
    purpose: &str,
    notes: &str,
    completed_override: Option<usize>,
) {
    let key = (hunt.to_owned(), profile.to_owned());
    if !seen.insert(key) { return; }
    let borrowed_map_hint = corpus.hunts.get(hunt).cloned().unwrap_or_else(|| "unknown".into());
    let completed_windows = completed_override
        .unwrap_or_else(|| completed_windows(reduced, hunt, profile));
    if completed_windows >= ENOUGH_WINDOWS { return; }
    let id = safe_id(&format!("{purpose}-{hunt}-{profile}"));
    let mut env = BTreeMap::new();
    env.insert("MOTH_HUNT_PACING_TRACE".into(), "1".into());
    env.insert("MOTH_HUNT_PACING_PROFILE".into(), profile.into());
    env.insert("MOTH_HUNT_PACING_RUN_ID".into(), id.clone());
    env.insert("MOTH_HUNT_PACING_EXPECT_HUNT".into(), hunt.into());
    jobs.push(CalibrationJob {
        id,
        purpose: purpose.into(),
        profile: profile.into(),
        hunt_slug: hunt.into(),
        borrowed_map_hint,
        target_windows: ENOUGH_WINDOWS,
        completed_windows,
        notes: notes.into(),
        available_on_profiles: available_profiles(reduced, hunt),
        env,
    });
}

fn completed_windows(reduced: &ReducedDataset, hunt: &str, profile: &str) -> usize {
    reduced.maps.values()
        .filter_map(|map| map.profiles.get(profile))
        .flat_map(|stats| stats.windows.iter())
        .filter(|window| window.hunt_slug.as_deref() == Some(hunt))
        .count()
}

fn pilot_candidates(
    corpus: &Corpus,
    reduced: Option<&ReducedDataset>,
) -> Vec<(String, String, &'static str, &'static str)> {
    let mut out = Vec::new();
    let mut used_maps = BTreeSet::new();

    if let Some((map, hunts)) = corpus.by_map.iter().find(|(_, hunts)|
        hunts.iter().filter(|hunt| hunt_is_available_opt(reduced, hunt)).count() >= 2)
    {
        for hunt in hunts.iter().filter(|hunt| hunt_is_available_opt(reduced, hunt)).take(2) {
            out.push((hunt.clone(), "st1".into(), "shared-map-pilot",
                "Shared-map identity/convergence pilot; repeat independent windows."));
        }
        if let Some(hunt) = hunts.iter().find(|hunt| hunt_is_available_opt(reduced, hunt)) {
            out.push((hunt.clone(), "aoe".into(), "shared-map-pilot",
                "AoE baseline for the shared-map pilot."));
        }
        used_maps.insert(map.clone());
    }

    for (map, hunts) in &corpus.by_map {
        if !map.starts_with("gerado_") || used_maps.contains(map) { continue; }
        let Some(hunt) = hunts.iter().find(|hunt| hunt_is_available_opt(reduced, hunt)) else { continue };
        out.push((hunt.clone(), "st1".into(), "generated-map-pilot",
            "Generated-map ST1 pilot; live campo.init remains authoritative."));
        out.push((hunt.clone(), "aoe".into(), "generated-map-pilot",
            "Generated-map AoE pilot; live campo.init remains authoritative."));
        used_maps.insert(map.clone());
        if used_maps.iter().filter(|m| m.starts_with("gerado_")).count() >= 2 { break; }
    }

    if let Some((map, hunts)) = corpus.by_map.iter().find(|(map, hunts)| {
        !used_maps.contains(*map) && hunts.iter().any(|hunt|
            is_outland_style(hunt) && hunt_is_available_opt(reduced, hunt))
    }) {
        if let Some(hunt) = hunts.iter().find(|hunt|
            is_outland_style(hunt) && hunt_is_available_opt(reduced, hunt)) {
            out.push((hunt.clone(), "st1".into(), "outland-pilot",
                "Outland-style ST1 pilot."));
            out.push((hunt.clone(), "aoe".into(), "outland-pilot",
                "Outland-style AoE pilot."));
            used_maps.insert(map.clone());
        }
    }

    for (map, hunts) in &corpus.by_map {
        if used_maps.contains(map) || map.starts_with("gerado_") || hunts.len() != 1 { continue; }
        let Some(hunt) = hunts.iter().find(|hunt| hunt_is_available_opt(reduced, hunt)) else { continue };
        if is_outland_style(hunt) { continue; }
        out.push((hunt.clone(), "st1".into(), "standard-map-pilot",
            "Ordinary unique-map ST1 control."));
        out.push((hunt.clone(), "aoe".into(), "standard-map-pilot",
            "Ordinary unique-map AoE control."));
        used_maps.insert(map.clone());
        if out.iter().filter(|(_, _, purpose, _)| *purpose == "standard-map-pilot").count() >= 4 {
            break;
        }
    }
    out
}

fn hunt_is_available(reduced: &ReducedDataset, hunt: &str) -> bool {
    if reduced.available_hunts_by_profile.is_empty() { return true; }
    reduced.available_hunts_by_profile.values()
        .any(|hunts| hunts.iter().any(|candidate| candidate == hunt))
}

fn hunt_is_available_opt(reduced: Option<&ReducedDataset>, hunt: &str) -> bool {
    reduced.map(|reduced| hunt_is_available(reduced, hunt)).unwrap_or(true)
}

fn available_profiles(reduced: &ReducedDataset, hunt: &str) -> Vec<String> {
    reduced.available_hunts_by_profile.iter()
        .filter(|(_, hunts)| hunts.iter().any(|candidate| candidate == hunt))
        .map(|(profile, _)| profile.clone())
        .collect()
}

fn is_outland_style(hunt: &str) -> bool {
    [
        "ancient_", "brave_", "furious_", "tribal_", "war_", "psy_",
        "magma_", "magnetic_", "taekwondo_", "toxic_", "roll_",
        "trickmaster_", "milch_",
    ].iter().any(|prefix| hunt.starts_with(prefix))
}

fn corpus() -> Corpus {
    let mut hunts = BTreeMap::<String, String>::new();
    let mut by_map = BTreeMap::<String, Vec<String>>::new();
    for line in BORROWED_TARGET.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.first().copied() != Some("hunt") || fields.len() < 3 { continue; }
        hunts.insert(fields[1].into(), fields[2].into());
        by_map.entry(fields[2].into()).or_default().push(fields[1].into());
    }
    for values in by_map.values_mut() { values.sort(); }
    Corpus { hunts, by_map }
}

fn safe_id(raw: &str) -> String {
    raw.chars().map(|ch| {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' }
    }).collect::<String>().trim_matches('-').chars().take(96).collect()
}

fn empty_reduced() -> ReducedDataset {
    ReducedDataset {
        schema: 1,
        source: "planner-empty".into(),
        generated_at_unix_ms: 0,
        game_version: "unknown".into(),
        config: Default::default(),
        trace_files: 0,
        maps: BTreeMap::new(),
        hunts: BTreeMap::new(),
        available_hunts_by_profile: BTreeMap::new(),
        unknown_hunts: Vec::new(),
        rejected_windows: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_catalog_filters_planned_hunts() {
        let corpus = corpus();
        let seed = pilot_candidates(&corpus, None);
        let allowed = seed.iter().take(3)
            .map(|(hunt, _, _, _)| hunt.clone()).collect::<BTreeSet<_>>();
        let mut reduced = empty_reduced();
        reduced.available_hunts_by_profile.insert(
            "calibration-account".into(), allowed.iter().cloned().collect());
        let plan = build(Some(&reduced), 32);
        assert!(plan.jobs.iter().all(|job| allowed.contains(&job.hunt_slug)));
        assert!(plan.jobs.iter().all(|job|
            job.available_on_profiles == vec!["calibration-account".to_owned()]));
    }

    #[test]
    fn pilot_has_shared_generated_outland_and_standard_work() {
        let plan = build(None, 32);
        let purposes = plan.jobs.iter().map(|job| job.purpose.as_str()).collect::<BTreeSet<_>>();
        assert!(purposes.contains("shared-map-pilot"));
        assert!(purposes.contains("generated-map-pilot"));
        assert!(purposes.contains("outland-pilot"));
        assert!(purposes.contains("standard-map-pilot"));
        assert!(plan.jobs.iter().all(|job|
            job.env.get("MOTH_HUNT_PACING_EXPECT_HUNT") == Some(&job.hunt_slug)));
        assert!(plan.jobs.iter().all(|job|
            job.env.keys().all(|key| !key.contains("MAP"))));
        assert!(plan.jobs.iter().all(|job| job.available_on_profiles.is_empty()));
    }
}
