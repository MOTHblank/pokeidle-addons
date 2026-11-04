//! Explicit, credential-free local settings backup. No vaults, cookies,
//! server automation rules, histories, or arbitrary files are ever serialized.
use crate::config::{data_root, AccountConfig, Config, KickStreamPreferences, UiOptions};
use crate::preferences::AppPreferences;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct KickBackup {
    pub channels: [String; 4],
    pub preferences: [KickStreamPreferences; 4],
}

#[derive(Clone, Debug)]
pub struct SettingsBackup {
    pub app: AppPreferences,
    pub ui: UiOptions,
    pub accounts: [AccountConfig; 4],
    /// None for older safe settings backups; then existing Kick choices stay.
    pub kick: Option<KickBackup>,
}

impl SettingsBackup {
    pub fn path() -> Result<PathBuf, String> {
        Ok(data_root()?.join("moth-settings-backup.json"))
    }

    pub fn from_active(app: AppPreferences, ui: UiOptions, accounts: &[AccountConfig; 4]) -> Self {
        Self { app, ui, accounts: (*accounts).clone(), kick: None }
    }

    /// Capture persisted local Kick choices, never browser cookies or tokens.
    /// Called only when explicitly exporting or preparing a restore rollback.
    pub fn with_saved_kick(mut self) -> Result<Self, String> {
        self.kick = Some(KickBackup {
            channels: Config::load_kick_channels()?,
            preferences: Config::load_kick_stream_preferences()?,
        });
        Ok(self)
    }

    fn to_value(&self) -> Value {
        json!({
            "format": "MothControllerSafeSettings",
            "version": 1,
            "application": self.app.to_value(),
            "controller": self.ui.to_value(),
            "kick": self.kick.as_ref().map(|kick| json!({
                "accounts": kick.channels.iter().zip(kick.preferences.iter())
                    .enumerate().map(|(slot, (channels, pref))| json!({
                        "slot": slot, "channels": channels,
                        "max_streams": pref.max_streams, "favorites": pref.favorites,
                    })).collect::<Vec<_>>()
            })),
            "accounts": self.accounts.iter().enumerate().map(|(slot, account)| {
                json!({"slot": slot, "name": account.name, "enabled": account.enabled,
                    "autoPvp": account.auto_pvp})
            }).collect::<Vec<_>>(),
        })
    }

    fn from_value(value: &Value) -> Result<Self, String> {
        if value.get("format").and_then(Value::as_str) != Some("MothControllerSafeSettings")
            || value.get("version").and_then(Value::as_u64) != Some(1) {
            return Err("Unsupported or invalid settings backup format".into());
        }
        let app = value.get("application").filter(|v| v.is_object())
            .ok_or("Backup has no application settings")?;
        let ui = value.get("controller").filter(|v| v.is_object())
            .ok_or("Backup has no controller settings")?;
        let entries = value.get("accounts").and_then(Value::as_array)
            .filter(|entries| entries.len() == 4).ok_or("Backup must contain four account slots")?;
        let mut accounts = std::array::from_fn(|index|
            AccountConfig::default_for(crate::config::GameProfile::from_index(index)
                .expect("valid profile")));
        let mut slots_seen = [false; 4];
        for entry in entries {
            let slot = entry.get("slot").and_then(Value::as_u64)
                .and_then(|v| usize::try_from(v).ok())
                .filter(|v| *v < 4).ok_or("Invalid account slot in backup")?;
            if slots_seen[slot] { return Err("Duplicate account slot in backup".into()); }
            let name = entry.get("name").and_then(Value::as_str)
                .map(str::trim).filter(|name| !name.is_empty() && name.chars().count() <= 80)
                .ok_or("Invalid account name in backup")?;
            let enabled = entry.get("enabled").and_then(Value::as_bool)
                .ok_or("Invalid account enabled state in backup")?;
            let auto_pvp = entry.get("autoPvp").and_then(Value::as_bool);
            accounts[slot] = AccountConfig { name: name.to_owned(), enabled, auto_pvp };
            slots_seen[slot] = true;
        }
        if slots_seen.iter().any(|seen| !seen) {
            return Err("Missing account slot in backup".into());
        }
        let kick = match value.get("kick").filter(|value| !value.is_null()) {
            None => None,
            Some(raw) => {
                let entries = raw.get("accounts").and_then(Value::as_array)
                    .filter(|entries| entries.len() == 4)
                    .ok_or("Kick backup must contain four account slots")?;
                let mut channels: [String; 4] = std::array::from_fn(|_| String::new());
                let mut preferences: [KickStreamPreferences; 4] =
                    std::array::from_fn(|_| KickStreamPreferences::default());
                let mut seen = [false; 4];
                for entry in entries {
                    let slot = entry.get("slot").and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .filter(|value| *value < 4)
                        .ok_or("Invalid Kick account slot")?;
                    if seen[slot] { return Err("Duplicate Kick account slot".into()); }
                    seen[slot] = true;
                    let channel_list = entry.get("channels").and_then(Value::as_str)
                        .filter(|text| text.len() <= 2048)
                        .ok_or("Invalid Kick channel list")?;
                    if !channel_list.trim().is_empty() {
                        crate::native_kick::normalize_channels(channel_list)?;
                    }
                    let limit = entry.get("max_streams").and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .filter(|value| *value <= 12)
                        .ok_or("Invalid Kick stream limit")?;
                    let favorites = entry.get("favorites").and_then(Value::as_array)
                        .filter(|items| items.len() <= 128)
                        .ok_or("Invalid Kick favorites")?;
                    let mut normalized = Vec::with_capacity(favorites.len());
                    for favorite in favorites {
                        let name = favorite.as_str().ok_or("Invalid Kick favorite")?;
                        let parsed = crate::native_kick::normalize_channels(name)?;
                        if parsed.len() != 1 || parsed[0] != name || normalized.iter()
                            .any(|previous: &String| previous == name) {
                            return Err("Invalid or repeated Kick favorite".into());
                        }
                        normalized.push(name.to_string());
                    }
                    channels[slot] = channel_list.to_string();
                    preferences[slot] = KickStreamPreferences {
                        max_streams: limit, favorites: normalized
                    };
                }
                if seen.iter().any(|seen| !seen) {
                    return Err("Missing Kick account slot".into());
                }
                Some(KickBackup { channels, preferences })
            }
        };
        Ok(Self {
            app: AppPreferences::from_value(app),
            ui: UiOptions::from_value(ui), accounts, kick
        })
    }

