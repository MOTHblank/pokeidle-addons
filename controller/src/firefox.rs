use crate::config::Config;
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

pub fn launch(config: &Config) -> Result<Child, String> {
    migrate_legacy_profile(config)?;

    fs::create_dir_all(&config.profile_dir).map_err(|error| {
        format!(
            "could not create profile directory {}: {error}",
            config.profile_dir.display()
        )
    })?;

    provision_profile(&config.profile_dir)?;

    Command::new(&config.firefox_executable)
        // Each Moth game profile must be a genuinely separate Firefox
        // instance so Game 1 and Game 2 cannot inherit each other's cookies,
        // logins, or session state. --no-remote implies --new-instance.
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

    if user_js.exists() {
        return Ok(());
    }

    fs::write(&user_js, USER_PREFS).map_err(|error| {
        format!(
            "could not create browser defaults {}: {error}",
            user_js.display()
        )
    })
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
