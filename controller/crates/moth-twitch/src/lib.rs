//! Native Twitch services shared by Moth Controller features.
//!
//! This crate deliberately has no egui dependency and uses the controller's
//! existing blocking HTTP + worker-thread model. It is an independent
//! implementation; third-party miners are protocol/behavior references only.

mod client;
mod gql;
mod miner;
mod model;
mod pubsub;
mod streak;
mod watch;

pub use client::{
    encode_reference_credential, random_hex, random_string, ANDROID_TV_USER_AGENT,
    DEFAULT_CLIENT_VERSION,
    DEVICE_FLOW_SCOPES, LINUX_FIREFOX_USER_AGENT, REFERENCE_CLIENT_ID,
};
pub use miner::{Miner, MinerControl};
pub use model::{
    normalize_login, Credentials, MinerConfig, MinerEvent, MinerPhase, MinerSnapshot,
    StreamerState, MAX_CHANNELS,
};
