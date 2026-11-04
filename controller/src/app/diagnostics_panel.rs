//! Foreground-only performance dashboard. Reuses the controller's shared
//! Runtime chips and Kick HLS counters; no worker is spawned to render it.
use super::{egui, runtime_chip, tr, ControllerApp, RichText, ACCENT, BORDER,
    DIM, GOOD, MUTED, PANEL, PANEL_ALT, TEXT, WARN};
use crate::diagnostics::{
    DiagnosticsSampler, HlsAccountSample, HlsNetworkSample, PerformanceSample,
};

fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
}

fn cpu_label(cpu: Option<f64>) -> String {
    cpu.map(|percent| format!("{percent:.1}%")).unwrap_or_else(|| "—".into())
}

fn memory_label(bytes: Option<u64>) -> String {
    bytes.map(mib).unwrap_or_else(|| "—".into())
}

fn history_chart(
    ui: &mut egui::Ui,
    title: &str,
    samples: &[PerformanceSample],
    metric: fn(&PerformanceSample) -> Option<f64>,
    color: egui::Color32,
    unit: &str,
) {
    egui::Frame::new().fill(PANEL).stroke(egui::Stroke::new(1.0, BORDER))
        .corner_radius(8.0).inner_margin(12).show(ui, |ui| {
            ui.label(RichText::new(title).size(11.0).strong().color(TEXT));
            let values: Vec<f64> = samples.iter().filter_map(metric).collect();
            let peak = values.iter().copied().fold(0.0_f64, f64::max);
            let max_scale = peak.max(1.0);
            ui.label(RichText::new(format!(
                "{}: {peak:.1} {unit}", tr("Observed peak")
            )).size(10.0).color(MUTED));
            let width = ui.available_width().max(120.0);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(width, 76.0), egui::Sense::hover()
            );
            ui.painter().rect_filled(rect, 6.0, PANEL_ALT);
            if values.len() < 2 {
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                    tr("Collecting samples"), egui::FontId::proportional(11.0), DIM);
                return;
            }
            let points: Vec<egui::Pos2> = values.iter().enumerate().map(|(i, value)| {
                let x = rect.left() + 6.0 + (rect.width() - 12.0)
                    * i as f32 / (values.len() - 1) as f32;
                let y = rect.bottom() - 6.0 - (rect.height() - 12.0)
                    * (*value / max_scale) as f32;
                egui::pos2(x, y)
            }).collect();
            ui.painter().add(egui::Shape::line(points,
                egui::Stroke::new(1.8, color)));
        });
}

