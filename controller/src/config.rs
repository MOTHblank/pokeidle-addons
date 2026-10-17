use std::env;
use std::path::PathBuf;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameProfile {
    Game1,
    Game2,
    Game3,
    Game4,
}

impl GameProfile {
    pub const ALL: [Self; 4] = [Self::Game1, Self::Game2, Self::Game3, Self::Game4];

    pub const fn index(self) -> usize {
        match self {
            Self::Game1 => 0,
            Self::Game2 => 1,
            Self::Game3 => 2,
            Self::Game4 => 3,
        }
    }

    pub const fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Game1),
            1 => Some(Self::Game2),
            2 => Some(Self::Game3),
            3 => Some(Self::Game4),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Game1 => "Game1",
            Self::Game2 => "Game2",
            Self::Game3 => "Game3",
            Self::Game4 => "Game4",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Game1 => "Game 1",
            Self::Game2 => "Game 2",
            Self::Game3 => "Game 3",
            Self::Game4 => "Game 4",
        }
    }

    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::Game1 => "AccountA",
            Self::Game2 => "AccountB",
            Self::Game3 => "AccountC",
            Self::Game4 => "AccountD",
        }
    }

    pub const fn default_enabled(self) -> bool {
        matches!(self, Self::Game1 | Self::Game2)
    }
}

#[derive(Debug, Clone)]
pub struct AccountConfig {
    pub name: String,
    pub enabled: bool,
}

impl AccountConfig {
    pub fn default_for(profile: GameProfile) -> Self {
        Self {
            name: profile.label().to_string(),
            enabled: profile.default_enabled(),
        }
    }
}

impl Config {
    pub fn accounts_path() -> Result<PathBuf, String> {
        Ok(data_root()?.join("accounts.json"))
    }

    pub fn load_accounts() -> Result<[AccountConfig; 4], String> {
        let defaults = std::array::from_fn(|i| AccountConfig::default_for(GameProfile::from_index(i).expect("valid account slot")));
        let path = Self::accounts_path()?;
        if !path.exists() {
            return Ok(defaults);
        }

        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read account configuration: {error}"))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("invalid account configuration: {error}"))?;

        let mut accounts = defaults;
        if let Some(entries) = value.get("accounts").and_then(Value::as_array) {
            for entry in entries {
                let Some(index) = entry.get("slot").and_then(Value::as_u64).and_then(|v| usize::try_from(v).ok()).filter(|v| *v < 4) else {
                    continue;
                };
                if let Some(name) = entry.get("name").and_then(Value::as_str) {
                    let name = name.trim();
                    if !name.is_empty() {
                        accounts[index].name = name.to_string();
                    }
                }
                if let Some(enabled) = entry.get("enabled").and_then(Value::as_bool) {
                    accounts[index].enabled = enabled;
                }
            }
        }
        Ok(accounts)
    }

    /// Public Kick channel choices only. API tokens are never stored here.
    pub fn kick_channels_path() -> Result<PathBuf, String> {
        Ok(data_root()?.join("kick-channels.json"))
    }

    pub fn load_kick_channels() -> Result<[String; 4], String> {
        let mut channels: [String; 4] = std::array::from_fn(|_| String::new());
        let path = Self::kick_channels_path()?;
        if !path.exists() { return Ok(channels); }
        let raw = std::fs::read_to_string(&path)
            .map_err(|_| "Could not read Kick channel settings".to_owned())?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|_| "Invalid Kick channel settings".to_owned())?;
        if let Some(entries) = value.get("accounts").and_then(Value::as_array) {
            for entry in entries {
                let Some(slot) = entry.get("slot").and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .filter(|value| *value < 4) else { continue; };
                if let Some(raw) = entry.get("channels").and_then(Value::as_str) {
                    if raw.len() <= 2048 { channels[slot] = raw.to_owned(); }
                }
            }
        }
        Ok(channels)
    }

    pub fn save_kick_channels(channels: &[String; 4]) -> Result<(), String> {
        for raw in channels {
            if raw.len() > 2048 { return Err("Kick channel list is too large".into()); }
            if !raw.trim().is_empty() {
                crate::native_kick::normalize_channels(raw)?;
            }
        }
        let path = Self::kick_channels_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| "Could not prepare Kick channel settings".to_owned())?;
        }
        let value = json!({
            "version": 1,
            "accounts": channels.iter().enumerate().map(|(slot, channels)| json!({
                "slot": slot, "channels": channels
            })).collect::<Vec<_>>()
        });
        let raw = serde_json::to_string_pretty(&value)
            .map_err(|_| "Could not serialize Kick channels".to_owned())?;
        save_config_atomic(&path, &raw, "Kick channel settings")
    }

    pub fn save_accounts(accounts: &[AccountConfig; 4]) -> Result<(), String> {
        let path = Self::accounts_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create account data directory: {error}"))?;
        }

        let value = json!({
            "version": 1,
            "accounts": accounts.iter().enumerate().map(|(index, account)| json!({
                "slot": index,
                "name": account.name,
                "enabled": account.enabled
            })).collect::<Vec<_>>()
        });
        let raw = serde_json::to_string_pretty(&value)
            .map_err(|error| format!("could not serialize account configuration: {error}"))?;
        save_config_atomic(&path, &raw, "account configuration")
    }
}

