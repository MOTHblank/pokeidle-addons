use serde_json::Value;
use std::cmp::Ordering;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const RELEASE_API_URL: &str =
    "https://api.github.com/repos/MOTHblank/pokeidle-addons/releases/latest";
pub const RELEASES_URL: &str = "https://github.com/MOTHblank/pokeidle-addons/releases/latest";
#[cfg(windows)]
const LEGACY_WINDOWS_UPDATE_ASSET: &str = "Moth-Controller-Portable-windows-x86_64.exe";
#[cfg(windows)]
const WINDOWS_UNINSTALL_SUBKEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{5E1E5540-9723-4D63-9A92-7A9AA6BDA74C}_is1";
#[cfg(windows)]
const WINDOWS_APP_FILENAME: &str = "Moth Controller.exe";

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowsUpdateKind {
    Portable,
    Installer,
}

#[cfg(windows)]
fn windows_update_assets(tag: &str) -> [String; 2] {
    [
        format!("Moth-Controller-Portable-{tag}-windows-x86_64.exe"),
        LEGACY_WINDOWS_UPDATE_ASSET.to_string(),
    ]
}

#[cfg(windows)]
fn windows_installer_asset(tag: &str) -> String {
    format!("Moth-Controller-Setup-{tag}.exe")
}

#[derive(Clone, Debug)]
struct ReleaseInfo {
    tag: String,
    notes_url: String,
    #[cfg(windows)]
    portable_asset_url: Option<String>,
    #[cfg(windows)]
    portable_checksum_url: Option<String>,
    #[cfg(windows)]
    installer_asset_url: Option<String>,
    #[cfg(windows)]
    installer_checksum_url: Option<String>,
}

#[cfg(windows)]
impl ReleaseInfo {
    fn package(&self, kind: WindowsUpdateKind) -> Option<(&str, &str)> {
        let (asset, checksum) = match kind {
            WindowsUpdateKind::Portable =>
                (&self.portable_asset_url, &self.portable_checksum_url),
            WindowsUpdateKind::Installer =>
                (&self.installer_asset_url, &self.installer_checksum_url),
        };
        Some((asset.as_deref()?, checksum.as_deref()?))
    }
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
    #[cfg(windows)]
    staged_kind: Option<WindowsUpdateKind>,
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
                                state.staged_kind = None;
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
    let (portable_asset_url, portable_checksum_url, installer_asset_url, installer_checksum_url) = {
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
        let pair = |name: &str| {
            let checksum_name = format!("{name}.sha256");
            (find(name), find(&checksum_name))
        };

        let mut portable = (None, None);
        for asset_name in windows_update_assets(&tag) {
            let candidate = pair(&asset_name);
            if candidate.0.is_some() && candidate.1.is_some() {
                portable = candidate;
                break;
            }
        }
        let installer = pair(&windows_installer_asset(&tag));
        (portable.0, portable.1, installer.0, installer.1)
    };

    Ok(ReleaseInfo {
        tag,
        notes_url,
        #[cfg(windows)]
        portable_asset_url,
        #[cfg(windows)]
        portable_checksum_url,
        #[cfg(windows)]
        installer_asset_url,
        #[cfg(windows)]
        installer_checksum_url,
    })
}

#[cfg(windows)]
mod win32_reg {
    use std::ffi::c_void;

