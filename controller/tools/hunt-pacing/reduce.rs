use crate::trace::{
    active_hp, active_pokemon_id, aoe_enabled, bicycle_factor, fingerprint, in_center,
    TraceRecord, TraceSession,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

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
pub struct KillBoundary {
    pub kill_recv_monotonic_ms: u64,
    pub previous_kill_recv_monotonic_ms: Option<u64>,
    pub kill_server_ts: Option<u64>,
    pub previous_kill_server_ts: Option<u64>,
    pub target_slot: Value,
    pub species: Value,
    pub attack: Value,
    pub next_target_recv_monotonic_ms: Option<u64>,
    pub next_target: Value,
    pub first_movement_recv_monotonic_ms: Option<u64>,
    pub first_movement: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowSample {
    pub session_id: String,
    pub map_id: String,
    pub hunt_slug: Option<String>,
    pub profile: String,
    pub bicycle_factor: f64,
    pub scene_generation: u64,
    pub measurement_clock: String,
    pub start_measurement_ms: u64,
    pub end_measurement_ms: u64,
    pub start_recv_monotonic_ms: u64,
    pub end_recv_monotonic_ms: u64,
    pub declared_hits: Option<u32>,
    pub validation_combat_seconds: Option<f64>,
    pub kills: u64,
    pub total_seconds: f64,
    pub seconds_per_kill: f64,
    pub kill_batch_sizes: Vec<u64>,
    pub kill_boundaries: Vec<KillBoundary>,
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
    last_record_ms: u64,
    kill_times: Vec<u64>,
    kill_server_times: Vec<Option<u64>>,
    kill_batch_sizes: Vec<u64>,
    kill_boundaries: Vec<KillBoundary>,
    pending_boundary_start: Option<usize>,
    attack_actions_since_kill: u32,
    max_attack_actions_between_kills: u32,
    declared_hits: Option<u32>,
    validation_combat_seconds: Option<f64>,
    last_active_hp: Option<u64>,
    invalid_reason: Option<String>,
}

pub fn reduce_sessions(sessions: &[TraceSession], config: ReduceConfig) -> ReducedDataset {
    let mut windows = Vec::new();
    let mut rejected = BTreeMap::<String, u64>::new();
    let mut observed_hunts = BTreeMap::<String, Option<String>>::new();
    let mut game_version = "unknown".to_owned();
    let generated_at_unix_ms = sessions.iter()
        .flat_map(|session| session.records.iter())
        .map(|record| record.recv_unix_ms)
        .max().unwrap_or(0);

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
        generated_at_unix_ms,
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
                || current.hunt_slug != record.hunt_slug
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
                profile: profile.clone(),
                bicycle_factor: bicycle_factor(record),
                scene_generation: record.scene_generation,
                fingerprint: current_fingerprint,
                connection_generation: record.connection_generation,
                last_record_ms: record.recv_monotonic_ms,
                kill_times: Vec::new(),
                kill_server_times: Vec::new(),
                kill_batch_sizes: Vec::new(),
                kill_boundaries: Vec::new(),
                pending_boundary_start: None,
                attack_actions_since_kill: 0,
                max_attack_actions_between_kills: 0,
                declared_hits: validation_hits(record),
                validation_combat_seconds: validation_combat_seconds(record),
                last_active_hp: active_hp(record),
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
        if let Some(hp) = active_hp(record) {
            if current.last_active_hp.is_some_and(|previous| hp > previous) {
                current.invalid_reason = Some("active-hp-increase".into());
                finish_segment(segment.take(), config, windows, rejected);
                continue;
            }
            current.last_active_hp = Some(hp);
        }

        if matches!(record.packet_type.as_deref(), Some("campo") | Some("campo.init")) {
            attach_post_kill_field(current, record);
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
                let boundary_start = current.kill_boundaries.len();
                for kill in &record.kills {
                    let at = kill.get("kill_recv_monotonic_ms")
                        .and_then(Value::as_u64)
                        .unwrap_or(record.recv_monotonic_ms);
                    current.kill_times.push(at);
                    let kill_server_ts = kill.get("kill_server_ts").and_then(Value::as_u64);
                    current.kill_server_times.push(kill_server_ts);
                    current.kill_boundaries.push(KillBoundary {
                        kill_recv_monotonic_ms: at,
                        previous_kill_recv_monotonic_ms: kill
                            .get("previous_kill_recv_monotonic_ms")
                            .and_then(Value::as_u64),
                        kill_server_ts,
                        previous_kill_server_ts: kill
                            .get("previous_kill_server_ts")
                            .and_then(Value::as_u64),
                        target_slot: kill.get("slot").cloned().unwrap_or(Value::Null),
                        species: kill.get("species").cloned().unwrap_or(Value::Null),
                        attack: kill.get("attack").cloned().unwrap_or(Value::Null),
                        next_target_recv_monotonic_ms: None,
                        next_target: Value::Null,
                        first_movement_recv_monotonic_ms: None,
                        first_movement: Value::Null,
                    });
                }
                current.pending_boundary_start = Some(boundary_start);
                current.attack_actions_since_kill = 0;
            }
        }
    }

    finish_segment(segment.take(), config, windows, rejected);
}

