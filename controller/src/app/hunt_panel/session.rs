use super::*;
pub(super) fn draw_atlas_session(
    ui: &mut egui::Ui, view: &HuntView, native: &NativeSession,
    index: usize, workspace: &mut WorkspaceState,
) {
                    let session=&view.session;
                    ui.label(RichText::new(tr("Battle results recorded by the game"))
                        .size(15.0).strong().color(TEXT));
                    ui.label(tr_dynamic(format!("Duration {:02}:{:02}:{:02}",
                        session.duration_secs/3600,
                        session.duration_secs%3600/60,
                        session.duration_secs%60)));
                    ui.add_space(8.0);
                    egui::Frame::new().fill(PANEL).corner_radius(8.0)
                        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
                            let stats=[
                                ("Kills",format_number(session.kills)),
                                ("Captures",format_number(session.captures)),
                                (tr("Trainer XP"),format_number(session.trainer_xp)),
                                (tr("Pokémon XP"),format_number(session.pokemon_xp)),
                                ("Combat gold",format_number(session.combat_gold)),
                                ("Auto-sale gold",format_number(session.auto_sale_gold)),
                            ];
                            let count=if ui.available_width()>=600.0{3}else{2};
                            ui.columns(count,|cols|{
                                for (i,(label,value)) in stats.iter().enumerate(){
                                    detail_stat_line(&mut cols[i%count],label,value);
                                }
                            });
                            ui.label(tr_dynamic(format!("Shiny encounter events: {} · shiny captures: {}",
                                session.shiny_seen,session.shiny_captured)));
                            let throws=session.balls_used.values().sum::<u64>();
                            ui.label(tr_dynamic(format!("Identified ball throws: {} · confirmed successes: {}",
                                throws,session.ball_captures)));
                            if throws>0{
                                ui.label(tr_dynamic(format!("Observed success per ball: {:.1}% ({} throws)",
                                    session.ball_captures as f64*100.0/throws as f64,throws)))
                                    .on_hover_text(tr("Observed ball outcomes in this session; not a species capture probability."));
                            }else {
                                ui.label(RichText::new(tr("Capture success rate will appear after your first recorded throw."))
                                    .color(MUTED));
                            }
                        });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui|{
                        if !workspace.session_reset_confirm[index] {
                            if ui.small_button(tr("Reset session counters")).clicked() {
                                workspace.session_reset_confirm[index]=true;
                            }
                        } else {
                            ui.label(RichText::new(tr("Clear session-only totals?")).color(WARN));
                            if ui.small_button(tr("Confirm")).clicked(){
                                native.reset_hunt_session();
                                workspace.session_reset_confirm[index]=false;
                            }
                            if ui.small_button(tr("Cancel")).clicked(){
                                workspace.session_reset_confirm[index]=false;
                            }
                        }
                        if ui.small_button(tr("Open persistent history")).clicked(){
                            workspace.selected_details=Some(index);
                            workspace.dashboard_show_history=true;
                            workspace.history_rows_limit=100;
                            workspace.panel_tab=PanelTab::Overview;
                        }
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr("Balls used")).size(13.0).strong().color(TEXT));
                    if session.balls_used.is_empty(){ui.label(tr("No ball throws recorded yet."));}
                    for (id,count) in &session.balls_used {
                        let name=view.ball_names.get(id).map(String::as_str).unwrap_or(tr("Unknown ball"));
                        ui.label(format!("{name} (#{id}) · {count}"));
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Items acquired from battles"))
                            .size(13.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "{} unique drops",session.acquired.len()
                        )).size(10.0).color(MUTED));
                    });
                    let mut drops=session.acquired.iter().collect::<Vec<_>>();
                    drops.sort_by(|a,b|b.1.cmp(a.1).then_with(||a.0.cmp(b.0)));
                    if drops.is_empty(){
                        ui.label(RichText::new(tr("No confirmed battle drops yet."))
                            .size(11.0).color(MUTED));
                    }else{
                        for (name,qty) in drops.iter().take(8){
                            ui.horizontal_wrapped(|ui|{
                                ui.label(RichText::new(*name)
                                    .size(11.0).color(TEXT));
                                ui.label(RichText::new(format!("×{qty}"))
                                    .size(11.0).strong().color(ACCENT));
                            });
                        }
                        if drops.len()>8{
                            ui.collapsing(format!(
                                "All acquired items ({} unique)",drops.len()
                            ),|ui|{
                                for (name,qty) in drops.iter().take(200){
                                    ui.label(format!("{name} ×{qty}"));
                                }
                                if drops.len()>200{
                                    ui.label(tr("Showing the first 200 items."));
                                }
                            });
                        }
                    }
                    if !session.recent_drops.is_empty(){
                        ui.collapsing(tr("Latest drop events"),|ui|{
                            for (name,count) in &session.recent_drops{
                                ui.label(format!("{name} ×{count}"));
                            }
                        });
                    }
                    ui.separator();
                    ui.collapsing(tr("Persisted history for current hunt"),|ui|{
                        let history=native.history_snapshot();
                        if let Some(error)=history.load_error.as_deref()
                            .or(history.save_error.as_deref()) {
                            ui.label(RichText::new(error).color(WARN));
                        }
                        if let Some(slug)=view.hunt_slug.as_deref(){
                            let records:Vec<_>=history.days.iter()
                                .filter(|row|row.hunt==slug).collect();
                            if records.is_empty(){
                                ui.label(tr("No historical observations saved for this hunt."));
                            }else{
                                let sum=|getter:fn(&crate::native_history::HistoryDay)->u64|
                                    records.iter().fold(0u64,|total,row|
                                        total.saturating_add(getter(row)));
                                ui.label(format!(
                                    "{} {} · {} {} · {} {} · {} {}",
                                    records.len(),tr("retained days"),sum(|d|d.kills),tr("kills"),
                                    sum(|d|d.captures),tr("captures"),sum(|d|d.shiny_seen),tr("shiny events")
                                ));
                                ui.label(format!(
                                    "{} {} · {} {} · {} {}",
                                    tr("Trainer XP"),format_hunt_amount(sum(|d|d.trainer_xp) as f64),
                                    tr("Pokémon XP"),format_hunt_amount(sum(|d|d.pokemon_xp) as f64),
                                    tr("Combat gold"),format_hunt_amount(sum(|d|d.combat_gold) as f64),
                                ));
                                ui.label(RichText::new(
                                    tr("These are persistent totals, not historical XP/hour; per-hunt elapsed time is not currently retained.")
                                ).size(10.0).color(MUTED));
                            }
                        }else{
                            ui.label(tr("No current hunt ID is available for historical grouping."));
                        }
                    });
}
