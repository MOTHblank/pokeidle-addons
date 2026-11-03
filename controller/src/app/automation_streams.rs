//! Read-only stream watching, bonus and browser-tab summaries from Health.
use super::{bonus_chip, compact_text, egui, format_pct, tr, Health,
    RichText, DIM, GOOD, MUTED, PANEL_ALT, TEXT, WARN};

pub(super) fn draw_stream_bonus_details(ui: &mut egui::Ui, health: &Health) {
            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("STREAMS & BONUS"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if !health.bridge_snapshot_ok {
                        ui.label(RichText::new(tr("Twitch XP bonus unavailable until game data loads")).size(10.0).color(DIM));
                    } else if !health.stream_watching.is_empty() && health.stream_bonus_pct > 0.0 {
                        ui.label(
                            RichText::new(format!(
                                "{} · +{}% XP",
                                tr("ACTIVE"),
                                format_pct(health.stream_bonus_pct)
                            ))
                            .size(11.0)
                            .strong()
                            .color(GOOD),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                tr("Watching:"),
                                health.stream_watching.join(", ")
                            ))
                            .size(10.0)
                            .color(MUTED),
                        );
                    } else if !health.stream_missing.is_empty() {
                        ui.label(
                            RichText::new(tr("LIVE BONUS AVAILABLE"))
                                .size(10.0)
                                .strong()
                                .color(WARN),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}{}",
                                tr("Bonus not registered for:"),
                                health.stream_missing.join(", "),
                                if health.stream_bonus_pct > 0.0 {
                                    format!(" · +{}% XP", format_pct(health.stream_bonus_pct))
                                } else {
                                    String::new()
                                }
                            ))
                            .size(10.0)
                            .color(TEXT),
                        );
                    } else {
                        ui.label(
                            RichText::new(tr(if health.stream_watching.is_empty() { "No Twitch XP bonus confirmed by the game" } else { "Chat activity reported; XP bonus not confirmed" }))
                                .size(10.0)
                                .color(DIM),
                        );
                    }
                    ui.label(RichText::new(tr("Twitch channels and game-reported XP bonuses are different signals.")).size(9.0).color(DIM));

                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in health.tabs.iter().filter(|tab| {
                            tab.kind == "Twitch" || tab.kind == "KICK"
                        }) {
                            let state = if tab.kind == "Twitch" {
                                if tab.low_resource { tr("LOW") } else { tr("not optimized") }
                            } else {
                                tr("OPEN")
                            };

                            ui.label(
                                RichText::new(format!(
                                    "{} · {} · {}",
                                    tab.kind,
                                    compact_text(
                                        tab.title.rsplit('/').next().unwrap_or(&tab.title),
                                        24
                                    ),
                                    state
                                ))
                                .size(9.0)
                                .color(if tab.kind == "Twitch" && tab.low_resource {
                                    GOOD
                                } else {
                                    MUTED
                                }),
                            );
                        }
                    });
                });

            ui.add_space(12.0);
            ui.label(
                RichText::new(tr("XP BONUS SOURCES"))
                    .size(9.0)
                    .strong()
                    .color(DIM),
            );
            ui.add_space(6.0);

            egui::Frame::new()
                .fill(PANEL_ALT)
                .corner_radius(10.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    let mut sources = health.xp_sources.clone();
                    for bonus in &health.xp_bonuses {
                        if !sources.iter().any(|source| source == bonus) {
                            sources.push(bonus.clone());
                        }
                    }

                    if sources.is_empty() {
                        ui.label(
                            RichText::new(tr(if health.bridge_snapshot_ok { "No active XP bonus detected" } else { "XP bonus data unavailable" }))
                                .size(10.0)
                                .color(DIM),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for source in sources {
                                bonus_chip(ui, &source);
                            }
                        });
                    }
                });

}
