//! Per-account native session cache. Credentials must never be placed in
//! accounts.json, browser telemetry, logs, plain files, or command arguments.
//!
//! Windows: current-user DPAPI encrypts the *whole* game+Twitch bootstrap.
//! Linux: Secret Service desktop keyring via secret-tool (stdin for writes).
//! If an OS vault is unavailable, a one-time Firefox authentication is still
//! required for that launch; there is deliberately no plaintext fallback.
use crate::config::{self, GameProfile};
use serde_json::Value;

const MAX_BOOTSTRAP_BYTES: usize = 512 * 1024;

fn serialize(value: &Value) -> Result<Vec<u8>, String> {
    crate::native_background::validate_bootstrap(value)?;
    let bytes = serde_json::to_vec(value)
        .map_err(|_| "Could not encode native credentials".to_owned())?;
    if bytes.len() > MAX_BOOTSTRAP_BYTES {
        return Err("Native credential payload exceeds safe size limit".into());
    }
    Ok(bytes)
}
fn deserialize(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_BOOTSTRAP_BYTES {
        return Err("Stored native credential payload is too large".into());
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "Native credential cache is not valid JSON".to_owned())?;
    crate::native_background::validate_bootstrap(&value)?;
    Ok(value)
}

pub fn load(profile: GameProfile) -> Result<Option<Value>, String> {
    platform::load(profile)?.map(|data| deserialize(&data)).transpose()
}
pub fn save(profile: GameProfile, value: &Value) -> Result<(), String> {
    let bytes = serialize(value)?;
    platform::save(profile, &bytes)
}
// Revocation is intentionally retained for a future explicit sign-out action.
// Never erase a player’s saved login as an incidental cleanup step.
#[allow(dead_code)]
pub fn clear(profile: GameProfile) -> Result<(), String> {
    platform::clear(profile)
}

/// Kick uses a dedicated OS-vault entry, never the PokeIdle/Twitch cache.
/// No plaintext fallback and no Kick credentials written to accounts.json.
const MAX_KICK_TOKEN_BYTES: usize = 8192;
fn check_kick_token(token: &str) -> Result<&str, String> {
    let trimmed = token.trim();
    let bearer = trimmed.strip_prefix("Bearer ").unwrap_or(trimmed).trim();
    if bearer.is_empty() || bearer.len() > MAX_KICK_TOKEN_BYTES ||
        !bearer.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err("Invalid Kick Bearer token".into());
    }
    Ok(bearer)
}
pub fn load_kick_token(profile: GameProfile) -> Result<Option<String>, String> {
    platform::load_kick(profile)?.map(|bytes| {
        if bytes.len() > MAX_KICK_TOKEN_BYTES { return Err("Stored Kick token is oversized".into()); }
        let token = String::from_utf8(bytes)
            .map_err(|_| "Stored Kick token is invalid".to_owned())?;
        check_kick_token(&token).map(str::to_owned)
    }).transpose()
}
pub fn save_kick_token(profile: GameProfile, token: &str) -> Result<(), String> {
    platform::save_kick(profile, check_kick_token(token)?.as_bytes())
}
pub fn clear_kick_token(profile: GameProfile) -> Result<(), String> {
    platform::clear_kick(profile)
}

// UI events may toggle Auto Catch and native Restock in rapid succession.
// Serialize the read-modify-encrypt-write cycle so one toggle never silently
// overwrites the other; no credentials are exposed outside OS vault storage.
static PREFERENCE_WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_native_preference(profile: GameProfile, path: &[&str], enabled: bool)
    -> Result<(), String> {
    let _guard = PREFERENCE_WRITE.lock()
        .map_err(|_| "Native settings writer unavailable".to_owned())?;
    let Some(mut saved) = load(profile)? else {
        return Err("Native credential cache is unavailable".into());
    };
    if !saved.get("autocatch").is_some_and(Value::is_object) {
        saved["autocatch"] = serde_json::json!({});
    }
    let mut target = &mut saved["autocatch"];
    for key in path.iter().take(path.len().saturating_sub(1)) {
        if !target.get(*key).is_some_and(Value::is_object) {
            target[*key] = serde_json::json!({});
        }
        target = &mut target[*key];
    }
    let Some(key) = path.last() else {
        return Err("Invalid native settings field".into());
    };
    target[*key] = Value::Bool(enabled);
    save(profile, &saved)
}

/// Persist the complete validated capture configuration atomically, preserving
/// the encrypted authenticated session and unrelated bootstrap fields.
pub fn set_capture_settings(profile: GameProfile,
    config: &crate::native_capture_config::CatchSettings) -> Result<(),String> {
    config.validate()?;
    let _guard=PREFERENCE_WRITE.lock()
        .map_err(|_|"Native settings writer unavailable".to_owned())?;
    let Some(mut saved)=load(profile)? else {
        return Err("Native credential cache unavailable; authenticate again to save preferences".into());
    };
    saved["autocatch"]=config.as_value();
    save(profile,&saved)
}

