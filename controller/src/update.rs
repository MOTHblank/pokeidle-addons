use serde_json::Value;
use std::cmp::Ordering;
use std::process::Command;
use std::sync::{Arc, Mutex};
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
        .map_err(|error| format!("could not run curl.exe: {error}"))?;

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

#[cfg(test)]
mod tests {
    use super::is_newer_version;

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
