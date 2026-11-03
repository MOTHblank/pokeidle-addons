//! Native account overview, live summaries and session history presentation.
use super::*;

// One coherent per-account sample is shared by the overview totals, status
// projection and card widgets. Without this, a four-account dashboard clones
// each potentially large HuntView several times per redraw.
pub(super) struct OverviewAccountSample {
    pub(super) health: Health,
    pub(super) view: Option<HuntView>,
    pub(super) native: Option<crate::native_background::NativeSnapshot>,
}

impl OverviewAccountSample {
    pub(super) fn capture(slot: &GameSlot) -> Self {
        let native = slot.native.as_ref().map(|session| session.snapshot());
        let view = slot.native.as_ref().map(|session| session.hunt_snapshot());
        let health = slot.health_with_native(native.as_ref().zip(view.as_ref()));
        Self { health, view, native }
    }
}

#[derive(Default)]
pub(super) struct OverviewTotals {
    online: usize,
    xp_h: f64,
    xp_samples: usize,
    gold_h: f64,
    gold_samples: usize,
    gold: u64,
    gold_balances: usize,
    gems: u64,
    gem_balances: usize,
}

// Only confirmed live snapshots contribute. Offline accounts are not assumed
// to have zero balances; these totals explicitly describe online balances.
pub(super) fn collect_overview_totals(
    app: &mut ControllerApp,
    profiles: &[usize],
    samples: &[Option<OverviewAccountSample>; 4],
) -> OverviewTotals {
    let mut result = OverviewTotals::default();
    for &index in profiles {
        let sample = samples[index].as_ref().expect("enabled account sampled");
        let health = &sample.health;
        let running = app.runtimes[index].game.is_running();
        if !running || !health.logged_in || !health.bridge_snapshot_ok ||
            (app.runtimes[index].game.native.is_none() && !health.telemetry_is_fresh()) {
            continue;
        }
        result.online += 1;
        if let (Some(view), Some(snapshot)) = (sample.view.as_ref(), sample.native.as_ref()) {
            if view.measurements.valid_trainer {
                result.xp_h += view.measurements.trainer_xp_h;
                result.xp_samples += 1;
            }
            if view.measurements.valid_gold {
                result.gold_h += view.measurements.gold_net_h;
                result.gold_samples += 1;
            }
            if let Some(value) = view.gold.or(snapshot.gold) {
                result.gold = result.gold.saturating_add(value);
                result.gold_balances += 1;
            }
            if let Some(value) = view.orbs.or(snapshot.orbs) {
                result.gems = result.gems.saturating_add(value);
                result.gem_balances += 1;
            }
        } else {
            if let Some(rate) = app.runtimes[index].telemetry.xp_rates.observe(health) {
                result.xp_h += rate;
                result.xp_samples += 1;
            }
            // Gold/h is not provided as an observed browser rate; do not
            // silently include Hunt Atlas estimates in a measured total.
            result.gold = result.gold.saturating_add(health.gold);
            result.gems = result.gems.saturating_add(health.orbs);
            result.gold_balances += 1;
            result.gem_balances += 1;
        }
    }
    result
}

pub(super) fn draw_overview_totals(ui: &mut egui::Ui, totals: &OverviewTotals) {
    let entries = [
        ("✧", "XP/h total", if totals.xp_samples > 0 {
            format_hunt_amount(totals.xp_h)
        } else { "—".into() }, ACCENT),
        ("◈", "Gold/h total", if totals.gold_samples > 0 {
            format_hunt_amount(totals.gold_h)
        } else { "—".into() }, WARN),
        ("●", "Online accounts", format!("{}/4", totals.online), GOOD),
        ("◆", "Live gold", if totals.gold_balances > 0 {
            format_number(totals.gold)
        } else { "—".into() }, WARN),
        ("◇", "Live gems", if totals.gem_balances > 0 {
            format_number(totals.gems)
        } else { "—".into() }, ACCENT),
    ];
    let gap = 8.0;
    let available = ui.available_width();
    let columns = if available >= 900.0 { 5 }
        else if available >= 550.0 { 3 } else { 2 };
    let tile_width = ((available - gap * (columns - 1) as f32) / columns as f32).max(90.0);
    for chunk in entries.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (symbol, label, value, color) in chunk {
                ui.allocate_ui_with_layout(egui::vec2(tile_width, 0.0),
                    Layout::top_down(Align::Min), |ui| {
                        egui::Frame::new().fill(PANEL)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(9.0).inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.set_width((tile_width - 20.0).max(70.0));
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(*symbol).size(15.0).color(*color));
                                    ui.add(egui::Label::new(
                                        RichText::new(tr(label)).size(10.0).color(MUTED)
                                    ).truncate());
                                });
                                ui.add(egui::Label::new(
                                    RichText::new(value).size(17.0).strong().color(TEXT)
                                ).truncate());
                            });
                    });
            }
        });
        ui.add_space(7.0);
    }
    ui.label(RichText::new(tr(
        "Verified online balances only; unavailable accounts are not counted."
    )).size(9.0).color(DIM))
        .on_hover_text(tr("Rates are measured, never projected; unavailable data stays blank."));
}

