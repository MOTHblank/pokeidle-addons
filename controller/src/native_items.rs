//! Public read-only item catalog for inventory labels. No auth/cookies.
use serde_json::Value;
use std::{collections::BTreeMap,io::Read,time::Duration};
const URL:&str="https://pokeidle.io/assets/items.json";
const LIMIT:u64=2*1024*1024;
#[derive(Clone,Debug)]
pub struct ItemMeta {
    pub name: String,
    /// Relative game artwork path or public URL; never trusted blindly.
    pub icon: Option<String>,
    /// Only an explicitly positive price from PokéIdle's public catalog.
    /// Unknown price must never be synthesized for a shop.buy.
    pub price_gold: Option<u64>,
}
#[derive(Clone,Debug,Default)]
pub struct ItemCatalog {pub items:BTreeMap<u64,ItemMeta>}
impl ItemCatalog {
    pub fn name(&self,id:u64)->String {
        self.items.get(&id).map(|m|m.name.clone()).unwrap_or_else(||format!("Item #{id}"))
    }
}
pub fn parse(source:&str)->Result<ItemCatalog,String> {
    let data:Value=serde_json::from_str(source)
        .map_err(|_|"Malformed public item catalog".to_owned())?;
    let entries=data.get("items").and_then(Value::as_array)
        .ok_or_else(||"Public item catalog has no items".to_owned())?;
    let mut items=BTreeMap::new();
    for item in entries {
        let Some(id)=item.get("id").and_then(Value::as_u64) else {continue};
        let Some(name)=item.get("name").and_then(Value::as_str) else {continue};
        if name.trim().is_empty() {continue}
        let price_gold = item.get("preco").or_else(||item.get("npcPrice"))
            .or_else(||item.get("priceGold"))
            .and_then(Value::as_u64).filter(|price|*price>0);
        let icon=item.get("icon").and_then(Value::as_str)
            .filter(|icon| !icon.trim().is_empty()).map(str::to_owned);
        items.insert(id,ItemMeta{name:name.to_owned(),icon,price_gold});
    }
    if items.is_empty(){return Err("Public item catalog is empty".into());}
    Ok(ItemCatalog{items})
}
pub fn load()->Result<ItemCatalog,String> {
    crate::native_tls::ensure()?;
    let client=reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_|"Item catalog network unavailable".to_owned())?;
    let response=client.get(URL).send().map_err(|e|format!("Item catalog unavailable: {e}"))?
        .error_for_status().map_err(|e|format!("Item catalog failed: {e}"))?;
    if response.content_length().is_some_and(|n|n>LIMIT){return Err("Item catalog oversized".into())}
    let mut buffer=Vec::new();
    response.take(LIMIT+1).read_to_end(&mut buffer)
        .map_err(|_|"Could not read item catalog".to_owned())?;
    if buffer.len() as u64>LIMIT {return Err("Item catalog oversized".into())}
    parse(std::str::from_utf8(&buffer).map_err(|_|"Item catalog invalid UTF-8".to_owned())?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_ids_are_enriched_but_unknown_ids_stay_visible() {
        let catalog=parse(r#"{"items":[{"id":7,"name":"Heart Stone","category":"stones"},
            {"id":200,"name":"Potion","category":"potions"}]}"#).unwrap();
        assert_eq!(catalog.name(7),"Heart Stone");
        assert_eq!(catalog.items.get(&7).unwrap().price_gold,None);
        assert_eq!(catalog.name(8),"Item #8");
        assert!(parse(r#"{"items":[]}"#).is_err());
        let priced=parse(r#"{"items":[{"id":200,"name":"Potion",
            "category":"heal","preco":40},{"id":201,"name":"Potion without price"}]}"#).unwrap();
        assert_eq!(priced.items[&200].price_gold,Some(40));
        assert_eq!(priced.items[&201].price_gold,None);
    }
}
