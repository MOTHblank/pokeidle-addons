//! Timed, per-account native hunt transitions. Only active while a selection
//! is pending; the game socket's existing 60 ms wake drives retries.
use crate::native_gameplay::GameCommand;
use crate::native_hunts::HuntView;
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SELECT_RETRY: Duration = Duration::from_millis(350);
const HEAL_RETRY: Duration = Duration::from_millis(2500);
const SURRENDER_RETRY: Duration = Duration::from_secs(6);
const STUCK_FOR: Duration = Duration::from_secs(14);

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

#[derive(Default)]
pub struct HuntNavigator {
    gate: SwitchGate,
    target: Option<String>,
    started: Option<Instant>,
    last_select: Option<Instant>,
    surrendered: Option<Instant>,
    last_heal: Option<Instant>,
    healed: bool,
}
impl HuntNavigator {
    pub fn select(&mut self, slug: String, now: Instant) {
        if self.target.as_deref() == Some(slug.as_str()) { return; }
        self.target = Some(slug);
        self.started = Some(now);
        self.last_select = None;
        // If a previous destination triggered surrender, finish healing before
        // switching to the newly requested hunt instead of abandoning recovery.
        if self.surrendered.is_none() {
            self.last_heal = None;
            self.healed = false;
        }
    }
    pub fn cancel(&mut self) {
        self.target = None;
        self.started = None;
        self.last_select = None;
        self.surrendered = None;
        self.last_heal = None;
        self.healed = false;
    }
    pub fn observe(&mut self, frame: &Value, view: &HuntView, now: Instant) {
        self.gate.observe(frame, now);
        if self.surrendered.is_some() {
            if let Some(events) = frame.get("ev").and_then(Value::as_array) {
                for event in events {
                    let kind = event.get("k").and_then(Value::as_str);
                    if kind == Some("curado") ||
                        (kind == Some("joy") && matches!(
                            event.get("fala").and_then(Value::as_str),
                            Some("jaCurado" | "curou"))) {
                        self.healed = true;
                    }
                }
            }
        }
        if !view.in_center && self.target.as_deref() == view.hunt_slug.as_deref() {
            self.cancel();
        }
    }
    pub fn next_action(&mut self, view: &HuntView, now: Instant) -> Option<GameCommand> {
        let target = self.target.as_ref()?;
        if !view.catalog_ready || !view.hunts.iter().any(|hunt|hunt.slug == *target) {
            self.cancel();
            return None;
        }
        if !view.in_center && view.hunt_slug.as_deref() == Some(target) {
            self.cancel();
            return None;
        }

        if let Some(surrendered) = self.surrendered {
            if !view.in_center {
                if now.saturating_duration_since(surrendered) >= SURRENDER_RETRY {
                    self.surrendered = Some(now);
                    return Some(GameCommand::GiveUp);
                }
                return None;
            }
            if !self.healed {
                if self.last_heal.is_none_or(|sent|
                    now.saturating_duration_since(sent) >= HEAL_RETRY) {
                    self.last_heal = Some(now);
                    return Some(GameCommand::Heal);
                }
                return None;
            }
            self.surrendered = None;
        } else if !view.in_center && view.hunt_slug.is_some() {
            let (entry, combat) = self.gate.remaining(now);
            if entry > 0 || combat > 0 {
                let ordinary = view.hunt_slug.as_deref().and_then(|slug|
                    view.hunts.iter().find(|hunt|hunt.slug == slug))
                    .is_some_and(|hunt| hunt.species.len() == 1 &&
                        !matches!(hunt.slug.as_str(), "safari" | "arena_mistica"));
                if entry == 0 && combat > 350 && ordinary &&
                    self.started.is_some_and(|at|
                        now.saturating_duration_since(at) >= STUCK_FOR) &&
                    self.gate.last_attack.is_some_and(|at|
                        now.saturating_duration_since(at) < Duration::from_secs(2)) {
                    self.surrendered = Some(now);
                    self.last_heal = None;
                    self.healed = false;
                    return Some(GameCommand::GiveUp);
                }
                return None;
            }
        }

        if self.last_select.is_some_and(|sent|
            now.saturating_duration_since(sent) < SELECT_RETRY) {
            return None;
        }
        self.last_select = Some(now);
        Some(GameCommand::Hunt(target.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_hunts::{HuntEntry, HuntSpecies};
    use serde_json::json;
    use std::sync::Arc;

    fn view() -> HuntView {
        HuntView {
            catalog_ready: true,
            hunt_slug: Some("old".into()),
            hunts: Arc::new(vec![
                HuntEntry { slug: "old".into(), species: vec![HuntSpecies {
                    id: 1, weight: 1.0
                }], ..Default::default() },
                HuntEntry { slug: "new".into(), species: vec![HuntSpecies {
                    id: 2, weight: 1.0
                }], ..Default::default() },
            ]),
            ..Default::default()
        }
    }
    fn kind(action: Option<GameCommand>) -> &'static str {
        match action {
            Some(GameCommand::Hunt(_)) => "hunt",
            Some(GameCommand::GiveUp) => "giveup",
            Some(GameCommand::Heal) => "heal",
            _ => "none",
        }
    }
    #[test]
    fn attempts_exactly_when_server_combat_timer_expires() {
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
    fn entry_lock_is_respected_and_new_wave_opens_combat() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let mut state = view();
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1008000,
            "trocaHuntLivreEm":1001000,"ondaSeq":4,"combateOndaSeq":3
        }}), &state, at);
        nav.select("new".into(), at);
        assert_eq!(kind(nav.next_action(&state, at)), "none");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(1))), "hunt");
        state.hunt_slug = Some("new".into());
        nav.observe(&json!({"t":"batalha","ev":[{"k":"hunt","slug":"new"}]}),
            &state, at + Duration::from_secs(1));
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(2))), "none");
    }
    #[test]
    fn repeated_attacks_trigger_surrender_then_confirmed_heal() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        let mut state = view();
        nav.select("new".into(), at);
        nav.observe(&json!({"t":"estado","estado":{
            "servidorAgora":1000000,"centroLivreEm":1020000,
            "trocaHuntLivreEm":1000000}}), &state, at);
        nav.observe(&json!({"t":"batalha","ev":[{"k":"ataque"}]}),
            &state, at + Duration::from_secs(14));
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(14))), "giveup");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(15))), "none");
        state.in_center = true;
        nav.observe(&json!({"t":"estado","estado":{"noCentro":true}}),
            &state, at + Duration::from_secs(15));
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(15))), "heal");
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(16))), "none");
        nav.observe(&json!({"t":"batalha","ev":[{"k":"curado"}]}),
            &state, at + Duration::from_secs(16));
        assert_eq!(kind(nav.next_action(&state, at + Duration::from_secs(16))), "hunt");
    }
    #[test]
    fn manual_cancellation_never_surrenders_or_travels() {
        let at = Instant::now();
        let mut nav = HuntNavigator::default();
        nav.select("new".into(), at);
        nav.cancel();
        assert_eq!(kind(nav.next_action(&view(), at + Duration::from_secs(20))), "none");
    }
}
