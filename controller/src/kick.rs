use crate::config::{Config, GameProfile};
use crate::logging;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

#[cfg(windows)]
type BrowserSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[cfg(windows)]
type Hwnd = *mut std::ffi::c_void;

#[cfg(windows)]
const WM_CLOSE: u32 = 0x0010;

#[cfg(windows)]
unsafe extern "system" fn collect_window_callback(hwnd: Hwnd, lparam: isize) -> i32 {
    unsafe {
        let windows = &mut *(lparam as *mut Vec<Hwnd>);
        if IsWindowVisible(hwnd) != 0 {
            windows.push(hwnd);
        }
    }
    1
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: Option<unsafe extern "system" fn(Hwnd, isize) -> i32>,
        lparam: isize,
    ) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn IsWindow(hwnd: Hwnd) -> i32;
    fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
    fn PostMessageW(hwnd: Hwnd, message: u32, wparam: usize, lparam: isize) -> i32;
}

#[cfg(windows)]
fn visible_windows() -> Vec<Hwnd> {
    let mut windows = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(collect_window_callback),
            &mut windows as *mut _ as isize,
        );
    }
    windows
}

#[cfg(windows)]
fn window_is_valid(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 }
}

#[cfg(windows)]
fn window_title(hwnd: Hwnd) -> String {
    let mut buffer = [0u16; 1024];
    let length = unsafe {
        GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32)
    };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

#[cfg(windows)]
fn window_snapshot() -> HashSet<Hwnd> {
    visible_windows().into_iter().collect()
}

#[cfg(windows)]
fn find_new_browser_window(before: &HashSet<Hwnd>) -> Option<Hwnd> {
    let current = visible_windows();

    current
        .iter()
        .copied()
        .filter(|hwnd| !before.contains(hwnd) && window_is_valid(*hwnd))
        .find(|hwnd| window_title(*hwnd).to_ascii_lowercase().contains("kick"))
        .or_else(|| {
            current
                .into_iter()
                .filter(|hwnd| !before.contains(hwnd) && window_is_valid(*hwnd))
                .next()
        })
}

#[cfg(windows)]
fn close_window(hwnd: Hwnd) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

#[cfg(windows)]
fn remote_port_open(port: u16) -> bool {
    let Some(address) = ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
    else {
        return false;
    };

    TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok()
}

#[derive(Clone, Debug)]
pub struct KickStream {
    pub name: String,
    pub url: String,
}

#[cfg(windows)]
#[derive(Default)]
struct KickSession {
    profile_dir: Option<PathBuf>,
    browser_executable: Option<PathBuf>,
    browser_pid: Option<u32>,
    managed: bool,
    headless: bool,
    browser_hwnd: Option<Hwnd>,
    launch_started: Option<Instant>,
    launch_windows: Option<HashSet<Hwnd>>,
    socket: Option<BrowserSocket>,
    next_id: u64,
    tabs: HashMap<String, String>,
    pending: HashMap<String, Instant>,
}

