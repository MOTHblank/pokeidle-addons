//! Account-local market rule editor. Keeps spending authorization separate from
//! Auto Catch and server automation. Zero spending caps mean no cap; rules are
//! disabled by default and require Save before the worker uses them.
use super::*;
use crate::native_market::{BuyRule, MarketRules};

pub(super) fn draw(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    request_item_catalog(app);
    let item_catalog=app.inventory_catalog.clone();
    let item_catalog_error=app.inventory_catalog_error.clone();
    let profile = app.runtimes[index].game.profile;
    if app.runtimes[index].automation.market_draft.is_none() {
        match MarketRules::load(profile) {
            Ok(settings) => app.runtimes[index].automation.market_draft = Some(settings),
            Err(error) => {
                app.runtimes[index].automation.market_draft = Some(MarketRules::default());
                app.runtimes[index].automation.market_notice = format!("Could not load market rules; buying disabled: {error}");
            }
        }
    }
    let live_status=app.runtimes[index].game.native.as_ref().map(|n|n.snapshot().native_market);
    let mut save=false;
    let mut jump_to_snatch=false;
    let mut retry_item_catalog=false;
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr("Automatic market buying · Moth Watch"))
                .size(14.0).strong().color(TEXT));
            if let Some(status)=&live_status {
                let state=if !status.enabled {tr("Off")} else if status.pending.is_some() {tr("Buying")} else {tr("Scanning")};
                ui.label(RichText::new(state).size(10.0)
                    .color(if status.enabled{GOOD}else{DIM}));
                ui.label(RichText::new(tr_dynamic(format!("{} attempts · {} purchases · {} awaiting confirmation",
                    status.sent,status.confirmed,status.unknown))).size(10.0).color(MUTED));
            } else {
                ui.label(RichText::new(tr("Your saved rules will start when this account connects"))
                    .size(10.0).color(DIM));
            }
        });
        ui.label(RichText::new(
            tr("Automatically buy items or Pokémon that match your rules. Works without Firefox. Buying stays off until you enable it and save.")
        ).size(10.0).color(DIM));
        ui.label(RichText::new(tr("Fast buying is enabled: checks several offers at once, retries quickly, and buys as many units as your rules and balance allow."))
            .size(10.0).color(MUTED));
        if let Some(status)=&live_status {
            if let Some(p)=&status.pending {ui.label(RichText::new(tr_dynamic(format!("Waiting for purchase confirmation: {p}"))).size(10.0).color(WARN));}
            if !status.last.is_empty() {ui.label(RichText::new(tr_dynamic(status.last.clone())).size(10.0).color(MUTED));}
            if let Some(error)=&status.error {ui.label(RichText::new(tr_dynamic(error.to_string())).size(10.0).color(WARN));}
        }
        if !app.runtimes[index].automation.market_notice.is_empty() {
            ui.label(RichText::new(tr_dynamic(app.runtimes[index].automation.market_notice.clone())).size(10.0).color(WARN));
        }
        ui.label(RichText::new(tr("An enabled Any item rule may buy items even when a matching Specific item rule is off.")).size(10.0).color(MUTED));
        draw_market_copy_controls(app,ui,index);
        let automation = &mut app.runtimes[index].automation;
        let Some(cfg)=automation.market_draft.as_mut() else {return};
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui|{
            ui.checkbox(&mut cfg.enabled,tr("Enable automatic buying"))
                .on_hover_text(tr("Affects this account after saving"));
            ui.checkbox(&mut cfg.paused,tr("Pause purchases"))
                .on_hover_text(tr("Pause buying without deleting rules"));
            if ui.button(tr("Save changes"))
                .on_hover_text(tr("Apply the current editor settings to this account")).clicked(){
                save=true;
            }
            if ui.small_button(tr("↓ Snatch queue"))
                .on_hover_text(tr("Jump to saved offers and pending listings")).clicked(){
                jump_to_snatch=true;
            }
            ui.label(RichText::new(tr("Save to apply changes"))
                .size(10.0).color(DIM));
        });
        ui.separator();
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr("Balance reserves")).strong().size(11.0).color(TEXT));
            field(ui,tr("Gold"),&mut cfg.gold_reserve);
            field(ui,tr("Gems"),&mut cfg.gem_reserve);
        });
        ui.collapsing(tr("Global spending limits (0 = unlimited)"),|ui|{
            ui.horizontal_wrapped(|ui|{
                field(ui,tr("Gold / buy"),&mut cfg.max_gold_per_buy);
                field(ui,tr("Gems / buy"),&mut cfg.max_gems_per_buy);
                field(ui,tr("Gold / session"),&mut cfg.max_gold_session);
                field(ui,tr("Gems / session"),&mut cfg.max_gems_session);
            });
            ui.horizontal_wrapped(|ui|{
                field(ui,tr("Gold / UTC day"),&mut cfg.max_gold_day);
                field(ui,tr("Gems / UTC day"),&mut cfg.max_gems_day);
                field(ui,tr("Purchases / session"),&mut cfg.max_purchases_session);
                field(ui,tr("Units / session"),&mut cfg.max_quantity_session);
            });
            ui.label(RichText::new(tr("Purchases awaiting confirmation still count toward your spending limits. Daily limits are kept after restarting the app."))
                .size(10.0).color(DIM));
        });
        ui.add_space(7.0);
        ui.horizontal_wrapped(|ui|{
            ui.label(RichText::new(tr_dynamic(format!("Buying rules ({})",cfg.rules.len()))).strong().color(TEXT));
            if ui.small_button(tr("+ Specific item")).clicked() {new_rule(cfg,"item");}
            if ui.small_button(tr("+ Any item")).clicked() {new_rule(cfg,"any_item");}
            if ui.small_button(tr("+ Pokémon")).clicked() {new_rule(cfg,"pokemon_any");}
        });
        if cfg.rules.is_empty() {
            ui.label(RichText::new(tr("No buying rules. Add an item or Pokémon price ceiling, then save."))
                .size(10.0).color(DIM));
        }
        let mut remove=None;
        for (i,rule) in cfg.rules.iter_mut().enumerate() {
            egui::Frame::new().fill(PANEL_ALT)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(7.0)
                .inner_margin(egui::Margin::symmetric(10,7)).show(ui,|ui|{
                ui.horizontal_wrapped(|ui|{
                    ui.checkbox(&mut rule.enabled,"")
                        .on_hover_text(tr("Include this rule when automatic buying is enabled"));
                    let item=if rule.kind=="item" {
                        format!(" · {}",item_name(item_catalog.as_deref(),rule.item_id))
                    } else {String::new()};
                    ui.label(RichText::new(format!("{}{} · {} {}/{} · {}",
                        kind_label(&rule.kind),item,tr("max"),rule.buy_max,tr("unit"),
                        if rule.currency=="orb"{tr("Gems")}else{tr("Gold")}))
                        .size(11.0).strong().color(if rule.enabled{TEXT}else{MUTED}));
                    if ui.small_button(tr("Remove"))
                        .on_hover_text(tr("Remove from the editor; save to apply")).clicked(){
                        remove=Some(i);
                    }
                });
                egui::CollapsingHeader::new(tr("Edit rule & limits"))
                    .id_salt(("market-rule-editor",index,rule.id.as_str()))
                    .default_open(rule.buy_max==0 || (rule.kind=="item" && rule.item_id==0))
                    .show(ui,|ui|{
                        ui.horizontal_wrapped(|ui|{
                            egui::ComboBox::from_id_salt(("native-market-kind",index,i))
                                .selected_text(kind_label(&rule.kind))
                                .show_ui(ui,|ui|{
                                    for (value,label) in [
                                        ("item",tr("Specific item")),("any_item",tr("Any item")),
                                        ("pokemon_any",tr("Any Pokémon")),("pokemon_normal",tr("Non-shiny Pokémon")),("pokemon_p5",tr("P5 Pokémon")),
                                        ("pokemon_shiny",tr("Shiny Pokémon"))] {
                                        ui.selectable_value(&mut rule.kind,value.into(),label);
                                    }
                                });
                            if rule.kind=="pokemon_shiny"{rule.currency="orb".into();}
                            egui::ComboBox::from_id_salt(("native-market-currency",index,i))
                                .selected_text(if rule.currency=="orb"{tr("Gems")}else{tr("Gold")})
                                .show_ui(ui,|ui|{
                                    ui.selectable_value(&mut rule.currency,"gold".into(),tr("Gold"));
                                    ui.selectable_value(&mut rule.currency,"orb".into(),tr("Gems"));
                                });
                            field(ui,tr("Max unit price"),&mut rule.buy_max);
                        });
                        if rule.kind=="item" {
                            let search=automation.market_item_searches
                                .entry(rule.id.clone()).or_default();
                            draw_item_picker(
                                ui,item_catalog.as_deref(),item_catalog_error.as_deref(),
                                search,&mut rule.item_id,
                                format!("market-rule-item-picker-{index}-{}",rule.id),
                                &mut retry_item_catalog
                            );
                        }
                        if rule.kind.starts_with("pokemon_") {
                            ui.label(RichText::new(tr("Pokémon potency (0 = any, 1–5)"))
                                .size(10.0).color(MUTED));
                            ui.horizontal_wrapped(|ui|{
                                ui.label(tr("Minimum"));
                                ui.add(egui::DragValue::new(&mut rule.min_potency).range(0..=5));
                                ui.label(tr("Maximum"));
                                ui.add(egui::DragValue::new(&mut rule.max_potency).range(0..=5));
                            });
                            ui.horizontal_wrapped(|ui|{
                                ui.label(tr("Species ID (0 = any)"));
                                ui.add(egui::DragValue::new(&mut rule.species_id).range(0..=1_000_000));
                                ui.label(tr("Minimum level (0 = any)"));
                                ui.add(egui::DragValue::new(&mut rule.min_level).range(0..=1_000_000));
                                ui.label(tr("Minimum IV sum (0 = any)"));
                                ui.add(egui::DragValue::new(&mut rule.min_iv).range(0..=192));
                            });
                        } else {
                            ui.horizontal_wrapped(|ui|{
                                ui.label(RichText::new(tr("Minimum listing units (0 = any)"))
                                    .size(10.0).color(MUTED));
                                ui.add(egui::DragValue::new(&mut rule.min_listing_quantity)
                                    .range(0..=1_000_000_000));
                                ui.label(RichText::new(tr("Maximum units per purchase (0 = all affordable)"))
                                    .size(10.0).color(MUTED));
                                ui.add(egui::DragValue::new(&mut rule.max_units_per_buy)
                                    .range(0..=1_000_000_000));
                            });
                        }
                        ui.collapsing(tr("Per-rule limits (0 = unlimited)"),|ui|{
                            ui.horizontal_wrapped(|ui|{
                                field(ui,tr("Spend / session"),&mut rule.max_spend_session);
                                field(ui,tr("Spend / UTC day"),&mut rule.max_spend_day);
                                field(ui,tr("Purchases / session"),&mut rule.max_purchases_session);
                                field(ui,tr("Units / session"),&mut rule.max_quantity_session);
                            });
                        });
                    });
            });
            ui.add_space(4.0);
        }
        if let Some(i)=remove {
            let removed=cfg.rules.remove(i);
            automation.market_item_searches.remove(&removed.id);
        }

        ui.add_space(8.0);
        ui.separator();
        if ui.small_button(tr("Show Snatch queue ↓"))
            .on_hover_text(tr("Jump to your saved offers and new listings")).clicked(){
            jump_to_snatch=true;
        }
        ui.add_space(5.0);
        let snatch_response = egui::CollapsingHeader::new(
            tr_dynamic(format!("Snatch · {} saved offers · buy when available",cfg.snatch_targets.len())))
            .id_salt(("market-snatch", index))
            .default_open(true)
            .show(ui,|ui|{
            ui.label(RichText::new(
                "Snatch watches up to 40 offers and attempts to buy them immediately when they unlock. It checks shortly before release and retries rapidly during the four-second buying window. Your price and spending limits always apply. Enable automatic buying to use Snatch."
            ).size(10.0).color(MUTED));
            ui.label(RichText::new(
                tr("Enter the offer ID and the highest price you will pay per unit. Before buying, Snatch checks the seller, quantity, currency, price and unlock time. It will not buy an offer it cannot verify.")
            ).size(10.0).color(DIM));
            {
                let t=&mut automation.market_snatch_draft;
                ui.horizontal_wrapped(|ui|{
                egui::ComboBox::from_id_salt(("snatch-kind",index))
                    .selected_text(if t.kind=="pokemon"{"Pokémon"}else{tr("Item")})
                    .show_ui(ui,|ui|{
                        ui.selectable_value(&mut t.kind,"item".into(),tr("Item"));
                        ui.selectable_value(&mut t.kind,"pokemon".into(),"Pokémon");
                    });
                egui::ComboBox::from_id_salt(("snatch-currency",index))
                    .selected_text(if t.currency=="orb"{tr("Gems")}else{tr("Gold")})
                    .show_ui(ui,|ui|{
                        ui.selectable_value(&mut t.currency,"gold".into(),tr("Gold"));
                        ui.selectable_value(&mut t.currency,"orb".into(),tr("Gems"));
                    });
                field(ui,tr("Listing #"),&mut t.id);
                field(ui,tr("Max unit price"),&mut t.max_price);
                });
                if t.kind=="item" {
                    draw_item_picker(
                        ui,item_catalog.as_deref(),item_catalog_error.as_deref(),
                        &mut automation.market_snatch_item_search,&mut t.item_id,
                        format!("market-snatch-item-picker-{index}"),
                        &mut retry_item_catalog
                    );
                }
                if ui.add_enabled(cfg.snatch_targets.len()<40,
                    egui::Button::new(tr("Add to Snatch"))).clicked() {
                    let mut target=t.clone();
                    target.arm();
                    if target.id==0 || target.max_price==0 ||
                        (target.kind=="item" && target.item_id==0) {
                        automation.market_notice=
                            tr("Choose an item (for item listings), enter a real listing ID, and set a maximum price.").into();
                    } else if cfg.snatch_targets.iter().any(|old|old.id==target.id) {
                        automation.market_notice=
                            tr("This offer is already in your Snatch list.").into();
                    } else {
                        cfg.snatch_targets.push(target);
                        automation.market_notice=tr("Offer added to Snatch and saved for this account.").into();
                        save=true;
                    }
                }
            }
            let mut remove=None;
            for (i,t) in cfg.snatch_targets.iter().enumerate() {
                let seen=live_status.as_ref().and_then(|st|
                    st.snatches.iter().find(|row|row.id==t.id));
                egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
                    .inner_margin(egui::Margin::symmetric(9,6)).show(ui,|ui|{
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(format!(
                            "#{} · {} · {} · {} {} / {}",
                            t.id,if t.kind=="pokemon"{"Pokémon"}else{
                                item_name(item_catalog.as_deref(),t.item_id)
                            },
                            if t.currency=="orb"{tr("Gems")}else{tr("Gold")},
                            tr("max"),t.max_price,tr("unit")
                        )).size(11.0).strong().color(TEXT));
                        if ui.small_button(tr("Remove from Snatch")).clicked(){remove=Some(i);}
                    });
                    if let Some(s)=seen {
                        ui.label(RichText::new(format!(
                            "{} · {} {} · {} {}",
                            tr_dynamic(s.phase.clone()),s.attempts,tr("attempts"),
                            s.quantity,tr("units available")
                        )).size(10.0).color(if s.phase.contains("burst"){GOOD}else{MUTED}));
                    } else {
                        ui.label(RichText::new(tr("Saved · waiting for the account to connect and the offer to appear"))
                            .size(10.0).color(DIM));
                    }
                });
            }
            if let Some(i)=remove {
                cfg.snatch_targets.remove(i);
                automation.market_notice=tr("Offer removed from Snatch.").into();
                save=true;
            }
            if let Some(status)=live_status.as_ref() {
                ui.label(RichText::new(tr_dynamic(format!("Snatch buy attempts: {}",status.snatch_sent)))
                    .size(10.0).color(MUTED));
                let automatic=status.snatches.iter().filter(|s|!s.manual)
                    .take(20).collect::<Vec<_>>();
                let discovery=status.held_offers.iter().filter(|row|
                    !cfg.snatch_targets.iter().any(|t|t.id==row.id))
                    .take(30).collect::<Vec<_>>();
                if !discovery.is_empty() {
                    ui.label(RichText::new(tr("Offers waiting to become available"))
                        .strong().size(11.0).color(TEXT));
                    ui.label(RichText::new(
                        tr("Choose an offer to add it to Snatch at its current unit price. No purchase happens until automatic buying is enabled.")
                    ).size(10.0).color(DIM));
                    let mut arm_from_scan=None;
                    for row in discovery {
                        ui.horizontal_wrapped(|ui|{
                            ui.label(RichText::new(format!(
                                "#{} · {} · {} {} /{} · {}x · {}",
                                row.id,if row.kind=="item" {
                                    item_name(item_catalog.as_deref(),row.item_id)
                                } else {tr(&row.kind).to_owned()},row.price,tr(&row.currency),
                                tr("unit"),row.quantity,tr_dynamic(row.phase.clone())
                            )).size(10.0).color(MUTED));
                            if ui.add_enabled(cfg.snatch_targets.len()<40,
                                egui::Button::new(tr("Snatch"))).clicked() {
                                let mut target=crate::native_market::SnatchTarget {
                                    id:row.id,kind:row.kind.clone(),currency:row.currency.clone(),
                                    item_id:row.item_id,max_price:row.price,..Default::default()
                                };
                                target.arm();
                                arm_from_scan=Some(target);
                            }
                        });
                    }
                    if let Some(target)=arm_from_scan {
                        if !cfg.snatch_targets.iter().any(|t|t.id==target.id) &&
                            cfg.snatch_targets.len()<40 {
                            cfg.snatch_targets.push(target);
                            automation.market_notice=tr("Offer added to your Snatch list.").into();
                            save=true;
                        }
                    }
                }
                if !automatic.is_empty() {
                    ui.label(RichText::new(tr("Rule-triggered held listings")).strong()
                        .size(11.0).color(TEXT));
                    for row in automatic {
                        ui.label(RichText::new(format!(
                            "#{} · {} {} · {} · {} {}",
                            row.id,row.price,tr(&row.currency),tr_dynamic(row.phase.clone()),
                            row.attempts,tr("attempts")
                        )).size(10.0).color(MUTED));
                    }
                }
            }
        });
        if jump_to_snatch {
            ui.scroll_to_rect(snatch_response.header_response.rect, Some(Align::Min));
        }
        ui.collapsing(tr("Import existing Moth Watch rules"),|ui|{
            ui.label(RichText::new(
                "To import old Moth Watch rules, open Firefox developer tools and copy the result of localStorage.getItem('moth-pokeidle-market-bot-config-v1'). Paste it below. Imported rules stay disabled until you review and save them."
            ).size(10.0).color(DIM));
            ui.add(egui::TextEdit::multiline(&mut automation.market_import)
                .hint_text(tr("Paste Moth Watch JSON settings…"))
                .desired_rows(3).desired_width(f32::INFINITY));
            if ui.small_button(tr("Import settings into editor")).clicked() {
                match MarketRules::import_watch(&automation.market_import) {
                    Ok(imported)=>{
                        *cfg=imported;
                        automation.market_item_searches.clear();
                        automation.market_notice=tr("Rules imported. Review and save them to use automatic buying.").into();
                    }
                    Err(error)=>automation.market_notice=format!("Import rejected: {error}"),
                }
            }
        });
        ui.horizontal_wrapped(|ui|{
            if ui.button(tr("Save market rules")).clicked(){save=true;}
            if ui.small_button(tr("Revert unsaved changes")).clicked(){
                match MarketRules::load(profile) {
                    Ok(saved)=>{
                        *cfg=saved;
                        automation.market_item_searches.clear();
                        automation.market_notice=tr("Reloaded saved market rules.").into();
                    }
                    Err(error)=>automation.market_notice=error,
                }
            }
            ui.label(RichText::new(tr("Changes take effect only after Save.")).size(10.0).color(DIM));
        });
    });
    if retry_item_catalog {
        app.inventory_catalog_error=None;
        app.inventory_catalog_busy=false;
        request_item_catalog(app);
    }
    if save {
        let cfg=app.runtimes[index].automation.market_draft.as_ref().cloned().unwrap_or_default();
        let result=match app.runtimes[index].game.native.as_ref() {
            Some(native)=>native.set_market_rules(cfg),
            None=>cfg.save(profile),
        };
        match result {
            Ok(())=>app.runtimes[index].automation.market_notice=tr("Market rules saved for this account.").into(),
            Err(error)=>app.runtimes[index].automation.market_notice=format!("Market settings NOT applied: {error}"),
        }
    }
}
/// Explicitly copy the editor's buying strategy (including unsaved edits).
/// Each destination retains its own Snatch listing queue, login and ledger.
fn draw_market_copy_controls(app: &mut ControllerApp, ui: &mut egui::Ui, source: usize) {
    let mut apply = false;
    ui.collapsing(tr("Copy buying settings to other accounts"), |ui| {
        ui.label(RichText::new(
            tr("Copy rules, price ceilings, reserves and spending limits. Existing buying settings on the selected accounts are replaced; their Snatch queues stay intact.")
        ).size(10.0).color(MUTED));
        ui.label(RichText::new(
            tr("Copies the values currently in this editor, including unsaved changes.")
        ).size(10.0).color(DIM));
        ui.horizontal_wrapped(|ui| {
            for dest in 0..app.runtimes.len() {
                if dest == source { continue; }
                let name = app.accounts[dest].name.trim();
                let label = if name.is_empty() {
                    tr_dynamic(format!("Account {}", dest+1))
                } else {
                    format!("{} · {}", dest+1, name)
                };
                ui.checkbox(&mut app.workspace.market_copy_targets[dest],label);
            }
        });
        ui.checkbox(&mut app.workspace.market_copy_activation,
            tr("Also copy buying activation (may immediately start purchases)"));
        ui.label(RichText::new(
            tr("Default: buying remains OFF on destination accounts. Only select the option above to copy the source's enabled/paused state.")
        ).size(10.0).color(DIM));
        let selected=(0..app.runtimes.len()).any(|dest|
            dest!=source && app.workspace.market_copy_targets[dest]);
        if ui.add_enabled(selected,egui::Button::new(
            tr("Overwrite buying settings on selected accounts"))).clicked() {
            apply=true;
        }
    });
    if !apply { return; }
    let Some(source_rules)=app.runtimes[source].automation.market_draft.as_ref().cloned() else { return };
    if let Err(error)=source_rules.validate() {
        app.runtimes[source].automation.market_notice=format!("Copy canceled; fix source rules: {error}");
        return;
    }
    let mut copied=Vec::new();
    let mut failures=Vec::new();
    for dest in 0..app.runtimes.len() {
        if dest==source || !app.workspace.market_copy_targets[dest] { continue; }
        let profile=app.runtimes[dest].game.profile;
        // Respect the destination's in-memory draft first: a newly armed
        // Snatch listing may not have been saved yet. Its queue must survive
        // copying the separate automatic buying strategy.
        let existing=if let Some(draft)=app.runtimes[dest].automation.market_draft.as_ref() {
            draft.clone()
        } else {
            match MarketRules::load(profile) {
                Ok(rules)=>rules,
                Err(error)=>{
                    failures.push(format!("Account {}: cannot load: {error}",dest+1));
                    continue;
                }
            }
        };
        let copied_rules=existing.copy_buying_from(
            &source_rules,app.workspace.market_copy_activation);
        let result=match app.runtimes[dest].game.native.as_ref() {
            Some(native)=>native.set_market_rules(copied_rules.clone()),
            None=>copied_rules.save(profile)
        };
        match result {
            Ok(())=>{
                app.runtimes[dest].automation.market_draft=Some(copied_rules);
                app.runtimes[dest].automation.market_notice=format!(
                    "Buying settings copied from account {}. Snatch queue preserved.",
                    source+1);
                app.workspace.market_copy_targets[dest]=false;
                copied.push((dest+1).to_string());
            }
            Err(error)=>failures.push(format!("Account {}: {error}",dest+1)),
        }
    }
    app.workspace.market_copy_activation=false;
    let success=if copied.is_empty() {
        tr("No accounts updated").to_owned()
    } else {
        format!("Buying settings copied to account(s) {}",copied.join(", "))
    };
    app.runtimes[source].automation.market_notice=if failures.is_empty() {
        success
    } else {
        format!("{success}. Failed: {}",failures.join(" · "))
    };
}