fn attach_post_kill_field(segment: &mut Segment, record: &TraceRecord) {
    let Some(start) = segment.pending_boundary_start else { return };
    if start >= segment.kill_boundaries.len() {
        segment.pending_boundary_start = None;
        return;
    }

    let explicit_target = record.packet.get("alvo")
        .filter(|target| !target.is_null()).cloned();
    let movement = first_field_movement(record);

    for boundary in &mut segment.kill_boundaries[start..] {
        if boundary.next_target_recv_monotonic_ms.is_none() {
            if let Some(target) = explicit_target.as_ref() {
                boundary.next_target_recv_monotonic_ms = Some(record.recv_monotonic_ms);
                boundary.next_target = target.clone();
            }
        }
        if boundary.first_movement_recv_monotonic_ms.is_none() {
            if let Some(movement) = movement.as_ref() {
                boundary.first_movement_recv_monotonic_ms = Some(record.recv_monotonic_ms);
                boundary.first_movement = movement.clone();
            }
        }
    }

    if segment.kill_boundaries[start..].iter().all(|boundary|
        boundary.next_target_recv_monotonic_ms.is_some()
            && boundary.first_movement_recv_monotonic_ms.is_some())
    {
        segment.pending_boundary_start = None;
    }
}

fn first_field_movement(record: &TraceRecord) -> Option<Value> {
    let field = &record.field_observation;
    if let Some(hero) = field.get("hero_movement")
        .filter(|value| value.get("transition").and_then(Value::as_bool) == Some(true))
    {
        return Some(hero.clone());
    }
    field.get("mob_updates").and_then(Value::as_array)
        .into_iter().flatten()
        .filter_map(|mob| mob.get("movement"))
        .find(|movement| movement.get("transition").and_then(Value::as_bool) == Some(true))
        .cloned()
}

fn accepted_profile(record: &TraceRecord) -> Option<String> {
    if is_special_scene(record) || in_center(record) || active_pokemon_id(record).is_none() {
        return None;
    }
    let bike = bicycle_factor(record);
    let aoe = aoe_enabled(record);
    match record.declared_profile.as_str() {
        "st1" if bike <= 1.000_001 && !aoe => Some("st1".into()),
        "aoe" if bike <= 1.000_001 && aoe => Some("aoe".into()),
        "bike-validation" if !aoe => {
            let rarity = record.calibration.get("bicycle")
                .and_then(|bike| bike.get("rarity"))
                .and_then(Value::as_str)
                .unwrap_or("foot");
            Some(format!("bike-validation:{rarity}:{bike:.3}"))
        }
        "deadtime-validation" if bike <= 1.000_001 && !aoe => {
            let hits = validation_hits(record)
                .map(|hits| hits.to_string()).unwrap_or_else(|| "unknown".into());
            let combat = validation_combat_seconds(record)
                .map(|seconds| format!("{seconds:.3}"))
                .unwrap_or_else(|| "unknown".into());
            Some(format!("deadtime-validation:hits={hits}:combat={combat}"))
        }
        _ => None,
    }
}

fn validation_hits(record: &TraceRecord) -> Option<u32> {
    record.calibration.get("validation")
        .and_then(|validation| validation.get("declared_hits"))
        .and_then(Value::as_u64)
        .and_then(|hits| u32::try_from(hits).ok())
        .filter(|hits| *hits > 0)
}

fn validation_combat_seconds(record: &TraceRecord) -> Option<f64> {
    record.calibration.get("validation")
        .and_then(|validation| validation.get("combat_seconds"))
        .and_then(Value::as_f64)
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
}

