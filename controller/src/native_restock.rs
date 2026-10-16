//! Native Auto Catch restocking without browser/user-script execution.
//! All prices, funds and inventory counts are server-confirmed. At most one
//! shop.buy can be in flight per account. Purchase acknowledgments must match
//! the pending order; unrelated purchases cannot unblock the worker.
use crate::native_gameplay::GameCommand;
use crate::native_capture_config::CatchSettings;
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const MAX_QTY: u64 = 5000;
const ACK_TIMEOUT: Duration = Duration::from_secs(10);
const SETTLE: Duration = Duration::from_secs(2);
const RETRY_BACKOFF: Duration = Duration::from_secs(30);
const ACK_STOCK_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Default)]
pub struct RestockStatus {
    pub enabled: bool,
    pub configured: bool,
    pub phase: String,
    pub ordered: u64,
    pub confirmed: u64,
    pub pending: bool,
    pub last_order: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
struct Rule {
    kind: &'static str,
    id: u64,
    low: u64,
    target: u64,
}
#[derive(Clone, Debug)]
struct ShopEntry { price: u64, name: String }
#[derive(Clone, Debug)]
struct Order {
    kind: &'static str,
    id: u64,
    name: String,
    qty: u64,
    price: u64,
    before: u64,
    sent: Instant,
    ack: bool,
}

pub struct Restock {
    pub status: RestockStatus,
    rules: Vec<Rule>,
    reserve: u64,
    prices: BTreeMap<(&'static str, u64), ShopEntry>,
    stock: BTreeMap<(&'static str, u64), u64>,
    known_stock: [bool; 2],
    gold: Option<u64>,
    pending: Option<Order>,
    settle_until: Option<Instant>,
    blocked_until: Option<Instant>,
}
impl Restock {
    pub fn new(settings: Option<&Value>) -> Self {
        let config = settings.and_then(|v| v.get("restock"));
        // The userscript merged these defaults when reading its legacy v4
        // settings; older saved JSON may contain only an "enabled" flag.
        // Import the same preferences without requiring that userscript.
        let num = |field: &str, fallback: u64| config.and_then(|v| v.get(field))
            .and_then(Value::as_u64).unwrap_or(fallback);
        let mut rules = Vec::new();
        for (kind, id_field, low_field, target_field, default_id, default_low, default_to) in [
            ("ball", "selectedBallId", "ballAt", "ballTo", 1, 50, 500),
            ("item", "selectedPotionId", "potionAt", "potionTo", 200, 10, 50),
            ("item", "selectedReviveId", "reviveAt", "reviveTo", 0, 1, 5),
        ] {
            let id = num(id_field, default_id);
            let low = num(low_field, default_low);
            let target = num(target_field, default_to);
            if id > 0 && target > low && target <= 1_000_000 {
                rules.push(Rule { kind, id, low, target });
            }
        }
        let enabled = config.and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool) == Some(true);
        let configured = !rules.is_empty();
        Self {
            status: RestockStatus {
                enabled, configured,
                phase: if !enabled { "Off" } else if !configured {
                    "Needs settings"
                } else { "Waiting for inventory" }.into(),
                ..Default::default()
            },
            rules, reserve: num("goldReserve", 0),
            prices: BTreeMap::new(), stock: BTreeMap::new(),
            known_stock: [false, false], gold: None, pending: None,
            settle_until: None, blocked_until: None,
        }
    }
    /// Apply settings while maintaining pending transaction and authoritative
    /// shop/inventory state. Replacing the Restock worker would lose in-flight
    /// purchase identity and could trigger a duplicate order.
    pub fn configure(&mut self, config: &CatchSettings) {
        let candidate=Self::new(Some(&config.as_value()));
        self.rules=candidate.rules;
        self.reserve=candidate.reserve;
        self.status.configured=candidate.status.configured;
        self.set_enabled(config.restock_enabled);
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        // Called each native worker tick. A repeated value must not erase
        // waiting/confirmed/backoff phase and mislead the UI.
        if self.status.enabled == enabled { return; }
        self.status.enabled = enabled;
        self.status.phase = if !enabled { "Off" }
            else if !self.status.configured { "Needs settings" }
            else { "Waiting for inventory" }.into();
        // Keep a previously sent transaction pending until it is reconciled.
        self.status.pending = self.pending.is_some();
    }
    fn idx(kind: &str) -> usize { if kind == "ball" { 0 } else { 1 } }
    fn read_catalog(&mut self, kind: &'static str, raw: Option<&Value>) {
        let Some(items) = raw.and_then(Value::as_array) else { return };
        for item in items {
            if item.get("compravel").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            // Pokéballs explicitly advertise buyability in the welcome.
            if kind == "ball" && item.get("compravel").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let Some(id) = item.get("itemId").or_else(|| item.get("id"))
                .and_then(Value::as_u64).filter(|id| *id > 0) else { continue };
            let Some(price) = item.get("priceGold").or_else(|| item.get("preco"))
                .or_else(|| item.get("npcPrice"))
                .and_then(Value::as_u64).filter(|p| *p > 0) else { continue };
            let name = item.get("nome").or_else(|| item.get("name"))
                .and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
            // For items, only potion and revive entries may be auto-purchased.
            // A misconfigured ID must never buy arbitrary marketplace items.
            if kind == "item" && !(name.contains("potion") ||
                name.contains("pocao") || name.contains("poção") ||
                name.contains("revive")) { continue; }
            self.prices.insert((kind, id), ShopEntry { price, name });
        }
    }
    pub fn on_disconnect(&mut self, now: Instant) {
        if self.pending.take().is_some() {
            // The server might have accepted a purchase immediately before
            // disconnect. Never retry it until a fresh welcome and cool-down.
            self.blocked_until = Some(now + RETRY_BACKOFF);
            self.status.phase = "Rechecking last order".into();
        } else {
            self.status.phase = "Reconnecting".into();
        }
        self.prices.clear();
        self.stock.clear();
        self.gold = None;
        self.known_stock = [false, false];
        self.status.pending = false;
    }
    pub fn ingest(&mut self, raw: &str, now: Instant) {
        let Ok(frame) = serde_json::from_str::<Value>(raw) else { return };
        if frame.get("t").and_then(Value::as_str) == Some("welcome") {
            if self.pending.is_some() {
                self.on_disconnect(now);
            }
            self.prices.clear();
            self.stock.clear();
            self.gold = None;
            self.known_stock = [false, false];
            self.read_catalog("ball", frame.get("catalogoBolas"));
            self.read_catalog("item", frame.get("itensCompraveis"));
        }
        // Match purchase results by ID, kind, name, or exact quantity+spend.
        // It is common for the server to omit ID or kind in purchase events.
        if let Some(events) = frame.get("ev").and_then(Value::as_array) {
            for event in events {
                if event.get("k").and_then(Value::as_str) != Some("compra") {
                    continue;
                }
                if let Some(order) = &mut self.pending {
                    let event_id = event.get("id").or_else(|| event.get("itemId"))
                        .or_else(|| event.get("ballId")).and_then(Value::as_u64);
                    let event_kind = event.get("kind").and_then(Value::as_str);
                    let event_name = event.get("nome").or_else(|| event.get("name"))
                        .and_then(Value::as_str).map(|s| s.trim().to_lowercase());
                    // Explicit mismatching information must never acknowledge.
                    if event_id.is_some_and(|id| id != order.id) ||
                        event_kind.is_some_and(|kind| kind != order.kind) {
                        continue;
                    }
                    let id_matches = event_id == Some(order.id);
                    let name_matches = event_name.as_deref() == Some(order.name.as_str())
                        && !order.name.is_empty();
                    let qty_matches = event.get("qtd").and_then(Value::as_u64) == Some(order.qty);
                    let spent_matches = match (
                        event.get("gasto").and_then(Value::as_u64),
                        order.qty.checked_mul(order.price)
                    ) {
                        (Some(actual), Some(expected)) => actual == expected,
                        _ => false
                    };
                    if id_matches || name_matches || (qty_matches && spent_matches) {
                        order.ack = true;
                        self.status.phase = "Confirming inventory".into();
                    }
                }
            }
        }
        if let Some(state) = frame.get("estado") {
            if let Some(gold) = state.get("gold").and_then(Value::as_u64) {
                self.gold = Some(gold);
            }
            for (kind, field) in [("ball", "balls"), ("item", "items")] {
                if let Some(map) = state.get(field).and_then(Value::as_object) {
                    self.stock.retain(|(stored_kind, _), _| *stored_kind != kind);
                    for (id, count) in map {
                        if let (Ok(id), Some(count)) =
                            (id.parse::<u64>(), count.as_u64()) {
                            self.stock.insert((kind, id), count);
                        }
                    }
                    self.known_stock[Self::idx(kind)] = true;
                    if let Some(order) = &self.pending {
                        if order.kind == kind {
                            let current = self.stock.get(&(kind, order.id))
                                .copied().unwrap_or(0);
                            // A purchase's stock delta is sufficient evidence
                            // even if its event arrives late or gets lost.
                            if current > order.before {
                                self.confirm(now);
                            }
                        }
                    }
                }
            }
        }
        self.status.pending = self.pending.is_some();
    }
    fn confirm(&mut self, now: Instant) {
        if let Some(order) = self.pending.take() {
            self.status.confirmed = self.status.confirmed.saturating_add(1);
            self.status.last_order = Some(format!("{} ×{}", order.name, order.qty));
            self.status.last_error = None;
            self.status.phase = "Stock synced".into();
            self.settle_until = Some(now + SETTLE);
            self.status.pending = false;
        }
    }
    pub fn next(&mut self, now: Instant, catch_enabled: bool) -> Option<GameCommand> {
        if !catch_enabled || !self.status.enabled {
            self.status.phase = "Off".into();
            return None;
        }
        if !self.status.configured {
            self.status.phase = "Needs settings".into();
            return None;
        }
        if let Some(order) = &self.pending {
            let age = now.saturating_duration_since(order.sent);
            if order.ack && age < ACK_STOCK_TIMEOUT {
                self.status.phase = "Confirming inventory".into();
                return None;
            }
            if !order.ack && age < ACK_TIMEOUT {
                self.status.phase = "Waiting for purchase".into();
                return None;
            }
            let reason = if order.ack {
                "Purchase confirmed; inventory did not update"
            } else {
                "Purchase acknowledgement timed out"
            };
            self.status.last_error = Some(reason.into());
            self.status.phase = "Backoff".into();
            self.pending = None;
            self.status.pending = false;
            self.blocked_until = Some(now + RETRY_BACKOFF);
        }
        if self.blocked_until.is_some_and(|until| now < until) ||
            self.settle_until.is_some_and(|until| now < until) {
            return None;
        }
        let Some(gold) = self.gold else {
            self.status.phase = "Waiting for gold".into();
            return None;
        };
        let mut missing_catalog = false;
        let mut missing_inventory = false;
        let mut limited_by_reserve = false;
        for rule in &self.rules {
            if !self.known_stock[Self::idx(rule.kind)] {
                missing_inventory = true;
                continue;
            }
            let current = self.stock.get(&(rule.kind, rule.id)).copied().unwrap_or(0);
            if current > rule.low || current >= rule.target { continue; }
            let Some(meta) = self.prices.get(&(rule.kind, rule.id)) else {
                missing_catalog = true;
                continue;
            };
            let qty = (rule.target - current).min(MAX_QTY)
                .min(gold.saturating_sub(self.reserve) / meta.price);
            if qty == 0 {
                limited_by_reserve = true;
                continue;
            }
            let order = Order {
                kind: rule.kind, id: rule.id, name: meta.name.clone(),
                qty, price: meta.price, before: current,
                sent: now, ack: false,
            };
            self.status.last_order = Some(format!("{} ×{qty}", order.name));
            self.status.ordered = self.status.ordered.saturating_add(1);
            self.status.pending = true;
            self.status.phase = "Waiting for purchase".into();
            self.pending = Some(order);
            return Some(GameCommand::ShopBuy {
                kind: rule.kind.to_owned(), id: rule.id, qty
            });
        }
        self.status.phase = if missing_catalog { "Waiting for shop" }
            else if missing_inventory { "Waiting for inventory" }
            else if limited_by_reserve { "Gold reserve protected" }
            else { "Stock OK" }.into();
        None
    }
    pub fn write_failed(&mut self, now: Instant) {
        self.pending = None;
        self.status.pending = false;
        self.status.last_error = Some("Shop purchase send failed".into());
        self.status.phase = "Backoff".into();
        self.blocked_until = Some(now + RETRY_BACKOFF);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn configured() -> Restock {
        Restock::new(Some(&json!({"enabled":true,"restock":{
            "enabled":true,"selectedBallId":4,"ballAt":5,"ballTo":20,
            "selectedPotionId":200,"potionAt":2,"potionTo":12,"goldReserve":100
        }})))
    }
    fn welcome(r: &mut Restock, now: Instant) {
        r.ingest(r#"{"t":"welcome",
            "catalogoBolas":[{"id":4,"nome":"Ultra Ball","priceGold":10,"compravel":true}],
            "itensCompraveis":[{"itemId":200,"name":"Golden Potion","preco":5}],
            "estado":{"gold":190,"balls":{"4":3},"items":{"200":20}}
        }"#, now);
    }
    #[test]
    fn matches_exact_purchase_and_waits_for_stock() {
        let now = Instant::now();
        let mut r = configured();
        welcome(&mut r, now);
        assert_eq!(r.next(now, true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"ball","id":4,"qty":9}));
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra","id":123,"qtd":9,"gasto":90}]}"#, now);
        assert!(!r.pending.as_ref().unwrap().ack);
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra","id":4,"qtd":9,"gasto":90}]}"#, now);
        assert!(r.pending.as_ref().unwrap().ack);
        assert!(r.next(now + Duration::from_secs(2), true).is_none());
        r.ingest(r#"{"t":"estado","estado":{"balls":{"4":12},"gold":100}}"#, now);
        assert_eq!(r.status.confirmed, 1);
        assert!(r.next(now + Duration::from_secs(1), true).is_none());
    }
    #[test]
    fn no_purchases_without_authoritative_inventory_and_price() {
        let mut r = configured();
        let now = Instant::now();
        assert!(r.next(now, true).is_none());
        r.ingest(r#"{"t":"welcome","catalogoBolas":[{"id":4,"priceGold":10}],
            "estado":{"balls":{"4":0},"gold":1000}}"#, now);
        assert!(r.next(now, true).is_none());
        assert_eq!(r.status.phase, "Waiting for shop");
    }
    #[test]
    fn unacknowledged_purchase_backs_off_instead_of_immediate_duplicate() {
        let now = Instant::now();
        let mut r = configured();
        welcome(&mut r, now);
        assert!(r.next(now, true).is_some());
        assert!(r.next(now + Duration::from_secs(11), true).is_none());
        assert_eq!(r.status.phase, "Backoff");
        assert!(r.next(now + Duration::from_secs(12), true).is_none());
    }
    #[test]
    fn item_purchases_are_supported_only_from_verified_catalog() {
        let now = Instant::now();
        let mut r = configured();
        welcome(&mut r, now);
        r.ingest(r#"{"t":"estado","estado":{"balls":{"4":30},"items":{"200":1}}}"#, now);
        assert_eq!(r.next(now, true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":11}));
    }
    #[test]
    fn migration_imports_legacy_default_thresholds_without_a_userscript() {
        let r = Restock::new(Some(&json!({
            "enabled": true, "restock": { "enabled": true }
        })));
        assert!(r.status.enabled);
        assert!(r.status.configured);
        assert_eq!(r.rules.len(), 2);
        assert_eq!(r.rules[0].id, 1);
        assert_eq!(r.rules[0].low, 50);
        assert_eq!(r.rules[0].target, 500);
        assert_eq!(r.rules[1].id, 200);
    }
    #[test]
    fn reports_missing_item_price_instead_of_misleading_stock_ok() {
        let now = Instant::now();
        let mut r = configured();
        r.ingest(r#"{"t":"welcome",
            "catalogoBolas":[{"id":4,"nome":"Ultra Ball","priceGold":10,"compravel":true}],
            "estado":{"gold":10000,"balls":{"4":30},"items":{"200":0}}
        }"#, now);
        assert!(r.next(now, true).is_none());
        assert_eq!(r.status.phase, "Waiting for shop");
    }
    #[test]
    fn no_spend_below_gold_reserve_or_when_disabled() {
        let now = Instant::now();
        let mut r = configured();
        welcome(&mut r, now);
        r.set_enabled(false);
        assert!(r.next(now, true).is_none());
        r.set_enabled(true);
        r.ingest(r#"{"t":"estado","estado":{"gold":105}}"#, now);
        assert!(r.next(now, true).is_none());
    }
    #[test]
    fn unchanged_enabled_flag_does_not_erase_purchase_state() {
        let now=Instant::now();
        let mut r=configured();welcome(&mut r,now);
        r.next(now,true);
        assert_eq!(r.status.phase,"Waiting for purchase");
        r.set_enabled(true);
        assert_eq!(r.status.phase,"Waiting for purchase");
    }
    #[test]
    fn reconfiguration_preserves_inflight_purchase() {
        let now=Instant::now();let mut r=configured();
        welcome(&mut r,now);
        assert!(r.next(now,true).is_some());
        let mut cfg=CatchSettings::from_value(Some(&json!({
            "enabled":true,"restock":{"enabled":true,
                "selectedBallId":4,"ballAt":5,"ballTo":40}
        })));
        r.configure(&cfg);
        assert!(r.status.pending);
        assert!(r.next(now+Duration::from_secs(1),true).is_none());
        cfg.restock_enabled=false;r.configure(&cfg);
        assert!(r.status.pending);
    }
    #[test]
    fn reconnect_discards_uncertain_purchase_and_requires_fresh_state() {
        let now = Instant::now();
        let mut r = configured();
        welcome(&mut r, now);
        r.next(now, true).unwrap();
        r.on_disconnect(now);
        assert!(r.next(now + Duration::from_secs(31), true).is_none());
        assert_eq!(r.status.phase, "Waiting for gold");
    }
}
