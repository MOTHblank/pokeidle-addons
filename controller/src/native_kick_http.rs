//! Small synchronous HTTP adapter for Kick's undocumented website endpoints.
//! Unlike an always-running embedded browser, system curl exits after each
//! request. Credentials are passed through stdin, NEVER command-line args.
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

const MARKER: &str = "\n__MOTH_KICK_HTTP_CODE__:";
const MAX_REPLY_BYTES: usize = 1024 * 1024;

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
    let binary = if cfg!(windows) { "curl.exe" } else { "curl" };
    let mut child = match Command::new(binary)
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
         header = \"User-Agent: Chatterino7\"\n");
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
    }
    for (name, value) in extra_headers {
        if !matches!(*name, "X-Chatroom" | "X-User-Id" | "Content-Type") {
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
