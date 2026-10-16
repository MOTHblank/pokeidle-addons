//! Account-local market rule editor. Keeps spending authorization separate from
//! Auto Catch and server automation. Zero spending caps mean no cap; rules are
//! disabled by default and require Save before the worker uses them.
use super::*;
use crate::native_market::{BuyRule, MarketRules};

pub(super) fn draw(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    let profile = app.games[index].profile;
    if app.market_drafts[index].is_none() {
        match MarketRules::load(profile) {
            Ok(settings) => app.market_drafts[index] = Some(settings),
            Err(error) => {
                app.market_drafts[index] = Some(MarketRules::default());
                app.market_notices[index] = format!("Could not load market rules; buying disabled: {error}");
            }
        }
    }
    let live_status=app.games[index].native.as_ref().map(|n|n.snapshot().native_market);
    let mut save=false;
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new("Market rule buying · Moth Watch")
                .size(14.0).strong().color(TEXT));
            if let Some(status)=&live_status {
                let state=if !status.enabled {"Off"} else if status.pending.is_some() {"Buying"} else {"Scanning"};
                ui.label(RichText::new(state).size(10.0)
                    .color(if status.enabled{GOOD}else{DIM}));
                ui.label(RichText::new(format!("{} sent · {} confirmed · {} unverified",
                    status.sent,status.confirmed,status.unknown)).size(10.0).color(MUTED));
            } else {
                ui.label(RichText::new("Saved rules apply when the native game connects")
                    .size(10.0).color(DIM));
            }
        });
        ui.label(RichText::new(
            "Native authenticated game socket · no Violentmonkey · independent of Auto Catch. Buying stays off until enabled and saved."
        ).size(10.0).color(DIM));
        if let Some(status)=&live_status {
            if let Some(p)=&status.pending {ui.label(RichText::new(format!("Awaiting receipt: {p}")).size(10.0).color(WARN));}
            if !status.last.is_empty() {ui.label(RichText::new(&status.last).size(10.0).color(MUTED));}
            if let Some(error)=&status.error {ui.label(RichText::new(error).size(10.0).color(WARN));}
        }
        if !app.market_notices[index].is_empty() {
            ui.label(RichText::new(&app.market_notices[index]).size(10.0).color(WARN));
        }
        let Some(cfg)=app.market_drafts[index].as_mut() else {return};
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui|{
            ui.checkbox(&mut cfg.enabled,"Enable native market buying");
            ui.checkbox(&mut cfg.paused,"Pause purchases");
            ui.label(RichText::new(if cfg.enabled&&!cfg.paused {"Save to arm"}else{"Disarmed"})
                .size(10.0).color(if cfg.enabled&&!cfg.paused{WARN}else{DIM}));
        });
        ui.separator();
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new("Balance reserves").strong().size(11.0).color(TEXT));
            field(ui,"Gold",&mut cfg.gold_reserve);
            field(ui,"Gems",&mut cfg.gem_reserve);
        });
        ui.collapsing("Global spending limits (0 = unlimited)",|ui|{
            ui.horizontal_wrapped(|ui|{
                field(ui,"Gold / buy",&mut cfg.max_gold_per_buy);
                field(ui,"Gems / buy",&mut cfg.max_gems_per_buy);
                field(ui,"Gold / session",&mut cfg.max_gold_session);
                field(ui,"Gems / session",&mut cfg.max_gems_session);
            });
            ui.horizontal_wrapped(|ui|{
                field(ui,"Gold / UTC day",&mut cfg.max_gold_day);
                field(ui,"Gems / UTC day",&mut cfg.max_gems_day);
                field(ui,"Purchases / session",&mut cfg.max_purchases_session);
                field(ui,"Units / session",&mut cfg.max_quantity_session);
            });
            ui.label(RichText::new("Unknown purchase outcomes retain their quoted spending exposure. Daily limits persist across app restarts.")
                .size(10.0).color(DIM));
        });
        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(format!("Rules ({})",cfg.rules.len())).strong().color(TEXT));
            if ui.small_button("+ Specific item").clicked() {new_rule(cfg,"item");}
            if ui.small_button("+ Any item").clicked() {new_rule(cfg,"any_item");}
            if ui.small_button("+ Pokémon").clicked() {new_rule(cfg,"pokemon_any");}
        });
        if cfg.rules.is_empty() {
            ui.label(RichText::new("No buying rules. Add an item or Pokémon price ceiling, then save.")
                .size(10.0).color(DIM));
        }
        let mut remove=None;
        for (i,rule) in cfg.rules.iter_mut().enumerate() {
            egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
                .inner_margin(egui::Margin::symmetric(9,6)).show(ui,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.checkbox(&mut rule.enabled,"");
                    let item=if rule.kind=="item" {format!(" #{}",rule.item_id)} else {String::new()};
                    ui.label(RichText::new(format!("{}{} · max {}/unit · {}",
                        kind_label(&rule.kind),item,rule.buy_max,rule.currency)).size(11.0).strong());
                    if ui.small_button("Remove").clicked() {remove=Some(i);}
                });
                ui.horizontal_wrapped(|ui|{
                    egui::ComboBox::from_id_salt(("native-market-kind",index,i))
                        .selected_text(kind_label(&rule.kind))
                        .show_ui(ui,|ui|{
                            for (value,label) in [
                                ("item","Specific item"),("any_item","Any item"),
                                ("pokemon_any","Any Pokémon"),("pokemon_p5","P5 Pokémon"),
                                ("pokemon_shiny","Shiny Pokémon")] {
                                ui.selectable_value(&mut rule.kind,value.into(),label);
                            }
                        });
                    if rule.kind=="pokemon_shiny" {rule.currency="orb".into();}
                    egui::ComboBox::from_id_salt(("native-market-currency",index,i))
                        .selected_text(if rule.currency=="orb"{"Gems"}else{"Gold"})
                        .show_ui(ui,|ui|{
                            ui.selectable_value(&mut rule.currency,"gold".into(),"Gold");
                            ui.selectable_value(&mut rule.currency,"orb".into(),"Gems");
                        });
                    if rule.kind=="item" {field(ui,"Item ID",&mut rule.item_id);}
                    field(ui,"Max unit price",&mut rule.buy_max);
                });
                ui.collapsing("Per-rule limits (0 = unlimited)",|ui|{
                    ui.horizontal_wrapped(|ui|{
                        field(ui,"Spend / session",&mut rule.max_spend_session);
                        field(ui,"Spend / UTC day",&mut rule.max_spend_day);
                        field(ui,"Purchases / session",&mut rule.max_purchases_session);
                        field(ui,"Units / session",&mut rule.max_quantity_session);
                    });
                });
            });
            ui.add_space(4.0);
        }
        if let Some(i)=remove {cfg.rules.remove(i);}
        ui.collapsing("Import existing Moth Watch rules",|ui|{
            ui.label(RichText::new(
                "From a Firefox profile where Moth Watch is configured, copy localStorage.getItem('moth-pokeidle-market-bot-config-v1') and paste its JSON here. Import never enables native buying automatically."
            ).size(10.0).color(DIM));
            ui.add(egui::TextEdit::multiline(&mut app.market_imports[index])
                .hint_text("Paste Moth Watch JSON settings…")
                .desired_rows(3).desired_width(f32::INFINITY));
            if ui.small_button("Import settings into editor").clicked() {
                match MarketRules::import_watch(&app.market_imports[index]) {
                    Ok(imported)=>{
                        *cfg=imported;
                        app.market_notices[index]="Rules imported but not yet saved; native buying is OFF.".into();
                    }
                    Err(error)=>app.market_notices[index]=format!("Import rejected: {error}"),
                }
            }
        });
        ui.horizontal_wrapped(|ui|{
            if ui.button("Save market rules").clicked(){save=true;}
            if ui.small_button("Revert unsaved changes").clicked(){
                match MarketRules::load(profile) {
                    Ok(saved)=>{*cfg=saved;app.market_notices[index]="Reloaded saved market rules.".into();}
                    Err(error)=>app.market_notices[index]=error,
                }
            }
            ui.label(RichText::new("Changes take effect only after Save.").size(10.0).color(DIM));
        });
    });
    if save {
        let cfg=app.market_drafts[index].as_ref().cloned().unwrap_or_default();
        let result=match app.games[index].native.as_ref() {
            Some(native)=>native.set_market_rules(cfg),
            None=>cfg.save(profile),
        };
        match result {
            Ok(())=>app.market_notices[index]="Market rules saved for this account.".into(),
            Err(error)=>app.market_notices[index]=format!("Market settings NOT applied: {error}"),
        }
    }
}
fn field(ui:&mut egui::Ui,label:&str,value:&mut u64){
    ui.label(RichText::new(label).size(10.0).color(MUTED));
    ui.add(egui::DragValue::new(value).range(0..=1_000_000_000_000u64).speed(1.0));
}
fn kind_label(kind:&str)->&'static str {
    match kind {
        "item"=>"Specific item","any_item"=>"Any item",
        "pokemon_any"=>"Any Pokémon","pokemon_p5"=>"P5 Pokémon",
        "pokemon_shiny"=>"Shiny Pokémon",_=>"Invalid rule"
    }
}
fn new_rule(cfg:&mut MarketRules,kind:&str) {
    if cfg.rules.len()>=64{return}
    let id=format!("rule-{}",std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    cfg.rules.push(BuyRule{id,kind:kind.into(),
        currency:if kind=="pokemon_shiny"{"orb"}else{"gold"}.into(),..Default::default()});
}
