use crate::config::{Config, GameProfile};
use crate::logging;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};

#[derive(Clone, Debug)]
pub struct KickStream {
    pub name: String,
    pub url: String,
}

#[cfg(windows)]
type BrowserSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[cfg(windows)]
#[derive(Default)]
struct KickSession {
    profile_dir: Option<PathBuf>,
    browser_executable: Option<PathBuf>,
    browser_pid: Option<u32>,
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
    ) {
        #[cfg(not(windows))]
        {
            let _ = (profile, streams, state_available);
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

            if let Err(error) = self.sync_tabs(profile, streams, state_available) {
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

            self.ensure_browser(profile, url)?;

            let id = self.next_id(profile);
            let result = {
                let session = self.session_mut(profile);
                if session.socket.is_none() {
                    return Err("KICK Firefox remote control is not ready yet".to_string());
                }

                send_and_wait(
                    session.socket.as_mut().unwrap(),
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

            let context = result
                .get("result")
                .and_then(|value| value.get("context"))
                .and_then(Value::as_str)
                .ok_or_else(|| "KICK tab creation returned no context".to_string())?
                .to_string();

            let navigate_id = self.next_id(profile);
            {
                let session = self.session_mut(profile);
                send_and_wait(
                    session.socket.as_mut().unwrap(),
                    navigate_id,
                    json!({
                        "id": navigate_id,
                        "method": "browsingContext.navigate",
                        "params": {
                            "context": context,
                            "url": url,
                            "wait": "none"
                        }
                    }),
                )?;
            }

            Ok(())
        }
    }

    pub fn shutdown(&mut self) {
        #[cfg(windows)]
        {
            for profile in GameProfile::ALL {
                let _ = self.close_all_managed_tabs(profile);

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
    ) -> Result<(), String> {
        let (profile_dir, executable) = {
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
            )
        };

        std::fs::create_dir_all(&profile_dir)
            .map_err(|error| format!("could not create KICK profile: {error}"))?;

        let port = 27801 + profile.index() as u16;

        let alive = {
            let session = self.session_mut(profile);
            session
                .browser_pid
                .map(process_alive)
                .unwrap_or(false)
        };

        if !alive {
            let child = Command::new(&executable)
                .arg("--no-remote")
                .arg(format!("--remote-debugging-port={}", port))
                .arg("--profile")
                .arg(&profile_dir)
                .arg("--new-window")
                .arg(initial_url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|error| format!("could not start normal KICK Firefox: {error}"))?;

            let pid = child.id();

            let session = self.session_mut(profile);
            session.browser_pid = Some(pid);
            session.socket = None;
            session.tabs.clear();
            session.pending.clear();
        }

        if self.session_mut(profile).socket.is_none() {
            self.connect(profile, port)?;
        }

        Ok(())
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
    ) -> Result<(), String> {
        let mut desired = HashMap::new();

        for stream in streams.iter().take(10) {
            let key = normalize(&stream.name);
            if key.is_empty() || !is_kick_url(&stream.url) {
                continue;
            }
            desired.insert(key, stream.url.clone());
        }

        if desired.is_empty() && !state_available {
            return Ok(());
        }

        let initial_url = desired
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| "https://kick.com/".to_string());

        if let Err(error) = self.ensure_browser(profile, &initial_url) {
            return Err(error);
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

            self.create_stream_tab(profile, key, url)?;
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
        {
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
                }),
            )?;
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

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    #[link(name = "kernel32")]
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