    type HKey = *mut c_void;
    const HKEY_CURRENT_USER: HKey = 0x8000_0001u32 as usize as HKey;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_MORE_DATA: i32 = 234;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            key: HKey,
            sub_key: *const u16,
            options: u32,
            desired: u32,
            result: *mut HKey,
        ) -> i32;
        fn RegQueryValueExW(
            key: HKey,
            value_name: *const u16,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            data_bytes: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: HKey,
            value_name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            data_bytes: u32,
        ) -> i32;
        fn RegCloseKey(key: HKey) -> i32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn key_exists(subkey: &str) -> bool {
        let subkey = wide(subkey);
        let mut handle: HKey = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        if status == ERROR_SUCCESS {
            unsafe { RegCloseKey(handle) };
            true
        } else {
            false
        }
    }

    pub fn query_string(subkey: &str, name: &str) -> Option<String> {
        let subkey = wide(subkey);
        let name = wide(name);
        let mut handle: HKey = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let mut kind: u32 = 0;
        let mut bytes: u32 = 512;
        let mut buffer = vec![0u16; (bytes / 2) as usize];
        let mut read_status = unsafe {
            RegQueryValueExW(
                handle,
                name.as_ptr(),
                std::ptr::null_mut(),
                &mut kind,
                buffer.as_mut_ptr() as *mut u8,
                &mut bytes,
            )
        };
        if read_status == ERROR_MORE_DATA && bytes > 0 {
            buffer.resize((bytes as usize).div_ceil(2), 0);
            read_status = unsafe {
                RegQueryValueExW(
                    handle,
                    name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut kind,
                    buffer.as_mut_ptr() as *mut u8,
                    &mut bytes,
                )
            };
        }
        unsafe { RegCloseKey(handle) };
        if read_status != ERROR_SUCCESS || kind != REG_SZ {
            return None;
        }
        let len = (bytes as usize / 2).min(buffer.len());
        let end = buffer[..len].iter().position(|&c| c == 0).unwrap_or(len);
        let text = String::from_utf16_lossy(&buffer[..end]).trim().to_owned();
        (!text.is_empty()).then_some(text)
    }

    pub fn set_string(subkey: &str, name: &str, value: &str) -> Result<(), String> {
        let subkey = wide(subkey);
        let name = wide(name);
        let value = wide(value);
        let mut handle: HKey = std::ptr::null_mut();
        let open_status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut handle,
            )
        };
        if open_status != ERROR_SUCCESS {
            return Err(format!("RegOpenKeyExW failed with error {open_status}"));
        }
        let byte_len = u32::try_from(value.len() * 2).unwrap_or(u32::MAX);
        let write_status = unsafe {
            RegSetValueExW(
                handle,
                name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr() as *const u8,
                byte_len,
            )
        };
        unsafe { RegCloseKey(handle) };
        if write_status != ERROR_SUCCESS {
            return Err(format!("RegSetValueExW failed with error {write_status}"));
        }
        Ok(())
    }
}

#[cfg(windows)]
fn windows_uninstall_key_exists() -> bool {
    win32_reg::key_exists(WINDOWS_UNINSTALL_SUBKEY)
}

#[cfg(windows)]
fn windows_registry_value(name: &str) -> Option<String> {
    win32_reg::query_string(WINDOWS_UNINSTALL_SUBKEY, name)
}

#[cfg(windows)]
fn same_windows_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left.to_string_lossy().eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(windows)]
fn expected_windows_install_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("Programs")
            .join("Moth Controller")
            .join(WINDOWS_APP_FILENAME)
    })
}

#[cfg(windows)]
fn is_installed_windows_copy() -> bool {
    if !windows_uninstall_key_exists() {
        return false;
    }
    let Ok(current) = std::env::current_exe() else { return false; };
    let registered = windows_registry_value("InstallLocation")
        .map(PathBuf::from)
        .map(|path| path.join(WINDOWS_APP_FILENAME))
        .or_else(expected_windows_install_path);
    registered.is_some_and(|path| same_windows_path(&current, &path))
}

#[cfg(windows)]
fn windows_update_kind() -> WindowsUpdateKind {
    if is_installed_windows_copy() {
        WindowsUpdateKind::Installer
    } else {
        WindowsUpdateKind::Portable
    }
}

/// Self-heal the one-time transition from the pre-installer-aware updater.
/// v0.2.2 can replace an installed executable directly, so the first newer
/// binary repairs Inno Setup's DisplayVersion before future updates use Setup.
#[cfg(windows)]
pub fn repair_windows_install_metadata() -> Result<bool, String> {
    if !is_installed_windows_copy() {
        return Ok(false);
    }
    let current = env!("CARGO_PKG_VERSION");
    if windows_registry_value("DisplayVersion")
        .as_deref()
        .is_some_and(|registered| compare_versions(registered, current) == Some(Ordering::Equal))
    {
        return Ok(false);
    }

    win32_reg::set_string(WINDOWS_UNINSTALL_SUBKEY, "DisplayVersion", current)
        .map_err(|error| format!("could not repair Windows install metadata: {error}"))?;
    Ok(true)
}

#[cfg(not(windows))]
pub fn repair_windows_install_metadata() -> Result<bool, String> {
    Ok(false)
}