    pub fn export(&self) -> Result<PathBuf, String> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create backup folder: {error}"))?;
        }
        let raw = serde_json::to_string_pretty(&self.to_value())
            .map_err(|error| format!("could not serialize backup: {error}"))?;
        let staging = path.with_extension("json.tmp");
        std::fs::write(&staging, raw)
            .map_err(|error| format!("could not stage backup: {error}"))?;
        std::fs::rename(&staging, &path)
            .map_err(|error| format!("could not save backup: {error}"))?;
        Ok(path)
    }

    pub fn load_preview() -> Result<Self, String> {
        let path = Self::path()?;
        let size = std::fs::metadata(&path)
            .map_err(|error| format!("could not inspect backup: {error}"))?.len();
        if size > 256 * 1024 { return Err("Settings backup exceeds 256 KiB".into()); }
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read backup: {error}"))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("invalid settings backup: {error}"))?;
        Self::from_value(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_contains_only_allowlisted_settings_and_account_names() {
        let mut accounts = std::array::from_fn(|i| AccountConfig::default_for(
            crate::config::GameProfile::from_index(i).unwrap()));
        accounts[0].auto_pvp = Some(true);
        accounts[1].auto_pvp = Some(false);
        let mut backup = SettingsBackup::from_active(AppPreferences::default(), UiOptions::default(), &accounts);
        backup.kick = Some(KickBackup {
            channels: ["pokemon".into(), String::new(), String::new(), String::new()],
            preferences: std::array::from_fn(|_| KickStreamPreferences::default()),
        });
        let value = backup.to_value();
        let restored = SettingsBackup::from_value(&value).unwrap();
        assert_eq!(restored.accounts[0].name, accounts[0].name);
        assert_eq!(restored.accounts[0].auto_pvp, Some(true));
        assert_eq!(restored.accounts[1].auto_pvp, Some(false));
        assert_eq!(restored.accounts[2].auto_pvp, None);
        assert_eq!(restored.kick.unwrap().channels[0], "pokemon");
        let mut legacy = value.clone();
        legacy["accounts"][0].as_object_mut().unwrap().remove("autoPvp");
        assert_eq!(SettingsBackup::from_value(&legacy).unwrap().accounts[0].auto_pvp, None);
        let raw = value.to_string();
        assert!(!raw.contains("token"));
        assert!(!raw.contains("password"));
        assert!(!raw.contains("cookies"));
        assert!(!raw.contains("credentials"));
    }

    #[test]
    fn invalid_backups_are_rejected_before_any_write() {
        let accounts = std::array::from_fn(|i| AccountConfig::default_for(
            crate::config::GameProfile::from_index(i).unwrap()));
        let mut value = SettingsBackup::from_active(
            AppPreferences::default(), UiOptions::default(), &accounts).to_value();
        value["accounts"][1]["slot"] = json!(0);
        assert!(SettingsBackup::from_value(&value).is_err());
        value["format"] = json!("Unknown");
        assert!(SettingsBackup::from_value(&value).is_err());
    }

    #[test]
    fn older_backup_keeps_kick_settings_unchanged() {
        let accounts = std::array::from_fn(|i| AccountConfig::default_for(
            crate::config::GameProfile::from_index(i).unwrap()));
        let backup = SettingsBackup::from_active(
            AppPreferences::default(), UiOptions::default(), &accounts);
        let mut value = backup.to_value();
        value.as_object_mut().unwrap().remove("kick");
        assert!(SettingsBackup::from_value(&value).unwrap().kick.is_none());
    }

    #[test]
    fn malformed_kick_settings_are_rejected() {
        let accounts = std::array::from_fn(|i| AccountConfig::default_for(
            crate::config::GameProfile::from_index(i).unwrap()));
        let mut backup = SettingsBackup::from_active(
            AppPreferences::default(), UiOptions::default(), &accounts);
        backup.kick = Some(KickBackup {
            channels: std::array::from_fn(|_| String::new()),
            preferences: std::array::from_fn(|_| KickStreamPreferences::default()),
        });
        let mut value = backup.to_value();
        value["kick"]["accounts"][0]["max_streams"] = json!(99);
        assert!(SettingsBackup::from_value(&value).is_err());
        value["kick"]["accounts"][0]["max_streams"] = json!(3);
        value["kick"]["accounts"][2]["slot"] = json!(0);
        assert!(SettingsBackup::from_value(&value).is_err());
    }
}
