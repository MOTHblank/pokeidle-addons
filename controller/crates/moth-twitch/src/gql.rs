use crate::client::{
    extract_twilight_build_id, random_hex, random_string, ANDROID_TV_USER_AGENT,
    DEFAULT_CLIENT_VERSION, REFERENCE_CLIENT_ID,
};
use crate::Credentials;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TWITCH_URL: &str = "https://www.twitch.tv";
const GQL_URL: &str = "https://gql.twitch.tv/gql";
const INTEGRITY_URL: &str = "https://gql.twitch.tv/integrity";
const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";
const BROWSER_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const USER_ID_HASH: &str = "94e82a7b1e3c21e186daa73ee2afc4b8f23bade1fbbff6fe8ac133f50a2f58ca";
const POINTS_CONTEXT_HASH: &str = "374314de591e69925fce3ddc2bcf085796f56ebb8cad67a0daa3165c03adc345";
const PLAYBACK_ACCESS_HASH: &str = "ed230aa1e33e07eebb8928504583da78a5173989fadfb1ac94be06a04f3cdbe9";
const CLAIM_POINTS_HASH: &str = "46aaeebe02c99afdf4fc97c7c0cba964124bf6b0af229395f1f6d1feed05b3d0";
const REWARD_LIST_HASH: &str = "0b1471876d7647993731b9e3c6a13bf304c67fb31d07f06a945d42286ee377c4";
const STREAM_INFO_HASH: &str = "e785b65ff71ad7b363b34878335f27dd9372869ad0c5740a130b9268bcdbe7e7";
const STREAM_LIVE_HASH: &str = "04e46329a6786ff3a81c01c50bfa5d725902507a0deb83b0edbf7abe7a3716ea";
const CLIENT_VERSION_TTL: Duration = Duration::from_secs(10 * 60 * 60);
const INTEGRITY_REFRESH_MARGIN_MS: u64 = 5 * 60 * 1000;

#[derive(Debug)]
pub(crate) enum GqlError {
    Auth,
    Transport(String),
    Protocol(String),
}

impl GqlError {
    pub(crate) fn user_message(&self) -> String {
        match self {
            Self::Auth => "Twitch sign-in expired".into(),
            Self::Transport(message) | Self::Protocol(message) => message.clone(),
        }
    }
    pub(crate) fn is_auth(&self) -> bool {
        matches!(self, Self::Auth)
    }
}

impl fmt::Display for GqlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ViewerIdentity {
    pub(crate) login: String,
    pub(crate) user_id: String,
}