#[cfg(windows)]
pub fn download_update(status: &SharedUpdateStatus) -> Result<bool, String> {
    let (release, kind) = {
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
        let kind = windows_update_kind();
        if release.package(kind).is_none() {
            return Err(match kind {
                WindowsUpdateKind::Installer =>
                    "This release does not include the Windows Setup package required to update the installed edition.".into(),
                WindowsUpdateKind::Portable =>
                    "This release does not include the portable Windows update package.".into(),
            });
        }
        state.downloading = true;
        state.last_error = None;
        (release, kind)
    };

    let worker_status = Arc::clone(status);
    let spawn = thread::Builder::new()
        .name("release-update-download".into())
        .spawn(move || {
            let result = download_release(&release, kind);
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
                        state.staged_kind = Some(kind);
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
            (name.ends_with("-update.exe")
                || name.ends_with("-update.exe.download")
                || name.ends_with("-setup.exe")
                || name.ends_with("-setup.exe.download"));
        if is_update {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(windows)]
fn download_release(
    release: &ReleaseInfo,
    kind: WindowsUpdateKind,
) -> Result<(PathBuf, String), String> {
    let (asset_url, checksum_url) = release
        .package(kind)
        .ok_or("missing update package or checksum")?;
    let directory = crate::config::data_root()?.join("Updates");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create update staging folder: {error}"))?;
    // A Windows helper or installer cannot delete its own running image after a
    // successful update. Remove leftovers before staging a new package.
    cleanup_old_update_files(&directory);

    let safe_tag = release
        .tag
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
        .collect::<String>();
    let suffix = match kind {
        WindowsUpdateKind::Portable => "update.exe",
        WindowsUpdateKind::Installer => "setup.exe",
    };
    let final_path = directory.join(format!("moth-controller-{safe_tag}-{suffix}"));
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
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("could not open update file for SHA-256 verification: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("could not read update file for SHA-256 verification: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finish_hex())
}

#[cfg(windows)]
struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

#[cfg(windows)]
impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a,
                0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19,
            ],
            buffer: [0u8; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.total_len += data.len() as u64;
        if self.buffer_len > 0 {
            let take = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }
        while data.len() >= 64 {
            let (block, rest) = data.split_at(64);
            self.compress(block.try_into().expect("64-byte block"));
            data = rest;
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffer_len = data.len();
        }
    }

    fn finish_hex(mut self) -> String {
        let bit_len = self.total_len.wrapping_mul(8);
        self.update(&[0x80]);
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..64].fill(0);
            let block = self.buffer;
            self.compress(&block);
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);

        let mut hex = String::with_capacity(64);
        for word in self.state {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{word:08x}");
        }
        hex
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5,
            0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174,
            0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da,
            0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967,
            0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85,
            0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070,
            0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
            0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
        ];
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
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
    let (staged, checksum, version, kind) = {
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
            state
                .staged_kind
                .ok_or_else(|| "Staged update type is unavailable.".to_string())?,
        )
    };
    if windows_sha256(&staged)? != checksum {
        return Err("Staged update failed SHA-256 verification.".into());
    }

    match kind {
        WindowsUpdateKind::Installer => {
            if !is_installed_windows_copy() {
                return Err("The installed-edition update package no longer matches this Moth Controller location.".into());
            }
            let mut setup = Command::new(&staged);
            setup.creation_flags(0x08000000);
            setup.args([
                "/SP-",
                "/VERYSILENT",
                "/SUPPRESSMSGBOXES",
                "/NORESTART",
                "/CLOSEAPPLICATIONS",
                "/MOTHUPDATE=1",
            ]);
            setup
                .spawn()
                .map_err(|error| format!("could not start Moth Controller Setup: {error}"))?;
        }
        WindowsUpdateKind::Portable => {
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
        }
    }
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
    use std::ffi::c_void;
    type Handle = *mut c_void;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_TIMEOUT: u32 = 0x0000_0102;
    const ERROR_ACCESS_DENIED: u32 = 5;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn GetLastError() -> u32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, parent_pid) };
    if handle.is_null() {
        return unsafe { GetLastError() } == ERROR_ACCESS_DENIED;
    }
    let alive = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
    unsafe { CloseHandle(handle) };
    alive
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
            Err(_) if !target.exists() => break,
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
