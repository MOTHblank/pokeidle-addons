use crate::config::Config;
use crate::logging;
use crate::monitor::MonitorHandle;
use std::fs;
use std::io;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};


const USER_PREFS: &str = r#"
// Moth defaults: keep the browser quiet and cheap without changing
// PokéIdle, Twitch, or KICK functionality. This file is created once
// per game profile; user changes made later are left intact.

user_pref("browser.newtabpage.enabled", false);
user_pref("browser.newtabpage.activity-stream.feeds.snippets", false);
user_pref("browser.newtabpage.activity-stream.feeds.section.topstories", false);
user_pref("browser.newtabpage.preload", false);
user_pref("browser.startup.page", 0);
user_pref("browser.sessionstore.restore_on_demand", true);
user_pref("browser.sessionstore.restore_tabs_lazily", true);
user_pref("browser.pagethumbnails.capturing_disabled", true);
user_pref("browser.discovery.enabled", false);
user_pref("browser.sessionstore.interval", 600000);
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);
user_pref("datareporting.usage.uploadEnabled", false);
user_pref("toolkit.telemetry.archive.enabled", false);
user_pref("toolkit.telemetry.enabled", false);
user_pref("browser.uitour.enabled", false);
user_pref("browser.shell.checkDefaultBrowser", false);
user_pref("browser.urlbar.suggest.searches", false);
user_pref("network.prefetch-next", false);
user_pref("network.predictor.enabled", false);
user_pref("network.predictor.enable-prefetch", false);
user_pref("network.dns.disablePrefetch", true);
user_pref("layout.frame_rate", 15);

// Firefox keeps preallocated content processes ready for future tabs.
// Moth does not need that background process reserve.
user_pref("dom.ipc.processPrelaunch.fission.number", 0);

// Keep the shared web-content pool at its minimum. Fission remains enabled
// for browser security; isolated sites may still receive their own process.
user_pref("dom.ipc.processCount", 1);

// Moth never intentionally plays stream video. Blocking autoplay and
// suspending background media prevents accidental decode/work from a tab.
user_pref("media.autoplay.default", 5);
user_pref("media.autoplay.blocking_policy", 1);
user_pref("media.autoplay.ask-permission", false);
user_pref("media.block-autoplay-until-in-foreground", true);
user_pref("media.suspend-background-video.enabled", true);
"#;