// A failed/interrupted write must not truncate the previously valid account
// or options JSON. Both files live on the same filesystem as their staging file.
fn save_config_atomic(path: &PathBuf, raw: &str, label: &str) -> Result<(), String> {
    let staging = path.with_extension("json.tmp");
    std::fs::write(&staging, raw)
        .map_err(|error| format!("could not stage {label}: {error}"))?;
    std::fs::rename(&staging, path).map_err(|error| {
        let _ = std::fs::remove_file(&staging);
        format!("could not save {label}: {error}")
    })
}

/// Local controller preferences, independent of browser profiles and account auth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiOptions {
    /// Keep native UI redraw and Firefox/BiDi polling at their foreground
    /// cadence while the controller window is visible but has lost focus.
    pub reduce_updates_unfocused: bool,
    /// Same choice for a window hidden in the system tray. Tray wins over
    /// focus, so the controls can be configured independently.
    pub reduce_updates_in_tray: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            reduce_updates_unfocused: true,
            reduce_updates_in_tray: true,
        }
    }
}

impl UiOptions {
    fn from_value(value: &Value) -> Self {
        let defaults = Self::default();
        Self {
            reduce_updates_unfocused: value.get("reduceUpdatesUnfocused")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.reduce_updates_unfocused),
            reduce_updates_in_tray: value.get("reduceUpdatesInTray")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.reduce_updates_in_tray),
        }
    }

    fn to_value(self) -> Value {
        json!({
            "version": 1,
            "reduceUpdatesUnfocused": self.reduce_updates_unfocused,
            "reduceUpdatesInTray": self.reduce_updates_in_tray,
        })
    }
}

impl Config {
    pub fn ui_options_path() -> Result<PathBuf, String> {
        Ok(data_root()?.join("controller-options.json"))
    }

    pub fn load_ui_options() -> Result<UiOptions, String> {
        let path = Self::ui_options_path()?;
        if !path.exists() {
            return Ok(UiOptions::default());
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read controller options: {error}"))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("invalid controller options: {error}"))?;
        if !value.is_object() {
            return Err("invalid controller options: expected JSON object".into());
        }
        Ok(UiOptions::from_value(&value))
    }

    pub fn save_ui_options(options: &UiOptions) -> Result<(), String> {
        let path = Self::ui_options_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create controller options directory: {error}"))?;
        }
        let raw = serde_json::to_string_pretty(&options.to_value())
            .map_err(|error| format!("could not serialize controller options: {error}"))?;
        save_config_atomic(&path, &raw, "controller options")
    }
}

#[cfg(test)]
mod ui_options_tests {
    use super::*;

