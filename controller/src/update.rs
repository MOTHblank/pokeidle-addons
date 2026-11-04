use serde_json::Value;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const RELEASE_API_URL: &str =
    "https://api.github.com/repos/MOTHblank/pokeidle-addons/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/MOTHblank/pokeidle-addons/releases/latest";
#[cfg(windows)]
const WINDOWS_UPDATE_ASSET: &str = "Moth-Controller-Portable-windows-x86_64.exe";

#[derive(Clone, Debug)]
struct ReleaseInfo {
    tag: String,
    notes_url: String,
    #[cfg(windows)]
    asset_url: Option<String>,
    #[cfg(windows)]
    checksum_url: Option<String>,
}

#[derive(Debug, Default)]
pub struct UpdateStatus {
    pub latest_version: Option<String>,
    pub release_notes_url: Option<String>,
    pub checking: bool,
    pub downloading: bool,
    pub staged_version: Option<String>,
    pub last_error: Option<String>,
    release: Option<ReleaseInfo>,
    #[cfg(windows)]
    staged_path: Option<PathBuf>,
    #[cfg(windows)]
    staged_sha256: Option<String>,
}

pub type SharedUpdateStatus = Arc<Mutex<UpdateStatus>>;

/// There is no polling timer: checks run once at startup if enabled, or
/// explicitly on demand. No worker is created when checks are disabled.
pub fn start_update_check(enabled: bool) -> SharedUpdateStatus {
    let status = Arc::new(Mutex::new(UpdateStatus::default()));
    if enabled {
        let _ = check_now(&status);
    }
    status
}

pub fn check_now(status: &SharedUpdateStatus) -> Result<bool, String> {
    let mut state = status.lock().map_err(|_| "update status lock poisoned")?;
    if state.checking {
        return Ok(false);
    }
    state.checking = true;
    state.last_error = None;

    let worker_status = Arc::clone(status);
    let spawn = thread::Builder::new()
        .name("release-update-check".into())
        .spawn(move || {
            let result = fetch_latest_release();
            if let Ok(mut state) = worker_status.lock() {
                state.checking = false;
                match result {
                    Ok(release) => {
                        crate::logging::info(&format!(
                            "GitHub release check completed; latest version is {}",
                            release.tag
                        ));
                        if state.staged_version.as_deref() != Some(release.tag.as_str()) {
                            #[cfg(windows)]
                            if let Some(path) = state.staged_path.take() {
                                let _ = std::fs::remove_file(path);
                            }
                            #[cfg(windows)]
                            {
                                state.staged_sha256 = None;
                            }
                            state.staged_version = None;
                        }
                        state.latest_version = Some(release.tag.clone());
                        state.release_notes_url = Some(release.notes_url.clone());
                        state.release = Some(release);
                        state.last_error = None;
                    }
                    Err(error) => {
                        crate::logging::warn(&format!("GitHub release check failed: {error}"));
                        state.last_error = Some(error);
                    }
                }
            }
        });
    if let Err(error) = spawn {
        state.checking = false;
        state.last_error = Some(format!("could not start release-check thread: {error}"));
        return Err(state.last_error.clone().unwrap_or_default());
    }
    Ok(true)
}

fn http_client(timeout: Duration) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent("MothController-updater")
        .build()
        .map_err(|error| format!("could not initialize update client: {error}"))
}

fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    let client = http_client(Duration::from_secs(10))?;
    let response: Value = client
        .get(RELEASE_API_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("GitHub release request failed: {error}"))?
        .json()
        .map_err(|error| format!("invalid GitHub release response: {error}"))?;

    let tag = response
        .get("tag_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "GitHub latest release did not contain a tag_name".to_string())?
        .to_string();
    let notes_url = response
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with("https://github.com/"))
        .unwrap_or(RELEASES_URL)
        .to_string();

    #[cfg(windows)]
    let (asset_url, checksum_url) = {
        let assets = response
            .get("assets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let find = |name: &str| {
            assets.iter().find_map(|asset| {
                (asset.get("name").and_then(Value::as_str) == Some(name))
                    .then(|| asset.get("browser_download_url").and_then(Value::as_str))
                    .flatten()
                    .filter(|url| url.starts_with("https://github.com/"))
                    .map(str::to_string)
            })
        };
        let checksum_name = format!("{WINDOWS_UPDATE_ASSET}.sha256");
        (find(WINDOWS_UPDATE_ASSET), find(&checksum_name))
    };

    Ok(ReleaseInfo {
        tag,
        notes_url,
        #[cfg(windows)]
        asset_url,
        #[cfg(windows)]
        checksum_url,
    })
}

