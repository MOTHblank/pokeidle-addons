//! Server-authoritative automation updates on the native game socket.
//! Never replace the server's entire automation object with a locally guessed
//! schema. Each mutation is serialized and confirmed by an estado patch.
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const CONFIRM_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_QUEUE: usize = 16;

#[derive(Clone, Debug, Default)]
pub struct AutomationStatus {
    pub pending_field: Option<String>,
    pub queued: usize,
    pub confirmed: u64,
    pub error: Option<String>,
}
struct Mutation { field: String, value: Value }
struct InFlight { mutation: Mutation, sent: Instant }
pub struct AutomationWriter {
    pub status: AutomationStatus,
    queue: VecDeque<Mutation>,
    in_flight: Option<InFlight>,
}
impl AutomationWriter {
    pub fn new() -> Self {
        Self { status: AutomationStatus::default(),
            queue: VecDeque::new(), in_flight: None }
    }
    pub fn valid(field: &str, value: &Value) -> bool {
        match field {
            "autoPotion" | "autoRevive" | "autoVendaLoot" |
            "autoLockShiny" | "autoLockNota9" | "autoLockP5" |
            "autoVoltarHunt" => value.is_boolean(),
            "hpLimiar" => value.as_f64().is_some_and(|v|
                v.is_finite() && (0.0..=100.0).contains(&v)),
            "autoLockNotaMin" => value.as_f64().is_some_and(|v|
                v.is_finite() && (0.0..=10.0).contains(&v)),
            "potionIds" | "ballIds" | "reviveIds" =>
                value.as_array().is_some_and(|array|
                    array.len() <= 20 && array.iter().all(|id|
                        id.as_u64().is_some_and(|n|n>0 && n<=1_000_000))),
            _ => false,
        }
    }
    pub fn queue(&mut self, field: String, value: Value) -> Result<(), String> {
        if !Self::valid(&field, &value) {
            return Err("This automation setting is not supported or the value is invalid".into());
        }
        if self.queue.len() >= MAX_QUEUE {
            return Err("Too many automation changes are pending. Try again shortly.".into());
        }
        // Subsequent adjustments of the same control are last-write-wins
        // until they are sent. Never cancel an already transmitted frame.
        self.queue.retain(|m|m.field != field);
        self.queue.push_back(Mutation { field, value });
        self.status.queued = self.queue.len();
        Ok(())
    }
    pub fn next(&mut self, current: Option<&Value>, now: Instant) -> Option<Map<String,Value>> {
        if self.in_flight.as_ref().is_some_and(|sent|
            now.saturating_duration_since(sent.sent) > CONFIRM_TIMEOUT) {
            let failed = self.in_flight.take().expect("checked");
            self.status.error = Some(format!(
                "Server did not confirm {} within 12 seconds", failed.mutation.field
            ));
            self.status.pending_field = None;
        }
        if self.in_flight.is_some() { return None; }
        let mut payload = current.and_then(Value::as_object)?.clone();
        let mutation = self.queue.pop_front()?;
        // A full server snapshot must exist before mutating a single field;
        // preserve undocumented fields unmodified.
        payload.insert(mutation.field.clone(), mutation.value.clone());
        self.status.pending_field = Some(mutation.field.clone());
        self.status.queued = self.queue.len();
        self.status.error = None;
        self.in_flight = Some(InFlight { mutation, sent: now });
        Some(payload)
    }
    pub fn observe(&mut self, automation: &Value) {
        let Some(current) = automation.as_object() else {return};
        if self.in_flight.as_ref().is_some_and(|sent|
            current.get(&sent.mutation.field) == Some(&sent.mutation.value)) {
            self.in_flight = None;
            self.status.pending_field = None;
            self.status.confirmed = self.status.confirmed.saturating_add(1);
            self.status.error = None;
        }
    }
    pub fn on_disconnect(&mut self) {
        // Unconfirmed writes must never be replayed after a socket reconnect.
        self.queue.clear();
        self.in_flight = None;
        self.status.pending_field = None;
        self.status.queued = 0;
        self.status.error = Some("Connection lost; automation changes not replayed".into());
    }
    pub fn send_failed(&mut self) {
        self.in_flight = None;
        self.status.pending_field = None;
        self.status.error = Some("Could not send automation update".into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preserves_unknown_server_fields_and_confirms_only_echo() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        writer.queue("autoPotion".into(),json!(true)).unwrap();
        let frame=writer.next(Some(&json!({
            "autoPotion":false,"mysteryFlag":{"retain":true}
        })),now).unwrap();
        assert_eq!(frame["mysteryFlag"],json!({"retain":true}));
        assert_eq!(frame["autoPotion"],json!(true));
        writer.observe(&json!({"autoPotion":false}));
        assert_eq!(writer.status.pending_field.as_deref(),Some("autoPotion"));
        writer.observe(&json!({"autoPotion":true}));
        assert_eq!(writer.status.confirmed,1);
        assert!(writer.status.pending_field.is_none());
    }
    #[test]
    fn serializes_writes_and_does_not_replay_after_disconnect() {
        let mut writer=AutomationWriter::new();let now=Instant::now();
        writer.queue("autoRevive".into(),json!(true)).unwrap();
        writer.queue("autoPotion".into(),json!(false)).unwrap();
        assert!(writer.next(None,now).is_none());
        let current=json!({"autoRevive":false,"autoPotion":true});
        assert!(writer.next(Some(&current),now).is_some());
        assert!(writer.next(Some(&current),now).is_none());
        writer.observe(&json!({"autoRevive":true}));
        assert!(writer.next(Some(&json!({"autoRevive":true,"autoPotion":true})),now).is_some());
        writer.on_disconnect();
        assert_eq!(writer.status.queued,0);
        assert!(writer.next(Some(&current),now).is_none());
    }
    #[test]
    fn invalid_values_are_not_transmitted_and_timeout_releases_queue() {
        let mut writer=AutomationWriter::new();let now=Instant::now();
        assert!(writer.queue("admin".into(),json!(true)).is_err());
        assert!(writer.queue("potionIds".into(),json!([-1])).is_err());
        assert!(writer.queue("hpLimiar".into(),json!(101)).is_err());
        writer.queue("hpLimiar".into(),json!(40)).unwrap();
        let state=json!({"hpLimiar":35});
        writer.next(Some(&state),now);
        assert!(writer.next(Some(&state),now+Duration::from_secs(13)).is_none());
        assert!(writer.status.error.as_deref().unwrap().contains("12 seconds"));
    }
}
