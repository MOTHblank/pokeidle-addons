use crate::config::Config;
use crate::logging;
use crate::monitor::MonitorHandle;
use std::fs;
use std::io;
use std::process::{Child, Command, Stdio};


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

    let monitor = MonitorHandle::start(config.remote_debug_port);
    Ok((child, monitor))
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
        "starting unmonitored headless Firefox: executable={} profile={} port={} url={}",
        config.firefox_executable.display(),
        config.profile_dir.display(),
        config.remote_debug_port,
        config.url
    ));

    Command::new(&config.firefox_executable)
        .arg("--headless")
        .arg("--no-remote")
        .arg(format!("--remote-debugging-port={}", config.remote_debug_port))
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
