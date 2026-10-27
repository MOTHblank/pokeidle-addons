//! Per-account travel commands wait for server-confirmed switch openings.
//! No automatic surrender, healing, or other destructive navigation.
use crate::native_gameplay::GameCommand;
use crate::native_hunts::HuntView;
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SELECT_RETRY: Duration = Duration::from_millis(350);
const STUCK_FOR: Duration = Duration::from_secs(14);

/// A warning, not permission to surrender. The decision belongs to the player.
pub const BLOCKED_NOTICE: &str =
    "Combat keeps blocking travel. This opponent may be too strong to leave without giving up. Keep waiting for an opening, or choose Pokémon Center (Give Up) manually. Give Up KOs your Pokémon and may cost XP.";

#[derive(Default)]
struct SwitchGate {
    server_clock: Option<(Instant, u64)>,
    entry_until: u64,
    combat_until: u64,
    wave: u64,
    combat_wave: u64,
    last_attack: Option<Instant>,
}
impl SwitchGate {
    fn number(value: &Value, key: &str) -> Option<u64> {
        value.get(key).and_then(Value::as_u64)
    }
    fn server_ms(&self, now: Instant) -> u64 {
        self.server_clock.map(|(at, ms)| {
            ms.saturating_add(now.saturating_duration_since(at).as_millis() as u64)
        }).unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_millis() as u64)
    }
    fn observe(&mut self, frame: &Value, now: Instant) {
        if let Some(state) = frame.get("estado") {
            if let Some(ms) = Self::number(state, "servidorAgora") {
                let ms = if (1_000_000_000..10_000_000_000).contains(&ms) {
                    ms.saturating_mul(1000)
                } else { ms };
                self.server_clock = Some((now, ms));
            }
            if let Some(until) = Self::number(state, "centroLivreEm") {
                self.combat_until = until;
            }
            if let Some(until) = Self::number(state, "trocaHuntLivreEm")
                .or_else(|| Self::number(state, "trocaLivreEm")) {
                self.entry_until = until;
            }
            if let Some(wave) = Self::number(state, "ondaSeq") { self.wave = wave; }
            if let Some(wave) = Self::number(state, "combateOndaSeq") {
                self.combat_wave = wave;
            }
        }
        if let Some(events) = frame.get("ev").and_then(Value::as_array) {
            for event in events {
                match event.get("k").and_then(Value::as_str) {
                    Some("ataque") => {
                        self.last_attack = Some(now);
                        self.combat_until = self.server_ms(now).saturating_add(3000);
                        self.combat_wave = self.wave;
                    }
                    Some("hunt") => {
                        self.entry_until = Self::number(event, "trocaLivreEm")
                            .unwrap_or(0);
                        self.combat_until = 0;
                        self.wave = 0;
                        self.combat_wave = 0;
                        self.last_attack = None;
                    }
                    Some("centro") => { self.combat_until = 0; }
                    _ => {}
                }
            }
        }
    }
    fn remaining(&self, now: Instant) -> (u64, u64) {
        let ms = self.server_ms(now);
        let entry = self.entry_until.saturating_sub(ms);
        let wave_open = self.wave > 0 && self.wave > self.combat_wave;
        let combat = if wave_open { 0 } else {
            self.combat_until.saturating_sub(ms)
        };
        (entry, combat)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Destination {
    Hunt(String),
    Center,
}

#[derive(Default)]
pub struct HuntNavigator {
    gate: SwitchGate,
    target: Option<Destination>,
    started: Option<Instant>,
    last_select: Option<Instant>,
}
impl HuntNavigator {
    pub fn select(&mut self, slug: String, now: Instant) {
        let destination = Destination::Hunt(slug);
        if self.target.as_ref() == Some(&destination) { return; }
        self.target = Some(destination);
        self.started = Some(now);
        self.last_select = None;
    }
    pub fn center(&mut self, now: Instant) {
        if self.target == Some(Destination::Center) { return; }
        self.target = Some(Destination::Center);
        self.started = Some(now);
        self.last_select = None;
    }
    pub fn center_pending(&self) -> bool {
        self.target == Some(Destination::Center)
    }
    pub fn cancel(&mut self) {
        self.target = None;
        self.started = None;
        self.last_select = None;
    }
    pub fn observe(&mut self, frame: &Value, view: &HuntView, now: Instant) {
        self.gate.observe(frame, now);
        let arrived = match self.target.as_ref() {
            Some(Destination::Hunt(slug)) =>
                !view.in_center && view.hunt_slug.as_deref() == Some(slug.as_str()),
            Some(Destination::Center) => view.in_center,
            None => false,
        };
        if arrived { self.cancel(); }
    }
    /// Warn only if attacks are continuously preventing travel after the
    /// entry lock is already clear. Never turn this warning into Give Up.
    pub fn blocked(&self, view: &HuntView, now: Instant) -> bool {
        if self.target.is_none() || view.in_center ||
            !self.started.is_some_and(|at|
                now.saturating_duration_since(at) >= STUCK_FOR) {
            return false;
        }
        let (entry, combat) = self.gate.remaining(now);
        entry == 0 && combat > 350 &&
            self.gate.last_attack.is_some_and(|at|
                now.saturating_duration_since(at) < Duration::from_secs(2))
    }
    pub fn next_action(&mut self, view: &HuntView, now: Instant) -> Option<GameCommand> {
        let target = self.target.clone()?;
        match &target {
            Destination::Hunt(slug) => {
                if !view.catalog_ready ||
                    !view.hunts.iter().any(|hunt| hunt.slug == slug.as_str()) {
                    self.cancel();
                    return None;
                }
                if !view.in_center && view.hunt_slug.as_deref() == Some(slug.as_str()) {
                    self.cancel();
                    return None;
                }
            }
            Destination::Center if view.in_center => {
                self.cancel();
                return None;
            }
            Destination::Center => {}
        }

        if !view.in_center && view.hunt_slug.is_some() {
            let (entry, combat) = self.gate.remaining(now);
            if entry > 0 || combat > 0 { return None; }
        }
        if self.last_select.is_some_and(|sent|
            now.saturating_duration_since(sent) < SELECT_RETRY) {
            return None;
        }
        self.last_select = Some(now);
        Some(match target {
            Destination::Hunt(slug) => GameCommand::Hunt(slug),
            Destination::Center => GameCommand::Center,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_hunts::HuntEntry;
    use serde_json::json;
    use std::sync::Arc;
    fn view() -> HuntView {
        HuntView {
            catalog_ready: true,
            hunt_slug: Some("old".into()),
            hunts: Arc::new(vec![
                HuntEntry { slug: "old".into(), ..Default::default() },
                HuntEntry { slug: "new".into(), ..Default::default() },
            ]),
            ..Default::default()
        }
    }
    fn kind(action: Option<GameCommand>) -> &'static str {
        match action {
            Some(GameCommand::Hunt(_)) => "hunt",
            Some(GameCommand::Center) => "center",
            Some(GameCommand::GiveUp) => "giveup",
            _ => "none",
        }
    }
    #[test]
    fn hunt_select_waits_until_server_combat_timer_expires() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let state = view();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1003000,
            "trocaHuntLivreEm":1000000}}), &state, at);
        nav.select("new".into(), at);
        assert_eq!(kind(nav.next_action(&state, at)), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(2999))), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(3000))), "hunt");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(3100))), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(3350))), "hunt");
    }
    #[test]
    fn normal_center_uses_same_gate_and_retry_as_hunt() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let mut state = view();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1003000,
            "trocaHuntLivreEm":1001000}}), &state, at);
        nav.center(at);
        assert!(nav.center_pending());
        assert_eq!(kind(nav.next_action(&state, at)), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(3))), "center");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(3100))), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_millis(3350))), "center");
        state.in_center = true;
        nav.observe(&json!({"t":"estado","estado":{"noCentro":true}}),
            &state, at + Duration::from_secs(4));
        assert!(!nav.center_pending());
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(4))), "none");
    }
    #[test]
    fn new_wave_opens_combat_but_does_not_bypass_entry_lock() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let state = view();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1008000,
            "trocaHuntLivreEm":1001000,"ondaSeq":4,"combateOndaSeq":3
        }}), &state, at);
        nav.center(at);
        assert_eq!(kind(nav.next_action(&state, at)), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(1))), "center");
    }
    #[test]
    fn permanently_blocked_combat_never_surrenders_or_heals() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let state = view();
        nav.select("new".into(), at);
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1030000,
            "trocaHuntLivreEm":1000000}}), &state, at);
        for seconds in 14..60 {
            let now = at + Duration::from_secs(seconds);
            nav.observe(&json!({"t":"batalha","ev":[{"k":"ataque"}]}), &state, now);
            assert!(nav.blocked(&state, now));
            assert_eq!(kind(nav.next_action(&state, now)), "none");
        }
        assert_eq!(kind(nav.next_action(&state, at+Duration::from_secs(120))), "hunt");
    }
    #[test]
    fn center_and_hunt_requests_replace_each_other() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let mut state = view();
        nav.select("new".into(), at);
        nav.center(at);
        assert_eq!(kind(nav.next_action(&state, at)), "center");
        nav.select("new".into(), at);
        assert!(!nav.center_pending());
        assert_eq!(kind(nav.next_action(&state, at)), "hunt");
        state.in_center = true;
        nav.observe(&json!({"t":"estado","estado":{"noCentro":true}}), &state, at);
        assert_eq!(kind(nav.next_action(&state, at+Duration::from_millis(350))), "hunt");
        nav.cancel();
        assert_eq!(kind(nav.next_action(&state, at+Duration::from_secs(20))), "none");
    }
}
