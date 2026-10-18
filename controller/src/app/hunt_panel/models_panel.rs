//! Compact scenario lab: conditional, never presented as real server odds.
use super::*;
use crate::native_history::HistorySnapshot;

fn value(n:Option<f64>)->String {
    n.filter(|n|n.is_finite()).map(format_hunt_amount)
        .unwrap_or_else(||"—".into())
}
fn selected_hunt<'a>(view:&'a HuntView,guide:Option<&'a AtlasReference>,
    scenario:&AtlasScenario)->Option<(&'a native_hunts::HuntEntry,bool)>{
    let hunts=atlas_destinations(view,guide);
    let requested=if scenario.hunt_slug.is_empty(){
        view.hunt_slug.as_deref()
    }else{Some(scenario.hunt_slug.as_str())};
    hunts.iter().find(|(h,_)|Some(h.slug.as_str())==requested)
        .copied().or_else(||hunts.into_iter().next())
}
pub(super) fn draw_retrospective(
    ui:&mut egui::Ui,history:&HistorySnapshot,slug:Option<&str>,
    forecast:Option<&native_atlas::Forecast>
){
    ui.label(RichText::new(tr("Historical observation vs current forecast"))
        .size(13.0).strong().color(TEXT));
    let Some(slug)=slug else{
        ui.label(tr("No selected hunt; history cannot be grouped."));return
    };
    let rows:Vec<_>=history.days.iter().filter(|d|d.hunt==slug).collect();
    if rows.is_empty(){
        ui.label(tr("No retained observations for this hunt."));return
    }
    // Older history has no model checkpoints. Compare current models and
    // archived snapshots only when the actual fields really exist.
    let latest_day=rows.iter().map(|d|d.day).max().unwrap_or(0);
    ui.label(RichText::new(tr("The summary uses the CURRENT model. Archived checkpoints below preserve point-in-time predictions when Hunt Atlas was open. Old history without observed durations remains unavailable."))
        .size(10.0).color(WARN));
    egui::ScrollArea::horizontal().id_salt(("atlas-history-x",slug))
        .show(ui,|ui|{
            egui::Grid::new(("atlas-history",slug)).striped(true)
                .spacing([15.0,6.0]).show(ui,|ui|{
            ui.strong(tr("Retained window"));ui.strong(tr("Observed trainer/h"));
            ui.strong(tr("Observed Pokémon/h"));ui.strong(tr("Current model / observed"));ui.end_row();
            for (days,title) in [(7,"7 days"),(30,"30 days"),(90,"90 days")]{
                let matching=rows.iter().filter(|d|
                    d.day<=latest_day&&latest_day-d.day<days);
                let (seconds,trainer,pokemon,kills,tr_receipts,pk_receipts)=
                    matching.fold((0u64,0u64,0u64,0u64,0u64,0u64),
                    |(s,t,p,k,tr,pk),row|(
                        s.saturating_add(row.observed_seconds),
                        t.saturating_add(row.timed_trainer_xp),
                        p.saturating_add(row.timed_pokemon_xp),
                        k.saturating_add(row.timed_kills),
                        tr.saturating_add(row.timed_trainer_receipts),
                        pk.saturating_add(row.timed_pokemon_receipts)));
                let train=if kills>0&&tr_receipts==kills {
                    models::retrospective_rate(trainer,seconds)
                }else{None};
                let poke=if kills>0&&pk_receipts==kills{
                    models::retrospective_rate(pokemon,seconds)
                }else{None};
                ui.label(title);
                ui.label(format!("{} / h",value(train)));
                ui.label(format!("{} / h",value(poke)));
                let residual=train.filter(|n|*n>0.0)
                    .and_then(|n|forecast.map(|f|f.trainer_xp_h/n));
                ui.label(residual.map(|n|format!("{n:.2}×"))
                    .unwrap_or_else(||"—".into()));
                ui.end_row();
            }
        });
        });
    let mut archived=history.forecasts.iter().filter(|entry|entry.hunt==slug)
        .collect::<Vec<_>>();
    archived.sort_by_key(|record|std::cmp::Reverse(record.hour));
    ui.collapsing(tr_dynamic(format!("Archived prediction checkpoints ({})",archived.len())),|ui|{
        if archived.is_empty(){
            ui.label(tr("No point-in-time forecasts were captured yet. Open Hunt Atlas during an authenticated session to collect checkpoints."));
        }else{
            ui.label(RichText::new(tr("Checkpoints are saved when this panel is open, no more than once per 15 minutes. Same-day observations may cover different hours and are not exact paired combat sessions."))
                .size(10.0).color(MUTED));
            egui::ScrollArea::vertical().id_salt(("atlas-archived",slug))
                .max_height(240.0).show(ui,|ui|{
                    egui::Grid::new(("atlas-archived-grid",slug))
                        .striped(true).spacing([12.0,5.0]).show(ui,|ui|{
                            ui.strong(tr("Day · hour"));ui.strong(tr("Archived trainer / Pokémon XP/h"));
                            ui.strong(tr("Same-day trainer/h"));ui.strong(tr("Forecast / measured"));
                            ui.end_row();
                            for sample in archived.iter().take(30){
                                let observed=rows.iter().find(|row|row.day==sample.day)
                                    .and_then(|row|if row.timed_kills>0
                                        && row.timed_trainer_receipts==row.timed_kills{
                                            models::retrospective_rate(
                                                row.timed_trainer_xp,row.observed_seconds)
                                        }else{None});
                                let residual=observed.filter(|n|*n>0.0)
                                    .map(|n|sample.trainer_xp_h/n);
                                ui.label(format!("{} · {:02}:00 UTC",sample.day,
                                    sample.hour%24));
                                ui.label(format!("{} / {} · {:.0} {}",
                                    value(Some(sample.trainer_xp_h)),
                                    value(Some(sample.pokemon_xp_h)),sample.kills_h,tr("kills/h")));
                                ui.label(value(observed));
                                ui.label(residual.map(|n|format!("{n:.2}×"))
                                    .unwrap_or_else(||"—".into()));
                                ui.end_row();
                            }
                        });
                });
        }
    });
    let mut latest=rows.iter().filter(|r|r.observed_seconds>=60).collect::<Vec<_>>();
    latest.sort_by_key(|row|std::cmp::Reverse(row.day));
    ui.collapsing(tr_dynamic(format!("Daily time-attributed measurements ({})",latest.len())),|ui|{
        egui::ScrollArea::vertical().id_salt(("atlas-daily",slug))
            .max_height(215.0).show(ui,|ui|{
                for d in latest.iter().take(90){
                    ui.horizontal_wrapped(|ui|{
                        ui.label(tr_dynamic(format!("Day {} · {} kills · {}s observed",
                            d.day,d.timed_kills,d.observed_seconds)));
                        ui.label(tr_dynamic(format!("Trainer {} / h",
                            value(if d.timed_kills>0 &&
                                d.timed_trainer_receipts==d.timed_kills{
                                models::retrospective_rate(
                                    d.timed_trainer_xp,d.observed_seconds)
                            }else{None}))));
                        ui.label(tr_dynamic(format!("Pokémon {} / h",
                            value(if d.timed_kills>0 &&
                                d.timed_pokemon_receipts==d.timed_kills{
                                models::retrospective_rate(
                                    d.timed_pokemon_xp,d.observed_seconds)
                            }else{None}))));
                    });
                }
            });
    });
}