fn item_name(catalog:Option<&ItemCatalog>,id:u64)->String {
    if id==0 {return tr("No item selected").to_owned()}
    catalog.map(|c|c.name(id)).unwrap_or_else(||format!("Item #{id}"))
}

fn matching_items(catalog:&ItemCatalog,query:&str,limit:usize)->Vec<(u64,String)> {
    let needle=query.trim().to_lowercase();
    let mut rows=catalog.items.iter()
        .filter(|(_,item)|needle.is_empty() || item.name.to_lowercase().contains(&needle))
        .map(|(id,item)|(*id,item.name.clone()))
        .collect::<Vec<_>>();
    rows.sort_by(|a,b|a.1.to_lowercase().cmp(&b.1.to_lowercase()).then(a.0.cmp(&b.0)));
    rows.truncate(limit);
    rows
}

fn draw_item_picker(
    ui:&mut egui::Ui,
    catalog:Option<&ItemCatalog>,
    catalog_error:Option<&str>,
    search:&mut String,
    selected:&mut u64,
    id_salt:String,
    retry:&mut bool,
) {
    let selected_name=item_name(catalog,*selected);
    egui::CollapsingHeader::new(format!("{} · {}",tr("Item"),selected_name))
        .id_salt(id_salt)
        .default_open(*selected==0)
        .show(ui,|ui|{
            let Some(catalog)=catalog else {
                if let Some(error)=catalog_error {
                    ui.label(RichText::new(format!("{}: {error}",tr("Item catalog unavailable")))
                        .size(10.0).color(WARN));
                    if ui.small_button(tr("Retry item catalog")).clicked(){*retry=true;}
                } else {
                    ui.label(RichText::new(tr("Loading PokéIdle item catalog…"))
                        .size(10.0).color(DIM));
                }
                return;
            };
            ui.add(egui::TextEdit::singleline(search)
                .hint_text(tr("Search items by name…"))
                .desired_width(f32::INFINITY));
            let matches=matching_items(catalog,search,80);
            if matches.is_empty() {
                ui.label(RichText::new(tr("No matching items")).size(10.0).color(DIM));
                return;
            }
            egui::ScrollArea::vertical().max_height(190.0).show(ui,|ui|{
                for (id,name) in &matches {
                    let active=*selected==*id;
                    if ui.selectable_label(active,name).clicked() {
                        *selected=*id;
                        search.clear();
                    }
                }
            });
            if matches.len()==80 {
                ui.label(RichText::new(tr("Showing the first 80 matches. Type more to narrow the catalog."))
                    .size(9.0).color(DIM));
            }
            if *selected>0 && !catalog.items.contains_key(selected) {
                ui.label(RichText::new(tr("The saved item is not present in the current PokéIdle catalog. Choose a replacement before saving."))
                    .size(10.0).color(WARN));
            }
        });
}