/// Current game inventory (server quantities) and separately battle-acquired
/// drops. Shop purchases are never labeled as loot.
pub(super) fn request_item_catalog(app:&mut ControllerApp) {
    if app.inventory_catalog.is_none() && app.inventory_catalog_error.is_none() &&
        !app.inventory_catalog_busy {
        app.inventory_catalog_busy=true;
        let tx=app.inventory_catalog_tx.clone();
        std::thread::spawn(move || {
            let result=std::panic::catch_unwind(native_items::load)
                .unwrap_or_else(|_|Err("Item catalog loader panicked".into()));
            let _=tx.send(result);
        });
    }
}
/// Offline pages should offer one useful next action instead of leaving an
/// almost entirely empty workspace. Authentication is never inferred complete.
pub(super) fn draw_native_missing_panel(
    app:&mut ControllerApp,
    ui:&mut egui::Ui,
    selected:Option<usize>,
    purpose:&str,
) {
    egui::Frame::new().fill(PANEL_ALT).corner_radius(8.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.label(RichText::new(purpose).size(13.0).strong().color(TEXT));
            if let Some(index)=selected {
                let profile=app.runtimes[index].game.profile;
                let running=app.runtimes[index].game.is_running();
                let name=app.account_name(profile);
                ui.label(RichText::new(format!("{} · {}",name,
                    if running { tr("Waiting for sign-in") }
                    else { tr("Account is stopped") }))
                    .size(11.0).color(if running { WARN } else { DIM }));
                ui.horizontal_wrapped(|ui|{
                    if ui.add_enabled(!running,egui::Button::new(tr("Start account")))
                        .clicked() {
                        app.launch_one(profile);
                    }
                    if ui.small_button(tr("Account settings")).clicked() {
                        app.workspace.panel_tab=PanelTab::Accounts;
                    }
                });
            } else {
                ui.label(RichText::new(tr("Choose an account to get started"))
                    .size(11.0).color(DIM));
                ui.horizontal_wrapped(|ui|{
                    if ui.small_button(tr("Launch enabled accounts")).clicked() {
                        app.launch_enabled();
                    }
                    if ui.small_button(tr("Manage accounts")).clicked() {
                        app.workspace.panel_tab=PanelTab::Accounts;
                    }
                });
            }
        });
}

pub(super) fn format_hunt_amount(value: f64) -> String {
    if !value.is_finite() { return "—".into(); }
    let negative = if value < 0.0 { "-" } else { "" };
    let num = value.abs();
    if num >= 1_000_000_000.0 { format!("{negative}{:.2}B", num / 1_000_000_000.0) }
    else if num >= 1_000_000.0 { format!("{negative}{:.2}M", num / 1_000_000.0) }
    else if num >= 1_000.0 { format!("{negative}{:.1}K", num / 1_000.0) }
    else { format!("{negative}{num:.0}") }
}

