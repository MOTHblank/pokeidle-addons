//! Restock selectors and settings draft synchronization.
use super::*;

// Auto Catch+ restock picker: compact selectable items instead of raw IDs.
// IDs are the game protocol IDs. Public metadata
// only enriches captions; absent metadata never switches a saved selection.
// Compact, icon-only Auto Catch+ choice. Names and stock remain in tooltips.
// Every variant must occupy the same fixed-sized cell: a centered/justified
// fallback label used to stretch the "Off" cell across an entire column,
// hiding the real ball/potion icons on the next wrapped row.
const RESTOCK_ICON_SIZE: f32 = 23.0;
const RESTOCK_ICON_PADDING: i8 = 4;

fn restock_selector_columns(available_width: f32) -> usize {
    // Each full picker needs roughly 240 px for its six or seven icon cells.
    // Overview account cards are often only 350–500 px wide, so use stacked
    // full-width groups there instead of three unusably narrow columns.
    if available_width >= 900.0 { 3 } else { 1 }
}

#[cfg(test)]
mod restock_picker_layout_tests {
    use super::restock_selector_columns;
    #[test]
    fn full_width_groups_on_account_cards_and_three_columns_on_wide_panels() {
        for width in [230.0, 350.0, 480.0, 779.0, 899.0] {
            assert_eq!(restock_selector_columns(width), 1);
        }
        for width in [900.0, 1100.0, 1400.0] {
            assert_eq!(restock_selector_columns(width), 3);
        }
    }
}

// Rebase only the restock portion onto the latest worker preferences. This
// prevents an old restock draft from resetting Auto Catch's ball priorities,
// reserves or enabled state when the two settings pages are edited in sequence.
pub(super) fn copy_restock_fields(to: &mut CatchSettings, from: &CatchSettings) {
    to.restock_enabled = from.restock_enabled;
    to.gold_reserve = from.gold_reserve;
    to.ball_id = from.ball_id;
    to.potion_id = from.potion_id;
    to.revive_id = from.revive_id;
    to.ball_at = from.ball_at;
    to.ball_to = from.ball_to;
    to.potion_at = from.potion_at;
    to.potion_to = from.potion_to;
    to.revive_at = from.revive_at;
    to.revive_to = from.revive_to;
}

pub(super) fn restock_fields_equal(a: &CatchSettings, b: &CatchSettings) -> bool {
    let mut updated = b.clone();
    copy_restock_fields(&mut updated, a);
    updated == *b
}

// Use the worker's latest confirmed settings when processing any queued save.
// This is independent of whether the Overview/restock editor is currently open.
pub(super) fn prepare_restock_save(
    current: &CatchSettings,
    staged: &CatchSettings,
) -> Result<Option<CatchSettings>, String> {
    let mut next=current.clone();
    copy_restock_fields(&mut next,staged);
    if next==*current { return Ok(None); }
    next.validate()?;
    Ok(Some(next))
}

#[cfg(test)]
mod restock_selection_tests {
    use super::*;

    #[test]
    fn queued_restock_rebases_on_latest_capture_settings_after_navigation() {
        let mut staged=CatchSettings::default();
        staged.restock_enabled=true;
        staged.ball_id=5;
        staged.potion_id=203;
        staged.revive_id=206;
        staged.gold_reserve=500;
        let mut confirmed=CatchSettings::default();
        confirmed.enabled=true;
        confirmed.interval_ms=900;
        confirmed.priorities=vec!["beast".into()];
        let next=prepare_restock_save(&confirmed,&staged).unwrap().unwrap();
        assert_eq!(next.restock_enabled,true);
        assert_eq!(next.ball_id,5);
        assert_eq!(next.potion_id,203);
        assert_eq!(next.revive_id,206);
        assert_eq!(next.gold_reserve,500);
        assert!(next.enabled);
        assert_eq!(next.interval_ms,900);
        assert_eq!(next.priorities,vec!["beast"]);
        assert!(prepare_restock_save(&next,&staged).unwrap().is_none());
    }