fn field(ui:&mut egui::Ui,label:&str,value:&mut u64){
    ui.label(RichText::new(label).size(10.0).color(MUTED));
    ui.add(egui::DragValue::new(value).range(0..=1_000_000_000_000u64).speed(1.0));
}
fn kind_label(kind:&str)->&'static str {
    match kind {
        "item"=>tr("Specific item"),"any_item"=>tr("Any item"),
        "pokemon_any"=>tr("Any Pokémon"),"pokemon_normal"=>tr("Non-shiny Pokémon"),"pokemon_p5"=>tr("P5 Pokémon"),
        "pokemon_shiny"=>tr("Shiny Pokémon"),_=>tr("Invalid rule")
    }
}
fn new_rule(cfg:&mut MarketRules,kind:&str) {
    if cfg.rules.len()>=64{return}
    let id=format!("rule-{}",std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
    cfg.rules.push(BuyRule{id,kind:kind.into(),
        currency:if kind=="pokemon_shiny"{"orb"}else{"gold"}.into(),..Default::default()});
}


#[cfg(test)]
mod item_picker_tests {
    use super::*;

    #[test]
    fn item_search_is_case_insensitive_and_name_based() {
        let catalog=crate::native_items::parse(r#"{"items":[
            {"id":205,"name":"Revive"},{"id":206,"name":"Max Revive"},
            {"id":70000,"name":"Bronze Boss Token"}]}"#).unwrap();
        assert_eq!(matching_items(&catalog,"revive",80),
            vec![(206,"Max Revive".into()),(205,"Revive".into())]);
        assert_eq!(matching_items(&catalog,"BOSS",80),
            vec![(70000,"Bronze Boss Token".into())]);
        assert!(matching_items(&catalog,"205",80).is_empty());
    }

    #[test]
    fn selected_item_uses_catalog_name_without_changing_protocol_id() {
        let catalog=crate::native_items::parse(
            r#"{"items":[{"id":205,"name":"Revive"}]}"#).unwrap();
        assert_eq!(item_name(Some(&catalog),205),"Revive");
        assert_eq!(item_name(Some(&catalog),999),"Item #999");
        assert_eq!(item_name(Some(&catalog),0),tr("No item selected"));
    }
}