// Hunt Atlas lives in a dedicated child module, sharing the native state and
// rendering primitives without pulling browser JavaScript into the controller.
/// At-a-glance card: keep the operational summary visible for every enabled
/// account, independently of the much longer optional diagnostics panel.
pub(super) fn draw_overview_card(
    app: &mut ControllerApp, ui: &mut egui::Ui, index: usize, width: f32,
    sample: &OverviewAccountSample,
) {
    ui.set_width(width);
    let profile = app.runtimes[index].game.profile;
    let health = &sample.health;
    let running = app.runtimes[index].game.is_running();
    let native_view = sample.view.as_ref();
    let account_name = native_view
        .and_then(|view| view.account_nick.as_deref())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| app.account_name(profile));
    let xp_per_hour = if let Some(view) = native_view {
        if view.measurements.valid_trainer { Some(view.measurements.trainer_xp_h) } else { None }
    } else if running {
        app.runtimes[index].telemetry.xp_rates.observe(health)
    } else {
        app.runtimes[index].telemetry.xp_rates = XpRateTracker::default();
        None
    };
    // Bridge inactivity checks require Firefox telemetry; native battle
    // receipts are assessed independently by the Hunts panel.
    let idle_warning = if app.runtimes[index].game.native.is_some() {
        app.runtimes[index].telemetry.idle_progress = IdleProgressTracker::default();
        false
    } else { app.idle_warning_for(index, health) };
    // Only authentication/setup issues belong on the dashboard.
    // Only actual account sign-in problems appear here.
    let needs_setup = !app.runtimes[index].telemetry.account_setup.setup_complete
        && app.runtimes[index].game.native.is_none();
    // The authenticated game socket supplies the data directly. Browser
    // sampling intervals must not hide its last confirmed snapshot.
    let fresh = running && (health.telemetry_is_fresh()
        || sample.native.as_ref().is_some_and(|state|
            state.phase == "Online" && state.last_received.is_some()
        ));
    let repair_available = monitor_repair_available(
        running,
        health.last_error.as_deref(),
        app.runtimes[index].game.orphaned_pid.is_some() && app.runtimes[index].game.monitor.is_none(),
    );

    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, if app.workspace.selected_details == Some(index) { ACCENT } else { BORDER }))
        .corner_radius(12.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(
                    RichText::new(compact_text(&account_name, 21))
                        .size(14.0).strong().color(TEXT),
                ).truncate()).on_hover_text(account_name.as_str());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let expanded = app.workspace.selected_details == Some(index);
                    let label = tr(if expanded { "Hide details" } else { "Show details" });
                    if ui.small_button(label).clicked() {
                        app.workspace.selected_details = if expanded { None } else { Some(index) };
                        if !expanded { app.select_workspace_account(index); }
                    }
                });
            });
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                status_badge(ui, health, running);
                ui.label(RichText::new(if app.runtimes[index].game.native.is_some() {
                    tr("PokéIdle")
                } else if running { "Firefox" } else { tr("Offline") })
                    .size(10.0).color(DIM));
                if fresh {
                    if let Some(view) = native_view {
                        if view.bonuses.vip_active {
                            ui.label(RichText::new("VIP").size(10.0).strong().color(WARN));
                        }
                        if view.bonuses.guild_boost_active {
                            ui.label(RichText::new(tr("Guild boost")).size(10.0).color(GOOD));
                        }
                        for (label, percent, known) in [
                            ("Twitch", view.bonuses.twitch_pct, view.bonuses.twitch_known),
                            ("Guild", view.bonuses.guild_pct, view.bonuses.guild_known),
                            ("Event", view.bonuses.trainer_event_pct, view.bonuses.trainer_event_known),
                        ] {
                            if known && percent > 0.0 {
                                ui.label(RichText::new(format!("{label} +{percent:.0}%"))
                                    .size(10.0).color(GOOD));
                            }
                        }
                    }
                }
            });
            ui.add_space(8.0);

            // Preserve the last confirmed account snapshot through a temporary
            // transport outage. Never portray it as fresh live activity.
            if !fresh && native_view.is_some() && health.last_successful_probe_at.is_some() {
                ui.label(RichText::new(tr("Last confirmed data; reconnecting"))
                    .size(10.0).color(WARN));
            }
            draw_overview_matchup(app, ui, health, fresh, native_view);
            ui.add_space(6.0);
            if app.workspace.dashboard_session_view {
                draw_overview_session(ui, native_view);
            } else {
                draw_overview_account_summary(app, ui, index, fresh,
                    xp_per_hour, native_view);
            }
            draw_overview_restock(app, ui, index);
            ui.add_space(7.0);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let hunt = if !running { "—" } else if health.hunt.is_empty() {
                    match health.activity.as_str() {
                        "Center" => tr("Online · Center"),
                        "Login" => tr("Waiting for login"),
                        "Hunting" => tr("Hunting"),
                        _ => "—",
                    }
                } else { &health.hunt };
                ui.label(RichText::new(format!("⌖ {}: {}", tr("Current hunt"),
                    compact_text(hunt, 36))).size(10.0).color(MUTED))
                    .on_hover_text(hunt);
                // Native fields are Option-valued; an unreceived balance
                // must not masquerade as a legitimate zero.
                let verified = fresh && health.bridge_snapshot_ok;
                let balances = sample.native.as_ref();
                let cached_native = running && native_view.is_some()
                    && balances.as_ref().is_some_and(|s| s.last_received.is_some());
                let gold = if let Some(view) = native_view.filter(|_| verified || cached_native) {
                    view.gold.or_else(|| balances.as_ref().and_then(|s| s.gold))
                } else if verified { Some(health.gold) } else { None };
                let gems = if let Some(view) = native_view.filter(|_| verified || cached_native) {
                    view.orbs.or_else(|| balances.as_ref().and_then(|s| s.orbs))
                } else if verified { Some(health.orbs) } else { None };
                let suffix = if cached_native && !verified {
                    " · último dado"
                } else { "" };
                ui.label(RichText::new(format!("◆ {}: {}{}",
                    tr("Gold"), gold.map(format_number).unwrap_or_else(|| "—".into()), suffix))
                    .size(10.0).color(WARN));
                ui.label(RichText::new(format!("◇ {}: {}{}",
                    tr("Gems"), gems.map(format_number).unwrap_or_else(|| "—".into()), suffix))
                    .size(10.0).color(ACCENT));
            });

            ui.add_space(9.0);
            ui.horizontal_wrapped(|ui| {
                // Overview reports the saved Auto Catch switch, NOT temporary
                // throw readiness, inventory state or receipt timing. Those
                // operational diagnostics belong exclusively to Automation.
                let catch_enabled = app.runtimes[index].game.native.as_ref()
                    .map(|native|native.capture_settings().enabled)
                    .unwrap_or(health.autocatch_on);
                automation_badge(
                    ui, "Auto Catch",
                    tr(auto_catch_setting_label(catch_enabled)),
                    catch_enabled,
                );
                let bonus_confirmed = fresh && health.bridge_snapshot_ok
                    && health.stream_bonus_pct > 0.0 && !health.stream_watching.is_empty();
                let bonus_text = if bonus_confirmed {
                    format!("+{}% XP", format_pct(health.stream_bonus_pct))
                } else {
                    tr("Not confirmed").to_string()
                };
                automation_badge(ui, "XP bonus", &bonus_text, bonus_confirmed);
            });

            if needs_setup {
                ui.add_space(7.0);
                if ui.small_button(RichText::new(tr("Setup needed")).color(WARN)).clicked() {
                    app.select_workspace_account(index);
                    app.workspace.panel_tab = PanelTab::Accounts;
                }
            }
            if idle_warning {
                ui.add_space(5.0);
                ui.label(
                    RichText::new(tr("Idle warning: no EXP or Pokéballs used in 5 minutes"))
                        .size(10.0).color(WARN),
                );
            }
            if running && health.game_ready && health.logged_in
                && health.activity == "Hunting" && !health.bridge_connected {
                ui.label(
                    RichText::new(tr("Game progress unavailable"))
                        .size(9.0).color(DIM),
                );
            }
            if repair_available {
                ui.add_space(6.0);
                if ui.small_button(
                    RichText::new(tr("Restart Firefox to reconnect")).color(WARN)
                )
                .on_hover_text(tr("Restarts Firefox for this account. The game will disconnect briefly."))
                .clicked() {
                    app.repair_monitor(profile);
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                // Native and Firefox are mutually exclusive launch options.
                let action = tr(if !running { "Start account" }
                    else if app.runtimes[index].game.native.is_some() { "Stop" }
                    else if app.runtimes[index].game.browser_only { "Stop Firefox" }
                    else { "Stop login" });
                let (fill, stroke) = if running {
                    (BAD.linear_multiply(0.18), Stroke::new(1.0, BAD.linear_multiply(0.55)))
                } else {
                    (ACCENT.linear_multiply(0.8), Stroke::new(1.0, ACCENT))
                };
                if ui.add(
                    egui::Button::new(
                        RichText::new(action.to_uppercase())
                            .size(12.0)
                            .strong()
                            .color(TEXT),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(8.0),
                ).clicked() {
                    if running {
                        app.stop_one(profile);
                    } else {
                        app.launch_one(profile);
                    }
                }
                if ui.add(
                    egui::Button::new(
                        RichText::new(tr("Open in Firefox").to_uppercase())
                            .size(10.0).strong().color(TEXT),
                    )
                    .fill(PANEL_ALT)
                    .stroke(Stroke::new(1.0, BORDER))
                    .corner_radius(8.0),
                ).on_hover_text(tr("Play this account in a Firefox window instead of the background connection."))
                .clicked() {
                    app.open_in_firefox(profile);
                }
                // Keep diagnostics in the account details; show only a compact status.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let (Some(native), Some(game)) =
                        (app.runtimes[index].game.native.as_ref(), sample.native.as_ref()) {
                        let twitch = native.twitch_snapshot();
                        let online = game.phase == "Online" && twitch.phase == "Online";
                        let summary = if online {
                            format!("Twitch: {}/{} channels", twitch.joined, twitch.desired)
                        } else {
                            format!("Twitch: {}", tr(&twitch.phase))
                        };
                        ui.add(egui::Label::new(
                            RichText::new(summary).size(10.0)
                                .color(if online { GOOD } else { WARN })
                        ).truncate()).on_hover_text(format!(
                            "PokéIdle: {} · {} updates · {} battles · {} captures\nTwitch: {} · {}/{} channels",
                            game.phase, game.frames, game.battle_kills, game.captures,
                            twitch.phase, twitch.joined, twitch.desired
                        ));
                    } else if running {
                        let stage = if app.runtimes[index].game.browser_only {
                            tr("Firefox running")
                        } else if app.runtimes[index].session.native_verifying {
                            tr("Connecting to PokéIdle")
                        } else if app.runtimes[index].session.native_handoff_busy {
                            tr("Checking sign-in")
                        } else {
                            tr("Firefox login")
                        };
                        ui.add(egui::Label::new(
                            RichText::new(stage).size(10.0).color(WARN)
                        ).truncate()).on_hover_text(tr(if app.runtimes[index].game.browser_only {
                            "Firefox remains open until you stop this account."
                        } else {
                            "Firefox closes after sign-in. PokéIdle and Twitch keep running in the app."
                        }));
                    }
                });
            });
            if app.runtimes[index].game.native.is_none() {
                if let Some(error) = app.runtimes[index].session.native_error.as_deref() {
                    ui.add_space(5.0);
                    ui.add(egui::Label::new(
                        RichText::new(format!("{}: {}", tr("Connection error"), compact_text(&tr_dynamic(localize_status(error.to_owned())), 60)))
                            .size(10.0).color(BAD)
                    ).truncate()).on_hover_text(error);
                }
            }
        });
}

