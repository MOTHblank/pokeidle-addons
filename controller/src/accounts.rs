use crate::config::{Config, GameProfile};

pub const TWITCH_LOGIN: &str = "https://www.twitch.tv/login";

const SETUP_MARKER: &str = ".moth-setup-complete";

/// This marker records a completed PokéIdle and Twitch sign-in.
pub fn setup_complete(profile: GameProfile) -> Result<bool, String> {
    Ok(Config::for_profile(profile)?.profile_dir.join(SETUP_MARKER).is_file())
}

pub fn complete_setup(profile: GameProfile) -> Result<(), String> {
    let path = Config::for_profile(profile)?.profile_dir;
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("Could not save setup progress: {e}"))?;
    std::fs::write(path.join(SETUP_MARKER), b"1\n")
        .map_err(|e| format!("Could not save setup progress: {e}"))
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