#[cfg(not(windows))]
#[derive(Default)]
struct KickSession;

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
        headless: bool,
    ) {
        #[cfg(not(windows))]
        {
            let _ = (profile, streams, state_available, headless);
            return;
        }

        #[cfg(windows)]
        {
            let Ok(config) = Config::for_profile(profile) else {
                logging::warn(&format!(
                    "KICK profile {} could not be configured",
                    profile.label()
                ));
                return;
            };

            self.configure(profile, &config);

            if let Err(error) = self.sync_tabs(profile, streams, state_available, headless) {
                logging::warn(&format!(
                    "KICK tab sync failed for {}: {}",
                    profile.label(),
                    error
                ));
            }
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
            if !url.starts_with("https://kick.com/") {
                return Err("refusing to open a non-KICK HTTPS URL".to_string());
            }

            let config = Config::for_profile(profile)?;
            self.configure(profile, &config);

            self.open_unmanaged_login(profile, url)
        }
    }

    pub fn set_headless_test(&mut self, profile: GameProfile, enabled: bool) {
        #[cfg(not(windows))]
        {
            let _ = (profile, enabled);
        }

        #[cfg(windows)]
        {
            let current = self.session_mut(profile).headless;
            if current == enabled {
                return;
            }

            if self.session_mut(profile).managed {
                self.stop_managed_browser(profile);
            }

            let session = self.session_mut(profile);
            session.headless = enabled;
            session.managed = false;
            session.socket = None;
            session.tabs.clear();
            session.pending.clear();
            session.browser_pid = None;
            session.browser_hwnd = None;
            session.launch_started = None;
            session.launch_windows = None;
        }
    }

    pub fn shutdown(&mut self) {
        #[cfg(windows)]
        {
            for profile in GameProfile::ALL {
                let _ = self.close_all_managed_tabs(profile);

                let (headless, browser_hwnd, browser_pid) = {
                    let session = self.session_mut(profile);
                    (session.headless, session.browser_hwnd, session.browser_pid)
                };

                if let Some(hwnd) = browser_hwnd {
                    if window_is_valid(hwnd) {
                        close_window(hwnd);
                    }
                }

                if headless {
                    if let Some(pid) = browser_pid {
                        let _ = Command::new("taskkill")
                            .arg("/PID")
                            .arg(pid.to_string())
                            .arg("/T")
                            .arg("/F")
                            .status();
                    }
                }

                let session = self.session_mut(profile);
                if let Some(socket) = session.socket.as_mut() {
                    let id = session.next_id;
                    session.next_id += 1;
                    let _ = send_and_wait(
                        socket,
                        id,
                        json!({
                            "id": id,
                            "method": "session.end",
                            "params": {}
                        }),
                    );
                    let _ = socket.close(None);
                }
                session.socket = None;
                session.tabs.clear();
                session.pending.clear();
                session.browser_pid = None;
                session.browser_hwnd = None;
                session.launch_started = None;
                session.launch_windows = None;
                session.managed = false;
                session.headless = false;
            }
        }
    }

    #[cfg(windows)]
    fn session_mut(&mut self, profile: GameProfile) -> &mut KickSession {
        &mut self.sessions[profile.index()]
    }

    #[cfg(windows)]
    fn next_id(&mut self, profile: GameProfile) -> u64 {
        let session = self.session_mut(profile);
        if session.next_id == 0 {
            session.next_id = 1;
        }
        let id = session.next_id;
        session.next_id += 1;
        id
    }

    #[cfg(windows)]
    fn configure(&mut self, profile: GameProfile, config: &Config) {
        let session = self.session_mut(profile);
        session.profile_dir = Some(config.kick_profile_dir());
        session.browser_executable = Config::kick_firefox_executable().ok();
    }

    #[cfg(windows)]
    fn ensure_browser(
        &mut self,
        profile: GameProfile,
        initial_url: &str,
        headless: bool,
    ) -> Result<bool, String> {
        let (profile_dir, executable) = {
            let session = self.session_mut(profile);
            (
                session.profile_dir.clone().ok_or_else(|| "KICK browser profile is not initialized".to_string())?,
                session.browser_executable.clone().ok_or_else(|| "KICK Firefox executable is not initialized".to_string())?,
            )
        };

        std::fs::create_dir_all(&profile_dir)
            .map_err(|error| format!("could not create KICK profile: {error}"))?;

        self.discover_browser_window(profile);

        let launch_recent = self
            .session_mut(profile)
            .launch_started
            .map(|started| started.elapsed() < Duration::from_secs(120))
            .unwrap_or(false);

        let port = 27801 + profile.index() as u16;

        if self.session_mut(profile).managed {
            if self.session_mut(profile).headless != headless {
                return Ok(false);
            }

            if self.session_mut(profile).socket.is_some() {
                return Ok(true);
            }

            // The BiDi endpoint is the authoritative identity of a managed
            // KICK browser. Never launch another one while that port exists.
            if remote_port_open(port) {
                // A newly launched Firefox already owns this port. Connecting
                // to it is safe while startup is recent; delaying here leaves
                // the first page open but prevents the remaining KICK tabs
                // from being created until window detection or a 120s timeout.
                if self.connect(profile, port).is_ok() {
                    return Ok(true);
                }
                return Ok(false);
            }

            if self.session_browser_exists(profile) || launch_recent {
                return Ok(false);
            }

            self.clear_browser_tracking(profile);
        } else if self.session_browser_exists(profile) || launch_recent {
            // Uncontrolled login browser. The stream manager must not touch it.
            return Ok(false);
        }

        let launch_windows = window_snapshot();

        let mut command = Command::new(&executable);
        command
            .arg("--no-remote")
            .arg(format!("--remote-debugging-port={}", port));
        if headless {
            command.arg("--headless");
        }

        command
            .arg("--profile")
            .arg(&profile_dir)
            .arg("--new-window")
            .arg(initial_url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let child = command
            .spawn()
            .map_err(|error| format!("could not start managed KICK Firefox: {error}"))?;

        let session = self.session_mut(profile);
        session.browser_pid = Some(child.id());
        session.managed = true;
        session.headless = headless;
        session.browser_hwnd = None;
        session.launch_started = Some(Instant::now());
        session.launch_windows = Some(launch_windows);
        session.socket = None;
        session.tabs.clear();
        session.pending.clear();

        logging::info(&format!(
            "KICK starting {} managed Firefox for {}",
            if headless { "headless" } else { "visible" },
            profile.label()
        ));

        Ok(false)
    }

    #[cfg(windows)]
    fn discover_browser_window(&mut self, profile: GameProfile) {
        if self.session_mut(profile).headless {
            return;
        }

        let before = self.session_mut(profile).launch_windows.clone();
        let Some(before) = before else { return; };

        if let Some(hwnd) = find_new_browser_window(&before) {
            let session = self.session_mut(profile);
            session.browser_hwnd = Some(hwnd);
            session.launch_windows = None;
            session.launch_started = None;
        }
    }

    #[cfg(windows)]
    fn session_browser_exists(&mut self, profile: GameProfile) -> bool {
        let session = self.session_mut(profile);
        if let Some(hwnd) = session.browser_hwnd {
            if window_is_valid(hwnd) {
                return true;
            }
            session.browser_hwnd = None;
        }
        false
    }

    #[cfg(windows)]
    fn clear_browser_tracking(&mut self, profile: GameProfile) {
        let session = self.session_mut(profile);
        session.browser_pid = None;
        session.browser_hwnd = None;
        session.launch_started = None;
        session.launch_windows = None;
        session.socket = None;
        session.tabs.clear();
        session.pending.clear();
    }

    #[cfg(windows)]
    fn open_unmanaged_login(
        &mut self,
        profile: GameProfile,
        url: &str,
    ) -> Result<(), String> {
        let (profile_dir, executable, managed) = {
            let session = self.session_mut(profile);
            (
                session
                    .profile_dir
                    .clone()
                    .ok_or_else(|| "KICK browser profile is not initialized".to_string())?,
                session
                    .browser_executable
                    .clone()
                    .ok_or_else(|| "KICK Firefox executable is not initialized".to_string())?,
                session.managed,
            )
        };

        if managed {
            return Err(
                "KICK is currently managed by the controller. Close its managed Firefox window before starting KICK login."
                    .to_string(),
            );
        }

        std::fs::create_dir_all(&profile_dir)
            .map_err(|error| format!("could not create KICK profile: {error}"))?;

        self.discover_browser_window(profile);

        if self.session_browser_exists(profile) {
            Command::new(&executable)
                .arg("--profile")
                .arg(&profile_dir)
                .arg("--new-tab")
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|error| format!("could not open KICK login tab: {error}"))?;

            logging::info(&format!(
                "KICK login opened as a tab in the existing uncontrolled Firefox for {}",
                profile.label()
            ));
            return Ok(());
        }

        let launch_windows = window_snapshot();
        Command::new(&executable)
            .arg("--no-remote")
            .arg("--profile")
            .arg(&profile_dir)
            .arg("--new-window")
            .arg(url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| format!("could not start uncontrolled KICK login Firefox: {error}"))?;

        let session = self.session_mut(profile);
        session.managed = false;
        session.headless = false;
        session.browser_hwnd = None;
        session.launch_started = Some(Instant::now());
        session.launch_windows = Some(launch_windows);
        session.socket = None;
        session.tabs.clear();
        session.pending.clear();

        logging::info(&format!(
            "KICK login opened in uncontrolled normal Firefox for {}",
            profile.label()
        ));

        Ok(())
    }
    #[cfg(windows)]
    fn stop_managed_browser(&mut self, profile: GameProfile) {
        let (headless, hwnd, pid) = {
            let session = self.session_mut(profile);
            (session.headless, session.browser_hwnd, session.browser_pid)
        };

        let session_id = self.next_id(profile);
        if let Some(socket) = self.session_mut(profile).socket.as_mut() {
            let _ = send_and_wait(
                socket,
                session_id,
                json!({
                    "id": session_id,
                    "method": "session.end",
                    "params": {}
                }),
            );
        }

        if headless {
            if let Some(pid) = pid {
                let _ = Command::new("taskkill")
                    .arg("/PID").arg(pid.to_string())
                    .arg("/T").arg("/F")
                    .status();
            }
        } else if let Some(hwnd) = hwnd {
            if window_is_valid(hwnd) {
                close_window(hwnd);
            }
        }

        self.clear_browser_tracking(profile);
        let session = self.session_mut(profile);
        session.managed = false;
    }

    #[cfg(windows)]
    fn connect(&mut self, profile: GameProfile, port: u16) -> Result<(), String> {
        let (mut socket, _) = connect(format!("ws://127.0.0.1:{port}/session"))
            .map_err(|error| error.to_string())?;

        send_and_wait(
            &mut socket,
            1,
            json!({
                "id": 1,
                "method": "session.new",
                "params": {
                    "capabilities": {}
                }
            }),
        )?;

        let session = self.session_mut(profile);
        session.socket = Some(socket);
        session.managed = true;
        session.next_id = 2;

        logging::info(&format!(
            "KICK normal Firefox connected on BiDi port {} for {}",
            port,
            profile.label()
        ));

        Ok(())
    }

    #[cfg(windows)]
    fn sync_tabs(
        &mut self,
        profile: GameProfile,
        streams: &[KickStream],
        state_available: bool,
        headless: bool,
    ) -> Result<(), String> {
        let mut desired = HashMap::new();

        for stream in streams.iter() {
            let key = normalize(&stream.name);
            if key.is_empty() || !is_kick_url(&stream.url) {
                continue;
            }
            desired.insert(key, stream.url.clone());
        }

        if desired.is_empty() {
            // Nothing to open. When the controller already owns a KICK browser,
            // reconcile/close stale managed tabs; otherwise leave the user's
            // uncontrolled authentication browser completely alone.
            if !state_available {
                return Ok(());
            }

            if self.session_mut(profile).socket.is_none() {
                return Ok(());
            }
        }

        let initial_url = desired
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "https://kick.com/".to_string());

        if !self.ensure_browser(profile, &initial_url, headless)? {
            return Ok(());
        }

        let contexts = self.contexts(profile)?;
        let mut current = HashMap::new();

        for context in contexts {
            let Some(channel) = kick_channel_from_url(&context.url) else {
                continue;
            };

            if desired.contains_key(&channel) && !current.contains_key(&channel) {
                current.insert(channel, context.id);
            }
        }

        {
            let session = self.session_mut(profile);
            session.tabs = current.clone();
            session.pending.retain(|key, _| desired.contains_key(key));
        }

        for (key, url) in &desired {
            if current.contains_key(key) {
                continue;
            }

            let retry_blocked = {
                let session = self.session_mut(profile);
                session
                    .pending
                    .get(key)
                    .map(|started| started.elapsed() < Duration::from_secs(15))
                    .unwrap_or(false)
            };

            if retry_blocked {
                continue;
            }

            self.session_mut(profile)
                .pending
                .insert(key.clone(), Instant::now());

            if let Err(error) = self.create_stream_tab(profile, key, url) {
                logging::warn(&format!(
                    "KICK stream tab creation failed for {}: {}",
                    key, error
                ));
            }
        }

        if state_available {
            let stale = {
                let session = self.session_mut(profile);
                session
                    .tabs
                    .keys()
                    .filter(|key| !desired.contains_key(*key))
                    .cloned()
                    .collect::<Vec<_>>()
            };

            for key in stale {
                self.close_stream_tab(profile, &key)?;
            }
        }

        let tracked_keys: HashSet<String> = {
            let session = self.session_mut(profile);
            session.tabs.keys().cloned().collect()
        };

        let session = self.session_mut(profile);
        session
            .pending
            .retain(|key, _| desired.contains_key(key) && !tracked_keys.contains(key));

        Ok(())
    }

    #[cfg(windows)]
    fn contexts(&mut self, profile: GameProfile) -> Result<Vec<KickContext>, String> {
        let id = self.next_id(profile);
        let response = {
            let session = self.session_mut(profile);
            let socket = session
                .socket
                .as_mut()
                .ok_or_else(|| "KICK Firefox remote control is not connected".to_string())?;

            send_and_wait(
                socket,
                id,
                json!({
                    "id": id,
                    "method": "browsingContext.getTree",
                    "params": {
                        "maxDepth": 10
                    }
                }),
            )?
        };

        let mut contexts = Vec::new();
        if let Some(list) = response
            .get("result")
            .and_then(|value| value.get("contexts"))
            .and_then(Value::as_array)
        {
            for value in list {
                collect_contexts(value, &mut contexts);
            }
        }

        Ok(contexts)
    }

    #[cfg(windows)]
    fn create_stream_tab(
        &mut self,
        profile: GameProfile,
        key: &str,
        url: &str,
    ) -> Result<(), String> {
        let id = self.next_id(profile);
        let response = {
            let session = self.session_mut(profile);
            let socket = session
                .socket
                .as_mut()
                .ok_or_else(|| "KICK Firefox remote control is not connected".to_string())?;

            send_and_wait(
                socket,
                id,
                json!({
                    "id": id,
                    "method": "browsingContext.create",
                    "params": {
                        "type": "tab"
                    }
                }),
            )?
        };

        let context = response
            .get("result")
            .and_then(|value| value.get("context"))
            .and_then(Value::as_str)
            .ok_or_else(|| "KICK tab creation returned no context".to_string())?
            .to_string();

        let navigate_id = self.next_id(profile);
        let navigation_result = {
            let session = self.session_mut(profile);
            let socket = session
                .socket
                .as_mut()
                .ok_or_else(|| "KICK Firefox remote control is not connected".to_string())?;

            send_and_wait(
                socket,
                navigate_id,
                json!({
                    "id": navigate_id,
                    "method": "browsingContext.navigate",
                    "params": {
                        "context": context,
                        "url": url,
                        "wait": "none"
                    }
                })
            )
        };

        if let Err(error) = navigation_result {
            let close_id = self.next_id(profile);
            if let Some(socket) = self.session_mut(profile).socket.as_mut() {
                let _ = send_and_wait(
                    socket,
                    close_id,
                    json!({
                        "id": close_id,
                        "method": "browsingContext.close",
                        "params": {
                            "context": context
                        }
                    })
                );
            }
            return Err(error);
        }

        let session = self.session_mut(profile);
        session.tabs.insert(key.to_string(), context);
        session.pending.remove(key);

        logging::info(&format!(
            "KICK opened stream as tab in normal Firefox: {}",
            url
        ));

        Ok(())
    }

    #[cfg(windows)]
    fn close_stream_tab(
        &mut self,
        profile: GameProfile,
        key: &str,
    ) -> Result<(), String> {
        let context = {
            let session = self.session_mut(profile);
            session.tabs.remove(key)
        };

        let Some(context) = context else {
            return Ok(());
        };

        let id = self.next_id(profile);
        let session = self.session_mut(profile);
        let socket = session
            .socket
            .as_mut()
            .ok_or_else(|| "KICK Firefox remote control is not connected".to_string())?;

        send_and_wait(
            socket,
            id,
            json!({
                "id": id,
                "method": "browsingContext.close",
                "params": {
                    "context": context
                }
            }),
        )?;

        logging::info(&format!(
            "KICK closed offline stream tab: {}",
            key
        ));

        Ok(())
    }

    #[cfg(windows)]
    fn close_all_managed_tabs(&mut self, profile: GameProfile) -> Result<(), String> {
        let keys = {
            let session = self.session_mut(profile);
            session.tabs.keys().cloned().collect::<Vec<_>>()
        };

        for key in keys {
            let _ = self.close_stream_tab(profile, &key);
        }

        Ok(())
    }
}

