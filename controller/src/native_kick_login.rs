//! Kick login through an ordinary, uninstrumented Firefox window.
//! No Remote Agent, BiDi, WebDriver, injected scripts or page automation.
//! After Firefox closes, import only this profile's own Kick session cookie
//! from its local cookie database and store it in the OS credential vault.
use crate::config::{Config, GameProfile};
use crate::{firefox, native_auth};
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL: Duration = Duration::from_millis(500);
const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_OPEN_READONLY: c_int = 1;

// Use the operating system's SQLite (Windows ships WinSQLite3). This avoids
// bundling another SQLite engine just to read a single signed-in cookie.
#[cfg_attr(windows, link(name = "winsqlite3"))]
#[cfg_attr(not(windows), link(name = "sqlite3"))]
unsafe extern "C" {
    fn sqlite3_open_v2(filename: *const c_char, db: *mut *mut c_void,
        flags: c_int, vfs: *const c_char) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn sqlite3_prepare_v2(db: *mut c_void, sql: *const c_char,
        bytes: c_int, stmt: *mut *mut c_void, tail: *mut *const c_char) -> c_int;
    fn sqlite3_step(stmt: *mut c_void) -> c_int;
    fn sqlite3_finalize(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_text(stmt: *mut c_void, column: c_int) -> *const u8;
    fn sqlite3_column_bytes(stmt: *mut c_void, column: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut c_void, column: c_int) -> i64;
}

struct Database(*mut c_void);
impl Drop for Database {
    fn drop(&mut self) { unsafe { sqlite3_close(self.0); } }
}
struct Statement(*mut c_void);
impl Drop for Statement {
    fn drop(&mut self) { unsafe { sqlite3_finalize(self.0); } }
}
fn cookie_looks_like_kick_token(value: &str) -> bool {
    let Some((id, secret)) = value.split_once('|') else { return false };
    !id.is_empty() && id.len() <= 18 && id.bytes().all(|b| b.is_ascii_digit())
        && secret.len() >= 16 && value.len() <= 8192
        && secret.bytes().all(|b| b.is_ascii_graphic())
}

/// Return one unexpired Kick session token or None. Never copy other cookies
/// or open an arbitrary Firefox profile/database.
/// Decode URL-encoded session values such as 123%7Csecret, then
/// validate the actual credential. All bytes remain local and bounded.
fn decode_kick_session_cookie(value: &str) -> Option<String> {
    let bytes = value.trim().as_bytes();
    if bytes.is_empty() || bytes.len() > 8192 { return None; }
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = |b: u8| -> Option<u8> {
                match b {
                    b'0'..=b'9' => Some(b - b'0'),
                    b'a'..=b'f' => Some(b - b'a' + 10),
                    b'A'..=b'F' => Some(b - b'A' + 10),
                    _ => None,
                }
            };
            decoded.push(hex(*bytes.get(i + 1)?)? * 16 +
                hex(*bytes.get(i + 2)?)?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let token = String::from_utf8(decoded).ok()?;
    cookie_looks_like_kick_token(&token).then_some(token)
}

fn read_kick_cookie(profile_dir: &Path) -> Result<Option<String>, String> {
    let path = profile_dir.join("cookies.sqlite");
    if !path.is_file() { return Ok(None); }
    let path = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| "Invalid Kick Firefox profile path".to_owned())?;
    let mut raw = ptr::null_mut();
    let rc = unsafe {
        sqlite3_open_v2(path.as_ptr(), &mut raw, SQLITE_OPEN_READONLY, ptr::null())
    };
    if rc != SQLITE_OK {
        if !raw.is_null() { unsafe { sqlite3_close(raw); } }
        return Err("Could not open the Kick Firefox cookie database".into());
    }
    let db = Database(raw);
    // Values are never used to construct SQL; restrict to the domain and the
    // specific session cookie, not Cloudflare or third-party cookies.
    let sql = c"SELECT value, expiry FROM moz_cookies WHERE name = 'session_token'         AND host IN ('kick.com', '.kick.com', 'www.kick.com', '.www.kick.com')         ORDER BY expiry DESC";
    let mut stmt = ptr::null_mut();
    if unsafe { sqlite3_prepare_v2(db.0, sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) } != SQLITE_OK {
        return Err("Kick Firefox cookies database has an unsupported schema".into());
    }
    let stmt = Statement(stmt);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs() as i64;
    loop {
        match unsafe { sqlite3_step(stmt.0) } {
            SQLITE_ROW => {
                let expiry = unsafe { sqlite3_column_int64(stmt.0, 1) };
                if expiry > 0 && expiry <= now { continue; }
                let length = unsafe { sqlite3_column_bytes(stmt.0, 0) };
                if !(1..=8192).contains(&length) { continue; }
                let text = unsafe { sqlite3_column_text(stmt.0, 0) };
                if text.is_null() { continue; }
                let bytes = unsafe { std::slice::from_raw_parts(text, length as usize) };
                if let Ok(raw) = std::str::from_utf8(bytes) {
                    if let Some(token) = decode_kick_session_cookie(raw) {
                        return Ok(Some(token));
                    }
                }
            }
            SQLITE_DONE => return Ok(None),
            _ => return Err("Could not read Kick Firefox session cookie".into()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoginStatus {
    Idle,
    Running,
    Done,
    Error(String),
}
struct LoginJob {
    status: LoginStatus,
    cancelled: Arc<AtomicBool>,
}
fn jobs() -> &'static Mutex<[LoginJob; 4]> {
    static JOBS: OnceLock<Mutex<[LoginJob; 4]>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(std::array::from_fn(|_| LoginJob {
        status: LoginStatus::Idle, cancelled: Arc::new(AtomicBool::new(false)),
    })))
}
pub fn status(profile: GameProfile) -> LoginStatus {
    jobs().lock().ok().map(|jobs| jobs[profile.index()].status.clone())
        .unwrap_or(LoginStatus::Error("Kick login state unavailable".into()))
}
pub fn cancel(profile: GameProfile) {
    if let Ok(jobs) = jobs().lock() {
        jobs[profile.index()].cancelled.store(true, Ordering::Relaxed);
    }
}
pub fn reset(profile: GameProfile) {
    if let Ok(mut jobs) = jobs().lock() {
        if !matches!(jobs[profile.index()].status, LoginStatus::Running) {
            jobs[profile.index()].status = LoginStatus::Idle;
        }
    }
}
pub fn start(profile: GameProfile) -> Result<(), String> {
    let cancelled = {
        let mut jobs = jobs().lock()
            .map_err(|_| "Kick login state unavailable".to_owned())?;
        let job = &mut jobs[profile.index()];
        if matches!(job.status, LoginStatus::Running) {
            return Err("Kick login already in progress".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        job.status = LoginStatus::Running;
        job.cancelled = Arc::clone(&cancelled);
        cancelled
    };
    let spawned = thread::Builder::new()
        .name(format!("moth-kick-login-{}", profile.name()))
        .spawn(move || {
            let result = login(profile, &cancelled);
            if let Ok(mut jobs) = jobs().lock() {
                jobs[profile.index()].status = match result {
                    Ok(()) => LoginStatus::Done,
                    Err(error) => LoginStatus::Error(error),
                };
            }
        });
    if let Err(error) = spawned {
        reset_after_spawn_failure(profile);
        return Err(format!("Could not start Kick login: {error}"));
    }
    Ok(())
}
fn reset_after_spawn_failure(profile: GameProfile) {
    if let Ok(mut jobs) = jobs().lock() {
        jobs[profile.index()].status = LoginStatus::Idle;
    }
}

/// Reuse the *same* Game1..Game4 Firefox profile that authenticated
/// PokéIdle and Google. The normal browser can only open after the game's
/// BiDi handoff closed Firefox and released its profile lock. Starting with
/// no Remote Agent does not erase existing cookies, Google sessions or prefs.
fn kick_config(profile: GameProfile) -> Result<Config, String> {
    Config::for_profile(profile)
}
pub fn login(profile: GameProfile, cancelled: &AtomicBool) -> Result<(), String> {
    let config = kick_config(profile)?;
    // A prior Firefox visit to Kick may already have a session. Avoid
    // opening another login window when it can be reused directly.
    if let Some(token) = read_kick_cookie(&config.profile_dir)? {
        if cancelled.load(Ordering::Relaxed) { return Err("Kick login cancelled".into()); }
        return native_auth::save_kick_token(profile, &token);
    }
    if firefox::find_profile_process(&config)?.is_some() {
        return Err("This PokéIdle Firefox profile is still open. Finish the native game handoff and try Kick login again.".into());
    }
    let mut browser = firefox::launch_kick_login(&config)?;
    // Ordinary Firefox is user-controlled. No inspection or interaction with
    // its web pages: the user signs in and closes the browser when finished.
    loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = browser.kill();
            let _ = browser.wait();
            return Err("Kick login cancelled".into());
        }
        // Read the first-party session during the active login. Some
        // browsers clear transient cookies when the window is closed.
        if let Ok(Some(token)) = read_kick_cookie(&config.profile_dir) {
            native_auth::save_kick_token(profile, &token)?;
            return Ok(());
        }
        match browser.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => thread::sleep(CHECK_INTERVAL),
            Err(_) => return Err("Could not observe the Kick Firefox login window".into()),
        }
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err("Kick login cancelled".into());
    }
    let token = read_kick_cookie(&config.profile_dir)?.ok_or_else(|| {
        "No valid Kick session_token was found. Sign in, then close the Firefox window.".to_owned()
    })?;
    native_auth::save_kick_token(profile, &token)
}

#[cfg(test)]
mod tests {
    use super::{cookie_looks_like_kick_token, decode_kick_session_cookie};
    #[test]
    fn decodes_percent_encoded_kick_cookies() {
        let expected = Some("123|abcdefghijklmnop123456".to_owned());
        assert_eq!(decode_kick_session_cookie("123%7Cabcdefghijklmnop123456"), expected);
        assert_eq!(decode_kick_session_cookie("123%7cabcdefghijklmnop123456"), expected);
        assert_eq!(decode_kick_session_cookie("123|abcdefghijklmnop123456"), expected);
        assert_eq!(decode_kick_session_cookie("123%7Gabcdefghijklmnop123456"), None);
        assert_eq!(decode_kick_session_cookie("123%25257Cabcdefghijklmnop123456"), None);
    }
    #[test]
    fn accepts_site_session_format_only() {
        assert!(cookie_looks_like_kick_token("123|abcdefghijklmnop123456"));
        assert!(!cookie_looks_like_kick_token("Bearer 123|abcdefghijklmnop123456"));
        assert!(!cookie_looks_like_kick_token("123|short"));
        assert!(!cookie_looks_like_kick_token("123|hello there and a space"));
        assert!(!cookie_looks_like_kick_token("abc|abcdefghijklmnop123456"));
    }
}
