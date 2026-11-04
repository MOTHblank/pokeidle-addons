//! Per-account native session cache. Credentials must never be placed in
//! accounts.json, browser telemetry, logs, plain files, or command arguments.
//!
//! Windows: current-user DPAPI encrypts the whole native game bootstrap.
//! Linux: Secret Service desktop keyring via secret-tool (stdin for writes).
//! If an OS vault is unavailable, a one-time Firefox authentication is still
//! required for that launch; there is deliberately no plaintext fallback.
use crate::config::GameProfile;
#[cfg(windows)]
use crate::config;
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

#[cfg(test)]
mod capture_handoff_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn login_refresh_preserves_native_auto_catch_for_account_four() {
        let native=json!({
            "bootstrap":{"hello":"previous login"},
            "autocatch":{"enabled":true,"ballPriority":["ultra"],
                "restock":{"enabled":true,"selectedBallId":4}}
        });
        let firefox=json!({
            "bootstrap":{"hello":"new login"},
            "cookies":[],
            "autocatch":{"enabled":false,"ballPriority":["poke"],
                "restock":{"enabled":false,"selectedBallId":1}}
        });
        let merged=preserve_native_capture_preferences(&firefox,Some(&native));
        assert_eq!(merged["bootstrap"]["hello"],"new login");
        assert_eq!(merged["autocatch"],native["autocatch"]);
        assert_eq!(merged["autocatch"]["restock"]["selectedBallId"],4);
        assert_eq!(merged["autocatch"]["enabled"],true);
    }

    #[test]
    fn classified_capture_failures_do_not_guess_from_message_text() {
        let storage=CaptureSaveFailure::vault("Windows user credential protection failed");
        assert!(storage.retryable);
        let canceled=CaptureSaveFailure::permanent("Session changed");
        assert!(!canceled.retryable);
        assert_eq!(storage.message,"Windows user credential protection failed");
    }
    #[test]
    fn capture_vault_save_rejects_a_superseded_session() {
        use std::sync::atomic::{AtomicU64,Ordering};
        let generation=AtomicU64::new(4);
        assert!(capture_generation_is_current(&generation,4));
        generation.fetch_add(1,Ordering::AcqRel);
        assert!(!capture_generation_is_current(&generation,4));
        assert!(capture_generation_is_current(&generation,5));
    }

    #[test]
    fn session_invalidation_is_nonblocking_and_cancels_stalled_saves() {
        use std::sync::{mpsc,Arc};
        use std::sync::atomic::{AtomicU64,Ordering};
        use std::time::Duration;
        let generation=Arc::new(AtomicU64::new(3));
        let (inside_tx,inside_rx)=mpsc::sync_channel(0);
        let (release_tx,release_rx)=mpsc::sync_channel(0);
        let writer_generation=Arc::clone(&generation);
        let writer=std::thread::spawn(move || {
            capture_write_transaction(GameProfile::Game1,&writer_generation,3,|| {
                inside_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                if !capture_generation_is_current(&writer_generation,3) {
                    return Err(CaptureSaveFailure::permanent(
                        "Capture settings save canceled: session changed"));
                }
                Ok(())
            })
        });
        inside_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        invalidate_capture_generation(&generation);
        assert_eq!(generation.load(Ordering::Acquire),4,
            "Stop must invalidate immediately, without waiting for a vault write");
        let another=AtomicU64::new(9);
        assert!(capture_write_transaction(GameProfile::Game2,&another,9,
            ||Ok::<(),CaptureSaveFailure>(())).is_ok(),
            "Game2 must not wait on a stalled Game1 preference save");
        release_tx.send(()).unwrap();
        assert!(writer.join().unwrap().is_err());
        assert!(capture_write_transaction::<()>(GameProfile::Game1,&generation,3,||
            panic!("superseded session must not enter a vault write")).is_err());
    }
    #[test]
    fn canceled_inflight_capture_write_restores_previous_vault_contents() {
        use std::sync::atomic::{AtomicU64,Ordering};
        let generation=AtomicU64::new(10);
        let original=json!({"autocatch":{"enabled":false}});
        let updated=json!({"autocatch":{"enabled":true}});
        let mut stored=original.clone();
        let outcome=capture_write_transaction(GameProfile::Game1,&generation,10,||{
            commit_capture_update(&generation,10,&original,&updated,|value|{
                stored=value.clone();
                if value==&updated {
                    // Stop happens after the encrypted write, before commit.
                    generation.fetch_add(1,Ordering::AcqRel);
                }
                Ok(())
            })
        });
        assert!(outcome.is_err());
        assert_eq!(stored,original,
            "an invalidated write must restore the old settings");
    }

    #[test]
    fn committed_capture_write_is_not_reclassified_as_canceled() {
        use std::sync::atomic::{AtomicU64,Ordering};
        let generation=AtomicU64::new(10);
        let original=json!({"autocatch":{"enabled":false}});
        let updated=json!({"autocatch":{"enabled":true}});
        let mut stored=original.clone();
        let outcome=capture_write_transaction(GameProfile::Game1,&generation,10,||{
            commit_capture_update(&generation,10,&original,&updated,|value|{
                stored=value.clone();
                Ok(())
            })?;
            // Stop immediately after the last rollback opportunity.
            generation.fetch_add(1,Ordering::AcqRel);
            Ok(())
        });
        assert!(outcome.is_ok(),
            "a committed write cannot be falsely rejected without rollback");
        assert_eq!(stored,updated);
    }

    #[test]
    fn first_login_imports_the_browser_settings() {
        let firefox=json!({"autocatch":{"enabled":true,"ballPriority":["ultra"]}});
        assert_eq!(preserve_native_capture_preferences(&firefox,None),firefox);
        let no_old_settings=json!({"bootstrap":{"hello":"old"}});
        assert_eq!(preserve_native_capture_preferences(&firefox,Some(&no_old_settings)),firefox);
    }
}