#[cfg(windows)]
#[derive(Clone)]
struct KickContext {
    id: String,
    url: String,
}

#[cfg(windows)]
fn collect_contexts(value: &Value, out: &mut Vec<KickContext>) {
    let Some(id) = value.get("context").and_then(Value::as_str) else {
        return;
    };

    out.push(KickContext {
        id: id.to_string(),
        url: value
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    });

    if let Some(children) = value.get("children").and_then(Value::as_array) {
        for child in children {
            collect_contexts(child, out);
        }
    }
}

#[cfg(windows)]
fn normalize(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .trim_matches('/')
        .to_string()
}

#[cfg(windows)]
fn is_kick_url(url: &str) -> bool {
    url.starts_with("https://kick.com/")
}

#[cfg(windows)]
fn kick_channel_from_url(url: &str) -> Option<String> {
    let value = url.strip_prefix("https://kick.com/")?;
    let path = value.split(['?', '#', '/']).next().unwrap_or_default();
    let channel = normalize(path);

    if channel.is_empty() {
        None
    } else {
        Some(channel)
    }
}

fn send_and_wait<S>(
    socket: &mut WebSocket<S>,
    expected_id: u64,
    command: Value,
) -> Result<Value, String>
where
    S: Read + Write,
{
    socket
        .send(Message::Text(command.to_string().into()))
        .map_err(|error| error.to_string())?;

    loop {
        let message = socket.read().map_err(|error| error.to_string())?;

        let Message::Text(text) = message else {
            continue;
        };

        let value: Value = serde_json::from_str(text.as_ref())
            .map_err(|error| format!("invalid KICK BiDi JSON: {error}"))?;

        if value.get("id").and_then(Value::as_u64) != Some(expected_id) {
            continue;
        }

        if value.get("type").and_then(Value::as_str) == Some("error") {
            return Err(value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("KICK Firefox BiDi command failed")
                .to_string());
        }

        if value
            .get("result")
            .and_then(|result| result.get("type"))
            .and_then(Value::as_str)
            == Some("exception")
        {
            let result = value.get("result").unwrap_or(&Value::Null);
            let details = result
                .get("exceptionDetails")
                .and_then(|details| details.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("KICK BiDi script command raised an exception");

            return Err(format!("KICK script command exception: {details}"));
        }

        return Ok(value);
    }
}

impl Default for KickManager {
    fn default() -> Self {
        Self::new()
    }
}