fn overview_stat_tile(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6)).show(ui, |ui| {
            ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
            ui.add(egui::Label::new(
                RichText::new(value).size(14.0).strong().color(color)
            ).truncate()).on_hover_text(value);
        });
}


// Dashboard stock always refers to the selected restock item, never the sum
// of every Poké Ball, potion or revive. Inventory snapshots omit zero-stock
// entries; a missing selected ID means zero only when that inventory category
// has actually been received. An empty map remains unknown.
pub(super) fn selected_restock_stock(inventory: &std::collections::BTreeMap<u64, u64>,
                          selected_id: u64) -> Option<u64> {
    if selected_id == 0 || inventory.is_empty() {
        None
    } else {
        Some(inventory.get(&selected_id).copied().unwrap_or(0))
    }
}

#[cfg(test)]
mod selected_restock_stock_tests {
    use super::selected_restock_stock;
    use std::collections::BTreeMap;

    #[test]
    fn counts_only_the_selected_ball_or_healing_item() {
        let balls = BTreeMap::from([(1, 125), (2, 73), (4, 8), (5, 1)]);
        assert_eq!(selected_restock_stock(&balls, 1), Some(125));
        assert_eq!(selected_restock_stock(&balls, 4), Some(8));
        assert_eq!(selected_restock_stock(&balls, 3), Some(0));

        let items = BTreeMap::from([
            (200, 14), (201, 9), (204, 6), (70070, 2),
            (205, 11), (206, 3), (999, 500),
        ]);
        assert_eq!(selected_restock_stock(&items, 200), Some(14));
        assert_eq!(selected_restock_stock(&items, 70070), Some(2));
        assert_eq!(selected_restock_stock(&items, 205), Some(11));
        assert_eq!(selected_restock_stock(&items, 206), Some(3));
    }

    #[test]
    fn unselected_and_unavailable_stock_is_not_reported_as_zero() {
        let available = BTreeMap::from([(200, 20), (205, 4)]);
        assert_eq!(selected_restock_stock(&available, 0), None);
        assert_eq!(selected_restock_stock(&BTreeMap::new(), 200), None);
        assert_eq!(selected_restock_stock(&BTreeMap::new(), 0), None);
    }
}