pub fn launch(config: &Config, headless: bool) -> Result<(Child, MonitorHandle), String> {
    migrate_legacy_profile(config)?;

    fs::create_dir_all(&config.profile_dir).map_err(|error| {
        format!(
            "could not create profile directory {}: {error}",
            config.profile_dir.display()
        )
    })?;

    provision_profile(&config.profile_dir)?;
    ensure_debug_port_available(config.remote_debug_port)?;
    logging::info(&format!("starting Firefox: executable={} profile={} port={} mode={} url={}", config.firefox_executable.display(), config.profile_dir.display(), config.remote_debug_port, if headless { "headless" } else { "visible" }, config.url));

    let mut command = Command::new(&config.firefox_executable);
    if headless {
        command.arg("--headless");
    }

    command
        .arg("--no-remote")
        .arg(format!("--remote-debugging-port={}", config.remote_debug_port))
        .arg("--profile")
        .arg(&config.profile_dir)
        .arg("--new-window")
        .arg(&config.url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .map_err(format_spawn_error)?;

    let monitor = MonitorHandle::start(config.remote_debug_port, headless);
    Ok((child, monitor))
}

/// Reopen the selected PokéIdle Firefox profile as an ordinary browser
/// for Kick authentication. Keep Google cookies and the rest of the profile,
/// but do not launch a Remote Agent, BiDi, or any browser automation.
/// The normal PokéIdle handoff must have released the Firefox profile lock.
pub fn launch_kick_login(config: &Config) -> Result<Child, String> {
    // Reuse pre-existing Google auth, including accounts with legacy Moth
    // profile paths. Never create a blank profile for Kick as a fallback.
    migrate_legacy_profile(config)?;
    if !config.profile_dir.is_dir() {
        return Err("This PokéIdle Firefox profile does not exist yet. Sign in to PokéIdle with this account first.".into());
    }
    if find_profile_process(config)?.is_some() {
        return Err("The PokéIdle Firefox profile is still running. Wait for its native handoff to finish before logging into Kick.".into());
    }
    Command::new(&config.firefox_executable)
        .arg("--no-remote")
        .arg("--profile")
        .arg(&config.profile_dir)
        .arg("--new-window")
        .arg("https://kick.com/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(format_spawn_error)
}

pub fn launch_unmonitored(config: &Config) -> Result<Child, String> {
    migrate_legacy_profile(config)?;

    fs::create_dir_all(&config.profile_dir).map_err(|error| {
        format!(
            "could not create profile directory {}: {error}",
            config.profile_dir.display()
        )
    })?;

    provision_profile(&config.profile_dir)?;
    logging::info(&format!(
        "starting unmonitored visible Firefox without remote debugging: executable={} profile={} url={}",
        config.firefox_executable.display(),
        config.profile_dir.display(),
        config.url
    ));

    Command::new(&config.firefox_executable)
        .arg("--no-remote")
        .arg("--profile")
        .arg(&config.profile_dir)
        .arg("--new-window")
        .arg(&config.url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(format_spawn_error)
}

fn migrate_legacy_profile(config: &Config) -> Result<(), String> {
    if config.profile_dir.exists() {
        return Ok(());
    }

    let legacy_dir = config.legacy_profile_dir()?;
    if !legacy_dir.exists() {
        return Ok(());
    }

    fs::rename(&legacy_dir, &config.profile_dir).map_err(|error| {
        format!(
            "could not migrate legacy profile {} to {}: {error}",
            legacy_dir.display(),
            config.profile_dir.display()
        )
    })
}

fn provision_profile(profile_dir: &std::path::Path) -> Result<(), String> {
    let user_js = profile_dir.join("user.js");

    if !user_js.exists() {
        fs::write(&user_js, USER_PREFS).map_err(|error| {
            format!(
                "could not create browser defaults {}: {error}",
                user_js.display()
            )
        })?;

        return Ok(());
    }

    // Existing Moth profiles may have been created by an older controller.
    // Add only preferences that are not already present so user changes to
    // existing prefs are left untouched.
    let mut current = fs::read_to_string(&user_js).map_err(|error| {
        format!(
            "could not read browser defaults {}: {error}",
            user_js.display()
        )
    })?;

    const PREFS_TO_ENSURE: &[(&str, &str)] = &[
        ("browser.newtabpage.preload", r#"user_pref("browser.newtabpage.preload", false);"#),
        ("browser.sessionstore.restore_on_demand", r#"user_pref("browser.sessionstore.restore_on_demand", true);"#),
        ("browser.pagethumbnails.capturing_disabled", r#"user_pref("browser.pagethumbnails.capturing_disabled", true);"#),
        ("browser.discovery.enabled", r#"user_pref("browser.discovery.enabled", false);"#),
        ("browser.sessionstore.interval", r#"user_pref("browser.sessionstore.interval", 600000);"#),
        ("datareporting.healthreport.uploadEnabled", r#"user_pref("datareporting.healthreport.uploadEnabled", false);"#),
        ("datareporting.policy.dataSubmissionEnabled", r#"user_pref("datareporting.policy.dataSubmissionEnabled", false);"#),
        ("datareporting.usage.uploadEnabled", r#"user_pref("datareporting.usage.uploadEnabled", false);"#),
        ("toolkit.telemetry.archive.enabled", r#"user_pref("toolkit.telemetry.archive.enabled", false);"#),
        ("toolkit.telemetry.enabled", r#"user_pref("toolkit.telemetry.enabled", false);"#),
        ("browser.uitour.enabled", r#"user_pref("browser.uitour.enabled", false);"#),
        ("browser.shell.checkDefaultBrowser", r#"user_pref("browser.shell.checkDefaultBrowser", false);"#),
        ("browser.urlbar.suggest.searches", r#"user_pref("browser.urlbar.suggest.searches", false);"#),
        ("network.dns.disablePrefetch", r#"user_pref("network.dns.disablePrefetch", true);"#),
        ("layout.frame_rate", r#"user_pref("layout.frame_rate", 15);"#),
        ("dom.ipc.processPrelaunch.fission.number", r#"user_pref("dom.ipc.processPrelaunch.fission.number", 0);"#),
        ("dom.ipc.processCount", r#"user_pref("dom.ipc.processCount", 1);"#),
        ("media.autoplay.default", r#"user_pref("media.autoplay.default", 5);"#),
        ("media.autoplay.blocking_policy", r#"user_pref("media.autoplay.blocking_policy", 1);"#),
        ("media.autoplay.ask-permission", r#"user_pref("media.autoplay.ask-permission", false);"#),
        ("media.block-autoplay-until-in-foreground", r#"user_pref("media.block-autoplay-until-in-foreground", true);"#),
        ("media.suspend-background-video.enabled", r#"user_pref("media.suspend-background-video.enabled", true);"#),
    ];

    let mut added = false;

    for (pref, line) in PREFS_TO_ENSURE {
        if !current.contains(&format!(r#"user_pref("{}""#, pref)) {
            if !current.ends_with('\n') {
                current.push('\n');
            }

            current.push_str(line);
            current.push('\n');
            added = true;
        }
    }

    if added {
        fs::write(&user_js, current).map_err(|error| {
            format!(
                "could not update browser defaults {}: {error}",
                user_js.display()
            )
        })?;
    }

    Ok(())
}

fn format_spawn_error(error: io::Error) -> String {
    format!("could not start Firefox: {error}")
}

/// Open setup/login/Violentmonkey links using Firefox's normal command-line
/// URL handler, rather than a WebDriver BiDi-created browsing context.
/// Installing a userscript can interrupt a navigation while Violentmonkey
/// takes over; a BiDi navigation failure must not discard the installer tab.
/// This restores the setup path that worked before v0.1.3.
pub fn open_url(config: &Config, url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("refusing to open a non-HTTPS URL".to_string());
    }

    fs::create_dir_all(&config.profile_dir).map_err(|error| {
        format!(
            "could not create profile directory {}: {error}",
            config.profile_dir.display()
        )
    })?;

    Command::new(&config.firefox_executable)
        .arg("--profile")
        .arg(&config.profile_dir)
        .arg("--new-tab")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(format_spawn_error)
}

/// Stop only the Firefox process started by this controller. Prefer a clean
/// WebDriver BiDi browser.close (headless and visible); then try window close
/// for visible or sign-in browsers. Force termination is the final fallback.
pub fn stop_child(child: &mut Child, monitor: Option<&MonitorHandle>) -> Result<(), String> {
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) => return Err(format!("could not check Firefox process: {error}")),
    }

    if let Some(monitor) = monitor {
        match monitor.close_browser(Duration::from_secs(3)) {
            Ok(()) => logging::info("Firefox received BiDi browser.close"),
            Err(error) => {
                logging::warn(&format!("Firefox BiDi close unavailable: {error}"));
                // A timed-out request must not close a newly launched browser
                // later if this monitor reconnects to the same port.
                monitor.stop();
            }
        }
        if wait_child(child, Duration::from_secs(3))? {
            return Ok(());
        }
    }

    let windows = request_window_close(child.id());
    if windows > 0 && wait_child(child, Duration::from_secs(4))? {
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        send_linux_signal(child.id(), "-TERM")?;
        if wait_child(child, Duration::from_secs(4))? {
            return Ok(());
        }
    }

    // This PID belongs to our Child handle, so force kill cannot hit a
    // different Firefox profile through a broad process-name lookup.
    logging::warn(&format!(
        "Firefox PID {} did not exit gracefully; forcing termination",
        child.id()
    ));
    match child.kill() {
        Ok(()) => {
            child.wait().map_err(|error| format!("could not reap Firefox: {error}"))?;
            Ok(())
        }
        Err(error) => match child.try_wait() {
            Ok(Some(_)) => Ok(()), // exited between the last poll and kill
            _ => Err(format!("could not terminate Firefox: {error}")),
        },
    }
}

fn wait_child(child: &mut Child, timeout: Duration) -> Result<bool, String> {
    let until = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(true),
            Ok(None) => {}
            Err(error) => return Err(format!("could not check Firefox process: {error}")),
        }
        if Instant::now() >= until {
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Find a pre-existing Firefox parent launched with this *exact* profile.
/// Used only when a process is lost or a new launch is requested: querying
/// WMI from the egui repaint loop would be too expensive.
#[cfg(windows)]
pub fn find_profile_process(config: &Config) -> Result<Option<u32>, String> {
    let quoted = format!("'{}'", config.profile_dir.to_string_lossy().replace('\'', "''"));
    let script = format!(
        r#"$ErrorActionPreference='Stop'; $p={quoted}; $pattern='(?i)(?:^|\s)-{{1,2}}profile\s+"?' + [regex]::Escape($p) + '"?(?=\s|$)'; @(Get-CimInstance Win32_Process -Filter "Name = 'firefox.exe'" | Where-Object {{ $_.CommandLine -match $pattern }} | Select-Object -ExpandProperty ProcessId) -join ','"#
    );
    let mut command = Command::new("powershell.exe");
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = command
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|error| format!("could not inspect running Firefox profiles: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Firefox profile inspection failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = String::from_utf8_lossy(&output.stdout);
    // WMI enumeration order is not stable. The same PID must be chosen on
    // successive checks, or a legitimate multiprocess Firefox profile can
    // appear to change ownership halfway through a verified Stop.
    let mut ids = value.split(',')
        .filter(|id| !id.trim().is_empty())
        .map(|id| id.trim().parse::<u32>()
            .map_err(|error| format!("invalid Firefox PID from process inspection: {error}")))
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > 1 {
        logging::warn("Multiple Firefox parent processes advertise the same Moth profile; reconciling by stable PID");
    }
    Ok(ids.into_iter().next())
}

#[cfg(target_os = "linux")]
pub fn find_profile_process(config: &Config) -> Result<Option<u32>, String> {
    use std::os::unix::ffi::OsStrExt;
    let desired = config.profile_dir.as_os_str().as_bytes();
    let mut found = Vec::new();
    let entries = fs::read_dir("/proc")
        .map_err(|error| format!("cannot inspect /proc for Firefox profiles: {error}"))?;
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(raw) = fs::read(entry.path().join("cmdline")) else { continue; };
        let args: Vec<&[u8]> = raw.split(|byte| *byte == 0).filter(|arg| !arg.is_empty()).collect();
        if args.is_empty() {
            continue;
        }
        if linux_command_has_profile(&args, desired) {
            found.push(pid);
        }
    }
    found.sort_unstable();
    if found.len() > 1 {
        logging::warn("Multiple Firefox processes advertise the same Linux profile");
    }
    Ok(found.into_iter().next())
}

#[cfg(target_os = "linux")]
fn linux_command_has_profile(args: &[&[u8]], desired: &[u8]) -> bool {
    let Some(executable) = args.first().copied() else { return false; };
    let executable = executable.rsplit(|byte| *byte == b'/').next().unwrap_or_default();
    if !executable.starts_with(b"firefox") {
        return false;
    }
    args.windows(2).any(|pair| {
        (pair[0] == b"--profile" || pair[0] == b"-profile") && pair[1] == desired
    }) || args.iter().any(|arg| arg.starts_with(b"--profile=") && &arg[10..] == desired)
}

#[cfg(all(test, target_os = "linux"))]
mod linux_process_tests {
    use super::linux_command_has_profile;
    #[test]
    fn only_accept_exact_firefox_profile() {
        let args: &[&[u8]] = &[b"/usr/lib/firefox/firefox", b"--profile", b"/tmp/Game1", b"--new-window"];
        assert!(linux_command_has_profile(args, b"/tmp/Game1"));
        assert!(!linux_command_has_profile(args, b"/tmp/Game2"));
        assert!(!linux_command_has_profile(args, b"/tmp/Game"));
        assert!(!linux_command_has_profile(&[b"/usr/bin/chrome", b"--profile", b"/tmp/Game1"], b"/tmp/Game1"));
        assert!(linux_command_has_profile(&[b"firefox-bin", b"--profile=/tmp/Game1"], b"/tmp/Game1"));
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn find_profile_process(_config: &Config) -> Result<Option<u32>, String> {
    Err("Firefox profile detection is only available on Windows or Linux".to_string())
}

/// Close a recovered Firefox only after verifying that its command line still
/// identifies our own profile. This is called on an explicit Stop action,
/// never on controller Drop: we must not kill somebody else's browser.
pub fn stop_existing(config: &Config, pid: u32) -> Result<(), String> {
    if find_profile_process(config)? != Some(pid) {
        return Err("Firefox profile process changed; refresh and try again".to_string());
    }
    let windows = request_window_close(pid);
    if windows > 0 && wait_pid(pid, Duration::from_secs(5)) {
        return Ok(());
    }

    if !process_alive(pid) {
        return Ok(());
    }
    if find_profile_process(config)? != Some(pid) {
        return Err("Firefox profile process changed during shutdown".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        send_linux_signal(pid, "-TERM")?;
        if wait_pid(pid, Duration::from_secs(5)) {
            return Ok(());
        }
        if find_profile_process(config)? != Some(pid) {
            return Err("Firefox profile changed during shutdown".to_string());
        }
        logging::warn(&format!("Recovered Firefox PID {pid} ignored SIGTERM; forcing SIGKILL"));
        send_linux_signal(pid, "-KILL")?;
    }

    #[cfg(windows)]
    {
        logging::warn(&format!("Recovered Firefox PID {pid} ignored window close; terminating"));
        let output = Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|error| format!("could not terminate recovered Firefox: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "could not terminate recovered Firefox: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    return Err("recovered browser shutdown is unsupported on this platform".to_string());
    if !wait_pid(pid, Duration::from_secs(3)) {
        return Err("recovered Firefox did not exit after termination request".to_string());
    }
    Ok(())
}

/// Shut down every Firefox process whose *command line* explicitly names
/// one of Moth's dedicated profile directories. A previous controller crash
/// can strand multiple Firefox parents under the same profile; stopping only
/// the first PID leaves the profile locked on the next Launch.
///
/// Never use taskkill /IM firefox.exe or wildcard process termination: other
/// unrelated Firefox profiles must remain untouched.
pub fn stop_profile_processes(config: &Config) -> Result<(), String> {
    const MAX_PROCESSES: usize = 24;
    for _ in 0..MAX_PROCESSES {
        let Some(pid) = find_profile_process(config)? else { return Ok(()); };
        logging::info(&format!(
            "{}: closing existing Firefox profile process PID {pid}",
            config.profile.label()
        ));
        stop_existing(config, pid)?;
    }
    if find_profile_process(config)?.is_some() {
        Err("Moth Firefox profile still has processes after repeated verified shutdown".into())
    } else {
        Ok(())
    }
}

fn wait_pid(pid: u32, timeout: Duration) -> bool {
    let until = Instant::now() + timeout;
    while process_alive(pid) {
        if Instant::now() >= until {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
    true
}

#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SYNCHRONIZE is enough to inspect whether a process handle is signaled.
    let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let result = unsafe { WaitForSingleObject(handle, 0) };
    unsafe { CloseHandle(handle); }
    result == 0x102 // WAIT_TIMEOUT: the process is still running
}

#[cfg(target_os = "linux")]
pub fn process_alive(pid: u32) -> bool {
    // /proc/<pid>/stat reports Z for an exited process waiting to be reaped.
    let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else { return false; };
    stat.rsplit_once(") ").and_then(|(_, rest)| rest.chars().next()) != Some('Z')
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn process_alive(_pid: u32) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn send_linux_signal(pid: u32, signal: &str) -> Result<(), String> {
    let output = Command::new("kill")
        .args([signal, "--", &pid.to_string()])
        .output()
        .map_err(|error| format!("could not signal Firefox PID {pid}: {error}"))?;
    if output.status.success() || !process_alive(pid) {
        return Ok(());
    }
    Err(format!("could not signal Firefox PID {pid}: {}",
        String::from_utf8_lossy(&output.stderr).trim()))
}

#[cfg(windows)]
#[repr(C)]
struct CloseWindows {
    pid: u32,
    sent: usize,
}

#[cfg(windows)]
unsafe extern "system" fn close_window_for_pid(hwnd: *mut std::ffi::c_void, parameter: isize) -> i32 {
    let request = unsafe { &mut *(parameter as *mut CloseWindows) };
    let mut owner = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
    if owner == request.pid && unsafe { PostMessageW(hwnd, 0x0010, 0, 0) } != 0 {
        request.sent += 1; // WM_CLOSE; Firefox controls its own cleanup
    }
    1 // continue enumerating all top-level windows for this PID
}

#[cfg(windows)]
fn request_window_close(pid: u32) -> usize {
    let mut request = CloseWindows { pid, sent: 0 };
    unsafe { EnumWindows(Some(close_window_for_pid), &mut request as *mut CloseWindows as isize); }
    request.sent
}

#[cfg(not(windows))]
fn request_window_close(_pid: u32) -> usize { 0 }

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(callback: Option<unsafe extern "system" fn(*mut std::ffi::c_void, isize) -> i32>, parameter: isize) -> i32;
    fn GetWindowThreadProcessId(hwnd: *mut std::ffi::c_void, pid: *mut u32) -> u32;
    fn PostMessageW(hwnd: *mut std::ffi::c_void, message: u32, wparam: usize, lparam: isize) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, milliseconds: u32) -> u32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}


/// A stale debugging listener might belong to another Firefox profile. If
/// that happened, browser.close could shut down an unrelated browser.
fn ensure_debug_port_available(port: u16) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
        format!("Firefox BiDi port {port} is already in use or unavailable: {error}")
    })?;
    drop(listener);
    Ok(())
}

#[cfg(test)]
mod lifecycle_tests {
    use super::ensure_debug_port_available;
    use std::net::TcpListener;

    #[test]
    fn occupied_debugging_port_is_rejected() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("local TCP bind");
        let port = listener.local_addr().expect("listener address").port();
        assert!(ensure_debug_port_available(port).is_err());
    }
}
