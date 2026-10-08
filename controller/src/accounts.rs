use crate::config::{Config, GameProfile};
use crate::firefox;
use serde_json::Value;

pub const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
pub const KICK_LOGIN: &str = "https://kick.com/";
pub const VIOLENTMONKEY_INSTALL: &str =
    "https://addons.mozilla.org/firefox/addon/violentmonkey/";

pub const ADDONS: &[(&str, &str)] = &[
    (
        "Auto Catch+",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/autocatch.user.js",
    ),
    (
        "Performance+",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/performance.user.js",
    ),
    (
        "Live Stream Scanner",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/stream-auto-open.user.js?v=6.3.1",
    ),
    (
        "Controller Bridge",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/controller-bridge.user.js?v=1.2.1",
    ),
    (
        "Twitch Low Resource",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/twitch-low-resource.user.js?v=2.1.0",
    ),
    (
        "Hunt Atlas",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/hunt-atlas.user.js?v=1.7.12",
    ),
    (
        "Moth Watch",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/market-bot.user.js?v=0.1.17",
    ),

];

pub fn violentmonkey_installed(profile: GameProfile) -> Result<bool, String> {
    let config = Config::for_profile(profile)?;
    let extensions = config.profile_dir.join("extensions.json");

    if !extensions.is_file() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(&extensions)
        .map_err(|error| format!("could not read Firefox extensions registry: {error}"))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid Firefox extensions registry: {error}"))?;

    let Some(addons) = value.get("addons").and_then(Value::as_array) else {
        return Ok(false);
    };

    Ok(addons.iter().any(|addon| {
        let name = addon
            .get("defaultLocale")
            .and_then(|locale| locale.get("name"))
            .and_then(Value::as_str)
            .or_else(|| addon.get("name").and_then(Value::as_str))
            .unwrap_or_default();

        let id = addon
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let active = addon
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        active
            && (name.to_ascii_lowercase().contains("violentmonkey")
                || id.to_ascii_lowercase().contains("violentmonkey"))
    }))
}

pub fn open_violentmonkey(profile: GameProfile) -> Result<String, String> {
    let config = Config::for_profile(profile)?;
    firefox::open_url(&config, VIOLENTMONKEY_INSTALL)?;
    Ok(format!("Violentmonkey installer opened in {}.", profile.label()))
}

pub fn open_game(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    let _ = firefox::launch_unmonitored(&config)?;
    Ok(())
}

pub fn open_addons(profile: GameProfile) -> Result<usize, String> {
    if !violentmonkey_installed(profile)? {
        return Err(format!(
            "Violentmonkey is not installed and active in {}. Use \"Install Violentmonkey\" first.",
            profile.label()
        ));
    }

    let config = Config::for_profile(profile)?;
    let mut opened = 0usize;
    let mut failures = Vec::new();

    for &(name, url) in ADDONS {
        match firefox::open_url(&config, url) {
            Ok(()) => opened += 1,
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }

    if failures.is_empty() {
        Ok(opened)
    } else {
        Err(format!(
            "Opened {opened} addon installers, but some failed:\n{}",
            failures.join("\n")
        ))
    }
}

pub fn open_login(profile: GameProfile, url: &str, service: &str) -> Result<String, String> {
    let config = Config::for_profile(profile)?;
    firefox::open_url(&config, url)?;
    Ok(format!("{service} opened in {}.", profile.label()))
}

pub fn open_profile_folder(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    std::fs::create_dir_all(&config.profile_dir)
        .map_err(|e| format!("could not create profile directory: {e}"))?;

    std::process::Command::new("explorer.exe")
        .arg(&config.profile_dir)
        .spawn()
        .map_err(|e| format!("could not open profile folder: {e}"))?;

    Ok(())
}