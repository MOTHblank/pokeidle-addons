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
struct Mutation { field: String, value: Value, from_restock: bool }
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
            "autoVoltarHunt" | "pvpAutoFila" => value.is_boolean(),
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
        self.queue.push_back(Mutation { field, value, from_restock:false });
        self.status.queued = self.queue.len();
        Ok(())
    }
    /// Mirror the selected restock items into the game's own healing-item
    /// priorities. Each preference is independent; never alter autoPotion,
    /// autoRevive, hpLimiar, or any other server-owned automation setting.
    ///
    /// If restock is disabled or the corresponding selector is Off, relinquish
    /// control of that field without clearing the game's existing choice.
    pub fn sync_restock_potion(&mut self, potion_id: Option<u64>, current: Option<&Value>) {
        self.sync_restock_item("potionIds",potion_id,current);
    }
    pub fn sync_restock_revive(&mut self, revive_id: Option<u64>, current: Option<&Value>) {
        self.sync_restock_item("reviveIds",revive_id,current);
    }
    fn sync_restock_item(&mut self, field:&'static str, item_id:Option<u64>,
        current:Option<&Value>) {
        debug_assert!(field=="potionIds" || field=="reviveIds");
        let target=item_id.filter(|id|*id>0 && *id<=1_000_000)
            .map(|id|Value::Array(vec![Value::from(id)]));
        let Some(target)=target else {
            // Only discard unsent restock writes for this selector.
            // A manual change and the other healing selector are untouched.
            self.queue.retain(|mutation|
                mutation.field!=field || !mutation.from_restock);
            self.status.queued=self.queue.len();
            return;
        };
        let Some(server)=current.and_then(Value::as_object) else{return};
        if server.get(field)==Some(&target) {
            // Already confirmed on the game; do not send redundant writes.
            self.queue.retain(|mutation|mutation.field!=field);
            self.status.queued=self.queue.len();
            return;
        }
        if self.in_flight.as_ref().is_some_and(|sent|
            sent.mutation.field==field && sent.mutation.value==target) {
            return;
        }
        if self.queue.iter().any(|queued|
            queued.field==field && queued.value==target) {
            return;
        }
        // Coalesce only this field's unsent changes; keep other mutations,
        // including edits to the game's own enable switches.
        self.queue.retain(|mutation|mutation.field!=field);
        if self.queue.len()>=MAX_QUEUE { return; }
        self.queue.push_back(Mutation {
            field:field.into(),value:target,from_restock:true
        });
        self.status.queued=self.queue.len();
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
    fn restocked_potion_updates_only_potion_ids_and_confirms_game_echo() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        let server=json!({"autoPotion":false,"hpLimiar":0.7,
            "reviveIds":[205],"potionIds":[200,201],"unknownSetting":99});
        writer.sync_restock_potion(Some(203),Some(&server));
        writer.sync_restock_potion(Some(203),Some(&server));
        assert_eq!(writer.status.queued,1);
        let sent=writer.next(Some(&server),now).unwrap();
        assert_eq!(sent["potionIds"],json!([203]));
        assert_eq!(sent["autoPotion"],json!(false));
        assert_eq!(sent["hpLimiar"],json!(0.7));
        assert_eq!(sent["reviveIds"],json!([205]));
        assert_eq!(sent["unknownSetting"],json!(99));
        writer.sync_restock_potion(Some(203),Some(&server));
        assert_eq!(writer.status.queued,0);
        writer.observe(&json!({"potionIds":[200,201]}));
        assert_eq!(writer.status.pending_field.as_deref(),Some("potionIds"));
        writer.observe(&json!({"potionIds":[203]}));
        assert_eq!(writer.status.confirmed,1);
        writer.sync_restock_potion(Some(203),Some(&json!({"potionIds":[203]})));
        assert!(writer.next(Some(&server),now).is_none());
    }
    #[test]
    fn restock_changes_replace_unsent_choice_and_off_keeps_game_healing() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        let server=json!({"autoPotion":true,"potionIds":[201],"autoRevive":true});
        writer.sync_restock_potion(Some(200),Some(&server));
        writer.sync_restock_potion(Some(204),Some(&server));
        assert_eq!(writer.status.queued,1);
        let sent=writer.next(Some(&server),now).unwrap();
        assert_eq!(sent["potionIds"],json!([204]));
        assert_eq!(sent["autoPotion"],json!(true));
        writer.observe(&json!({"potionIds":[204]}));
        // Disabling potion restock must not send potionIds=[] or autoPotion=false.
        writer.sync_restock_potion(None,Some(&server));
        assert!(writer.next(Some(&server),now).is_none());
        writer.sync_restock_potion(Some(200),Some(&server));
        assert_eq!(writer.status.queued,1);
        writer.sync_restock_potion(None,Some(&server));
        assert_eq!(writer.status.queued,0);
        assert!(writer.next(Some(&server),now).is_none());
    }
    #[test]
    fn inactive_restock_does_not_drop_manual_potion_id_edits() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        writer.queue("potionIds".into(),json!([201])).unwrap();
        writer.sync_restock_potion(None,Some(&json!({"potionIds":[200]})));
        assert_eq!(writer.status.queued,1);
        let frame=writer.next(Some(&json!({"potionIds":[200]})),now).unwrap();
        assert_eq!(frame["potionIds"],json!([201]));
    }
    #[test]
    fn sync_waits_for_authoritative_automation_and_serializes_other_edits() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        writer.sync_restock_potion(Some(200),None);
        assert_eq!(writer.status.queued,0);
        let server=json!({"autoPotion":false,"potionIds":[201]});
        writer.queue("autoPotion".into(),json!(true)).unwrap();
        writer.sync_restock_potion(Some(200),Some(&server));
        let first=writer.next(Some(&server),now).unwrap();
        assert_eq!(first["autoPotion"],json!(true));
        assert_eq!(first["potionIds"],json!([201]));
        assert!(writer.next(Some(&server),now).is_none());
        writer.observe(&json!({"autoPotion":true,"potionIds":[201]}));
        let second=writer.next(Some(&json!({"autoPotion":true,"potionIds":[201]})),now).unwrap();
        assert_eq!(second["autoPotion"],json!(true));
        assert_eq!(second["potionIds"],json!([200]));
        writer.on_disconnect();
        assert_eq!(writer.status.queued,0);
        assert_eq!(writer.status.pending_field,None);
    }
    #[test]
    fn revive_restock_tracks_selected_revive_without_changing_automation_switches() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        let server=json!({"autoPotion":false,"autoRevive":false,"hpLimiar":0.8,
            "potionIds":[204],"reviveIds":[205,206],"unknownSetting":"safe"});
        writer.sync_restock_revive(Some(206),Some(&server));
        writer.sync_restock_revive(Some(206),Some(&server));
        assert_eq!(writer.status.queued,1);
        let sent=writer.next(Some(&server),now).unwrap();
        assert_eq!(sent["reviveIds"],json!([206]));
        assert_eq!(sent["potionIds"],json!([204]));
        assert_eq!(sent["autoPotion"],json!(false));
        assert_eq!(sent["autoRevive"],json!(false));
        assert_eq!(sent["hpLimiar"],json!(0.8));
        assert_eq!(sent["unknownSetting"],json!("safe"));
        writer.observe(&json!({"reviveIds":[205]}));
        assert_eq!(writer.status.pending_field.as_deref(),Some("reviveIds"));
        writer.observe(&json!({"reviveIds":[206]}));
        assert_eq!(writer.status.confirmed,1);
        writer.sync_restock_revive(Some(206),Some(&json!({"reviveIds":[206]})));
        assert_eq!(writer.status.queued,0);
    }
    #[test]
    fn potion_and_revive_restock_sync_independently_in_order() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        let original=json!({"autoPotion":true,"autoRevive":true,
            "potionIds":[200],"reviveIds":[205],"hpLimiar":0.3});
        writer.sync_restock_potion(Some(202),Some(&original));
        writer.sync_restock_revive(Some(206),Some(&original));
        assert_eq!(writer.status.queued,2);
        let first=writer.next(Some(&original),now).unwrap();
        assert_eq!(first["potionIds"],json!([202]));
        assert_eq!(first["reviveIds"],json!([205]));
        assert_eq!(first["autoRevive"],json!(true));
        assert!(writer.next(Some(&original),now).is_none());
        writer.observe(&json!({"potionIds":[202]}));
        let updated=json!({"autoPotion":true,"autoRevive":true,
            "potionIds":[202],"reviveIds":[205],"hpLimiar":0.3});
        let second=writer.next(Some(&updated),now).unwrap();
        assert_eq!(second["potionIds"],json!([202]));
        assert_eq!(second["reviveIds"],json!([206]));
        writer.observe(&json!({"reviveIds":[206]}));
        assert_eq!(writer.status.queued,0);
        assert_eq!(writer.status.confirmed,2);
    }
    #[test]
    fn disabling_either_restock_selection_preserves_other_pending_sync() {
        let mut writer=AutomationWriter::new();
        let now=Instant::now();
        let server=json!({"potionIds":[200],"reviveIds":[205],
            "autoPotion":true,"autoRevive":true});
        writer.sync_restock_potion(Some(204),Some(&server));
        writer.sync_restock_revive(Some(206),Some(&server));
        writer.sync_restock_revive(None,Some(&server));
        assert_eq!(writer.status.queued,1);
        assert_eq!(writer.next(Some(&server),now).unwrap()["potionIds"],json!([204]));
        writer.observe(&json!({"potionIds":[204]}));
        assert!(writer.next(Some(&server),now).is_none());
        writer.sync_restock_revive(Some(206),Some(&server));
        writer.sync_restock_potion(None,Some(&server));
        assert_eq!(writer.status.queued,1);
        let revive=writer.next(Some(&server),now).unwrap();
        assert_eq!(revive["reviveIds"],json!([206]));
        assert_eq!(revive["autoRevive"],json!(true));
    }
    #[test]
    fn new_revive_preference_coalesces_without_affecting_potion_preference() {
        let mut writer=AutomationWriter::new();
        let server=json!({"potionIds":[200],"reviveIds":[205]});
        writer.sync_restock_revive(Some(206),Some(&server));
        writer.sync_restock_potion(Some(203),Some(&server));
        writer.sync_restock_revive(Some(205),Some(&server));
        assert_eq!(writer.status.queued,1);
        let sent=writer.next(Some(&server),Instant::now()).unwrap();
        assert_eq!(sent["potionIds"],json!([203]));
        assert_eq!(sent["reviveIds"],json!([205]));
        writer.on_disconnect();
        assert_eq!(writer.status.queued,0);
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
