//! Native Auto Catch restocking without browser/user-script execution.
//! All prices, funds and inventory counts are server-confirmed. At most one
//! shop.buy can be in flight per account. Purchase acknowledgments must match
//! the pending order; unrelated purchases cannot unblock the worker.
use crate::native_gameplay::GameCommand;
use crate::native_capture_config::CatchSettings;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const MAX_QTY: u64 = 5000;
const ACK_TIMEOUT: Duration = Duration::from_secs(10);
const SETTLE: Duration = Duration::from_secs(2);
const RETRY_BACKOFF: Duration = Duration::from_secs(30);
const ACK_STOCK_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug, Default)]
pub struct RestockRuleStatus {
    pub kind: String,
    pub id: u64,
    pub name: String,
    pub current: Option<u64>,
    pub low: u64,
    pub target: u64,
    pub price: Option<u64>,
    pub price_source: String,
}
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
    pub rules: Vec<RestockRuleStatus>,
}

#[derive(Clone, Debug)]
struct Rule {
    kind: &'static str,
    id: u64,
    low: u64,
    target: u64,
}
#[derive(Clone, Debug)]
struct ShopEntry { price: u64, name: String, source: &'static str }

// PokéIdle's published items catalog identifies these healing items by ID.
// Its npcPrice is only reference metadata, NOT an authenticated shop quote.
// A reference price permits a single-item probe; larger purchases require
// the live shop price or the actual unit cost in a server purchase receipt.
// Golden Potion is a separately observed market item absent from older JSON.
const REFERENCE_HEALING_ITEMS: &[(u64, &str, u64)] = &[
    (200, "Small Potion", 50),
    (201, "Great Potion", 150),
    (202, "Ultra Potion", 400),
    (203, "Hyper Potion", 800),
    (204, "Ultimate Potion", 1500),
    (205, "Revive", 600),
    (206, "Max Revive", 2500),
    (70070, "Golden Potion", 2500),
];
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
    unavailable: BTreeSet<(&'static str, u64)>,
    stock: BTreeMap<(&'static str, u64), u64>,
    known_stock: [bool; 2],
    gold: Option<u64>,
    pending: Option<Order>,
    settle_until: Option<Instant>,
    blocked_until: Option<Instant>,
    awaiting_inventory: Option<&'static str>,
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
        let mut worker = Self {
            status: RestockStatus {
                enabled, configured,
                phase: if !enabled { "Off" } else if !configured {
                    "Needs settings"
                } else { "Waiting for inventory" }.into(),
                ..Default::default()
            },
            rules, reserve: num("goldReserve", 0),
            prices: BTreeMap::new(), unavailable: BTreeSet::new(),
            stock: BTreeMap::new(),
            known_stock: [false, false], gold: None, pending: None,
            settle_until: None, blocked_until: None,
            awaiting_inventory: None,
        };
        worker.seed_reference_items();
        worker
    }
    /// Apply settings while maintaining pending transaction and authoritative
    /// shop/inventory state. Replacing the Restock worker would lose in-flight
    /// purchase identity and could trigger a duplicate order.
    pub fn configure(&mut self, config: &CatchSettings) {
        let candidate=Self::new(Some(&config.as_value()));
        self.rules=candidate.rules;
        self.reserve=candidate.reserve;
        self.status.configured=candidate.status.configured;
        self.seed_reference_items();
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
    fn seed_reference_items(&mut self) {
        for rule in &self.rules {
            if rule.kind != "item" || self.unavailable.contains(&("item", rule.id)) {
                continue;
            }
            if let Some((_, name, price)) = REFERENCE_HEALING_ITEMS.iter()
                .find(|(id, _, _)| *id == rule.id)
            {
                self.prices.entry(("item", rule.id)).or_insert(ShopEntry {
                    price: *price, name: name.to_lowercase(), source: "Reference catalog"
                });
            }
        }
    }
    fn read_catalog(&mut self, kind: &'static str, raw: Option<&Value>) {
        let Some(items) = raw.and_then(Value::as_array) else { return };
        for item in items {
            let Some(id) = item.get("itemId").or_else(|| item.get("id"))
                .and_then(Value::as_u64).filter(|id| *id > 0) else { continue };
            if item.get("compravel").and_then(Value::as_bool) == Some(false) {
                // An explicit denial from the authenticated shop wins over
                // older public catalog metadata.
                self.unavailable.insert((kind,id));
                self.prices.remove(&(kind,id));
                continue;
            }
            // Pokéballs explicitly advertise buyability in the welcome.
            if kind == "ball" && item.get("compravel").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let price = ["priceGold","preco","npcPrice"].iter()
                .filter_map(|field|item.get(field).and_then(Value::as_u64))
                .find(|price| *price > 0);
            let Some(price) = price else { continue };
            if item.get("compravel").and_then(Value::as_bool) == Some(true) {
                self.unavailable.remove(&(kind,id));
            }
            let name = item.get("nome").or_else(|| item.get("name"))
                .and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
            // For items, only potion and revive entries may be auto-purchased.
            // A misconfigured ID must never buy arbitrary marketplace items.
            if kind == "item" && !(name.contains("potion") ||
                name.contains("pocao") || name.contains("poção") ||
                name.contains("revive")) { continue; }
            self.prices.insert((kind, id), ShopEntry { price, name, source: "Server" });
        }
    }
    /// A configured, low-stock potion/revive may require the game's public
    /// item metadata when the welcome omits the purchasable-item list.
    pub fn needs_item_metadata(&self) -> bool {
        self.status.enabled && self.rules.iter().any(|rule|
            rule.kind == "item" && !self.unavailable.contains(&(rule.kind, rule.id))
                && self.prices.get(&(rule.kind, rule.id))
                    .is_none_or(|meta| meta.source == "Reference catalog")
        )
    }

    /// Accept only priced healing items from the game's own public catalog.
    /// An unpriced or non-healing item is never treated as shop-buyable.
    /// Authenticated server entries always take precedence over asset data.
    pub fn merge_item_metadata(&mut self, catalog: &crate::native_items::ItemCatalog) {
        for rule in &self.rules {
            if rule.kind != "item" { continue; }
            let Some(meta) = catalog.items.get(&rule.id) else { continue; };
            let name = meta.name.trim().to_lowercase();
            if !(name.contains("potion") || name.contains("poção") ||
                name.contains("pocao") || name.contains("revive") ||
                name.contains("reviver")) { continue; }
            let Some(price) = meta.price_gold.filter(|value|*value>0) else { continue; };
            if self.unavailable.contains(&("item",rule.id)) { continue; }
            let key = ("item", rule.id);
            if self.prices.get(&key).is_some_and(|meta|
                meta.source == "Server" || meta.source == "Server purchase") {
                continue;
            }
            self.prices.insert(key, ShopEntry { price, name, source: "Public catalog" });
        }
    }

    pub fn rule_statuses(&self) -> Vec<RestockRuleStatus> {
        self.rules.iter().map(|rule| {
            let item = self.prices.get(&(rule.kind,rule.id));
            RestockRuleStatus {
                kind: rule.kind.into(),
                id: rule.id,
                name: item.map(|meta|meta.name.clone())
                    .unwrap_or_else(||format!("{} #{}",rule.kind,rule.id)),
                current: if self.known_stock[Self::idx(rule.kind)] {
                    Some(self.stock.get(&(rule.kind,rule.id)).copied().unwrap_or(0))
                } else {None},
                low:rule.low,
                target:rule.target,
                price:item.map(|meta|meta.price),
                price_source: if self.unavailable.contains(&(rule.kind,rule.id)) {
                    "Not purchasable".into()
                } else {item.map(|meta|meta.source).unwrap_or("Unknown").into()},
            }
        }).collect()
    }
    pub fn publish_status(&mut self) {
        self.status.rules = self.rule_statuses();
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
        self.unavailable.clear();
        self.seed_reference_items();
        self.stock.clear();
        self.gold = None;
        self.awaiting_inventory = None;
        self.known_stock = [false, false];
        self.status.pending = false;
    }
    #[cfg(test)]
    pub fn ingest(&mut self, raw: &str, now: Instant) {
        let Ok(frame) = serde_json::from_str::<Value>(raw) else { return };
        self.ingest_value(&frame, now);
    }
    pub fn ingest_value(&mut self, frame: &Value, now: Instant) {
        if frame.get("t").and_then(Value::as_str) == Some("welcome") {
            if self.pending.is_some() {
                self.on_disconnect(now);
            }
            self.prices.clear();
            self.unavailable.clear();
            self.seed_reference_items();
            self.stock.clear();
            self.gold = None;
            self.awaiting_inventory = None;
            self.known_stock = [false, false];
        }
        // The live shop may be refreshed independently of welcome, including
        // inside an estado delta. Reconcile those authoritative price updates
        // without restarting the native socket.
        self.read_catalog("ball", frame.get("catalogoBolas"));
        self.read_catalog("item", frame.get("itensCompraveis"));
        if let Some(state) = frame.get("estado") {
            self.read_catalog("ball", state.get("catalogoBolas"));
            self.read_catalog("item", state.get("itensCompraveis"));
        }
        // Match purchase results by ID, kind, name, or exact quantity+spend.
        // It is common for the server to omit ID or kind in purchase events.
        let mut acknowledged = false;
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
                    let explicit_name_conflicts = event_name.as_deref()
                        .is_some_and(|name| !order.name.is_empty() && name != order.name);
                    if id_matches || name_matches ||
                        (qty_matches && spent_matches && !explicit_name_conflicts) {
                        order.ack = true;
                        // The paid amount, not npcPrice, determines the safe
                        // size of all subsequent restock orders.
                        if let (Some(qty), Some(spent)) = (
                            event.get("qtd").and_then(Value::as_u64),
                            event.get("gasto").and_then(Value::as_u64)
                        ) {
                            if qty == order.qty && qty > 0 && spent > 0 &&
                                spent % qty == 0 {
                                if let Some(meta) =
                                    self.prices.get_mut(&(order.kind, order.id)) {
                                    meta.price = spent / qty;
                                    meta.source = "Server purchase";
                                }
                            }
                        }
                        acknowledged = true;
                    }
                }
            }
        }
        if acknowledged {
            // A purchase receipt is authoritative even if a healing item was
            // consumed before the next inventory patch. Confirm the order,
            // but wait for fresh stock before considering another purchase.
            if let Some(order) = &self.pending {
                self.awaiting_inventory = Some(order.kind);
            }
            self.confirm(now);
            self.status.phase = "Waiting for updated inventory".into();
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
                    if self.awaiting_inventory == Some(kind) {
                        self.awaiting_inventory = None;
                        if self.pending.is_none() {
                            self.status.phase = "Stock synced".into();
                        }
                    }
                    if let Some(order) = &self.pending {
                        if order.kind == kind {
                            let current = self.stock.get(&(kind, order.id))
                                .copied().unwrap_or(0);
                            // A positive stock delta confirms an order even
                            // if its event was lost. When a matching server
                            // compra was already received, the next matching
                            // inventory patch also reconciles the purchase:
                            // a potion may be consumed before that patch,
                            // leaving unchanged or even lower net stock.
                            if current > order.before || order.ack {
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
        if self.awaiting_inventory.is_some() {
            self.status.phase = "Waiting for updated inventory".into();
            return None;
        }
        let Some(gold) = self.gold else {
            self.status.phase = "Waiting for gold".into();
            return None;
        };
        let mut missing_catalog = false;
        let mut unavailable = false;
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
                if self.unavailable.contains(&(rule.kind,rule.id)) {
                    unavailable = true;
                } else { missing_catalog = true; }
                continue;
            };
            let mut qty = (rule.target - current).min(MAX_QTY)
                .min(gold.saturating_sub(self.reserve) / meta.price);
            if meta.source == "Reference catalog" {
                // A known ID alone does not establish the current shop price.
                // Prices supplied by the public item catalog can fill the
                // configured target just as Auto Catch+ does.
                qty = qty.min(1);
            }
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
        self.status.phase = if unavailable { "Shop item unavailable" }
            else if missing_catalog { "Waiting for shop" }
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
        assert_eq!(r.status.confirmed, 1);
        assert!(!r.status.pending);
        assert!(r.next(now + Duration::from_secs(2), true).is_none());
        r.ingest(r#"{"t":"estado","estado":{"balls":{"4":12},"gold":100}}"#, now);
        assert_eq!(r.status.confirmed, 1);
        assert!(r.next(now + Duration::from_secs(1), true).is_none());
    }
    #[test]
    fn matching_purchase_and_flat_stock_reconciles_consumed_potion() {
        let now=Instant::now();
        let mut r=configured();
        welcome(&mut r,now);
        r.ingest(r#"{"t":"estado","estado":{"balls":{"4":30},
            "items":{"200":1}}}"#,now);
        let cmd=r.next(now,true).unwrap();
        assert_eq!(cmd.wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":11}));
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra","nome":"Golden Potion",
            "qtd":11,"gasto":55}]}"#,now+Duration::from_secs(1));
        assert_eq!(r.status.phase,"Waiting for updated inventory");
        assert_eq!(r.status.confirmed,1);
        // Shop purchase succeeded, but all new potions were consumed before
        // the following authoritative stock snapshot.
        r.ingest(r#"{"t":"estado","estado":{"items":{"200":1},"gold":130}}"#,
            now+Duration::from_secs(2));
        assert_eq!(r.status.confirmed,1);
        assert!(!r.status.pending);
        assert_eq!(r.status.phase,"Stock synced");
    }
    #[test]
    fn receipt_confirms_even_without_inventory_patch_but_prevents_double_buy() {
        let now=Instant::now();
        let mut r=configured();
        welcome(&mut r,now);
        r.ingest(r#"{"t":"estado","estado":{"balls":{"4":30},"items":{"200":0}}}"#,now);
        assert!(r.next(now,true).is_some());
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra",
            "id":200,"kind":"item","qtd":12,"gasto":60}]}"#,now);
        assert_eq!(r.status.confirmed,1);
        assert!(!r.status.pending);
        assert!(r.next(now+Duration::from_secs(31),true).is_none());
        assert_eq!(r.status.phase,"Waiting for updated inventory");
        r.ingest(r#"{"t":"estado","estado":{"items":{"200":12},"gold":130}}"#,now);
        assert!(r.next(now+Duration::from_secs(31),true).is_none());
        assert_eq!(r.status.phase,"Stock OK");
    }
    #[test]
    fn public_catalog_price_can_fill_target_without_one_item_probe() {
        let now=Instant::now();
        let mut r=configured();
        r.ingest(r#"{"t":"welcome","estado":{"gold":10000,
            "balls":{"4":30},"items":{"200":0}}}"#,now);
        let cat=crate::native_items::parse(r#"{"items":[
            {"id":200,"name":"Small Potion","preco":50}]}"#).unwrap();
        r.merge_item_metadata(&cat);
        assert_eq!(r.next(now,true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":12}));
    }
    #[test]
    fn unrelated_item_state_does_not_release_ball_purchase_receipt_wait() {
        let now=Instant::now();let mut r=configured();
        welcome(&mut r,now);
        r.next(now,true);
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra","id":4,
            "qtd":9,"gasto":90}]}"#,now);
        r.ingest(r#"{"t":"estado","estado":{"items":{"200":50}}}"#,now);
        assert!(!r.status.pending);
        assert_eq!(r.status.confirmed,1);
        assert!(r.next(now+Duration::from_secs(3),true).is_none());
        assert_eq!(r.status.phase,"Waiting for updated inventory");
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
    fn unknown_item_id_waits_for_shop_instead_of_guessing_its_price() {
        let now = Instant::now();
        let mut r = Restock::new(Some(&json!({"restock":{
            "enabled":true,"selectedBallId":0,"selectedPotionId":777,
            "potionAt":2,"potionTo":12
        }})));
        r.ingest(r#"{"t":"welcome",
            "estado":{"gold":10000,"balls":{},"items":{"777":0}}
        }"#, now);
        assert!(r.next(now, true).is_none());
        assert_eq!(r.status.phase, "Waiting for shop");
    }
    #[test]
    fn selected_revive_is_a_separate_restock_category() {
        let now=Instant::now();
        let mut r=Restock::new(Some(&json!({"restock":{
            "enabled":true,"selectedBallId":0,"selectedPotionId":0,
            "selectedReviveId":205,"reviveAt":1,"reviveTo":5
        }})));
        r.ingest(r#"{"t":"welcome",
            "itensCompraveis":[{"itemId":205,"name":"Revive","preco":600}],
            "estado":{"gold":10000,"balls":{},"items":{"205":1}}
        }"#,now);
        assert_eq!(r.next(now,true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":205,"qty":4}));
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra",
            "id":205,"kind":"item","qtd":4,"gasto":2400}]}"#,now);
        assert_eq!(r.status.confirmed,1);
    }
    #[test]
    fn golden_potion_is_known_without_synthetic_live_shop_quote() {
        let now=Instant::now();
        let mut r=Restock::new(Some(&json!({"restock":{
            "enabled":true,"selectedBallId":0,"selectedPotionId":70070,
            "potionAt":10,"potionTo":50
        }})));
        r.ingest(r#"{"t":"welcome",
            "estado":{"gold":10000,"balls":{},"items":{"70070":0}}
        }"#,now);
        assert_eq!(r.next(now,true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":70070,"qty":1}));
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
    fn accepts_authoritative_shop_catalog_refreshes_from_estado() {
        let now = Instant::now();
        let mut r = Restock::new(Some(&json!({"restock":{
            "enabled":true,"selectedBallId":0,"selectedPotionId":777,
            "potionAt":2,"potionTo":12
        }})));
        r.ingest(r#"{"t":"welcome","estado":{"gold":10000,
            "balls":{},"items":{"777":0}}}"#,now);
        assert!(r.next(now,true).is_none());
        assert_eq!(r.status.phase,"Waiting for shop");
        r.ingest(r#"{"t":"estado","estado":{"itensCompraveis":[
            {"itemId":777,"nome":"Future Potion","preco":5}]}}"#,now);
        assert_eq!(r.next(now+Duration::from_secs(1),true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":777,"qty":12}));
    }

    #[test]
    fn public_catalog_prices_only_explicit_healing_items() {
        let mut r = configured();
        let catalog = crate::native_items::parse(r#"{"items":[
            {"id":200,"name":"Golden Potion","category":"heal","preco":8},
            {"id":4,"name":"Ultra Ball","category":"balls","preco":15},
            {"id":999,"name":"Heart Stone","category":"stone","preco":5}]}"#).unwrap();
        assert!(r.needs_item_metadata());
        r.merge_item_metadata(&catalog);
        assert_eq!(r.prices.get(&("item",200)).unwrap().price,8);
        assert!(!r.prices.contains_key(&("item",999)));
        assert!(!r.prices.contains_key(&("ball",4)));
        let no_price=crate::native_items::parse(r#"{"items":[
            {"id":777,"name":"Future Potion","category":"heal"}]}"#).unwrap();
        let mut empty=Restock::new(Some(&json!({"restock":{
            "enabled":true,"selectedBallId":0,"selectedPotionId":777,
            "potionAt":2,"potionTo":12
        }})));
        empty.merge_item_metadata(&no_price);
        assert!(!empty.prices.contains_key(&("item",777)));
    }

    #[test]
    fn known_potion_id_can_probe_without_live_shop_catalog_and_learn_real_price() {
        let now = Instant::now();
        let mut r = configured();
        r.ingest(r#"{"t":"welcome","estado":{
            "gold":10000,"balls":{"4":30},"items":{}
        }}"#, now);
        r.publish_status();
        let potion = r.status.rules.iter().find(|rule|rule.id == 200).unwrap();
        assert_eq!(potion.price_source, "Reference catalog");
        assert_eq!(potion.current, Some(0));
        assert_eq!(r.next(now, true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":1}));
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra",
            "id":200,"kind":"item","qtd":1,"gasto":77}]}"#,
            now+Duration::from_secs(1));
        r.ingest(r#"{"t":"estado","estado":{"items":{"200":1},"gold":9923}}"#,
            now+Duration::from_secs(1));
        assert_eq!(r.status.confirmed,1);
        r.publish_status();
        let potion = r.status.rules.iter().find(|rule|rule.id == 200).unwrap();
        assert_eq!(potion.price,Some(77));
        assert_eq!(potion.price_source,"Server purchase");
        assert_eq!(r.next(now+Duration::from_secs(3),true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":11}));
    }
    #[test]
    fn authoritative_shop_price_overrides_reference_and_skips_probe() {
        let now = Instant::now();
        let mut r = configured();
        r.ingest(r#"{"t":"welcome",
            "itensCompraveis":[{"itemId":200,"nome":"Small Potion","preco":60}],
            "estado":{"gold":10000,"balls":{"4":30},"items":{"200":0}}
        }"#,now);
        assert_eq!(r.next(now,true).unwrap().wire(),
            json!({"t":"shop.buy","kind":"item","id":200,"qty":12}));
        assert_eq!(r.prices.get(&("item",200)).unwrap().source,"Server");
    }
    #[test]
    fn unrelated_purchase_cannot_acknowledge_same_cost_different_item() {
        let now=Instant::now();
        let mut r=configured();
        welcome(&mut r,now);
        r.next(now,true);
        // Correct quantity/price but an explicitly different name.
        r.ingest(r#"{"t":"batalha","ev":[{"k":"compra",
            "nome":"Different Ball","qtd":9,"gasto":90}]}"#,now);
        assert!(!r.pending.as_ref().unwrap().ack);
    }


    #[test]
    fn explicit_server_unbuyable_item_cannot_be_reenabled_by_public_metadata() {
        let now=Instant::now();
        let mut r=configured();
        r.ingest(r#"{"t":"welcome","itensCompraveis":[
            {"itemId":200,"name":"Potion","preco":5,"compravel":false}],
            "estado":{"gold":1000,"balls":{"4":40},"items":{"200":0}}}"#,now);
        let meta=crate::native_items::parse(r#"{"items":[
            {"id":200,"name":"Potion","preco":5}]}"#).unwrap();
        r.merge_item_metadata(&meta);
        assert!(!r.prices.contains_key(&("item",200)));
        assert!(!r.needs_item_metadata());
        assert!(r.next(now,true).is_none());
        assert_eq!(r.status.phase,"Shop item unavailable");
        r.publish_status();
        assert_eq!(r.status.rules.iter().find(|v|v.id==200).unwrap().price_source,
            "Not purchasable");
    }
    #[test]
    fn status_explains_stock_price_and_catalog_provenance() {
        let now=Instant::now();
        let mut r=configured();
        welcome(&mut r,now);
        r.publish_status();
        let ball=r.status.rules.iter().find(|p|p.kind=="ball").unwrap();
        assert_eq!(ball.current,Some(3));
        assert_eq!(ball.low,5);
        assert_eq!(ball.target,20);
        assert_eq!(ball.price,Some(10));
        assert_eq!(ball.price_source,"Server");
        assert!(r.status.rules.iter().any(|p|p.kind=="item"&&p.price==Some(5)));
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
