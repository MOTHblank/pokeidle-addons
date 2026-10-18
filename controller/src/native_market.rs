//! Native Moth Watch-style rule buyer. All requests use the authenticated game
//! WebSocket already owned by Moth Controller; no browser/userscript is required.
//! Settings and a bounded conservative spending journal are profile-local.
use crate::config::{self, GameProfile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque, HashSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn clock_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn today() -> u64 { clock_ms() / 86_400_000 }
fn number(v: &Value, field: &str) -> Option<u64> {
    v.get(field).and_then(Value::as_u64)
}
fn path(profile: GameProfile, name: &str) -> Result<std::path::PathBuf, String> {
    Ok(config::data_root()?.join(format!("market-{name}-{}.json", profile.name())))
}
fn write_json<T: Serialize>(profile: GameProfile, name: &str, obj: &T) -> Result<(), String> {
    let path = path(profile, name)?;
    let parent = path.parent().ok_or("Invalid market configuration path")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Market directory: {e}"))?;
    // Purchase receipts are machine-readable and rewritten frequently.
    // Compact JSON reduces both serialization work and disk traffic.
    let contents = if name == "orders" {
        serde_json::to_vec(obj)
    } else {
        serde_json::to_vec_pretty(obj)
    }.map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents).map_err(|e| format!("Market save: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Market atomic replace: {e}"))
}
fn read_json<T: for<'de> Deserialize<'de>>(profile: GameProfile, name: &str) -> Result<Option<T>, String> {
    let p = path(profile, name)?;
    let bytes = match std::fs::read(p) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Market read: {e}")),
    };
    if bytes.len() > 2_000_000 { return Err("Market file exceeds 2 MB".into()); }
    serde_json::from_slice(&bytes).map(Some).map_err(|e| format!("Market JSON: {e}"))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BuyRule {
    pub id: String,
    /// "item", "any_item", "pokemon_any", "pokemon_shiny", "pokemon_p5"
    pub kind: String,
    pub item_id: u64,
    pub currency: String,
    pub enabled: bool,
    pub buy_max: u64,
    pub max_spend_session: u64,
    pub max_spend_day: u64,
    pub max_purchases_session: u64,
    pub max_quantity_session: u64,
}
impl Default for BuyRule {
    fn default() -> Self {
        Self { id: "new".into(), kind: "item".into(), item_id: 0,
            currency: "gold".into(), enabled: false, buy_max: 0,
            max_spend_session: 0, max_spend_day: 0,
            max_purchases_session: 0, max_quantity_session: 0 }
    }
}
impl BuyRule {
    fn valid(&self) -> bool {
        !self.id.is_empty() && self.id.len() <= 100 &&
        matches!(self.kind.as_str(), "item" | "any_item" | "pokemon_any" | "pokemon_shiny" | "pokemon_p5") &&
        matches!(self.currency.as_str(), "gold" | "orb") &&
        (self.kind != "item" || self.item_id > 0) &&
        (self.kind != "pokemon_shiny" || self.currency == "orb")
    }
}

