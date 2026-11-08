use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceRecord {
    #[serde(default)]
    pub schema: u32,
    pub event_type: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub declared_profile: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub expected_hunt: Option<String>,
    #[serde(default)]
    pub available_hunts: Vec<String>,
    #[serde(default)]
    pub recv_unix_ms: u64,
    #[serde(default)]
    pub recv_monotonic_ms: u64,
    #[serde(default)]
    pub server_ts: Option<u64>,
    #[serde(default)]
    pub connection_generation: u64,
    #[serde(default)]
    pub scene_generation: u64,
    #[serde(default)]
    pub hunt_slug: Option<String>,
    #[serde(default)]
    pub field_slug: Option<String>,
    #[serde(default)]
    pub map_id: Option<String>,
    #[serde(default)]
    pub calibration: Value,
    #[serde(default)]
    pub packet_type: Option<String>,
    #[serde(default)]
    pub packet: Value,
    #[serde(default)]
    pub field_observation: Value,
    #[serde(default)]
    pub kills: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct TraceSession {
    pub path: PathBuf,
    pub records: Vec<TraceRecord>,
}

pub fn load_sessions(path: &Path) -> Result<Vec<TraceSession>, String> {
    let mut paths = Vec::new();
    if path.is_file() {
        paths.push(path.to_path_buf());
    } else {
        let entries = fs::read_dir(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let file = entry.path();
            if file.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                paths.push(file);
            }
        }
    }
    paths.sort();

    let mut sessions = Vec::new();
    for path in paths {
        let file = fs::File::open(&path)
            .map_err(|error| format!("could not open {}: {error}", path.display()))?;
        let mut records = Vec::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|error| format!("{}: {error}", path.display()))?;
            if line.trim().is_empty() { continue; }
            let record = serde_json::from_str::<TraceRecord>(&line)
                .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?;
            records.push(record);
        }
        if !records.is_empty() {
            sessions.push(TraceSession { path, records });
        }
    }
    Ok(sessions)
}

pub fn fingerprint(record: &TraceRecord) -> String {
    serde_json::to_string(
        record.calibration.get("fingerprint").unwrap_or(&Value::Null)
    ).unwrap_or_else(|_| "null".into())
}

pub fn bicycle_factor(record: &TraceRecord) -> f64 {
    record.calibration.get("bicycle")
        .and_then(|bike| bike.get("factor"))
        .and_then(Value::as_f64)
        .filter(|factor| factor.is_finite() && *factor > 0.0)
        .unwrap_or(1.0)
}

pub fn aoe_enabled(record: &TraceRecord) -> bool {
    record.calibration.get("fingerprint")
        .and_then(|state| state.get("tm_aoe"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn active_pokemon_id(record: &TraceRecord) -> Option<u64> {
    record.calibration.get("active")
        .and_then(|active| active.get("id"))
        .and_then(Value::as_u64)
}

pub fn active_hp(record: &TraceRecord) -> Option<u64> {
    record.calibration.get("active")
        .and_then(|active| active.get("hp"))
        .and_then(Value::as_u64)
}

pub fn in_center(record: &TraceRecord) -> bool {
    record.calibration.get("in_center")
        .and_then(Value::as_bool).unwrap_or(false)
}
