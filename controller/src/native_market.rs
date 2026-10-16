//! Native Moth Watch-style rule buyer. All requests use the authenticated game
//! WebSocket already owned by Moth Controller; no browser/userscript is required.
//! Settings and a bounded conservative spending journal are profile-local.
use crate::config::{self, GameProfile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
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
    let contents = serde_json::to_vec_pretty(obj).map_err(|e| e.to_string())?;
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
}
impl Default for MarketRules {
    fn default() -> Self {
        Self { enabled: false, paused: false, gold_reserve: 0, gem_reserve: 0,
            max_gold_per_buy: 0, max_gems_per_buy: 0, max_gold_session: 0,
            max_gems_session: 0, max_gold_day: 0, max_gems_day: 0,
            max_purchases_session: 0, max_quantity_session: 0, rules: vec![] }
    }
}
impl MarketRules {
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
#[derive(Clone, Debug, Default)]
pub struct MarketStatus {
    pub enabled: bool, pub pending: Option<String>, pub sent: u64,
    pub confirmed: u64, pub unknown: u64, pub last: String, pub error: Option<String>,
    pub last_scan: Option<Instant>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Order {
    listing_id: u64, rule_id: String, currency: String, kind: String,
    quantity: u64, spend: u64, day: u64, session: u64, status: String,
    sent_ms: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Journal { orders: Vec<Order> }
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
pub struct MarketBuyer {
    profile: GameProfile, session: u64, journal: Journal,
    pub status: MarketStatus, nick: String, balances: [Option<u64>;2],
    server_offset_ms: i64, offers: Vec<Offer>, summary: BTreeMap<u64,[u64;2]>,
    pending: Option<usize>, pending_scan: Option<(Scan,Instant)>,
    last_scan: HashMap<String,Instant>, last_buy: Option<Instant>,
    blocked: HashMap<u64, Instant>, backoff_until: Option<Instant>, failed_closed: bool,
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
        Self { profile, session: clock_ms(), journal, status: MarketStatus {error:error.clone(),..Default::default()},
            nick:String::new(), balances:[None,None], server_offset_ms:0, offers:vec![],
            summary:BTreeMap::new(),pending:None,pending_scan:None,last_scan:HashMap::new(),
            last_buy:None,blocked:HashMap::new(),backoff_until:None,failed_closed:error.is_some() }
    }
    pub fn on_disconnect(&mut self) {
        self.pending = None; self.pending_scan=None; self.offers.clear();
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
    fn settle(&mut self, success: bool) {
        if let Some(idx) = self.pending.take() {
            if let Some(o)=self.journal.orders.get_mut(idx) {
                o.status=if success {"confirmed"} else {"refused"}.into();
                self.status.last=format!("{} #{} · {} × {} {}",
                    if success {"Confirmed"} else {"Refused"},o.listing_id,o.quantity,o.spend,o.currency);
                if success {self.status.confirmed+=1;}
                self.persist();
            }
        }
        self.status.pending=None;
    }
    pub fn ingest(&mut self, text: &str, now: Instant) {
        let Ok(frame)=serde_json::from_str::<Value>(text) else{return};
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
        if t=="batalha" {
            if let Some(events)=frame.get("ev").and_then(Value::as_array) {
                for event in events {
                    if event.get("k").and_then(Value::as_str)!=Some("marketComprado"){continue}
                    let ids=["listingId","anuncioId","vendaId"].iter()
                        .filter_map(|k|number(event,k)).collect::<Vec<_>>();
                    let ids_present=!ids.is_empty() || event.get("anuncio").is_some() || event.get("listing").is_some();
                    let id=self.pending.and_then(|n|self.journal.orders.get(n)).map(|o|o.listing_id);
                    if ids.contains(&id.unwrap_or(0)) ||
                        (!ids_present && id.is_some() && !self.journal.orders.iter().any(|o|
                            o.status=="unknown" && clock_ms().saturating_sub(o.sent_ms)<30_000)) {
                        self.settle(true);
                    }
                }
            }
        }
        if t=="erro" {
            let id=["listingId","anuncioId","vendaId"].iter().find_map(|k|number(&frame,k));
            if id.is_some() && id==self.pending.and_then(|i|self.journal.orders.get(i))
                    .map(|o|o.listing_id) { self.settle(false); }
            let message=["msg","message","chave"].iter().filter_map(|k|frame.get(k).and_then(Value::as_str)).collect::<Vec<_>>().join(" ");
            if message.to_lowercase().contains("rate limit") || message.to_lowercase().contains("muitas mensagens") {
                self.status.error=Some("Market request rate-limited; backing off".into());
                self.backoff_until=Some(now+Duration::from_secs(15));
            }
        }
        if t!="market" {return}
        let Some((scan,_))=self.pending_scan.take() else{return};
        let matched=match (&scan,frame.get("aba").and_then(Value::as_str)) {
            (Scan::Summary,Some("itens"))=>{
                if let Some(rows)=frame.get("resumo").and_then(Value::as_object) {
                    self.summary.clear();
                    for (id,row) in rows {
                        if let Ok(id)=id.parse::<u64>() {
                            if id>0 && row.get("anuncios").and_then(Value::as_u64).unwrap_or(0)>0 {
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
            (Scan::Pokemon(currency,page),Some("vitrine")) if
                frame.get("tipo").and_then(Value::as_str)==Some("pokemon") &&
                frame.get("moeda").and_then(Value::as_str)==Some(currency.as_str()) &&
                number(&frame,"pagina")==Some(*page) => {
                if let Some(rows)=frame.get("linhas").and_then(Value::as_array) {
                    if *page==0 {self.offers.retain(|o|!(o.kind=="pokemon"&&o.currency==*currency));}
                    for o in rows.iter().take(120) {
                        if let Some(o)=Offer::from_listing(o,"pokemon",0,currency,now){self.offers.push(o);}
                    }
                }
                true
            },
            _=>false
        };
        if !matched {
            // Spontaneous market frames are not evidence that our request finished,
            // and must never renew its original timeout.
            self.pending_scan=Some((scan,self.last_scan.values().copied().max().unwrap_or(now)));
        }
        else {self.status.last_scan=Some(now);}
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
        out
    }
    fn budget(&self, cfg:&MarketRules, rule:&BuyRule, offer:&Offer) -> u64 {
        let currency=offer.currency.as_str();
        let i=if currency=="orb"{1}else{0};
        let available=self.balances[i].unwrap_or(0).saturating_sub(if i==0{cfg.gold_reserve}else{cfg.gem_reserve});
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
        qty.min(offer.qty).min(if offer.kind=="pokemon"{1}else{5000})
    }
    pub fn next(&mut self,cfg:&MarketRules,now:Instant) -> Option<Value> {
        self.status.enabled=cfg.enabled&&!cfg.paused;
        self.offers.retain(|offer|offer.at.elapsed()<Duration::from_secs(60));
        self.blocked.retain(|_,at|at.elapsed()<Duration::from_secs(3600));
        if self.failed_closed || !self.status.enabled || self.nick.is_empty() ||
            self.backoff_until.is_some_and(|until|now<until) ||
            self.balances.iter().any(Option::is_none) || !cfg.rules.iter().any(|r|r.enabled) {return None}
        if let Some(i)=self.pending {
            if self.journal.orders.get(i).is_some_and(|o|clock_ms().saturating_sub(o.sent_ms)>7_000) {
                if let Some(o)=self.journal.orders.get_mut(i) {
                    o.status="unknown".into();
                    self.status.unknown+=1;
                    self.status.error=Some(format!("Unverified market order #{}; reserved its quoted spend",o.listing_id));
                }
                self.pending=None;self.status.pending=None;
                self.persist();
            } else {return None;}
        }
        if self.last_buy.is_some_and(|at|at.elapsed()<Duration::from_millis(400)){return None}
        let server_ms=(clock_ms() as i64-self.server_offset_ms).max(0) as u64;
        for rule in cfg.rules.iter().filter(|r|r.enabled&&r.buy_max>0) {
            let selected=self.offers.iter()
                .filter(|o|o.currency==rule.currency&&o.matches(rule,&self.nick,server_ms))
                .filter(|o|!self.blocked.get(&o.id).is_some_and(|at|at.elapsed()<Duration::from_secs(30)))
                .filter(|o|!self.journal.orders.iter().any(|x|x.listing_id==o.id &&
                    (x.status=="unknown" || (o.kind=="pokemon" && x.status=="confirmed"))))
                .min_by_key(|o|(o.price,o.id)).cloned();
            let Some(offer)=selected else {continue};
            let qty=self.budget(cfg,rule,&offer);
            if qty==0 {continue}
            let Some(spend)=offer.price.checked_mul(qty) else {continue};
            if self.journal.orders.len()>=20_000 {
                self.failed_closed=true;self.status.error=Some("Market journal is full".into());return None
            }
            let order=Order{listing_id:offer.id,rule_id:rule.id.clone(),currency:offer.currency.clone(),
                kind:offer.kind.clone(),quantity:qty,spend,day:today(),session:self.session,
                status:"unknown".into(),sent_ms:clock_ms()};
            self.journal.orders.push(order);
            if !self.persist() {return None}
            self.pending=Some(self.journal.orders.len()-1);
            self.blocked.insert(offer.id,now);
            self.status.sent+=1;
            self.last_buy=Some(now);
            self.status.pending=Some(format!("#{} · {} × {} {}",offer.id,qty,offer.price,offer.currency));
            self.status.last=format!("Rule {}: sent {}x listing #{}",rule.id,qty,offer.id);
            return Some(json!({"t":"market.comprar","id":offer.id,"qtd":qty,
                "preco":offer.price,"moeda":offer.currency}));
        }
        if self.pending_scan.as_ref().is_some_and(|(_,at)|at.elapsed()<Duration::from_secs(7)){return None}
        self.pending_scan=None;
        if self.last_scan.values().any(|at|at.elapsed()<Duration::from_secs(2)){return None}
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
        for (_,id,currency) in nominated.into_iter().take(24) {
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
        let (key,scan,payload)=targets.into_iter()
            .filter(|(key,_,_)|self.last_scan.get(key).is_none_or(|at|
                at.elapsed()>=Duration::from_secs(if key=="summary"{15}else{25})))
            .min_by_key(|(key,_,_)|self.last_scan.get(key).copied())?;
        self.last_scan.insert(key,now);self.pending_scan=Some((scan,now));
        Some(payload)
    }
    pub fn send_failed(&mut self) {
        // Fail closed: dispatch state is unverified after a broken socket.
        self.status.error=Some("Market dispatch failed; order outcome unknown".into());
        self.on_disconnect();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn imported_watch_rules_are_disarmed_globally() {
        let rules=MarketRules::import_watch(r#"{"fixedRulesEnabled":true,"fixedItemRules":[{"itemId":42,"buyMax":100,"currency":"gold","buyEnabled":true}],"pokemonBuyRules":[{"kind":"shiny","buyMax":2,"currency":"gold"}]}"#).unwrap();
        assert!(!rules.enabled);assert_eq!(rules.rules.len(),1);assert!(rules.rules[0].enabled);
    }
    #[test] fn item_buy_uses_maximum_affordable_quantity_and_actual_listing() {
        let mut buyer=MarketBuyer::new(GameProfile::Game1);
        buyer.failed_closed=false;buyer.nick="Me".into();buyer.balances=[Some(1000),Some(0)];
        buyer.offers.push(Offer::from_listing(&json!({"id":9,"preco":20,"qtd":80,"vendedor":"Other"}),"item",42,"gold",Instant::now()).unwrap());
        let cfg=MarketRules{enabled:true,gold_reserve:100,rules:vec![BuyRule{id:"a".into(),kind:"item".into(),item_id:42,enabled:true,buy_max:25,..BuyRule::default()}],..MarketRules::default()};
        assert_eq!(buyer.budget(&cfg,&cfg.rules[0],&buyer.offers[0]),45);
    }
    #[test] fn missing_metadata_or_own_listing_not_buyable() {
        let now=Instant::now();
        let o=Offer::from_listing(&json!({"id":2,"preco":4,"qtd":5,"vendedor":"me"}),"item",4,"gold",now).unwrap();
        let r=BuyRule{kind:"item".into(),item_id:4,currency:"gold".into(),buy_max:6,..Default::default()};
        assert!(!o.matches(&r,"ME",clock_ms()));
        assert!(Offer::from_listing(&json!({"id":1,"preco":3,"qtd":3}),"item",4,"gold",now).is_none());
    }
}
