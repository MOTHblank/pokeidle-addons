//! Opt-in raw field/combat trace for empirical Hunt Atlas pacing calibration.
//!
//! This module never opens another socket and never polls the game. The native
//! game worker hands it the same decoded frames already consumed by HuntIntel.
//! Unless MOTH_HUNT_PACING_TRACE is enabled (or an explicit trace directory is
//! configured), the collector owns no file, allocates no trace records and its
//! hot-path cost is one Option check.

use crate::config::GameProfile;
use crate::native_hunts::HuntView;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct HuntPacingCollector {
    enabled: Option<EnabledCollector>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct TraceMove {
    move_type: String,
    special: bool,
    power: f64,
    cooldown_ms: f64,
}

struct EnabledCollector {
    writer: BufWriter<File>,
    path: PathBuf,
    session_id: String,
    profile: String,
    declared_profile: String,
    game_version: String,
    origin: Instant,
    connection_generation: u64,
    scene_generation: u64,
    field_slug: Option<String>,
    map_id: Option<String>,
    mob_slots: BTreeSet<String>,
    removed_slots: BTreeSet<String>,
    mob_dead: BTreeMap<String, bool>,
    packet_sequence: u64,
    last_kill_monotonic_ms: Option<u64>,
    writes_since_flush: u32,
    failed: bool,
}

impl HuntPacingCollector {
    pub fn new(profile: GameProfile) -> Self {
        let enabled = std::env::var_os("MOTH_HUNT_PACING_TRACE_DIR").is_some()
            || std::env::var("MOTH_HUNT_PACING_TRACE")
                .ok()
                .is_some_and(|raw| matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                ));
        if !enabled {
            return Self { enabled: None };
        }

        let root = std::env::var_os("MOTH_HUNT_PACING_TRACE_DIR")
            .map(PathBuf::from)
            .or_else(|| crate::config::data_root().ok()
                .map(|root| root.join("diagnostics").join("hunt-pacing").join("raw")))
            .unwrap_or_else(|| PathBuf::from("hunt-pacing").join("raw"));
        if let Err(error) = fs::create_dir_all(&root) {
            crate::logging::warn(&format!(
                "Hunt pacing trace disabled: could not create {}: {error}",
                root.display()
            ));
            return Self { enabled: None };
        }

        let unix_ms = unix_ms();
        let safe_profile = safe_component(profile.name());
        let session_id = format!("{unix_ms}-{}-{}", std::process::id(), safe_profile);
        let path = root.join(format!("{session_id}.jsonl"));
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) => {
                crate::logging::warn(&format!(
                    "Hunt pacing trace disabled: could not create {}: {error}",
                    path.display()
                ));
                return Self { enabled: None };
            }
        };

        let declared_profile = std::env::var("MOTH_HUNT_PACING_PROFILE")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| matches!(
                value.as_str(),
                "st1" | "aoe" | "bike-validation" | "deadtime-validation" | "diagnostic"
            ))
            .unwrap_or_else(|| "diagnostic".into());
        let game_version = std::env::var("MOTH_HUNT_PACING_GAME_VERSION")
            .ok().map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".into());

        crate::logging::info(&format!(
            "Hunt pacing trace enabled for {} ({declared_profile}) -> {}",
            profile.label(), path.display()
        ));
        Self {
            enabled: Some(EnabledCollector {
                writer: BufWriter::new(file),
                path,
                session_id,
                profile: profile.name().to_owned(),
                declared_profile,
                game_version,
                origin: Instant::now(),
                connection_generation: 0,
                scene_generation: 0,
                field_slug: None,
                map_id: None,
                mob_slots: BTreeSet::new(),
                removed_slots: BTreeSet::new(),
                mob_dead: BTreeMap::new(),
                packet_sequence: 0,
                last_kill_monotonic_ms: None,
                writes_since_flush: 0,
                failed: false,
            }),
        }
    }

    pub fn connection_open(&mut self, view: &HuntView, received_at: Instant) {
        let Some(inner) = self.enabled.as_mut() else { return };
        inner.connection_generation = inner.connection_generation.wrapping_add(1);
        inner.scene_generation = 0;
        inner.field_slug = None;
        inner.map_id = None;
        inner.mob_slots.clear();
        inner.removed_slots.clear();
        inner.mob_dead.clear();
        inner.last_kill_monotonic_ms = None;
        let record = inner.base_record("connection.open", received_at, view, None);
        inner.write_record(record, true);
    }

    pub fn connection_closed(&mut self, reason: &str, view: &HuntView, received_at: Instant) {
        let Some(inner) = self.enabled.as_mut() else { return };
        let mut record = inner.base_record("connection.closed", received_at, view, None);
        record["reason"] = Value::String(reason.to_owned());
        inner.write_record(record, true);
        inner.last_kill_monotonic_ms = None;
    }

    pub fn observe(&mut self, frame: &Value, view: &HuntView, received_at: Instant) {
        let Some(inner) = self.enabled.as_mut() else { return };
        let Some(kind) = frame.get("t").and_then(Value::as_str) else { return };
        if !matches!(kind, "campo.init" | "campo" | "batalha" | "estado") {
            return;
        }

        if kind == "campo.init" {
            inner.scene_generation = inner.scene_generation.wrapping_add(1);
            inner.field_slug = frame.get("slug").and_then(Value::as_str)
                .map(str::to_owned);
            inner.map_id = frame.get("mapa").and_then(Value::as_str)
                .or_else(|| frame.get("slug").and_then(Value::as_str))
                .map(str::to_owned);
            inner.mob_slots.clear();
            inner.removed_slots.clear();
            inner.mob_dead.clear();
            inner.last_kill_monotonic_ms = None;
        }

        inner.packet_sequence = inner.packet_sequence.wrapping_add(1);
        let server_ts = number_u64(frame.get("ts"));
        let mut record = inner.base_record("packet", received_at, view, server_ts);
        record["packet_sequence"] = json!(inner.packet_sequence);
        record["packet_type"] = Value::String(kind.to_owned());
        record["packet"] = frame.clone();

        if kind == "campo.init" {
            record["scene"] = json!({
                "hunt_slug": view.hunt_slug,
                "field_slug": inner.field_slug,
                "map_id": inner.map_id,
                "box": frame.get("box").cloned().unwrap_or(Value::Null),
                "groundZ": frame.get("groundZ").cloned().unwrap_or(Value::Null),
                "entered_at_server": server_ts,
            });
        }
        if matches!(kind, "campo.init" | "campo") {
            record["field_observation"] = inner.field_observation(frame);
        }
        if kind == "batalha" {
            record["kills"] = inner.kill_observations(frame, received_at);
        }

        let important = kind == "campo.init" || kind == "batalha";
        inner.write_record(record, important);
    }
}

