//! Native PvP matchmaking only. Battles remain entirely server-simulated.
//! This coordinator never changes a hunt, leaves combat, or issues battle actions.
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const INFO_INTERVAL: Duration = Duration::from_secs(150);
const JOIN_ACK_TIMEOUT: Duration = Duration::from_secs(15);
const REFUSAL_BACKOFF: Duration = Duration::from_secs(60);
const VIP_SYNC_RETRY: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Default)]
pub struct PvpStatus {
    pub enabled: bool,
    pub vip: bool,
    pub queued: bool,
    pub awaiting_reply: bool,
    pub cooldown_secs: u64,
    pub retry_secs: u64,
    pub waiting_for_team: bool,
    pub waiting_for_level: Option<u32>,
    pub checking_queue: bool,
    pub paused: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PvpAction { Info, Join, Leave, SetVipAuto(bool) }

impl PvpAction {
    pub fn wire(self) -> Option<Value> {
        match self {
            Self::Info => Some(json!({"t":"pvp.info"})),
            Self::Join => Some(json!({"t":"pvp.fila.entrar"})),
            Self::Leave => Some(json!({"t":"pvp.fila.sair"})),
            // Uses AutomationWriter's server-state-preserving auto.set path.
            Self::SetVipAuto(_) => None,
        }
    }
}

/// All timers and queue ownership belong to one authenticated socket worker.
/// No UI frame, external timer, or extra thread is involved.
pub struct PvpAutomation {
    queue_known: bool,
    queued: bool,
    owned_queue: bool,
    team_ready: Option<bool>,
    min_level: Option<u32>,
    cooldown_until: Option<Instant>,
    join_sent: Option<Instant>,
    next_join_at: Option<Instant>,
    next_info_at: Option<Instant>,
    last_vip_sync: Option<(bool, Instant)>,
    vip_cancel_pending: bool,
    vip_rejected: bool,
    leave_sent: Option<Instant>,
    warning: Option<String>,
}

impl Default for PvpAutomation {
    fn default() -> Self {
        Self {
            queue_known: false, queued: false, owned_queue: false,
            team_ready: None, min_level: None, cooldown_until: None,
            join_sent: None, next_join_at: None, next_info_at: None,
            last_vip_sync: None, vip_cancel_pending: false,
            vip_rejected: false, leave_sent: None, warning: None,
        }
    }
}

impl PvpAutomation {
    pub fn on_frame(&mut self, frame: &Value, now: Instant) {
        if frame.get("t").and_then(Value::as_str) != Some("pvp") { return; }
        // The game reports the initial PvP team as 'time', but an edit
        // acknowledgement uses 'timeSalvo'. Both are authoritative.
        if let Some(time) = frame.get("timeSalvo").or_else(|| frame.get("time"))
            .and_then(Value::as_array) {
            self.team_ready = Some(!time.is_empty());
        }
        if let Some(level) = frame.get("regras")
            .and_then(|rules| rules.get("nivelMin")).and_then(Value::as_u64)
            .and_then(|level| u32::try_from(level).ok()) {
            self.min_level = Some(level);
        }
        if let Some(fila) = frame.get("fila").and_then(Value::as_object) {
            if let Some(na) = fila.get("na").and_then(Value::as_bool) {
                self.queued = na;
                if na {
                    self.queue_known = true;
                    self.join_sent = None;
                    if !self.vip_rejected { self.warning = None; }
                } else if self.join_sent.is_some() {
                    // A late info response may predate our pending Join.
                    self.queue_known = false;
                } else {
                    self.queue_known = true;
                    self.owned_queue = false;
                    self.leave_sent = None;
                }
            }
            if let Some(ms) = fila.get("cooldownMs").and_then(Value::as_u64) {
                self.cooldown_until = Some(now + Duration::from_millis(ms));
            }
        }
        if let Some(recusa) = frame.get("filaRecusa") {
            self.queued = false;
            self.queue_known = true;
            self.owned_queue = false;
            self.join_sent = None;
            self.leave_sent = None;
            self.next_join_at = Some(now + REFUSAL_BACKOFF);
            if let Some(ms) = recusa.get("restaMs").and_then(Value::as_u64) {
                self.cooldown_until = Some(now + Duration::from_millis(ms));
            }
            self.warning = Some("PokéIdle refused the PvP queue request; checking eligibility before retrying".into());
            self.queue_known = false;
            self.next_info_at = Some(now + REFUSAL_BACKOFF);
        }
        if frame.get("partida").is_some_and(|p| !p.is_null()) {
            // A match is automatic; the server's next queue/cooldown state
            // decides when another matchmaking request is allowed.
            self.queued = false;
            self.queue_known = false;
            self.owned_queue = false;
            self.join_sent = None;
            self.leave_sent = None;
            self.next_info_at = None;
            self.next_join_at = None;
        }
        if frame.get("filaAutoDesligada").and_then(Value::as_bool) == Some(true) {
            self.vip_rejected = true;
            self.warning = Some("PokéIdle disabled VIP automatic matchmaking. Check your PvP team and trainer level, then turn Auto PvP off and on to retry.".into());
        }
    }