/// Kick uses a dedicated OS-vault entry, never the PokéIdle game cache.
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
static PREFERENCE_WRITE: [std::sync::Mutex<()>;4] =
    [const {std::sync::Mutex::new(())};4];
fn preference_write(profile:GameProfile)->&'static std::sync::Mutex<()> {
    &PREFERENCE_WRITE[profile.index()]
}

/// Keep each profile's controller-edited capture settings authoritative when
/// Firefox is used again to refresh an expired or missing game login. A fresh
/// browser localStorage value is an import source only, never an implicit reset.
fn preserve_native_capture_preferences(fresh: &Value, prior: Option<&Value>) -> Value {
    let mut merged=fresh.clone();
    if let Some(settings)=prior.and_then(|v|v.get("autocatch"))
        .filter(|value|value.is_object()) {
        merged["autocatch"]=settings.clone();
    }
    merged
}

// Cached launches never read while an old write for their profile is
// unfinished, and a superseded launch cannot read the protected vault.
pub fn load_for_auth(profile:GameProfile,
    generation:&std::sync::atomic::AtomicU64,expected:u64
)->Result<Option<Value>,String> {
    let _guard=preference_write(profile).lock()
        .map_err(|_|"Native settings writer unavailable".to_owned())?;
    if !capture_generation_is_current(generation,expected) {
        return Err("Cached login canceled: launch changed".into());
    }
    let cached=load(profile)?;
    if !capture_generation_is_current(generation,expected) {
        return Err("Cached login canceled: launch changed".into());
    }
    Ok(cached)
}

