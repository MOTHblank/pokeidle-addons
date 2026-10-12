use crate::config::{Config, GameProfile};
use crate::firefox;
use serde_json::Value;

pub const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";
pub const VIOLENTMONKEY_INSTALL: &str =
    "https://addons.mozilla.org/firefox/addon/violentmonkey/";

// Only the script filenames are maintained here, never embedded version numbers.
// Installation URLs always address the live master script and use a fresh cache
// key so browser/CDN caches cannot pin the previous installer version.
const ADDON_FILES: &[(&str, &str)] = &[
    ("Auto Catch+", "autocatch.user.js"),
    ("Performance+", "performance.user.js"),
    ("Live Stream Scanner", "stream-auto-open.user.js"),
    ("Controller Bridge", "controller-bridge.user.js"),
    ("Twitch Low Resource", "twitch-low-resource.user.js"),
    ("Hunt Atlas", "hunt-atlas.user.js"),
    ("Moth Watch", "market-bot.user.js"),
];

fn installer_url(file: &str, nonce: u128) -> String {
    format!(
        "https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/{file}?moth_install={nonce}"
    )
}

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

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("could not generate installer cache key: {error}"))?
        .as_nanos();
    for &(name, file) in ADDON_FILES {
        let url = installer_url(file, nonce);
        match firefox::open_url(&config, &url) {
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

    std::process::Command::new(if cfg!(windows) { "explorer.exe" } else { "xdg-open" })
        .arg(&config.profile_dir)
        .spawn()
        .map_err(|e| format!("could not open profile folder: {e}"))?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{installer_url, ADDON_FILES};
    #[test]
    fn installer_urls_are_cache_busted_without_pinned_versions() {
        for (_, file) in ADDON_FILES {
            let url = installer_url(file, 123);
            assert!(url.ends_with("?moth_install=123"));
            assert!(url.contains("/master/addons/"));
            assert!(!url.contains("?v="));
        }
    }
}
