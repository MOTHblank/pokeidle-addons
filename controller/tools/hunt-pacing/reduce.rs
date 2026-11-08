use crate::trace::{
    active_hp, active_pokemon_id, aoe_enabled, bicycle_factor, fingerprint, in_center,
    TraceRecord, TraceSession,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReduceConfig {
    pub warmup_kills: usize,
    pub min_measured_kills: usize,
    pub max_packet_gap_ms: u64,
    pub min_seconds_per_kill: f64,
    pub max_seconds_per_kill: f64,
    pub outlier_mad_multiplier: f64,
}

impl Default for ReduceConfig {
    fn default() -> Self {
        Self {
            warmup_kills: 5,
            min_measured_kills: 20,
            max_packet_gap_ms: 10_000,
            min_seconds_per_kill: 0.20,
            max_seconds_per_kill: 120.0,
            outlier_mad_multiplier: 6.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowSample {
    pub session_id: String,
    pub map_id: String,
    pub hunt_slug: Option<String>,
    pub profile: String,
    pub bicycle_factor: f64,
    pub scene_generation: u64,
    pub start_recv_monotonic_ms: u64,
    pub end_recv_monotonic_ms: u64,
    pub kills: u64,
    pub total_seconds: f64,
    pub seconds_per_kill: f64,
    pub kill_batch_sizes: Vec<u64>,
    pub max_attack_actions_between_kills: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileStats {
    pub samples: usize,
    pub kills: u64,
    pub total_seconds: f64,
    pub mean_seconds_per_kill: f64,
    pub median_seconds_per_kill: f64,
    pub p10: f64,
    pub p90: f64,
    pub mad: f64,
    pub rejected_outlier_samples: usize,
    pub windows: Vec<WindowSample>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapReduction {
    pub profiles: BTreeMap<String, ProfileStats>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReducedDataset {
    pub schema: u32,
    pub source: String,
    pub generated_at_unix_ms: u64,
    pub game_version: String,
    pub config: ReduceConfig,
    pub trace_files: usize,
    pub maps: BTreeMap<String, MapReduction>,
    pub hunts: BTreeMap<String, String>,
    pub unknown_hunts: Vec<String>,
    pub rejected_windows: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
struct Segment {
    session_id: String,
    map_id: String,
    hunt_slug: Option<String>,
    profile: String,
    bicycle_factor: f64,
    scene_generation: u64,
    fingerprint: String,
    connection_generation: u64,
    first_record_ms: u64,
    last_record_ms: u64,
    kill_times: Vec<u64>,
    kill_batch_sizes: Vec<u64>,
    attack_actions_since_kill: u32,
    max_attack_actions_between_kills: u32,
    invalid_reason: Option<String>,
}

pub fn reduce_sessions(sessions: &[TraceSession], config: ReduceConfig) -> ReducedDataset {
    let mut windows = Vec::new();
    let mut rejected = BTreeMap::<String, u64>::new();
    let mut observed_hunts = BTreeMap::<String, Option<String>>::new();
    let mut game_version = "unknown".to_owned();

    for session in sessions {
        if game_version == "unknown" {
            if let Some(version) = session.records.iter()
                .map(|record| record.game_version.trim())
                .find(|version| !version.is_empty() && *version != "unknown")
            {
                game_version = version.to_owned();
            }
        }
        reduce_session(
            session,
            &config,
            &mut windows,
            &mut rejected,
            &mut observed_hunts,
        );
    }

    let mut grouped = BTreeMap::<(String, String), Vec<WindowSample>>::new();
    for sample in windows {
        grouped.entry((sample.map_id.clone(), sample.profile.clone()))
            .or_default().push(sample);
    }

    let mut maps = BTreeMap::<String, MapReduction>::new();
    for ((map_id, profile), samples) in grouped {
        let stats = summarize(samples, &config);
        if stats.samples == 0 { continue; }
        maps.entry(map_id).or_default().profiles.insert(profile, stats);
    }

    let mut hunts = BTreeMap::new();
    let mut unknown_hunts = BTreeSet::new();
    for (slug, map_id) in observed_hunts {
        match map_id {
            Some(map_id) => { hunts.insert(slug, map_id); }
            None => { unknown_hunts.insert(slug); }
        }
    }

    ReducedDataset {
        schema: 1,
        source: "moth-live-observation".into(),
        generated_at_unix_ms: unix_ms(),
        game_version,
        config,
        trace_files: sessions.len(),
        maps,
        hunts,
        unknown_hunts: unknown_hunts.into_iter().collect(),
        rejected_windows: rejected,
    }
}

fn reduce_session(
    session: &TraceSession,
    config: &ReduceConfig,
    windows: &mut Vec<WindowSample>,
    rejected: &mut BTreeMap<String, u64>,
    observed_hunts: &mut BTreeMap<String, Option<String>>,
) {
    let mut segment: Option<Segment> = None;

    for record in &session.records {
        observe_hunt_mapping(record, observed_hunts);

        if record.event_type == "connection.closed" {
            finish_segment(segment.take(), config, windows, rejected);
            continue;
        }
        if record.event_type != "packet" {
            continue;
        }

        let Some(map_id) = record.map_id.as_deref().filter(|map_id| !map_id.is_empty()) else {
            finish_segment(segment.take(), config, windows, rejected);
            continue;
        };
        let Some(profile) = accepted_profile(record) else {
            finish_segment(segment.take(), config, windows, rejected);
            continue;
        };

        let current_fingerprint = fingerprint(record);
        let should_split = segment.as_ref().is_some_and(|current| {
            current.map_id != map_id
                || current.profile != profile
                || current.scene_generation != record.scene_generation
                || current.connection_generation != record.connection_generation
                || current.fingerprint != current_fingerprint
                || record.recv_monotonic_ms.saturating_sub(current.last_record_ms)
                    > config.max_packet_gap_ms
        });
        if should_split {
            finish_segment(segment.take(), config, windows, rejected);
        }

        if segment.is_none() {
            segment = Some(Segment {
                session_id: record.session_id.clone(),
                map_id: map_id.to_owned(),
                hunt_slug: record.hunt_slug.clone(),
                profile: profile.to_owned(),
                bicycle_factor: bicycle_factor(record),
                scene_generation: record.scene_generation,
                fingerprint: current_fingerprint,
                connection_generation: record.connection_generation,
                first_record_ms: record.recv_monotonic_ms,
                last_record_ms: record.recv_monotonic_ms,
                kill_times: Vec::new(),
                kill_batch_sizes: Vec::new(),
                attack_actions_since_kill: 0,
                max_attack_actions_between_kills: 0,
                invalid_reason: None,
            });
        }

        let current = segment.as_mut().unwrap();
        current.last_record_ms = record.recv_monotonic_ms;

        if let Some(reason) = hard_interruption(record) {
            current.invalid_reason = Some(reason);
            finish_segment(segment.take(), config, windows, rejected);
            continue;
        }

        if record.packet_type.as_deref() == Some("batalha") {
            current.attack_actions_since_kill = current.attack_actions_since_kill
                .saturating_add(explicit_attack_actions(record));

            let batch = record.kills.len() as u64;
            if batch > 0 {
                current.max_attack_actions_between_kills =
                    current.max_attack_actions_between_kills
                        .max(current.attack_actions_since_kill);
                if current.profile == "st1" && batch != 1 {
                    current.invalid_reason = Some("st1-multi-kill-batch".into());
                }
                if matches!(current.profile.as_str(), "st1" | "aoe")
                    && current.attack_actions_since_kill > 1
                {
                    current.invalid_reason = Some("multiple-explicit-attacks-per-kill-boundary".into());
                }

                current.kill_batch_sizes.push(batch);
                for kill in &record.kills {
                    let at = kill.get("kill_recv_monotonic_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or(record.recv_monotonic_ms);
                    current.kill_times.push(at);
                }
                current.attack_actions_since_kill = 0;
            }
        }
    }

    finish_segment(segment.take(), config, windows, rejected);
}

fn accepted_profile(record: &TraceRecord) -> Option<&'static str> {
    if is_special_scene(record) || in_center(record) || active_pokemon_id(record).is_none() {
        return None;
    }
    let bike = bicycle_factor(record);
    let aoe = aoe_enabled(record);
    match record.declared_profile.as_str() {
        "st1" if bike <= 1.000_001 && !aoe => Some("st1"),
        "aoe" if bike <= 1.000_001 && aoe => Some("aoe"),
        "bike-validation" => Some("bike-validation"),
        "deadtime-validation" => Some("deadtime-validation"),
        _ => None,
    }
}

fn hard_interruption(record: &TraceRecord) -> Option<String> {
    if in_center(record) {
        return Some("center".into());
    }
    if active_hp(record) == Some(0) {
        return Some("active-pokemon-death".into());
    }
    if record.packet_type.as_deref() == Some("batalha") {
        for event in record.packet.get("ev").and_then(Value::as_array)
            .into_iter().flatten()
        {
            let kind = event.get("k").and_then(Value::as_str)
                .unwrap_or_default().to_ascii_lowercase();
            if kind.contains("revive") || kind.contains("reviver")
                || kind.contains("potion") || kind.contains("pocao")
                || kind.contains("poção")
            {
                return Some("healing-interruption".into());
            }
            if kind.starts_with("boss") || kind.contains("safari")
                || kind.contains("mist") || kind.contains("raid")
            {
                return Some("special-combat".into());
            }
            if kind == "morte" && event.get("quem").and_then(Value::as_str)
                .is_some_and(|who| who != "selvagem")
            {
                return Some("non-wild-death".into());
            }
        }
    }
    None
}

fn is_special_scene(record: &TraceRecord) -> bool {
    let text = [
        record.hunt_slug.as_deref().unwrap_or_default(),
        record.field_slug.as_deref().unwrap_or_default(),
        record.map_id.as_deref().unwrap_or_default(),
    ].join(" ").to_ascii_lowercase();
    ["boss", "safari", "mistico", "místico", "mystic", "raid", "pvp", "arena"]
        .iter().any(|needle| text.contains(needle))
}

fn explicit_attack_actions(record: &TraceRecord) -> u32 {
    record.packet.get("ev").and_then(Value::as_array)
        .into_iter().flatten()
        .filter(|event| {
            let kind = event.get("k").and_then(Value::as_str)
                .unwrap_or_default().to_ascii_lowercase();
            matches!(kind.as_str(), "ataque" | "attack" | "golpe" | "hit")
                || event.get("ataque").is_some_and(|value| !value.is_null())
                || event.get("attack").is_some_and(|value| !value.is_null())
                || event.get("golpe").is_some_and(|value| !value.is_null())
                || event.get("move").is_some_and(|value| !value.is_null())
        })
        .count().min(u32::MAX as usize) as u32
}

fn finish_segment(
    segment: Option<Segment>,
    config: &ReduceConfig,
    windows: &mut Vec<WindowSample>,
    rejected: &mut BTreeMap<String, u64>,
) {
    let Some(segment) = segment else { return };
    if let Some(reason) = segment.invalid_reason {
        reject(rejected, &reason);
        return;
    }

    let needed = config.warmup_kills
        .saturating_add(config.min_measured_kills)
        .saturating_add(1);
    if segment.kill_times.len() < needed {
        reject(rejected, "insufficient-kills");
        return;
    }

    let measured = &segment.kill_times[config.warmup_kills..];
    let Some((&start, &end)) = measured.first().zip(measured.last()) else {
        reject(rejected, "missing-boundary");
        return;
    };
    let kills = measured.len().saturating_sub(1) as u64;
    if kills == 0 || end <= start {
        reject(rejected, "non-positive-window");
        return;
    }
    let seconds = (end - start) as f64 / 1000.0;
    let seconds_per_kill = seconds / kills as f64;
    if !seconds_per_kill.is_finite()
        || seconds_per_kill < config.min_seconds_per_kill
        || seconds_per_kill > config.max_seconds_per_kill
    {
        reject(rejected, "implausible-seconds-per-kill");
        return;
    }

    let warmup_batches = batches_covering_kills(&segment.kill_batch_sizes, config.warmup_kills);
    let kill_batch_sizes = segment.kill_batch_sizes
        .into_iter().skip(warmup_batches).collect();

    windows.push(WindowSample {
        session_id: segment.session_id,
        map_id: segment.map_id,
        hunt_slug: segment.hunt_slug,
        profile: segment.profile,
        bicycle_factor: segment.bicycle_factor,
        scene_generation: segment.scene_generation,
        start_recv_monotonic_ms: start,
        end_recv_monotonic_ms: end,
        kills,
        total_seconds: seconds,
        seconds_per_kill,
        kill_batch_sizes,
        max_attack_actions_between_kills: segment.max_attack_actions_between_kills,
    });
}

fn batches_covering_kills(batches: &[u64], kills: usize) -> usize {
    if kills == 0 { return 0; }
    let mut total = 0usize;
    for (index, batch) in batches.iter().enumerate() {
        total = total.saturating_add(*batch as usize);
        if total >= kills { return index + 1; }
    }
    batches.len()
}

fn summarize(mut samples: Vec<WindowSample>, config: &ReduceConfig) -> ProfileStats {
    samples.retain(|sample| sample.seconds_per_kill.is_finite());
    let before = samples.len();

    if samples.len() >= 5 {
        let values = samples.iter().map(|sample| sample.seconds_per_kill)
            .collect::<Vec<_>>();
        let median = quantile(values.clone(), 0.5);
        let deviations = values.iter().map(|value| (value - median).abs())
            .collect::<Vec<_>>();
        let mad = quantile(deviations, 0.5);
        if mad > f64::EPSILON {
            let limit = config.outlier_mad_multiplier * mad;
            samples.retain(|sample| (sample.seconds_per_kill - median).abs() <= limit);
        }
    }

    let rejected_outlier_samples = before.saturating_sub(samples.len());
    if samples.is_empty() {
        return ProfileStats {
            samples: 0, kills: 0, total_seconds: 0.0,
            mean_seconds_per_kill: 0.0, median_seconds_per_kill: 0.0,
            p10: 0.0, p90: 0.0, mad: 0.0, rejected_outlier_samples,
            windows: Vec::new(),
        };
    }

    let kills = samples.iter().map(|sample| sample.kills).sum::<u64>();
    let total_seconds = samples.iter().map(|sample| sample.total_seconds).sum::<f64>();
    let values = samples.iter().map(|sample| sample.seconds_per_kill)
        .collect::<Vec<_>>();
    let median = quantile(values.clone(), 0.5);
    let mad = quantile(
        values.iter().map(|value| (value - median).abs()).collect(),
        0.5,
    );

    ProfileStats {
        samples: samples.len(),
        kills,
        total_seconds,
        mean_seconds_per_kill: if kills > 0 { total_seconds / kills as f64 } else { 0.0 },
        median_seconds_per_kill: median,
        p10: quantile(values.clone(), 0.10),
        p90: quantile(values, 0.90),
        mad,
        rejected_outlier_samples,
        windows: samples,
    }
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

fn observe_hunt_mapping(
    record: &TraceRecord,
    observed: &mut BTreeMap<String, Option<String>>,
) {
    let Some(slug) = record.hunt_slug.as_deref().filter(|slug| !slug.is_empty()) else {
        return;
    };
    let map = record.map_id.as_deref().filter(|map| !map.is_empty());
    match observed.get_mut(slug) {
        None => {
            observed.insert(slug.to_owned(), map.map(str::to_owned));
        }
        Some(current) => match (current.as_deref(), map) {
            (Some(old), Some(new)) if old != new => *current = None,
            (None, _) => {}
            (Some(_), None) => {}
            (None, Some(_)) => {}
            _ => {}
        },
    }
}

fn reject(rejected: &mut BTreeMap<String, u64>, reason: &str) {
    *rejected.entry(reason.to_owned()).or_default() += 1;
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_and_mad_are_deterministic() {
        let values = vec![3.0, 3.2, 3.4, 3.6, 3.8];
        assert_eq!(quantile(values.clone(), 0.5), 3.4);
        assert!((quantile(values.clone(), 0.1) - 3.08).abs() < 1e-9);
        let median = quantile(values.clone(), 0.5);
        assert!((quantile(values.iter().map(|v| (v - median).abs()).collect(), 0.5) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn warmup_batch_count_handles_aoe_batches() {
        assert_eq!(batches_covering_kills(&[2, 3, 1], 0), 0);
        assert_eq!(batches_covering_kills(&[2, 3, 1], 1), 1);
        assert_eq!(batches_covering_kills(&[2, 3, 1], 5), 2);
    }
}