#[cfg(windows)]
pub fn download_update(status: &SharedUpdateStatus) -> Result<bool, String> {
    let release = {
        let mut state = status.lock().map_err(|_| "update status lock poisoned")?;
        if state.downloading {
            return Ok(false);
        }
        let release = state
            .release
            .clone()
            .ok_or_else(|| "Check for updates before downloading.".to_string())?;
        if !is_newer_version(&release.tag, env!("CARGO_PKG_VERSION")) {
            return Err("Moth Controller is already up to date.".into());
        }
        if release.asset_url.is_none() || release.checksum_url.is_none() {
            return Err("This release does not include an in-app Windows update package.".into());
        }
        state.downloading = true;
        state.last_error = None;
        release
    };

    let worker_status = Arc::clone(status);
    let spawn = thread::Builder::new()
        .name("release-update-download".into())
        .spawn(move || {
            let result = download_release(&release);
            if let Ok(mut state) = worker_status.lock() {
                state.downloading = false;
                match result {
                    Ok((path, checksum)) => {
                        crate::logging::info(&format!(
                            "Update {} downloaded and verified at {}",
                            release.tag,
                            path.display()
                        ));
                        state.staged_version = Some(release.tag.clone());
                        state.staged_path = Some(path);
                        state.staged_sha256 = Some(checksum);
                        state.last_error = None;
                    }
                    Err(error) => {
                        crate::logging::warn(&format!("Update download failed: {error}"));
                        state.last_error = Some(error);
                    }
                }
            }
        });

    if let Err(error) = spawn {
        if let Ok(mut state) = status.lock() {
            state.downloading = false;
            state.last_error = Some(format!("could not start update download: {error}"));
        }
        return Err(format!("could not start update download: {error}"));
    }
    Ok(true)
}

#[cfg(not(windows))]
pub fn download_update(_status: &SharedUpdateStatus) -> Result<bool, String> {
    Err("In-app installation is currently available on Windows only.".into())
}

#[cfg(windows)]
fn cleanup_old_update_files(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else { continue; };
        let is_update = name.starts_with("moth-controller-") &&
            (name.ends_with("-update.exe") || name.ends_with("-update.exe.download"));
        if is_update {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(windows)]
fn download_release(release: &ReleaseInfo) -> Result<(PathBuf, String), String> {
    let asset_url = release.asset_url.as_deref().ok_or("missing update asset")?;
    let checksum_url = release
        .checksum_url
        .as_deref()
        .ok_or("missing update checksum")?;
    let directory = crate::config::data_root()?.join("Updates");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create update staging folder: {error}"))?;
    // A Windows helper cannot delete its own running image after a successful
    // replacement. Remove leftovers from previous updates before staging a new one.
    cleanup_old_update_files(&directory);

    let safe_tag = release
        .tag
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
        .collect::<String>();
    let final_path = directory.join(format!("moth-controller-{safe_tag}-update.exe"));
    let temporary_path = final_path.with_extension("exe.download");

    let client = http_client(Duration::from_secs(90))?;
    let checksum_text = client
        .get(checksum_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("could not download update checksum: {error}"))?
        .text()
        .map_err(|error| format!("could not read update checksum: {error}"))?;
    let expected = parse_sha256(&checksum_text)?;

    let mut response = client
        .get(asset_url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("could not download update: {error}"))?;
    let mut file = std::fs::File::create(&temporary_path)
        .map_err(|error| format!("could not stage update: {error}"))?;
    if let Err(error) = std::io::copy(&mut response, &mut file) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(format!("could not save update: {error}"));
    }
    drop(file);

    let actual = windows_sha256(&temporary_path)?;
    if actual != expected {
        let _ = std::fs::remove_file(&temporary_path);
        return Err("Downloaded update failed SHA-256 verification.".into());
    }
    if final_path.exists() {
        std::fs::remove_file(&final_path)
            .map_err(|error| format!("could not replace staged update: {error}"))?;
    }
    std::fs::rename(&temporary_path, &final_path)
        .map_err(|error| format!("could not finish staging update: {error}"))?;
    Ok((final_path, expected))
}

