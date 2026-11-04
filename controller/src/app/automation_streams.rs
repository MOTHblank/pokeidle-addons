//! Read-only stream redemption, XP bonus and browser-tab summaries from Health.
use super::{bonus_chip, compact_text, egui, format_pct, tr, Health,
    RichText, DIM, GOOD, MUTED, PANEL_ALT};

fn twitch_clock_ms(health:&Health)->u64 {
    if health.twitch_server_now_ms>0 {
        return health.twitch_server_now_ms.saturating_add(
            health.last_successful_probe_at.map(|at|at.elapsed().as_millis() as u64)
                .unwrap_or(0));
    }
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_millis() as u64
}

fn remaining_label(until:u64,now:u64)->String {
    let secs=until.saturating_sub(now)/1000;
    if secs>=3600 {
        let hours=secs/3600;
        let minutes=(secs%3600)/60;
        format!("{hours}h {minutes:02}m")
    } else if secs>=60 {
        format!("{}m",secs/60)
    } else {
        format!("{}s",secs)
    }
}

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
                ui.label(RichText::new(tr("Twitch bonus data unavailable until game data loads"))
                    .size(10.0).color(DIM));
            } else {
                let now=twitch_clock_ms(health);
                match health.twitch_linked {
                    Some(true) => {
                        let account=if health.twitch_login.is_empty() {
                            tr("Twitch linked").to_owned()
                        } else {
                            format!("{} · {}",tr("Twitch linked"),health.twitch_login)
                        };
                        ui.label(RichText::new(account).size(10.0).color(MUTED));
                    }
                    Some(false) => {
                        ui.label(RichText::new(tr("Twitch account not linked"))
                            .size(10.0).color(DIM));
                    }
                    None => {
                        ui.label(RichText::new(tr("Twitch linkage not reported by the game"))
                            .size(10.0).color(DIM));
                    }
                }

                let mut active=0usize;
                for (label,until) in [
                    ("XP",health.twitch_xp_until),
                    ("Capture Boost",health.twitch_capture_until),
                    ("Secret Lure",health.twitch_shiny_until),
                ] {
                    if until<=now {continue}
                    active+=1;
                    let pct=if health.twitch_bonus_pct>0.0 {
                        format!("+{}% ",format_pct(health.twitch_bonus_pct))
                    } else {String::new()};
                    ui.label(RichText::new(format!(
                        "{} · {}{} · {}",
                        tr("ACTIVE"),pct,tr(label),remaining_label(until,now)
                    )).size(10.0).strong().color(GOOD));
                }
                if active==0 && health.twitch_linked==Some(true) {
                    ui.label(RichText::new(tr("No Twitch channel-point bonus is active"))
                        .size(10.0).color(DIM));
                }
                ui.label(RichText::new(tr(
                    "Twitch bonuses come from channel-point redemptions. Chat presence is not tracked by Moth Controller."
                )).size(9.0).color(DIM));
            }

            let kick_tabs=health.tabs.iter().filter(|tab|tab.kind=="KICK")
                .collect::<Vec<_>>();
            if !kick_tabs.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for tab in kick_tabs {
                        ui.label(
                            RichText::new(format!(
                                "KICK · {} · {}",
                                compact_text(tab.title.rsplit('/').next().unwrap_or(&tab.title),24),
                                tr("OPEN")
                            )).size(9.0).color(MUTED),
                        );
                    }
                });
            }
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
                    RichText::new(tr(if health.bridge_snapshot_ok {
                        "No active XP bonus detected"
                    } else {
                        "XP bonus data unavailable"
                    })).size(10.0).color(DIM),
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