/// Refresh credentials and preserve Auto Catch/Restock preferences using the
/// same write lock as native UI edits. Return the merged bootstrap for the
/// imminent game connection as well as for future cached starts.
pub fn cache_browser_handoff(profile: GameProfile, fresh: &Value,
    generation:&std::sync::atomic::AtomicU64,expected:u64)
    -> (Value, Option<String>) {
    let _guard=match preference_write(profile).lock() {
        Ok(guard)=>guard,
        Err(_)=>return (fresh.clone(), Some("Native settings writer unavailable".into()))
    };
    if !capture_generation_is_current(generation,expected) {
        return (fresh.clone(),Some("Firefox login cache canceled: launch changed".into()));
    }
    let (prior,read_error)=match load(profile) {
        Ok(prior)=>(prior,None),
        Err(error)=>(None,Some(error))
    };
    let merged=preserve_native_capture_preferences(fresh,prior.as_ref());
    if !capture_generation_is_current(generation,expected) {
        return (fresh.clone(),Some("Firefox login cache canceled: launch changed".into()));
    }
    let saved=save(profile,&merged);
    // Stop may overtake a slow vault operation. Do not leave credentials
    // from the canceled attempt visible to a later cached native launch.
    if saved.is_ok() && !capture_generation_is_current(generation,expected) {
        let restored=match prior.as_ref() {
            Some(previous)=>save(profile,previous),
            None=>clear(profile),
        };
        return (fresh.clone(),Some(match restored {
            Ok(())=>"Firefox login cache canceled: launch changed".into(),
            Err(error)=>format!("Canceled login cache rollback failed: {error}"),
        }));
    }
    let warning=match saved {
        Err(error)=>Some(error),
        Ok(())=>read_error,
    };
    (merged,warning)
}


/// Persist the complete validated capture configuration atomically, preserving
/// the encrypted authenticated session and unrelated bootstrap fields.
fn capture_generation_is_current(
    generation:&std::sync::atomic::AtomicU64, expected:u64
)->bool {
    generation.load(std::sync::atomic::Ordering::Acquire)==expected
}