    #[test]
    fn account_and_option_json_replaces_existing_file() {
        let directory = std::env::temp_dir().join(format!(
            "moth-config-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("controller-options.json");
        std::fs::write(&path, "old state").unwrap();
        save_config_atomic(&path, r#"{"reduceUpdatesUnfocused":false}"#, "test").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"reduceUpdatesUnfocused":false}"#
        );
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn defaults_preserve_existing_background_throttling() {
        let options = UiOptions::default();
        assert!(options.reduce_updates_unfocused);
        assert!(options.reduce_updates_in_tray);
    }

    #[test]
    fn stored_options_round_trip_and_partial_values_retain_defaults() {
        let options = UiOptions {
            reduce_updates_unfocused: false,
            reduce_updates_in_tray: false,
        };
        assert_eq!(UiOptions::from_value(&options.to_value()), options);
        assert_eq!(
            UiOptions::from_value(&json!({"reduceUpdatesUnfocused": false})),
            UiOptions { reduce_updates_unfocused: false, reduce_updates_in_tray: true }
        );
        assert_eq!(
            UiOptions::from_value(&json!({"reduceUpdatesInTray": false})),
            UiOptions { reduce_updates_unfocused: true, reduce_updates_in_tray: false }
        );
        assert_eq!(
            UiOptions::from_value(&json!({"reduceUpdatesUnfocused": "no"})),
            UiOptions::default()
        );
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub firefox_executable: PathBuf,
    pub profile: GameProfile,
    pub profile_dir: PathBuf,
    pub url: String,
    pub remote_debug_port: u16,
}

impl Config {
    pub fn for_profile(profile: GameProfile) -> Result<Self, String> {
        let firefox_executable = find_firefox().ok_or_else(|| {
            "Firefox was not found. Install Firefox or set MOTH_FIREFOX to firefox.exe."
                .to_string()
        })?;

        let profile_dir = data_root()?.join("Profiles").join(profile.name());

        let remote_debug_port = 27701 + profile.index() as u16;

        Ok(Self {
            firefox_executable,
            profile,
            profile_dir,
            url: "https://pokeidle.io/app".to_string(),
            remote_debug_port,
        })
    }

    pub fn legacy_profile_dir(&self) -> Result<PathBuf, String> {
        Ok(data_root()?.join("Profiles").join(self.profile.legacy_name()))
    }
}

pub(crate) fn data_root() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        return env::var_os("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("Moth").join("PokeIdle"))
            .ok_or_else(|| "could not determine the Windows local application data directory".to_string());
    }
    #[cfg(not(windows))]
    {
        // Respect XDG_DATA_HOME, otherwise use ~/.local/share. Profiles,
        // extension state and logs share the same durable app directory.
        let root = env::var_os("XDG_DATA_HOME")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .ok_or_else(|| "could not determine XDG_DATA_HOME or HOME".to_string())?;
        Ok(root.join("Moth").join("PokeIdle"))
    }
}

fn find_firefox() -> Option<PathBuf> {
    if let Some(path) = env::var_os("MOTH_FIREFOX") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    // Prefer the official Firefox Developer Edition when installed. Moth
    // supplies an explicit profile path, so changing the browser executable
    // does not move, recreate, or delete the existing Game1/Game2 profiles.
    firefox_developer_candidates()
        .into_iter()
        .chain(firefox_standard_candidates())
        .find(|path| path.is_file())
}

#[cfg(windows)]
fn firefox_developer_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for env_name in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
        if let Some(root) = env::var_os(env_name) {
            let root = PathBuf::from(root);
            for folder in [
                "Firefox Developer Edition",
                "Mozilla Firefox Developer Edition",
                "Firefox Developer",
            ] {
                paths.push(root.join(folder).join("firefox.exe"));
            }
        }
    }

    paths
}

#[cfg(windows)]
fn firefox_standard_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(program_files) = env::var_os("PROGRAMFILES") {
        paths.push(
            PathBuf::from(program_files)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    if let Some(program_files_x86) = env::var_os("PROGRAMFILES(X86)") {
        paths.push(
            PathBuf::from(program_files_x86)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        paths.push(
            PathBuf::from(local_app_data)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    paths
}

#[cfg(not(windows))]
fn firefox_developer_candidates() -> Vec<PathBuf> {
    firefox_from_path(&["firefox-developer-edition", "firefox-developer", "firefox-dev"])
}

#[cfg(not(windows))]
fn firefox_standard_candidates() -> Vec<PathBuf> {
    firefox_from_path(&["firefox", "firefox-bin"])
}

#[cfg(not(windows))]
fn firefox_from_path(executables: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path_var) = env::var_os("PATH") {
        for directory in env::split_paths(&path_var) {
            for executable in executables {
                candidates.push(directory.join(executable));
            }
        }
    }
    candidates
}