pub(super) fn draw_diagnostics_panel(
    app: &mut ControllerApp,
    ui: &mut egui::Ui,
    active: bool,
) {
    egui::ScrollArea::vertical()
        .id_salt("performance-diagnostics-scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.label(RichText::new(tr("Performance diagnostics"))
                .size(17.0).strong().color(TEXT));
            ui.label(RichText::new(tr(
                "Measurements run only while this page is visible and the window is focused. Closing it or switching pages stops sampling and discards its history."
            )).size(11.0).color(MUTED));
            ui.add_space(10.0);

            if !active {
                ui.label(RichText::new(tr(
                    "Diagnostics inactive: focus the window to start a new sample."
                )).size(11.0).color(WARN));
                return;
            }

            // Allocate the sampler only on the first active foreground frame.
            let (sample, history, last_frame_ms, slowest_frame_ms, recorded_frames, paused, refresh_network) = {
                let sampler = app.diagnostics.get_or_insert_with(DiagnosticsSampler::default);
                ui.horizontal(|ui| {
                    let mut paused = sampler.paused;
                    if ui.checkbox(&mut paused, tr("Pause sampling")).changed() {
                        sampler.set_paused(paused);
                    }
                    if ui.small_button(tr("Reset samples")).clicked() {
                        *sampler = DiagnosticsSampler::default();
                    }
                });
                let refresh_network = sampler.sample_if_due();
                (sampler.current, sampler.history.iter().copied().collect::<Vec<_>>(),
                    sampler.last_frame_ms, sampler.slowest_frame_ms,
                    sampler.recorded_frames, sampler.paused, refresh_network)
            };

            // Snapshot Kick accounting on exactly the same two-second tick.
            // Pausing the panel also freezes this cache; there is no per-frame
            // polling of every Kick channel.
            if refresh_network {
                let mut network = HlsNetworkSample::default();
                for (index, runtime) in app.runtimes.iter().enumerate() {
                    if let Some(kick) = runtime.kick.kick_monitor.as_ref() {
                        let current = kick.snapshot();
                        network.bytes = network.bytes.saturating_add(current.bandwidth.bytes);
                        network.last_minute = network.last_minute
                            .saturating_add(current.bandwidth.last_minute);
                        network.rows.push(HlsAccountSample {
                            account: app.accounts[index].name.clone(),
                            bytes: current.bandwidth.bytes,
                            last_minute: current.bandwidth.last_minute,
                            channels: current.channels.len(),
                        });
                    }
                }
                app.diagnostics.as_mut().expect("active diagnostics").network = Some(network);
            }
            let network = app.diagnostics.as_ref()
                .and_then(|sampler| sampler.network.as_ref())
                .cloned().unwrap_or_default();

            if paused {
                ui.label(RichText::new(tr("Sampling paused")).size(11.0).color(WARN));
            } else {
                ui.label(RichText::new(tr("One sample every 2 seconds · last 2 minutes · no background worker"))
                    .size(10.0).color(DIM));
            }
            ui.add_space(12.0);

            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(14).show(ui, |ui| {
                    ui.label(RichText::new(tr("Controller process"))
                        .size(13.0).strong().color(TEXT));
                    ui.add_space(7.0);
                    let cpu = sample.and_then(|s| s.cpu_percent);
                    let rss = sample.and_then(|s| s.resident_bytes);
                    let peak = sample.and_then(|s| s.peak_bytes);
                    let threads = sample.and_then(|s| s.threads);
                    ui.horizontal_wrapped(|ui| {
                        runtime_chip(ui, tr("CPU"), &cpu_label(cpu), ACCENT);
                        runtime_chip(ui, tr("RAM"), &memory_label(rss), GOOD);
                        runtime_chip(ui, tr("Peak RAM"), &memory_label(peak), MUTED);
                        runtime_chip(ui, tr("Threads"),
                            &threads.map(|n| n.to_string()).unwrap_or_else(|| "—".into()), MUTED);
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr(
                        "CPU is measured as a percentage of one logical core (multi-core usage can exceed 100%). RAM is resident physical memory. Firefox is a separate process."
                    )).size(10.0).color(DIM));
                });

            ui.add_space(12.0);
            ui.columns(2, |columns| {
                history_chart(&mut columns[0], tr("CPU history"), &history,
                    |s| s.cpu_percent, ACCENT, "%");
                history_chart(&mut columns[1], tr("RAM history"), &history,
                    |s| s.resident_bytes.map(|bytes| bytes as f64 / 1_048_576.0),
                    GOOD, "MiB");
            });
            ui.add_space(12.0);
            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(14).show(ui, |ui| {
                    ui.label(RichText::new(tr("UI work")).size(13.0).strong().color(TEXT));
                    ui.horizontal_wrapped(|ui| {
                        runtime_chip(ui, tr("Last UI pass"),
                            &last_frame_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), ACCENT);
                        runtime_chip(ui, tr("Slowest UI pass"),
                            &if recorded_frames > 0 {
                                format!("{slowest_frame_ms:.2} ms")
                            } else { "—".into() }, MUTED);
                        runtime_chip(ui, tr("Observed UI passes"),
                            &recorded_frames.to_string(), MUTED);
                    });
                    ui.label(RichText::new(tr(
                        "Measures controller UI update work while diagnostics is active, not game-worker execution or frames per second."
                    )).size(10.0).color(DIM));
                });

            ui.add_space(12.0);
            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(14).show(ui, |ui| {
                    ui.label(RichText::new(tr("Kick HLS network"))
                        .size(13.0).strong().color(TEXT));
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        runtime_chip(ui, tr("Session transferred"), &mib(network.bytes), ACCENT);
                        runtime_chip(ui, tr("Last 60 seconds"), &mib(network.last_minute), GOOD);
                        runtime_chip(ui, tr("Last-minute rate"),
                            &format!("{:.1} KiB/s", network.last_minute as f64 / 60.0 / 1024.0), MUTED);
                    });
                    ui.add_space(8.0);
                    for row in &network.rows {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&row.account).size(11.0).strong().color(TEXT));
                            ui.label(RichText::new(format!(
                                "{} {} · {} {} · {} {}",
                                mib(row.bytes), tr("total"),
                                mib(row.last_minute), tr("last minute"),
                                row.channels, tr("channels")
                            )).size(10.0).color(MUTED));
                        });
                    }
                    if network.rows.is_empty() {
                        ui.label(RichText::new(tr("No active Kick sessions"))
                            .size(10.0).color(DIM));
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr(
                        "Only measured Kick HLS payload is included. Twitch, game socket, HTTP overhead, and Firefox network traffic are not measured here."
                    )).size(10.0).color(DIM));
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatting_never_claims_missing_metrics() {
        assert_eq!(cpu_label(None), "—");
        assert_eq!(memory_label(None), "—");
        assert_eq!(cpu_label(Some(150.0)), "150.0%");
        assert_eq!(mib(1_048_576), "1.0 MiB");
    }
}