fn overview_resource_tile(app: &mut ControllerApp, ui: &mut egui::Ui,
                          index: usize, label: &str, count: Option<u64>,
                          kind: SpriteKind, selected: u64) {
    let expanded = app.workspace.restock_open[index];
    let response = egui::Frame::new().fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, if expanded { ACCENT } else { BORDER }))
        .corner_radius(7.0).inner_margin(Margin::symmetric(6, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr(label)).size(9.0).color(DIM));
                    ui.label(RichText::new(count.map(format_number)
                        .unwrap_or_else(|| "—".into()))
                        .size(12.0).strong().color(TEXT));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !app.visuals.show(ui, kind, selected, None, 23.0) {
                        ui.label(RichText::new(if selected == 0 { "×" } else { "◆" })
                            .size(15.0).color(ACCENT));
                    }
                });
            });
        }).response.interact(egui::Sense::click());
    if response.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("{} · {} · {}", tr(label),
            tr(if selected == 0 { "No restock item selected" } else { "Selected item only" }),
            tr("Configure restock")))
        .clicked() {
        app.workspace.restock_open[index] = !expanded;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OverviewCombatState { Offline, Center, Active, Hunting, Waiting, Stalled }

// A wild encounter is short-lived; the account's confirmed hunt is not.
// Never switch the overview to "between encounters" or clear its hunt artwork
// just because the battle server briefly sends selvagem:null.
pub(super) fn overview_combat_state(view: Option<&HuntView>, live: bool) -> OverviewCombatState {
    if !live { return OverviewCombatState::Offline; }
    let Some(view) = view else { return OverviewCombatState::Waiting; };
    if view.in_center { return OverviewCombatState::Center; }
    if view.hunt_slug.is_none() { return OverviewCombatState::Waiting; }
    // An encounter may be absent immediately after a kill. Recent combat
    // receipts still confirm that the selected hunt is producing activity.
    if view.measurements.combat_event_age_secs.is_some_and(|age| age <= 180) {
        return OverviewCombatState::Active;
    }
    if view.measurements.combat_event_age_secs.is_some_and(|age| age > 180) {
        return OverviewCombatState::Stalled;
    }
    OverviewCombatState::Hunting
}

// Use the hunt catalog, not the ephemeral wild slot, for a stable right card.
// Choosing the highest-weight species is representative artwork only; the
// card names the hunt rather than asserting this is the exact current enemy.
fn overview_hunt(view: &HuntView) -> Option<&native_hunts::HuntEntry> {
    let slug = view.hunt_slug.as_deref()?;
    view.hunts.iter().find(|entry| entry.slug == slug)
}
fn overview_hunt_art_species(view: &HuntView) -> Option<u64> {
    // Artwork follows the hunt's own species ID, never the current wild
    // encounter. A live HuntView retains the selected ID across kills.
    view.hunt_art_species.or_else(|| {
        overview_hunt(view).and_then(native_hunts::representative_hunt_species)
    })
}

fn overview_hunt_art_looktype(view: &HuntView) -> Option<u64> {
    overview_hunt(view).and_then(|hunt| hunt.looktype).filter(|id| *id > 0)
}

// Looktypes are the game's visual IDs; species IDs require a separate
// catalog lookup. Never treat an arbitrary species ID as a looktype.
// 50% larger than the previous 48px overview sprites on both sides.
const OVERVIEW_POKEMON_SPRITE_SIZE: f32 = 72.0;

fn overview_combat_image(
    visuals: &mut VisualCache,
    ui: &mut egui::Ui,
    preferred_looktype: Option<u64>,
    regular_looktype: Option<u64>,
    species_id: Option<u64>,
    opponent: bool,
) {
    let texture = [
        (SpriteKind::PokemonLooktype, preferred_looktype),
        (SpriteKind::PokemonLooktype, regular_looktype),
        (SpriteKind::Pokemon, species_id),
    ].into_iter().find_map(|(kind, id)| {
        id.filter(|id| *id > 0)
            .and_then(|id| visuals.pokemon_texture_sized(ui, kind, id))
    });
    egui::Frame::new()
        .fill(PANEL_ALT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            if let Some((texture, image_size)) = texture {
                // Keep the existing 72px slot/frame; scale only the visible
                // Pokémon, centered and without stretching rectangular sprites.
                let (slot, _) = ui.allocate_exact_size(
                    egui::vec2(OVERVIEW_POKEMON_SPRITE_SIZE, OVERVIEW_POKEMON_SPRITE_SIZE),
                    egui::Sense::hover(),
                );
                let longest = image_size.x.max(image_size.y);
                if longest > 0.0 {
                    let size = image_size * (OVERVIEW_POKEMON_SPRITE_SIZE / longest);
                    ui.painter().image(texture,
                        egui::Rect::from_center_size(slot.center(), size),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE);
                }
            } else {
                ui.add_sized([OVERVIEW_POKEMON_SPRITE_SIZE; 2], egui::Label::new(
                    RichText::new(if opponent { "◇" } else { "◈" })
                        .size(26.0).color(if opponent { WARN } else { ACCENT })));
            }
        });
}

/// Matchup is always visible, even when the card's lower metrics show Session.
fn draw_overview_matchup(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    health: &Health,
    fresh: bool,
    view: Option<&HuntView>,
) {
    // Preserve the last confirmed identity and artwork during a reconnect.
    // Connectivity and combat activity below still require fresh telemetry.
    let pokemon = view.and_then(|v|v.active_pokemon.as_ref());
    let hunt = view.filter(|v| !v.in_center).and_then(overview_hunt);
    let hunt_species = view.filter(|v| !v.in_center)
        .and_then(overview_hunt_art_species);
    let hunt_looktype = view.filter(|v| !v.in_center)
        .and_then(overview_hunt_art_looktype);
    let pokemon_name = pokemon.map(|p|p.name.as_str())
        .filter(|name|!name.is_empty())
        .or_else(|| (fresh && !health.active_pokemon.is_empty())
            .then_some(health.active_pokemon.as_str())).unwrap_or("—");
    let pokemon_level = pokemon.map(|p|p.level.to_string())
        .filter(|level|level!="0")
        .unwrap_or_else(|| if fresh && !health.pokemon_level.is_empty() {
            health.pokemon_level.clone()
        } else { "—".into() });
    let combat = overview_combat_state(view, fresh);
    ui.columns(2, |cols| {
        cols[0].label(RichText::new(tr("Your Pokémon"))
            .size(9.0).color(DIM));
        cols[0].horizontal(|ui| {
            overview_combat_image(&mut app.visuals, ui,
                pokemon.and_then(|p| if p.shiny { p.shiny_looktype } else { p.looktype }),
                pokemon.and_then(|p| p.looktype),
                pokemon.and_then(|p| p.species_id), false);
            ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(
                    compact_text(pokemon_name, 16)).size(11.0)
                    .strong().color(TEXT)).truncate()).on_hover_text(pokemon_name);
                ui.label(RichText::new(format!("Lv. {pokemon_level}"))
                    .size(9.0).color(DIM));
            });
        });
        // The right half tracks the selected hunt, not the transient opponent
        // slot. The artwork represents a species encountered in that hunt.
        cols[1].with_layout(Layout::top_down(Align::Max), |ui| {
            ui.label(RichText::new(tr("Current hunt"))
                .size(9.0).color(DIM));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                overview_combat_image(&mut app.visuals, ui,
                    None, hunt_looktype, hunt_species, true);
                ui.vertical(|ui| {
                    let name = if combat == OverviewCombatState::Center {
                        tr("At Pokémon Center")
                    } else {
                        hunt.map(|h| h.name.as_str())
                            .filter(|s| !s.is_empty())
                            .or_else(|| (fresh && !health.hunt.is_empty())
                                .then_some(health.hunt.as_str()))
                            .unwrap_or("—")
                    };
                    ui.add(egui::Label::new(RichText::new(compact_text(name, 21))
                        .size(11.0).strong().color(TEXT))
                        .truncate()).on_hover_text(name);
                    let level = hunt.filter(|h| h.level > 0)
                        .map(|h| format!("Lv. {}", h.level))
                        .unwrap_or_else(|| "—".into());
                    ui.label(RichText::new(level).size(9.0).color(DIM));
                });
            });
        });
    });
    let (state_label, color)=match combat {
        OverviewCombatState::Active=>(tr("Hunt active"),GOOD),
        OverviewCombatState::Hunting=>(tr("Hunting"),GOOD),
        OverviewCombatState::Center=>(tr("At Pokémon Center"),WARN),
        OverviewCombatState::Waiting=>(tr("Waiting for combat"),WARN),
        OverviewCombatState::Stalled=>(tr("In hunt · no recent activity"),WARN),
        OverviewCombatState::Offline=>(tr("No live connection"),DIM),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("● {state_label}"))
            .size(11.0).strong().color(color));
        if let Some(age)=view.filter(|_|fresh)
            .and_then(|v|v.measurements.combat_event_age_secs) {
            ui.label(RichText::new(format!("{}: {age}s",
                tr("Last combat activity"))).size(9.0).color(DIM));
        }
    });
    ui.add_space(5.0);
}

