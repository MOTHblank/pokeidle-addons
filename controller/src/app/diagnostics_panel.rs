//! Foreground-only performance dashboard. Reuses the controller's shared
//! Runtime chips and Kick HLS counters; no worker is spawned to render it.
use super::{egui, runtime_chip, tr, ControllerApp, RichText, ACCENT, BORDER,
    DIM, GOOD, MUTED, PANEL, PANEL_ALT, TEXT, WARN};
use crate::diagnostics::{
    DiagnosticsSampler, GameQueueSample, HlsAccountSample, HlsNetworkSample,
    OperationalSample, PerformanceSample,
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

/// Resident-memory movement relative to when this diagnostics session opened,
/// not the process's lifetime high-water mark.
fn memory_change_label(current: Option<u64>, baseline: Option<u64>) -> String {
    current.zip(baseline)
        .map(|(value, start)| format!(
            "{:+.1} MiB", (value as i128 - start as i128) as f64 / 1_048_576.0
        ))
        .unwrap_or_else(|| "—".into())
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
        .corner_radius(8.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.label(RichText::new(title).size(11.0).strong().color(TEXT));
            let peak = samples.iter().filter_map(metric).fold(None::<f64>, |best, value| {
                if !value.is_finite() { return best; }
                Some(best.unwrap_or(value).max(value))
            });
            ui.label(RichText::new(match peak {
                Some(value) => format!("{}: {value:.1} {unit}", tr("Observed peak")),
                None => format!("{}: —", tr("Observed peak")),
            }).size(10.0).color(MUTED));
            let width = ui.available_width().max(120.0);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(width, 86.0), egui::Sense::hover()
            );
            ui.painter().rect_filled(rect, 6.0, PANEL_ALT);
            let first = samples.iter().find_map(|point| point.at);
            let last = samples.iter().rev().find_map(|point| point.at);
            let (Some(first), Some(last), Some(peak)) = (first, last, peak) else {
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                    tr("Collecting samples"), egui::FontId::proportional(11.0), DIM);
                return;
            };
            let span = last.saturating_duration_since(first).as_secs_f32().max(0.001);
            let scale = peak.max(1.0);
            let x_for = |at: std::time::Instant| -> f32 {
                rect.left() + 6.0 + (rect.width() - 12.0)
                    * (at.saturating_duration_since(first).as_secs_f32() / span).clamp(0.0, 1.0)
            };
            let mut segment: Vec<egui::Pos2> = Vec::new();
            let flush = |segment: &mut Vec<egui::Pos2>| {
                if segment.len() >= 2 {
                    ui.painter().add(egui::Shape::line(
                        std::mem::take(segment), egui::Stroke::new(1.8, color)
                    ));
                } else if let Some(point) = segment.pop() {
                    ui.painter().circle_filled(point, 2.5, color);
                }
            };
            for point in samples {
                let value = metric(point).filter(|v| v.is_finite());
                if let (Some(at), Some(value)) = (point.at, value) {
                    segment.push(egui::pos2(
                        x_for(at),
                        rect.bottom() - 6.0 - (rect.height() - 12.0)
                            * (value.max(0.0) / scale) as f32
                    ));
                } else {
                    // A missing value, failed read, or Resume gap must not
                    // be connected to its neighbors by an invented line.
                    flush(&mut segment);
                }
            }
            flush(&mut segment);
            if let Some(pointer) = response.hover_pos() {
                let nearest = samples.iter().filter_map(|point|
                    point.at.map(|at| (at, metric(point)))
                ).min_by(|a, b| {
                    (x_for(a.0) - pointer.x).abs()
                        .total_cmp(&(x_for(b.0) - pointer.x).abs())
                });
                if let Some((at, value)) = nearest {
                    let label = value.filter(|v| v.is_finite())
                        .map(|v| format!("{v:.2} {unit}"))
                        .unwrap_or_else(|| tr("Unavailable").into());
                    response.on_hover_text_at_pointer(format!(
                        "{title}: {label} · {:.0} s {}", last.saturating_duration_since(at).as_secs_f64(),
                        tr("before latest sample")
                    ));
                }
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("-{:.0}s", span)).size(9.0).color(DIM));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(tr("Latest sample")).size(9.0).color(DIM));
                });
            });
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
            let mut export_requested = false;
            let (sample, history, last_frame_ms, slowest_frame_ms,
                recorded_frames, mean_frame_ms, p95_frame_ms,
                preflight_ms, panel_ms, process_probe_ms, network_probe_ms,
                paused, refresh_network) = {
                let sampler = app.diagnostics.get_or_insert_with(DiagnosticsSampler::default);
                ui.horizontal(|ui| {
                    let mut paused = sampler.paused;
                    if ui.checkbox(&mut paused, tr("Pause sampling")).changed() {
                        sampler.set_paused(paused);
                    }
                    let mut inspect_threads = sampler.inspect_threads;
                    if ui.checkbox(&mut inspect_threads, tr("Inspect thread CPU")).changed() {
                        sampler.set_thread_inspection(inspect_threads);
                    }
                    if ui.small_button(tr("Reset samples")).clicked() {
                        sampler.reset();
                    }
                    if ui.small_button(tr("Export report")).clicked() {
                        export_requested = true;
                    }
                });
                let refresh_network = sampler.sample_if_due();
                (sampler.current, sampler.history.iter().copied().collect::<Vec<_>>(),
                    sampler.last_frame_ms, sampler.slowest_frame_ms,
                    sampler.recorded_frames, sampler.mean_frame_ms(), sampler.p95_frame_ms(),
                    sampler.last_preflight_ms, sampler.last_panel_ms,
                    sampler.last_process_probe_ms, sampler.last_network_probe_ms,
                    sampler.paused, refresh_network)
            };

            // Snapshot Kick accounting on exactly the same two-second tick.
            // Pausing the panel also freezes this cache; there is no per-frame
            // polling of every Kick channel.
            if refresh_network {
                let probe_start = std::time::Instant::now();
                let mut network = HlsNetworkSample::default();
                let mut operational = OperationalSample::default();
                for (index, runtime) in app.runtimes.iter().enumerate() {
                    if let Some(game) = runtime.game.native.as_ref() {
                        if let Some(snapshot) = game.diagnostics_snapshot() {
                            operational.game.push(GameQueueSample {
                                slot: index,
                                phase: snapshot.phase,
                                capture_queued: snapshot.capture_queued,
                                automation_queued: snapshot.automation_queued,
                                automation_pending: snapshot.automation_pending,
                                market_pending: snapshot.market_pending,
                                restock_pending: snapshot.restock_pending,
                            });
                        }
                    }
                    if let Some(kick) = runtime.kick.kick_monitor.as_ref() {
                        if let Some(current) = kick.diagnostics_snapshot() {
                            network.bytes = network.bytes.saturating_add(current.bandwidth.bytes);
                            network.last_minute = network.last_minute
                                .saturating_add(current.bandwidth.last_minute);
                            network.rows.push(HlsAccountSample {
                                account: app.accounts[index].name.clone(),
                                bytes: current.bandwidth.bytes,
                                last_minute: current.bandwidth.last_minute,
                                channels: current.channels,
                            });
                        }
                    }
                }
                let (channels, hls) = crate::native_kick_pool::diagnostics_usage();
                operational.kick_channels = channels;
                operational.kick_hls = hls;
                let sampler = app.diagnostics.as_mut().expect("active diagnostics");
                sampler.network = Some(network);
                sampler.operational = Some(operational);
                sampler.last_network_probe_ms =
                    Some(probe_start.elapsed().as_secs_f64() * 1000.0);
            }
            let (memory_baseline, detailed_threads, hot_threads, threads_unavailable) = {
                let sampler = app.diagnostics.as_ref().expect("active diagnostics");
                (sampler.resident_baseline, sampler.inspect_threads,
                    sampler.hot_threads.clone(), sampler.threads_unavailable)
            };
            let network = app.diagnostics.as_ref()
                .and_then(|sampler| sampler.network.as_ref())
                .cloned().unwrap_or_default();
            let operational = app.diagnostics.as_ref()
                .and_then(|sampler| sampler.operational.as_ref())
                .cloned().unwrap_or_default();

            if export_requested {
                // The report contains bounded numerical metrics and profile
                // indices only, never credentials, stream URLs or account names.
                let result = app.diagnostics.as_ref()
                    .expect("active diagnostics").export_report();
                match result {
                    Ok(path) => app.set_status(format!(
                        "{}: {}", tr("Diagnostics report saved"), path.display()
                    ), false),
                    Err(error) => app.set_status(error, true),
                }
            }

            if paused {
                ui.label(RichText::new(tr("Sampling paused")).size(11.0).color(WARN));
            } else {
                ui.label(RichText::new(tr("Every 2 seconds · up to 60 timestamped samples · no background worker"))
                    .size(10.0).color(DIM));
            }
            ui.add_space(12.0);

            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
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
                        runtime_chip(ui, tr("RAM since opening"),
                            &memory_change_label(rss, memory_baseline), MUTED);
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
            if detailed_threads {
                ui.add_space(12.0);
                egui::Frame::new().fill(PANEL).corner_radius(10.0)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                        ui.label(RichText::new(tr("Top CPU threads"))
                            .size(13.0).strong().color(TEXT));
                        ui.label(RichText::new(tr(
                            "Optional OS thread inspection every 15 seconds. CPU is percent of one core; new threads need two readings. Names may be generic, especially on Windows."
                        )).size(10.0).color(MUTED));
                        ui.add_space(7.0);
                        if threads_unavailable {
                            ui.label(RichText::new(tr("Thread inspection unavailable"))
                                .size(10.0).color(WARN));
                        } else if hot_threads.is_empty() {
                            ui.label(RichText::new(tr("Collecting thread CPU baseline"))
                                .size(10.0).color(DIM));
                        } else {
                            for thread in &hot_threads {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(&thread.name).size(11.0).strong().color(TEXT))
                                        .on_hover_text(format!("TID {}", thread.id));
                                    ui.label(RichText::new(format!(
                                        "#{} · {:.1}%", thread.id, thread.cpu_percent
                                    )).size(10.0).color(ACCENT));
                                });
                                let fraction = (thread.cpu_percent / 100.0).clamp(0.0, 1.0) as f32;
                                ui.add(egui::ProgressBar::new(fraction)
                                    .desired_width(ui.available_width()));
                            }
                        }
                    });
            }

            ui.add_space(12.0);
            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
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
                        runtime_chip(ui, tr("Average UI pass"),
                            &mean_frame_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                        runtime_chip(ui, tr("UI P95"),
                            &p95_frame_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                    });
                    ui.add_space(7.0);
                    ui.label(RichText::new(tr("Observed UI stages"))
                        .size(11.0).strong().color(TEXT));
                    ui.horizontal_wrapped(|ui| {
                        runtime_chip(ui, tr("UI preparation"),
                            &preflight_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                        runtime_chip(ui, tr("Main panel"),
                            &panel_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                        runtime_chip(ui, tr("Process probe"),
                            &process_probe_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                        runtime_chip(ui, tr("Diagnostics refresh"),
                            &network_probe_ms.map(|ms| format!("{ms:.2} ms"))
                                .unwrap_or_else(|| "—".into()), MUTED);
                    });
                    ui.label(RichText::new(tr(
                        "UI pass includes preparation, rendering and housekeeping; it is not GPU frame time or game-worker CPU. Probe timings expose diagnostics overhead."
                    )).size(10.0).color(DIM));
                });

            ui.add_space(12.0);
            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
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

            ui.add_space(12.0);
            egui::Frame::new().fill(PANEL).corner_radius(10.0)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin::same(14)).show(ui, |ui| {
                    ui.label(RichText::new(tr("Native worker queues"))
                        .size(13.0).strong().color(TEXT));
                    ui.label(RichText::new(tr(
                        "Existing queue snapshots, not per-feature CPU usage. Collected only on diagnostics ticks."
                    )).size(10.0).color(DIM));
                    ui.add_space(8.0);
                    for game in &operational.game {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!(
                                "{} {} · {}", tr("Profile"), game.slot + 1, game.phase
                            )).size(11.0).strong().color(TEXT));
                            ui.label(RichText::new(format!(
                                "{}: {} · {}: {} · {}: {} · {}: {} · {}: {}",
                                tr("Capture queue"), game.capture_queued,
                                tr("Automation queue"), game.automation_queued,
                                tr("Automation pending"), if game.automation_pending { tr("Yes") } else { tr("No") },
                                tr("Market pending"), if game.market_pending { tr("Yes") } else { tr("No") },
                                tr("Restock pending"), if game.restock_pending { tr("Yes") } else { tr("No") },
                            )).size(10.0).color(MUTED));
                        });
                    }
                    if operational.game.is_empty() {
                        ui.label(RichText::new(tr("No native game sessions")).size(10.0).color(DIM));
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new(tr("Kick worker pools"))
                        .size(11.0).strong().color(TEXT));
                    for (name, pool) in [
                        (tr("Channel workers"), operational.kick_channels),
                        (tr("HLS workers"), operational.kick_hls),
                    ] {
                        if let Some(pool) = pool {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(name).size(10.0).strong().color(TEXT));
                                ui.label(RichText::new(format!(
                                    "{}: {} · {}: {} · {}: {}",
                                    tr("Workers"), pool.workers,
                                    tr("Idle"), pool.idle,
                                    tr("Queued"), pool.pending
                                )).size(10.0).color(MUTED));
                            });
                        }
                    }
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
        assert_eq!(memory_change_label(Some(4_194_304), Some(2_097_152)), "+2.0 MiB");
        assert_eq!(memory_change_label(Some(2_097_152), Some(4_194_304)), "-2.0 MiB");
        assert_eq!(memory_change_label(None, Some(2_097_152)), "—");
    }
}