    /// The UI may toggle off and back on between worker ticks. Its revision
    /// ensures this explicit rearm is observed even if the final state is on.
    pub fn on_user_toggle(&mut self) {
        self.vip_rejected = false;
        self.last_vip_sync = None;
        self.warning = None;
    }

    pub fn on_reconnect(&mut self) {
        self.queue_known = false;
        self.queued = false;
        self.team_ready = None;
        self.min_level = None;
        self.cooldown_until = None;
        self.join_sent = None;
        self.next_info_at = None;
        self.next_join_at = None;
        self.last_vip_sync = None;
        self.vip_cancel_pending = false;
        self.leave_sent = None;
        // Preserve ownership to cancel an in-flight queue if the user disabled
        // Auto PvP while disconnected and the server restored their entry.
    }

    pub fn enabled(preference: Option<bool>, vip: bool, vip_auto: Option<bool>) -> bool {
        preference.unwrap_or(vip && vip_auto == Some(true))
    }

    pub fn status(&self, preference: Option<bool>, vip: bool,
        vip_auto: Option<bool>, level: Option<u32>, now: Instant) -> PvpStatus {
        let enabled = Self::enabled(preference, vip, vip_auto);
        PvpStatus {
            enabled, vip, queued: self.queued,
            awaiting_reply: self.join_sent.is_some(),
            cooldown_secs: self.cooldown_until.map(|until|
                until.saturating_duration_since(now).as_secs()).unwrap_or(0),
            retry_secs: self.next_join_at.map(|until|
                until.saturating_duration_since(now).as_secs()).unwrap_or(0),
            waiting_for_team: enabled && !vip && self.team_ready == Some(false),
            waiting_for_level: if enabled && !vip {
                self.min_level.filter(|minimum| level.is_none_or(|lv| lv < *minimum))
            } else { None },
            checking_queue: enabled && !self.queue_known,
            paused: enabled && vip && self.vip_rejected,
            warning: enabled.then(|| self.warning.clone()).flatten(),
        }
    }

