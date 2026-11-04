use super::*;

    #[test]
    fn center_return_keeps_last_server_confirmed_hunt() {
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry {
            slug:"kanto-route-1".into(),
            name:"Route 1".into(),
            ..Default::default()
        }]);
        view.hunt_slug=Some("kanto-route-1".into());
        view.in_center=true;
        assert_eq!(atlas_return_hunt_slug(&view),Some("kanto-route-1"));
        // A public guide entry or stale ID is not an executable destination.
        view.hunt_slug=Some("guide-only-route".into());
        assert_eq!(atlas_return_hunt_slug(&view),None);
        view.hunt_slug=None;
        assert_eq!(atlas_return_hunt_slug(&view),None);
    }
    #[test]
    fn weakness_palette_matches_all_eighteen_classic_pokemon_colors() {
        let expected=[
            ("NORMAL",0xA8A77A),("FIRE",0xEE8130),
            ("WATER",0x6390F0),("ELECTRIC",0xF7D02C),
            ("GRASS",0x7AC74C),("ICE",0x96D9D6),
            ("FIGHTING",0xC22E28),("POISON",0xA33EA1),
            ("GROUND",0xE2BF65),("FLYING",0xA98FF3),
            ("PSYCHIC",0xF95587),("BUG",0xA6B91A),
            ("ROCK",0xB6A136),("GHOST",0x735797),
            ("DRAGON",0x6F35FC),("DARK",0x705746),
            ("STEEL",0xB7B7CE),("FAIRY",0xD685AD),
        ];
        assert_eq!(ATLAS_TYPES.len(),expected.len());
        for (index,(kind,hex)) in expected.iter().enumerate(){
            assert_eq!(ATLAS_TYPES[index],*kind);
            let color=atlas_type_color(kind);
            let actual=((color.r() as u32)<<16)
                |((color.g() as u32)<<8)|(color.b() as u32);
            assert_eq!(actual,*hex,"incorrect Pokémon type color for {kind}");
            let text=atlas_type_text_color(color);
            let linear=|byte:u8|{
                let x=byte as f64/255.0;
                if x<=0.04045{x/12.92}else{((x+0.055)/1.055).powf(2.4)}
            };
            let brightness=0.2126*linear(color.r())+
                0.7152*linear(color.g())+0.0722*linear(color.b());
            let contrast=if text==Color32::BLACK{
                (brightness+0.05)/0.05
            }else{1.05/(brightness+0.05)};
            assert!(contrast>=4.5,
                "type button label lacks readable contrast: {kind} {contrast}");
        }
    }
    #[test]
    fn weakness_tiles_reflow_without_horizontal_overflow() {
        assert_eq!(atlas_type_grid_columns(180.0),1);
        assert_eq!(atlas_type_grid_columns(280.0),2);
        assert_eq!(atlas_type_grid_columns(420.0),3);
        assert_eq!(atlas_type_grid_columns(620.0),5);
        assert_eq!(atlas_type_grid_columns(1100.0),6);
        for width in [180.0,280.0,420.0,620.0,1100.0] {
            let columns=atlas_type_grid_columns(width);
            let gap=6.0;
            let tile=(width-gap*(columns-1) as f32)/columns as f32;
            assert!(columns as f32*tile+(columns-1) as f32*gap<=width+0.1);
            assert!(tile>=112.0 || columns==1);
        }
    }
    #[test]
    fn weakness_selection_preserves_all_and_any_matching() {
        let mut guide=AtlasReference::default();
        guide.chart.insert("FIRE".into(),
            [("GRASS".into(),2.0)].into_iter().collect());
        guide.chart.insert("ICE".into(),
            [("GRASS".into(),2.0)].into_iter().collect());
        guide.chart.insert("WATER".into(),
            [("GRASS".into(),0.5)].into_iter().collect());
        // The chart validates each defending type as a known type.
        guide.chart.insert("GRASS".into(),Default::default());
        let defender=vec!["GRASS".to_owned()];
        let selected=["FIRE".to_owned(),"WATER".to_owned()]
            .into_iter().collect::<std::collections::BTreeSet<_>>();
        let is_weak=|attack:&String|atlas_type_multiplier(&guide,attack,&defender)
            .is_some_and(|multiplier|multiplier>1.0);
        assert!(selected.iter().any(is_weak));
        assert!(!selected.iter().all(is_weak));
    }
    fn guide_fixture()->AtlasReference{
        let mut guide=AtlasReference::default();
        guide.chart.insert("FIGHTING".into(),
            [("NORMAL".into(),2.0)].into_iter().collect());
        guide.chart.insert("GHOST".into(),
            [("NORMAL".into(),0.0)].into_iter().collect());
        guide.chart.insert("NORMAL".into(),Default::default());
        guide.species.insert(1,native_atlas::ReferenceSpecies{
            name:"Testmon".into(),types:vec!["NORMAL".into()],
            base:vec![100.0;6],moves:vec![]
        });
        guide
    }
    #[test]
    fn type_matchups_respect_immunity_stacked_dual_types_and_neutral(){
        let mut guide=AtlasReference::default();
        for kind in ["FIRE","GRASS","ICE","WATER","ROCK","NORMAL","GHOST"]{
            guide.chart.insert(kind.into(),Default::default());
        }
        guide.chart.get_mut("FIRE").unwrap().extend([
            ("GRASS".into(),2.0),("ICE".into(),2.0),
            ("WATER".into(),0.5),("ROCK".into(),0.5),
        ]);
        guide.chart.get_mut("NORMAL").unwrap()
            .insert("GHOST".into(),0.0);
        let hit=|attack:&str,types:&[&str]|{
            atlas_type_multiplier(&guide,attack,
                &types.iter().map(|v|v.to_string()).collect::<Vec<_>>())
        };
        assert_eq!(hit("NORMAL",&["GHOST"]),Some(0.0));
        assert_eq!(hit("FIRE",&["WATER"]),Some(0.5));
        assert_eq!(hit("FIRE",&["WATER","ROCK"]),Some(0.25));
        assert_eq!(hit("FIRE",&["GRASS"]),Some(2.0));
        assert_eq!(hit("FIRE",&["GRASS","ICE"]),Some(4.0));
        assert_eq!(hit("FIRE",&["NORMAL"]),Some(1.0));
        assert_eq!(hit("fire",&["Grass","ICE"]),Some(4.0));
        assert_eq!(hit("FIRE",&[]),None);
        assert_eq!(hit("FIRE",&["NOT_IN_CHART"]),None);
        assert_eq!(hit("UNKNOWN",&["ICE"]),None);
    }
    #[test]
    fn matchup_colors_change_with_damage_direction(){
        assert_eq!(atlas_multiplier_color(4.0,false),GOOD);
        assert_eq!(atlas_multiplier_color(4.0,true),BAD);
        assert_eq!(atlas_multiplier_color(0.0,false),BAD);
        assert_eq!(atlas_multiplier_color(0.0,true),GOOD);
        assert_eq!(atlas_multiplier_color(0.5,false),BAD);
        assert_eq!(atlas_multiplier_color(0.5,true),GOOD);
        assert_eq!(atlas_multiplier_text(0.0),"0×");
        assert_eq!(atlas_multiplier_text(0.25),"0,25×");
        assert_eq!(atlas_multiplier_text(0.5),"0,5×");
        assert_eq!(atlas_multiplier_text(2.0),"2×");
        assert_eq!(atlas_multiplier_text(4.0),"4×");
    }
    #[test]
    fn live_party_types_take_precedence_over_guide_fallback(){
        let mut guide=AtlasReference::default();
        guide.species.insert(25,native_atlas::ReferenceSpecies{
            name:"Pikachu".into(),types:vec!["ELECTRIC".into()],
            base:vec![10.0;6],moves:vec![]
        });
        let mut view=HuntView::default();
        assert!(atlas_types_for_active(&view,&guide).is_empty());
        view.active_pokemon=Some(native_hunts::PokemonInfo{
            species_id:Some(25),..Default::default()
        });
        assert_eq!(atlas_types_for_active(&view,&guide),vec!["ELECTRIC"]);
        view.active_pokemon.as_mut().unwrap().types=vec!["STEEL".into()];
        assert_eq!(atlas_types_for_active(&view,&guide),vec!["STEEL"]);
    }
    #[test]
    fn explorer_release_drops_previous_source_handles() {
        let mut cache=ExplorerCache::default();
        let view=HuntView::default();
        let original=Arc::clone(&view.hunts);
        cache.update(&view,None,&EstimateCache::default(),
            &AtlasFilters::default(),&AtlasScenario::default());
        assert!(Arc::strong_count(&original)>=3);
        cache.release();
        assert_eq!(Arc::strong_count(&original),2);
        assert!(!cache.initialized);
    }
    #[test]
    fn cached_destination_does_not_clone_hunt_species() {
        let mut hunt=native_hunts::HuntEntry::default();
        hunt.slug="test".into();
        hunt.name="Test".into();
        hunt.level=100;
        hunt.species.push(native_hunts::HuntSpecies::default());
        let label=AtlasHuntLabel::from_hunt(&hunt);
        assert_eq!(label.slug,"test");
        assert_eq!(label.name,"Test");
        assert_eq!(label.level,100);
    }
    #[test]
    fn species_action_uses_an_unlocked_destination_not_a_locked_best(){
        let locked=native_hunts::HuntEntry{
            slug:"guide-only".into(),..Default::default()
        };
        let accessible=native_hunts::HuntEntry{
            slug:"server-unlocked".into(),..Default::default()
        };
        let mut row=AtlasSpeciesRow{
            id:1,name:"Testmon".into(),types:vec![],owned:false,
            best:Some(200.0),
            destinations:vec![
                AtlasDestination{
                    hunt:Arc::new(AtlasHuntLabel::from_hunt(&locked)),unlocked:false,
                    forecast:None,objective:Some(200.0),mkt_value:None
                },
                AtlasDestination{
                    hunt:Arc::new(AtlasHuntLabel::from_hunt(&accessible)),unlocked:true,
                    forecast:None,objective:Some(100.0),mkt_value:None
                }
            ],
        };
        assert_eq!(
            atlas_first_unlocked(&row).map(|destination|destination.hunt.slug.as_str()),
            Some("server-unlocked")
        );
        row.destinations[1].unlocked=false;
        assert!(atlas_first_unlocked(&row).is_none());
    }
    #[test]
    fn hunt_status_uses_confirmed_location_not_wild_slot(){
        let mut view=HuntView::default();
        assert_eq!(atlas_combat_status(&view,false).0,tr("Offline / stale"));
        assert_eq!(atlas_combat_status(&view,true).0,tr("No confirmed hunt"));
        view.hunt_slug=Some("scyther-field".into());
        assert_eq!(atlas_combat_status(&view,true).0,tr("Hunting"));
        view.measurements.combat_event_age_secs=Some(10);
        assert_eq!(atlas_combat_status(&view,true).0,tr("Hunt active"));
        view.wild=Some(native_hunts::WildInfo::default());
        assert_eq!(atlas_combat_status(&view,true).0,tr("Hunt active"));
        view.wild=None; // Encounter cleared; location and activity persist.
        assert_eq!(atlas_combat_status(&view,true).0,tr("Hunt active"));
        view.measurements.combat_event_age_secs=Some(181);
        assert_eq!(atlas_combat_status(&view,true).0,tr("In hunt · no recent activity"));
        view.in_center=true;
        assert_eq!(atlas_combat_status(&view,true).0,tr("At Pokémon Center"));
    }
    #[test]
    fn location_selector_uses_game_area_and_region(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"same".into(),area:"Kanto".into(),region:"Viridian".into(),
            ..Default::default()
        }]);
        let names=atlas_locations(&view,None);
        assert!(["Kanto","Viridian"].iter().all(|n|names.contains(*n)));
        assert!(!names.contains("Outland"));
        assert!(atlas_hunt_matches_location(&view.hunts[0],"Kanto"));
        assert!(atlas_hunt_matches_location(&view.hunts[0],"Viridian"));
        assert!(!atlas_hunt_matches_location(&view.hunts[0],"Johto"));
    }
    #[test]
    fn explorer_shows_only_authenticated_game_destinations(){
        let catalog=guide_fixture();
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"server-unlocked".into(),name:"Current".into(),level:50,
            species:vec![native_hunts::HuntSpecies{id:1,weight:80.0}],
            ..Default::default()
        }]);
        let locations=atlas_destinations(&view,Some(&catalog));
        assert_eq!(locations.len(),1);
        assert!(locations.iter().all(|(hunt,unlocked)|
            *unlocked && hunt.slug=="server-unlocked"));
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        let rows=atlas_species_rows(&view,Some(&catalog),&cache,&filters,
            &AtlasScenario::default());
        assert_eq!(rows.len(),1);
        assert_eq!(rows[0].destinations.len(),1);
        filters.min_level=250;
        assert!(atlas_species_rows(&view,Some(&catalog),&cache,&filters,
            &AtlasScenario::default()).is_empty());
    }
    #[test]
    fn multi_weakness_filter_uses_each_multiplier_for_any_and_all_modes(){
        let mut guide=AtlasReference::default();
        for kind in ["FIRE","FIGHTING","GRASS","ICE","NORMAL","ROCK"]{
            guide.chart.insert(kind.into(),Default::default());
        }
        guide.chart.get_mut("FIRE").unwrap().extend([
            ("GRASS".into(),2.0),("ICE".into(),2.0),
            ("NORMAL".into(),1.0),("ROCK".into(),2.0),
        ]);
        guide.chart.get_mut("FIGHTING").unwrap().extend([
            ("GRASS".into(),0.5),("ICE".into(),1.0),
            ("NORMAL".into(),2.0),("ROCK".into(),2.0),
        ]);
        for (id,name,types) in [
            (1,"Four times Fire",vec!["GRASS","ICE"]),
            (2,"Double Fire Quadruple Fighting",vec!["NORMAL","ROCK"]),
        ]{
            guide.species.insert(id,native_atlas::ReferenceSpecies{
                name:name.into(),types:types.into_iter().map(str::to_owned).collect(),
                base:vec![100.0;6],moves:vec![],
            });
        }
        let types_a=&guide.species[&1].types;
        let types_b=&guide.species[&2].types;
        assert_eq!(atlas_type_multiplier(&guide,"FIRE",types_a),Some(4.0));
        assert_eq!(atlas_type_multiplier(&guide,"FIGHTING",types_a),Some(0.5));
        assert_eq!(atlas_type_multiplier(&guide,"FIRE",types_b),Some(2.0));
        assert_eq!(atlas_type_multiplier(&guide,"FIGHTING",types_b),Some(4.0));
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"multi".into(),name:"Multi".into(),level:100,
            species:vec![
                native_hunts::HuntSpecies{id:1,weight:50.0},
                native_hunts::HuntSpecies{id:2,weight:50.0},
            ],
            ..Default::default()
        }]);
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.weak_to.extend(["FIRE".into(),"FIGHTING".into()]);
        let scenario=AtlasScenario::default();
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows.len(),2,"any-mode shows species weak to either");
        filters.weakness_match_all=true;
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows.len(),1,"all-mode requires both selected types >1×");
        assert_eq!(rows[0].id,2);
    }
    #[test]
    fn explorer_weakness_and_depot_filters_follow_selected_account(){
        let guide=guide_fixture();
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"current".into(),name:"Current".into(),level:100,
            species:vec![native_hunts::HuntSpecies{id:1,weight:100.0}],
            ..Default::default()
        }]);
        view.pokemon_roster=Arc::new(vec![native_hunts::PokemonInfo{
            species_id:Some(1),id:15,..Default::default()
        }]);
        view.roster_ready=true;
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.collection=AtlasCollection::InDepot;
        filters.weak_to.insert("FIGHTING".into());
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).len(),1);
        filters.weak_to.insert("GHOST".into());
        filters.weakness_match_all=true;
        assert!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).is_empty());
        filters.weakness_match_all=false;
        filters.collection=AtlasCollection::NotInDepot;
        assert!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&AtlasScenario::default()).is_empty());
    }

    #[test]
    fn gross_gold_and_sale_value_do_not_include_obsolete_npc_drops(){
        let hunt=native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:120,area:"Kanto".into(),
            ..Default::default()
        };
        let forecast=native_atlas::Forecast {
            kills_h:100.0,trainer_xp_h:1000.0,pokemon_xp_h:900.0,
            gold_h:80.0,combat_gold_h:80.0,
            trainer_xp_per_kill:10.0,pokemon_xp_per_kill:9.0,
            seconds_per_kill:36.0,coverage:1.0,calibrated:false,
            deaths_h:0.0,one_hit:false,aoe:false,engaged:1.0,
            worst_hit_fraction:0.25,incoming_damage_per_kill:0.0,
        };
        assert_eq!(atlas_objective(AtlasSort::Gold,Some(&forecast),&hunt,1.0,None),Some(80.0));
        let calibrated=native_atlas::Forecast{calibrated:true,..forecast.clone()};
        assert_eq!(atlas_objective(AtlasSort::Gold,Some(&calibrated),&hunt,1.0,None),Some(80.0));
        assert_eq!(atlas_objective(AtlasSort::Xp,Some(&forecast),&hunt,1.0,None),Some(1000.0));
        assert_eq!(atlas_objective(AtlasSort::SaleValue,None,&hunt,1.0,Some(500.0)),Some(500.0));
        assert_eq!(atlas_objective(AtlasSort::SaleValue,None,&hunt,1.0,None),None);
        assert_eq!(atlas_objective(AtlasSort::Shiny,None,&hunt,0.5,None),None);
        assert_eq!(atlas_objective(AtlasSort::Shiny,Some(&forecast),&hunt,0.5,None),Some(100.0*0.5/24000.0));
        let outland=native_hunts::HuntEntry{area:"Outland".into(),..hunt};
        assert_eq!(atlas_objective(AtlasSort::Shiny,Some(&forecast),&outland,0.5,None),None);
    }
    #[test]
    fn capture_chance_and_sale_value_do_not_depend_on_kills_per_hour(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"route".into(),name:"Route".into(),level:120,
            species:vec![native_hunts::HuntSpecies{id:1,weight:100.0}],
            ..Default::default()
        }]);
        let mut guide=guide_fixture();
        let cache=EstimateCache::default();
        let mut filters=AtlasFilters::default();
        filters.sort=AtlasSort::Capture;
        let scenario=AtlasScenario::default();
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows[0].best,None);
        guide.capture_rarities.insert(1,12000.0);
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(rows[0].best,None,
            "Old economic rarity must not masquerade as ordinary hunt capture odds");
        guide.capture_stages.insert(1,20);
        let rows=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert!(rows[0].best.is_some_and(|n|n>0.0&&n<=0.10));
        filters.sort=AtlasSort::SaleValue;
        assert_eq!(atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario)[0].best,None);
        guide.catalog_sell_values.insert(1,400.0);
        let after=atlas_species_rows(&view,Some(&guide),&cache,&filters,&scenario);
        assert_eq!(after[0].best,Some(560.0)); // ×1.4: wild 120, capture tier 20
    }
    #[test]
    fn unknown_rate_is_never_turned_into_zero(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:100,..Default::default()
        }]);
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        let rows=ranked_hunts(&view,None,HuntRanking::TrainerXp,true,"","","",&cache);
        assert_eq!(rows.len(),1);
        assert!(rows[0].value.is_none());
        assert_eq!(rows[0].source,RateSource::Unavailable);
    }
    #[test]
    fn current_measured_and_other_scenario_are_separately_labeled(){
        let mut view=HuntView::default();
        view.hunt_slug=Some("a".into());
        view.hunts=Arc::new(vec![
            native_hunts::HuntEntry{slug:"a".into(),name:"A".into(),level:100,..Default::default()},
            native_hunts::HuntEntry{slug:"b".into(),name:"B".into(),level:120,..Default::default()},
        ]);
        view.measurements.seconds=60.0;
        view.measurements.kills=10;
        view.measurements.kills_h=600.0;
        view.measurements.trainer_xp_h=1200.0;
        view.measurements.valid_trainer=true;
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        let rows=ranked_hunts(&view,None,HuntRanking::TrainerXp,true,"","","",&cache);
        assert_eq!(rows.iter().find(|r|r.hunt.slug=="a").unwrap().source,RateSource::Measured);
        assert_eq!(rows.iter().find(|r|r.hunt.slug=="b").unwrap().source,RateSource::Scenario);
        view.bonuses.vip_active=true;
        view.bonuses.guild_boost_active=true;
        let boosted=ranked_hunts(&view,None,HuntRanking::TrainerXp,true,"","","",&cache);
        assert_eq!(boosted.iter().find(|r|r.hunt.slug=="a").unwrap().value,Some(1200.0),
            "Measured XP already contains buffs and must not be multiplied again");
        let stale=ranked_hunts(&view,None,HuntRanking::TrainerXp,false,"","","",&cache);
        assert!(stale.iter().all(|r|r.value.is_none()));
    }
    #[test]
    fn bonus_changes_invalidate_forecast_cache_without_lead_changes(){
        let mut view=HuntView::default();
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        cache.refreshed=Some(Instant::now());
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_some());
        view.bonuses.vip_active=true;
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none(),"VIP activation must invalidate XP forecasts");
        cache.refreshed=Some(Instant::now());
        view.bonuses.guild_boost_active=true;
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none(),"guild XP activation must invalidate forecasts");
        cache.refreshed=Some(Instant::now());
        view.bonuses.vip_active=false;
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none(),"VIP expiry must invalidate forecasts");
        cache.refreshed=Some(Instant::now());
        view.bonuses.sacrifice_xp_active=true;
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none(),"Sacrifice boost activation must invalidate forecasts");
        cache.refreshed=Some(Instant::now());
        view.bonuses.sacrifice_xp_active=false;
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none(),"Sacrifice boost expiry must invalidate forecasts");
    }
    #[test]
    fn timed_guild_and_vip_are_displayed_independently_of_guild_rank(){
        let mut b=native_hunts::HuntBonuses::default();
        b.twitch_known=true;
        b.twitch_pct=20.0;
        b.guild_known=true;
        b.guild_pct=0.0; // A zero guild rank does not disable the timed boost.
        b.guild_boost_known=true;
        b.guild_boost_active=true;
        b.vip_known=true;
        b.vip_active=true;
        let label=xp_bonus_summary(&b);
        assert!(label.contains("+20.0%"));
        assert!(label.contains("×1.1"));
        assert!(label.contains("×1.5"));
        assert!(!label.contains("+0.0%"));
        assert!(guild_ranking_bonus(&b).is_none());
        b.guild_pct=7.0;
        assert!(guild_ranking_bonus(&b).unwrap().contains("+7.0%"));
        b.vip_active=false;
        assert!(!xp_bonus_summary(&b).contains("×1.5"));
        b.vip_known=false;
        assert!(xp_bonus_summary(&b).contains(tr("Unknown")));
    }



    #[test]
    fn obsolete_forecast_worker_is_coalesced_and_never_published() {
        let mut view=HuntView::default();
        let guide=Arc::new(AtlasReference::default());
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,Some(&guide),true,false);
        // Simulate a slow in-flight worker whose receiver cannot complete.
        let (tx,rx)=std::sync::mpsc::sync_channel(1);
        cache.pending=Some((cache.generation,true,rx));
        cache.refreshed=None;
        let previous=cache.generation;
        view.bonuses.vip_active=true;
        cache.refresh(0,&view,Some(&guide),true,false);
        assert_ne!(previous,cache.generation);
        assert_eq!(cache.cancellation.load(std::sync::atomic::Ordering::Acquire),
            cache.generation);
        assert_eq!(cache.pending.as_ref().unwrap().0,previous,
            "no second worker may launch before the old one exits");
        let mut stale=std::collections::BTreeMap::new();
        stale.insert("obsolete".into(),Some(simulated_forecast(10.0,false)));
        tx.send(stale).unwrap();
        cache.refresh(0,&view,Some(&guide),true,false);
        assert!(!cache.values.contains_key("obsolete"),
            "an obsolete generation must not publish its result");
        assert_eq!(cache.pending.as_ref().unwrap().0,cache.generation,
            "newest request starts only after the previous worker finishes");
    }
    #[test]
    fn geometry_revision_recomputes_only_its_destination() {
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![
            native_hunts::HuntEntry{slug:"a".into(),..Default::default()},
            native_hunts::HuntEntry{slug:"b".into(),..Default::default()},
        ]);
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        cache.full_required=false;
        cache.refreshed=Some(Instant::now());
        cache.values.insert("a".into(),Some(simulated_forecast(100.0,false)));
        cache.values.insert("b".into(),Some(simulated_forecast(250.0,false)));
        view.geo_revisions=Arc::new(std::collections::BTreeMap::from([
            ("a".into(),1),("b".into(),1)]));
        cache.refresh(0,&view,None,true,false);
        assert!(cache.get("b").is_none(),
            "the first geometry sample affects both destinations");
        assert!(cache.geometry_dirty.contains("a"));
        assert!(cache.geometry_dirty.contains("b"));
        cache.geometry_dirty.clear();
        cache.values.insert("a".into(),Some(simulated_forecast(100.0,false)));
        cache.values.insert("b".into(),Some(simulated_forecast(250.0,false)));
        cache.refreshed=Some(Instant::now());
        let mut next=(*view.geo_revisions).clone();
        next.insert("a".into(),2);
        view.geo_revisions=Arc::new(next);
        cache.refresh(0,&view,None,true,false);
        assert_eq!(cache.geometry_dirty.len(),1);
        assert!(cache.geometry_dirty.contains("a"));
        assert!(cache.get("a").is_none());
        assert_eq!(cache.get("b").unwrap().kills_h,250.0,
            "unaffected forecasts must remain cached");
        // A completed geometry-only worker patches one row, not the full map.
        let (tx,rx)=std::sync::mpsc::sync_channel(1);
        cache.pending=Some((cache.generation,false,rx));
        tx.send(std::collections::BTreeMap::from([
            ("a".to_owned(),Some(simulated_forecast(175.0,false)))
        ])).unwrap();
        cache.refresh(0,&view,None,true,false);
        assert!(cache.geometry_dirty.is_empty());
        assert_eq!(cache.get("a").unwrap().kills_h,175.0);
        assert_eq!(cache.get("b").unwrap().kills_h,250.0);
        let refreshed=cache.refreshed;
        view.geo_revisions=Arc::new((*view.geo_revisions).clone());
        cache.refresh(0,&view,None,true,false);
        assert_eq!(cache.refreshed,refreshed,
            "a new map allocation without version changes is not invalidation");
    }
    #[test]
    fn combat_cache_does_not_recompute_by_age_or_hunt_switch(){
        let mut view=HuntView::default();
        let reference=Arc::new(AtlasReference::default());
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,Some(&reference),true,false);
        // Force a completed generation older than the former 20s timer.
        cache.pending=None;
        let old=Instant::now()-Duration::from_secs(120);
        cache.refreshed=Some(old);
        cache.refresh(0,&view,Some(&reference),true,false);
        assert!(cache.pending.is_none(),"age alone must not spawn a worker");
        assert_eq!(cache.refreshed,Some(old));
        view.hunt_slug=Some("other-hunt".into());
        cache.refresh(0,&view,Some(&reference),true,false);
        assert!(cache.pending.is_none(),"a hunt switch must reuse the same model");
        assert_eq!(cache.refreshed,Some(old));
    }
    #[test]
    fn combat_cache_invalidates_when_max_hp_or_species_changes(){
        let mut view=HuntView::default();
        view.active_pokemon=Some(native_hunts::PokemonInfo{
            id:1,species_id:Some(25),max_hp:Some(100),..Default::default()
        });
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        cache.refreshed=Some(Instant::now());
        view.active_pokemon.as_mut().unwrap().max_hp=Some(200);
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none());
        cache.refreshed=Some(Instant::now());
        view.active_pokemon.as_mut().unwrap().species_id=Some(26);
        cache.refresh(0,&view,None,true,false);
        assert!(cache.refreshed.is_none());
    }
    #[test]
    fn rankings_cache_ignores_receipts_when_forecasts_are_complete(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),..Default::default()
        }]);
        let mut estimates=EstimateCache::default();
        estimates.values.insert("a".into(),Some(simulated_forecast(300.0,false)));
        estimates.refreshed=Some(Instant::now());
        let mut results=RankingsCache::default();
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        assert_eq!(results.rebuilds,1);
        view.measurements.seconds=60.0;
        view.measurements.kills=10;
        view.measurements.kills_h=600.0;
        view.hunt_slug=Some("a".into());
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        assert_eq!(results.rebuilds,1);
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"A","","");
        assert_eq!(results.rebuilds,2,"filter edits must invalidate rankings");
        estimates.revision=estimates.revision.wrapping_add(1);
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"A","","");
        assert_eq!(results.rebuilds,3,"new forecasts must invalidate rankings");
    }
    #[test]
    fn repeated_invalidation_refreshes_rankings_even_without_a_completed_forecast() {
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),..Default::default()
        }]);
        let mut estimates=EstimateCache::default();
        estimates.refresh(0,&view,None,true,false);
        estimates.values.insert("a".into(),Some(simulated_forecast(250.0,false)));
        estimates.refreshed=None;
        let mut ranked=RankingsCache::default();
        let mut explorer=ExplorerCache::default();
        let filters=AtlasFilters::default();
        let scenario=AtlasScenario::default();
        ranked.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        explorer.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(ranked.rebuilds,1);
        assert_eq!(explorer.rebuilds,1);
        let baseline=estimates.revision;
        view.geo_revisions=Arc::new(std::collections::BTreeMap::from([
            ("a".into(),1)
        ]));
        estimates.refresh(0,&view,None,true,false);
        assert!(estimates.get("a").is_none());
        assert!(estimates.refreshed.is_none());
        assert_ne!(baseline,estimates.revision);
        ranked.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        explorer.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(ranked.rebuilds,2);
        assert_eq!(explorer.rebuilds,2);
        let mut newer=(*view.geo_revisions).clone();
        newer.insert("a".into(),2);
        view.geo_revisions=Arc::new(newer);
        estimates.refresh(0,&view,None,true,false);
        assert!(estimates.refreshed.is_none());
        ranked.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        explorer.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(ranked.rebuilds,3);
        assert_eq!(explorer.rebuilds,3);
    }
    #[test]
    fn rankings_cache_tracks_live_fallback_when_forecast_is_missing(){
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),..Default::default()
        }]);
        let estimates=EstimateCache::default();
        let mut results=RankingsCache::default();
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        assert_eq!(results.rebuilds,1);
        view.hunt_slug=Some("a".into());
        view.measurements.seconds=60.0;
        view.measurements.kills=10;
        view.measurements.kills_h=600.0;
        view.measurements.trainer_xp_h=1200.0;
        view.measurements.valid_trainer=true;
        results.update(&view,None,&estimates,HuntRanking::TrainerXp,true,"","","");
        assert_eq!(results.rebuilds,2);
        assert_eq!(results.rows[0].value,Some(1200.0));
    }
    #[test]
    fn explorer_cache_ignores_roster_hp_and_unrelated_scenario_edits(){
        let mut view=HuntView::default();
        view.pokemon_roster=Arc::new(vec![native_hunts::PokemonInfo{
            species_id:Some(25),hp:Some(20),..Default::default()
        }]);
        let estimates=EstimateCache::default();
        let filters=AtlasFilters::default();
        let mut scenario=AtlasScenario::default();
        let mut results=ExplorerCache::default();
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1);
        view.pokemon_roster=Arc::new(vec![native_hunts::PokemonInfo{
            species_id:Some(25),hp:Some(10),..Default::default()
        }]);
        scenario.modifiers_open=true;
        scenario.hunt_slug="different".into();
        scenario.ball=BallKind::Beast;
        view.hunt_slug=Some("other-hunt".into());
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1);
        view.pokemon_roster=Arc::new(vec![native_hunts::PokemonInfo{
            species_id:Some(26),hp:Some(10),..Default::default()
        }]);
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,2,"ownership changes must refresh badges");
    }
    #[test]
    fn explorer_cache_skips_selection_only_redraws_but_not_new_filters() {
        let view=HuntView::default();
        let estimates=EstimateCache::default();
        let scenario=AtlasScenario::default();
        let mut filters=AtlasFilters::default();
        let mut results=ExplorerCache::default();
        assert!(results.update(&view,None,&estimates,&filters,&scenario).is_empty());
        assert_eq!(results.rebuilds,1);
        filters.selected_species=Some(25);
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1);
        filters.min_level=80;
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,2);
    }
    #[test]
    fn explorer_gold_ignores_unrelated_combat_receipts_and_reuses_forecast_cache(){
        let mut view=HuntView::default();
        let estimates=EstimateCache::default();
        let scenario=AtlasScenario::default();
        let filters=AtlasFilters{sort:AtlasSort::Gold,..AtlasFilters::default()};
        let mut results=ExplorerCache::default();
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1);
        view.measurements.seconds=40.0;
        view.measurements.kills=10;
        view.measurements.valid_gross_gold=true;
        results.update(&view,None,&estimates,&filters,&scenario);
        assert_eq!(results.rebuilds,1,
            "receipts must not invalidate identical simulated gold rankings");
    }
    fn simulated_forecast(kills:f64,aoe:bool)->native_atlas::Forecast {
        native_atlas::Forecast{
            kills_h:kills,trainer_xp_h:kills*10.0,pokemon_xp_h:kills*10.0,
            gold_h:kills*5.0,combat_gold_h:kills*5.0,
            trainer_xp_per_kill:10.0,pokemon_xp_per_kill:10.0,
            seconds_per_kill:5.0,coverage:1.0,calibrated:false,
            deaths_h:0.0,one_hit:false,aoe,engaged:if aoe {4.0}else{1.0},
            worst_hit_fraction:0.2,incoming_damage_per_kill:10.0,
        }
    }
    #[test]
    fn aoe_cue_reflects_checkbox_not_server_tm_flag() {
        let mut mon=native_hunts::PokemonInfo::default();
        assert_eq!(atlas_aoe_cue(None,false),AtlasAoeCue::NoLead);
        assert_eq!(atlas_aoe_cue(Some(&mon),false),AtlasAoeCue::SingleTarget);
        mon.tm_aoe=true; // partial game state must not silently opt the user in
        assert_eq!(atlas_aoe_cue(Some(&mon),false),AtlasAoeCue::SingleTarget);
        mon.tm_aoe=false;
        assert_eq!(atlas_aoe_cue(Some(&mon),true),AtlasAoeCue::Enabled);
    }
    #[test]
    fn changing_account_aoe_checkbox_invalidates_forecast_cache() {
        let mut view=HuntView::default();
        view.active_pokemon=Some(native_hunts::PokemonInfo {
            id:99,level:100,..Default::default()
        });
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        let single=cache.active_signature.clone();
        cache.refresh(0,&view,None,true,true);
        assert_ne!(cache.active_signature,single);
        let enabled=cache.active_signature.clone();
        cache.refresh(0,&view,None,true,false);
        assert_eq!(cache.active_signature,single);
        assert_ne!(enabled,single);
    }
    #[test]
    fn explorer_speed_sort_uses_simulated_throughput_for_all_hunts() {
        let mut view=HuntView::default();
        view.hunts=Arc::new(vec![
            native_hunts::HuntEntry{
                slug:"aoe".into(),name:"AoE".into(),level:100,
                species:vec![native_hunts::HuntSpecies{id:1,weight:16.0}],
                ..Default::default()
            },
            native_hunts::HuntEntry{
                slug:"single".into(),name:"Single".into(),level:120,
                species:vec![native_hunts::HuntSpecies{id:2,weight:16.0}],
                ..Default::default()
            },
        ]);
        let mut guide=guide_fixture();
        let second_species=guide.species[&1].clone();
        guide.species.insert(2,second_species);
        let mut estimates=EstimateCache::default();
        estimates.values.insert("aoe".into(),Some(simulated_forecast(300.0,true)));
        estimates.values.insert("single".into(),Some(simulated_forecast(450.0,false)));
        let mut filters=AtlasFilters{sort:AtlasSort::Kills,..Default::default()};
        let rows=atlas_species_rows(&view,Some(&guide),&estimates,&filters,
            &AtlasScenario::default());
        assert_eq!(rows.len(),2);
        assert_eq!(rows[0].id,2,"higher simulated kills/h wins across all hunts");
        filters.sort=AtlasSort::Gold;
        view.measurements.valid_gross_gold=true;
        view.measurements.gold_gross_h=100_000.0;
        view.hunt_slug=Some("aoe".into());
        let rows=atlas_species_rows(&view,Some(&guide),&estimates,&filters,
            &AtlasScenario::default());
        assert_eq!(rows[0].best,Some(2250.0),
            "all hunts use simulated gold and the faster hunt stays first");
    }
    #[test]
    fn gold_ranking_never_compares_net_wallet_spend_to_gross_rewards(){
        let mut view=HuntView::default();
        view.hunt_slug=Some("a".into());
        view.hunts=Arc::new(vec![native_hunts::HuntEntry{
            slug:"a".into(),name:"A".into(),level:100,..Default::default()
        }]);
        view.measurements.valid_gold=true;
        view.measurements.gold_net_h=-50_000.0;
        let mut cache=EstimateCache::default();
        cache.refresh(0,&view,None,true,false);
        let rows=ranked_hunts(&view,None,HuntRanking::Gold,true,"","","",&cache);
        assert_eq!(rows.len(),1);
        assert!(rows[0].value.is_none());
        assert_eq!(rows[0].source,RateSource::Unavailable);
    }