impl EnabledCollector {
    fn base_record(
        &self,
        event_type: &str,
        received_at: Instant,
        view: &HuntView,
        server_ts: Option<u64>,
    ) -> Value {
        json!({
            "schema": 1,
            "event_type": event_type,
            "session_id": self.session_id,
            "profile": self.profile,
            "declared_profile": self.declared_profile,
            "game_version": self.game_version,
            "recv_unix_ms": unix_ms(),
            "recv_monotonic_ms": received_at
                .saturating_duration_since(self.origin).as_millis() as u64,
            "server_ts": server_ts,
            "connection_generation": self.connection_generation,
            "scene_generation": self.scene_generation,
            "hunt_slug": view.hunt_slug,
            "field_slug": self.field_slug,
            "map_id": self.map_id,
            "calibration": calibration_snapshot(view),
        })
    }

    fn field_observation(&mut self, frame: &Value) -> Value {
        let hero = frame.get("heroi")
            .and_then(|raw| movement("hero", None, raw));
        let mut mobs = Vec::new();
        for raw in frame.get("mobs").and_then(Value::as_array)
            .into_iter().flatten()
        {
            let Some(slot_value) = raw.get("s") else { continue };
            let slot_key = slot_key(slot_value);
            let existed = self.mob_slots.contains(&slot_key);
            let replenished = self.removed_slots.remove(&slot_key);
            self.mob_slots.insert(slot_key.clone());

            let now_dead = raw.get("x").and_then(Value::as_bool)
                .unwrap_or_else(|| self.mob_dead.get(&slot_key).copied().unwrap_or(false));
            let was_dead = self.mob_dead.insert(slot_key.clone(), now_dead).unwrap_or(false);
            let lifecycle = if !existed {
                if replenished { "respawn" } else { "spawn" }
            } else if now_dead && !was_dead {
                "death"
            } else {
                "update"
            };
            mobs.push(json!({
                "slot": slot_value,
                "lifecycle": lifecycle,
                "movement": movement("mob", Some(slot_value.clone()), raw),
                "dead": raw.get("x").cloned().unwrap_or(Value::Null),
                "hp": raw.get("hp").cloned().unwrap_or(Value::Null),
                "max_hp": raw.get("mhp").cloned().unwrap_or(Value::Null),
                "species": raw.get("speciesId").or_else(|| raw.get("pokeId"))
                    .cloned().unwrap_or(Value::Null),
                "name": raw.get("n").or_else(|| raw.get("nome"))
                    .cloned().unwrap_or(Value::Null),
                "corpse_remaining_ms": raw.get("xr").cloned().unwrap_or(Value::Null),
            }));
        }

        let mut removals = Vec::new();
        for slot in frame.get("fora").and_then(Value::as_array)
            .into_iter().flatten()
        {
            let key = slot_key(slot);
            self.mob_slots.remove(&key);
            self.mob_dead.remove(&key);
            self.removed_slots.insert(key);
            removals.push(slot.clone());
        }

        json!({
            "hero_movement": hero,
            "mob_updates": mobs,
            "removals": removals,
            "target": frame.get("alvo").cloned().unwrap_or(Value::Null),
        })
    }