    #[test]
    fn golden_potion_and_max_revive_preserve_independent_selections() {
        let mut staged = CatchSettings::default();
        staged.potion_id = 70070;
        staged.revive_id = 206;
        staged.restock_enabled = true;
        staged.validate().unwrap();

        let mut latest = CatchSettings::default();
        latest.enabled = true;
        latest.interval_ms = 800;
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 206);
        assert!(latest.enabled);
        assert_eq!(latest.interval_ms, 800);
        assert!(restock_fields_equal(&staged, &latest));
        assert_eq!(CatchSettings::from_value(Some(&latest.as_value())), latest);

        staged.revive_id = 205;
        assert!(!restock_fields_equal(&staged, &latest));
        copy_restock_fields(&mut latest, &staged);
        assert_eq!(latest.potion_id, 70070);
        assert_eq!(latest.revive_id, 205);
    }
}

fn capture_restock_chip(
    ui: &mut egui::Ui, visuals: &mut VisualCache, kind: SpriteKind,
    group: &'static str, selected: &mut u64, id: u64, name: &str, icon: &str,
    icon_hint: Option<&str>, price: Option<u64>, stock: Option<u64>,
) {
    let active = *selected == id;
    // A real Button owns both its artwork and pointer/keyboard interaction.
    // Interacting with a Frame's response after placing an Image inside it
    // let adjacent/wrapped controls compete for clicks, particularly at the
    // Golden Potion → Revives boundary. The ID is stable across icon loads.
    ui.push_id(("restock-item", group, id), |ui| {
        let button = if let Some(texture) = visuals.texture_id(ui, kind, id, icon_hint) {
            // Button::image clamps artwork to the text height; use the
            // regular Button atom to preserve the full 23px game icon.
            egui::Button::new(egui::Image::new((
                texture, egui::vec2(RESTOCK_ICON_SIZE, RESTOCK_ICON_SIZE)
            )))
        } else {
            egui::Button::new(RichText::new(icon).size(15.0))
        }
        .fill(if active { ACCENT.linear_multiply(0.25) } else { PANEL_ALT })
        .stroke(Stroke::new(if active { 1.5 } else { 1.0 },
            if active { ACCENT } else { BORDER }))
        .corner_radius(5.0)
        .selected(active);
        let price_hint = price.map(|p| format!(" · {p} gold")).unwrap_or_default();
        let stock_hint = stock.map(|n| format!(" · {n} in bag")).unwrap_or_default();
        if ui.add_sized(
            [RESTOCK_ICON_SIZE + 2.0 * RESTOCK_ICON_PADDING as f32;
             2], button
        ).on_hover_text(format!("{name}{price_hint}{stock_hint}")).clicked() {
            *selected = id;
        }
    });
}
fn capture_restock_threshold(
    ui:&mut egui::Ui,label:&str,at:&mut u64,to:&mut u64,
    current:Option<u64>
) {
    ui.horizontal_wrapped(|ui|{
        ui.add_sized([77.0,18.0],egui::Label::new(
            RichText::new(label).strong().size(10.0).color(TEXT)));
        ui.label(RichText::new(tr("at ≤")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(at).range(0..=1_000_000));
        ui.label(RichText::new(tr("to")).size(10.0).color(DIM));
        ui.add_sized([75.0,20.0],egui::DragValue::new(to).range(0..=1_000_000));
        if let Some(count)=current {
            ui.label(RichText::new(format!("{count} {}",tr("in bag"))).size(10.0)
                .color(if count <= *at {WARN} else {DIM}));
        }
    });
}
/// Live restock picker ported from Auto Catch+. Each account card edits only its
/// own authenticated native settings, with no item-ID input or explicit save step.
pub(super) fn draw_overview_restock(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    if !app.workspace.restock_open[index] { return; }
    request_item_catalog(app);
    app.visuals.poll(ui.ctx());
    let Some(native) = app.games[index].native.as_ref() else {
        ui.label(RichText::new(tr("Start this account to configure restock"))
            .size(10.0).color(WARN));
        return;
    };
    let current = native.capture_settings();
    if app.restock_drafts[index].is_none() {
        app.restock_drafts[index] = Some(current.clone());
    }
    let cfg = app.restock_drafts[index].as_mut().expect("restock draft initialized");
    let previous = cfg.clone();
    {
    let restock_status=native.snapshot().native_restock;
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
        ui.horizontal_wrapped(|ui|{
            ui.checkbox(&mut cfg.restock_enabled,tr("Automatic restock"));
            ui.label(RichText::new(tr(&restock_status.phase)).size(10.0)
                .color(if restock_status.pending {WARN} else {DIM}));
            ui.label(RichText::new(format!(
                "{} ordered · {} confirmed",
                restock_status.ordered,restock_status.confirmed
            )).size(10.0).color(DIM));
        });
        let game_catalog=native.hunt_snapshot();
        let catalog=app.inventory_catalog.as_deref();
        let selected_stock=|kind:&str,id:u64| -> Option<u64> {
            // The authoritative HuntView inventory contains every owned item.
            // RestockStatus only contains already-saved rules, so it cannot
            // give stock for a newly selected potion/revive until saving ends.
            let inventory = if kind == "ball" {
                game_catalog.balls.as_ref()
            } else {
                game_catalog.items.as_ref()
            };
            selected_restock_stock(inventory, id).or_else(||
                restock_status.rules.iter()
                    .find(|rule|rule.kind==kind && rule.id==id)
                    .and_then(|rule|rule.current))
        };
        let selected_price=|kind:&str,id:u64| -> Option<u64> {
            restock_status.rules.iter()
                .find(|rule|rule.kind==kind && rule.id==id)
                .and_then(|rule|rule.price)
        };
        ui.add_space(5.0);
        let columns = restock_selector_columns(ui.available_width());
        ui.columns(columns, |cols| {
        let potion_col = if columns == 3 { 1 } else { 0 };
        let revive_col = if columns == 3 { 2 } else { 0 };
        cols[0].label(RichText::new(tr("Balls")).strong().size(11.0).color(TEXT));
        cols[0].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,0,"Off","—",None,None,None);
            for (id, fallback) in [(1,"Poké Ball"),(2,"Great Ball"),
                (3,"Super Ball"),(4,"Ultra Ball"),(5,"Beast Ball")] {
                let name = game_catalog.ball_names.get(&id)
                    .map(String::as_str).unwrap_or(fallback);
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,name,"●",None,
                    selected_price("ball",id),selected_stock("ball",id));
            }
            if cfg.ball_id>5 && !game_catalog.ball_names.contains_key(&cfg.ball_id) {
                let id=cfg.ball_id;
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Ball,
                "ball", &mut cfg.ball_id,id,&format!("Saved ball #{id}"),"●",None,None,None);
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[potion_col].label(RichText::new(tr("Potions")).strong().size(11.0).color(TEXT));
        // Match Auto Catch+: Small, Ultimate and Golden Potion. Preserve
        // any existing selection even if the downloaded catalog is old.
        let mut potion_choices=vec![
            (200_u64,String::from("Small Potion")),
            (201_u64,String::from("Great Potion")),
            (202_u64,String::from("Ultra Potion")),
            (203_u64,String::from("Hyper Potion")),
            (204_u64,String::from("Ultimate Potion")),
            (70070_u64,String::from("Golden Potion"))
        ];
        if let Some(c) = catalog {
            for (id, item) in &c.items {
                let name = item.name.to_lowercase();
                if (name.contains("potion") || name.contains("poção") ||
                    name.contains("pocao")) && item.price_gold.unwrap_or(0)>0 &&
                    !potion_choices.iter().any(|(existing,_)|existing==id) {
                    potion_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.potion_id>0 && !potion_choices.iter().any(|(id,_)|*id==cfg.potion_id) {
            let name=catalog.map(|c|c.name(cfg.potion_id))
                .unwrap_or_else(||format!("Saved potion #{}",cfg.potion_id));
            potion_choices.push((cfg.potion_id,name));
        }
        cols[potion_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,0,"Off","—",None,None,None);
            for (id,name) in potion_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "potion", &mut cfg.potion_id,id,&name,"💊",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        if columns == 1 { cols[0].add_space(8.0); }
        cols[revive_col].label(RichText::new(tr("Revives")).strong().size(11.0).color(TEXT));
        let mut revive_choices=vec![
            (205_u64,String::from("Revive")),
            (206_u64,String::from("Max Revive"))
        ];
        if let Some(c)=catalog {
            for (id,item) in &c.items {
                if item.name.to_lowercase().contains("revive") &&
                    !revive_choices.iter().any(|(existing,_)|existing==id) {
                    revive_choices.push((*id,item.name.clone()));
                }
            }
        }
        if cfg.revive_id>0 && !revive_choices.iter().any(|(id,_)|*id==cfg.revive_id) {
            revive_choices.push((cfg.revive_id,format!("Saved revive #{}",cfg.revive_id)));
        }
        cols[revive_col].horizontal_wrapped(|ui|{
            capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,0,"Off","—",None,None,None);
            for (id,name) in revive_choices {
                let price=selected_price("item",id)
                    .or_else(||catalog.and_then(|c|c.items.get(&id))
                        .and_then(|item|item.price_gold));
                capture_restock_chip(ui,&mut app.visuals,SpriteKind::Item,
                "revive", &mut cfg.revive_id,id,&name,"✚",
                    catalog.and_then(|c|c.items.get(&id)).and_then(|m|m.icon.as_deref()),
                    price,selected_stock("item",id));
            }
        });
        });
        if cfg.restock_enabled && cfg.potion_id>0 {
            ui.label(RichText::new(tr(
                "Game Auto-Potion uses the potion selected here while restock is on. Its on/off switch and HP threshold stay unchanged."
            )).size(10.0).color(DIM));
        }
        if cfg.restock_enabled && cfg.revive_id>0 {
            ui.label(RichText::new(tr(
                "Game Auto-Revive uses the revive selected here while restock is on. Its on/off switch stays unchanged."
            )).size(10.0).color(DIM));
        }
        ui.add_space(10.0);
        ui.label(RichText::new(tr("Restock at ≤ / Fill to")).size(10.0).color(DIM));
        capture_restock_threshold(ui,tr("Balls"),&mut cfg.ball_at,&mut cfg.ball_to,
            selected_stock("ball",cfg.ball_id));
        capture_restock_threshold(ui,tr("Potions"),&mut cfg.potion_at,&mut cfg.potion_to,
            selected_stock("item",cfg.potion_id));
        capture_restock_threshold(ui,tr("Revives"),&mut cfg.revive_at,&mut cfg.revive_to,
            selected_stock("item",cfg.revive_id));
        ui.horizontal(|ui|{
            ui.label(RichText::new(tr("Gold reserve")).size(10.0).color(MUTED));
            ui.add(egui::DragValue::new(&mut cfg.gold_reserve).range(0..=u64::MAX));
        });
        if let Some(error)=restock_status.last_error.as_deref() {
            ui.label(RichText::new(tr(error)).size(10.0).color(WARN));
        }
    });
    }
    if *cfg != previous {
        app.restock_dirty[index] = true;
        // An explicit new edit unblocks an exhausted/permanent save error.
        app.restock_retries[index] = RestockSaveRetry::default();
        app.capture_save_error[index] = None;
        ui.ctx().request_repaint();
    }
    if let Some(error) = app.capture_save_error[index].as_deref() {
        ui.label(RichText::new(error).size(10.0).color(WARN));
    }
    if app.capture_save_busy[index] {
        ui.label(RichText::new(if app.restock_dirty[index] {
            tr("Restock changes queued; saving previous selection...")
        } else {
            tr("Saving encrypted preferences...")
        }).size(10.0).color(if app.restock_dirty[index] { WARN } else { DIM }));
    }
    // The save queue is dispatched by refresh_processes, never from the
    // rendered picker. It continues after navigation or closing this panel.
}

