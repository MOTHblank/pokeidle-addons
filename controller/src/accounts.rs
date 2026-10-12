use crate::config::{Config, GameProfile};
use crate::firefox;
use serde_json::Value;
use std::process::Command;

pub const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
pub const VIOLENTMONKEY_INSTALL: &str =
    "https://addons.mozilla.org/firefox/addon/violentmonkey/";

const ADDONS_API: &str =
    "https://api.github.com/repos/MOTHblank/pokeidle-addons/contents/addons?ref=master";

/// Discover the active addon scripts directly from GitHub. No versions or
/// addon filenames are pinned in the native binary.
pub(crate) fn discover_addon_files() -> Result<Vec<String>, String> {
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = command.args([
        "--fail", "--silent", "--show-error", "--location",
        "--max-time", "12", "--user-agent", "MothController-addons",
        "-H", "Accept: application/vnd.github+json", ADDONS_API,
    ]).output().map_err(|error| format!("could not list addons from GitHub: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub addon list unavailable (curl exited with {})",
            output.status
        ));
    }
    let response: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid GitHub addon list response: {error}"))?;
    parse_addon_files(&response)
}

fn parse_addon_files(value: &Value) -> Result<Vec<String>, String> {
    let entries = value.as_array().ok_or_else(|| "GitHub addon list was not an array".to_string())?;
    let mut files: Vec<String> = entries.iter().filter_map(|entry| {
        if entry.get("type").and_then(Value::as_str) != Some("file") {
            return None;
        }
        let name = entry.get("name").and_then(Value::as_str)?;
        let path = entry.get("path").and_then(Value::as_str)?;
        // Trust only single-file userscripts in this repository's addon directory.
        if !name.ends_with(".user.js") || path != format!("addons/{name}")
            || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return None;
        }
        Some(name.to_string())
    }).collect();
    files.sort_unstable();
    files.dedup();
    if files.is_empty() {
        return Err("GitHub returned no installable userscripts".to_string());
    }
    Ok(files)
}

fn installer_url(file: &str, nonce: u128) -> String {
    format!(
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/{file}?moth_install={nonce}"
    )
}