pub(crate) struct PointsContext {
    pub(crate) balance: u64,
    pub(crate) available_claim_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PlaybackAccess {
    pub(crate) value: String,
    pub(crate) signature: String,
}

pub(crate) struct StreamInfo {
    pub(crate) broadcast_id: String,
    pub(crate) started_at: String,
    pub(crate) game_id: Option<String>,
    pub(crate) game_name: Option<String>,
}

pub(crate) struct LiveStream {
    pub(crate) started_at: Option<String>,
}

pub(crate) struct StreakMilestone {
    pub(crate) length: Option<u32>,
    pub(crate) achieved_at: Option<String>,
}

impl StreakMilestone {
    fn from_response(response:&Value)->Self {
        let milestone=response.pointer("/data/channel/self/watchStreakMilestone/watchStreakMilestone");
        Self {
            length: milestone.and_then(|item|item.get("value"))
                .and_then(|value| value.as_str()
                    .and_then(|s|s.parse::<u32>().ok())
                    .or_else(||value.as_u64().and_then(|n|u32::try_from(n).ok()))),
            achieved_at: milestone.and_then(|item|item.get("achievementTimestamp"))
                .and_then(Value::as_str).map(str::to_owned),
        }
    }
}

struct ClientVersionState {
    value: String,
    fetched: Option<Instant>,
}

struct IntegrityState {
    token: String,
    expiration_ms: u64,
}

fn epoch_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn stream_started_at<'a>(stream:&'a Value,fallback:Option<&'a str>)->Option<&'a str> {
    stream.get("createdAt").and_then(Value::as_str)
        .filter(|at|!at.is_empty())
        .or_else(||fallback.filter(|at|!at.is_empty()))
}

fn has_integrity_challenge(value: &Value) -> bool {
    if value.pointer("/extensions/challenge/type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("integrity"))
    {
        return true;
    }
    value.get("errors").and_then(Value::as_array)
        .is_some_and(|errors| errors.iter().any(|error| {
            error.get("message").and_then(Value::as_str)
                .is_some_and(|message| message.to_ascii_lowercase().contains("integrity"))
        }))
}

pub(crate) struct TwitchGql {
    client: Client,
    credentials: Credentials,
    client_id: Option<String>,
    user_id: Option<String>,
    device_id: String,
    client_session: String,
    client_version: Mutex<ClientVersionState>,
    integrity: Mutex<IntegrityState>,
}

impl TwitchGql {
    pub(crate) fn new(credentials: Credentials) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(ANDROID_TV_USER_AGENT)
            .build()
            .map_err(|_| "Could not create Twitch HTTP client".to_owned())?;
        Ok(Self {
            client,
            device_id: credentials.device_id.clone()
                .unwrap_or_else(||random_string(32)),
            credentials,
            client_id: None,
            user_id: None,
            client_session: random_hex(8),
            client_version: Mutex::new(ClientVersionState {
                value: DEFAULT_CLIENT_VERSION.to_owned(),
                fetched: None,
            }),
            integrity: Mutex::new(IntegrityState {
                token: String::new(),
                expiration_ms: 0,
            }),
        })
    }

    /// Validate the token for viewer identity. Reference-device tokens report
    /// REFERENCE_CLIENT_ID here; imported browser tokens keep their issuing
    /// client id so old saved sessions remain usable.
    pub(crate) fn authenticate(&mut self) -> Result<ViewerIdentity, GqlError> {
        let response = self.client.get(VALIDATE_URL)
            .header("Authorization", format!("OAuth {}", self.credentials.oauth_token))
            .header("User-Agent", ANDROID_TV_USER_AGENT)
            .send()
            .map_err(|_| GqlError::Transport("Could not validate Twitch sign-in".into()))?;
        if response.status().as_u16() == 401 {
            return Err(GqlError::Auth);
        }
        if !response.status().is_success() {
            return Err(GqlError::Transport(format!(
                "Twitch sign-in validation failed with HTTP {}", response.status().as_u16()
            )));
        }
        let value: Value = response.json()
            .map_err(|_| GqlError::Protocol("Twitch returned invalid sign-in data".into()))?;
        let client_id = value.get("client_id").and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| GqlError::Protocol("Twitch sign-in did not include a client id".into()))?;
        let login = value.get("login").and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| GqlError::Protocol("Twitch sign-in did not include a username".into()))?;
        let user_id = value.get("user_id").and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| GqlError::Protocol("Twitch sign-in did not include a user id".into()))?;
        self.client_id = Some(if self.credentials.device_id.is_some() {
            REFERENCE_CLIENT_ID.to_owned()
        } else {
            client_id.to_owned()
        });
        self.user_id = Some(user_id.to_owned());
        Ok(ViewerIdentity {
            login: login.to_ascii_lowercase(),
            user_id: user_id.to_owned(),
        })
    }

    fn client_version(&self) -> String {
        let now=Instant::now();
        let fallback = match self.client_version.lock() {
            Ok(mut state) => {
                if state.fetched.is_some_and(|at|
                    now.saturating_duration_since(at)<CLIENT_VERSION_TTL) {
                    return state.value.clone();
                }
                state.fetched=Some(now);
                state.value.clone()
            }
            Err(_) => DEFAULT_CLIENT_VERSION.to_owned(),
        };
        let updated=self.client.get(TWITCH_URL)
            .header("Cookie",self.session_cookie())
            .send().ok()
            .filter(|response|response.status().is_success())
            .and_then(|response|response.text().ok())
            .and_then(|page|extract_twilight_build_id(&page));
        let Some(updated)=updated else {return fallback};
        if let Ok(mut state)=self.client_version.lock() {
            state.value=updated.clone();
        }
        updated
    }

    fn session_cookie(&self)->String {
        match self.user_id.as_deref() {
            Some(user_id)=>format!(
                "auth-token={}; persistent={user_id}",self.credentials.oauth_token
            ),
            None=>format!("auth-token={}",self.credentials.oauth_token),
        }
    }

    fn clear_integrity_token(&self) {
        if let Ok(mut state) = self.integrity.lock() {
            state.token.clear();
            state.expiration_ms = 0;
        }
    }

    fn integrity_token(&self) -> Result<String, GqlError> {
        let now = epoch_ms();
        let mut state = self.integrity.lock()
            .map_err(|_| GqlError::Protocol("Twitch integrity state unavailable".into()))?;
        if !state.token.is_empty()
            && state.expiration_ms.saturating_sub(now) > INTEGRITY_REFRESH_MARGIN_MS
        {
            return Ok(state.token.clone());
        }

        let response = self.client.post(INTEGRITY_URL)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("OAuth {}", self.credentials.oauth_token))
            .header("Client-Id", BROWSER_CLIENT_ID)
            .header("Client-Session-Id", &self.client_session)
            .header("X-Device-Id", &self.device_id)
            .header("User-Agent", ANDROID_TV_USER_AGENT)
            .body("{}")
            .send()
            .map_err(|_| GqlError::Transport(
                "Could not refresh Twitch integrity token".into()
            ))?;
        if !response.status().is_success() {
            return Err(GqlError::Protocol(format!(
                "Twitch integrity token request failed with HTTP {}",
                response.status().as_u16()
            )));
        }
        let value: Value = response.json()
            .map_err(|_| GqlError::Protocol(
                "Twitch returned invalid integrity token data".into()
            ))?;
        let token = value.get("token").and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| GqlError::Protocol(
                "Twitch did not return an integrity token".into()
            ))?;
        let expiration_ms = value.get("expiration").and_then(Value::as_u64)
            .unwrap_or_else(|| now.saturating_add(10 * 60 * 1000));
        state.token = token.to_owned();
        state.expiration_ms = expiration_ms;
        Ok(state.token.clone())
    }

    fn post_raw(
        &self,
        operation: &str,
        hash: &str,
        variables: &Value,
        client_id: &str,
        integrity_token: Option<&str>,
    ) -> Result<Value, GqlError> {
        let body = json!({
            "operationName": operation,
            "variables": variables,
            "extensions": {
                "persistedQuery": { "version": 1, "sha256Hash": hash }
            }
        });
        let mut request = self.client.post(GQL_URL)
            .header("Authorization", format!("OAuth {}", self.credentials.oauth_token))
            .header("Client-Id", client_id)
            .header("Client-Session-Id", &self.client_session)
            .header("Client-Version", self.client_version())
            .header("User-Agent", ANDROID_TV_USER_AGENT)
            .header("X-Device-Id", &self.device_id)
            .header("Cookie", self.session_cookie());
        if let Some(token) = integrity_token {
            request = request.header("Client-Integrity", token);
        }
        let response = request.json(&body)
            .send()
            .map_err(|_| GqlError::Transport("Could not reach Twitch".into()))?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(GqlError::Auth);
        }
        if !response.status().is_success() {
            return Err(GqlError::Transport(format!(
                "Twitch request failed with HTTP {}", response.status().as_u16()
            )));
        }
        response.json()
            .map_err(|_| GqlError::Protocol("Twitch returned invalid JSON".into()))
    }

    fn finish_graphql(&self, operation: &str, value: Value) -> Result<Value, GqlError> {
        if let Some(errors) = value.get("errors").and_then(Value::as_array) {
            if !errors.is_empty() {
                let messages=errors.iter().filter_map(|error| error.get("message"))
                    .filter_map(Value::as_str).collect::<Vec<_>>();
                let auth = messages.iter().any(|message| {
                    let lower = message.to_ascii_lowercase();
                    lower.contains("unauthorized") || lower.contains("auth")
                });
                return Err(if auth {
                    GqlError::Auth
                } else {
                    let detail=messages.first().map(|message| {
                        let clean=message.chars()
                            .filter(|ch|!ch.is_control())
                            .take(180).collect::<String>();
                        if clean.is_empty() {"no error detail".to_owned()} else {clean}
                    }).unwrap_or_else(||"no error detail".to_owned());
                    GqlError::Protocol(format!(
                        "Twitch rejected GraphQL {operation}: {detail}"
                    ))
                });
            }
        }
        Ok(value)
    }

    fn post(&self, operation: &str, hash: &str, variables: Value) -> Result<Value, GqlError> {
        let client_id = self.client_id.as_deref().ok_or(GqlError::Auth)?;
        let initial = self.post_raw(operation, hash, &variables, client_id, None)?;
        if !has_integrity_challenge(&initial) {
            return self.finish_graphql(operation, initial);
        }

        // Protected Twitch mutations can challenge the TV/reference GQL
        // identity even when the OAuth token itself is still valid. Mirror
        // current channel-points clients: obtain a short-lived integrity token
        // with Twitch's browser client identity and retry only the challenged
        // request, leaving ordinary stream/live requests on the reference path.
        self.clear_integrity_token();
        let mut last = initial;
        for _ in 0..2 {
            let integrity = match self.integrity_token() {
                Ok(token) => token,
                Err(error) => {
                    return Err(GqlError::Protocol(format!(
                        "Twitch rejected GraphQL {operation}: failed integrity check; {}",
                        error.user_message()
                    )));
                }
            };
            match self.post_raw(
                operation,
                hash,
                &variables,
                BROWSER_CLIENT_ID,
                Some(&integrity),
            ) {
                Ok(value) => {
                    if !has_integrity_challenge(&value) {
                        return self.finish_graphql(operation, value);
                    }
                    last = value;
                    self.clear_integrity_token();
                }
                Err(GqlError::Auth) => {
                    // A browser-client retry can be rejected independently of
                    // an otherwise-valid TV token. Preserve the real failure
                    // instead of incorrectly forcing the user to sign in again.
                    return self.finish_graphql(operation, last);
                }
                Err(error) => return Err(error),
            }
        }
        self.finish_graphql(operation, last)
    }

    pub(crate) fn oauth_token(&self) -> &str {
        &self.credentials.oauth_token
    }

    pub(crate) fn reference_client(&self)->bool {
        self.client_id.as_deref()==Some(REFERENCE_CLIENT_ID)
    }

    pub(crate) fn playback_access(&self, login:&str) -> Result<PlaybackAccess,GqlError> {
        let value=self.post(
            "PlaybackAccessToken",
            PLAYBACK_ACCESS_HASH,
            json!({
                "isLive":true,"isVod":false,"login":login,
                "platform":"web","playerType":"site","vodID":""
            })
        )?;
        let token=value.pointer("/data/streamPlaybackAccessToken")
            .ok_or_else(||GqlError::Protocol(format!(
                "Twitch playback token unavailable for {login}"
            )))?;
        let access=token.get("value").and_then(Value::as_str)
            .filter(|value|!value.is_empty())
            .ok_or_else(||GqlError::Protocol(format!(
                "Twitch playback token value unavailable for {login}"
            )))?;
        let signature=token.get("signature").and_then(Value::as_str)
            .filter(|value|!value.is_empty())
            .ok_or_else(||GqlError::Protocol(format!(
                "Twitch playback signature unavailable for {login}"
            )))?;
        Ok(PlaybackAccess {
            value:access.to_owned(), signature:signature.to_owned()
        })
    }

    pub(crate) fn lookup_channel_id(&self, login: &str) -> Result<String, GqlError> {
        let value = self.post(
            "GetIDFromLogin",
            USER_ID_HASH,
            json!({ "login": login.to_ascii_lowercase() }),
        )?;
        value.pointer("/data/user/id").and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| GqlError::Protocol(format!("Twitch channel {login} was not found")))
    }

    pub(crate) fn points_context(&self, login: &str) -> Result<PointsContext, GqlError> {
        let value = self.post(
            "ChannelPointsContext",
            POINTS_CONTEXT_HASH,
            json!({ "channelLogin": login }),
        )?;
        let points = value.pointer("/data/community/channel/self/communityPoints")
            .ok_or_else(|| GqlError::Protocol(format!(
                "Channel points are unavailable for {login}"
            )))?;
        let balance = points.get("balance").and_then(Value::as_u64)
            .ok_or_else(|| GqlError::Protocol(format!(
                "Twitch did not return a confirmed point balance for {login}"
            )))?;
        let available_claim_id = points.pointer("/availableClaim/id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        Ok(PointsContext { balance, available_claim_id })
    }

    pub(crate) fn is_stream_live(&self, channel_id: &str) -> Result<Option<LiveStream>, GqlError> {
        let value = self.post(
            "WithIsStreamLiveQuery",
            STREAM_LIVE_HASH,
            json!({ "id": channel_id }),
        )?;
        let Some(stream) = value.pointer("/data/user/stream") else {
            return Ok(None);
        };
        if stream.is_null() {
            return Ok(None);
        }
        Ok(Some(LiveStream {
            started_at: stream.get("createdAt").and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
        }))
    }

    pub(crate) fn stream_info(
        &self,
        login: &str,
        fallback_started_at: Option<&str>,
    ) -> Result<Option<StreamInfo>, GqlError> {
        let value = self.post(
            "VideoPlayerStreamInfoOverlayChannel",
            STREAM_INFO_HASH,
            json!({ "channel": login }),
        )?;
        let Some(user)=value.pointer("/data/user") else {
            return Err(GqlError::Protocol(format!(
                "Twitch stream user data is unavailable for {login}"
            )));
        };
        if user.is_null() {
            return Err(GqlError::Protocol(format!(
                "Twitch channel {login} was not found"
            )));
        }
        let Some(stream)=user.get("stream") else {return Ok(None)};
        if stream.is_null() || user.get("broadcastSettings").is_none_or(Value::is_null) {
            return Ok(None);
        }
        let broadcast_id = stream.get("id").and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| GqlError::Protocol(format!(
                "Twitch live stream id is unavailable for {login}"
            )))?;
        let started_at=stream_started_at(stream,fallback_started_at)
            .ok_or_else(|| GqlError::Protocol(format!(
                "Twitch stream start time unavailable for {login}"
            )))?;
        let game=user.pointer("/broadcastSettings/game");
        Ok(Some(StreamInfo {
            broadcast_id: broadcast_id.to_owned(),
            started_at: started_at.to_owned(),
            game_id: game.and_then(|game|game.get("id")).and_then(Value::as_str)
                .filter(|value|!value.is_empty()).map(str::to_owned),
            game_name: game.and_then(|game|game.get("name")).and_then(Value::as_str)
                .filter(|value|!value.is_empty()).map(str::to_owned),
        }))
    }

    pub(crate) fn streak(&self, channel_id: &str) -> Result<StreakMilestone, GqlError> {
        let response=self.post("RewardList",REWARD_LIST_HASH,json!({
            "channelID":channel_id,"shouldIncludeAllSuspendedStreaks":false
        }))?;
        Ok(StreakMilestone::from_response(&response))
    }

    pub(crate) fn claim_bonus(&self, channel_id: &str, claim_id: &str) -> Result<(), GqlError> {
        let value = self.post(
            "ClaimCommunityPoints",
            CLAIM_POINTS_HASH,
            json!({ "input": { "channelID": channel_id, "claimID": claim_id } }),
        )?;
        if let Some(message) = value.pointer("/data/claimCommunityPoints/error/message")
            .and_then(Value::as_str)
            .filter(|message| !message.is_empty())
        {
            return Err(GqlError::Protocol(format!("Twitch bonus claim failed: {message}")));
        }
        let claimed = value.pointer("/data/claimCommunityPoints")
            .filter(|response| !response.is_null())
            .ok_or_else(|| GqlError::Protocol(
                "Twitch did not acknowledge the bonus claim".into()
            ))?;
        if let Some(status) = claimed.get("status").and_then(Value::as_str) {
            if !matches!(status, "SUCCESS" | "ALREADY_CLAIMED") {
                return Err(GqlError::Protocol(format!(
                    "Twitch bonus claim returned {status}"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_rejection_message_identifies_operation() {
        let message=format!("Twitch rejected GraphQL {}: {}",
            "ChannelPointsContext","PersistedQueryNotFound");
        assert!(message.contains("ChannelPointsContext"));
        assert!(message.contains("PersistedQueryNotFound"));
    }

    #[test]
    fn parses_reward_list_milestone_and_absent_streak() {
        let result=json!({"data":{"channel":{"self":{"watchStreakMilestone":{
            "watchStreakMilestone":{
                "value":"5","achievementTimestamp":"2026-11-04T20:15:00Z"
            }
        }}}}});
        let milestone=StreakMilestone::from_response(&result);
        assert_eq!(milestone.length,Some(5));
        assert_eq!(milestone.achieved_at.as_deref(),Some("2026-11-04T20:15:00Z"));
        let absent=StreakMilestone::from_response(&json!({"data":{"channel":{"self":{}}}}));
        assert_eq!(absent.length,None);
        assert_eq!(absent.achieved_at,None);
    }

    #[test]
    fn reference_client_id_matches_reference_miner() {
        assert_eq!(REFERENCE_CLIENT_ID,"ue6666qo983tsx6so1t0vnawi233wa");
    }

    #[test]
    fn detects_integrity_challenges() {
        assert!(has_integrity_challenge(&json!({
            "errors":[{"message":"failed integrity check"}]
        })));
        assert!(has_integrity_challenge(&json!({
            "extensions":{"challenge":{"type":"integrity"}}
        })));
        assert!(!has_integrity_challenge(&json!({
            "errors":[{"message":"PersistedQueryNotFound"}]
        })));
    }

    #[test]
    fn browser_client_id_matches_twitch_web_client() {
        assert_eq!(BROWSER_CLIENT_ID,"kimne78kx3ncx6brgo4mv6wki5h1ko");
    }

    #[test]
    fn stream_start_time_can_fall_back_to_live_query() {
        let stream=json!({"id":"broadcast-1"});
        assert_eq!(
            stream_started_at(&stream,Some("2026-11-04T20:00:00Z")),
            Some("2026-11-04T20:00:00Z")
        );
        let stream=json!({"id":"broadcast-1","createdAt":"2026-11-04T21:00:00Z"});
        assert_eq!(
            stream_started_at(&stream,Some("2026-11-04T20:00:00Z")),
            Some("2026-11-04T21:00:00Z")
        );
    }
}