#[cfg(windows)]
fn parse_sha256(value: &str) -> Result<String, String> {
    value
        .split_whitespace()
        .map(|part| part.trim().to_ascii_lowercase())
        .find(|part| part.len() == 64 && part.chars().all(|value| value.is_ascii_hexdigit()))
        .ok_or_else(|| "Update checksum file did not contain a SHA-256 digest.".to_string())
}

#[cfg(windows)]
fn windows_sha256(path: &Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("certutil.exe");
    command.creation_flags(0x08000000);
    let output = command
        .arg("-hashfile")
        .arg(path)
        .arg("SHA256")
        .output()
        .map_err(|error| format!("could not run Windows SHA-256 verification: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Windows SHA-256 verification failed with status {}",
            output.status
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(|line| line.split_whitespace().collect::<String>().to_ascii_lowercase())
        .find(|line| line.len() == 64 && line.chars().all(|value| value.is_ascii_hexdigit()))
        .ok_or_else(|| "Windows did not return a readable SHA-256 digest.".to_string())
}

#[cfg(windows)]
fn ensure_target_directory_writable(target: &Path) -> Result<(), String> {
    let directory = target
        .parent()
        .ok_or_else(|| "could not determine application folder".to_string())?;
    let probe = directory.join(format!(".moth-update-write-test-{}", std::process::id()));
    std::fs::write(&probe, b"moth")
        .map_err(|error| format!("Moth cannot update this folder: {error}"))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}

#[cfg(windows)]
pub fn request_restart_and_install(status: &SharedUpdateStatus) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let (staged, checksum, version) = {
        let state = status.lock().map_err(|_| "update status lock poisoned")?;
        (
            state
                .staged_path
                .clone()
                .ok_or_else(|| "Download the update before restarting.".to_string())?,
            state
                .staged_sha256
                .clone()
                .ok_or_else(|| "Staged update checksum is unavailable.".to_string())?,
            state
                .staged_version
                .clone()
                .ok_or_else(|| "Staged update version is unavailable.".to_string())?,
        )
    };
    if windows_sha256(&staged)? != checksum {
        return Err("Staged update failed SHA-256 verification.".into());
    }

    let target = std::env::current_exe()
        .map_err(|error| format!("could not locate Moth Controller executable: {error}"))?;
    ensure_target_directory_writable(&target)?;

    let mut helper = Command::new(&staged);
    helper.creation_flags(0x08000000);
    helper
        .arg("--apply-update")
        .arg(&target)
        .arg(&checksum)
        .arg(std::process::id().to_string());
    helper
        .spawn()
        .map_err(|error| format!("could not start update helper: {error}"))?;
    Ok(version)
}

#[cfg(not(windows))]
pub fn request_restart_and_install(_status: &SharedUpdateStatus) -> Result<String, String> {
    Err("In-app installation is currently available on Windows only.".into())
}

/// Returns true when this process was launched as the short-lived Windows
/// replacement helper. This check must run before single-instance acquisition.
pub fn maybe_run_update_helper() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return false;
    }

    #[cfg(windows)]
    {
        let target = args.next().map(PathBuf::from);
        let checksum = args.next().and_then(|value| value.into_string().ok());
        let parent_pid = args
            .next()
            .and_then(|value| value.into_string().ok())
            .and_then(|value| value.parse::<u32>().ok());
        let result = match (target.as_ref(), checksum.as_deref(), parent_pid) {
            (Some(target), Some(checksum), Some(parent_pid)) => {
                run_update_helper(target, checksum, parent_pid)
            }
            _ => Err("invalid update helper arguments".into()),
        };
        if let Err(error) = result {
            write_helper_error(&error);
            // A failed update must not strand the user with Moth closed.
            if let Some(target) = target.as_ref().filter(|path| path.exists()) {
                let _ = Command::new(target).spawn();
            }
        }
    }
    true
}