// Invalidating a session is nonblocking; each profile's vault transaction
// serializes with later cached-login reads. An obsolete write detects its
// generation change and rolls back before releasing the profile lock.
pub fn invalidate_capture_generation(generation:&std::sync::atomic::AtomicU64) {
    // Only mutate the atomic here: the UI must never wait for a vault service.
    // An in-flight writer is responsible for rolling back after cancellation.
    generation.fetch_add(1,std::sync::atomic::Ordering::AcqRel);
}
/// Save failures are categorized at the vault boundary, never inferred from
/// localized/user-facing error text. Only OS storage failures get retries.
#[derive(Clone,Debug)]
pub struct CaptureSaveFailure {
    pub message:String,
    pub retryable:bool,
}
impl CaptureSaveFailure {
    fn permanent(message:impl Into<String>)->Self {
        Self{message:message.into(),retryable:false}
    }
    fn vault(message:impl Into<String>)->Self {
        Self{message:message.into(),retryable:true}
    }
}
impl std::fmt::Display for CaptureSaveFailure {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl From<&str> for CaptureSaveFailure {
    fn from(message:&str)->Self {Self::permanent(message)}
}
impl From<String> for CaptureSaveFailure {
    fn from(message:String)->Self {Self::permanent(message)}
}
fn capture_write_transaction<T>(
    profile:GameProfile,
    generation:&std::sync::atomic::AtomicU64, expected:u64,
    write:impl FnOnce()->Result<T,CaptureSaveFailure>
)->Result<T,CaptureSaveFailure> {
    if !capture_generation_is_current(generation,expected) {
        return Err(CaptureSaveFailure::permanent(
            "Capture settings save canceled: session changed"));
    }
    let _guard=preference_write(profile).lock()
        .map_err(|_|CaptureSaveFailure::vault("Native settings writer unavailable"))?;
    if !capture_generation_is_current(generation,expected) {
        return Err(CaptureSaveFailure::permanent(
            "Capture settings save canceled: session changed"));
    }
    // The write closure owns the final post-write cancellation check and
    // rollback. Once that check succeeds, the save has committed. Checking
    // again outside the closure could falsely report a completed vault
    // write as canceled, without having rolled it back.
    write()
}

fn load_capture_vault(profile:GameProfile)->Result<Option<Value>,CaptureSaveFailure>{
    platform::load(profile).map_err(CaptureSaveFailure::vault)?
        .map(|bytes|deserialize(&bytes).map_err(CaptureSaveFailure::permanent))
        .transpose()
}
fn save_capture_vault(profile:GameProfile,value:&Value)->Result<(),CaptureSaveFailure>{
    let bytes=serialize(value).map_err(CaptureSaveFailure::permanent)?;
    platform::save(profile,&bytes).map_err(CaptureSaveFailure::vault)
}
// Called while holding this profile's preference_write lock. An invalidation
// observed after the vault write must restore the original encrypted value
// before a replacement session is allowed to read it. Once the post-write
// check succeeds, the write is committed even if Stop happens immediately
// afterward: a successful commit must not be reclassified as a failed save.
fn commit_capture_update(
    generation:&std::sync::atomic::AtomicU64,expected:u64,
    original:&Value,updated:&Value,
    mut save:impl FnMut(&Value)->Result<(),CaptureSaveFailure>,
)->Result<(),CaptureSaveFailure>{
    if !capture_generation_is_current(generation,expected) {
        return Err(CaptureSaveFailure::permanent(
            "Capture settings save canceled: session changed"));
    }
    save(updated)?;
    if !capture_generation_is_current(generation,expected) {
        save(original).map_err(|error|CaptureSaveFailure::vault(format!(
            "Canceled capture save rollback failed: {error}")))?;
        return Err(CaptureSaveFailure::permanent(
            "Capture settings save canceled: session changed"));
    }
    Ok(())
}
pub fn set_capture_settings(profile: GameProfile,
    config: &crate::native_capture_config::CatchSettings,
    generation:&std::sync::atomic::AtomicU64, expected:u64
) -> Result<(),CaptureSaveFailure> {
    config.validate().map_err(CaptureSaveFailure::permanent)?;
    capture_write_transaction(profile,generation,expected,|| {
        let Some(mut saved)=load_capture_vault(profile)? else {
            return Err(CaptureSaveFailure::permanent(
                "Native credential cache unavailable; authenticate again to save preferences"));
        };
        let original=saved.clone();
        saved["autocatch"]=config.as_value();
        commit_capture_update(generation,expected,&original,&saved,
            |value|save_capture_vault(profile,value))
    })
}

/// Persist to the OS vault, then apply the confirmed preferences to the live
/// worker directly. UI repaint/visibility only controls acknowledgement text,
/// never when Auto Catch or Restock starts using a successful save.
pub fn persist_and_apply_capture_settings(
    profile: GameProfile,
    config: &crate::native_capture_config::CatchSettings,
    generation: &std::sync::atomic::AtomicU64,
    expected: u64,
    updater: &crate::native_background::CaptureSettingsUpdater,
) -> Result<(), CaptureSaveFailure> {
    set_capture_settings(profile,config,generation,expected)?;
    if !capture_generation_is_current(generation,expected) {
        // The vault committed before Stop. The old updater is no longer
        // authoritative, but persistence must not be reported as canceled.
        return Ok(());
    }
    updater.apply(config.clone()).map_err(CaptureSaveFailure::permanent)
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
    // Write encrypted data beside its destination and rename it into place.
    // An interrupted write must never truncate the previous valid login.
    fn atomic_replace(path:&std::path::Path,bytes:&[u8])->Result<(),String>{
        use std::io::Write;
        static TEMP_ID:std::sync::atomic::AtomicU64=
            std::sync::atomic::AtomicU64::new(0);
        let parent=path.parent().ok_or("Protected credential path has no directory")?;
        fs::create_dir_all(parent)
            .map_err(|_|"Could not prepare protected credential directory".to_owned())?;
        let name=path.file_name().ok_or("Protected credential path has no filename")?;
        for _ in 0..16 {
            let mut temporary=std::ffi::OsString::from(".");
            temporary.push(name);
            temporary.push(format!(".{}.{}.tmp",std::process::id(),
                TEMP_ID.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
            let pending=parent.join(temporary);
            let mut output=match fs::OpenOptions::new().write(true).create_new(true).open(&pending) {
                Ok(output)=>output,
                Err(error) if error.kind()==std::io::ErrorKind::AlreadyExists=>continue,
                Err(_)=>return Err("Could not create temporary protected credential".into()),
            };
            let prepared=output.write_all(bytes).and_then(|_|output.sync_all());
            drop(output);
            // On Windows, same-volume rename replaces the old destination.
            // If it fails (e.g. access denied), the original remains intact.
            let result=prepared.and_then(|_|fs::rename(&pending,path));
            if result.is_err() {
                let _=fs::remove_file(&pending);
                return Err("Could not atomically save protected credential".into());
            }
            return Ok(());
        }
        Err("Could not allocate temporary protected credential file".into())
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
        atomic_replace(&kick_location(profile)?,&cipher)
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
        atomic_replace(&location(profile)?,&cipher)
    }
    #[cfg(test)]
    mod atomic_write_tests {
        use super::*;
        #[test]
        fn replacing_a_protected_file_preserves_the_latest_complete_bytes() {
            let id=std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let directory=std::env::temp_dir().join(format!(
                "moth-dpapi-atomic-{}-{id}",std::process::id()));
            let path=directory.join("account.dpapi");
            atomic_replace(&path,b"encrypted-before").unwrap();
            assert_eq!(fs::read(&path).unwrap(),b"encrypted-before");
            atomic_replace(&path,b"encrypted-after").unwrap();
            assert_eq!(fs::read(&path).unwrap(),b"encrypted-after");
            let pending=fs::read_dir(&directory).unwrap().count();
            assert_eq!(pending,1,"no temporary credentials left behind");
            fs::remove_file(path).unwrap();
            fs::remove_dir(directory).unwrap();
        }
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
    use std::io::{Read,Write};
    use std::process::{Command, Stdio};
    use std::time::{Duration,Instant};
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

    // Do not leave a locked Secret Service request holding a profile forever.
    // Concurrent pipe draining also prevents large lookup results or writes
    // from deadlocking before the timeout can be enforced.
    fn secret_tool(args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
        const LIMIT:Duration=Duration::from_secs(12);
        let mut child=Command::new("secret-tool")
            .args(args)
            .stdin(if input.is_some() {Stdio::piped()} else {Stdio::null()})
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_|"Desktop keyring unavailable: install libsecret-tools and unlock a Secret Service keyring".to_owned())?;
        let stdout=child.stdout.take()
            .ok_or_else(||"Desktop keyring output unavailable".to_owned())?;
        let stderr=child.stderr.take()
            .ok_or_else(||"Desktop keyring diagnostics unavailable".to_owned())?;
        let stdout_reader=std::thread::spawn(move ||{
            let mut bytes=Vec::new();
            let mut stream=stdout.take((MAX_BOOTSTRAP_BYTES+5) as u64);
            let _=stream.read_to_end(&mut bytes);
            bytes
        });
        let stderr_reader=std::thread::spawn(move ||{
            let mut bytes=Vec::new();
            let mut stream=stderr.take(4096);
            let _=stream.read_to_end(&mut bytes);
            bytes
        });
        let input_writer=input.map(|data|{
            let writer=child.stdin.take();
            let data=data.to_vec();
            std::thread::spawn(move ||{
                let Some(mut writer)=writer else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,"keyring stdin unavailable"));
                };
                writer.write_all(&data)
            })
        });
        let started=Instant::now();
        let status=loop {
            match child.try_wait() {
                Ok(Some(status))=>break status,
                Ok(None)=>{},
                Err(_)=>{
                    let _=child.kill();
                    let _=child.wait();
                    return Err("Desktop keyring request failed".into());
                }
            }
            if started.elapsed()>=LIMIT {
                let _=child.kill();
                let _=child.wait();
                return Err("Desktop keyring request timed out".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        let output=stdout_reader.join()
            .map_err(|_|"Desktop keyring output reader failed".to_owned())?;
        let diagnostics=stderr_reader.join()
            .map_err(|_|"Desktop keyring diagnostics reader failed".to_owned())?;
        if input_writer.is_some_and(|writer|
            writer.join().map_or(true,|result|result.is_err())) {
            return Err("Desktop keyring refused credential input".into());
        }
        if !status.success() {
            if args.first()==Some(&"lookup")
                && missing_lookup(status.code(),&output,&diagnostics)
            {
                return Ok(Vec::new());
            }
            return Err("Desktop keyring request failed: ensure Secret Service is running and unlocked".into());
        }
        if output.len()>MAX_BOOTSTRAP_BYTES+4 {
            return Err("Desktop keyring credential response too large".into());
        }
        Ok(output)
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
        let mut cmd = vec!["store", "--label=Moth PokeIdle native game"];
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
        assert!(serialize(&json!({"bootstrap": {"hello":"not enough"}})).is_err());
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
