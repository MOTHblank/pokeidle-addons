//! Server-native boss farming, safe hunt round trips and a bounded loot journal.
//! Boss repetition/healing is entirely owned by PokéIdle. Starting from a hunt
//! first routes through the Hunt Atlas Center destination, then enters the boss
//! from Center; all travel therefore shares one retry/give-up policy.
use crate::config::{data_root, GameProfile};
use crate::native_gameplay::GameCommand;
use crate::native_hunt_navigation::HuntNavigator;
use crate::native_hunts::HuntView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_LOOT: usize = 500;
/// shared/pokemon-nivel-treinador.mjs: non-starters can fight up to trainer + 5.
const POKEMON_LEVEL_GRACE: u32 = 5;

/// v1.400.1 shared/bosses-lendarios.mjs: boss combat level follows the
/// strongest Pokémon that is actually on the team and allowed to fight.
/// A full roster is required because partial pkMud patches cannot prove who
/// is absent from the team. Zero is authoritative when the confirmed team has
/// no eligible Pokémon; the boss's own floor still determines combat level.
pub fn strongest_eligible_team_level(hunt: &HuntView) -> Option<u32> {
    if !hunt.roster_ready {
        return None;
    }
    let trainer_level = hunt.trainer_level?;
    let ceiling = trainer_level.saturating_add(POKEMON_LEVEL_GRACE);
    Some(hunt.pokemon_roster.iter()
        .filter(|pokemon| pokemon.slot.is_some())
        .filter(|pokemon| pokemon.starter || pokemon.level <= ceiling)
        .map(|pokemon| pokemon.level)
        .max()
        .unwrap_or(0))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BossEntry {
    pub key: String,
    pub name: String,
    pub min_level: u32,
    pub token_id: u64,
    pub token_name: String,
    pub tokens_per_entry: u64,
    /// Pokémon species used for artwork/status when exposed by the boss catalog.
    pub species_id: Option<u64>,
    /// Authoritative battle types from bossesJogaveis, with bossesCatalogo
    /// as a welcome-time fallback for older payload shapes.
    pub types: Vec<String>,
}
impl BossEntry {
    fn species_id(raw: &Value) -> Option<u64> {
        ["pokeId", "pokemonId", "speciesId", "dex"].into_iter()
            .find_map(|field| raw.get(field).and_then(Value::as_u64))
            .filter(|id| *id > 0)
    }

    /// shared/bosses-lendarios.mjs v1.400.1: trainer level still unlocks
    /// the arena, but combat level follows the strongest eligible team
    /// Pokémon and never drops below this boss's own minimum.
    pub fn fight_level(&self, hunt: &HuntView) -> Option<u32> {
        strongest_eligible_team_level(hunt)
            .map(|level| level.max(self.min_level.max(1)))
    }

    fn parse(raw: &Value) -> Option<Self> {
        let key = raw.get("key")?.as_str()?.trim();
        if key.is_empty() || key.len() > 128 ||
            !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
            return None;
        }
        let entry = raw.get("entrada")?;
        let token_id = entry.get("itemId")?.as_u64()?;
        let tokens_per_entry = entry.get("qtd")?.as_u64()?;
        if token_id == 0 || tokens_per_entry == 0 { return None; }
        Some(Self {
            key: key.to_owned(),
            name: raw.get("nome").or_else(|| raw.get("name"))
                .and_then(Value::as_str).unwrap_or(key).to_owned(),
            min_level: raw.get("minNivelTreinador").or_else(|| raw.get("level"))
                .and_then(Value::as_u64).unwrap_or(0).min(u32::MAX as u64) as u32,
            token_id,
            token_name: entry.get("nome").and_then(Value::as_str)
                .unwrap_or("Boss Token").to_owned(),
            tokens_per_entry,
            species_id: Self::species_id(raw),
            types: ["type1", "type2"].into_iter().filter_map(|field|
                raw.get(field).and_then(Value::as_str)
                    .map(str::trim).filter(|kind| !kind.is_empty())
                    .map(str::to_ascii_uppercase)
            ).collect(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BossDrop {
    pub item_id: u64,
    pub quantity: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BossLoot {
    pub at: u64,
    pub key: String,
    pub boss: String,
    pub drops: Vec<BossDrop>,
}

#[derive(Clone, Debug, Default)]
pub struct BossView {
    pub catalog_ready: bool,
    pub state_known: bool,
    pub catalog: Vec<BossEntry>,
    pub auto: bool,
    pub auto_key: Option<String>,
    pub arena: Option<String>,
    pub busy: bool,
    pub can_stop: bool,
    pub phase: String,
    pub original_hunt: Option<String>,
    pub warning: Option<String>,
    pub storage_warning: Option<String>,
    /// Shared with the disk writer; available without waiting for the game socket.
    pub write_status: Arc<Mutex<LootWriteStatus>>,
    pub log: Arc<Vec<BossLoot>>,
}

#[derive(Default, Debug)]
pub struct LootWriteStatus {
    pub unsaved: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage { Starting, Running, Returning }
struct Trip {
    key: String,
    original: Option<String>,
    repeat: bool,
    stage: Stage,
    confirmed: bool,
    ended: bool,
    stop_pending: bool,
    recovering: bool,
    dispatched_at: Option<Instant>,
    resume_stop: bool,
    stop_requested_at: Option<Instant>,
}

#[derive(Default)]
struct LogGate { busy: Mutex<bool>, ready: Condvar }
struct LogLease(Arc<LogGate>);
impl LogGate {
    fn acquire(self: &Arc<Self>) -> LogLease {
        let mut busy = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        while *busy {
            busy = self.ready.wait(busy).unwrap_or_else(|e| e.into_inner());
        }
        *busy = true;
        LogLease(Arc::clone(self))
    }
}
impl Drop for LogLease {
    fn drop(&mut self) {
        let mut busy = self.0.busy.lock().unwrap_or_else(|e| e.into_inner());
        *busy = false;
        self.0.ready.notify_one();
    }
}
fn gate(profile: GameProfile) -> Arc<LogGate> {
    static GATES: OnceLock<[Arc<LogGate>; 4]> = OnceLock::new();
    Arc::clone(&GATES.get_or_init(||
        std::array::from_fn(|_| Arc::new(LogGate::default())))[profile.index()])
}
fn log_path(profile: GameProfile, nick: &str) -> Result<PathBuf, String> {
    // Same stable per-profile/per-player namespace approach as battle history.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in nick.to_lowercase().bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(data_root()?.join("Boss").join(format!("{}-{hash:016x}.json", profile.name())))
}
fn save_log(path: &PathBuf, entries: &[BossLoot]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Boss log directory: {e}"))?;
    }
    let data = serde_json::to_vec(entries).map_err(|e| format!("Boss log encoding: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data).map_err(|e| format!("Boss log staging: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("Boss log save: {e}"))
}

/// Never overwrite a journal that could not be read. Damaged JSON is moved
/// aside before a new journal is allowed to use the original path.
fn load_log(path: &PathBuf) -> Result<(Vec<BossLoot>, Option<String>), String> {
    // On Windows reading a directory can succeed with empty bytes. Never
    // quarantine or rename anything except an actual regular journal file.
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
            return Ok((Vec::new(), None)),
        Err(error) => return Err(format!(
            "Boss loot journal metadata unavailable (original preserved): {error}"
        )),
    };
    if !metadata.file_type().is_file() {
        return Err("Boss loot journal path is not a regular file (original preserved)".into());
    }
    let data = match fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
            return Ok((Vec::new(), None)),
        Err(error) => return Err(format!(
            "Boss loot journal cannot be read (original preserved): {error}"
        )),
    };
    match serde_json::from_slice::<Vec<BossLoot>>(&data) {
        Ok(entries) => Ok((entries, None)),
        Err(error) => {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_nanos();
            for attempt in 0..1000 {
                let backup = path.with_extension(format!(
                    "json.corrupt-{stamp}-{attempt}"
                ));
                if backup.exists() { continue; }
                fs::rename(path, &backup).map_err(|move_error| format!(
                    "Boss loot JSON is invalid ({error}); could not preserve it separately: {move_error}"
                ))?;
                let warning =
                    "Damaged boss loot history was preserved in a backup file. New victories use a fresh journal.".to_owned();
                crate::logging::warn(&format!("{warning} {}", backup.display()));
                return Ok((Vec::new(), Some(warning)));
            }
            Err(format!(
                "Boss loot JSON is invalid ({error}); no unused backup name available"
            ))
        }
    }
}
struct LootWriter {
    entries: Arc<Mutex<Vec<BossLoot>>>,
    wake: Option<mpsc::SyncSender<()>>,
    worker: Option<thread::JoinHandle<()>>,
    status: Arc<Mutex<LootWriteStatus>>,
}
impl LootWriter {
    fn start(profile: GameProfile, nick: &str) -> Result<(Self, Vec<BossLoot>, Option<String>), String> {
        // Ownership survives a previous disconnected socket until its last
        // asynchronous write finishes; no new reader can race an old writer.
        let lease = gate(profile).acquire();
        let path = log_path(profile, nick)?;
        let (existing, load_warning) = load_log(&path)?;
        let existing: Vec<BossLoot> = existing.into_iter().rev().take(MAX_LOOT)
            .collect::<Vec<_>>().into_iter().rev().collect();
        let entries = Arc::new(Mutex::new(existing.clone()));
        let writer_entries = Arc::clone(&entries);
        let status = Arc::new(Mutex::new(LootWriteStatus::default()));
        let writer_status = Arc::clone(&status);
        let (wake, rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new().name("moth-boss-loot".into()).spawn(move || {
            let _lease = lease;
            let mut pending = false;
            let mut disconnected = false;
            let mut failures = 0u8;
            loop {
                if !pending {
                    if rx.recv().is_err() { break; }
                    pending = true;
                }
                // Coalesce all victory notifications; the full document is
                // serialized only by this worker, never by the game socket.
                while rx.try_recv().is_ok() {}
                let result = writer_entries.lock()
                    .map_err(|_| "Boss loot snapshot lock poisoned".to_owned())
                    .and_then(|entries| save_log(&path, entries.as_slice()));
                match result {
                    Ok(()) => {
                        if let Ok(mut state) = writer_status.lock() {
                            state.unsaved = false;
                            state.warning = None;
                        }
                        pending = false;
                        failures = 0;
                    }
                    Err(error) => {
                        crate::logging::warn(&error);
                        if let Ok(mut state) = writer_status.lock() {
                            state.unsaved = true;
                            state.warning = Some(error.clone());
                        }
                        failures = failures.saturating_add(1);
                        // A closed game worker must not hang application exit
                        // indefinitely on a permanently read-only filesystem.
                        if disconnected && failures >= 3 { break; }
                        if disconnected {
                            thread::sleep(Duration::from_millis(250));
                        } else {
                            // New victories must not defeat the backoff and
                            // cause repeated disk writes on the game worker.
                            let deadline = Instant::now() +
                                Duration::from_secs(1u64 << failures.min(5));
                            loop {
                                let remaining = deadline.saturating_duration_since(Instant::now());
                                if remaining.is_zero() { break; }
                                match rx.recv_timeout(remaining) {
                                    Ok(()) => {}, // coalesce without an early retry
                                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                                        disconnected = true;
                                        break;
                                    }
                                }
                            }
                        }
                        // Keep the latest snapshot pending even without any
                        // new victories; retry a failed write automatically.
                    }
                }
            }
        }).map_err(|e| format!("Could not start boss loot writer: {e}"))?;
        Ok((Self { entries, wake: Some(wake), worker: Some(worker), status },
            existing, load_warning))
    }
    fn append(&self, item: BossLoot) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(item);
            if entries.len() > MAX_LOOT { entries.remove(0); }
            if let Ok(mut state) = self.status.lock() { state.unsaved = true; }
            if let Some(wake) = self.wake.as_ref() { let _ = wake.try_send(()); }
        }
    }
}
impl Drop for LootWriter {
    fn drop(&mut self) {
        // Dropped only on the game worker. Close the queue and join its final
        // bounded persistence attempt before that worker is considered done.
        self.wake.take();
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                crate::logging::warn("Boss loot writer panicked during shutdown");
            }
        }
    }
}

pub struct BossManager {
    pub view: BossView,
    trip: Option<Trip>,
    writer: Option<LootWriter>,
}
impl BossManager {
    pub fn new(profile: GameProfile, nick: &str) -> Self {
        // A read-only or damaged data directory must never prevent an
        // authenticated game connection from starting. Farming still works;
        // the UI explicitly explains when loot persistence is unavailable.
        let (writer, log, storage_warning) = match LootWriter::start(profile, nick) {
            Ok((writer, log, warning)) => (Some(writer), log, warning),
            Err(error) => {
                crate::logging::warn(&format!("Boss loot journal unavailable: {error}"));
                (None, Vec::new(), Some(
                    "Boss loot journal is unavailable; new drops will not survive restart".into()))
            }
        };
        let write_status = writer.as_ref()
            .map(|writer| Arc::clone(&writer.status)).unwrap_or_default();
        Self { view: BossView {
            log: Arc::new(log), storage_warning, write_status, ..Default::default()
        }, trip: None, writer }
    }
    pub fn publish(&self, shared: &Arc<Mutex<BossView>>) {
        if let Ok(mut state) = shared.lock() {
            *state = self.view.clone();
            state.busy = self.trip.is_some();
            state.can_stop = self.view.auto || self.trip.as_ref().is_some_and(|trip|
                !trip.recovering && (trip.stage == Stage::Starting ||
                    (trip.repeat && trip.stage == Stage::Running)));
        }
    }
    pub fn start(&mut self, key: &str, repeat: bool, hunt: &HuntView,
                 navigator: &mut HuntNavigator, now: Instant) -> Result<(), String> {
        if self.trip.is_some() { return Err("A boss trip is already in progress".into()); }
        if self.view.auto || self.view.arena.is_some() {
            return Err("A boss encounter or server repetition is already active".into());
        }
        if !self.view.catalog_ready {
            return Err("Waiting for the server boss catalog".into());
        }
        if !self.view.state_known {
            return Err("Waiting for authoritative server boss state".into());
        }
        let boss = self.view.catalog.iter().find(|b| b.key == key)
            .ok_or_else(|| "This boss does not have a playable server arena".to_owned())?;
        if hunt.trainer_level.unwrap_or(0) < boss.min_level {
            return Err(format!("Trainer level {} required for {}", boss.min_level, boss.name));
        }
        let tokens = hunt.items.get(&boss.token_id).copied().unwrap_or(0);
        if tokens < boss.tokens_per_entry {
            return Err(format!("Not enough {} to enter {}", boss.token_name, boss.name));
        }
        if !hunt.in_center && hunt.hunt_slug.is_none() {
            return Err("Start from a regular hunt or the Pokémon Center".into());
        }
        let original = if hunt.in_center { None } else { hunt.hunt_slug.clone() };
        if original.as_ref().is_some_and(|slug|
            !hunt.catalog_ready || !hunt.hunts.iter().any(|entry| entry.slug == *slug)) {
            return Err("Current hunt is not confirmed in the server's travel catalog".into());
        }
        self.trip = Some(Trip {
            key: key.to_owned(), original: original.clone(), repeat,
            stage: Stage::Starting, confirmed: false, ended: false,
            stop_pending: false, recovering: false, dispatched_at: None,
            resume_stop: false, stop_requested_at: None,
        });
        self.view.original_hunt = original;
        self.view.warning = None;
        if hunt.in_center {
            self.view.phase = "At Pokémon Center; entering boss".into();
            navigator.boss(key.to_owned(), repeat, now);
        } else {
            self.view.phase = "Returning to Pokémon Center before boss entry".into();
            navigator.center(now);
        }
        Ok(())
    }
    pub fn stop(&mut self, navigator: &mut HuntNavigator,
                _hunt: &HuntView, now: Instant) -> Option<GameCommand> {
        if self.trip.is_none() && self.view.auto {
            self.view.phase = "Stopping server boss repetition".into();
            return self.view.auto_key.clone().map(|key|
                GameCommand::BossAuto { active: false, key });
        }
        let trip = self.trip.as_mut()?;
        trip.ended = true;
        if trip.stage == Stage::Starting {
            // The HuntNavigator has not emitted the boss destination yet.
            navigator.cancel();
            self.trip = None;
            self.view.original_hunt = None;
            self.view.phase = "Boss request cancelled before entry".into();
            return None;
        }
        // A Start that is waiting for confirmation may still be queued for a
        // Hunt Atlas-style retry. Stop must retire that destination first.
        navigator.cancel();
        if !trip.repeat { return None; }
        trip.stop_pending = true;
        trip.stop_requested_at = Some(now);
        self.view.phase = "Stopping boss repetition; waiting for the arena to clear".into();
        Some(GameCommand::BossAuto { active: false, key: trip.key.clone() })
    }
    pub fn leave(&mut self, navigator: &mut HuntNavigator) -> Vec<GameCommand> {
        let mut commands = Vec::new();
        // Leaving/cancelling owns the destination now; never allow a pending
        // boss retry to race the exit command.
        navigator.cancel();
        if let Some(trip) = self.trip.as_mut() {
            if trip.stage == Stage::Starting {
                navigator.cancel();
                self.trip = None;
                self.view.original_hunt = None;
                self.view.phase = "Boss trip cancelled".into();
                return commands;
            }
            trip.ended = true;
            if trip.repeat {
                commands.push(GameCommand::BossAuto { active: false, key: trip.key.clone() });
            }
        }
        if self.trip.is_none() && self.view.auto {
            if let Some(key) = self.view.auto_key.as_ref() {
                commands.push(GameCommand::BossAuto { active: false, key: key.clone() });
            }
        }
        commands.push(GameCommand::BossExit);
        self.view.phase = "Leaving boss arena; return to hunt pending".into();
        commands
    }
    pub fn ingest(&mut self, frame: &Value) {
        if frame.get("t").and_then(Value::as_str) == Some("welcome") {
            if let Some(available) = frame.get("bossesJogaveis").and_then(Value::as_array) {
                let gallery = frame.get("bossesCatalogo").and_then(Value::as_array);
                self.view.catalog = available.iter().filter_map(BossEntry::parse).map(|mut boss| {
                    // Current upstream sends type1/type2 on the playable
                    // boss record. Keep the gallery fallback because the
                    // browser client intentionally supports both shapes.
                    if let Some(meta) = gallery.and_then(|rows| rows.iter().find(|row|
                        row.get("key").and_then(Value::as_str) == Some(boss.key.as_str())
                    )) {
                        if boss.types.is_empty() {
                            boss.types = ["type1", "type2"].into_iter().filter_map(|field|
                                meta.get(field).and_then(Value::as_str)
                                    .map(str::trim).filter(|kind| !kind.is_empty())
                                    .map(str::to_ascii_uppercase)
                            ).collect();
                        }
                        if boss.species_id.is_none() {
                            boss.species_id = BossEntry::species_id(meta);
                        }
                    }
                    boss
                }).collect();
                self.view.catalog_ready = true;
            }
        }
        if let Some(boss) = frame.get("estado").and_then(|state| state.get("boss")) {
            self.view.state_known = boss.is_object();
            self.view.auto = boss.get("auto").and_then(Value::as_bool).unwrap_or(false);
            self.view.auto_key = boss.get("autoKey").and_then(Value::as_str).map(str::to_owned);
            self.view.arena = boss.get("arena").and_then(|arena| arena.get("key"))
                .and_then(Value::as_str).map(str::to_owned);
            if let Some(trip) = self.trip.as_mut() {
                if trip.stage == Stage::Running {
                    if trip.repeat {
                        if self.view.auto &&
                            self.view.auto_key.as_deref() == Some(trip.key.as_str()) {
                            trip.confirmed = true;
                        }
                        // A fresh authoritative boss state after Stop is enough
                        // to settle the request, even if Start was never acked.
                        if !self.view.auto && (trip.confirmed || trip.stop_pending) {
                            trip.ended = true;
                            trip.stop_pending = false;
                        }
                    } else if self.view.arena.as_deref() == Some(trip.key.as_str()) {
                        trip.confirmed = true;
                    }
                }
            }
        }
        if let Some(events) = frame.get("ev").and_then(Value::as_array) {
            for event in events {
                match event.get("k").and_then(Value::as_str) {
                    Some("bossMorto") => {
                        let drops = event.get("drops").and_then(Value::as_array)
                            .into_iter().flatten().filter_map(|drop| {
                                let item_id = drop.get("itemId")?.as_u64()?;
                                let quantity = drop.get("qtd")?.as_u64()?;
                                (item_id > 0 && quantity > 0)
                                    .then_some(BossDrop { item_id, quantity })
                            }).collect();
                        let entry = BossLoot {
                            at: SystemTime::now().duration_since(UNIX_EPOCH)
                                .unwrap_or_default().as_secs(),
                            key: event.get("key").and_then(Value::as_str)
                                .unwrap_or("").to_owned(),
                            boss: event.get("nome").and_then(Value::as_str)
                                .unwrap_or("Boss").to_owned(),
                            drops,
                        };
                        let log = Arc::make_mut(&mut self.view.log);
                        log.push(entry.clone());
                        if log.len() > MAX_LOOT { log.remove(0); }
                        if let Some(writer) = &self.writer { writer.append(entry); }
                        if let Some(trip) = self.trip.as_mut() {
                            if !trip.repeat && trip.stage == Stage::Running &&
                                trip.confirmed && event.get("key").and_then(Value::as_str)
                                    == Some(trip.key.as_str()) {
                                trip.ended = true;
                            }
                        }
                    }
                    Some("bossPerdeu") | Some("bossSaiu") => {
                        if let Some(trip) = self.trip.as_mut() {
                            if !trip.repeat { trip.ended = true; }
                        }
                    }
                    Some("bossEntrou") => {
                        if let Some(trip) = self.trip.as_mut() {
                            if event.get("key").and_then(Value::as_str)
                                .map_or(true, |key| key == trip.key) {
                                trip.confirmed = true;
                            }
                        }
                    }
                    Some("bossAuto") => {
                        if let Some(active) = event.get("ativo").and_then(Value::as_bool) {
                            self.view.auto = active;
                            if let Some(trip) = self.trip.as_mut() {
                                if trip.repeat && trip.stage == Stage::Running {
                                    if active { trip.confirmed = true; }
                                    else if trip.confirmed || trip.stop_pending {
                                        trip.ended = true;
                                        trip.stop_pending = false;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    /// Boss entry is a two-step Hunt Atlas trip: reach Center first, then use
    /// the same navigator to enter the requested arena/repetition.
    pub fn next_action(&mut self, hunt: &HuntView,
                       navigator: &mut HuntNavigator, now: Instant) -> Option<GameCommand> {
        if self.trip.as_ref()?.recovering { return None; }

        if self.trip.as_ref().is_some_and(|trip| trip.resume_stop) {
            let trip = self.trip.as_mut().expect("trip still present");
            trip.resume_stop = false;
            // Fresh command, only after the new authenticated welcome confirms
            // the server is still repeating the same boss we were stopping.
            return Some(GameCommand::BossAuto { active:false, key:trip.key.clone() });
        }

        if let Some(key) = self.trip.as_ref()
            .filter(|trip| trip.confirmed)
            .map(|trip| trip.key.clone()) {
            navigator.confirm_boss(&key);
        }

        if self.trip.as_ref().is_some_and(|trip| trip.stage == Stage::Starting) {
            if !hunt.in_center {
                if !navigator.center_pending() {
                    navigator.center(now);
                }
                self.view.phase = "Returning to Pokémon Center before boss entry".into();
                return navigator.next_action(hunt, now);
            }
            if !navigator.boss_pending() {
                let trip = self.trip.as_ref().expect("trip still present");
                navigator.boss(trip.key.clone(), trip.repeat, now);
                self.view.phase = "At Pokémon Center; entering boss".into();
            }
        }

        let awaiting_entry = self.trip.as_ref().is_some_and(|trip|
            matches!(trip.stage, Stage::Starting | Stage::Running) &&
            !trip.confirmed && !trip.ended && !trip.stop_pending);
        if awaiting_entry {
            let command = navigator.next_action(hunt, now)?;
            let (key, repeat) = match &command {
                GameCommand::BossEnter(key) => (key.as_str(), false),
                GameCommand::BossAuto { active: true, key } => (key.as_str(), true),
                _ => {
                    navigator.cancel();
                    self.view.warning = Some(
                        "Boss destination was replaced by another navigation request".into());
                    self.view.phase = "Boss entry cancelled".into();
                    self.trip = None;
                    self.view.original_hunt = None;
                    return None;
                }
            };
            let matches_trip = self.trip.as_ref().is_some_and(|trip|
                trip.key == key && trip.repeat == repeat);
            if !matches_trip {
                navigator.cancel();
                self.view.warning = Some(
                    "Boss destination no longer matches the active request".into());
                self.view.phase = "Boss entry cancelled".into();
                self.trip = None;
                self.view.original_hunt = None;
                return None;
            }

            let first_dispatch = self.trip.as_ref()
                .is_some_and(|trip| trip.dispatched_at.is_none());
            if first_dispatch {
                // Recheck only before the first actual send. Once accepted,
                // the token can disappear from inventory before the entry
                // event arrives, and that must not cancel a valid request.
                let allowed = self.view.catalog.iter().find(|boss| boss.key == key)
                    .is_some_and(|boss| hunt.trainer_level.unwrap_or(0) >= boss.min_level &&
                        hunt.items.get(&boss.token_id).copied().unwrap_or(0) >= boss.tokens_per_entry);
                if !allowed {
                    navigator.cancel();
                    self.view.warning = Some(
                        "Boss tokens or requirements changed before entry; request cancelled".into());
                    self.view.phase = "Boss entry cancelled".into();
                    self.trip = None;
                    self.view.original_hunt = None;
                    return None;
                }
                if let Some(trip) = self.trip.as_mut() {
                    trip.stage = Stage::Running;
                    trip.dispatched_at = Some(now);
                    self.view.phase = if trip.repeat {
                        "Switching from the current hunt to boss farm".into()
                    } else {
                        "Switching from the current hunt to the boss".into()
                    };
                }
            }
            return Some(command);
        }

        let trip = self.trip.as_mut()?;
        if trip.stage == Stage::Running && trip.stop_pending &&
            trip.stop_requested_at.is_some_and(|at|
                now.saturating_duration_since(at) >= Duration::from_secs(20)) &&
            !self.view.auto && self.view.arena.is_none() && hunt.in_center {
            // A missing Stop event must not strand a trip forever when the
            // last authoritative state reports no repetition or arena.
            trip.stop_pending = false;
            self.view.warning = Some(
                "Boss Stop was not acknowledged; server currently reports no active boss".into()
            );
        }
        if trip.stage == Stage::Running && trip.ended && !trip.stop_pending &&
            !self.view.auto && hunt.in_center && self.view.arena.is_none() {
            if let Some(original) = trip.original.as_ref() {
                if !hunt.catalog_ready || !hunt.hunts.iter().any(|entry| entry.slug == *original) {
                    self.view.warning = Some("Original hunt is no longer available; return manually".into());
                    self.view.phase = "Boss run complete; return destination unavailable".into();
                    self.trip = None;
                    self.view.original_hunt = None;
                    return None;
                }
                navigator.select(original.clone(), now);
                trip.stage = Stage::Returning;
                self.view.phase = "Returning to saved hunt".into();
            } else {
                self.trip = None;
                self.view.original_hunt = None;
                self.view.phase = "Boss run complete at Pokémon Center".into();
            }
        }
        if let Some(trip) = self.trip.as_ref() {
            if trip.stage == Stage::Returning &&
                !hunt.in_center && hunt.hunt_slug == trip.original {
                self.trip = None;
                self.view.original_hunt = None;
                self.view.phase = "Boss run complete; original hunt restored".into();
            }
        }
        None
    }
    pub fn active(&self) -> bool { self.trip.is_some() }

    /// A disconnected socket never replays a boss-enter/start command. Keep
    /// only an already-started trip's return target until the new welcome
    /// establishes the server's current arena, auto state and location.
    pub fn on_disconnect(&mut self, _navigator: &HuntNavigator, _hunt: &HuntView) {
        if let Some(trip) = self.trip.as_mut() {
            if trip.stage == Stage::Starting {
                // The boss command is emitted only after this stage changes to
                // Running, so a disconnect here proves nothing was sent.
                self.trip = None;
                self.view.original_hunt = None;
                self.view.warning = Some(
                    "Boss request cancelled before entry because the connection dropped".into());
                self.view.phase = "Boss trip interrupted".into();
            } else {
                trip.recovering = true;
                self.view.warning = Some(
                    "Connection interrupted; verifying boss state before restoring saved hunt".into());
                self.view.phase = "Verifying boss state after reconnect".into();
            }
        }
        self.view.catalog_ready = false;
        self.view.state_known = false;
        self.view.catalog.clear();
        self.view.auto = false;
        self.view.auto_key = None;
        self.view.arena = None;
    }

    /// Called ONLY after a fresh, authenticated welcome from the same socket
    /// owner. Resume passive monitoring or the final hunt return; never restart
    /// a battle or enable repetition on an unconfirmed network reconnection.
    pub fn recover_on_welcome(&mut self, hunt: &HuntView) {
        let Some(trip) = self.trip.as_mut() else { return; };
        if !trip.recovering { return; }
        trip.recovering = false;
        self.view.warning = None;
        if !self.view.state_known {
            self.view.warning = Some(
                "Server boss state unavailable after reconnect; return requires manual review".into());
            self.trip = None;
            self.view.original_hunt = None;
            self.view.phase = "Boss trip interrupted".into();
            return;
        }
        if self.view.auto {
            if self.view.auto_key.as_deref() != Some(trip.key.as_str()) {
                self.view.warning = Some(
                    "Another boss repetition is active; saved hunt return was cancelled".into());
                self.trip = None;
                self.view.original_hunt = None;
                self.view.phase = "Boss trip interrupted".into();
                return;
            }
            trip.stage = Stage::Running;
            trip.confirmed = true;
            trip.resume_stop = trip.stop_pending;
            trip.ended = trip.stop_pending;
            self.view.phase = if trip.resume_stop {
                "Stopping boss repetition after reconnect".into()
            } else {
                "Server boss repetition resumed after reconnect".into()
            };
        } else if let Some(arena) = self.view.arena.as_deref() {
            if arena != trip.key.as_str() {
                self.view.warning = Some(
                    "Another boss encounter is active; saved hunt return was cancelled".into());
                self.trip = None;
                self.view.original_hunt = None;
                self.view.phase = "Boss trip interrupted".into();
                return;
            }
            trip.stage = Stage::Running;
            trip.confirmed = true;
            // The server is no longer repeating. Wait for the existing arena
            // to clear, even if its victory/loss event occurred while offline.
            trip.ended = true;
            trip.stop_pending = false;
            self.view.phase = "Waiting for boss encounter after reconnect".into();
        } else if hunt.in_center {
            trip.stage = Stage::Running;
            trip.ended = true;
            trip.stop_pending = false;
            self.view.phase = "Restoring saved hunt after reconnect".into();
        } else if trip.original.as_deref() == hunt.hunt_slug.as_deref()
            && trip.original.is_some() {
            self.trip = None;
            self.view.original_hunt = None;
            self.view.phase = "Original hunt already restored after reconnect".into();
        } else {
            self.trip = None;
            self.view.original_hunt = None;
            self.view.warning = Some(
                "Game location changed during reconnection; automatic return cancelled".into());
            self.view.phase = "Boss trip interrupted".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::native_hunts::HuntEntry;
    fn manager() -> BossManager {
        let mut manager = BossManager {
            view: BossView::default(), trip: None, writer: None,
        };
        manager.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"bronze","nome":"Bronze Boss",
            "minNivelTreinador":20,
            "entrada":{"itemId":500,"qtd":2,"nome":"Bronze Boss Token"}
        }],"estado":{"boss":{"auto":false,"arena":null}}}));
        manager
    }
    fn hunt() -> HuntView {
        let mut view = HuntView::default();
        view.hunt_slug = Some("bulbasaur".into());
        view.hunts = Arc::new(vec![HuntEntry { slug: "bulbasaur".into(), ..Default::default() }]);
        view.catalog_ready = true;
        view.trainer_level = Some(22);
        view.items = Arc::new([(500,5)].into());
        view
    }
    #[test]
    fn playable_boss_types_use_server_payload_and_catalog_fallback() {
        let mut direct=manager();
        direct.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"kyogre","nome":"Kyogre","type1":"water",
            "entrada":{"itemId":500,"qtd":1}
        }],"estado":{"boss":{"auto":false,"arena":null}}}));
        assert_eq!(direct.view.catalog[0].types,vec!["WATER"]);

        let mut fallback=manager();
        fallback.ingest(&json!({"t":"welcome",
            "bossesCatalogo":[{"key":"regice","type1":"ICE"}],
            "bossesJogaveis":[{
                "key":"regice","nome":"Regice",
                "entrada":{"itemId":500,"qtd":1}
            }],"estado":{"boss":{"auto":false,"arena":null}}}));
        assert_eq!(fallback.view.catalog[0].types,vec!["ICE"]);
    }

    #[test]
    fn effective_boss_level_uses_strongest_eligible_team_pokemon() {
        let catalog=serde_json::json!({"key":"regice","nome":"Regice",
            "minNivelTreinador":300,
            "entrada":{"itemId":500,"qtd":1}});
        let boss=BossEntry::parse(&catalog).expect("playable boss");
        let mut h=HuntView::default();
        h.trainer_level=Some(5_000);
        assert_eq!(boss.fight_level(&h),None,
            "combat level must wait for a complete roster");

        h.roster_ready=true;
        h.pokemon_roster=Arc::new(vec![
            crate::native_hunts::PokemonInfo {
                level:2_000, slot:Some(0), ..Default::default()
            },
            crate::native_hunts::PokemonInfo {
                level:30_000, slot:None, ..Default::default()
            },
            crate::native_hunts::PokemonInfo {
                level:5_006, slot:Some(1), ..Default::default()
            },
        ]);
        assert_eq!(strongest_eligible_team_level(&h),Some(2_000),
            "depot Pokémon and non-starters above trainer + 5 must not count");
        assert_eq!(boss.fight_level(&h),Some(2_000));

        let rayquaza=BossEntry {
            min_level:3_100, ..boss.clone()
        };
        assert_eq!(rayquaza.fight_level(&h),Some(3_100),
            "the boss minimum remains the combat-level floor");

        h.pokemon_roster=Arc::new(vec![crate::native_hunts::PokemonInfo {
            level:8_000, slot:Some(0), starter:true, ..Default::default()
        }]);
        assert_eq!(boss.fight_level(&h),Some(8_000),
            "starter Pokémon ignore the trainer-level usability ceiling");
        assert_eq!(boss.min_level,300,
            "trainer-level unlock requirement remains separate");
    }
    #[test]
    fn farm_routes_through_center_and_restores_exact_hunt() {
        let mut m = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut h = hunt();
        m.start("bronze", true, &h, &mut nav, now).unwrap();
        assert!(matches!(m.next_action(&h, &mut nav, now), Some(GameCommand::Center)));
        h.in_center = true;
        nav.observe(&json!({"t":"estado","estado":{"noCentro":true}}), &h, now);
        assert!(matches!(m.next_action(&h, &mut nav, now),
            Some(GameCommand::BossAuto { active:true, .. })));
        m.ingest(&json!({"t":"estado","estado":{"boss":{"auto":true,
            "autoKey":"bronze","arena":{"key":"bronze"}}}}));
        m.ingest(&json!({"t":"batalha","ev":[{"k":"bossAuto","ativo":false}]}));
        m.ingest(&json!({"t":"estado","estado":{"boss":{"auto":false,"arena":null}}}));
        h.in_center = true;
        m.next_action(&h, &mut nav, now);
        assert_eq!(m.view.phase,"Returning to saved hunt");
        assert!(matches!(nav.next_action(&h, now), Some(GameCommand::Hunt(slug)) if slug=="bulbasaur"));
        h.in_center = false;
        m.next_action(&h, &mut nav, now);
        assert!(!m.active());
    }
    #[test]
    fn validates_unlocked_tokens_and_rejects_nonplayable_bosses() {
        let mut m = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut h = hunt();
        assert!(m.start("missing", true, &h, &mut nav, now).is_err());
        h.items = Arc::new([(500,1)].into());
        assert!(m.start("bronze", true, &h, &mut nav, now).is_err());
        h.items = Arc::new([(500,5)].into());
        h.trainer_level = Some(10);
        assert!(m.start("bronze", true, &h, &mut nav, now).is_err());
        assert!(!m.active());
    }
    #[test]
    fn stop_waits_for_server_confirmation_before_returning() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut state = hunt();
        state.in_center = true;
        manager.start("bronze", true, &state, &mut nav, now).unwrap();
        assert!(matches!(manager.next_action(&state, &mut nav, now),
            Some(GameCommand::BossAuto { active:true, .. })));
        assert!(matches!(manager.stop(&mut nav, &state, now),
            Some(GameCommand::BossAuto { active:false, .. })));
        manager.next_action(&state, &mut nav, now);
        assert!(manager.active(), "must not return before the server stops");
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossAuto","ativo":false}]}));
        manager.next_action(&state, &mut nav, now);
        assert!(!manager.active(), "center-started farm should finish after server ack");
    }
    #[test]
    fn leaving_external_server_repeat_disables_it_before_exit() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        manager.ingest(&json!({"t":"estado","estado":{"boss":{
            "auto":true,"autoKey":"bronze","arena":{"key":"bronze"}}}}));
        let commands = manager.leave(&mut nav);
        assert!(matches!(commands.first(),
            Some(GameCommand::BossAuto { active:false, .. })));
        assert!(matches!(commands.last(),Some(GameCommand::BossExit)));
    }
    #[test]
    fn return_after_socket_reconnect_never_restarts_boss() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        view.in_center = true;
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::BossAuto {active:true,..})));
        manager.ingest(&json!({"t":"estado","estado":{"boss":{
            "auto":true,"autoKey":"bronze","arena":{"key":"bronze"}}}}));
        manager.on_disconnect(&nav, &view);
        assert!(manager.active());
        assert!(manager.next_action(&view, &mut nav, now).is_none());

        // Farm ended while offline. The new socket's full welcome is the
        // first authoritative evidence that returning is now safe.
        manager.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"bronze","nome":"Bronze Boss",
            "entrada":{"itemId":500,"qtd":2,"nome":"Bronze Boss Token"}
        }],"estado":{"boss":{"auto":false,"arena":null}}}));
        let mut fresh = HuntNavigator::default();
        manager.recover_on_welcome(&view);
        assert!(manager.next_action(&view, &mut fresh, now).is_none());
        assert!(matches!(fresh.next_action(&view, now),
            Some(GameCommand::Hunt(slug)) if slug=="bulbasaur"));
        view.in_center = false;
        manager.next_action(&view, &mut fresh, now);
        assert!(!manager.active());
    }

    #[test]
    fn reconnect_while_auto_running_only_monitors_existing_server_farm() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::Center)));
        view.in_center = true;
        nav.observe(&json!({"t":"estado","estado":{"noCentro":true}}), &view, now);
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::BossAuto {active:true,..})));
        manager.on_disconnect(&nav, &view);
        manager.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"bronze","nome":"Bronze Boss",
            "entrada":{"itemId":500,"qtd":2,"nome":"Bronze Boss Token"}
        }],"estado":{"boss":{"auto":true,"autoKey":"bronze",
            "arena":{"key":"bronze"}}}}));
        let mut fresh = HuntNavigator::default();
        manager.recover_on_welcome(&view);
        assert!(manager.active());
        assert!(manager.next_action(&view, &mut fresh, now).is_none());
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossAuto","ativo":false}]}));
        manager.ingest(&json!({"t":"estado","estado":{"boss":{
            "auto":false,"arena":null}}}));
        manager.next_action(&view, &mut fresh, now);
        assert!(!manager.active(), "a trip started at center should finish there");
    }

    #[test]
    fn requested_stop_survives_reconnect_without_restarting_farm() {
        let mut manager = manager();
        let mut navigator = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", true, &view, &mut navigator, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut navigator, now),
            Some(GameCommand::BossAuto {active:true,..})));
        manager.ingest(&json!({"t":"estado","estado":{"boss":{
            "auto":true,"autoKey":"bronze","arena":{"key":"bronze"}}}}));
        assert!(matches!(manager.stop(&mut navigator, &view, now),
            Some(GameCommand::BossAuto {active:false,..})));
        manager.on_disconnect(&navigator, &view);
        manager.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"bronze","nome":"Bronze Boss",
            "entrada":{"itemId":500,"qtd":2,"nome":"Bronze Boss Token"}
        }],"estado":{"boss":{"auto":true,"autoKey":"bronze",
            "arena":{"key":"bronze"}}}}));
        let mut fresh = HuntNavigator::default();
        manager.recover_on_welcome(&view);
        assert!(matches!(manager.next_action(&view, &mut fresh, now),
            Some(GameCommand::BossAuto {active:false,..})));
        assert!(manager.next_action(&view, &mut fresh, now).is_none());
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossAuto","ativo":false}]}));
        manager.ingest(&json!({"t":"estado","estado":{"boss":{
            "auto":false,"arena":null}}}));
        manager.next_action(&view, &mut fresh, now);
        assert!(!manager.active());
    }

    #[test]
    fn lost_connection_before_boss_dispatch_cancels_pending_entry() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        manager.on_disconnect(&nav, &view);
        view.in_center = true;
        manager.recover_on_welcome(&view);
        assert!(manager.next_action(&view, &mut nav, now).is_none());
        assert!(!manager.active());
    }

    #[test]
    fn disconnect_after_boss_dispatch_still_restores_saved_hunt() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::BossAuto {active:true,..})));
        manager.on_disconnect(&nav, &view);
        assert!(manager.active());
        view.in_center = true;
        manager.ingest(&json!({"t":"welcome","bossesJogaveis":[{
            "key":"bronze","nome":"Bronze Boss",
            "entrada":{"itemId":500,"qtd":2,"nome":"Bronze Boss Token"}
        }],"estado":{"boss":{"auto":false,"arena":null}}}));
        let mut fresh = HuntNavigator::default();
        manager.recover_on_welcome(&view);
        assert!(manager.next_action(&view, &mut fresh, now).is_none());
        assert!(matches!(fresh.next_action(&view, now),
            Some(GameCommand::Hunt(slug)) if slug == "bulbasaur"));
        view.in_center = false;
        manager.next_action(&view, &mut fresh, now);
        assert!(!manager.active());
    }

    #[test]
    fn one_off_boss_victory_must_match_requested_boss() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", false, &view, &mut nav, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::BossEnter(_))));
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossMorto",
            "key":"other","nome":"Other Boss","drops":[]}]}));
        assert!(manager.active());
        assert!(!manager.trip.as_ref().unwrap().ended);
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossEntrou","key":"bronze"}]}));
        manager.ingest(&json!({"t":"batalha","ev":[{"k":"bossMorto",
            "key":"bronze","nome":"Bronze Boss","drops":[]}]}));
        assert!(manager.trip.as_ref().unwrap().ended);
    }

    #[test]
    fn token_balance_is_rechecked_after_center_before_boss_entry() {
        let mut manager = manager();
        let mut navigator = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        manager.start("bronze", true, &view, &mut navigator, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut navigator, now),
            Some(GameCommand::Center)));
        view.in_center = true;
        navigator.observe(&json!({"t":"estado","estado":{"noCentro":true}}), &view, now);
        view.items = Arc::new([(500,0)].into());
        assert!(manager.next_action(&view, &mut navigator, now).is_none());
        assert_eq!(manager.view.phase, "Boss entry cancelled");
        assert!(!manager.active());
        assert!(navigator.next_action(&view, now).is_none());
    }
    #[test]
    fn unconfirmed_boss_start_retries_like_hunt_atlas() {
        let mut manager = manager();
        let mut navigator = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", true, &view, &mut navigator, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut navigator, now),
            Some(GameCommand::BossAuto {active:true,..})));
        assert!(manager.next_action(&view, &mut navigator,
            now + Duration::from_millis(100)).is_none(),
            "the shared navigator must respect the Hunt Atlas retry cadence");
        assert!(matches!(manager.next_action(&view, &mut navigator,
            now + Duration::from_millis(350)),
            Some(GameCommand::BossAuto {active:true,..})));
        assert!(matches!(manager.next_action(&view, &mut navigator,
            now + Duration::from_secs(20)),
            Some(GameCommand::BossAuto {active:true,..})),
            "an unconfirmed boss destination keeps retrying just like a hunt change");
        assert!(manager.active());
    }
    #[test]
    fn cancel_before_boss_dispatch_does_not_leave_a_trip_running() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let view = hunt();
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        assert!(manager.stop(&mut nav, &view, now).is_none());
        assert!(!manager.active());
        assert!(nav.next_action(&view, now).is_none());
    }

    #[test]
    fn boss_start_waits_on_the_same_switch_gate_as_hunt_atlas() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let at = Instant::now();
        let view = hunt();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,
            "centroLivreEm":1003000,
            "trocaHuntLivreEm":1000000
        }}), &view, at);
        manager.start("bronze", true, &view, &mut nav, at).unwrap();
        assert!(manager.next_action(&view, &mut nav, at).is_none());
        assert!(manager.next_action(&view, &mut nav,
            at + Duration::from_millis(2999)).is_none());
        assert!(matches!(manager.next_action(&view, &mut nav,
            at + Duration::from_millis(3000)), Some(GameCommand::Center)));
    }

    #[test]
    fn canceled_pending_boss_request_never_emits_a_start() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let at = Instant::now();
        let view = hunt();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,
            "centroLivreEm":1003000,
            "trocaHuntLivreEm":1000000
        }}), &view, at);
        manager.start("bronze", true, &view, &mut nav, at).unwrap();
        assert!(manager.next_action(&view, &mut nav, at).is_none());
        assert!(manager.stop(&mut nav, &view, at).is_none());
        assert!(!manager.active());
        assert!(manager.next_action(&view, &mut nav,
            at + Duration::from_secs(3)).is_none());
    }

    #[test]
    fn stop_state_without_start_ack_finishes_when_server_is_inactive() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        assert!(matches!(manager.next_action(&view, &mut nav, now),
            Some(GameCommand::BossAuto { active: true, .. })));
        assert!(matches!(manager.stop(&mut nav, &view, now),
            Some(GameCommand::BossAuto { active: false, .. })));
        manager.ingest(&json!({"t":"estado","estado":{"boss":{"auto":false,"arena":null}}}));
        manager.next_action(&view, &mut nav, now);
        assert!(!manager.active(), "authoritative inactive state must settle Stop");
    }

    #[test]
    fn stop_without_event_times_out_only_if_server_is_inactive() {
        let mut manager = manager();
        let mut nav = HuntNavigator::default();
        let now = Instant::now();
        let mut view = hunt();
        view.in_center = true;
        manager.start("bronze", true, &view, &mut nav, now).unwrap();
        manager.next_action(&view, &mut nav, now);
        manager.stop(&mut nav, &view, now);
        manager.next_action(&view, &mut nav, now + Duration::from_secs(19));
        assert!(manager.active());
        manager.next_action(&view, &mut nav, now + Duration::from_secs(20));
        assert!(!manager.active());
        assert!(manager.view.warning.as_deref().unwrap_or("").contains("not acknowledged"));
    }

    #[test]
    fn invalid_loot_json_is_quarantined_before_new_writes() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_nanos();
        let root = std::env::temp_dir().join(format!(
            "moth-boss-corrupt-test-{}-{stamp}", std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("loot.json");
        let damaged = b"invalid boss journal, keep me";
        fs::write(&path, damaged).unwrap();
        let (loaded, warning) = load_log(&path).unwrap();
        assert!(loaded.is_empty());
        assert!(warning.unwrap_or_default().contains("preserved"));
        assert!(!path.exists());
        let quarantined = fs::read_dir(&root).unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|candidate| candidate != &path)
            .expect("damaged journal backup");
        assert_eq!(fs::read(&quarantined).unwrap().as_slice(), &damaged[..]);
        let fresh = BossLoot {
            at: 1, key: "bronze".into(), boss: "Bronze Boss".into(), drops: vec![],
        };
        save_log(&path, &[fresh.clone()]).unwrap();
        assert_eq!(load_log(&path).unwrap().0, vec![fresh]);
        assert_eq!(fs::read(&quarantined).unwrap(), damaged);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unreadable_journal_is_not_replaced_by_empty_history() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_nanos();
        let root = std::env::temp_dir().join(format!(
            "moth-boss-directory-test-{}-{stamp}", std::process::id()));
        let journal = root.join("journal.json");
        fs::create_dir_all(&journal).unwrap();
        assert!(load_log(&journal).is_err());
        assert!(journal.is_dir(), "the invalid journal path must not be renamed");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loot_uses_confirmed_boss_death_events_only() {
        let mut m = manager();
        m.ingest(&json!({"t":"batalha","ev":[{"k":"morte","drops":[{"itemId":3,"qtd":1}]}]}));
        assert!(m.view.log.is_empty());
        m.ingest(&json!({"t":"batalha","ev":[{"k":"bossMorto","key":"bronze",
            "nome":"Bronze Boss","drops":[{"itemId":70,"qtd":2}]}]}));
        assert_eq!(m.view.log[0].drops,vec![BossDrop {item_id:70,quantity:2}]);
    }
}