#[cfg(windows)]
fn parent_process_alive(parent_pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("tasklist.exe");
    command.creation_flags(0x08000000);
    let output = command
        .arg("/FI")
        .arg(format!("PID eq {parent_pid}"))
        .args(["/FO", "CSV", "/NH"])
        .output();
    let Ok(output) = output else {
        // If tasklist itself is unavailable, the file replacement loop below
        // remains the final authority on whether the old image is released.
        return true;
    };
    String::from_utf8_lossy(&output.stdout).contains(&format!("\"{parent_pid}\""))
}

#[cfg(windows)]
fn wait_for_parent_exit(parent_pid: u32) {
    for _ in 0..240 {
        if !parent_process_alive(parent_pid) {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(windows)]
fn sibling_with_suffix(target: &Path, suffix: &str) -> Result<PathBuf, String> {
    let file = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "application filename is not valid UTF-8".to_string())?;
    Ok(target.with_file_name(format!("{file}{suffix}")))
}

#[cfg(windows)]
fn run_update_helper(target: &Path, checksum: &str, parent_pid: u32) -> Result<(), String> {
    let source = std::env::current_exe()
        .map_err(|error| format!("could not locate staged update: {error}"))?;
    let checksum = checksum.trim().to_ascii_lowercase();
    if checksum.len() != 64 || !checksum.chars().all(|value| value.is_ascii_hexdigit()) {
        return Err("invalid staged update checksum".into());
    }
    if windows_sha256(&source)? != checksum {
        return Err("staged update checksum changed before installation".into());
    }

    let temporary = sibling_with_suffix(target, ".update-new.exe")?;
    let backup = sibling_with_suffix(target, ".previous.exe")?;
    let _ = std::fs::remove_file(&temporary);
    std::fs::copy(&source, &temporary)
        .map_err(|error| format!("could not copy update into application folder: {error}"))?;
    if windows_sha256(&temporary)? != checksum {
        let _ = std::fs::remove_file(&temporary);
        return Err("copied update failed SHA-256 verification".into());
    }

    wait_for_parent_exit(parent_pid);

    let _ = std::fs::remove_file(&backup);
    let mut moved_old = false;
    for _ in 0..120 {
        match std::fs::rename(target, &backup) {
            Ok(()) => {
                moved_old = true;
                break;
            }
            Err(error) if !target.exists() => break,
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    if target.exists() && !moved_old {
        let _ = std::fs::remove_file(&temporary);
        return Err("Moth Controller did not release its executable for updating.".into());
    }

    if let Err(error) = std::fs::rename(&temporary, target) {
        if moved_old {
            let _ = std::fs::rename(&backup, target);
        }
        return Err(format!("could not activate downloaded update: {error}"));
    }
    if windows_sha256(target)? != checksum {
        let _ = std::fs::remove_file(target);
        if moved_old {
            let _ = std::fs::rename(&backup, target);
        }
        return Err("installed update failed SHA-256 verification".into());
    }

    Command::new(target)
        .spawn()
        .map_err(|error| format!("update installed but Moth could not restart: {error}"))?;
    let _ = std::fs::remove_file(&backup);
    let _ = std::fs::remove_file(source);
    Ok(())
}

#[cfg(windows)]
fn write_helper_error(error: &str) {
    let path = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(std::env::temp_dir)
        .join("moth-update-error.txt");
    let _ = std::fs::write(path, error);
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
