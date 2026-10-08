use crate::config::{Config, GameProfile};
use crate::logging;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct KickStream {
    pub name: String,
    pub url: String,
}

#[cfg(windows)]
type Hwnd = *mut std::ffi::c_void;

#[cfg(windows)]
const WM_CLOSE: u32 = 0x0010;

#[cfg(windows)]
#[derive(Default)]
struct EnumWindowsContext {
    windows: Vec<Hwnd>,
}

#[cfg(windows)]
unsafe extern "system" fn collect_window_callback(
    hwnd: Hwnd,
    lparam: isize,
) -> i32 {
    let context = &mut *(lparam as *mut EnumWindowsContext);

    if IsWindowVisible(hwnd) != 0 {
        context.windows.push(hwnd);
    }

    1
}

#[cfg(windows)]
unsafe extern "system" {
    fn EnumWindows(
        callback: Option<unsafe extern "system" fn(Hwnd, isize) -> i32>,
        lparam: isize,
    ) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn IsWindow(hwnd: Hwnd) -> i32;
    fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;
    fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
    fn PostMessageW(hwnd: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
}

#[cfg(windows)]
fn visible_windows() -> Vec<Hwnd> {
    let mut context = EnumWindowsContext::default();

    unsafe {
        let _ = EnumWindows(
            Some(collect_window_callback),
            &mut context as *mut _ as isize,
        );
    }

    context.windows
}

#[cfg(windows)]
fn window_pid(hwnd: Hwnd) -> u32 {
    let mut pid = 0u32;

    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, &mut pid);
    }

    pid
}

#[cfg(windows)]
fn window_title(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 1024];

    let length = unsafe {
        GetWindowTextW(
            hwnd,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };

    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

#[cfg(windows)]
fn window_is_valid(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 }
}

#[cfg(windows)]
fn close_window(hwnd: Hwnd) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

#[cfg(windows)]
fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(windows)]
fn snapshot_windows() -> HashSet<Hwnd> {
    visible_windows().into_iter().collect()
}

#[cfg(windows)]
fn find_kick_window(
    candidate_windows: impl IntoIterator<Item = Hwnd>,
    browser_pid: Option<u32>,
    channel: &str,
) -> Option<Hwnd> {
    let needle = normalized(channel);

    for hwnd in candidate_windows {
        if !window_is_valid(hwnd) {
            continue;
        }

        if let Some(pid) = browser_pid {
            if window_pid(hwnd) != pid {
                continue;
            }
        }

        let title = normalized(&window_title(hwnd));

        if title.contains("kick") && title.contains(&needle) {
            return Some(hwnd);
        }
    }

    None
}

#[cfg(windows)]
fn find_new_kick_window(
    before: &HashSet<Hwnd>,
    browser_pid: u32,
    channel: &str,
) -> Option<Hwnd> {
    let current = visible_windows();

    if let Some(hwnd) = find_kick_window(
        current.iter().copied().filter(|hwnd| !before.contains(hwnd)),
        Some(browser_pid),
        channel,
    ) {
        return Some(hwnd);
    }

    current
        .into_iter()
        .filter(|hwnd| !before.contains(hwnd))
        .filter(|hwnd| window_is_valid(*hwnd) && window_pid(*hwnd) == browser_pid)
        .find(|hwnd| {
            let title = normalized(&window_title(*hwnd));
            title.contains("kick") || title.contains(&normalized(channel))
        })
}

#[derive(Default)]
struct KickSession {
    profile_dir: Option<PathBuf>,
    browser_executable: Option<PathBuf>,
    browser_pid: Option<u32>,
    windows: HashMap<String, Hwnd>,
}

pub struct KickManager {
    sessions: [KickSession; 4],
}

impl KickManager {
    pub fn new() -> Self {
        Self {
            sessions: std::array::from_fn(|_| KickSession::default()),
        }
    }

    pub fn sync(
        &mut self,
        profile: GameProfile,
        streams: &[KickStream],
        state_available: bool,
    ) {
        #[cfg(not(windows))]
        {
            let _ = (profile, streams, state_available);
            return;
        }

        #[cfg(windows)]
        {
            let Ok(config) = Config::for_profile(profile) else {
                logging::warn(&format!("KICK profile {} could not be configured", profile.label()));
                return;
            };
            self.configure(profile, &config);
            self.sync_windows(profile, streams, state_available);
        }
    }

    pub fn open_login(
        &mut self,
        profile: GameProfile,
        url: &str,
    ) -> Result<(), String> {
        #[cfg(not(windows))]
        {
            let _ = (profile, url);
            return Err("native KICK browser is only supported on Windows".to_string());
        }

        #[cfg(windows)]
        {
            let config = Config::for_profile(profile)?;
            self.configure(profile, &config);
            self.open_url(profile, url).map(|_| ())
        }
    }

    pub fn shutdown(&mut self) {
        #[cfg(windows)]
        {
            for session in &mut self.sessions {
                for hwnd in session.windows.values().copied() {
                    if window_is_valid(hwnd) {
                        close_window(hwnd);
                    }
                }

                session.windows.clear();
                session.browser_pid = None;
            }
        }
    }

    #[cfg(windows)]
    fn session_mut(&mut self, profile: GameProfile) -> &mut KickSession {
        &mut self.sessions[profile.index()]
    }

