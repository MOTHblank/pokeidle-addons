//! Small synchronous HTTP adapter for Kick's undocumented website endpoints.
//! Unlike an always-running embedded browser, system curl exits after each
//! request. Credentials are passed through stdin, NEVER command-line args.
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Condvar, Mutex, OnceLock};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const MARKER: &str = "\n__MOTH_KICK_HTTP_CODE__:";
const MAX_REPLY_BYTES: usize = 1024 * 1024;
const MAX_CONCURRENT_REQUESTS: usize = 2;

/// A very small shared pool; 7 channels must not launch 7 curl processes.
struct Permit(&'static (Mutex<usize>, Condvar));
impl Drop for Permit {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.0.lock() {
            *active = active.saturating_sub(1);
            self.0.1.notify_one();
        }
    }
}
fn acquire() -> Result<Permit, String> {
    static SLOTS: OnceLock<(Mutex<usize>, Condvar)> = OnceLock::new();
    let slots = SLOTS.get_or_init(|| (Mutex::new(0), Condvar::new()));
    let mut active = slots.0.lock().map_err(|_|
        "Kick HTTP queue unavailable".to_owned())?;
    while *active >= MAX_CONCURRENT_REQUESTS {
        active = slots.1.wait(active).map_err(|_|
            "Kick HTTP queue unavailable".to_owned())?;
    }
    *active += 1;
    Ok(Permit(slots))
}

/// HTTP status and JSON response; non-200 replies have a null JSON value.
/// Kick can change these undocumented endpoints without notice.
pub fn get(url: &str, bearer: Option<&str>, extra_headers: &[(&str, String)])
    -> Result<(u16, Value), String>
{
    if !(url.starts_with("https://kick.com/api/v2/channels/") ||
         url == "https://websockets.kick.com/viewer/v1/token") {
        return Err("Kick request blocked: unexpected URL".into());
    }
    // The URL is drawn from a whitelisted slug or a fixed constant; no
    // redirects that could forward credentials to a different website.
    let _permit = acquire()?;
    let binary = if cfg!(windows) { "curl.exe" } else { "curl" };
    let mut command = Command::new(binary);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut child = match command
        .args(["--silent", "--show-error", "--compressed",
            "--max-time", "12", "--connect-timeout", "6",
            "--max-filesize", "1048576", "--proto", "=https",
            "--max-redirs", "0", "--config", "-",
            "--write-out", "\n__MOTH_KICK_HTTP_CODE__:%{http_code}"])
        .arg(url)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
            return Err("curl unavailable".into()),
        Err(_) => return Err("Could not start system curl for Kick API".into()),
    };
    // curl reads sensitive headers from stdin (-K -). They are not included
    // in OS process listings, failure messages, diagnostics or shell history.
    let mut config = String::from(
        "header = \"Accept: application/json, text/plain, */*\"\n\
         header = \"Origin: https://kick.com\"\n\
         header = \"Referer: https://kick.com/\"\n\
         header = \"X-Requested-With: XMLHttpRequest\"\n\
         header = \"Accept-Language: en-US,en;q=0.9\"\n\
         header = \"User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36\"\n");
    fn quoted(value: &str) -> Result<String, String> {
        if value.is_empty() || value.len() > 8192 ||
            value.bytes().any(|b| b.is_ascii_control()) {
            return Err("Invalid Kick HTTP header value".into());
        }
        Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
    }
    if let Some(token) = bearer {
        config.push_str(&format!(
            "header = \"Authorization: Bearer {}\"\n", quoted(token)?
        ));
        // A regular Kick browser also sends its first-party session cookie.
        // Only forward it to the two exact Kick endpoint families above;
        // never send this cookie to HLS CDNs or arbitrary redirects.
        if token.contains('|') {
            config.push_str(&format!(
                "header = \"Cookie: session_token={}\"\n",
                quoted(&token.replace('|', "%7C"))?
            ));
        }
    }
    for (name, value) in extra_headers {
        if !matches!(*name, "X-Chatroom" | "X-User-Id" | "Content-Type" | "X-Client-Token") {
            return Err("Unexpected Kick API header".into());
        }
        config.push_str(&format!(
            "header = \"{}: {}\"\n", name, quoted(value)?
        ));
    }
    if let Some(mut input) = child.stdin.take() {
        let written = input.write_all(config.as_bytes()).is_ok();
        drop(input);
        // Do not retain or log request headers after passing them to curl.
        config.clear();
        if !written { let _ = child.kill(); let _ = child.wait();
            return Err("Unable to configure Kick HTTP request".into()); }
    } else {
        let _ = child.kill(); let _ = child.wait();
        return Err("Kick HTTP subprocess stdin unavailable".into());
    }
    let output = child.wait_with_output()
        .map_err(|_| "Kick HTTP request did not finish".to_owned())?;
    if !output.status.success() {
        // curl stderr can contain request data; never echo it or the args.
        return Err(format!("Kick HTTP transport failed (curl exit {})",
            output.status.code().unwrap_or(-1)));
    }
    if output.stdout.len() > MAX_REPLY_BYTES + 80 {
        return Err("Kick HTTP reply exceeded safety limit".into());
    }
    let body = String::from_utf8(output.stdout)
        .map_err(|_| "Kick API returned non-UTF8 response".to_owned())?;
    let (json_body, status) = body.rsplit_once(MARKER)
        .ok_or_else(|| "Kick HTTP transport did not report status".to_owned())?;
    let status: u16 = status.trim().parse()
        .map_err(|_| "Kick HTTP transport status invalid".to_owned())?;
    if status != 200 { return Ok((status, Value::Null)); }
    let value: Value = serde_json::from_str(json_body)
        .map_err(|_| "Kick API returned non-JSON content (possible site challenge)".to_owned())?;
    Ok((status, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unexpected_endpoints() {
        assert!(get("https://example.com", None, &[]).is_err());
        assert!(get("https://kick.com@evil.example", None, &[]).is_err());
    }
}
