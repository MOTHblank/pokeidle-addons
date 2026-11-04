//! Application-wide preferences. Never stores profile credentials or tokens.
use crate::config::data_root;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppPreferences {
    pub language_pt_br: bool,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub confirm_exit_with_accounts: bool,
    pub launch_enabled_on_start: bool,
    pub start_with_windows: bool,
    pub ui_scale_percent: u8,
    pub compact_spacing: bool,
    pub reduce_motion: bool,
    pub notifications_enabled: bool,
    pub notify_auth: bool,
    pub notify_disconnect: bool,
    pub notify_automation: bool,
    pub notify_boss: bool,
    pub quiet_notifications: bool,
    pub check_updates_on_start: bool,
    pub log_max_megabytes: u16,
    pub log_retention_days: u16,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            language_pt_br: true,
            close_to_tray: false,
            start_minimized: false,
            confirm_exit_with_accounts: true,
            launch_enabled_on_start: false,
            start_with_windows: false,
            ui_scale_percent: 100,
            compact_spacing: false,
            reduce_motion: false,
            notifications_enabled: false,
            notify_auth: true,
            notify_disconnect: true,
            notify_automation: true,
            notify_boss: true,
            quiet_notifications: false,
            check_updates_on_start: true,
            log_max_megabytes: 8,
            log_retention_days: 7,
        }
    }
}

impl AppPreferences {
    pub fn from_value(value: &Value) -> Self {
        let defaults = Self::default();
        let boolean = |key, default| value.get(key).and_then(Value::as_bool).unwrap_or(default);
        Self {
            language_pt_br: boolean("languagePtBr", defaults.language_pt_br),
            close_to_tray: boolean("closeToTray", defaults.close_to_tray),
            start_minimized: boolean("startMinimized", defaults.start_minimized),
            confirm_exit_with_accounts: boolean("confirmExitWithAccounts", defaults.confirm_exit_with_accounts),
            launch_enabled_on_start: boolean("launchEnabledOnStart", defaults.launch_enabled_on_start),
            start_with_windows: boolean("startWithWindows", defaults.start_with_windows),
            ui_scale_percent: value.get("uiScalePercent").and_then(Value::as_u64)
                .filter(|value| (85..=150).contains(value))
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or(defaults.ui_scale_percent),
            compact_spacing: boolean("compactSpacing", defaults.compact_spacing),
            reduce_motion: boolean("reduceMotion", defaults.reduce_motion),
            notifications_enabled: boolean("notificationsEnabled", defaults.notifications_enabled),
            notify_auth: boolean("notifyAuth", defaults.notify_auth),
            notify_disconnect: boolean("notifyDisconnect", defaults.notify_disconnect),
            notify_automation: boolean("notifyAutomation", defaults.notify_automation),
            notify_boss: boolean("notifyBoss", defaults.notify_boss),
            quiet_notifications: boolean("quietNotifications", defaults.quiet_notifications),
            check_updates_on_start: boolean("checkUpdatesOnStart", defaults.check_updates_on_start),
            log_max_megabytes: value.get("logMaxMegabytes").and_then(Value::as_u64)
                .filter(|value| (4..=128).contains(value))
                .and_then(|value| u16::try_from(value).ok())
                .unwrap_or(defaults.log_max_megabytes),
            log_retention_days: value.get("logRetentionDays").and_then(Value::as_u64)
                .filter(|value| (1..=90).contains(value))
                .and_then(|value| u16::try_from(value).ok())
                .unwrap_or(defaults.log_retention_days),
        }
    }

    pub fn to_value(self) -> Value {
        json!({
            "version": 1,
            "languagePtBr": self.language_pt_br,
            "closeToTray": self.close_to_tray,
            "startMinimized": self.start_minimized,
            "confirmExitWithAccounts": self.confirm_exit_with_accounts,
            "launchEnabledOnStart": self.launch_enabled_on_start,
            "startWithWindows": self.start_with_windows,
            "uiScalePercent": self.ui_scale_percent,
            "compactSpacing": self.compact_spacing,
            "reduceMotion": self.reduce_motion,
            "notificationsEnabled": self.notifications_enabled,
            "notifyAuth": self.notify_auth,
            "notifyDisconnect": self.notify_disconnect,
            "notifyAutomation": self.notify_automation,
            "notifyBoss": self.notify_boss,
            "quietNotifications": self.quiet_notifications,
            "checkUpdatesOnStart": self.check_updates_on_start,
            "logMaxMegabytes": self.log_max_megabytes,
            "logRetentionDays": self.log_retention_days,
        })
    }

    pub fn path() -> Result<PathBuf, String> {
        Ok(data_root()?.join("application-options.json"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        if !path.exists() { return Ok(Self::default()); }
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read application options: {error}"))?;
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("invalid application options: {error}"))?;
        if !value.is_object() {
            return Err("invalid application options: expected object".into());
        }
        Ok(Self::from_value(&value))
    }

    pub fn save(self) -> Result<(), String> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create application options folder: {error}"))?;
        }
        let raw = serde_json::to_string_pretty(&self.to_value())
            .map_err(|error| format!("could not serialize application options: {error}"))?;
        let staging = path.with_extension("json.tmp");
        std::fs::write(&staging, raw)
            .map_err(|error| format!("could not stage application options: {error}"))?;
        std::fs::rename(&staging, &path).map_err(|error| {
            let _ = std::fs::remove_file(&staging);
            format!("could not save application options: {error}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_never_autostart_or_send_notifications() {
        let preferences = AppPreferences::default();
        assert!(!preferences.start_with_windows);
        assert!(!preferences.launch_enabled_on_start);
        assert!(!preferences.notifications_enabled);
        assert!(!preferences.close_to_tray);
        assert!(preferences.check_updates_on_start);
        assert_eq!(preferences.log_max_megabytes, 8);
        assert_eq!(preferences.log_retention_days, 7);
    }

    #[test]
    fn valid_preferences_round_trip_and_bad_ranges_fall_back() {
        let mut preferences = AppPreferences::default();
        preferences.start_minimized = true;
        preferences.ui_scale_percent = 125;
        preferences.log_max_megabytes = 64;
        preferences.notifications_enabled = true;
        assert_eq!(AppPreferences::from_value(&preferences.to_value()), preferences);
        let invalid = json!({"uiScalePercent": 250, "logMaxMegabytes": 0,
            "startWithWindows": "yes", "logRetentionDays": -1});
        assert_eq!(AppPreferences::from_value(&invalid), AppPreferences::default());
    }
}
