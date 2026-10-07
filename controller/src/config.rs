use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub firefox_executable: PathBuf,
    pub profile_name: String,
    pub profile_dir: PathBuf,
    pub url: String,
}

impl Config {
    pub fn for_profile(profile_name: &str) -> Result<Self, String> {
        validate_profile_name(profile_name)?;

        let firefox_executable = find_firefox()
            .ok_or_else(|| {
                "Firefox was not found. Install Firefox or set MOTH_FIREFOX to firefox.exe."
                    .to_string()
            })?;

        let profile_dir = data_root()?.join("Profiles").join(profile_name);

        Ok(Self {
            firefox_executable,
            profile_name: profile_name.to_string(),
            profile_dir,
            url: "https://pokeidle.io/app".to_string(),
        })
    }

    pub fn profiles_dir() -> Result<PathBuf, String> {
        Ok(data_root()?.join("Profiles"))
    }
}

fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "profile name must contain only ASCII letters, digits, '-' or '_'.".to_string(),
        );
    }

    Ok(())
}

fn data_root() -> Result<PathBuf, String> {
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(local_app_data).join("Moth").join("PokeIdle"));
    }

    Err("could not determine the Windows local application data directory".to_string())
}

fn find_firefox() -> Option<PathBuf> {
    if let Some(path) = env::var_os("MOTH_FIREFOX") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    firefox_candidates().into_iter().find(|path| path.is_file())
}

#[cfg(windows)]
fn firefox_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(program_files) = env::var_os("PROGRAMFILES") {
        paths.push(
            PathBuf::from(program_files)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    if let Some(program_files_x86) = env::var_os("PROGRAMFILES(X86)") {
        paths.push(
            PathBuf::from(program_files_x86)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        paths.push(
            PathBuf::from(local_app_data)
                .join("Mozilla Firefox")
                .join("firefox.exe"),
        );
    }

    paths
}