fn draw_overview_account_summary(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    index: usize,
    fresh: bool,
    xp_per_hour: Option<f64>,
    view: Option<&HuntView>,
) {
    // Cached native measurements remain visible while marked as historical;
    // they must never contribute to the live aggregate totals.
    let showing_last_known = !fresh && view.is_some();
    let gold_h = view.and_then(|v|
        v.measurements.valid_gold.then_some(v.measurements.gold_net_h));
    // The restock editor stages selections while the encrypted save runs.
    // Use those staged IDs immediately so the icon and count always agree.
    let restock_selection = app.runtimes[index].capture.restock_draft.as_ref().cloned()
        .or_else(|| app.runtimes[index].game.native.as_ref().map(|n| n.capture_settings()))
        .unwrap_or_default();
    let balls = view.and_then(|v|
        selected_restock_stock(v.balls.as_ref(), restock_selection.ball_id));
    let potions = view.and_then(|v|
        selected_restock_stock(v.items.as_ref(), restock_selection.potion_id));
    let revives = view.and_then(|v|
        selected_restock_stock(v.items.as_ref(), restock_selection.revive_id));
    ui.columns(2, |cols| {
        overview_stat_tile(&mut cols[0],"XP/h",
            &if fresh || showing_last_known {xp_per_hour.map(format_hunt_amount)
                .unwrap_or_else(||"—".into())}else{"—".into()},
            if showing_last_known { DIM } else { ACCENT });
        overview_stat_tile(&mut cols[1],"Gold/h",
            &gold_h.map(format_hunt_amount)
                .unwrap_or_else(||"—".into()),WARN);
    });
    ui.add_space(4.0);
    ui.columns(3, |cols| {
        overview_resource_tile(app,&mut cols[0],index,"Potions",
            potions,SpriteKind::Item,restock_selection.potion_id);
        overview_resource_tile(app,&mut cols[1],index,"Balls",
            balls,SpriteKind::Ball,restock_selection.ball_id);
        overview_resource_tile(app,&mut cols[2],index,"Revives",
            revives,SpriteKind::Item,restock_selection.revive_id);
    });
}

#[cfg(test)]
mod overview_combat_tests {
    use super::{overview_combat_state, overview_hunt_art_species, OverviewCombatState};
    use crate::native_hunts::{HuntEntry, HuntSpecies, HuntView, WildInfo};
    use std::sync::Arc;

    #[test]
    fn confirmed_hunt_stays_visible_across_encounters() {
        let mut view = HuntView { hunt_slug: Some("scyther-field".into()),
            ..Default::default() };
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Hunting);
        view.wild = Some(WildInfo { name: "Scyther".into(), ..Default::default() });
        view.measurements.combat_event_age_secs = Some(10);
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Active);
        view.wild = None; // selvagem:null between fights
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Active);
        view.measurements.combat_event_age_secs = Some(181);
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Stalled);
        assert_eq!(overview_combat_state(Some(&view), false), OverviewCombatState::Offline);
        view.in_center = true;
        assert_eq!(overview_combat_state(Some(&view), true), OverviewCombatState::Center);
    }

    #[test]
    fn overview_art_is_the_hunt_species_not_the_current_wild() {
        let mut view = HuntView { hunt_slug: Some("field-a".into()),
            hunts: Arc::new(vec![
                HuntEntry { slug: "field-a".into(), species: vec![
                    HuntSpecies { id: 2006, weight: 1.0 },
                ], ..Default::default() },
                HuntEntry { slug: "field-b".into(), species: vec![
                    HuntSpecies { id: 14009, weight: 1.0 }
                ], ..Default::default() },
            ]), ..Default::default() };
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.wild = Some(WildInfo { species_id: Some(127), ..Default::default() });
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.wild = None;
        assert_eq!(overview_hunt_art_species(&view), Some(2006));
        view.hunt_slug = Some("field-b".into());
        assert_eq!(overview_hunt_art_species(&view), Some(14009));
        view.hunt_art_species = Some(14009);
        assert_eq!(overview_hunt_art_species(&view), Some(14009));
    }
}

