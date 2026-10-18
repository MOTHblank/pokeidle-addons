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

// Windows can link against the built-in WinSQLite3 API. On Linux, resolve
// the distro's versioned SQLite runtime at first use. This avoids requiring a
// Linux development sysroot/libsqlite3.so during Windows -> Linux zigbuild,
// without bundling a separate SQLite engine or reading cookies as text.
#[cfg(not(target_os = "linux"))]
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

type Open = unsafe extern "C" fn(*const c_char, *mut *mut c_void, c_int, *const c_char) -> c_int;
type Close = unsafe extern "C" fn(*mut c_void) -> c_int;
type Prepare = unsafe extern "C" fn(*mut c_void, *const c_char, c_int,
    *mut *mut c_void, *mut *const c_char) -> c_int;
type Step = unsafe extern "C" fn(*mut c_void) -> c_int;
type Finalize = unsafe extern "C" fn(*mut c_void) -> c_int;
type ColumnText = unsafe extern "C" fn(*mut c_void, c_int) -> *const u8;
type ColumnBytes = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type ColumnInt64 = unsafe extern "C" fn(*mut c_void, c_int) -> i64;

struct SqliteApi {
    open: Open,
    close: Close,
    prepare: Prepare,
    step: Step,
    finalize: Finalize,
    column_text: ColumnText,
    column_bytes: ColumnBytes,
    column_int64: ColumnInt64,
}

#[cfg(not(target_os = "linux"))]
fn sqlite_api() -> Result<&'static SqliteApi, String> {
    static API: SqliteApi = SqliteApi {
        open: sqlite3_open_v2,
        close: sqlite3_close,
        prepare: sqlite3_prepare_v2,
        step: sqlite3_step,
        finalize: sqlite3_finalize,
        column_text: sqlite3_column_text,
        column_bytes: sqlite3_column_bytes,
        column_int64: sqlite3_column_int64,
    };
    Ok(&API)
}

#[cfg(target_os = "linux")]
#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

#[cfg(target_os = "linux")]
fn sqlite_api() -> Result<&'static SqliteApi, String> {
    static API: OnceLock<Result<SqliteApi, &'static str>> = OnceLock::new();
    API.get_or_init(|| {
        // RTLD_NOW = 2 on Linux. Load the versioned SONAME available in
        // regular runtime packages, not the unversioned -dev symlink.
        let handle = unsafe { dlopen(c"libsqlite3.so.0".as_ptr(), 2) };
        if handle.is_null() {
            return Err("Kick login requires libsqlite3.so.0 (install libsqlite3-0)");
        }
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                let address = unsafe { dlsym(handle, concat!($name, "\0").as_ptr().cast()) };
                if address.is_null() {
                    return Err("Installed SQLite is missing required symbols");
                }
                // Linux/POSIX dlsym yields a C function address. Only call it
                // with the exact ABI and signature declared for that symbol.
                unsafe { std::mem::transmute::<*mut c_void, $ty>(address) }
            }};
        }
        let loaded = (|| -> Result<SqliteApi, &'static str> {
            Ok(SqliteApi {
                open: load!("sqlite3_open_v2", Open),
                close: load!("sqlite3_close", Close),
                prepare: load!("sqlite3_prepare_v2", Prepare),
                step: load!("sqlite3_step", Step),
                finalize: load!("sqlite3_finalize", Finalize),
                column_text: load!("sqlite3_column_text", ColumnText),
                column_bytes: load!("sqlite3_column_bytes", ColumnBytes),
                column_int64: load!("sqlite3_column_int64", ColumnInt64),
            })
        })();
        if loaded.is_err() {
            unsafe { dlclose(handle); }
        }
        // On success the library stays resident, keeping all function
        // pointers valid through Database/Statement Drop and across threads.
        loaded
    }).as_ref().map_err(|message| (*message).to_owned())
}

struct Database {
    raw: *mut c_void,
    api: &'static SqliteApi,
}
impl Drop for Database {
    fn drop(&mut self) { unsafe { (self.api.close)(self.raw); } }
}
struct Statement {
    raw: *mut c_void,
    api: &'static SqliteApi,
}
impl Drop for Statement {
    fn drop(&mut self) { unsafe { (self.api.finalize)(self.raw); } }
}

fn cookie_looks_like_kick_token(value: &str) -> bool {
    let Some((id, secret)) = value.split_once('|') else { return false };
    !id.is_empty() && id.len() <= 18 && id.bytes().all(|b| b.is_ascii_digit())
        && secret.len() >= 16 && value.len() <= 8192
        && secret.bytes().all(|b| b.is_ascii_graphic())
}

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

/// Return one unexpired session credential from the selected account's
/// first-party Kick cookie database. Do not touch other browser data.
fn read_kick_cookie(profile_dir: &Path) -> Result<Option<String>, String> {
    let path = profile_dir.join("cookies.sqlite");
    if !path.is_file() { return Ok(None); }
    let path = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| "Invalid Kick Firefox profile path".to_owned())?;
    let api = sqlite_api()?;
    let mut raw = ptr::null_mut();
    let rc = unsafe {
        (api.open)(path.as_ptr(), &mut raw, SQLITE_OPEN_READONLY, ptr::null())
    };
    if rc != SQLITE_OK {
        if !raw.is_null() { unsafe { (api.close)(raw); } }
        return Err("Could not open the Kick Firefox cookie database".into());
    }
    let db = Database { raw, api };
    // Values are never used to construct SQL; restrict to the domain and the
    // specific session cookie, not Cloudflare or third-party cookies.
    let sql = c"SELECT value, expiry FROM moz_cookies WHERE name = 'session_token'         AND host IN ('kick.com', '.kick.com', 'www.kick.com', '.www.kick.com')         ORDER BY expiry DESC";
    let mut stmt = ptr::null_mut();
    if unsafe { (api.prepare)(db.raw, sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) } != SQLITE_OK {
        return Err("Kick Firefox cookies database has an unsupported schema".into());
    }
    let stmt = Statement { raw: stmt, api };
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs() as i64;
    loop {
        match unsafe { (api.step)(stmt.raw) } {
            SQLITE_ROW => {
                let expiry = unsafe { (api.column_int64)(stmt.raw, 1) };
                if expiry > 0 && expiry <= now { continue; }
                let length = unsafe { (api.column_bytes)(stmt.raw, 0) };
                if !(1..=8192).contains(&length) { continue; }
                let text = unsafe { (api.column_text)(stmt.raw, 0) };
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
    // The browser stays uninstrumented. Watch the local Firefox cookie
    // database; no site scripts, Remote Agent or BiDi are involved.
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
            // The OS vault now owns the login. Close only the temporary
            // Firefox process started by this button, not another account.
            let _ = firefox::stop_child(&mut browser, None);
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
        "Kick login not detected. Sign in using Firefox; Moth will import automatically.".to_owned()
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
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_sqlite_runtime_symbols_resolve() {
        // Loading must not require the unversioned libsqlite3.so development
        // symlink. The shared library stays loaded for the process lifetime.
        let api = super::sqlite_api().expect("install libsqlite3-0 for Kick login");
        assert_ne!(api.open as usize, 0);
        assert_ne!(api.finalize as usize, 0);
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
