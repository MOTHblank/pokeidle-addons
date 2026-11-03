use super::*;
pub(super) fn draw_atlas_current(
    ui: &mut egui::Ui, view: &HuntView,
    reference: Option<&AtlasReference>, estimates: &EstimateCache,
    aoe_enabled: bool,
) {
                    let hunts=&view;
                    let (activity,activity_color)=atlas_combat_status(hunts,live);
                    ui.horizontal_wrapped(|ui|{
                        ui.label(RichText::new(tr("Current hunt"))
                            .size(15.0).strong().color(TEXT));
                        ui.colored_label(activity_color,format!("● {activity}"));
                    });
                    let current=hunts.hunts.iter().find(|h|
                        hunts.hunt_slug.as_deref()==Some(h.slug.as_str()));
                    let title=if hunts.in_center{tr("At Pokémon Center")}else{
                        current.map(|h|h.name.as_str())
                            .or(hunts.hunt_slug.as_deref()).unwrap_or(tr("No confirmed hunt"))
                    };
                    ui.label(RichText::new(format!(
                        "{title} · {} {} · {} {}",
                        hunts.hunts.len(),tr("unlocked hunts"),hunts.battle_kills,tr("recorded kills")
                    )).size(11.0).color(MUTED));
                    ui.add_space(8.0);
                    let two_columns=ui.available_width()>=720.0;
                    ui.columns(if two_columns{2}else{1},|cols|{
                        egui::Frame::new().fill(PANEL).corner_radius(9.0)
                            .stroke(Stroke::new(1.0,BORDER))
                            .inner_margin(egui::Margin::same(12))
                            .show(&mut cols[0],|ui|{
                                ui.label(RichText::new(tr("LEAD & HUNT")).size(10.0)
                                    .strong().color(DIM));
                                ui.columns(2,|sides|{
                                    sides[0].vertical_centered(|ui|{
                                        ui.label(RichText::new(tr("YOUR POKÉMON"))
                                            .size(9.0).strong().color(DIM));
                                        if let Some(p)=hunts.active_pokemon.as_ref(){
                                            ui.label(RichText::new(&p.name)
                                                .size(13.0).strong().color(TEXT));
                                            ui.label(RichText::new(format!(
                                                "Lv {} · {}",p.level,
                                                if p.shiny{"Shiny"}else{tr("Active")}
                                            )).size(10.0).color(MUTED));
                                        }else{
                                            ui.label(RichText::new(tr("Lead not loaded"))
                                                .size(11.0).color(MUTED));
                                        }
                                    });
                                    sides[1].vertical_centered(|ui|{
                                        ui.label(RichText::new(tr("HUNT SPECIES"))
                                            .size(9.0).strong().color(DIM));
                                        let hunt_name=if hunts.in_center {
                                            tr("At Pokémon Center")
                                        } else {
                                            current.map(|h|h.name.as_str())
                                                .or(hunts.hunt_slug.as_deref())
                                                .unwrap_or(tr("No confirmed hunt"))
                                        };
                                        ui.label(RichText::new(hunt_name)
                                            .size(13.0).strong().color(TEXT));
                                        if let Some(h)=current.filter(|_|!hunts.in_center){
                                            ui.label(RichText::new(format!(
                                                "Lv {} · {}",h.level,tr("hunt species")
                                            )).size(10.0).color(MUTED));
                                        }else if hunts.in_center{
                                            ui.label(RichText::new(tr("Not hunting"))
                                                .size(10.0).color(MUTED));
                                        }
                                    });
                                });
                                if let Some(p)=hunts.active_pokemon.as_ref(){
                                    ui.separator();
                                    ui.label(RichText::new(format!(
                                        "P{} · {} {} · {} {}",
                                        p.potency.map(|n|n.to_string())
                                            .unwrap_or_else(||"—".into()),
                                        tr("Quality"),p.quality.map(|n|format!("{n:.3}"))
                                            .unwrap_or_else(||"—".into()),
                                        hunts.pokemon_count,tr("Pokémon in party")
                                    )).size(10.0).color(MUTED));
                                    let names=["HP","ATK","DEF",tr("SpA"),tr("SpD"),"SPD"];
                                    ui.label(RichText::new(names.iter().enumerate()
                                        .map(|(i,k)|format!("{} {}",k,
                                            p.ivs[i].map(|v|v.to_string())
                                                .unwrap_or_else(||"—".into())))
                                        .collect::<Vec<_>>().join(" · "))
                                        .size(10.0).color(DIM));
                                }
                            });
                        if !two_columns{cols[0].add_space(8.0);}
                        egui::Frame::new().fill(PANEL).corner_radius(9.0)
                            .stroke(Stroke::new(1.0,BORDER))
                            .inner_margin(egui::Margin::same(12))
                            .show(&mut cols[if two_columns{1}else{0}],|ui|{
                                let b=&hunts.bonuses;
                                ui.label(RichText::new(tr("BONUSES & LOCATION"))
                                    .size(10.0).strong().color(DIM));
                                ui.label(xp_bonus_summary(b));
                                if let Some(ranking)=guild_ranking_bonus(b) {
                                    ui.label(RichText::new(ranking).size(10.0).color(MUTED));
                                }
                                ui.label(format!(
                                    "{} {} · {} {}",
                                    tr("Trainer event"),known_percent(b.trainer_event_pct,b.trainer_event_known),
                                    tr("Pokémon event"),known_percent(b.pokemon_event_pct,b.pokemon_event_known)
                                ));
                                ui.separator();
                                if let Some(h)=current{
                                    ui.label(RichText::new(format!(
                                        "{} · {} · Lv {} · {} {}",
                                        h.area,h.region,h.level,h.total_spawns,tr("spawn slots")
                                    )).size(10.0).color(MUTED));
                                }
                                if let Some(slug)=hunts.pending_hunt_slug.as_deref(){
                                    ui.colored_label(WARN,tr_dynamic(format!("Switch pending: {slug}")));
                                }
                                if hunts.pending_center {
                                    ui.colored_label(WARN,tr("Pokémon Center pending"));
                                }
                                if hunts.in_center{
                                    ui.colored_label(WARN,tr("At Pokémon Center"));
                                }
                                if let Some(map)=hunts.field_map.as_deref(){
                                    ui.label(RichText::new(tr_dynamic(format!("Field: {map}")))
                                        .size(10.0).color(DIM));
                                }
                            });
                    });
                    ui.add_space(8.0);
                    let guide=reference;
                    let lead_types=guide.map(|guide|
                        atlas_types_for_active(hunts,guide))
                        .unwrap_or_default();
                    let wild_target=guide.and_then(|guide|
                        hunts.wild.as_ref().and_then(|wild|
                            wild.species_id.and_then(|id|
                                guide.species.get(&id)
                                    .map(|species|(species.name.as_str(),
                                        species.types.as_slice(),
                                        tr("Current wild species"))))));
                    let representative=guide.and_then(|guide|
                        current.and_then(|hunt|hunt.species.iter()
                            .filter(|species|species.weight>0.0)
                            .max_by(|left,right|
                                left.weight.total_cmp(&right.weight))
                            .and_then(|spawn|guide.species.get(&spawn.id)
                                .map(|species|(species.name.as_str(),
                                    species.types.as_slice(),
                                    tr("Most common hunt species"))))));
                    let target=representative.or(wild_target);
                    let (target_name,target_types,note)=target.unwrap_or((
                        tr("Unidentified opponent"),&[][..],
                        tr("Opponent typing is unavailable; no advantages or weaknesses are assumed.")
                    ));
                    draw_atlas_matchup(ui,guide,&lead_types,target_types,
                        hunts.active_pokemon.as_ref().map(|p|p.name.as_str())
                            .unwrap_or(tr("Your Pokémon")),
                        target_name,note);
                    ui.add_space(8.0);
    let m = &hunts.measurements;
    egui::Frame::new().fill(PANEL_ALT).corner_radius(10.0)
        .inner_margin(egui::Margin::same(13)).show(ui, |ui| {
            ui.label(RichText::new(tr("Measured during current hunt"))
                .size(12.0).strong().color(TEXT));
            ui.horizontal_wrapped(|ui| {
                for (label, amount, valid) in [
                    (tr("Trainer XP/h"), m.trainer_xp_h, live && m.valid_trainer),
                    (tr("Pokémon XP/h"), m.pokemon_xp_h, live && m.valid_pokemon),
                    (tr("Gross gold/h"), m.gold_gross_h, live && m.valid_gross_gold),
                    (tr("Net gold/h"), m.gold_net_h, live && m.valid_gold),
                    (tr("Kills/h"), m.kills_h, live && m.seconds >= 20.0 &&
                        m.last_battle_age_secs.is_none_or(|age|age<=180)),
                ] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).size(10.0).color(DIM));
                        ui.label(RichText::new(if valid {
                            format_hunt_amount(amount)
                        } else { "—".into() }).size(15.0).strong()
                            .color(if valid { GOOD } else { MUTED }));
                    });
                    ui.add_space(13.0);
                }
            });
            ui.label(RichText::new(format!(
                "{} · {:.0}s · {} {}",
                hunts.sample_quality, m.seconds, m.kills, tr("kills in window")
            )).size(10.0).color(MUTED));
        });
    ui.add_space(10.0);
    egui::Frame::new().fill(PANEL).corner_radius(9.0)
        .inner_margin(egui::Margin::same(12)).show(ui,|ui|{
            ui.label(RichText::new(tr("Current hunt · estimated results"))
                .size(12.0).strong().color(TEXT));
            if let Some(forecast)=hunts.hunt_slug.as_deref()
                .and_then(|slug|estimates.get(slug)) {
                draw_atlas_combat_cues(ui,hunts.active_pokemon.as_ref(),Some(&forecast),
                    aoe_enabled);
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui|{
                    for (label,value) in [
                        (tr("Trainer XP/h"),Some(forecast.trainer_xp_h)),
                        (tr("Pokémon XP/h"),Some(forecast.pokemon_xp_h)),
                        (tr("Gross gold/h"),Some(forecast.gold_h)),
                        (tr("Kills/h"),Some(forecast.kills_h)),
                    ]{
                        draw_rate(ui,label,value,
                            if forecast.calibrated {RateSource::Calibrated}
                            else {RateSource::Experimental});
                        ui.add_space(12.0);
                    }
                });
                ui.label(RichText::new(format!(
                    "{:.2}s / {} · {:.0}% {}",
                    forecast.seconds_per_kill,tr("kill"),forecast.coverage*100.0,
                    tr("species modeled")
                )).size(10.0).color(MUTED));
                ui.horizontal_wrapped(|ui|{
                    ui.label(format!(
                        "{} {} · {} {}",
                        tr("Trainer XP/kill"),format_hunt_amount(forecast.trainer_xp_per_kill),
                        tr("Pokémon XP/kill"),format_hunt_amount(forecast.pokemon_xp_per_kill)
                    ));
                    ui.label(format!("{} {}",
                        tr("Combat gold/h"),format_hunt_amount(forecast.combat_gold_h)));
                });
                for (channel,observed,valid,estimate) in [
                    (tr("Trainer XP"),m.trainer_xp_h,m.valid_trainer,forecast.trainer_xp_h),
                    (tr("Pokémon XP"),m.pokemon_xp_h,m.valid_pokemon,forecast.pokemon_xp_h),
                ]{
                    if live && valid && observed>0.0{
                        let ratio=estimate/observed;
                        ui.label(RichText::new(format!(
                            "{channel}: measured {} / h vs model {} / h ({ratio:.2}×)",
                            format_hunt_amount(observed),format_hunt_amount(estimate)
                        )).size(10.0).color(
                            if (ratio-1.0).abs()>=0.25 {WARN}else{MUTED}
                        ));
                    }
                }
                ui.label(RichText::new(
                    tr("These trainer/Pokémon streams must not be added together. Modeled gross gold excludes spending.")
                ).size(10.0).color(MUTED));
            } else {
                ui.label(RichText::new(if estimates.is_pending(){
                    tr("Calculating your forecast…")
                }else{
                    tr("Combat estimate unavailable for current Pokémon.")
                }).size(10.0).color(MUTED));
            }
        });

}
