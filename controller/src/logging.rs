use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub fn init() {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Moth")
        .join("PokeIdle")
        .join("moth-controller.log");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = LOG_PATH.set(path);
    info("logging initialized");
}

pub fn info(message: &str) { write("INFO", message); }
pub fn warn(message: &str) { write("WARN", message); }
pub fn error(message: &str) { write("ERROR", message); }

fn write(level: &str, message: &str) {
    let Some(path) = LOG_PATH.get() else { return; };
    let Ok(_guard) = LOG_LOCK.lock() else { return; };
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message);
    }
}
