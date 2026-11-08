pub const ENABLE_ENV: &str = "MOTH_HUNT_PACING_TRACE";
pub const DIRECTORY_ENV: &str = "MOTH_HUNT_PACING_TRACE_DIR";
pub const PROFILE_ENV: &str = "MOTH_HUNT_PACING_PROFILE";
pub const GAME_VERSION_ENV: &str = "MOTH_HUNT_PACING_GAME_VERSION";

pub fn instructions() -> String {
    format!(
r#"Live collection is disabled by default and uses the controller's existing game socket.

Set:
  {ENABLE_ENV}=1
  {PROFILE_ENV}=st1

Optional:
  {GAME_VERSION_ENV}=<game version override>
  {DIRECTORY_ENV}=<raw trace directory>

The collector reads the game version from the authenticated welcome when the
server exposes it; {GAME_VERSION_ENV} is only an explicit override.

Profiles:
  st1                 on foot, single-target, deliberately configured one-shot calibration
  aoe                 on foot, stable AoE, deliberately configured one-shot calibration
  bike-validation     deliberate bicycle-factor validation
  deadtime-validation deliberate 1/2/3/multi-hit dead-time validation
  diagnostic          raw capture only; never enters the production pacing aggregates

The collector stores only campo.init, campo, batalha and estado packets plus normalized
calibration context. It never opens a second socket and never reads map geometry."#
    )
}