/* Manual Snatches are explicit price-ceiling authorizations for one listing.
   No remembered browser quote is trusted after native reconnect: exact
   market.anuncio verification supplies the listing and unlock timestamp. */
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnatchTarget {
    pub id: u64, pub kind: String, pub currency: String,
    pub item_id: u64, pub max_price: u64, pub expires_at: u64,
}
impl SnatchTarget {
    pub fn arm(&mut self) {
        // Expiry will extend to 30m after the discovered hold release.
        self.expires_at=clock_ms().saturating_add(30*60*1000);
    }
    fn valid(&self)->bool {
        self.id>0 && matches!(self.kind.as_str(),"item"|"pokemon") &&
        matches!(self.currency.as_str(),"gold"|"orb") &&
        self.max_price>0 && (self.kind=="pokemon" || self.item_id>0) &&
        self.expires_at>0
    }
}
#[derive(Clone,Debug,Default)]
pub struct SnatchRow {
    pub id:u64, pub kind:String, pub currency:String, pub item_id:u64,
    pub price:u64, pub quantity:u64, pub release_at:u64,
    pub attempts:u64, pub manual:bool, pub phase:String,
}
#[derive(Clone)]
struct SnatchEntry {
    offer:Offer, rule_id:Option<String>, attempts:u64,
    retry_at:Option<Instant>, last_verified:Option<Instant>,
    needs_verify:bool, release_at:u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarketRules {
    pub enabled: bool,
    pub paused: bool,
    pub gold_reserve: u64,
    pub gem_reserve: u64,
    pub max_gold_per_buy: u64,
    pub max_gems_per_buy: u64,
    pub max_gold_session: u64,
    pub max_gems_session: u64,
    pub max_gold_day: u64,
    pub max_gems_day: u64,
    pub max_purchases_session: u64,
    pub max_quantity_session: u64,
    pub rules: Vec<BuyRule>,
    pub snatch_targets: Vec<SnatchTarget>,
}
impl Default for MarketRules {
    fn default() -> Self {
        Self { enabled: false, paused: false, gold_reserve: 0, gem_reserve: 0,
            max_gold_per_buy: 0, max_gems_per_buy: 0, max_gold_session: 0,
            max_gems_session: 0, max_gold_day: 0, max_gems_day: 0,
            max_purchases_session: 0, max_quantity_session: 0, rules: vec![], snatch_targets:vec![] }
    }
}
impl MarketRules {
    /// Clone buying rules, reserves and limits across profiles while leaving
    /// the destination's listing-specific Snatch queue untouched. Activation
    /// is an explicit choice: the default is not to start spending money.
    pub fn copy_buying_from(&self, source: &Self, copy_activation: bool) -> Self {
        let mut result = source.clone();
        result.snatch_targets = self.snatch_targets.clone();
        if !copy_activation {
            result.enabled = false;
        }
        result
    }
    pub fn load(profile: GameProfile) -> Result<Self, String> {
        let rules: Self = read_json(profile, "rules")?.unwrap_or_default();
        rules.validate()?;
        Ok(rules)
    }
    pub fn save(&self, profile: GameProfile) -> Result<(), String> {
        self.validate()?;
        write_json(profile, "rules", self)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.rules.len() > 64 { return Err("Maximum 64 market rules".into()); }
        if self.snatch_targets.len()>40 {return Err("Maximum 40 Snatch targets".into())}
        let mut snatch_ids=HashSet::new();
        for target in &self.snatch_targets {
            if !target.valid() || !snatch_ids.insert(target.id) {
                return Err(format!("Invalid or duplicated Snatch listing #{}",target.id));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.rules {
            if !r.valid() || !ids.insert(&r.id) {
                return Err(format!("Invalid or duplicate rule {}", r.id));
            }
            if r.enabled && r.buy_max == 0 { return Err(format!("Rule {} needs a price ceiling", r.id)); }
        }
        Ok(())
    }
    /// Accept raw JSON copied from browser's Moth Watch localStorage config.
    /// Imports rule categories and cap semantics; never activates buying by default.
    pub fn import_watch(input: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(input).map_err(|e| format!("Moth Watch JSON: {e}"))?;
        if !v.is_object() { return Err("Expected Moth Watch settings object".into()); }
        let mut out = Self::default();
        let get = |k: &str| number(&v, k).unwrap_or(0);
        out.paused = v.get("purchasesPaused").and_then(Value::as_bool).unwrap_or(false);
        out.gold_reserve = get("goldReserve");
        out.gem_reserve = get("gemReserve");
        out.max_gold_per_buy = get("maxCoinsPerBuy");
        out.max_gems_per_buy = get("maxGemsPerBuy");
        out.max_gold_session = get("maxCoinsPerSession");
        out.max_gems_session = get("maxGemsPerSession");
        out.max_gold_day = get("maxCoinsPerDay");
        out.max_gems_day = get("maxGemsPerDay");
        out.max_purchases_session = get("maxPurchasesPerSession");
        out.max_quantity_session = get("maxQuantityPerSession");
        for (section, prefix) in [("fixedItemRules", "item"),
             ("anyItemRules", "any_item"), ("pokemonBuyRules", "pokemon")] {
            let Some(entries) = v.get(section).and_then(Value::as_array) else { continue; };
            for (i, entry) in entries.iter().enumerate() {
                let kind = if prefix == "pokemon" {
                    match entry.get("kind").and_then(Value::as_str) {
                        Some("any") => "pokemon_any",
                        Some("shiny") => "pokemon_shiny",
                        Some("p5") => "pokemon_p5", _ => continue
                    }
                } else { prefix };
                let rule = BuyRule {
                    id: format!("import-{section}-{i}"),
                    kind: kind.to_owned(),
                    item_id: number(entry, "itemId").unwrap_or(0),
                    currency: entry.get("currency").and_then(Value::as_str).unwrap_or("gold").into(),
                    // Preserve each rule's arming state, but the global engine is OFF.
                    enabled: entry.get("buyEnabled").and_then(Value::as_bool).unwrap_or(section == "fixedItemRules"),
                    buy_max: number(entry, "buyMax").unwrap_or(0),
                    max_spend_session: number(entry, "maxSpendSession").unwrap_or(0),
                    max_spend_day: number(entry, "maxSpendDay").unwrap_or(0),
                    max_purchases_session: number(entry, "maxPurchasesSession").unwrap_or(0),
                    max_quantity_session: number(entry, "maxQuantitySession").unwrap_or(0),
                };
                if rule.buy_max > 0 && rule.valid() { out.rules.push(rule); }
            }
        }
        out.validate()?;
        Ok(out)
    }
}
#[cfg(test)]
mod buying_copy_tests {
    use super::*;
    #[test]
    fn copy_preserves_target_snatches_and_disables_buying_by_default() {
        let mut source=MarketRules::default();
        source.enabled=true;
        source.gold_reserve=1234;
        source.rules=vec![BuyRule {id:"test-rule".into(),enabled:true,
            buy_max:200,kind:"item".into(),item_id:66,..Default::default()}];
        let mut target=MarketRules::default();
        let mut snatch=SnatchTarget {
            id:991, kind:"item".into(), item_id:66,
            currency:"gold".into(), max_price:10,..Default::default()
        };
        snatch.arm();
        target.snatch_targets.push(snatch.clone());
        let copied=target.copy_buying_from(&source,false);
        assert!(!copied.enabled);
        assert_eq!(copied.gold_reserve,1234);
        assert_eq!(copied.rules,source.rules);
        assert_eq!(copied.snatch_targets,vec![snatch.clone()]);
        copied.validate().unwrap();
        let live=target.copy_buying_from(&source,true);
        assert!(live.enabled);
        assert_eq!(live.snatch_targets,vec![snatch]);
        live.validate().unwrap();
    }
}

#[derive(Clone, Debug, Default)]
pub struct MarketStatus {
    pub enabled: bool, pub pending: Option<String>, pub sent: u64,
    pub confirmed: u64, pub unknown: u64, pub last: String, pub error: Option<String>,
    pub last_scan: Option<Instant>,
    pub snatches:Vec<SnatchRow>, pub held_offers:Vec<SnatchRow>,
    pub snatch_sent:u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Order {
    listing_id: u64, rule_id: String, currency: String, kind: String,
    quantity: u64, spend: u64, day: u64, session: u64, status: String,
    sent_ms: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ArchivedExposure {
    session: u64, day: u64, currency: String, rule_id: String,
    spend: u64, count: u64, quantity: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Journal { orders: Vec<Order>, archived: Vec<ArchivedExposure> }
#[derive(Clone)]
struct Offer {
    id: u64, kind: String, item_id: u64, currency: String, price: u64, qty: u64,
    seller: String, held_until: u64, shiny: Option<bool>, potency: Option<u64>,
    at: Instant,
}
impl Offer {
    fn from_listing(v: &Value, kind: &str, item_id: u64, currency: &str, now: Instant) -> Option<Self> {
        let id = number(v, "id")?;
        let price = number(v, "preco")?;
        let qty = number(v, "qtd").unwrap_or(if kind == "pokemon" { 1 } else { 0 });
        // PokeIdle Manager checks explicit listing scope and state before
        // dispatch. Do not require optional aliases that Watch also accepts,
        // but reject a reply explicitly contradicting its market.item scope.
        if v.get("tipo").and_then(Value::as_str).is_some_and(|s|s!=kind) ||
           v.get("moeda").and_then(Value::as_str).is_some_and(|s|s!=currency) ||
           v.get("itemId").and_then(Value::as_u64).is_some_and(|id|id!=item_id) ||
           v.get("estado").and_then(Value::as_str).is_some_and(|s|
               !s.is_empty() && !s.eq_ignore_ascii_case("aberto")) { return None; }
        let seller = v.get("vendedor").and_then(Value::as_str)?.trim();
        if id == 0 || price == 0 || qty == 0 || seller.is_empty() { return None; }
        let ficha = v.get("ficha");
        if kind == "pokemon" && ficha.is_none() { return None; }
        Some(Self {
            id, price, qty, kind: kind.into(), item_id, currency: currency.into(),
            seller: seller.to_owned(),
            held_until: number(v, "compravelEm").unwrap_or(0),
            shiny: ficha.and_then(|v|v.get("shiny")).and_then(Value::as_bool),
            potency: ficha.and_then(|v|number(v,"potencia")), at: now,
        })
    }
    fn matches(&self, rule: &BuyRule, nick: &str, server_ms: u64) -> bool {
        if self.price > rule.buy_max || self.seller.eq_ignore_ascii_case(nick) ||
           self.held_until > server_ms || self.at.elapsed() > Duration::from_secs(30) { return false; }
        match rule.kind.as_str() {
            "item" => self.kind == "item" && self.item_id == rule.item_id,
            "any_item" => self.kind == "item",
            "pokemon_any" => self.kind == "pokemon" && self.shiny.is_some() &&
                (1..=5).contains(&self.potency.unwrap_or(0)) &&
                (self.currency == "orb" || self.shiny == Some(false)),
            "pokemon_p5" => self.kind == "pokemon" && self.shiny.is_some() &&
                self.potency == Some(5) && (self.currency == "orb" || self.shiny == Some(false)),
            "pokemon_shiny" => self.kind == "pokemon" && self.shiny == Some(true) &&
                self.currency == "orb" && (1..=5).contains(&self.potency.unwrap_or(0)),
            _ => false,
        }
    }
}
#[derive(Clone)]
enum Scan { Summary, Item(u64,String), Pokemon(String,u64) }

// Market vitrine replies may omit top-level kind/currency on nonempty pages.
// Accept them only with positive per-row Pokémon and currency evidence. An
// empty page has no such evidence and must carry all three discriminators.
fn pokemon_scan_rows<'a>(frame: &'a Value, currency: &str, page: u64) -> Option<&'a Vec<Value>> {
    if frame.get("tipo").and_then(Value::as_str).is_some_and(|kind|kind!="pokemon") ||
        frame.get("moeda").and_then(Value::as_str).is_some_and(|value|value!=currency) ||
        number(frame,"pagina").is_some_and(|value|value!=page) {return None}
    let rows=frame.get("linhas")?.as_array()?;
    if rows.is_empty() {
        return (frame.get("tipo").and_then(Value::as_str)==Some("pokemon") &&
            frame.get("moeda").and_then(Value::as_str)==Some(currency) &&
            number(frame,"pagina")==Some(page)).then_some(rows);
    }
    if page!=0 && number(frame,"pagina").is_none() {return None}
    if !rows.iter().all(|row|
        row.get("ficha").is_some_and(Value::is_object) &&
        row.get("tipo").and_then(Value::as_str).is_none_or(|kind|kind=="pokemon") &&
        row.get("moeda").and_then(Value::as_str).is_none_or(|value|value==currency)) {return None}
    if frame.get("moeda").and_then(Value::as_str).is_none() &&
        !rows.iter().all(|row|row.get("moeda").and_then(Value::as_str)==Some(currency)) {
        return None;
    }
    Some(rows)
}
pub struct MarketBuyer {
    profile: GameProfile, session: u64, journal: Journal,
    last_housekeeping: Option<Instant>,
    pub status: MarketStatus, nick: String, balances: [Option<u64>;2],
    server_offset_ms: i64, offers: Vec<Offer>, summary: BTreeMap<u64,[u64;2]>,
    // Up to five in-flight orders per account and three per item listing, as
    // with Moth Watch. Independent receipts can be resolved without serial
    // acknowledgement stalls. All attempts are journaled before dispatch.
    pending: VecDeque<usize>, pending_scan: Option<(Scan,Instant)>,
    last_scan: HashMap<String,Instant>, urgent_details: HashSet<String>,
    scan_sequence: u64, last_buy: Option<Instant>,
    blocked: HashMap<u64, Instant>, backoff_until: Option<Instant>, failed_closed: bool,
    snatch:HashMap<u64,SnatchEntry>, snatch_closed:HashMap<u64,String>,
    snatch_verifying:HashMap<u64,Instant>, snatch_last_verify:HashMap<u64,Instant>,
    snatch_last_send:HashMap<u64,Instant>,
}
impl MarketBuyer {
    pub fn new(profile: GameProfile) -> Self {
        let loaded = read_json::<Journal>(profile, "orders");
        let (mut journal,error) = match loaded {
            Ok(Some(j)) if j.orders.len() <= 20_000 => (j,None),
            Ok(None) => (Journal::default(),None),
            Ok(_) => (Journal::default(),Some("Market order journal too large; buying disabled".into())),
            Err(e) => (Journal::default(),Some(e)),
        };
        // Remove history that cannot contribute to today's caps on startup.
        // We never truncate a live pending order's vector index mid-session.
        journal.orders.retain(|o| o.day.saturating_add(1)>=today());
        journal.archived.retain(|o| o.day.saturating_add(1)>=today());
        // A crash can interrupt dispatch after journaling but before a receipt.
        // Such entries remain exposure, never an assumed confirmation.
        for o in &mut journal.orders {
            if o.status=="submitted" {o.status="unknown".into();}
        }
        Self { profile, session: clock_ms(), journal, last_housekeeping: None,
            status: MarketStatus {error:error.clone(),..Default::default()},
            nick:String::new(), balances:[None,None], server_offset_ms:0, offers:vec![],
            summary:BTreeMap::new(),pending:VecDeque::new(),pending_scan:None,last_scan:HashMap::new(),
            urgent_details:HashSet::new(),scan_sequence:0,
            last_buy:None,blocked:HashMap::new(),backoff_until:None,failed_closed:error.is_some(),
            snatch:HashMap::new(),snatch_closed:HashMap::new(),
            snatch_verifying:HashMap::new(),snatch_last_verify:HashMap::new(),
            snatch_last_send:HashMap::new() }
    }
    pub fn on_disconnect(&mut self) {
        let had_pending=!self.pending.is_empty();
        let old=std::mem::take(&mut self.pending);
        for idx in old {
            if let Some(o)=self.journal.orders.get_mut(idx) {
                if o.status=="submitted" {o.status="unknown".into();}
            }
        }
        // Welcome/reconnect with no outstanding buys does not mutate the
        // journal and must not rewrite it to disk.
        if had_pending { self.persist(); }
        self.pending.clear(); self.pending_scan=None; self.offers.clear();
        self.urgent_details.clear();
        self.snatch.clear();self.snatch_closed.clear();self.snatch_verifying.clear();
        self.snatch_last_verify.clear();self.snatch_last_send.clear();
        self.status.snatches.clear();
        self.status.held_offers.clear();
        self.summary.clear(); self.nick.clear(); self.balances=[None,None];
        self.status.pending=None;
    }
    fn persist(&mut self) -> bool {
        // Record every spend exposure before network dispatch; old days were
        // pruned at startup, never while a pending vector index is live.
        match write_json(self.profile, "orders", &self.journal) {
            Ok(()) => true,
            Err(e) => { self.failed_closed=true; self.status.error=Some(e); false }
        }
    }
    fn compact(&mut self) {
        // Mirror Watch's rolling journal: retain pending/current attempts,
        // archive older outcomes instead of stopping automated purchases.
        if self.journal.orders.len()<2400 {return}
        let threshold=self.journal.orders.len().saturating_sub(1400);
        let in_flight=self.pending.iter().copied().collect::<HashSet<_>>();
        let mut new_pending=VecDeque::new();
        let mut retained=Vec::new();
        for (idx,o) in std::mem::take(&mut self.journal.orders).into_iter().enumerate() {
            let keep=idx>=threshold || in_flight.contains(&idx) ||
                ((o.status=="unknown" || o.status=="submitted") &&
                    clock_ms().saturating_sub(o.sent_ms)<60_000);
            if keep {
                if in_flight.contains(&idx){new_pending.push_back(retained.len());}
                retained.push(o);
            } else if o.status!="refused" {
                let key=(o.session,o.day,o.currency.as_str(),o.rule_id.as_str());
                if let Some(a)=self.journal.archived.iter_mut().find(|a|
                    (a.session,a.day,a.currency.as_str(),a.rule_id.as_str())==key) {
                    a.spend=a.spend.saturating_add(o.spend);
                    a.count=a.count.saturating_add(1);
                    a.quantity=a.quantity.saturating_add(o.quantity);
                } else {
                    self.journal.archived.push(ArchivedExposure {
                        session:o.session,day:o.day,currency:o.currency,
                        rule_id:o.rule_id,spend:o.spend,count:1,quantity:o.quantity,
                    });
                }
            }
        }
        self.journal.orders=retained;
        self.pending=new_pending;
        self.update_pending();
    }
    fn update_pending(&mut self) {
        self.status.pending=if self.pending.is_empty() {None} else {
            Some(format!("{} market purchase(s) awaiting confirmation",self.pending.len()))
        };
    }
    fn settle_index(&mut self, idx:usize, success:bool, now:Instant) {
        self.pending.retain(|v| *v!=idx);
        let Some(o)=self.journal.orders.get_mut(idx) else {self.update_pending();return};
        let id=o.listing_id;
        let is_item=o.kind=="item";
        o.status=if success {"confirmed"} else {"refused"}.into();
        self.status.last=format!("{} #{} · {} × {} {}",
            if success {"Confirmed"} else {"Refused"},id,o.quantity,o.spend,o.currency);
        if success {self.status.confirmed=self.status.confirmed.saturating_add(1);}
        self.snatch_reconcile(id,success,now);
        // Item listings can be partially filled. Retry maximum available
        // quantity immediately after a short cooldown, unlike the 30-second
        // native block. Unique Pokémon must never be purchased twice.
        let gap=if success {
            if is_item {Duration::from_millis(400)} else {Duration::from_secs(1800)}
        } else {Duration::from_secs(5)};
        self.blocked.insert(id,now+gap);
        if success && is_item {
            // Moth Watch refreshes the exact offer after a confirmed buy.
            // The server may have partially filled the lot, so replace the
            // cached quote before authorizing its next maximum-qty order.
            for o in self.offers.iter().filter(|offer|offer.id==id) {
                let key=format!("item:{}:{}",o.item_id,o.currency);
                self.last_scan.remove(&key);
                self.urgent_details.insert(key);
            }
            self.offers.retain(|offer|offer.id!=id);
        }
        self.persist();
        self.update_pending();
    }
    fn expire_pending(&mut self,now:Instant){
        let old=std::mem::take(&mut self.pending);
        let mut changed=false;
        for idx in old {
            let Some(o)=self.journal.orders.get(idx) else {continue};
            if clock_ms().saturating_sub(o.sent_ms)<5_000 {
                self.pending.push_back(idx);continue;
            }
            let id=o.listing_id;
            let item=o.kind=="item";
            if let Some(record)=self.journal.orders.get_mut(idx) {
                record.status="unknown".into();
            }
            self.status.unknown=self.status.unknown.saturating_add(1);
            self.status.error=Some(format!(
                "Outcome unverified for listing #{id}; quoted spend remains reserved"
            ));
            // Moth Watch retries item rules on fresh verified quotes even
            // after timeouts; unique Pokémon remain withheld until reconciled.
            self.blocked.insert(id,now+if item {
                Duration::from_millis(400)
            } else {Duration::from_secs(1800)});
            self.snatch_reconcile(id,false,now);
            changed=true;
        }
        if changed {self.persist();}
        self.update_pending();
    }
    #[cfg(test)]
    pub fn ingest(&mut self, text: &str, now: Instant, cfg:&MarketRules) {
        let Ok(frame)=serde_json::from_str::<Value>(text) else{return};
        self.ingest_value(&frame, now, cfg);
    }
    pub fn ingest_value(&mut self, frame: &Value, now: Instant, cfg:&MarketRules) {
        let t=frame.get("t").and_then(Value::as_str).unwrap_or("");
        if t=="welcome" { self.on_disconnect(); }
        if matches!(t,"welcome"|"estado") {
            if let Some(s)=frame.get("estado") {
                if let Some(nick)=s.get("nick").and_then(Value::as_str) {self.nick=nick.to_owned();}
                for (i,k) in ["gold","orbs"].iter().enumerate() {
                    if let Some(v)=number(s,k) {self.balances[i]=Some(v);}
                }
                if let Some(server)=number(s,"servidorAgora") {
                    self.server_offset_ms=clock_ms() as i64-server as i64;
                }
            }
        }
        if t=="market" && frame.get("aba").and_then(Value::as_str)==Some("anuncio") {
            self.snatch_exact_response(&frame,cfg,now);
            return;
        }
        if t=="batalha" {
            if let Some(events)=frame.get("ev").and_then(Value::as_array) {
                for event in events {
                    if event.get("k").and_then(Value::as_str)!=Some("marketComprado"){continue}
                    let mut ids=["listingId","anuncioId","vendaId"].iter()
                        .filter_map(|k|number(event,k)).collect::<Vec<_>>();
                    for nested in ["anuncio","listing"] {
                        if let Some(id)=event.get(nested).and_then(|v|number(v,"id")){ids.push(id);}
                    }
                    if !ids.is_empty() {
                        if let Some(idx)=self.pending.iter().copied().find(|i|
                            self.journal.orders.get(*i).is_some_and(|o|ids.contains(&o.listing_id))) {
                            self.settle_index(idx,true,now);
                        }
                        continue;
                    }
                    // An explicitly supplied but unparseable listing identity
                    // is not permission to credit an unrelated order.
                    if ["listingId","anuncioId","vendaId","anuncio","listing"].iter()
                        .any(|k|event.get(*k).is_some()){continue}
                    // Moth Watch accepts idless marketComprado only when a
                    // single listing can own it. PokeIdle Manager also uses
                    // confirmed currency/total to disambiguate some receipts.
                    let receipt_currency=event.get("moeda").and_then(Value::as_str);
                    let receipt_total=number(event,"total");
                    let recent_unknown=self.journal.orders.iter().any(|o|
                        o.status=="unknown" &&
                        clock_ms().saturating_sub(o.sent_ms)<30_000 &&
                        receipt_currency.is_none_or(|c|c==o.currency) &&
                        receipt_total.is_none_or(|v|v==o.spend));
                    if recent_unknown {continue}
                    let ids=self.pending.iter().filter_map(|i|
                        self.journal.orders.get(*i).map(|o|o.listing_id))
                        .collect::<HashSet<_>>();
                    let idx=if ids.len()==1 {
                        self.pending.front().copied().filter(|i|
                            self.journal.orders.get(*i).is_some_and(|o|
                                receipt_currency.is_none_or(|currency|currency==o.currency) &&
                                receipt_total.is_none_or(|total|total>0 && total<=o.spend)))
                    } else if let (Some(currency),Some(total))=(receipt_currency,receipt_total) {
                        let matching=self.pending.iter().copied().filter(|i|
                            self.journal.orders.get(*i).is_some_and(|o|
                                o.currency==currency && o.spend==total)).collect::<Vec<_>>();
                        if matching.len()==1 {matching.first().copied()} else {None}
                    } else {None};
                    if let Some(i)=idx {self.settle_index(i,true,now);}
                }
            }
        }
        if t=="erro" {
            let id=["listingId","anuncioId","vendaId"].iter()
                .find_map(|k|number(&frame,k))
                .or_else(||frame.get("anuncio").and_then(|v|number(v,"id")))
                .or_else(||frame.get("listing").and_then(|v|number(v,"id")));
            if let Some(id)=id {
                if let Some(idx)=self.pending.iter().copied().find(|i|
                    self.journal.orders.get(*i).is_some_and(|o|o.listing_id==id)) {
                    self.settle_index(idx,false,now);
                }
            }
            let message=["msg","message","chave"].iter().filter_map(|k|frame.get(k).and_then(Value::as_str)).collect::<Vec<_>>().join(" ");
            if message.to_lowercase().contains("rate limit") || message.to_lowercase().contains("muitas mensagens") {
                self.status.error=Some("Market request rate-limited; backing off".into());
                self.backoff_until=Some(now+Duration::from_secs(15));
            }
        }
        if t!="market" {return}
        let Some((scan,requested_at))=self.pending_scan.take() else{return};
        let matched=match (&scan,frame.get("aba").and_then(Value::as_str)) {
            (Scan::Summary,Some("itens"))=>{
                if let Some(rows)=frame.get("resumo").and_then(Value::as_object) {
                    self.summary.clear();
                    for (id,row) in rows {
                        if let Ok(id)=id.parse::<u64>() {
                            let advertised=row.get("anuncios").is_some_and(|v|
                                v.as_u64().is_some_and(|n|n>0) || v.as_bool()==Some(true));
                            if id>0 && advertised {
                                self.summary.insert(id,[number(row,"minGold").unwrap_or(0),number(row,"minOrb").unwrap_or(0)]);
                            }
                        }
                    }
                }
                true
            },
            (Scan::Item(item,currency),Some("item")) if number(&frame,"itemId")==Some(*item) &&
                frame.get("moeda").and_then(Value::as_str)==Some(currency.as_str()) => {
                if let Some(rows)=frame.get("linhas").and_then(Value::as_array) {
                    self.offers.retain(|o|!(o.kind=="item" && o.item_id==*item && o.currency==*currency));
                    for o in rows.iter().take(120) {
                        if let Some(o)=Offer::from_listing(o,"item",*item,currency,now){self.offers.push(o);}
                    }
                }
                true
            },
            (Scan::Pokemon(currency,page),Some("vitrine")) => {
                if let Some(rows)=pokemon_scan_rows(frame,currency,*page) {
                    if *page==0 {self.offers.retain(|o|!(o.kind=="pokemon"&&o.currency==*currency));}
                    for row in rows.iter().take(120) {
                        if let Some(offer)=Offer::from_listing(row,"pokemon",0,currency,now) {
                            self.offers.push(offer);
                        }
                    }
                    true
                } else {false}
            },
            _=>false
        };
        if matched {
            // Strictly scoped, authenticated list replies are the only
            // automatic Snatch discovery inputs. Held offers are retained
            // independently of the normal 60-second quote cache.
            let observed=match &scan {
                Scan::Item(item,currency)=>self.offers.iter()
                    .filter(|o|o.kind=="item"&&o.item_id==*item&&o.currency==*currency)
                    .cloned().collect::<Vec<_>>(),
                Scan::Pokemon(currency,_)=>self.offers.iter()
                    .filter(|o|o.kind=="pokemon"&&o.currency==*currency)
                    .cloned().collect::<Vec<_>>(),
                _=>vec![],
            };
            for offer in observed {self.snatch_record(offer,cfg,now);}
        }
        if !matched {self.pending_scan=Some((scan,requested_at));}
        else {
            if let Scan::Item(id,currency)=&scan {
                self.urgent_details.remove(&format!("item:{id}:{currency}"));
            }
            self.status.last_scan=Some(now);
        }
    }
    fn used(&self, currency: Option<&str>, rule_id: Option<&str>, period: &str) -> (u64,u64,u64) {
        let mut out=(0u64,0u64,0u64);
        for o in &self.journal.orders {
            if o.status=="refused" || currency.is_some_and(|c|o.currency!=c) ||
                rule_id.is_some_and(|r|o.rule_id!=r) ||
                (period=="day"&&o.day!=today()) || (period=="session"&&o.session!=self.session) {continue}
            out.0=out.0.saturating_add(o.spend);
            out.1=out.1.saturating_add(1);
            out.2=out.2.saturating_add(o.quantity);
        }
        for a in &self.journal.archived {
            if currency.is_some_and(|c|a.currency!=c) ||
                rule_id.is_some_and(|r|a.rule_id!=r) ||
                (period=="day"&&a.day!=today()) ||
                (period=="session"&&a.session!=self.session) {continue}
            out.0=out.0.saturating_add(a.spend);
            out.1=out.1.saturating_add(a.count);
            out.2=out.2.saturating_add(a.quantity);
        }
        out
    }
    fn budget_for(&self,cfg:&MarketRules,rule:&BuyRule,offer:&Offer,
        competitive:bool)->u64 {
        let currency=offer.currency.as_str();
        let i=if currency=="orb"{1}else{0};
        // Ordinary purchases reserve every outstanding attempt. In Snatch
        // only competitive retries of the exact same listing reuse its
        // reservation; different listings reserve independently. All sends
        // remain in the journal for the configured spending caps.
        let in_flight=self.pending.iter().filter_map(|i|self.journal.orders.get(*i))
            .filter(|o|o.currency==currency &&
                !(competitive && o.listing_id==offer.id))
            .fold(0u64,|total,o|total.saturating_add(o.spend));
        // Reserve pending spend against the last balance snapshot to prevent
        // a five-order burst from committing more than the account can pay.
        let available=self.balances[i].unwrap_or(0)
            .saturating_sub(if i==0{cfg.gold_reserve}else{cfg.gem_reserve})
            .saturating_sub(in_flight);
        let mut max=available;
        let spend_session=self.used(Some(currency),None,"session").0;
        let spend_day=self.used(Some(currency),None,"day").0;
        let rule_session=self.used(Some(currency),Some(&rule.id),"session");
        let rule_day=self.used(Some(currency),Some(&rule.id),"day");
        let total_session=self.used(None,None,"session");
        for (cap,used) in [
            (if i==0{cfg.max_gold_per_buy}else{cfg.max_gems_per_buy},0),
            (if i==0{cfg.max_gold_session}else{cfg.max_gems_session},spend_session),
            (if i==0{cfg.max_gold_day}else{cfg.max_gems_day},spend_day),
            (rule.max_spend_session,rule_session.0),(rule.max_spend_day,rule_day.0)
        ] {
            if cap>0{max=max.min(cap.saturating_sub(used));}
        }
        if (cfg.max_purchases_session>0&&total_session.1>=cfg.max_purchases_session)||
            (rule.max_purchases_session>0&&rule_session.1>=rule.max_purchases_session){return 0}
        let mut qty=max/offer.price;
        if cfg.max_quantity_session>0 {qty=qty.min(cfg.max_quantity_session.saturating_sub(total_session.2));}
        if rule.max_quantity_session>0 {qty=qty.min(rule.max_quantity_session.saturating_sub(rule_session.2));}
        // Community Market listings have no verified 5,000-unit maximum.
        // Unlike NPC shop.buy, request the full affordable listing quantity.
        if offer.kind=="pokemon" {qty.min(1)} else {qty.min(offer.qty)}
    }
    pub fn next(&mut self,cfg:&MarketRules,now:Instant) -> Option<Value> {
        self.status.enabled=cfg.enabled&&!cfg.paused;
        // Time-based cache cleanup is maintenance, not a Snatch deadline.
        // Avoid scanning every offer and blocked listing every 60 ms.
        if self.last_housekeeping.is_none_or(|at|
            now.saturating_duration_since(at)>=Duration::from_secs(1)) {
            self.offers.retain(|offer|now.saturating_duration_since(offer.at)<Duration::from_secs(60));
            self.blocked.retain(|_,at|now.saturating_duration_since(*at)<Duration::from_secs(3600));
            self.last_housekeeping=Some(now);
        }
        // Pausing buying never freezes the receipt/timeout state machine.
        self.expire_pending(now);
        if self.failed_closed || !self.status.enabled || self.nick.is_empty() ||
            self.backoff_until.is_some_and(|until|now<until) ||
            self.balances.iter().any(Option::is_none) || !cfg.rules.iter().any(|r|r.enabled) {return None}
        if self.snatch_critical(cfg) {return None}
        if let Some(buy)=self.next_ready_buy(cfg,now) {return Some(buy)}
        // Preserve one outstanding scan for response attribution. Pokémon
        // listing replies use the shorter, independently verified Watch wait.
        if self.pending_scan.as_ref().is_some_and(|(scan,at)| {
            let timeout=if matches!(scan,Scan::Pokemon(..)) {
                Duration::from_millis(2700)
            } else {Duration::from_secs(7)};
            now.saturating_duration_since(*at)<timeout
        }) {return None}
        self.pending_scan=None;
        // Only active Pokémon buying uses the faster rate; other market
        // scanning retains its conservative two-second request spacing.
        let pokemon_buying=cfg.rules.iter().any(|r|
            r.enabled && r.buy_max>0 && r.kind.starts_with("pokemon_"));
        let gap=if pokemon_buying {Duration::from_millis(1250)}
            else {Duration::from_secs(2)};
        if self.last_scan.values().any(|at|now.saturating_duration_since(*at)<gap){return None}
        let mut targets=Vec::<(String,Scan,Value)>::new();
        if cfg.rules.iter().any(|r|r.enabled&&r.kind=="any_item") {
            targets.push(("summary".into(),Scan::Summary,json!({"t":"market.itens"})));
        }
        for r in cfg.rules.iter().filter(|r|r.enabled&&r.kind=="item") {
            let key=format!("item:{}:{}",r.item_id,r.currency);
            if !targets.iter().any(|(k,_,_)|k==&key) {
                targets.push((key,Scan::Item(r.item_id,r.currency.clone()),
                    json!({"t":"market.item","itemId":r.item_id,"moeda":r.currency})));
            }
        }
        // Broad rules nominate item types using server summary minima.
        // Check the cheapest qualified types first, not the smallest item ID.
        // Offer details, never summary minima, authorize actual purchases.
        let mut nominated=Vec::new();
        for (id,minima) in &self.summary {
            for (index,currency) in ["gold","orb"].iter().enumerate() {
                let minimum=minima[index];
                if minimum==0 || !cfg.rules.iter().any(|r|
                    r.enabled&&r.kind=="any_item"&&r.currency==*currency&&minimum<=r.buy_max) {continue}
                nominated.push((minimum,*id,*currency));
            }
        }
        nominated.sort_unstable();
        for (_,id,currency) in nominated {
            let key=format!("item:{id}:{currency}");
            if !targets.iter().any(|(k,_,_)|k==&key) {
                targets.push((key,Scan::Item(id,currency.into()),
                    json!({"t":"market.item","itemId":id,"moeda":currency})));
            }
        }
        for currency in ["gold","orb"] {
            if cfg.rules.iter().any(|r|r.enabled&&r.kind.starts_with("pokemon_")&&r.currency==currency){
                for page in 0..3 {
                    let key=format!("pokemon:{currency}:{page}");
                    let mut payload=json!({"t":"market.listar","tipo":"pokemon","moeda":currency,
                        "busca":"","ordem":"baratos","criterios":[],"elemento":"","categoria":"",
                        "soShiny":false,"soP5":false,"soTmElemental":false,"soTmAoe":false,
                        "semOutland":false,"nivelMin":"","nivelMax":"","potenciaMin":"",
                        "potenciaMax":"","ivMin":"","ivMax":"","qualidadeMin":"","qualidadeMax":"",
                        "notaMin":"","notaMax":"","pagina":page,"especieId":0});
                    targets.push((key,Scan::Pokemon(currency.into(),page),std::mem::take(&mut payload)));
                }
            }
        }
        let fixed=cfg.rules.iter().filter(|r|r.enabled&&r.kind=="item")
            .map(|r|format!("item:{}:{}",r.item_id,r.currency))
            .collect::<HashSet<_>>();
        // Watch gives explicitly configured item rules priority over the
        // rotating broad-item pool. Avoid a huge any-item shortlist starving
        // the user's fixed rules, Pokémon scans and post-buy quote refreshes.
        let slot=self.scan_sequence%6;
        let (key,scan,payload)=targets.into_iter()
            .filter(|(key,_,_)|self.urgent_details.contains(key) ||
                self.last_scan.get(key).is_none_or(|at| {
                    let ttl=if key=="summary" {15} else if key.starts_with("pokemon:") {
                        if key.ends_with(":0") {6} else {40}
                    } else {25};
                    now.saturating_duration_since(*at)>=Duration::from_secs(ttl)
                }))
            .min_by_key(|(key,_,_)| {
                // Page zero contains the cheapest current Pokémon and is
                // where we discover held listings early enough to Snatch.
                // Deep pages remain sampled but cannot delay the front page
                // until their 40-second refresh comes due.
                let front_page=key.starts_with("pokemon:") && key.ends_with(":0");
                let tier=if self.urgent_details.contains(key) || front_page {0}
                    else if (slot<=2 && fixed.contains(key)) ||
                            (slot==3 && key=="summary") ||
                            (slot==4 && key.starts_with("pokemon:")) ||
                            (slot==5 && !fixed.contains(key) &&
                                key!="summary" && !key.starts_with("pokemon:")) {1}
                    else {2};
                (tier,self.last_scan.get(key).copied())
            })?;
        self.scan_sequence=self.scan_sequence.wrapping_add(1);
        self.last_scan.insert(key,now);self.pending_scan=Some((scan,now));
        Some(payload)
    }

    // Called both by the normal worker tick and immediately after a matched
    // market response, so a fresh bargain does not wait for another socket
    // read timeout or UI/history processing before submitting the buy.
    pub fn next_ready_buy(&mut self,cfg:&MarketRules,now:Instant)->Option<Value> {
        if self.failed_closed || !cfg.enabled || cfg.paused ||
            self.nick.is_empty() || self.balances.iter().any(Option::is_none) ||
            self.backoff_until.is_some_and(|until|now<until) ||
            self.snatch_critical(cfg) {return None}
        let server_ms=(clock_ms() as i64-self.server_offset_ms).max(0) as u64;
        // Multiple unrelated orders may be in flight, but never duplicate a
        // unique Pokémon order awaiting a server receipt.
        if self.pending.len()>=5 ||
            self.last_buy.is_some_and(|at|
                now.saturating_duration_since(at)<Duration::from_millis(120)) {return None}
        for rule in cfg.rules.iter().filter(|r|r.enabled&&r.buy_max>0) {
            let selected=self.offers.iter()
                .filter(|o|o.currency==rule.currency&&o.matches(rule,&self.nick,server_ms))
                .filter(|o|!self.blocked.get(&o.id).is_some_and(|until|now<*until))
                .filter(|o|self.pending.iter().filter(|i|self.journal.orders.get(**i)
                    .is_some_and(|x|x.listing_id==o.id)).count()<if o.kind=="item"{3}else{1})
                .filter(|o|o.kind=="item" || !self.journal.orders.iter().any(|x|
                    x.listing_id==o.id && x.status!="refused"))
                .min_by_key(|o|(o.price,o.id)).cloned();
            let Some(offer)=selected else {continue};
            if let Some(payload)=self.buy_quote(cfg,rule,&offer,now,false) {
                return Some(payload);
            }
        }
        None
    }

    /// Shared purchase journal and global caps: Snatch is never a competing
    /// WebSocket and never bypasses explicit account budgets / price ceilings.
    fn buy_quote(&mut self, cfg:&MarketRules, rule:&BuyRule,
        offer:&Offer, now:Instant, snatching:bool)->Option<Value> {
        let qty=self.budget_for(cfg,rule,offer,snatching);
        if qty==0 {return None}
        let spend=offer.price.checked_mul(qty)?;
        self.compact();
        if self.journal.orders.len()>=20_000 {
            self.failed_closed=true;
            self.status.error=Some("Market journal is full".into());
            return None;
        }
        self.journal.orders.push(Order {
            listing_id:offer.id, rule_id:rule.id.clone(),
            currency:offer.currency.clone(),kind:offer.kind.clone(),
            quantity:qty,spend,day:today(),session:self.session,
            status:"submitted".into(),sent_ms:clock_ms()
        });
        // Normal rules persist each order. Snatch builds a burst and commits
        // its complete journal once, before the native worker sends any
        // packet; this avoids eight full JSON rewrites in the 4s lottery.
        if !snatching && !self.persist(){return None}
        self.pending.push_back(self.journal.orders.len()-1);
        self.status.sent=self.status.sent.saturating_add(1);
        self.last_buy=Some(now);
        if snatching {
            self.status.snatch_sent=self.status.snatch_sent.saturating_add(1);
            self.snatch_last_send.insert(offer.id,now);
            if let Some(s)=self.snatch.get_mut(&offer.id) {
                s.attempts=s.attempts.saturating_add(1);
                s.retry_at=Some(now+Duration::from_millis(150));
            }
            self.status.last=format!("Snatch #{} · {} units at {} {}",
                offer.id,qty,offer.price,offer.currency);
        } else {
            self.blocked.insert(offer.id,now+Duration::from_millis(400));
            self.status.last=format!("Rule {}: sent {}x listing #{}",rule.id,qty,offer.id);
        }
        self.update_pending();
        Some(json!({"t":"market.comprar","id":offer.id,
            "qtd":qty,"preco":offer.price,"moeda":offer.currency}))
    }
    fn snatch_rule<'a>(&self,cfg:&'a MarketRules,offer:&Offer,server_ms:u64)
        ->Option<&'a BuyRule> {
        cfg.rules.iter().find(|r|r.enabled&&r.buy_max>0&&
            r.currency==offer.currency &&
            // A held listing matches at its release timestamp; ordinary
            // buying still rejects it until the actual server clock reaches it.
            // The authenticated pre-unlock quote is intentionally retained
            // for the four-second lottery, even when older than 30 seconds.
            { let mut retained=offer.clone(); retained.at=Instant::now();
              retained.matches(r,&self.nick,server_ms.max(offer.held_until)) })
    }
    fn snatch_record(&mut self,offer:Offer,cfg:&MarketRules,now:Instant) {
        if !matches!(offer.kind.as_str(),"item"|"pokemon") ||
            offer.seller.eq_ignore_ascii_case(&self.nick) ||
            self.snatch_closed.contains_key(&offer.id) {return}
        let manual=cfg.snatch_targets.iter().find(|t|t.id==offer.id &&
            t.kind==offer.kind && t.currency==offer.currency &&
            (t.kind!="item"||t.item_id==offer.item_id) &&
            offer.price<=t.max_price);
        let server_ms=(clock_ms() as i64-self.server_offset_ms).max(0) as u64;
        let rule=self.snatch_rule(cfg,&offer,server_ms);
        if manual.is_none() && !(offer.held_until>server_ms && rule.is_some()) &&
            !self.snatch.contains_key(&offer.id) {return}
        if let Some(t)=cfg.snatch_targets.iter().find(|t|t.id==offer.id) {
            if manual.is_none() {
                self.snatch_closed.insert(offer.id,"Quote exceeds authorized Snatch price or identity".into());
                self.snatch.remove(&offer.id);
                return;
            }
            if t.expires_at.max(offer.held_until.saturating_add(30*60_000))<clock_ms() {
                return;
            }
        }
        let id=offer.id;
        if !self.snatch.contains_key(&id) && self.snatch.len()>=40 {
            // Match Moth Watch's bounded observation set. Never displace a
            // player-armed target for incidental rule-based discovery.
            let evict=self.snatch.keys().copied().find(|other|
                !cfg.snatch_targets.iter().any(|t|t.id==*other));
            if let Some(evict)=evict {self.snatch.remove(&evict);}
            else {return}
        }
        let old=self.snatch.remove(&id);
        if let Some(previous)=old.as_ref() {
            if previous.offer.at>offer.at {
                self.snatch.insert(id,previous.clone());
                return;
            }
        }
        let release_at=if offer.held_until>0 {offer.held_until}
            else {old.as_ref().map(|o|o.release_at).unwrap_or(0)};
        self.snatch.insert(id,SnatchEntry {
            offer,rule_id:rule.map(|r|r.id.clone())
                .or_else(||old.as_ref().and_then(|v|v.rule_id.clone())),
            attempts:old.as_ref().map(|v|v.attempts).unwrap_or(0),
            retry_at:old.as_ref().and_then(|v|v.retry_at),
            last_verified:old.as_ref().and_then(|v|v.last_verified),
            needs_verify:old.as_ref().is_some_and(|v|v.needs_verify),
            release_at,
        });
        // No optimistic verification based on an ordinary summary.
        let _=now;
    }
    fn snatch_reconcile(&mut self,id:u64,success:bool,now:Instant) {
        if let Some(entry)=self.snatch.get_mut(&id) {
            if success && entry.offer.kind=="pokemon" {
                self.snatch.remove(&id);
                self.snatch_closed.insert(id,"Purchased".into());
            } else {
                entry.needs_verify=true;
                entry.retry_at=Some(now+Duration::from_millis(150));
            }
        }
    }
    fn snatch_exact_response(&mut self,frame:&Value,cfg:&MarketRules,now:Instant) {
        let Some(id)=number(frame,"id") else{return};
        if self.snatch_verifying.remove(&id).is_none(){return}
        let manual=cfg.snatch_targets.iter().find(|t|t.id==id);
        let previous=self.snatch.get(&id);
        let Some(raw)=frame.get("anuncio").filter(|v|v.is_object()) else {
            self.snatch.remove(&id);
            self.snatch_closed.insert(id,"Listing sold, closed or missing".into());
            return;
        };
        let kind=if raw.get("ficha").is_some() {"pokemon"}
            else {raw.get("tipo").and_then(Value::as_str)
                .or_else(||manual.map(|t|t.kind.as_str()))
                .or_else(||previous.map(|e|e.offer.kind.as_str()))
                .unwrap_or("")};
        let currency=raw.get("moeda").and_then(Value::as_str)
            .or_else(||manual.map(|t|t.currency.as_str()))
            .or_else(||previous.map(|e|e.offer.currency.as_str()))
            .unwrap_or("");
        let item_id=number(raw,"itemId")
            .or_else(||manual.map(|t|t.item_id))
            .or_else(||previous.map(|e|e.offer.item_id)).unwrap_or(0);
        let offer=Offer::from_listing(raw,kind,item_id,currency,now);
        if let Some(offer)=offer.filter(|o|o.id==id) {
            let manual_matches=manual.is_none_or(|t|
                t.kind==offer.kind && t.currency==offer.currency &&
                (t.kind!="item"||t.item_id==offer.item_id) &&
                offer.price<=t.max_price);
            if manual_matches && !self.nick.is_empty() &&
                !offer.seller.eq_ignore_ascii_case(&self.nick) {
                self.snatch_record(offer,cfg,now);
                if let Some(entry)=self.snatch.get_mut(&id) {
                    entry.last_verified=Some(now);
                    entry.needs_verify=false;
                }
                return;
            }
        }
        self.snatch.remove(&id);
        self.snatch_closed.insert(id,"Listing invalid, changed or not available".into());
    }
    fn snatch_sync(&mut self,cfg:&MarketRules,now:Instant) {
        let now_ms=clock_ms();
        let server_ms=(now_ms as i64-self.server_offset_ms).max(0) as u64;
        let manual=cfg.snatch_targets.iter().map(|t|t.id).collect::<HashSet<_>>();
        self.snatch.retain(|id,e|{
            let selected=cfg.snatch_targets.iter().find(|t|t.id==*id);
            selected.is_some_and(|t|t.expires_at.max(e.release_at.saturating_add(1_800_000))>now_ms) ||
                (e.rule_id.as_deref().is_some_and(|r|cfg.rules.iter().any(|rule|
                    rule.id==r && rule.enabled && rule.buy_max>=e.offer.price &&
                    rule.currency==e.offer.currency &&
                    (rule.kind!="item" || rule.item_id==e.offer.item_id))) &&
                    e.release_at.saturating_add(1_800_000)>server_ms)
        });
        self.snatch_closed.retain(|id,_|manual.contains(id));
        let active=self.snatch.keys().copied().chain(manual.iter().copied())
            .collect::<HashSet<_>>();
        self.snatch_last_send.retain(|id,_|active.contains(id));
        self.snatch_last_verify.retain(|id,_|active.contains(id));
        // Held rule matches are recorded even if no explicit player target exists.
        for offer in self.offers.iter().filter(|o|o.held_until>server_ms).cloned()
            .collect::<Vec<_>>() {
            self.snatch_record(offer,cfg,now);
        }
        let mut rows=Vec::new();
        for t in &cfg.snatch_targets {
            let entry=self.snatch.get(&t.id);
            let phase=if t.expires_at<now_ms &&
                entry.is_none_or(|e|e.release_at.saturating_add(1_800_000)<now_ms) {
                    "Expired · cancel or re-arm".into()
                } else if let Some(reason)=self.snatch_closed.get(&t.id){reason.clone()}
                else if let Some(e)=entry {
                    if e.release_at>server_ms {format!("Held · {}s to unlock",
                        (e.release_at-server_ms).div_ceil(1000))}
                    else if e.release_at>0 && server_ms<e.release_at+4000 {"Snatch burst".into()}
                    else if e.needs_verify || e.last_verified.is_none_or(|at|
                        now.saturating_duration_since(at)>Duration::from_secs(15)) {
                        "Checking exact listing".into()
                    }
                    else {"Competing · verified quote".into()}
                } else {"Recovering exact listing".into()};
            rows.push(SnatchRow {id:t.id,kind:t.kind.clone(),currency:t.currency.clone(),
                item_id:t.item_id,
                price:entry.map(|e|e.offer.price).unwrap_or(t.max_price),
                quantity:entry.map(|e|e.offer.qty).unwrap_or(0),
                release_at:entry.map(|e|e.release_at).unwrap_or(0),
                attempts:entry.map(|e|e.attempts).unwrap_or(0),
                manual:true,phase});
        }
        for (id,e) in &self.snatch {
            if manual.contains(id){continue}
            rows.push(SnatchRow{id:*id,kind:e.offer.kind.clone(),
                currency:e.offer.currency.clone(),item_id:e.offer.item_id,price:e.offer.price,
                quantity:e.offer.qty,release_at:e.release_at,attempts:e.attempts,
                manual:false,phase:if e.release_at>server_ms {
                    format!("Held · {}s to unlock",(e.release_at-server_ms).div_ceil(1000))
                } else if e.release_at>0 && server_ms<e.release_at+4000 {
                    "Snatch burst".into()
                } else {"Rule-triggered Snatch".into()}});
        }
        rows.sort_by_key(|r|(if r.release_at==0 {u64::MAX}else{r.release_at},r.id));
        rows.truncate(80);
        self.status.snatches=rows;
        // Offer discovery is a read-only UI feed. Merely displaying a quote
        // must never authorize Snatch; the player must explicitly arm it or
        // an enabled rule must match it.
        let mut seen=HashSet::new();
        let mut held=self.offers.iter().filter(|o|
            o.held_until>server_ms && o.price>0 && o.qty>0 &&
            !o.seller.eq_ignore_ascii_case(&self.nick) && seen.insert(o.id))
            .map(|o|SnatchRow {
                id:o.id,kind:o.kind.clone(),currency:o.currency.clone(),
                item_id:o.item_id,price:o.price,quantity:o.qty,
                release_at:o.held_until,attempts:0,manual:false,
                phase:format!("Unlocks in {}s",
                    (o.held_until-server_ms).div_ceil(1000))
            }).collect::<Vec<_>>();
        held.sort_by_key(|r|(r.release_at,r.id));
        held.truncate(40);
        self.status.held_offers=held;
    }
    pub fn snatch_critical(&self,cfg:&MarketRules)->bool {
        if !cfg.enabled || cfg.paused {return false}
        let server_ms=(clock_ms() as i64-self.server_offset_ms).max(0) as u64;
        self.snatch.values().any(|e|e.release_at>0 &&
            e.release_at<=server_ms.saturating_add(2_000) &&
            server_ms<e.release_at.saturating_add(4_000))
    }
    /// One worker iteration can dispatch up to eight eligible offers during
    /// the critical window. First attempts across listings precede retries.
    /// Verification requests are independent of the background market scanner.
    pub fn snatch_actions(&mut self,cfg:&MarketRules,now:Instant)->Vec<Value> {
        self.snatch_sync(cfg,now);
        self.snatch_verifying.retain(|_,sent|now.saturating_duration_since(*sent)<Duration::from_secs(4));
        if self.failed_closed || !cfg.enabled || cfg.paused ||
            self.nick.is_empty() || self.balances.iter().any(Option::is_none) ||
            self.backoff_until.is_some_and(|until|now<until) {return Vec::new()}
        let server_ms=(clock_ms() as i64-self.server_offset_ms).max(0) as u64;
        let mut ordered=self.snatch.keys().copied().collect::<Vec<_>>();
        ordered.sort_by_key(|id|{
            let e=&self.snatch[id];
            (e.attempts,e.release_at,*id)
        });
        let mut out=Vec::new();
        for id in &ordered {
            if out.len()>=8 {break}
            let Some(entry)=self.snatch.get(id).cloned() else {continue};
            if entry.release_at>server_ms || self.snatch_closed.contains_key(id){continue}
            if entry.retry_at.is_some_and(|at|now<at) ||
                self.snatch_last_send.get(id).is_some_and(|at|
                    now.saturating_duration_since(*at)<Duration::from_millis(150)) {continue}
            let burst=entry.release_at>0 && server_ms<entry.release_at.saturating_add(4_000);
            // Held quote remains usable for the release lottery; after the
            // four-second window require fresh exact market.anuncio evidence.
            if !burst && (entry.needs_verify ||
                entry.last_verified.is_none_or(|at|
                    now.saturating_duration_since(at)>Duration::from_secs(15))) {continue}
            let manual=cfg.snatch_targets.iter().find(|t|t.id==*id &&
                t.kind==entry.offer.kind && t.currency==entry.offer.currency &&
                t.max_price>=entry.offer.price &&
                (t.kind!="item"||t.item_id==entry.offer.item_id));
            let rule=if manual.is_none() {
                self.snatch_rule(cfg,&entry.offer,server_ms).cloned()
            } else {None};
            let synthetic=manual.map(|m|BuyRule {
                id:format!("snatch-manual-{}",m.id), kind:if m.kind=="pokemon" {
                    "pokemon_any".into()
                } else {"item".into()},
                item_id:m.item_id,currency:m.currency.clone(),enabled:true,
                buy_max:m.max_price,..Default::default()
            });
            let selected=rule.as_ref().or(synthetic.as_ref());
            if let Some(rule)=selected {
                if let Some(buy)=self.buy_quote(cfg,rule,&entry.offer,now,true) {
                    out.push(buy);
                }
            }
        }
        // Recover manually armed unknown IDs; verify 45s before unlock, and
        // probe exact listing status every 1.5s when no authoritative quote.
        for target in &cfg.snatch_targets {
            if target.expires_at<clock_ms() &&
                self.snatch.get(&target.id).is_none_or(|e|
                    e.release_at.saturating_add(1_800_000)<clock_ms()) {continue}
            if out.len()>=8 || self.snatch_verifying.len()>=8 {break}
            if self.snatch_closed.contains_key(&target.id){continue}
            let entry=self.snatch.get(&target.id);
            let due=entry.is_none_or(|e| {
                let warm=e.release_at==0 || e.release_at<=server_ms.saturating_add(45_000);
                let before_burst=e.release_at>server_ms;
                warm && (e.needs_verify ||
                    (before_burst && e.last_verified.is_none_or(|at|
                        now.saturating_duration_since(at)>Duration::from_secs(15))) ||
                    (!before_burst && e.release_at.saturating_add(4_000)<=server_ms &&
                        e.last_verified.is_none_or(|at|
                            now.saturating_duration_since(at)>Duration::from_secs(15))))
            });
            if !due || self.snatch_verifying.contains_key(&target.id) ||
                self.snatch_last_verify.get(&target.id).is_some_and(|at|
                    now.saturating_duration_since(*at)<Duration::from_millis(1500)) {continue}
            self.snatch_verifying.insert(target.id,now);
            self.snatch_last_verify.insert(target.id,now);
            out.push(json!({"t":"market.anuncio","id":target.id}));
        }
        // Also revalidate rule-owned held listings near unlock or after the
        // release burst. Never force exact lookups on every background scan.
        for id in ordered {
            if out.len()>=8 || self.snatch_verifying.len()>=8 {break}
            if self.snatch_verifying.contains_key(&id) ||
                self.snatch_closed.contains_key(&id) {continue}
            let Some(e)=self.snatch.get(&id) else {continue};
            let warm=e.release_at>server_ms &&
                e.release_at<=server_ms.saturating_add(45_000);
            let late=e.release_at<=server_ms && e.release_at.saturating_add(4_000)<=server_ms;
            if !(warm || late) ||
                (!e.needs_verify && e.last_verified.is_some_and(|at|
                    now.saturating_duration_since(at)<Duration::from_secs(15))) ||
                self.snatch_last_verify.get(&id).is_some_and(|at|
                    now.saturating_duration_since(*at)<Duration::from_millis(1500)) {continue}
            self.snatch_verifying.insert(id,now);
            self.snatch_last_verify.insert(id,now);
            out.push(json!({"t":"market.anuncio","id":id}));
        }
        // Batched durability precedes all network writes; if disk persistence
        // fails, issue none of the prepared buys (including exact lookups).
        if out.iter().any(|packet|packet.get("t").and_then(Value::as_str)==Some("market.comprar")) &&
            !self.persist() {return Vec::new()}
        out
    }
    pub fn send_failed(&mut self) {
        // The purchase was journaled first. A broken socket cannot prove it
        // failed, so every in-flight attempt becomes unknown exposure.
        self.on_disconnect();
        self.status.error=Some("Market dispatch failed; order outcome unknown".into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn native_snatch_requires_explicit_listing_identity_and_price() {
        let mut t=SnatchTarget {kind:"item".into(),currency:"gold".into(),
            id:317,item_id:42,max_price:120,..Default::default()};
        assert!(!t.valid());
        t.arm();
        assert!(t.valid());
        let mut cfg=MarketRules {snatch_targets:vec![t.clone()],..Default::default()};
        assert!(cfg.validate().is_ok());
        cfg.snatch_targets.push(t.clone());
        assert!(cfg.validate().is_err());
        cfg.snatch_targets.pop();
        cfg.snatch_targets[0].max_price=0;
        assert!(cfg.validate().is_err());
    }
    #[test] fn held_rule_listing_is_prepared_before_unlock_and_survives_cache_age() {
        let mut buyer=MarketBuyer::new(GameProfile::Game1);
        buyer.nick="Mine".into();
        let now=Instant::now();
        let release=clock_ms()+120_000;
        let offer=Offer::from_listing(&json!({"id":341,"preco":35,"qtd":4,
            "vendedor":"Other","compravelEm":release,"estado":"aberto",
            "tipo":"item","itemId":42,"moeda":"gold"}),"item",42,"gold",now).unwrap();
        let cfg=MarketRules {enabled:true,rules:vec![BuyRule {
            id:"auto".into(),kind:"item".into(),item_id:42,
            currency:"gold".into(),buy_max:45,enabled:true,..Default::default()
        }],..Default::default()};
        buyer.snatch_record(offer.clone(),&cfg,now);
        assert!(buyer.snatch.contains_key(&341));
        // Rule matching for release does not silently expire the verified
        // pre-unlock lot when its regular background cache is older than 30s.
        let mut earlier=offer;
        earlier.at=now-Duration::from_secs(70);
        assert!(buyer.snatch_rule(&cfg,&earlier,release).is_some());
    }
    #[test] fn exact_listing_lookup_rejects_closed_and_changed_quotes() {
        let mut buyer=MarketBuyer::new(GameProfile::Game1);
        buyer.nick="mine".into();
        let mut t=SnatchTarget{id:291,kind:"item".into(),currency:"gold".into(),
            item_id:42,max_price:100,..Default::default()};
        t.arm();
        let cfg=MarketRules{enabled:true,snatch_targets:vec![t],..Default::default()};
        let now=Instant::now();
        buyer.snatch_verifying.insert(291,now);
        buyer.snatch_exact_response(&json!({"t":"market","aba":"anuncio","id":291,
            "anuncio":{"id":291,"itemId":42,"tipo":"item","moeda":"gold",
                "estado":"aberto","qtd":5,"preco":99,"vendedor":"seller"}}),&cfg,now);
        assert!(buyer.snatch.get(&291).is_some());
        assert!(buyer.snatch.get(&291).unwrap().last_verified.is_some());
        buyer.snatch_verifying.insert(291,now);
        buyer.snatch_exact_response(&json!({"t":"market","aba":"anuncio","id":291,
            "anuncio":{"id":291,"itemId":42,"tipo":"item","moeda":"gold",
                "estado":"aberto","qtd":5,"preco":101,"vendedor":"seller"}}),&cfg,now);
        assert!(!buyer.snatch.contains_key(&291));
        assert!(buyer.snatch_closed.contains_key(&291));
        buyer.snatch_verifying.insert(292,now);
        buyer.snatch_exact_response(&json!({"t":"market","aba":"anuncio","id":292,
            "anuncio":null}),&cfg,now);
        assert!(buyer.snatch_closed.contains_key(&292));
    }
    #[test] fn manual_unknown_listing_is_looked_up_before_buying() {
        let mut buyer=MarketBuyer::new(GameProfile::Game4);
        buyer.nick="me".into(); buyer.balances=[Some(5000),Some(100)];
        let mut target=SnatchTarget{id:991,kind:"item".into(),currency:"gold".into(),
            item_id:42,max_price:100,..Default::default()};
        target.arm();
        let cfg=MarketRules{enabled:true,snatch_targets:vec![target],..Default::default()};
        let actions=buyer.snatch_actions(&cfg,Instant::now());
        assert_eq!(actions.len(),1);
        assert_eq!(actions[0],json!({"t":"market.anuncio","id":991}));
        assert!(buyer.snatch_verifying.contains_key(&991));
    }
    #[test]
    fn pokemon_reply_scope_accepts_proven_rows_but_not_ambiguous_pages() {
        let row=json!({"id":29,"preco":22,"qtd":1,"vendedor":"seller",
            "ficha":{"potencia":5,"shiny":false},"moeda":"gold"});
        let partial=json!({"t":"market","aba":"vitrine","linhas":[row.clone()]});
        assert!(pokemon_scan_rows(&partial,"gold",0).is_some());
        assert!(pokemon_scan_rows(&partial,"orb",0).is_none());
        assert!(pokemon_scan_rows(&partial,"gold",1).is_none());
        let explicit=json!({"t":"market","aba":"vitrine","tipo":"pokemon",
            "moeda":"gold","pagina":1,"linhas":[row.clone()]});
        assert!(pokemon_scan_rows(&explicit,"gold",1).is_some());
        assert!(pokemon_scan_rows(&explicit,"gold",0).is_none());
        let empty=json!({"t":"market","aba":"vitrine","linhas":[]});
        assert!(pokemon_scan_rows(&empty,"gold",0).is_none());
        let typed_empty=json!({"t":"market","aba":"vitrine","tipo":"pokemon",
            "moeda":"gold","pagina":0,"linhas":[]});
        assert!(pokemon_scan_rows(&typed_empty,"gold",0).is_some());
        let conflicting=json!({"t":"market","aba":"vitrine","tipo":"diamante",
            "moeda":"gold","pagina":0,"linhas":[row]});
        assert!(pokemon_scan_rows(&conflicting,"gold",0).is_none());
    }
    #[test]
    fn active_pokemon_rules_prioritize_fresh_front_page_scans() {
        let mut buyer=MarketBuyer::new(GameProfile::Game1);
        buyer.failed_closed=false;
        buyer.nick="me".into();
        buyer.balances=[Some(1000),Some(10)];
        let now=Instant::now();
        for page in 0..3 {
            buyer.last_scan.insert(format!("pokemon:gold:{page}"),
                now-Duration::from_secs(if page==0 {7} else {45}));
        }
        // Without front-page priority the oldest deep page wins here.
        let cfg=MarketRules{enabled:true,rules:vec![BuyRule{
            id:"pokemon-rule".into(),kind:"pokemon_any".into(),currency:"gold".into(),
            enabled:true,buy_max:100,..Default::default()
        }],..Default::default()};
        let scan=buyer.next(&cfg,now).unwrap();
        assert_eq!(scan["t"],"market.listar");
        assert_eq!(scan["pagina"],0);
        assert_eq!(scan["moeda"],"gold");
    }
    #[test] fn imported_watch_rules_are_disarmed_globally() {
        let rules=MarketRules::import_watch(r#"{"fixedRulesEnabled":true,"fixedItemRules":[{"itemId":42,"buyMax":100,"currency":"gold","buyEnabled":true}],"pokemonBuyRules":[{"kind":"shiny","buyMax":2,"currency":"gold"}]}"#).unwrap();
        assert!(!rules.enabled);assert_eq!(rules.rules.len(),1);assert!(rules.rules[0].enabled);
    }
    #[test] fn item_buy_uses_maximum_affordable_quantity_and_actual_listing() {
        let mut buyer=MarketBuyer::new(GameProfile::Game1);
        buyer.failed_closed=false;buyer.nick="Me".into();buyer.balances=[Some(1000),Some(0)];
        buyer.offers.push(Offer::from_listing(&json!({"id":9,"preco":20,"qtd":80,"vendedor":"Other"}),"item",42,"gold",Instant::now()).unwrap());
        let cfg=MarketRules{enabled:true,gold_reserve:100,rules:vec![BuyRule{id:"a".into(),kind:"item".into(),item_id:42,enabled:true,buy_max:25,..BuyRule::default()}],..MarketRules::default()};
        assert_eq!(buyer.budget_for(&cfg,&cfg.rules[0],&buyer.offers[0],false),45);
    }
    #[test] fn scope_conflicts_and_closed_listings_are_not_buyable() {
        let now=Instant::now();
        let base=json!({"id":5,"tipo":"item","itemId":42,"moeda":"gold",
            "estado":"aberto","preco":20,"qtd":5,"vendedor":"seller"});
        assert!(Offer::from_listing(&base,"item",42,"gold",now).is_some());
        for (field,value) in [
            ("tipo",json!("pokemon")),("itemId",json!(99)),
            ("moeda",json!("orb")),("estado",json!("vendido"))] {
            let mut wrong=base.clone();
            wrong[field]=value;
            assert!(Offer::from_listing(&wrong,"item",42,"gold",now).is_none());
        }
        // Older Moth Watch listing payloads omit some aliases; preserve them.
        let minimal=json!({"id":7,"preco":20,"qtd":5,"vendedor":"seller"});
        assert!(Offer::from_listing(&minimal,"item",42,"gold",now).is_some());
    }
    #[test] fn pending_spend_is_reserved_before_next_aggressive_buy() {
        let mut buyer=MarketBuyer::new(GameProfile::Game4);
        buyer.failed_closed=false;
        buyer.balances=[Some(1000),Some(0)];
        let now=Instant::now();
        let offer=Offer::from_listing(&json!({"id":17,"preco":10,"qtd":500,
            "vendedor":"seller"}),"item",42,"gold",now).unwrap();
        let rule=BuyRule{id:"test".into(),kind:"item".into(),item_id:42,
            buy_max:11,enabled:true,..Default::default()};
        let cfg=MarketRules{enabled:true,rules:vec![rule.clone()],..Default::default()};
        assert_eq!(buyer.budget_for(&cfg,&rule,&offer,false),100);
        buyer.journal.orders.push(Order{listing_id:17,rule_id:"test".into(),
            currency:"gold".into(),kind:"item".into(),quantity:60,spend:600,
            day:today(),session:buyer.session,status:"submitted".into(),
            sent_ms:clock_ms()});
        buyer.pending.push_back(0);
        assert_eq!(buyer.budget_for(&cfg,&rule,&offer,false),40);
    }
    #[test] fn journal_compaction_keeps_spend_limits_and_live_indices() {
        let mut buyer=MarketBuyer::new(GameProfile::Game4);
        buyer.journal=Journal::default();
        let now=clock_ms();
        for id in 0..2500 {
            buyer.journal.orders.push(Order {
                listing_id:id+1,rule_id:"r".into(),currency:"gold".into(),
                kind:"item".into(),quantity:2,spend:40,day:today(),session:buyer.session,
                status:if id==100 {"submitted"} else {"confirmed"}.into(),
                sent_ms:now,
            });
        }
        buyer.pending.push_back(100);
        buyer.compact();
        assert!(buyer.journal.orders.len()<2400);
        assert_eq!(buyer.pending.len(),1);
        assert_eq!(buyer.journal.orders[*buyer.pending.front().unwrap()].listing_id,101);
        assert_eq!(buyer.used(Some("gold"),Some("r"),"session"),(100_000,2500,5000));
    }
    #[test] fn missing_metadata_or_own_listing_not_buyable() {
        let now=Instant::now();
        let o=Offer::from_listing(&json!({"id":2,"preco":4,"qtd":5,"vendedor":"me"}),"item",4,"gold",now).unwrap();
        let r=BuyRule{kind:"item".into(),item_id:4,currency:"gold".into(),buy_max:6,..Default::default()};
        assert!(!o.matches(&r,"ME",clock_ms()));
        assert!(Offer::from_listing(&json!({"id":1,"preco":3,"qtd":3}),"item",4,"gold",now).is_none());
    }
}
