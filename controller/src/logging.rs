//! Bounded, on-demand log rotation. Normal writes never spawn workers.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_LOCK: Mutex<()> = Mutex::new(());
static MAX_LOG_BYTES: AtomicU64 = AtomicU64::new(8 * 1024 * 1024);
static RETENTION_DAYS: AtomicU64 = AtomicU64::new(7);
static WRITE_COUNT: AtomicU64 = AtomicU64::new(0);

fn archive(path: &Path) -> PathBuf { path.with_extension("log.1") }

fn prune_archive(path: &Path) {
    let older_than = Duration::from_secs(RETENTION_DAYS.load(Ordering::Relaxed) * 24 * 60 * 60);
    let rotated = archive(path);
    if let Ok(metadata) = fs::metadata(&rotated) {
        if metadata.modified().ok().and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age >= older_than) {
            let _ = fs::remove_file(rotated);
        }
    }
}

/// A changed logging policy takes effect immediately, without a background
/// cleanup thread. Active logs are never automatically deleted.
pub fn set_limits(max_megabytes: u16, retention_days: u16) {
    MAX_LOG_BYTES.store(max_megabytes.clamp(4, 128) as u64 * 1024 * 1024, Ordering::Relaxed);
    RETENTION_DAYS.store(retention_days.clamp(1, 90) as u64, Ordering::Relaxed);
    if let Some(path) = LOG_PATH.get() {
        if let Ok(_guard) = LOG_LOCK.lock() { prune_archive(path); }
    }
}

pub fn init() {
    let path = crate::config::data_root()
        .unwrap_or_else(|_| PathBuf::from(".").join("Moth").join("PokeIdle"))
        .join("moth-controller.log");
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let _ = LOG_PATH.set(path);
    let preferences = crate::preferences::AppPreferences::load().unwrap_or_default();
    set_limits(preferences.log_max_megabytes, preferences.log_retention_days);
    info("logging initialized");
}

pub fn info(message: &str) { write("INFO", message); }
pub fn warn(message: &str) { write("WARN", message); }
pub fn error(message: &str) { write("ERROR", message); }

fn write(level: &str, message: &str) {
    let Some(path) = LOG_PATH.get() else { return; };
    let Ok(_guard) = LOG_LOCK.lock() else { return; };
    // Metadata checks are amortized; writes stay cheap even with active bots.
    if WRITE_COUNT.fetch_add(1, Ordering::Relaxed) % 64 == 0 {
        if fs::metadata(path).is_ok_and(|meta| meta.len() >= MAX_LOG_BYTES.load(Ordering::Relaxed)) {
            let rotated = archive(path);
            let _ = fs::remove_file(&rotated);
            let _ = fs::rename(path, &rotated);
        }
        prune_archive(path);
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_logs_use_only_one_archive_file() {
        let path = PathBuf::from("moth-controller.log");
        assert_eq!(archive(&path), PathBuf::from("moth-controller.log.1"));
    }
}
