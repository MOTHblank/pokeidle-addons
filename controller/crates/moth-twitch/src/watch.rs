use crate::gql::PlaybackAccess;
use base64::Engine as _;
use reqwest::blocking::Client;
use reqwest::Url;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct WatchAck {
    pub(crate) media_error: Option<String>,
}

pub(crate) struct WatchClient {
    client: Client,
    spade_url: Option<Url>,
    playlist_urls: HashMap<String, Url>,
}

impl WatchClient {
    pub(crate) fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0")
            .build()
            .map_err(|_| "Could not create Twitch watch client".to_owned())?;
        Ok(Self {
            client,
            spade_url: None,
            playlist_urls: HashMap::new(),
        })
    }

    /// Match the reference miner's lightweight viewing behavior: touch the
    /// current HLS media segments, then send Twitch's minute-watched event.
    /// Media failures are reported but do not suppress the telemetry attempt.
    pub(crate) fn send_minute_watched<F>(
        &mut self,
        login: &str,
        channel_id: &str,
        broadcast_id: &str,
        user_id: &str,
        playback: F,
    ) -> Result<WatchAck, String>
    where
        F: FnOnce() -> Result<PlaybackAccess, String>,
    {
        let media_error = self.touch_media(login, playback).err();

        let spade_url = match self.spade_url.clone() {
            Some(url) => url,
            None => {
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
            Ok(WatchAck { media_error })
        } else {
            self.spade_url = None;
            Err(format!(
                "Twitch watch event failed with HTTP {}", response.status().as_u16()
            ))
        }
    }

    fn touch_media<F>(&mut self, login:&str, playback:F)->Result<(),String>
    where
        F: FnOnce()->Result<PlaybackAccess,String>,
    {
        let playlist = match self.playlist_urls.get(login).cloned() {
            Some(url) => url,
            None => {
                let token=playback()?;
                let mut master=Url::parse("https://usher.ttvnw.net/api/channel/hls/")
                    .map_err(|_|"Could not build Twitch stream URL".to_owned())?;
                master.set_path(&format!("/api/channel/hls/{login}.m3u8"));
                master.query_pairs_mut()
                    .append_pair("sig",&token.signature)
                    .append_pair("token",&token.value);
                let variants=self.read_playlist(&master)?;
                let selected=variants.last().cloned()
                    .ok_or_else(||"Twitch stream master playlist was empty".to_owned())?;
                if !selected.path().ends_with(".m3u8") || !allowed_media_url(&selected) {
                    return Err("Twitch returned an invalid media playlist".into());
                }
                self.playlist_urls.insert(login.to_owned(),selected.clone());
                selected
            }
        };

        let segments=match self.read_playlist(&playlist) {
            Ok(segments)=>segments,
            Err(error)=>{
                self.playlist_urls.remove(login);
                return Err(error);
            }
        };
        for segment in segments.into_iter().take(32) {
            if !allowed_media_url(&segment) {
                self.playlist_urls.remove(login);
                return Err("Twitch returned an invalid media segment URL".into());
            }
            let response=self.client.head(segment).send()
                .map_err(|_|"Could not touch Twitch media segment".to_owned())?;
            if !response.status().is_success() {
                self.playlist_urls.remove(login);
                return Err(format!(
                    "Twitch media segment returned HTTP {}",
                    response.status().as_u16()
                ));
            }
        }
        Ok(())
    }

    fn read_playlist(&self,address:&Url)->Result<Vec<Url>,String> {
        let response=self.client.get(address.clone()).send()
            .map_err(|_|"Could not load Twitch stream playlist".to_owned())?;
        if !response.status().is_success() {
            return Err(format!(
                "Twitch stream playlist returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let text=response.text()
            .map_err(|_|"Could not read Twitch stream playlist".to_owned())?;
        parse_playlist(address,&text)
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

fn allowed_media_url(url:&Url)->bool {
    url.scheme()=="https" && url.username().is_empty() && url.password().is_none()
        && url.host_str().is_some_and(|host|
            host=="ttvnw.net" || host.ends_with(".ttvnw.net")
            || host=="twitchcdn.net" || host.ends_with(".twitchcdn.net"))
}

fn parse_playlist(base:&Url,text:&str)->Result<Vec<Url>,String> {
    let mut lines=text.lines().map(str::trim).filter(|line|!line.is_empty());
    if lines.next()!=Some("#EXTM3U") {
        return Err("Invalid Twitch stream playlist".into());
    }
    let mut entries=Vec::new();
    for line in lines {
        if line.starts_with('#') {continue;}
        let entry=base.join(line)
            .map_err(|_|"Invalid Twitch stream playlist entry".to_owned())?;
        if !matches!(entry.scheme(),"http"|"https") || entry.host_str().is_none()
            || !entry.username().is_empty() || entry.password().is_some() {
            return Err("Invalid Twitch stream playlist entry".into());
        }
        entries.push(entry);
    }
    if entries.is_empty() {
        return Err("Twitch stream playlist was empty".into());
    }
    Ok(entries)
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

    #[test]
    fn parses_relative_hls_entries() {
        let base=Url::parse("https://video-weaver.test.ttvnw.net/live/index.m3u8").unwrap();
        let entries=parse_playlist(&base,
            "#EXTM3U\n#EXTINF:2.0,\none.ts\n#EXTINF:2.0,\ntwo.ts\n").unwrap();
        assert_eq!(entries[0].as_str(),
            "https://video-weaver.test.ttvnw.net/live/one.ts");
        assert_eq!(entries[1].as_str(),
            "https://video-weaver.test.ttvnw.net/live/two.ts");
    }

    #[test]
    fn media_hosts_are_restricted_to_twitch_delivery() {
        assert!(allowed_media_url(&Url::parse(
            "https://video-weaver.test.hls.ttvnw.net/a.ts").unwrap()));
        assert!(!allowed_media_url(&Url::parse(
            "https://127.0.0.1/a.ts").unwrap()));
    }
}