fn hard_interruption(record: &TraceRecord) -> Option<String> {
    if record.packet.get("manual").and_then(Value::as_bool) == Some(true)
        || record.packet.get("movimentoManual").and_then(Value::as_bool) == Some(true)
        || record.packet.get("manualMovement").and_then(Value::as_bool) == Some(true)
    {
        return Some("manual-movement".into());
    }
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
    // Count only events that explicitly represent an attack action. Death
    // receipts may repeat attack metadata once per AoE victim and therefore
    // must never be interpreted as multiple player attacks.
    record.packet.get("ev").and_then(Value::as_array)
        .into_iter().flatten()
        .filter(|event| {
            let kind = event.get("k").and_then(Value::as_str)
                .unwrap_or_default().to_ascii_lowercase();
            matches!(kind.as_str(), "ataque" | "attack")
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

    // Warm-up always ends on a complete server kill batch. This is identical
    // to kill-count warm-up for ST1 (batch size 1) and avoids starting an AOE
    // measurement halfway through one attack.
    let warmup_batches = batches_covering_kills(
        &segment.kill_batch_sizes, config.warmup_kills);
    let aligned_warmup_kills = segment.kill_batch_sizes.iter()
        .take(warmup_batches).map(|kills| *kills as usize).sum::<usize>();
    let measured_batches = &segment.kill_batch_sizes[warmup_batches..];
    if measured_batches.len() < 2 {
        reject(rejected, "insufficient-kill-batches");
        return;
    }

    // The first measured batch is the sequence boundary. Count the kills that
    // arrive at subsequent batch boundaries. For ST1 this is exactly the
    // ordinary (last kill - first kill)/(kills - 1) estimator. For AOE it
    // avoids crediting the kills that occurred at the zero-time start boundary.
    let kills = measured_batches.iter().skip(1).copied().sum::<u64>();
    if kills < config.min_measured_kills as u64 {
        reject(rejected, "insufficient-kills");
        return;
    }
    let Some(&start_recv) = segment.kill_times.get(aligned_warmup_kills) else {
        reject(rejected, "missing-boundary");
        return;
    };
    let Some(&end_recv) = segment.kill_times.last() else {
        reject(rejected, "missing-boundary");
        return;
    };
    if end_recv <= start_recv {
        reject(rejected, "non-positive-window");
        return;
    }

    let recv_delta = end_recv - start_recv;
    let server_start = segment.kill_server_times.get(aligned_warmup_kills)
        .copied().flatten();
    let server_end = segment.kill_server_times.last().copied().flatten();
    let use_server_clock = server_start.zip(server_end).is_some_and(|(start, end)| {
        if end <= start || recv_delta == 0 { return false; }
        let ratio = (end - start) as f64 / recv_delta as f64;
        (0.5..=2.0).contains(&ratio)
    });
    let (measurement_clock, start_measurement, end_measurement) =
        if use_server_clock {
            ("server", server_start.unwrap(), server_end.unwrap())
        } else {
            ("receipt-monotonic", start_recv, end_recv)
        };
    let seconds = (end_measurement - start_measurement) as f64 / 1000.0;
    let seconds_per_kill = seconds / kills as f64;
    if !seconds_per_kill.is_finite()
        || seconds_per_kill < config.min_seconds_per_kill
        || seconds_per_kill > config.max_seconds_per_kill
    {
        reject(rejected, "implausible-seconds-per-kill");
        return;
    }

    let kill_batch_sizes = segment.kill_batch_sizes
        .into_iter().skip(warmup_batches).collect();
    let kill_boundaries = segment.kill_boundaries
        .into_iter().skip(aligned_warmup_kills).collect();

    windows.push(WindowSample {
        session_id: segment.session_id,
        map_id: segment.map_id,
        hunt_slug: segment.hunt_slug,
        profile: segment.profile,
        bicycle_factor: segment.bicycle_factor,
        scene_generation: segment.scene_generation,
        measurement_clock: measurement_clock.into(),
        start_measurement_ms: start_measurement,
        end_measurement_ms: end_measurement,
        start_recv_monotonic_ms: start_recv,
        end_recv_monotonic_ms: end_recv,
        declared_hits: segment.declared_hits,
        validation_combat_seconds: segment.validation_combat_seconds,
        kills,
        total_seconds: seconds,
        seconds_per_kill,
        kill_batch_sizes,
        kill_boundaries,
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
    // The collector persists the current map ID on later packets for grouping,
    // but hunt→map identity is learned only from the authoritative scene init.
    if record.packet_type.as_deref() != Some("campo.init") {
        return;
    }
    let Some(slug) = record.hunt_slug.as_deref().filter(|slug| !slug.is_empty()) else {
        return;
    };
    let Some(map) = record.map_id.as_deref().filter(|map| !map.is_empty()) else {
        return;
    };
    match observed.get_mut(slug) {
        None => {
            observed.insert(slug.to_owned(), Some(map.to_owned()));
        }
        Some(current) => {
            if current.as_deref().is_some_and(|previous| previous != map) {
                // A conflict is retained as unknown rather than choosing
                // whichever observation happened to arrive first.
                *current = None;
            }
        }
    }
}

fn reject(rejected: &mut BTreeMap<String, u64>, reason: &str) {
    *rejected.entry(reason.to_owned()).or_default() += 1;
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