pub fn violentmonkey_installed(profile: GameProfile) -> Result<bool, String> {
    let config = Config::for_profile(profile)?;
    let extensions = config.profile_dir.join("extensions.json");

    if !extensions.is_file() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(&extensions)
        .map_err(|error| format!("could not read Firefox extensions registry: {error}"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid Firefox extensions registry: {error}"))?;

    let Some(addons) = value.get("addons").and_then(Value::as_array) else {
        return Ok(false);
    };

    Ok(addons.iter().any(|addon| {
        let name = addon
            .get("defaultLocale")
            .and_then(|locale| locale.get("name"))
            .and_then(Value::as_str)
            .or_else(|| addon.get("name").and_then(Value::as_str))
            .unwrap_or_default();

        let id = addon
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let active = addon
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        active
            && (name.to_ascii_lowercase().contains("violentmonkey")
                || id.to_ascii_lowercase().contains("violentmonkey"))
    }))
}

pub fn open_violentmonkey(profile: GameProfile) -> Result<String, String> {
    let config = Config::for_profile(profile)?;
    firefox::open_url(&config, VIOLENTMONKEY_INSTALL)?;
    Ok(format!("Violentmonkey installer opened in {}.", profile.label()))
}

pub fn open_login(profile: GameProfile, url: &str, service: &str) -> Result<String, String> {
    let config = Config::for_profile(profile)?;
    firefox::open_url(&config, url)?;
    Ok(format!("{service} opened in {}.", profile.label()))
}

/// Opens installers for all addons on first setup, or only explicitly selected
/// scripts when repairing a profile. Opening a tab does not imply installation.
pub fn open_addons(
    profile: GameProfile,
    requested: Option<&[String]>,
    mut progress: impl FnMut(String),
) -> Result<usize, String> {
    if !violentmonkey_installed(profile)? {
        return Err(format!(
            "Violentmonkey is not installed and active in {}. Use \"Install Violentmonkey\" first.",
            profile.label()
        ));
    }

    let files = select_addon_files(discover_addon_files()?, requested)?;
    let config = Config::for_profile(profile)?;
    let mut opened = 0usize;
    let mut failures = Vec::new();

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("could not generate installer cache key: {error}"))?
        .as_nanos();
    for file in files {
        let url = installer_url(&file, nonce);
        match firefox::open_url(&config, &url) {
            Ok(()) => {
                opened += 1;
                progress(format!("{file}: installer opened in Firefox (confirm in Violentmonkey)"));
            }
            Err(error) => {
                let message = format!("{file}: {error}");
                progress(message.clone());
                failures.push(message);
            }
        }
    }

    if failures.is_empty() {
        Ok(opened)
    } else {
        Err(format!(
            "Opened {opened} addon installers, but some failed:\n{}",
            failures.join("\n")
        ))
    }
}

/// Selected names must be present in the live repository list. Do not let UI
/// input turn into arbitrary URLs, and do not silently install every script.
fn select_addon_files(
    available: Vec<String>,
    requested: Option<&[String]>,
) -> Result<Vec<String>, String> {
    let Some(requested) = requested else { return Ok(available); };
    if requested.is_empty() {
        return Err("Select at least one addon to repair.".to_string());
    }
    let mut selected = Vec::new();
    for file in requested {
        if !available.contains(file) {
            return Err(format!("Addon no longer published: {file}"));
        }
        if !selected.contains(file) {
            selected.push(file.clone());
        }
    }
    Ok(selected)
}

const SETUP_MARKER: &str = ".moth-setup-complete";

/// This marker records only that the user completed initial onboarding.
pub fn setup_complete(profile: GameProfile) -> Result<bool, String> {
    Ok(Config::for_profile(profile)?.profile_dir.join(SETUP_MARKER).is_file())
}

pub fn complete_setup(profile: GameProfile) -> Result<(), String> {
    let path = Config::for_profile(profile)?.profile_dir;
    std::fs::create_dir_all(&path).map_err(|e| format!("Could not save setup progress: {e}"))?;
    std::fs::write(path.join(SETUP_MARKER), b"1\n")
        .map_err(|e| format!("Could not save setup progress: {e}"))
}

pub fn open_profile_folder(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    std::fs::create_dir_all(&config.profile_dir)
        .map_err(|e| format!("could not create profile directory: {e}"))?;

    std::process::Command::new(if cfg!(windows) { "explorer.exe" } else { "xdg-open" })
        .arg(&config.profile_dir)
        .spawn()
        .map_err(|e| format!("could not open profile folder: {e}"))?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{installer_url, parse_addon_files, select_addon_files};
    use serde_json::json;
    #[test]
    fn discovers_only_userscripts_and_never_uses_pinned_versions() {
        let listing = json!([
            {"type":"file","name":"autocatch.user.js","path":"addons/autocatch.user.js"},
            {"type":"file","name":"autocatch.user.js","path":"addons/autocatch.user.js"},
            {"type":"file","name":"README.md","path":"addons/README.md"},
            {"type":"file","name":"bad.user.js","path":"other/bad.user.js"},
            {"type":"dir","name":"fake.user.js","path":"addons/fake.user.js"}
        ]);
        assert_eq!(parse_addon_files(&listing).unwrap(), vec!["autocatch.user.js"]);
        let url = installer_url("autocatch.user.js", 123);
        assert!(url.ends_with("?moth_install=123"));
        assert!(!url.contains("?v="));
    }
    #[test]
    fn repairs_only_selected_addons() {
        let available = vec!["one.user.js".to_string(), "two.user.js".to_string()];
        let requested = vec!["two.user.js".to_string(), "two.user.js".to_string()];
        assert_eq!(select_addon_files(available.clone(), Some(&requested)).unwrap(), vec!["two.user.js"]);
        assert_eq!(select_addon_files(available.clone(), None).unwrap(), available);
        assert!(select_addon_files(available.clone(), Some(&[])).is_err());
        assert!(select_addon_files(available, Some(&["unknown.user.js".to_string()])).is_err());
    }

    #[test]
    fn empty_addon_list_is_an_error() {
        assert!(parse_addon_files(&json!([])).is_err());
    }
}