    /// Returns at most one low-frequency wire intent per tick. A VIP account
    /// uses the game's native auto-queue and NEVER joins via the client loop.
    pub fn next(&mut self, preference: Option<bool>, vip: bool,
        vip_auto: Option<bool>, level: Option<u32>, now: Instant) -> Option<PvpAction> {
        let enabled = Self::enabled(preference, vip, vip_auto);

        // Turning off a queue we started must cancel it. Do not cancel a
        // player-created queue when this controller never joined it.
        if !enabled && self.owned_queue {
            // Wait for an outstanding Join acknowledgement before cancelling;
            // the server can complete independent handlers out of order.
            if let Some(sent) = self.join_sent {
                if now.saturating_duration_since(sent) < JOIN_ACK_TIMEOUT {
                    return None;
                }
                self.join_sent = None;
                self.queue_known = false;
                self.next_info_at = None;
            }
            // A timed-out Leave is verified with Info, not blindly repeated.
            if let Some(sent) = self.leave_sent {
                if now.saturating_duration_since(sent) < JOIN_ACK_TIMEOUT {
                    return None;
                }
                self.leave_sent = None;
                self.queue_known = false;
                self.next_info_at = None;
            }
            if !self.queue_known {
                if self.next_info_at.is_none_or(|due| now >= due) {
                    self.next_info_at = Some(now + INFO_INTERVAL);
                    return Some(PvpAction::Info);
                }
                return None;
            }
            if !self.queued {
                self.owned_queue = false;
                return None;
            }
            self.leave_sent = Some(now);
            return Some(PvpAction::Leave);
        }
        if vip {
            if preference == Some(false) {
                // The user must acknowledge a server refusal before retrying.
                self.vip_rejected = false;
            }
            // None means inherit the existing server VIP preference. Avoid
            // overriding the user's in-game setting merely by connecting.
            if let Some(wanted) = preference {
                if wanted { self.vip_cancel_pending = false; }
                if !(wanted && self.vip_rejected) && vip_auto != Some(wanted) &&
                    self.last_vip_sync.is_none_or(|(old, sent)|
                        old != wanted || now.saturating_duration_since(sent) >= VIP_SYNC_RETRY) {
                    if !wanted && vip_auto == Some(true) {
                        // Disable the server auto-join first. Cancel its
                        // currently active search only after confirmation.
                        self.vip_cancel_pending = true;
                        self.next_info_at = None;
                    }
                    self.last_vip_sync = Some((wanted, now));
                    return Some(PvpAction::SetVipAuto(wanted));
                }
            }
            if self.vip_cancel_pending && vip_auto == Some(false) {
                if !self.queue_known {
                    if self.next_info_at.is_none_or(|due| now >= due) {
                        self.next_info_at = Some(now + INFO_INTERVAL);
                        return Some(PvpAction::Info);
                    }
                    return None;
                }
                self.vip_cancel_pending = false;
                if self.queued {
                    self.queued = false;
                    self.queue_known = false;
                    return Some(PvpAction::Leave);
                }
            }
            // A VIP queue can also run with the PvP page closed. Query its
            // status occasionally, but never submit a client-side Join.
            if enabled && self.next_info_at.is_none_or(|due| now >= due) {
                self.next_info_at = Some(now + INFO_INTERVAL);
                return Some(PvpAction::Info);
            }
            return None;
        }
        if !enabled { return None; }

        if let Some(sent) = self.join_sent {
            if now.saturating_duration_since(sent) < JOIN_ACK_TIMEOUT { return None; }
            // Unknown result: query the server instead of blindly joining again.
            self.join_sent = None;
            self.queue_known = false;
            self.next_info_at = None;
        }
        if self.next_info_at.is_none_or(|due| now >= due)
            && (!self.queue_known || self.queued || self.team_ready.is_none()) {
            self.next_info_at = Some(now + INFO_INTERVAL);
            return Some(PvpAction::Info);
        }
        if !self.queue_known { return None; }
        if self.queued || self.owned_queue { return None; }
        if self.team_ready != Some(true) ||
            self.min_level.is_none_or(|min| level.is_none_or(|lv| lv < min)) {
            if self.next_info_at.is_none_or(|due| now >= due) {
                self.next_info_at = Some(now + INFO_INTERVAL);
                return Some(PvpAction::Info);
            }
            return None;
        }
        if self.cooldown_until.is_some_and(|until| now < until) ||
            self.next_join_at.is_some_and(|until| now < until) {
            return None;
        }
        self.owned_queue = true;
        self.join_sent = Some(now);
        self.next_join_at = Some(now + REFUSAL_BACKOFF);
        self.warning = None;
        Some(PvpAction::Join)
    }

