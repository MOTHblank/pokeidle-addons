//! Windows per-user startup registration, only changed by explicit user action.
#[cfg(windows)]
pub fn set_start_with_windows(enabled: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not locate controller executable: {error}"))?;
    let mut command = std::process::Command::new("reg.exe");
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    if enabled {
        // Command::arg never invokes a shell, so spaces in install paths are safe.
        let quoted_executable = format!("\"{}\"", executable.display());
        command.args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v", "MothController", "/t", "REG_SZ", "/d", &quoted_executable, "/f"]);
    } else {
        command.args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v", "MothController", "/f"]);
    }
    let output = command.output()
        .map_err(|error| format!("could not configure Windows startup: {error}"))?;
    if output.status.success() { Ok(()) } else {
        Err(format!("Windows startup registration failed: {}", output.status))
    }
}

#[cfg(not(windows))]
pub fn set_start_with_windows(_enabled: bool) -> Result<(), String> {
    Err("Windows startup registration is not available on this platform".into())
}
