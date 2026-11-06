//! Read-only server-authoritative stream redemption and XP bonus summaries.
use crate::monitor::StreamBonusState;
use super::{bonus_chip, egui, format_pct, tr, Health, RichText, DIM, GOOD, MUTED, PANEL_ALT};

fn stream_clock_ms(health:&Health)->u64 {
    if health.stream_bonuses.server_now_ms>0 {
        return health.stream_bonuses.server_now_ms.saturating_add(
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

fn draw_stream_bonus(ui:&mut egui::Ui,name:&str,bonus:StreamBonusState,now:u64) {
    ui.label(RichText::new(name).size(10.0).strong().color(MUTED));
    let mut active=0usize;
    for (label,until) in [
        ("XP",bonus.xp_until),
        ("Capture Boost",bonus.capture_until),
        ("Secret Lure",bonus.shiny_until),
    ] {
        if until<=now {continue}
        active+=1;
        let pct=if bonus.pct>0.0 {
            format!("+{}% · ",format_pct(bonus.pct))
        } else {String::new()};
        ui.label(RichText::new(format!(
            "{} · {}{} · {}",
            tr("ACTIVE"),pct,tr(label),remaining_label(until,now)
        )).size(10.0).strong().color(GOOD));
    }
    if active==0 {
        ui.label(RichText::new(tr("No active stream bonus")).size(10.0).color(DIM));
    }
}

fn source_is_active(source:&str)->bool {
    !source.contains("+0%")
        && !source.contains("+0.0%")
        && !source.trim().is_empty()
}

pub(super) fn draw_stream_bonus_details(ui: &mut egui::Ui, health: &Health) {
    ui.add_space(12.0);
    ui.label(
        RichText::new(tr("STREAM BONUSES"))
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
                ui.label(RichText::new(tr("Stream bonus data unavailable until game data loads"))
                    .size(10.0).color(DIM));
                return;
            }

            if health.twitch_linked==Some(true) && !health.twitch_login.is_empty() {
                ui.label(RichText::new(format!("Twitch · {}",health.twitch_login))
                    .size(9.0).color(DIM));
            }
            let now=stream_clock_ms(health);
            draw_stream_bonus(ui,"Twitch",health.stream_bonuses.twitch,now);
            ui.add_space(6.0);
            draw_stream_bonus(ui,"Kick",health.stream_bonuses.kick,now);
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
            let mut sources=health.xp_sources.iter()
                .chain(health.xp_bonuses.iter())
                .filter(|source|source_is_active(source))
                .cloned().collect::<Vec<_>>();
            sources.sort();
            sources.dedup();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_percent_sources_are_hidden() {
        assert!(!source_is_active("Twitch +0%"));
        assert!(!source_is_active("Kick +0.0% XP"));
        assert!(source_is_active("Kick +15% XP"));
        assert!(source_is_active("Guild +10%"));
    }

    #[test]
    fn multi_hour_packages_keep_their_full_remaining_time() {
        let now=1_000;
        assert_eq!(remaining_label(now+3*60*60*1000,now),"3h 00m");
        assert_eq!(remaining_label(now+5*60*60*1000,now),"5h 00m");
        assert_eq!(remaining_label(now+10*60*60*1000,now),"10h 00m");
    }
}