pub(super) fn draw_atlas_models(
    ui:&mut egui::Ui, view:&HuntView,reference:Option<&AtlasReference>,
    cache:&EstimateCache,scenario:&mut AtlasScenario,history:&HistorySnapshot,live:bool
){
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(tr("Scenario lab"))
            .size(16.0).strong().color(TEXT));
        ui.label(RichText::new(tr("What-if reference · not observed server outcomes"))
            .size(10.0).color(WARN));
    });
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .stroke(Stroke::new(1.0,BORDER))
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.label(RichText::new(tr("SCENARIO INPUTS")).size(10.0)
                .strong().color(DIM));
    ui.horizontal_wrapped(|ui|{
        ui.label(tr("Destination"));
        let current=selected_hunt(view,reference,scenario)
            .map(|(hunt,unlocked)|format!("{} · {}",
                hunt.name,if unlocked{tr("Unlocked")}else{tr("Guide-only")}))
            .unwrap_or_else(||tr("No hunts loaded").into());
        egui::ComboBox::from_id_salt("atlas-model-destination")
            .width(245.0).selected_text(current).show_ui(ui,|ui|{
                let hunts=atlas_destinations(view,reference);
                for (hunt,unlocked) in hunts.iter().take(1500){
                    ui.selectable_value(&mut scenario.hunt_slug,hunt.slug.clone(),
                        format!("{} · Lv {} · {}",hunt.name,hunt.level,
                            if *unlocked{tr("unlocked")}else{tr("guide-only")}));
                }
            });
        ui.label(tr("Ball"));
        for ball in [BallKind::Ultra,BallKind::Beast]{
            ui.selectable_value(&mut scenario.ball,ball,ball.label());
        }
    });
        });
    ui.add_space(8.0);
    let Some((hunt,unlocked))=selected_hunt(view,reference,scenario) else{
        ui.label(tr("No available destination or public-guide preview."));return
    };
    if !unlocked {
        ui.label(RichText::new(tr("This location is listed in the guide but is not available for this account. You can compare estimates, but cannot switch to it here."))
            .size(10.0).color(WARN));
    }
    let observed_tier=view.outland_tier;
    let outland=hunt.area.eq_ignore_ascii_case("outland");
    if outland {
        ui.horizontal_wrapped(|ui|{
            ui.label(tr("Model preview tier"));
            ui.add(egui::Slider::new(&mut scenario.tier,1..=8).show_value(true));
            ui.label(tr_dynamic(format!("Requires trainer Lv {}",
                models::TIER_LEVELS[(scenario.tier-1) as usize])));
            if let Some(actual)=observed_tier{
                ui.label(RichText::new(tr_dynamic(format!("Game tier: Outland {actual}")))
                    .size(10.0).color(GOOD));
                if scenario.tier!=actual && ui.small_button(tr("Preview active tier")).clicked(){
                    scenario.tier=actual;
                }
            }else{
                ui.label(RichText::new(tr("Game tier unavailable"))
                    .size(10.0).color(MUTED));
            }
            ui.label(RichText::new(
                tr("Preview only. To change the game tier, use the Outland selector above.")
            ).size(10.0).color(DIM));
        });
    }
    ui.horizontal_wrapped(|ui|{
        let mut override_active=scenario.accepted_fraction_override.is_some();
        if ui.checkbox(&mut override_active,tr("Override target acceptance")).changed(){
            scenario.accepted_fraction_override=if override_active{Some(1.0)}else{None};
        }
        if let Some(fraction)=scenario.accepted_fraction_override.as_mut(){
            ui.add(egui::Slider::new(fraction,0.0..=1.0).text(tr("Attempt share")));
        }else{
            ui.label(RichText::new(tr("Assumes all eligible corpses are targeted: optimistic ceiling"))
                .size(10.0).color(DIM));
        }
    });
    ui.horizontal_wrapped(|ui|{
        if ui.small_button(if scenario.modifiers_open{tr("▾ Hide modifiers")}else{tr("▸ Hypothetical modifiers")})
            .clicked(){scenario.modifiers_open=!scenario.modifiers_open}
        ui.label(RichText::new(tr("No boost is treated as active automatically."))
            .size(10.0).color(DIM));
    });
    if scenario.modifiers_open {
        egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
            .inner_margin(egui::Margin::same(9)).show(ui,|ui|{
                let m=&mut scenario.modifiers;
                ui.horizontal_wrapped(|ui|{
                    ui.checkbox(&mut m.capture_boost,tr("Capture boost ×1.5"));
                    ui.checkbox(&mut m.shiny_boost,tr("Shiny boost ×1.5"));
                    ui.checkbox(&mut m.kick_capture,tr("Kick catch ×1.15"));
                    ui.checkbox(&mut m.kick_shiny,tr("Kick shiny ×1.15"));
                    ui.checkbox(&mut m.lure,tr("Shiny lure (1/12k)"));
                });
                ui.horizontal_wrapped(|ui|{
                    ui.label(tr("Catch event %"));
                    ui.add(egui::DragValue::new(&mut m.catch_event_pct).range(0.0..=1000.0));
                    ui.label(tr("Shiny event %"));
                    ui.add(egui::DragValue::new(&mut m.shiny_event_pct).range(0.0..=1000.0));
                    for (label,n) in [
                        (tr("Loot ×"),&mut m.loot),(tr("Guild ×"),&mut m.guild),
                        (tr("Drop event ×"),&mut m.drop_event),(tr("Kick drop ×"),&mut m.kick_drop)]{
                        ui.label(label);ui.add(egui::DragValue::new(n)
                            .range(1.0..=1000.0).speed(0.05));
                    }
                });
                if ui.small_button(tr("Restore neutral modifiers")).clicked(){
                    m.clone_from(&Modifiers::default());
                }
            });
    }
    ui.add_space(9.0);
    let f=if live{cache.get(&hunt.slug)}else{None};
    let kills=f.as_ref().map(|f|f.kills_h);
    egui::Frame::new().fill(PANEL_ALT).corner_radius(9.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label(RichText::new(tr("Combat baseline"))
                    .size(13.0).strong().color(TEXT));
                if let Some(f)=f.as_ref(){
                    ui.colored_label(WARN,if f.calibrated{
                        tr("Calibrated estimate")
                    }else{tr("Experimental model")});
                }
            });
            ui.horizontal_wrapped(|ui|{
                for (label,number) in [
                    (tr("Kills / h"),kills),
                    (tr("Trainer XP / h"),f.as_ref().map(|f|f.trainer_xp_h)),
                    (tr("Pokémon XP / h"),f.as_ref().map(|f|f.pokemon_xp_h)),
                ]{
                    ui.vertical(|ui|{
                        ui.label(RichText::new(label).size(10.0).color(DIM));
                        ui.label(RichText::new(value(number))
                            .size(15.0).strong().color(TEXT));
                    });
                    ui.add_space(14.0);
                }
            });
        });
    let ball_quote=view.ball_quotes.get(&scenario.ball.id());
    let efficiency=ball_quote.and_then(|q|q.efficiency)
        .unwrap_or_else(||scenario.ball.wiki_efficiency());
    let known_efficiency=ball_quote.and_then(|q|q.efficiency).is_some();
    ui.label(RichText::new(format!("{} · {} {:.2} ({}) · {}",
        scenario.ball.label(),tr("ball efficiency"),efficiency,
        if known_efficiency{tr("server catalog")}else{tr("wiki default")},
        tr("target HP assumed 0% for wiki scenario")) )
        .size(10.0).color(MUTED));
    let all_weight=hunt.species.iter().map(|s|s.weight.max(0.0)).sum::<f64>();
    let mut rows=Vec::new();
    for spawn in &hunt.species{
        if spawn.weight<=0.0||all_weight<=0.0 {continue}
        let share=spawn.weight/all_weight;
        let name=reference.and_then(|r|r.species.get(&spawn.id))
            .map(|m|m.name.as_str()).unwrap_or("Unknown");
        let rarity=reference.and_then(|r|r.capture_rarities.get(&spawn.id)).copied();
        let stage=reference.and_then(|r|r.capture_stages.get(&spawn.id)).copied();
        let plan=kills.and_then(|speed|models::capture_scenario_with_stage(
            stage,rarity,efficiency,speed,share,
            scenario.accepted_fraction_override,outland,&scenario.modifiers));
        rows.push((spawn.id,name.to_owned(),share,rarity,plan));
    }
    rows.sort_by(|a,b|b.2.total_cmp(&a.2));
    ui.label(RichText::new(tr_dynamic(format!("Capture forecasts · {} species · {} with rarity input",
        rows.len(),rows.iter().filter(|r|r.3.is_some()).count())))
        .size(12.0).strong().color(TEXT));
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .inner_margin(egui::Margin::same(11)).show(ui,|ui|{
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(tr("MOST COMMON ENCOUNTERS"))
                .size(10.0).strong().color(DIM));
            for (_,name,_share,_,plan) in rows.iter().take(5){
                ui.horizontal_wrapped(|ui|{
                    ui.label(RichText::new(name).strong().size(11.0).color(TEXT));
                    ui.label(RichText::new(format!(
                        "Catch {} · Caught/h {}",
                        plan.as_ref().map(|p|format!("{:.3}%",p.chance*100.0))
                            .unwrap_or_else(||"—".into()),
                        value(plan.as_ref().map(|p|p.catches_h))
                    )).size(10.0).color(TEXT));
                });
            }
            if rows.is_empty(){
                ui.label(RichText::new(tr("No eligible species in this destination."))
                    .size(10.0).color(MUTED));
            }
        });
    egui::CollapsingHeader::new(format!(
        "Full capture breakdown · up to {} species",
        rows.len().min(32)
    )).default_open(false).show(ui,|ui|{
    egui::Frame::new().fill(PANEL).corner_radius(8.0)
        .inner_margin(egui::Margin::same(8)).show(ui,|ui|{
            if ui.available_width()>=860.0{
                egui::Grid::new(("atlas-capture-models",&hunt.slug,scenario.ball.id()))
                    .striped(true).spacing([12.0,6.0]).show(ui,|ui|{
                        ui.strong("Pokémon");ui.strong(tr("Catch / throw"));
                        ui.strong(tr("Caught/h"));ui.strong(tr("Shiny/h"));ui.strong(tr("Shiny ≥1/24h"));ui.end_row();
                        for (_,name,_share,_,plan) in rows.iter().take(32){
                            ui.label(name);
                            ui.label(plan.as_ref().map(|p|format!("{:.3}%",p.chance*100.0))
                                .unwrap_or_else(||"—".into()));
                            ui.label(value(plan.as_ref().map(|p|p.catches_h)));
                            ui.label(value(plan.as_ref().and_then(|p|p.shiny_caught_h)));
                            ui.label(plan.as_ref().and_then(|p|p.shiny_caught_24h)
                                .map(|p|format!("{:.2}%",p*100.0))
                                .unwrap_or_else(||"—".into()));
                            ui.end_row();
                        }
                    });
            }else{
                for (_,name,_share,_,plan) in rows.iter().take(32){
                    ui.label(RichText::new(name).strong().size(12.0).color(TEXT));
                    ui.horizontal_wrapped(|ui|{
                        for (label,result) in [
                            (tr("Catch / throw"),plan.as_ref()
                                .map(|p|format!("{:.3}%",p.chance*100.0))
                                .unwrap_or_else(||"—".into())),
                            (tr("Caught / h"),value(plan.as_ref().map(|p|p.catches_h))),
                            (tr("Shiny / h"),value(plan.as_ref()
                                .and_then(|p|p.shiny_caught_h))),
                            (tr("Shiny ≥1 / 24h"),plan.as_ref()
                                .and_then(|p|p.shiny_caught_24h)
                                .map(|p|format!("{:.2}%",p*100.0))
                                .unwrap_or_else(||"—".into())),
                        ]{
                            ui.vertical(|ui|{
                                ui.label(RichText::new(label).size(9.0).color(DIM));
                                ui.label(RichText::new(result).size(11.0).color(TEXT));
                            });
                            ui.add_space(10.0);
                        }
                    });
                    ui.separator();
                }
            }
            if rows.len()>32 {ui.label(tr("First 32 species shown."));}
        });
    });
    ui.label(RichText::new(tr("Unknown rarity or combat pace = unavailable, not zero. Shiny capture probability in Outland is not inferred from generic wiki odds."))
        .size(10.0).color(MUTED));
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui|{
        ui.label(RichText::new(tr("Rare items & Outland"))
            .size(13.0).strong().color(TEXT));
        ui.label(tr("Ranking target"));
        egui::ComboBox::from_id_salt("atlas-rare-target")
            .selected_text(match scenario.rare_target{
                70000=>tr("Bronze Boss Token"),70011=>tr("Key Fragment"),
                70012=>tr("Shiny Stone Fragment"),70013=>tr("Bicycle Fragment"),
                70017=>tr("Shiny Egg Fragment"),_=>"Unknown",
            }).show_ui(ui,|ui|{
                for (id,name) in [
                    (70000,tr("Bronze Boss Token")),(70011,tr("Key Fragment")),
                    (70012,tr("Shiny Stone Fragment")),(70013,tr("Bicycle Fragment")),
                    (70017,tr("Shiny Egg Fragment")),
                ]{
                    ui.selectable_value(&mut scenario.rare_target,id,name);
                }
            });
    });
    if let Some(speed)=kills {
        let rare=models::rare_scenarios(outland,hunt.level,scenario.tier,
            view.trainer_level,speed,&scenario.modifiers);
        if rare.is_empty(){
            ui.label(RichText::new(if outland{
                tr("Preview tier is locked at this trainer level, or the level is unavailable.")
            }else{tr("This destination is below the shiny-egg threshold.")})
                .size(11.0).color(MUTED));
        }
        let columns=if ui.available_width()>=820.0{2}else{1};
        ui.columns(columns,|cols|{
            for (i,item) in rare.iter().enumerate(){
                let ui=&mut cols[i%columns];
            egui::Frame::new().fill(PANEL_ALT).corner_radius(7.0)
                .inner_margin(egui::Margin::symmetric(10,7)).show(ui,|ui|{
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(item.name)
                            .size(11.0).strong().color(TEXT));
                        ui.label(RichText::new(format!(
                            "{:.6}% per eligible kill",item.per_kill*100.0
                        )).size(10.0).color(MUTED));
                    });
                    ui.horizontal_wrapped(|ui|{
                        for (label,number) in [
                            (tr("Units/h"),format!("{:.5}",item.units_h)),
                            (tr("Chance / 24h"),format!("{:.2}%",item.chance_24h*100.0)),
                            (tr("Craft time"),format!("{:.1}h",item.craft_hours)),
                        ]{
                            ui.vertical(|ui|{
                                ui.label(RichText::new(label).size(9.0).color(DIM));
                                ui.label(RichText::new(number)
                                    .size(11.0).strong().color(TEXT));
                            });
                            ui.add_space(12.0);
                        }
                    });
                }).response.on_hover_text(format!(
                    "Mean hours to next drop: {:.1}",item.mean_hours
                ));
            }
        });
    }else{
        ui.label(tr("Rare drops need a modeled or measured kill rate."));
    }
    if outland {
        ui.collapsing(tr("All eight Outland tiers · base drop probabilities"),|ui|{
            egui::ScrollArea::horizontal()
                .id_salt(("atlas-outland-table-scroll",&hunt.slug))
                .show(ui,|ui|{
            egui::Grid::new(("atlas-outland-all",&hunt.slug))
                .striped(true).spacing([14.0,5.0]).show(ui,|ui|{
                    ui.strong(tr("Tier"));ui.strong(tr("Required Lv"));ui.strong(tr("Multiplier"));
                    ui.strong(tr("Token %/kill"));ui.strong(tr("Key/Stone/Bicycle %/kill"));
                    ui.strong(tr("Access"));ui.end_row();
                    for idx in 0..8{
                        let tier=(idx+1) as u8;
                        let mult=models::TIER_MULTIPLIERS[idx];
                        ui.label(tier.to_string());
                        ui.label(models::TIER_LEVELS[idx].to_string());
                        ui.label(format!("{mult:.2}×"));
                        ui.label(format!("{:.5}%",0.0005*mult*100.0));
                        ui.label(format!("{:.6}%",0.000005*mult*100.0));
                        ui.label(if models::unlocked_tier(tier,view.trainer_level).is_some(){
                            tr("Level eligible")
                        }else{tr("Locked / unknown")});
                        ui.end_row();
                    }
                });
                });
            ui.label(RichText::new(tr("The guide mentions Bicycle Fragments here, but the game has not confirmed this. Reaching the required level does not guarantee access."))
                .size(10.0).color(WARN));
        });
    }else{
        ui.label(RichText::new(tr("Outside Outland: Shiny Egg Fragment uses the upstream 30-threshold level ladder; 10 fragments per craft, 168h reference incubation."))
            .size(10.0).color(MUTED));
    }

    // The game no longer awards NPC drops, and catalog prices do not
    // represent player market sales. Show only modeled combat gold and
    // explicit item costs instead of fabricating hourly market revenue.
    ui.add_space(8.0);
    ui.label(RichText::new(tr("Gold and capture costs"))
        .size(13.0).strong().color(TEXT));
    let accepted_h=kills.map(|k|k*scenario.accepted_fraction_override.unwrap_or(1.0));
    let gold_cost=ball_quote.and_then(|q|q.gold_price)
        .filter(|price|*price>0.0 && ball_quote.and_then(|q|q.diamond_price)
            .is_none_or(|d|d<=0.0))
        .zip(accepted_h).map(|(price,h)|price*h);
    let diamond_cost=ball_quote.and_then(|q|q.diamond_price)
        .filter(|price|*price>0.0 && ball_quote.and_then(|q|q.gold_price)
            .is_none_or(|g|g<=0.0))
        .zip(accepted_h).map(|(price,h)|price*h);
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .stroke(Stroke::new(1.0,BORDER))
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                for (label,num) in [
                    (tr("Gross gold/h"),f.as_ref().filter(|f|f.calibrated).map(|f|f.combat_gold_h)),
                    (tr("Ball gold cost / h"),gold_cost),
                    (tr("Ball diamonds / h"),diamond_cost),
                ]{
                    ui.vertical(|ui|{
                        ui.label(RichText::new(label).size(10.0).color(DIM));
                        ui.label(RichText::new(value(num))
                            .size(14.0).strong().color(TEXT));
                    });
                    ui.add_space(12.0);
                }
            });
        });
    ui.label(RichText::new(tr("Converted ordinary loot is already included in combat gold. No NPC sales or player-market proceeds are assumed."))
        .size(10.0).color(MUTED));
    ui.add_space(10.0);
    ui.collapsing(tr("History & forecast comparisons"),|ui|{
        draw_retrospective(ui,history,Some(&hunt.slug),f.as_ref());
    });
}