/// Native Auto Catch remains functional and configurable without the addon.
pub fn set_catch_enabled(profile: GameProfile, enabled: bool) -> Result<(), String> {
    set_native_preference(profile, &["enabled"], enabled)
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{ffi::c_void, fs, ptr, slice};

    #[repr(C)]
    struct Blob { size: u32, data: *mut u8 }
    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(input: *mut Blob, description: *const u16,
            entropy: *mut Blob, reserved: *mut c_void, prompt: *mut c_void,
            flags: u32, output: *mut Blob) -> i32;
        fn CryptUnprotectData(input: *mut Blob, description: *mut *mut u16,
            entropy: *mut Blob, reserved: *mut c_void, prompt: *mut c_void,
            flags: u32, output: *mut Blob) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
    }

    fn crypt(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
        if bytes.len() > MAX_BOOTSTRAP_BYTES + 4096 {
            return Err("DPAPI input is oversized".into());
        }
        let mut source = bytes.to_vec();
        let mut input = Blob { size: source.len() as u32, data: source.as_mut_ptr() };
        let mut output = Blob { size: 0, data: ptr::null_mut() };
        // CRYPTPROTECT_UI_FORBIDDEN: background startup cannot show prompts.
        let ok = unsafe {
            if encrypt {
                CryptProtectData(&mut input, ptr::null(), ptr::null_mut(),
                    ptr::null_mut(), ptr::null_mut(), 0x1, &mut output)
            } else {
                CryptUnprotectData(&mut input, ptr::null_mut(), ptr::null_mut(),
                    ptr::null_mut(), ptr::null_mut(), 0x1, &mut output)
            }
        };
        if ok == 0 { return Err("Windows user credential protection failed".into()); }
        let result = unsafe {
            let copied = slice::from_raw_parts(output.data, output.size as usize).to_vec();
            LocalFree(output.data.cast::<c_void>());
            copied
        };
        Ok(result)
    }
    fn location(profile: GameProfile) -> Result<std::path::PathBuf, String> {
        Ok(config::data_root()?.join("native-credentials")
            .join(format!("{}.dpapi", profile.name())))
    }
    fn kick_location(profile: GameProfile) -> Result<std::path::PathBuf, String> {
        Ok(config::data_root()?.join("native-credentials")
            .join(format!("{}.kick.dpapi", profile.name())))
    }
    pub fn load_kick(profile: GameProfile) -> Result<Option<Vec<u8>>, String> {
        let bytes = match fs::read(kick_location(profile)?) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Could not read protected Kick login".into()),
        };
        crypt(&bytes, false).map(Some)
    }
    pub fn save_kick(profile: GameProfile, token: &[u8]) -> Result<(), String> {
        let cipher = crypt(token, true)?;
        let path = kick_location(profile)?;
        fs::create_dir_all(path.parent().expect("credential directory"))
            .map_err(|_| "Could not prepare Kick credential directory".to_owned())?;
        fs::write(path, cipher).map_err(|_| "Could not save protected Kick login".to_owned())
    }
    pub fn clear_kick(profile: GameProfile) -> Result<(), String> {
        match fs::remove_file(kick_location(profile)?) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Could not remove Kick login".into()),
        }
    }
    pub fn load(profile: GameProfile) -> Result<Option<Vec<u8>>, String> {
        let path = location(profile)?;
        let cipher = match fs::read(path) {
            Ok(value) => value,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Could not read protected native session".into()),
        };
        crypt(&cipher, false).map(Some)
    }
    pub fn save(profile: GameProfile, bytes: &[u8]) -> Result<(), String> {
        let cipher = crypt(bytes, true)?;
        let path = location(profile)?;
        fs::create_dir_all(path.parent().expect("credential directory"))
            .map_err(|_| "Could not create protected native credential directory".to_owned())?;
        fs::write(path, cipher)
            .map_err(|_| "Could not save DPAPI-protected native session".to_owned())
    }
    #[allow(dead_code)]
    pub fn clear(profile: GameProfile) -> Result<(), String> {
        match fs::remove_file(location(profile)?) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Could not clear protected native session".into()),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};
    const APP: &str = "moth-pokeidle-native";
    const KICK_APP: &str = "moth-kick-native";
    // User profile number is used only as an OS keyring attribute.
    fn base_args(profile: GameProfile) -> [&'static str; 4] {
        ["application", APP, "profile", profile.name()]
    }
    fn kick_args(profile: GameProfile) -> [&'static str; 4] {
        ["application", KICK_APP, "profile", profile.name()]
    }
    // secret-tool lookup exits with status 1 when no matching item exists.
    // An absent first-run login must not be mistaken for a broken keyring.
    // A locked/unavailable keyring typically emits a diagnostic on stderr;
    // never log that diagnostic or any credential payload.
    fn missing_lookup(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> bool {
        code == Some(1) && stdout.is_empty() && stderr.is_empty()
    }

    fn secret_tool(args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
        let mut child = Command::new("secret-tool")
            .args(args)
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| "Desktop keyring unavailable: install libsecret-tools and unlock a Secret Service keyring".to_owned())?;
        if let Some(data) = input {
            let mut stdin = child.stdin.take()
                .ok_or_else(|| "Could not reach desktop keyring input".to_owned())?;
            stdin.write_all(data)
                .map_err(|_| "Desktop keyring refused credential input".to_owned())?;
            drop(stdin);
        }
        let output = child.wait_with_output()
            .map_err(|_| "Desktop keyring request failed".to_owned())?;
        if !output.status.success() {
            if args.first() == Some(&"lookup")
                && missing_lookup(output.status.code(), &output.stdout, &output.stderr)
            {
                return Ok(Vec::new());
            }
            return Err("Desktop keyring request failed: ensure Secret Service is running and unlocked".into());
        }
        if output.stdout.len() > MAX_BOOTSTRAP_BYTES + 4 {
            return Err("Desktop keyring credential response too large".into());
        }
        Ok(output.stdout)
    }
    pub fn load_kick(profile: GameProfile) -> Result<Option<Vec<u8>>, String> {
        let args = kick_args(profile);
        let mut cmd = vec!["lookup"];
        cmd.extend(args);
        let mut found = secret_tool(&cmd, None)?;
        while found.last() == Some(&b'\n') { found.pop(); }
        if found.is_empty() { return Ok(None); }
        Ok(Some(found))
    }
    pub fn save_kick(profile: GameProfile, token: &[u8]) -> Result<(), String> {
        let args = kick_args(profile);
        let mut cmd = vec!["store", "--label=Moth Kick account login"];
        cmd.extend(args);
        secret_tool(&cmd, Some(token)).map(|_| ())
    }
    pub fn clear_kick(profile: GameProfile) -> Result<(), String> {
        let args = kick_args(profile);
        let mut cmd = vec!["clear"];
        cmd.extend(args);
        secret_tool(&cmd, None).map(|_| ())
    }
    pub fn load(profile: GameProfile) -> Result<Option<Vec<u8>>, String> {
        let args = base_args(profile);
        let mut cmd = vec!["lookup"];
        cmd.extend(args);
        let mut found = secret_tool(&cmd, None)?;
        if found.is_empty() { return Ok(None); }
        // secret-tool lookup appends a line break after the stored secret.
        while found.last() == Some(&b'\n') { found.pop(); }
        if found.is_empty() { return Ok(None); }
        Ok(Some(found))
    }
    pub fn save(profile: GameProfile, bytes: &[u8]) -> Result<(), String> {
        let args = base_args(profile);
        let mut cmd = vec!["store", "--label=Moth PokeIdle native game and Twitch"];
        cmd.extend(args);
        secret_tool(&cmd, Some(bytes)).map(|_| ())
    }
    #[allow(dead_code)]
    pub fn clear(profile: GameProfile) -> Result<(), String> {
        let args = base_args(profile);
        let mut cmd = vec!["clear"];
        cmd.extend(args);
        secret_tool(&cmd, None).map(|_| ())
    }

    #[cfg(test)]
    mod tests {
        use super::missing_lookup;

        #[test]
        fn missing_item_does_not_look_like_keyring_failure() {
            assert!(missing_lookup(Some(1), b"", b""));
            assert!(!missing_lookup(Some(1), b"", b"keyring locked"));
            assert!(!missing_lookup(Some(2), b"", b""));
            assert!(!missing_lookup(Some(1), b"secret", b""));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn invalid_auth_data_is_never_persisted() {
        assert!(serialize(&json!({"twitch": {"pass":"test"}})).is_err());
        assert!(deserialize(b"{}").is_err());
    }
    #[test]
    fn kick_token_validation_rejects_empty_and_control_bytes() {
        assert!(check_kick_token("  ").is_err());
        assert!(check_kick_token("some token").is_err());
        assert_eq!(check_kick_token("Bearer abc.DEF").unwrap(), "abc.DEF");
    }
    #[test]
    fn oversized_payload_is_rejected() {
        assert!(deserialize(&vec![b'a'; MAX_BOOTSTRAP_BYTES + 1]).is_err());
    }
}
