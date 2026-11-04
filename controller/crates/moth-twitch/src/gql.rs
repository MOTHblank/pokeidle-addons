use crate::Credentials;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;

const CLIENT_ID: &str = "ue6666qo983tsx6so1t0vnawi233wa";
const GQL_URL: &str = "https://gql.twitch.tv/gql";
const USER_ID_HASH: &str = "94e82a7b1e3c21e186daa73ee2afc4b8f23bade1fbbff6fe8ac133f50a2f58ca";
const POINTS_CONTEXT_HASH: &str = "374314de591e69925fce3ddc2bcf085796f56ebb8cad67a0daa3165c03adc345";
const CLAIM_POINTS_HASH: &str = "46aaeebe02c99afdf4fc97c7c0cba964124bf6b0af229395f1f6d1feed05b3d0";

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

pub(crate) struct PointsContext {
    pub(crate) balance: u64,
    pub(crate) available_claim_id: Option<String>,
}

pub(crate) struct TwitchGql {
    client: Client,
    credentials: Credentials,
}

impl TwitchGql {
    pub(crate) fn new(credentials: Credentials) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Moth-Controller/0.1 Twitch")
            .build()
            .map_err(|_| "Could not create Twitch HTTP client".to_owned())?;
        Ok(Self { client, credentials })
    }

    fn post(&self, operation: &str, hash: &str, variables: Value) -> Result<Value, GqlError> {
        let body = json!({
            "operationName": operation,
            "variables": variables,
            "extensions": {
                "persistedQuery": { "version": 1, "sha256Hash": hash }
            }
        });
        let response = self.client.post(GQL_URL)
            .header("Authorization", format!("OAuth {}", self.credentials.oauth_token))
            .header("Client-Id", CLIENT_ID)
            .json(&body)
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
        let value: Value = response.json()
            .map_err(|_| GqlError::Protocol("Twitch returned invalid JSON".into()))?;
        if let Some(errors) = value.get("errors").and_then(Value::as_array) {
            if !errors.is_empty() {
                let auth = errors.iter().filter_map(|error| error.get("message"))
                    .filter_map(Value::as_str)
                    .any(|message| {
                        let lower = message.to_ascii_lowercase();
                        lower.contains("unauthorized") || lower.contains("auth")
                    });
                return Err(if auth {
                    GqlError::Auth
                } else {
                    GqlError::Protocol("Twitch rejected the GraphQL request".into())
                });
            }
        }
        Ok(value)
    }

    pub(crate) fn verify_login(&self) -> Result<(), GqlError> {
        match self.credentials.username.as_deref() {
            Some(username) => self.lookup_channel_id(username).map(|_| ()),
            None => Ok(()),
        }
    }

    pub(crate) fn lookup_channel_id(&self, login: &str) -> Result<String, GqlError> {
        let value = self.post(
            "GetIDFromLogin",
            USER_ID_HASH,
            json!({ "login": login }),
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
        let balance = points.get("balance").and_then(Value::as_u64).unwrap_or(0);
        let available_claim_id = points.pointer("/availableClaim/id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        Ok(PointsContext { balance, available_claim_id })
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
        if let Some(status) = value.pointer("/data/claimCommunityPoints/status")
            .and_then(Value::as_str)
        {
            if !matches!(status, "SUCCESS" | "ALREADY_CLAIMED") {
                return Err(GqlError::Protocol(format!(
                    "Twitch bonus claim returned {status}"
                )));
            }
        }
        Ok(())
    }
}