    pub fn send_failed(&mut self, action: PvpAction) {
        if action == PvpAction::Join {
            self.owned_queue = false;
            self.join_sent = None;
            self.queue_known = false;
        }
        if action == PvpAction::Leave {
            self.leave_sent = None;
            self.queue_known = false;
            self.next_info_at = None;
        }
        self.warning = Some("Could not send PvP matchmaking request".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_vip_requests_info_then_joins_without_hunt_or_battle_commands() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        assert_eq!(p.next(Some(true), false, None, Some(25), t), Some(PvpAction::Info));
        assert_eq!(p.next(Some(true), false, None, Some(25), t), None);
        p.on_frame(&json!({"t":"pvp","time":[7],"regras":{"nivelMin":10},
            "fila":{"na":false,"cooldownMs":0}}),t);
        assert_eq!(p.next(Some(true), false, None, Some(25), t), Some(PvpAction::Join));
        assert_eq!(p.next(Some(true), false, None, Some(25), t), None);
        assert_eq!(PvpAction::Join.wire().unwrap(),json!({"t":"pvp.fila.entrar"}));
    }
    #[test]
    fn no_team_low_level_cooldown_and_refusal_are_respected() {
        let t=Instant::now();
        let mut p=PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","time":[],"regras":{"nivelMin":30},
            "fila":{"na":false}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(20),t),Some(PvpAction::Info));
        p.on_frame(&json!({"t":"pvp","time":[1],"fila":{"na":false,"cooldownMs":20000}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(20),t),None);
        assert_eq!(p.next(Some(true),false,None,Some(30),t+Duration::from_secs(19)),None);
        assert_eq!(p.next(Some(true),false,None,Some(30),t+Duration::from_secs(21)),Some(PvpAction::Join));
        p.on_frame(&json!({"t":"pvp","filaRecusa":{"restaMs":9000}}),t+Duration::from_secs(21));
        assert_eq!(p.next(Some(true),false,None,Some(30),t+Duration::from_secs(25)),None);
    }
    #[test]
    fn team_save_receipt_and_real_level_threshold_update_eligibility() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","time":[],"regras":{"nivelMin":35},
            "fila":{"na":false}}),t);
        let before = p.status(Some(true),false,None,Some(34),t);
        assert!(before.waiting_for_team);
        assert_eq!(before.waiting_for_level,Some(35));
        p.on_frame(&json!({"t":"pvp","timeSalvo":[11,22]}),t);
        let after = p.status(Some(true),false,None,Some(35),t);
        assert!(!after.waiting_for_team);
        assert_eq!(after.waiting_for_level,None);
        assert_eq!(p.next(Some(true),false,None,Some(35),t),Some(PvpAction::Join));
    }
    #[test]
    fn refusal_backoff_is_visible_and_never_blindly_rejoins() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","filaRecusa":{"restaMs":18000}}),t);
        let state = p.status(Some(true),false,None,Some(50),t);
        assert!(state.checking_queue);
        assert_eq!(state.cooldown_secs,18);
        assert_eq!(state.retry_secs,60);
        assert_eq!(p.next(Some(true),false,None,Some(50),
            t + Duration::from_secs(30)),None);
        assert_eq!(p.next(Some(true),false,None,Some(50),
            t + Duration::from_secs(61)),Some(PvpAction::Info));
    }
    #[test]
    fn disabling_only_cancels_a_queue_owned_by_this_controller() {
        let t=Instant::now();
        let mut p=PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        assert_eq!(p.next(Some(false),false,None,Some(40),t),None);
        p.on_frame(&json!({"t":"pvp","time":[1],"regras":{"nivelMin":10},
            "fila":{"na":false}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(40),t),Some(PvpAction::Join));
        // A fast toggle must not race the pending server-side Join.
        assert_eq!(p.next(Some(false),false,None,Some(40),t),None);
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        assert_eq!(p.next(Some(false),false,None,Some(40),t),Some(PvpAction::Leave));
        assert_eq!(p.next(Some(false),false,None,Some(40),t),None);
        p.on_frame(&json!({"t":"pvp","fila":{"na":false}}),t);
        assert!(!p.owned_queue);
    }
    #[test]
    fn vip_inherits_server_setting_and_never_client_joins() {
        let t=Instant::now();
        let mut p=PvpAutomation::default();
        assert_eq!(p.next(None,true,Some(true),Some(30),t),Some(PvpAction::Info));
        assert!(p.status(None,true,Some(true),Some(30),t).enabled);
        assert_eq!(p.next(Some(false),true,Some(true),Some(30),t),
            Some(PvpAction::SetVipAuto(false)));
        assert_eq!(p.next(Some(false),true,Some(true),Some(30),t),None);
        assert_eq!(p.next(Some(true),true,Some(false),Some(30),t),
            Some(PvpAction::SetVipAuto(true)));
    }
    #[test]
    fn turning_off_vip_auto_queue_cancels_the_active_search_after_confirmation() {
        let t=Instant::now();
        let mut p=PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        assert_eq!(p.next(Some(false),true,Some(true),Some(30),t),
            Some(PvpAction::SetVipAuto(false)));
        assert_eq!(p.next(Some(false),true,Some(true),Some(30),t),None);
        assert_eq!(p.next(Some(false),true,Some(false),Some(30),t),
            Some(PvpAction::Leave));
        assert_eq!(p.next(Some(false),true,Some(false),Some(30),t),None);
    }
    #[test]
    fn late_info_does_not_erase_an_unconfirmed_join() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","time":[1],"regras":{"nivelMin":5},
            "fila":{"na":false}}), t);
        assert_eq!(p.next(Some(true),false,None,Some(30),t),Some(PvpAction::Join));
        // The earlier pvp.info is not evidence that this Join failed.
        p.on_frame(&json!({"t":"pvp","fila":{"na":false}}), t + Duration::from_secs(1));
        assert!(p.owned_queue);
        assert_eq!(p.next(Some(false),false,None,Some(30),
            t + Duration::from_secs(1)),None);
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),
            t + Duration::from_secs(2));
        assert_eq!(p.next(Some(false),false,None,Some(30),
            t + Duration::from_secs(2)),Some(PvpAction::Leave));
        assert_eq!(p.next(Some(false),false,None,Some(30),
            t + Duration::from_secs(3)),None);
        p.on_frame(&json!({"t":"pvp","fila":{"na":false}}),
            t + Duration::from_secs(4));
        assert!(!p.owned_queue);
    }
    #[test]
    fn missing_leave_ack_queries_server_before_retry() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","time":[1],"regras":{"nivelMin":1},
            "fila":{"na":false}}), t);
        assert_eq!(p.next(Some(true),false,None,Some(15),t),Some(PvpAction::Join));
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        assert_eq!(p.next(Some(false),false,None,Some(15),t),Some(PvpAction::Leave));
        assert_eq!(p.next(Some(false),false,None,Some(15),
            t + Duration::from_secs(5)),None);
        assert_eq!(p.next(Some(false),false,None,Some(15),
            t + Duration::from_secs(16)),Some(PvpAction::Info));
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),
            t + Duration::from_secs(17));
        assert_eq!(p.next(Some(false),false,None,Some(15),
            t + Duration::from_secs(17)),Some(PvpAction::Leave));
    }
    #[test]
    fn reconnect_checks_ownership_before_cancelling_and_preserves_manual_queue() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        p.on_reconnect();
        assert_eq!(p.next(Some(false),false,None,Some(40),t),None);
        p.on_frame(&json!({"t":"pvp","fila":{"na":false}}),t);
        p.on_frame(&json!({"t":"pvp","time":[1],"regras":{"nivelMin":1},
            "fila":{"na":false}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(40),t),Some(PvpAction::Join));
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        p.on_reconnect();
        assert_eq!(p.next(Some(false),false,None,Some(40),t),Some(PvpAction::Info));
        p.on_frame(&json!({"t":"pvp","fila":{"na":true}}),t);
        assert_eq!(p.next(Some(false),false,None,Some(40),t),Some(PvpAction::Leave));
    }
    #[test]
    fn server_vip_refusal_pauses_retries_until_user_rearms() {
        let t = Instant::now();
        let mut p = PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","filaAutoDesligada":true}),t);
        assert!(p.status(Some(true),true,Some(false),Some(25),t).paused);
        assert_eq!(p.next(Some(true),true,Some(false),Some(25),t),Some(PvpAction::Info));
        assert_eq!(p.next(Some(true),true,Some(false),Some(25),
            t + Duration::from_secs(30)),None);
        assert_eq!(p.next(Some(false),true,Some(false),Some(25),t),None);
        assert_eq!(p.next(Some(true),true,Some(false),Some(25),t),
            Some(PvpAction::SetVipAuto(true)));
        p.on_frame(&json!({"t":"pvp","filaAutoDesligada":true}),t);
        p.on_user_toggle(); // Off/on happened between polling ticks.
        assert_eq!(p.next(Some(true),true,Some(false),Some(25),t),
            Some(PvpAction::SetVipAuto(true)));
    }
    #[test]
    fn match_and_reconnect_require_fresh_server_status_before_requeue() {
        let t=Instant::now();
        let mut p=PvpAutomation::default();
        p.on_frame(&json!({"t":"pvp","time":[1],"regras":{"nivelMin":1},
            "fila":{"na":false}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(8),t),Some(PvpAction::Join));
        p.on_frame(&json!({"t":"pvp","partida":{"id":9}}),t);
        assert_eq!(p.next(Some(true),false,None,Some(8),t),Some(PvpAction::Info));
        p.on_reconnect();
        assert_eq!(p.next(Some(true),false,None,Some(8),t),Some(PvpAction::Info));
    }
}
