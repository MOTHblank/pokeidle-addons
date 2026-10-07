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
    pub fn from_args<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut profile_name = String::from("AccountA");
        let mut firefox_executable = find_firefox()
            .ok_or_else(|| "Firefox was not found. Set MOTH_FIREFOX to firefox.exe.".to_string())?;
        let mut url = String::from("https://pokeidle.io/app");

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--profile" => {
                    profile_name = args
                        .next()
                        .ok_or_else(|| "--profile requires a value".to_string())?;
                }
                "--firefox" => {
                    firefox_executable = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "--firefox requires a path".to_string())?,
                    );
                }
                "--url" => {
                    url = args
                        .next()
                        .ok_or_else(|| "--url requires a value".to_string())?;
                }
                "--help" | "-h" => {
                    Self::print_usage();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }

        validate_profile_name(&profile_name)?;

        let profile_root = data_root()?;
        let profile_dir = profile_root.join("Profiles").join(&profile_name);

        Ok(Self {
            firefox_executable,
            profile_name,
            profile_dir,
            url,
        })
    }

    pub fn print_usage() {
        println!(
            "moth-controller\n\nUsage: moth-controller [OPTIONS]\n\nOptions:\n  --profile NAME       Browser profile name (default: AccountA)\n  --firefox PATH       Firefox executable path\n  --url URL            Initial page (default: https://pokeidle.io/app)\n  -h, --help           Show this help"
        );
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

    if let Some(home) = env::var_os("HOME") {
        return Ok(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("Moth")
                .join("PokeIdle"),
        );
    }

    Err("could not determine a local application data directory".to_string())
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

#[cfg(not(windows))]
fn firefox_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/bin/firefox"),
        PathBuf::from("/usr/local/bin/firefox"),
    ]
}