fn rare_session_drop_count(drops: &std::collections::BTreeMap<String, u64>,
                           matches: &[&str]) -> u64 {
    drops.iter()
        .filter(|(name, _)| {
            let name = name.to_lowercase();
            matches.iter().any(|term| name.contains(term))
        })
        .map(|(_, quantity)| *quantity)
        .fold(0u64, u64::saturating_add)
}

fn draw_overview_session(ui: &mut egui::Ui, view: Option<&HuntView>) {
    let Some(view) = view else {
        ui.label(RichText::new(tr("Start an account to see session details"))
            .size(11.0).color(MUTED));
        return;
    };
    let session = &view.session;
    ui.columns(3, |cols| {
        overview_stat_tile(&mut cols[0], "Session duration",
            &format!("{:02}:{:02}:{:02}",
                session.duration_secs / 3600,
                (session.duration_secs / 60) % 60,
                session.duration_secs % 60), TEXT);
        overview_stat_tile(&mut cols[1], "Kills", &format_number(session.kills), TEXT);
        overview_stat_tile(&mut cols[2], "Captures", &format_number(session.captures), TEXT);
    });
    ui.add_space(6.0);
    ui.label(RichText::new(tr("Rare drops")).size(9.0).strong().color(DIM));
    let drops: [(&str, &[&str]); 4] = [
        ("Boss Token", &["boss token"]),
        ("Key", &["chave", "key"]),
        ("Shiny Stone", &["shiny stone"]),
        ("Bicycle", &["bicicleta", "bicycle"]),
    ];
    ui.columns(4, |cols| {
        for (col, (label, matches)) in cols.iter_mut().zip(drops.iter()) {
            overview_stat_tile(col, label,
                &format_number(rare_session_drop_count(&session.acquired, matches)), TEXT);
        }
    });
    ui.add_space(6.0);
    ui.columns(2, |cols| {
        overview_stat_tile(&mut cols[0], "shinies seen",
            &format_number(session.shiny_seen), ACCENT);
        overview_stat_tile(&mut cols[1], "shinies caught",
            &format_number(session.shiny_captured), GOOD);
    });
}

#[cfg(test)]
mod dashboard_overview_tests {
    use super::rare_session_drop_count;
    use std::collections::BTreeMap;

    #[test]
    fn session_drop_tiles_count_only_matching_battle_drops() {
        let drops = BTreeMap::from([
            ("Boss Token".to_string(), 2),
            ("Boss Token Fragment".to_string(), 3),
            ("Great Ball".to_string(), 100),
            ("Chave Dourada".to_string(), 1),
        ]);
        assert_eq!(rare_session_drop_count(&drops, &["boss token"]), 5);
        assert_eq!(rare_session_drop_count(&drops, &["chave", "key"]), 1);
        assert_eq!(rare_session_drop_count(&drops, &["bicycle"]), 0);
    }
}