    #[cfg(windows)]
    fn sync_windows(
        &mut self,
        profile: GameProfile,
        streams: &[KickStream],
        state_available: bool,
    ) {
        let session = self.session_mut(profile);

        let mut desired = HashMap::new();
        for stream in streams.iter().take(10) {
            let key = normalized(&stream.name);
            if key.is_empty() || stream.url.is_empty() {
                continue;
            }
            desired.insert(key, stream.clone());
        }

        session.windows.retain(|_, hwnd| window_is_valid(*hwnd));

        let browser_pid = session.browser_pid;

        for stream in desired.values() {
            let key = normalized(&stream.name);

            if session.windows.contains_key(&key) {
                continue;
            }

            let existing = find_kick_window(
                visible_windows(),
                browser_pid,
                &stream.name,
            );

            if let Some(hwnd) = existing {
                session.windows.insert(key, hwnd);
                continue;
            }

            match Self::open_stream_window(session, stream) {
                Ok(hwnd) => {
                    session.windows.insert(key, hwnd);
                    logging::info(&format!(
                        "KICK opened in normal Firefox: {}",
                        stream.url
                    ));
                }
                Err(error) => {
                    logging::warn(&format!(
                        "KICK stream open failed for {}: {}",
                        stream.name, error
                    ));
                }
            }
        }

        if state_available {
            let live_keys: HashSet<String> = desired.keys().cloned().collect();

            let stale = session
                .windows
                .iter()
                .filter_map(|(key, hwnd)| {
                    if live_keys.contains(key) {
                        None
                    } else {
                        Some((key.clone(), *hwnd))
                    }
                })
                .collect::<Vec<_>>();

            for (key, hwnd) in stale {
                if window_is_valid(hwnd) {
                    close_window(hwnd);
                    logging::info(&format!("KICK closed offline stream window: {}", key));
                }
                session.windows.remove(&key);
            }
        }
    }

    #[cfg(windows)]
    fn open_url(
        &mut self,
        profile: GameProfile,
        url: &str,
    ) -> Result<Hwnd, String> {
        let session = self.session_mut(profile);

        let stream = KickStream {
            name: "login".to_string(),
            url: url.to_string(),
        };

        Self::open_stream_window(session, &stream)
    }

    #[cfg(windows)]
    fn open_stream_window(
        session: &mut KickSession,
        stream: &KickStream,
    ) -> Result<Hwnd, String> {
        if stream.url.is_empty() {
            return Err("empty KICK URL".to_string());
        }

        if !stream.url.starts_with("https://kick.com/") {
            return Err("refusing to open a non-KICK HTTPS URL".to_string());
        }

        let profile_dir = session
            .profile_dir
            .clone()
            .ok_or_else(|| "KICK browser profile is not initialized".to_string())?;

        let executable = session
            .browser_executable
            .clone()
            .ok_or_else(|| "KICK Firefox executable is not initialized".to_string())?;

        std::fs::create_dir_all(&profile_dir)
            .map_err(|error| format!("could not create KICK profile: {error}"))?;

        let before = snapshot_windows();

        if session
            .browser_pid
            .map(|pid| !process_alive(pid))
            .unwrap_or(true)
        {
            let child = Command::new(&executable)
                .arg("--profile")
                .arg(&profile_dir)
                .arg("--new-window")
                .arg(&stream.url)
                .spawn()
                .map_err(|error| format!("could not start normal KICK Firefox: {error}"))?;

            session.browser_pid = Some(child.id());

            for _ in 0..60 {
                if let Some(hwnd) = find_new_kick_window(
                    &before,
                    child.id(),
                    &stream.name,
                ) {
                    return Ok(hwnd);
                }

                thread::sleep(Duration::from_millis(100));
            }

            return Err("KICK Firefox started but its window was not detected".to_string());
        }

        let child = Command::new(&executable)
            .arg("--profile")
            .arg(&profile_dir)
            .arg("--new-window")
            .arg(&stream.url)
            .spawn()
            .map_err(|error| format!("could not request a new KICK Firefox window: {error}"))?;

        let _ = child;

        for _ in 0..60 {
            if let Some(hwnd) = find_new_kick_window(
                &before,
                session.browser_pid.unwrap_or(0),
                &stream.name,
            ) {
                return Ok(hwnd);
            }

            thread::sleep(Duration::from_millis(100));
        }

        Err("KICK Firefox did not create the requested window".to_string())
    }

    #[cfg(windows)]
    fn configure(&mut self, profile: GameProfile, config: &Config) {
        let session = self.session_mut(profile);
        session.profile_dir = Some(config.kick_profile_dir());
        session.browser_executable = Some(config.firefox_executable.clone());
    }
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    unsafe extern "system" {
        fn OpenProcess(
            desired_access: u32,
            inherit_handle: i32,
            process_id: u32,
        ) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetExitCodeProcess(
            process: *mut std::ffi::c_void,
            exit_code: *mut u32,
        ) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return false;
        }

        let mut exit_code = 0u32;
        let ok = GetExitCodeProcess(process, &mut exit_code) != 0;
        let _ = CloseHandle(process);

        ok && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(windows))]
fn process_alive(_pid: u32) -> bool {
    false
}

impl Default for KickManager {
    fn default() -> Self {
        Self::new()
    }
}