    fn kill_observations(&mut self, frame: &Value, received_at: Instant) -> Value {
        let now = received_at.saturating_duration_since(self.origin).as_millis() as u64;
        let mut kills = Vec::new();
        for event in frame.get("ev").and_then(Value::as_array)
            .into_iter().flatten()
        {
            if event.get("k").and_then(Value::as_str) != Some("morte")
                || event.get("quem").and_then(Value::as_str) != Some("selvagem")
            {
                continue;
            }
            let previous = self.last_kill_monotonic_ms.replace(now);
            kills.push(json!({
                "kill_recv_monotonic_ms": now,
                "previous_kill_recv_monotonic_ms": previous,
                "slot": event.get("slot").cloned().unwrap_or(Value::Null),
                "species": event.get("speciesId").or_else(|| event.get("pokeId"))
                    .cloned().unwrap_or(Value::Null),
                "name": event.get("nome").or_else(|| event.get("n"))
                    .cloned().unwrap_or(Value::Null),
                "attack": event.get("ataque").or_else(|| event.get("attack"))
                    .or_else(|| event.get("golpe")).or_else(|| event.get("move"))
                    .cloned().unwrap_or(Value::Null),
                "event": event,
            }));
        }
        Value::Array(kills)
    }

    fn write_record(&mut self, record: Value, flush: bool) {
        if self.failed { return; }
        let result = serde_json::to_writer(&mut self.writer, &record)
            .and_then(|_| self.writer.write_all(b"\n").map_err(serde_json::Error::io));
        if let Err(error) = result {
            self.failed = true;
            crate::logging::warn(&format!(
                "Hunt pacing trace stopped writing {}: {error}", self.path.display()
            ));
            return;
        }
        self.writes_since_flush = self.writes_since_flush.saturating_add(1);
        if flush || self.writes_since_flush >= 64 {
            if let Err(error) = self.writer.flush() {
                self.failed = true;
                crate::logging::warn(&format!(
                    "Hunt pacing trace could not flush {}: {error}", self.path.display()
                ));
            }
            self.writes_since_flush = 0;
        }
    }
}

impl Drop for EnabledCollector {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}

fn movement(entity: &str, slot: Option<Value>, raw: &Value) -> Option<Value> {
    let to_col = raw.get("c")?.clone();
    let to_row = raw.get("r")?.clone();
    let from_col = raw.get("dc").cloned().unwrap_or_else(|| to_col.clone());
    let from_row = raw.get("dr").cloned().unwrap_or_else(|| to_row.clone());
    Some(json!({
        "entity": entity,
        "slot": slot,
        "from_col": from_col,
        "from_row": from_row,
        "to_col": to_col,
        "to_row": to_row,
        "step_started_at": raw.get("em").cloned().unwrap_or_else(|| json!(0)),
        "step_duration_ms": raw.get("ms").cloned().unwrap_or_else(|| json!(300)),
        "direction": raw.get("d").cloned().unwrap_or(Value::Null),
    }))
}

