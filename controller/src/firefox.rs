use crate::config::Config;
use std::fs;
use std::io;
use std::process::{Child, Command, Stdio};

pub fn launch(config: &Config) -> Result<Child, String> {
    fs::create_dir_all(&config.profile_dir).map_err(|error| {
        format!(
            "could not create profile directory {}: {error}",
            config.profile_dir.display()
        )
    })?;

    Command::new(&config.firefox_executable)
        .arg("-no-remote")
        .arg("--profile")
        .arg(&config.profile_dir)
        .arg(&config.url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(format_spawn_error)
}

fn format_spawn_error(error: io::Error) -> String {
    format!("could not start Firefox: {error}")
}
