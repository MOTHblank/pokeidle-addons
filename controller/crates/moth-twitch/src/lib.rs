//! Native Twitch services shared by Moth Controller features.
//!
//! This crate deliberately has no egui dependency and uses the controller's
//! existing blocking HTTP + worker-thread model. It is an independent
//! implementation; third-party miners are protocol/behavior references only.

mod gql;
mod miner;
mod model;
mod pubsub;
mod streak;
mod watch;

pub use miner::{Miner, MinerControl};
pub use model::{
    normalize_login, Credentials, MinerConfig, MinerEvent, MinerPhase, MinerSnapshot,
    StreamerState, MAX_CHANNELS,
};