fn calibration_snapshot(view: &HuntView) -> Value {
    let active = view.active_pokemon.as_ref();
    let fingerprint = json!({
        "active_id": active.map(|p| p.id),
        "species_id": active.and_then(|p| p.species_id),
        "level": active.map(|p| p.level),
        "power": active.and_then(|p| p.power),
        "quality": active.and_then(|p| p.quality),
        "potency": active.and_then(|p| p.potency),
        "ivs": active.map(|p| p.ivs),
        "battle_stats": active.and_then(|p| p.battle_stats),
        "types": active.map(|p| &p.types),
        "held_item_id": active.and_then(|p| p.held_item_id),
        "moveset": active.and_then(|p| p.species_id)
            .map(moveset_for).unwrap_or_default(),
        "tm_aoe": active.is_some_and(|p| p.tm_aoe),
        "tm_elemental": active.and_then(|p| p.tm_elemental.as_deref()),
        "bicycle_rarity": view.bicycle_rarity,
        "bicycle_factor": view.bicycle_factor.unwrap_or(1.0),
        "seasonal_type": view.seasonal_type,
        "outland_tier": view.outland_tier,
        "gym_podium": &*view.gym_podium,
        "automation": view.automation,
    });
    json!({
        "fingerprint": fingerprint,
        "active": active.map(|p| json!({
            "id": p.id,
            "species_id": p.species_id,
            "name": p.name,
            "level": p.level,
            "hp": p.hp,
            "max_hp": p.max_hp,
            "power": p.power,
            "quality": p.quality,
            "grade": p.grade,
            "potency": p.potency,
            "ivs": p.ivs,
            "battle_stats": p.battle_stats,
            "types": p.types,
            "held_item_id": p.held_item_id,
            "moveset": p.species_id.map(moveset_for).unwrap_or_default(),
            "tm_aoe": p.tm_aoe,
            "tm_elemental": p.tm_elemental,
        })),
        "bicycle": {
            "rarity": view.bicycle_rarity,
            "factor": view.bicycle_factor.unwrap_or(1.0),
        },
        "seasonal_type": view.seasonal_type,
        "outland_tier": view.outland_tier,
        "in_center": view.in_center,
        "bonuses": {
            "trainer_event_pct": view.bonuses.trainer_event_pct,
            "pokemon_event_pct": view.bonuses.pokemon_event_pct,
            "guild_pct": view.bonuses.guild_pct,
            "twitch_pct": view.bonuses.twitch_pct,
            "kick_xp_pct": view.bonuses.kick_xp_pct,
            "guild_boost_active": view.bonuses.guild_boost_active,
            "vip_active": view.bonuses.vip_active,
            "sacrifice_xp_active": view.bonuses.sacrifice_xp_active,
        }
    })
}

fn moveset_for(species_id: u64) -> Vec<TraceMove> {
    static MOVESETS: OnceLock<BTreeMap<u64, Vec<TraceMove>>> = OnceLock::new();
    MOVESETS.get_or_init(|| {
        let mut by_species = BTreeMap::<u64, Vec<TraceMove>>::new();
        for line in include_str!("../assets/visual/upstream-movesets-v1408.tsv").lines() {
            if line.starts_with('#') || line.is_empty() { continue; }
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 5 { continue; }
            let (Ok(id), Ok(power), Ok(cooldown_ms)) = (
                fields[0].parse::<u64>(),
                fields[3].parse::<f64>(),
                fields[4].parse::<f64>(),
            ) else { continue };
            if !power.is_finite() || !cooldown_ms.is_finite() || cooldown_ms <= 0.0 {
                continue;
            }
            by_species.entry(id).or_default().push(TraceMove {
                move_type: fields[1].to_owned(),
                special: fields[2] == "S",
                power,
                cooldown_ms,
            });
        }
        by_species
    }).get(&species_id).cloned().unwrap_or_default()
}

fn slot_key(value: &Value) -> String {
    value.as_str().map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn number_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| value.as_u64().or_else(|| {
        value.as_f64().filter(|n| n.is_finite() && *n >= 0.0)
            .map(|n| n as u64)
    }))
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn safe_component(raw: &str) -> String {
    let cleaned = raw.chars().map(|ch| {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { '-' }
    }).collect::<String>();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() { "account".into() } else { trimmed.chars().take(48).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_uses_server_origin_and_duration_without_geometry() {
        let value = movement("hero", None, &json!({
            "c": 12, "r": 8, "dc": 11, "dr": 8, "em": 1234, "ms": 275
        })).unwrap();
        assert_eq!(value["from_col"], 11);
        assert_eq!(value["to_col"], 12);
        assert_eq!(value["step_started_at"], 1234);
        assert_eq!(value["step_duration_ms"], 275);
    }

    #[test]
    fn movement_defaults_match_campo_client_contract() {
        let value = movement("mob", Some(json!(7)), &json!({"c": 3, "r": 4})).unwrap();
        assert_eq!(value["from_col"], 3);
        assert_eq!(value["from_row"], 4);
        assert_eq!(value["step_duration_ms"], 300);
    }
}