/// Aggregated, 90-day battle history is loaded from the same account's
/// local disk file and updated by the native game worker. Never reconstruct
/// historical XP by adding Pokémon and trainer rewards together.
pub(super) fn draw_history_panel(app: &mut ControllerApp, ui: &mut egui::Ui, index: usize) {
    ui.label(RichText::new(tr("Battle history")).size(14.0)
        .strong().color(TEXT));
    let runtime = &mut app.runtimes[index];
    let profile=runtime.game.profile;
    let history=if let Some(native)=runtime.game.native.as_ref() {
        runtime.hunt.offline_history=None;
        native.history_snapshot()
    } else {
        if ui.small_button(tr("Reload saved history")).clicked() {
            runtime.hunt.offline_history=None;
        }
        runtime.hunt.offline_history
            .get_or_insert_with(||crate::native_history::load_offline(profile)).clone()
    };
    if !history.owner.is_empty() {
        ui.label(RichText::new(format!("{}: {}",tr("Account"),history.owner))
            .size(11.0).color(MUTED));
    }
    let totals=history.totals();
    ui.add_space(8.0);
    ui.label(RichText::new(tr(
        "Up to 90 days of battle history, saved locally for each account."
    )).size(10.0).color(DIM));
    if let Some(error)=history.load_error.as_deref(){
        ui.colored_label(WARN,tr_dynamic(format!("History load: {error}")));
    }
    if let Some(error)=history.save_error.as_deref(){
        ui.colored_label(WARN,tr_dynamic(format!("History save: {error}")));
    }
    ui.add_space(9.0);
    egui::Frame::new().fill(PANEL).corner_radius(10.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
        ui.label(RichText::new(tr("Lifetime totals")).size(13.0).strong().color(TEXT));
        ui.columns(3,|cols|{
            detail_stat_line(&mut cols[0],tr("kills"),&format_number(totals.kills));
            detail_stat_line(&mut cols[0],tr("captures"),&format_number(totals.captures));
            detail_stat_line(&mut cols[1],tr("Trainer XP earned"),&format_number(totals.trainer_xp));
            detail_stat_line(&mut cols[1],tr("Pokémon XP earned"),&format_number(totals.pokemon_xp));
            detail_stat_line(&mut cols[2],tr("Combat gold"),&format_number(totals.combat_gold));
            detail_stat_line(&mut cols[2],tr("Auto-sale gold"),&format_number(totals.auto_sale_gold));
        });
        ui.label(format!("{}: {} · {}: {}",tr("shinies seen"),totals.shiny_seen,
            tr("shinies caught"),totals.shiny_captured));
    });
    // Only show days containing activity. Rendering fourteen empty progress
    // bars made history look like an enormous blank table for new accounts.
    if let Some(last_day)=history.days.iter().map(|entry|entry.day).max() {
        let mut points=[(0u64,0u64,0u64);14];
        for (index,point) in points.iter_mut().enumerate() {
            point.0=last_day.saturating_sub((13-index) as u64);
        }
        for entry in &history.days {
            let age=last_day.saturating_sub(entry.day);
            if entry.day<=last_day && age<14 {
                let point=&mut points[13-age as usize];
                point.1=point.1.saturating_add(entry.kills);
                point.2=point.2.saturating_add(entry.captures);
            }
        }
        let active=points.iter().filter(|(_,kills,captures)|
            *kills>0 || *captures>0).collect::<Vec<_>>();
        if !active.is_empty() {
            let max_kills=active.iter().map(|(_,kills,_)|*kills)
                .max().unwrap_or(0).max(1);
            ui.add_space(6.0);
            egui::Frame::new().fill(PANEL_ALT).corner_radius(8.0)
                .inner_margin(egui::Margin::same(10)).show(ui,|ui|{
                    ui.label(RichText::new(tr("Recent active days · kills / captures"))
                        .size(12.0).strong().color(TEXT));
                    for (day,kills,captures) in active.into_iter().rev().take(7) {
                        ui.horizontal(|ui|{
                            ui.add_sized([83.0,16.0],egui::Label::new(
                                RichText::new(history_day_label(*day)).size(10.0).color(DIM)));
                            ui.add(egui::ProgressBar::new(*kills as f32/max_kills as f32)
                                .desired_width(ui.available_width().clamp(100.0,350.0))
                                .text(format!("{kills} / {captures}")));
                        });
                    }
                });
        }
    }
    ui.add_space(8.0);
    let mut entries=history.days.iter().collect::<Vec<_>>();
    entries.sort_by(|a,b|b.day.cmp(&a.day).then_with(||a.hunt.cmp(&b.hunt)));
    let total_entries=entries.len();
    let displayed_entries=total_entries.min(app.workspace.history_rows_limit);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Hunt activity")).size(13.0).strong().color(TEXT));
        ui.label(RichText::new(format!("{} / {} {}", displayed_entries,
            total_entries, tr("recorded hunt-days"))).size(10.0).color(DIM));
    });
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(8)).show(ui,|ui|{
            egui::ScrollArea::vertical().id_salt(("history-rows",index))
                .max_height(360.0).show(ui,|ui|{
                    if entries.is_empty() {
                        ui.label(tr("No recorded battles yet"));
                    } else {
                        egui::Grid::new(("history-grid",index))
                            .striped(true).spacing([12.0,6.0])
                            .show(ui,|ui|{
                                for heading in ["Date","Hunt","Kills","Caught",
                                    "Trainer XP","Pokémon XP","Gold","Drops"] {
                                    ui.label(RichText::new(tr(heading))
                                        .size(10.0).strong().color(MUTED));
                                }
                                ui.end_row();
                                for day in entries.into_iter().take(displayed_entries) {
                                    ui.label(RichText::new(history_day_label(day.day)).size(10.0));
                                    ui.label(RichText::new(compact_text(&day.hunt,25)).size(10.0))
                                        .on_hover_text(&day.hunt);
                                    ui.label(format_number(day.kills));
                                    ui.label(format_number(day.captures));
                                    ui.label(format_number(day.trainer_xp));
                                    ui.label(format_number(day.pokemon_xp));
                                    ui.label(format_number(day.combat_gold.saturating_add(day.auto_sale_gold)))
                                        .on_hover_text(tr_dynamic(format!("Combat: {} · Auto-sale: {}",
                                            format_number(day.combat_gold),
                                            format_number(day.auto_sale_gold))));
                                    let drop_text=if day.drops.is_empty() {
                                        "—".to_owned()
                                    } else {
                                        format!("{}",day.drops.len())
                                    };
                                    let drops=day.drops.iter()
                                        .map(|(name,qty)|format!("{name} ×{qty}"))
                                        .collect::<Vec<_>>().join("\n");
                                    ui.label(drop_text).on_hover_text(if drops.is_empty() {
                                        tr("No drops recorded").to_owned()
                                    } else {drops});
                                    ui.end_row();
                                }
                            });
                    }
                });
        });
    if displayed_entries<total_entries {
        if ui.small_button(tr("Show 100 more hunt-days")).clicked() {
            app.workspace.history_rows_limit=app.workspace.history_rows_limit.saturating_add(100);
        }
    }
    ui.add_space(7.0);
    ui.label(RichText::new(tr(
        "History begins when this version first records a battle. Previous in-memory sessions cannot be recovered retroactively."
    )).size(10.0).color(DIM));
}
fn history_day_label(day:u64)->String {
    // Civil-from-days (Gregorian UTC), no OS locale or timezone guesses.
    let z=day as i64+719_468;
    let era=if z>=0{z}else{z-146_096}/146_097;
    let doe=z-era*146_097;
    let yoe=(doe-doe/1460+doe/36_524-doe/146_096)/365;
    let mut year=yoe+era*400;
    let doy=doe-(365*yoe+yoe/4-yoe/100);
    let mp=(5*doy+2)/153;
    let d=doy-(153*mp+2)/5+1;
    let month=mp+if mp<10{3}else{-9};
    if month<=2 {year+=1;}
    format!("{year:04}-{month:02}-{d:02}")
}
#[cfg(test)]
mod history_ui_tests {
    use super::history_day_label;
    #[test]
    fn utc_epoch_date_and_leap_day_are_stable() {
        assert_eq!(history_day_label(0),"1970-01-01");
        assert_eq!(history_day_label(19782),"2024-02-29");
    }
}

/// Dedicated native automation workspace. Keeps settings out of the
/// scrollable dashboard and provides a stable per-account selection.
/// Consistent selected/unselected workspace navigation used by Automation
/// and Hunt Atlas. Tooltips explain controls without inflating the toolbar.
pub(super) fn workspace_tab_button(
    ui: &mut egui::Ui, label: &str, selected: bool, hint: &str,
) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(tr(label)).size(12.0)
            .strong().color(if selected { TEXT } else { MUTED }))
            .fill(if selected { ACCENT.linear_multiply(0.20) } else { PANEL })
            .stroke(Stroke::new(1.0, if selected { ACCENT } else { BORDER }))
            .corner_radius(7.0),
    ).on_hover_text(tr(hint))
}

