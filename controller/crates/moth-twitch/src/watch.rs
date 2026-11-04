use base64::Engine as _;
use reqwest::blocking::Client;
use reqwest::Url;
use serde_json::json;
use std::time::Duration;

pub(crate) struct WatchClient {
    client: Client,
    spade_url: Option<Url>,
}

impl WatchClient {
    pub(crate) fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0")
            .build()
            .map_err(|_| "Could not create Twitch watch client".to_owned())?;
        Ok(Self { client, spade_url: None })
    }

    pub(crate) fn send_minute_watched(
        &mut self,
        login: &str,
        channel_id: &str,
        broadcast_id: &str,
        user_id: &str,
    ) -> Result<(), String> {
        let spade_url = match self.spade_url.clone() {
            Some(url) => url,
            None => {
                // Twitch's standard Spade endpoint is stable. The current
                // channel page can omit its settings script; that must not
                // prevent watch-credit traffic entirely.
                let url = self.resolve_spade_url(login).or_else(|_| {
                    Url::parse("https://spade.twitch.tv/track")
                        .map_err(|_| "Could not resolve Twitch watch endpoint".to_owned())
                })?;
                self.spade_url = Some(url.clone());
                url
            }
        };
        let body = serde_json::to_vec(&[json!({
            "event": "minute-watched",
            "properties": {
                "channel_id": channel_id,
                "broadcast_id": broadcast_id,
                "user_id": user_id,
                "player": "site",
                "live": true,
                "channel": login
            }
        })]).map_err(|_| "Could not encode Twitch watch event".to_owned())?;
        let data = base64::engine::general_purpose::STANDARD.encode(body);
        let response = self.client.post(spade_url)
            .form(&[("data", data)])
            .send()
            .map_err(|_| "Could not send Twitch watch event".to_owned())?;
        if response.status().as_u16() == 204 {
            Ok(())
        } else {
            self.spade_url = None;
            Err(format!(
                "Twitch watch event failed with HTTP {}", response.status().as_u16()
            ))
        }
    }

    fn resolve_spade_url(&self, login: &str) -> Result<Url, String> {
        let page = self.client.get(format!("https://www.twitch.tv/{login}"))
            .send()
            .map_err(|_| "Could not load Twitch channel page".to_owned())?
            .text()
            .map_err(|_| "Could not read Twitch channel page".to_owned())?;
        let settings = extract_settings_url(&page)
            .ok_or_else(|| "Twitch settings script was not found".to_owned())?;
        let settings_url = Url::parse(&settings)
            .map_err(|_| "Twitch settings script URL was invalid".to_owned())?;
        let allowed = matches!(
            settings_url.host_str(),
            Some("static.twitchcdn.net") | Some("assets.twitch.tv")
        );
        if settings_url.scheme() != "https" || !allowed {
            return Err("Twitch settings script URL was rejected".into());
        }
        let script = self.client.get(settings_url)
            .send()
            .map_err(|_| "Could not load Twitch settings".to_owned())?
            .text()
            .map_err(|_| "Could not read Twitch settings".to_owned())?;
        let raw = extract_json_string(&script, "\"spade_url\":\"")
            .ok_or_else(|| "Twitch watch endpoint was not found".to_owned())?;
        let raw = raw.replace("\\/", "/").replace("\\u0026", "&");
        let url = Url::parse(&raw)
            .map_err(|_| "Twitch watch endpoint was invalid".to_owned())?;
        if url.scheme() != "https" || !url.host_str().is_some_and(|host| {
            host == "twitch.tv" || host.ends_with(".twitch.tv")
        }) {
            return Err("Twitch watch endpoint was rejected".into());
        }
        Ok(url)
    }
}

fn extract_settings_url(page: &str) -> Option<String> {
    for prefix in [
        "https://static.twitchcdn.net/config/settings",
        "https://assets.twitch.tv/config/settings",
    ] {
        let Some(start) = page.find(prefix) else { continue };
        let rest = &page[start..];
        let Some(end) = rest.find(".js") else { continue };
        return Some(rest[..end + 3].to_owned());
    }
    None
}

fn extract_json_string<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let start = text.find(prefix)? + prefix.len();
    let rest = &text[start..];
    let mut escaped = false;
    for (index, byte) in rest.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some(&rest[..index]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_url_is_limited_to_known_twitch_assets() {
        let page = r#"<script src="https://static.twitchcdn.net/config/settings.abc123.js"></script>"#;
        assert_eq!(
            extract_settings_url(page).as_deref(),
            Some("https://static.twitchcdn.net/config/settings.abc123.js")
        );
    }

    #[test]
    fn extracts_escaped_spade_value() {
        let script = r#"x={"spade_url":"https:\/\/spade.twitch.tv\/track"};"#;
        assert_eq!(
            extract_json_string(script, "\"spade_url\":\""),
            Some(r#"https:\/\/spade.twitch.tv\/track"#)
        );
    }
}
