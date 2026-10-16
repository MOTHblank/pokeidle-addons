use crate::accounts;
use serde_json::Value;
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::process::Command;
use std::sync::{mpsc::{self, Sender}, Arc, Mutex};
use std::thread;

const RELEASE_API_URL: &str =
    "https://api.github.com/repos/MOTHblank/pokeidle-addons/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/MOTHblank/pokeidle-addons/releases/latest";

#[derive(Debug, Default)]
pub struct UpdateStatus {
    pub latest_version: Option<String>,
}

pub type SharedUpdateStatus = Arc<Mutex<UpdateStatus>>;

/// Start a one-shot release check on a background thread. Network problems are
/// logged but never prevent the controller from starting.
pub fn start_update_check() -> SharedUpdateStatus {
    let status = Arc::new(Mutex::new(UpdateStatus::default()));
    let worker_status = Arc::clone(&status);

    let spawn_result = thread::Builder::new()
        .name("release-update-check".to_string())
        .spawn(move || match fetch_latest_version() {
            Ok(latest_version) => {
                crate::logging::info(&format!(
                    "GitHub release check completed; latest version is {latest_version}"
                ));
                if let Ok(mut state) = worker_status.lock() {
                    state.latest_version = Some(latest_version);
                }
            }
            Err(error) => {
                crate::logging::warn(&format!("GitHub release check failed: {error}"));
            }
        });

    if let Err(error) = spawn_result {
        crate::logging::warn(&format!("could not start release-check thread: {error}"));
    }

    status
}

fn fetch_latest_version() -> Result<String, String> {
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Keep the console-less GUI application from flashing a console window.
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = command
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "8",
            "--user-agent",
            "MothController-update-check",
            RELEASE_API_URL,
        ])
        .output()
        .map_err(|error| format!("could not run curl: {error}"))?;

    if !output.status.success() {
        return Err(format!("curl exited with status {}", output.status));
    }

    let response: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid GitHub release response: {error}"))?;
    let tag = response
        .get("tag_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "GitHub latest release did not contain a tag_name".to_string())?;

    Ok(tag.to_string())
}

/// Compare numeric version parts and basic prerelease labels. A shorter tag such
/// as 0.1 is treated as 0.1.0, so it matches Cargo's 0.1.0 package version.
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    compare_versions(latest, current) == Some(Ordering::Greater)
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    fn parse(version: &str) -> Option<(Vec<u64>, Option<&str>)> {
        let version = version
            .strip_prefix('v')
            .or_else(|| version.strip_prefix('V'))
            .unwrap_or(version);
        let version = version.split('+').next().unwrap_or(version);
        let (numeric, prerelease) = match version.split_once('-') {
            Some((numeric, prerelease)) => (numeric, Some(prerelease)),
            None => (version, None),
        };
        let parts = numeric
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if parts.is_empty() {
            return None;
        }
        Some((parts, prerelease))
    }

    let (left_parts, left_pre) = parse(left)?;
    let (right_parts, right_pre) = parse(right)?;
    let part_count = left_parts.len().max(right_parts.len());

    for index in 0..part_count {
        let left_part = left_parts.get(index).copied().unwrap_or(0);
        let right_part = right_parts.get(index).copied().unwrap_or(0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => {}
            ordering => return Some(ordering),
        }
    }

    Some(match (left_pre, right_pre) {
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left), Some(right)) => left.cmp(right),
        (None, None) => Ordering::Equal,
    })
}


/// Optional legacy userscript versions from the Premium archive.
/// This is a read-only status check; it never installs or replaces scripts.
#[derive(Debug, Clone, Default)]
pub struct AddonVersionStatus {
    pub checked: bool,
    pub checking: bool,
    pub versions: BTreeMap<String, String>,
    pub error: Option<String>,
}

pub type SharedAddonVersionStatus = Arc<Mutex<AddonVersionStatus>>;

/// Optional browser-script versions are checked only when explicitly requested
/// from the Accounts advanced panel. Never scan Premium on native startup.
/// All HTTP work stays off the UI thread.
pub fn start_addon_version_check() -> (SharedAddonVersionStatus, Sender<()>) {
    let status = Arc::new(Mutex::new(AddonVersionStatus::default()));
    let worker_status = Arc::clone(&status);
    let (tx, rx) = mpsc::channel::<()>();
    let spawn_result = thread::Builder::new()
        .name("addon-version-check".to_string())
        .spawn(move || {
            while rx.recv().is_ok() {
                // Repeated clicks while a scan is queued are coalesced.
                while rx.try_recv().is_ok() {}
                if let Ok(mut state) = worker_status.lock() {
                    state.checking = true;
                }
                let result = fetch_addon_versions();
                if let Ok(mut state) = worker_status.lock() {
                    state.checking = false;
                    match result {
                        Ok(versions) => {
                            state.versions = versions;
                            state.checked = true;
                            state.error = None;
                        }
                        Err(error) => {
                            crate::logging::warn(&format!("addon version check failed: {error}"));
                            state.checked = false;
                            state.error = Some(error);
                        }
                    }
                }
            }
        });
    if let Err(error) = spawn_result {
        crate::logging::warn(&format!("could not start addon version check: {error}"));
        if let Ok(mut state) = status.lock() {
            state.error = Some(error.to_string());
        }
    }
    (status, tx)
}

fn fetch_addon_versions() -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for file in accounts::discover_addon_files()? {
        let url = format!("https://raw.githubusercontent.com/MOTHblank/pokeidle-addons-premium/main/addons/{file}");
        let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        // Only headers are needed; GitHub may serve the complete file anyway.
        // All I/O and parsing occur on this worker, never on the egui thread.
        let output = command.args([
            "--fail", "--silent", "--show-error", "--location",
            "--range", "0-4095", "--max-time", "8",
            "--user-agent", "MothController-addon-versions", &url,
        ]).output().map_err(|error| format!("{file}: {error}"))?;
        if !output.status.success() {
            return Err(format!("{file}: curl exited with {}", output.status));
        }
        let version = parse_userscript_version(&output.stdout)
            .ok_or_else(|| format!("{file}: missing valid @version metadata"))?;
        result.insert(file, version);
    }
    Ok(result)
}

fn parse_userscript_version(contents: &[u8]) -> Option<String> {
    let header = String::from_utf8_lossy(contents);
    header.lines().take(50).find_map(|line| {
        let value = line.trim().strip_prefix("//")?
            .trim_start().strip_prefix("@version")?.trim();
        if value.is_empty() || value.len() > 40 ||
           !value.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-')) {
            return None;
        }
        Some(value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_userscript_version};

    #[test]
    fn reads_only_userscript_version_metadata() {
        assert_eq!(parse_userscript_version(b"// ==UserScript==\n// @name Demo\n// @version 1.2.3\n// ==/UserScript==").as_deref(), Some("1.2.3"));
        assert!(parse_userscript_version(b"// @version invalid version").is_none());
    }

    #[test]
    fn abbreviated_release_tag_matches_package_version() {
        assert!(!is_newer_version("0.1", "0.1.0"));
    }

    #[test]
    fn newer_release_is_detected() {
        assert!(is_newer_version("v0.2.0", "0.1.0"));
    }

    #[test]
    fn older_release_is_not_detected() {
        assert!(!is_newer_version("0.9.9", "1.0.0"));
    }

    #[test]
    fn stable_release_is_newer_than_same_prerelease() {
        assert!(is_newer_version("0.2.0", "0.2.0-rc.1"));
    }
}
