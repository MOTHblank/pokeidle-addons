use crate::config::{Config, GameProfile};
use crate::firefox;

pub const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
pub const KICK_LOGIN: &str = "https://kick.com/";

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
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/stream-auto-open.user.js?v=6.2.0",
    ),
    (
        "Controller Bridge",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/controller-bridge.user.js?v=1.2.0",
    ),
    (
        "Twitch Low Resource",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/twitch-low-resource.user.js?v=2.1.0",
    ),
    (
        "Hunt Atlas",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-huntatlas/main/hunt-atlas.user.js?v=1.7.8",
    ),
    (
        "Moth Watch",
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/market-bot.user.js?v=0.1.15",
    ),

];

pub fn open_game(profile: GameProfile) -> Result<(), String> {
    let config = Config::for_profile(profile)?;
    let _ = firefox::launch_unmonitored(&config)?;
    Ok(())
}

pub fn open_addons(profile: GameProfile) -> Result<usize, String> {
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