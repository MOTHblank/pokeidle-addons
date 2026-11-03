//! Reconcile worker completion events in the application's event pump.
//! Every asynchronous sender wakes this pump; display repaint throttling
//! never determines when a native/Firefox authentication stage completes.
use super::*;

// Reject prior-session and superseded request completions before touching
// their busy flags or editor/draft state.
fn capture_save_is_current(
    event:&CaptureSavedEvent, generation:u64, request_id:u64
)->bool {
    event.generation==generation && event.request_id==request_id
}

fn capture_fields_equal(a:&CatchSettings,b:&CatchSettings)->bool {
    a.enabled==b.enabled && a.priorities==b.priorities &&
        a.interval_ms==b.interval_ms && a.reserves==b.reserves
}

// Restock acknowledgements update only the restock fields of an open Capture
// editor. This preserves unsaved capture priorities, reserves and timing.
fn reconcile_capture_draft(
    draft: &mut Option<CatchSettings>,
    saved: &CatchSettings,
    origin: CaptureSaveOrigin,
) {
    match origin {
        CaptureSaveOrigin::Editor => {
            // Newer local edits must survive an older save acknowledgement.
            // Restock fields can change separately during an editor save.
            if draft.as_ref().is_some_and(|current|capture_fields_equal(current,saved)) {
                *draft=None;
            }
        },
        CaptureSaveOrigin::Toggle => {
            if let Some(draft)=draft.as_mut() { draft.enabled=saved.enabled; }
        }
        CaptureSaveOrigin::Restock => {
            if let Some(draft)=draft.as_mut() {
                automation_restock::copy_restock_fields(draft,saved);
            }
        }
    }
}

impl ControllerApp {
    /// Reconcile the only shutdown owner for this account. A failed cleanup
    /// returns its exact Child/monitor handles for a subsequent explicit Stop.
    fn finish_firefox_stop(
        &mut self, index: usize, generation: u64,
        ownership: FirefoxStopOwnership, result: Result<(), String>,
    ) {
        if !self.games[index].stopping {
            logging::error("Firefox shutdown completion without Stopping ownership");
            return;
        }
        let profile = self.games[index].profile;
        let current = self.auth_generations[index].load(Ordering::Acquire);
        self.games[index].stopping = false;
        if generation != current {
            // A newer session boundary may have invalidated an earlier queued
            // restart, but this worker still owns the browser being stopped.
            self.after_stop[index] = None;
        }
        match result {
            Ok(()) => {
                // Process and monitor were already closed in the worker.
                self.games[index].child = None;
                self.games[index].orphaned_pid = None;
                self.games[index].monitor = None;
                self.games[index].shutdown_failed = false;
                if let Some(action) = self.after_stop[index].take() {
                    self.run_after_stop(profile, action);
                } else {
                    self.set_status(format!(
                        "{}: stopped", self.account_name(profile)
                    ), false);
                }
            }
            Err(error) => {
                self.after_stop[index] = None;
                self.games[index].shutdown_failed = true;
                self.games[index].child = ownership.child;
                self.games[index].orphaned_pid = ownership.orphaned_pid;
                self.games[index].monitor = ownership.monitor;
                logging::error(&format!(
                    "{} Firefox shutdown failed: {}", self.account_name(profile), error
                ));
                self.set_status(format!(
                    "{}: could not close Firefox: {}", self.account_name(profile), error
                ), true);
            }
        }
    }

    // Only this central event pump starts writes, not whichever page is visible.
    // A new choice made during an earlier save is automatically processed on
    // its acknowledgement, even if the picker has since been closed.
    fn flush_queued_restock_saves(&mut self) {
        for index in 0..self.games.len() {
            if !self.restock_dirty[index] || self.capture_save_busy[index] ||
                !self.restock_retries[index].ready(Instant::now()) { continue; }
            let Some(native)=self.games[index].native.as_ref() else {continue};
            let current=native.capture_settings();
            let Some(staged)=self.restock_drafts[index].as_ref() else {
                self.restock_dirty[index]=false;
                self.restock_retries[index]=RestockSaveRetry::default();
                self.restock_save_error[index]=None;
                continue;
            };
            let next=automation_restock::prepare_restock_save(&current,staged);
            match next {
                Ok(None) => {
                    if let Some(draft)=self.restock_drafts[index].as_mut() {
                        automation_restock::copy_restock_fields(draft,&current);
                    }
                    self.restock_dirty[index]=false;
                    self.restock_retries[index]=RestockSaveRetry::default();
                    self.restock_save_error[index]=None;
                }
                Err(error) => {
                    // Preserve the unsaved draft and surface a validation error
                    // without repeatedly spawning a doomed persistence worker.
                    self.restock_save_error[index]=Some(error);
                    self.restock_retries[index].blocked=true;
                }
                Ok(Some(next)) => {
                    self.restock_save_error[index]=None;
                    self.capture_save_busy[index]=true;
                    self.capture_save_ids[index]=self.capture_save_ids[index].wrapping_add(1);
                    let request_id=self.capture_save_ids[index];
                    let profile=self.games[index].profile;
                    let tx=self.capture_save_tx.clone();
                    let generation_state=Arc::clone(&self.capture_generations[index]);
                    let generation=generation_state.load(Ordering::Acquire);
                    let updater=native.capture_settings_updater();
                    // Keep restock_dirty set until a successful, current
                    // acknowledgement of this exact selection arrives.
                    std::thread::spawn(move || {
                        let result=std::panic::catch_unwind(
                            std::panic::AssertUnwindSafe(||
                                crate::native_auth::persist_and_apply_capture_settings(
                                    profile,&next,&generation_state,generation,&updater)))
                            .unwrap_or_else(|_|
                                Err("Encrypted restock save panicked".into()));
                        let _=tx.send(CaptureSavedEvent {
                            index,generation,request_id,config:next,
                            origin:CaptureSaveOrigin::Restock,result
                        });
                    });
                }
            }
        }
    }

    pub(super) fn refresh_processes(&mut self) {
        while let Ok(event) = self.firefox_stopped_rx.try_recv() {
            self.finish_firefox_stop(
                event.index, event.generation, event.ownership, event.result
            );
        }
        while let Ok(event)=self.capture_save_rx.try_recv() {
            // An old completion cannot clear a newer request's busy flag or
            // overwrite drafts, even when the native session was replaced.
            if !capture_save_is_current(
                &event,
                self.capture_generations[event.index].load(Ordering::Acquire),
                self.capture_save_ids[event.index],
            ) {
                // Never resurrect a previous session's Restock draft. The
                // session boundary has already discarded its pending state.
                continue;
            }
            self.capture_save_busy[event.index]=false;
            match event.result {
                Ok(()) => {
                    // The encrypted save worker has already applied the
                    // settings to the native engine; this is UI-only feedback.
                    reconcile_capture_draft(
                        &mut self.capture_drafts[event.index],&event.config,event.origin
                    );
                    if self.restock_drafts[event.index].as_ref()
                        .is_some_and(|draft| restock_fields_equal(draft, &event.config))
                    {
                        // Only the acknowledged selection is considered saved.
                        self.restock_drafts[event.index]=Some(event.config);
                        self.restock_dirty[event.index]=false;
                        self.restock_retries[event.index]=RestockSaveRetry::default();
                        self.restock_save_error[event.index]=None;
                    } else if event.origin==CaptureSaveOrigin::Restock &&
                        self.restock_drafts[event.index].is_some() {
                        // A newer local selection must still be persisted.
                        self.restock_dirty[event.index]=true;
                        self.restock_retries[event.index]=RestockSaveRetry::default();
                        self.restock_save_error[event.index]=None;
                    }
                    if event.origin!=CaptureSaveOrigin::Restock {
                        self.capture_save_error[event.index]=None;
                    }
                    self.set_status(tr("Auto Catch settings saved"),false);
                }
                Err(error) => {
                    if event.origin==CaptureSaveOrigin::Restock &&
                        self.restock_drafts[event.index].is_some() {
                        self.restock_dirty[event.index]=true;
                        if self.restock_drafts[event.index].as_ref()
                            .is_some_and(|draft|restock_fields_equal(draft,&event.config)) {
                            // Retry transient vault failures with a bounded
                            // backoff, but not validation/auth/session errors.
                            self.restock_save_error[event.index]=Some(error.message.clone());
                            if let Some(delay)=self.restock_retries[event.index]
                                .failed(error.retryable,Instant::now()) {
                                self.lifecycle_ctx.request_repaint_after(delay);
                            }
                        } else {
                            // A newer edit superseded the failed submission.
                            self.restock_retries[event.index]=RestockSaveRetry::default();
                            self.restock_save_error[event.index]=None;
                        }
                    } else if event.origin!=CaptureSaveOrigin::Restock {
                        self.capture_save_error[event.index]=Some(error.message.clone());
                    }
                    self.set_status(format!("Could not save capture settings: {error}"),true);
                }
            }
        }
        // Dispatch any staged restock save after processing previous
        // acknowledgements, regardless of the currently selected UI page.
        self.flush_queued_restock_saves();
        while let Ok(result)=self.inventory_catalog_rx.try_recv() {
            self.inventory_catalog_busy=false;
            match result {
                Ok(catalog)=>{
                    logging::info(&format!("Native inventory catalog ready: {} items",catalog.items.len()));
                    self.inventory_catalog=Some(Arc::new(catalog));
                    self.inventory_catalog_error=None;
                }
                Err(error)=>{
                    logging::warn(&format!("Native inventory catalog unavailable: {error}"));
                    self.inventory_catalog_error=Some(error);
                }
            }
        }
        while let Ok(loaded) = self.hunt_reference_rx.try_recv() {
            self.hunt_reference_busy = false;
            match loaded {
                Ok(reference) => {
                    logging::info(&format!(
                        "Native Hunt Atlas reference ready: {} species",
                        reference.species.len()
                    ));
                    let reference=Arc::new(reference);
                    // The public Atlas guide is already in memory. Pass the
                    // same Arc to active accounts for one low-frequency
                    // current-hunt history sample, with no new downloads.
                    for index in 0..self.games.len() {
                        if let Some(native)=self.games[index].native.as_ref() {
                            native.enable_background_forecasts(
                                Arc::clone(&reference),
                                self.ui_options.atlas_aoe_enabled[index]);
                        }
                    }
                    self.hunt_reference = Some(reference);
                    self.hunt_reference_error = None;
                }
                Err(error) => {
                    logging::warn(&format!("Native Hunt Atlas reference unavailable: {error}"));
                    self.hunt_reference_error = Some(error);
                }
            }
        }
        // Only the game TAB is closed before proof. The authentication browser
        // stays alive until both native sockets have received real welcome.
        while let Ok(event) = self.native_verified_rx.try_recv() {
            let index = event.index;
            if event.generation!=self.auth_generations[index].load(Ordering::Acquire) {
                if let Ok(native)=event.result {native.stop();}
                continue;
            }
            if !self.native_verifying[index] {
                if let Ok(native)=event.result {native.stop();}
                continue;
            }
            self.native_verifying[index] = false;
            let profile = GameProfile::from_index(index).expect("known slot");
            let native = match event.result {
                Ok(native) => native,
                Err(error) => {
                    self.native_error[index] = Some(error.clone());
                    self.set_status(format!(
                        "{}: native verification failed: {error}; Firefox game restored or recovery reported",
                        self.account_name(profile)), true);
                    continue;
                }
            };
            if self.games[index].handoff_closing || self.games[index].native.is_some() {
                native.stop();
                continue;
            }
            let mut child = self.games[index].child.take();
            let orphaned_pid = self.games[index].orphaned_pid.take();
            if child.is_none() && orphaned_pid.is_none() {
                native.stop();
                self.set_status("Could not find the Firefox window used to sign in", true);
                continue;
            }
            let monitor = self.games[index].monitor.take();
            self.games[index].handoff_closing = true;
            self.native_close_cancelled[index] = false;
            let tx = self.native_closed_tx.clone();
            logging::info(&format!(
                "{} native game welcome + Twitch IRC 001 confirmed; closing Firefox",
                self.account_name(profile)));
            let worker = std::thread::spawn(move || {
                let mut result = if let Some(child) = child.as_mut() {
                    firefox::stop_child(child, monitor.as_ref())
                } else {
                    Ok(())
                };
                if result.is_ok() {
                    result = crate::config::Config::for_profile(profile)
                        .and_then(|config| firefox::stop_profile_processes(&config));
                }
                if result.is_ok() {
                    if let Some(monitor) = monitor.as_ref() { monitor.stop(); }
                }
                let _ = tx.send(NativeBrowserClosedEvent {
                    index, generation:event.generation,
                    native, cache_warning: event.cache_warning,
                    child, orphaned_pid, monitor, result
                });
            });
            track_firefox_shutdown(worker);
        }
        // Browser shutdown is asynchronous: the UI never blocks for BiDi
        // browser.close, process reaping or a force-kill fallback.
        while let Ok(event) = self.native_closed_rx.try_recv() {
            let index = event.index;
            let profile = GameProfile::from_index(index).expect("known slot");
            self.games[index].handoff_closing = false;
            let cancelled = std::mem::replace(&mut self.native_close_cancelled[index], false);
            if cancelled && self.games[index].stopping {
                // Explicit Stop takes precedence over either handoff outcome,
                // even if a future path cancels without changing generation.
                event.native.stop();
                let current=self.auth_generations[index].load(Ordering::Acquire);
                self.finish_firefox_stop(index, current, FirefoxStopOwnership {
                    child:event.child, orphaned_pid:event.orphaned_pid,
                    monitor:event.monitor
                }, event.result);
                continue;
            }
            if event.generation!=self.auth_generations[index].load(Ordering::Acquire) {
                event.native.stop();
                if self.games[index].stopping {
                    // Explicit Stop canceled this handoff. The handoff worker
                    // still owns Firefox; settle Stopping only after it finishes.
                    let current=self.auth_generations[index].load(Ordering::Acquire);
                    self.finish_firefox_stop(index, current, FirefoxStopOwnership {
                        child:event.child, orphaned_pid:event.orphaned_pid,
                        monitor:event.monitor
                    }, event.result);
                } else if event.result.is_err() {
                    // Retain failed cleanup when a different session boundary
                    // invalidated the handoff without an explicit Stop.
                    self.games[index].child=event.child;
                    self.games[index].orphaned_pid=event.orphaned_pid;
                    self.games[index].monitor=event.monitor;
                }
                continue;
            }
            match event.result {
                Ok(()) => {
                    self.native_error[index] = None;
                    // Successful Firefox close must be logged before any
                    // attempt to start native sockets, so a crash is localizable.
                    logging::info(&format!(
                        "{} Firefox exited after authenticated handoff",
                        self.account_name(profile)));
                    if cancelled {
                        event.native.stop();
                        self.set_status(format!(
                            "{}: sign-in canceled", self.account_name(profile)), false);
                        continue;
                    }
                    logging::info(&format!(
                        "{}: connecting to PokéIdle and Twitch", self.account_name(profile)));
                    self.adopt_native_session(index, event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.require_browser_login[index] = false;
                    let cache_note = event.cache_warning
                        .map(|warning| format!(" · You may need to sign in next time: {warning}"))
                        .unwrap_or_default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected. Firefox closed{cache_note}",
                        self.account_name(profile)), false);
                }
                Err(error) => {
                    // Native game is already authenticated; Firefox's game
                    // tab was closed. Retain native ownership and the remaining
                    // Firefox handle for a later explicit cleanup attempt.
                    self.adopt_native_session(index, event.native);
                    self.capture_drafts[index]=None;
                    self.restock_drafts[index]=None;
                    self.restock_dirty[index]=false;
                    self.automation_drafts[index]=AutomationDraft::default();
                    self.games[index].child = event.child;
                    self.games[index].orphaned_pid = event.orphaned_pid;
                    self.games[index].monitor = event.monitor;
                    self.set_status(format!(
                        "{}: connected, but Firefox could not close automatically: {error}",
                        self.account_name(profile)), true);
                }
            }
        }
        // Try the protected native credential vault BEFORE constructing any
        // Firefox process. OS keyring requests run off the UI thread.
        while let Ok(event) = self.cached_native_rx.try_recv() {
            if event.generation!=self.auth_generations[event.index].load(Ordering::Acquire)
                || !self.cached_native_busy[event.index] {
                if let Ok(Some(native))=event.result {native.stop();}
                continue;
            }
            self.cached_native_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            if self.games[event.index].is_running() {
                // A result that lost the race for this slot owns its socket
                // until explicitly stopped; never let it linger detached.
                if let Ok(Some(native))=event.result {native.stop();}
                continue;
            }
            if event.existing_firefox {
                // A stale managed Firefox process still owns this profile.
                // Do not create a competing native game socket from cache.
                // Adopt its BiDi session and let the two-phase handoff close it.
                self.require_browser_login[event.index] = true;
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            if self.require_browser_login[event.index] {
                self.launch_firefox_bootstrap(profile, false);
                continue;
            }
            match event.result {
                Ok(Some(session)) => {
                    self.adopt_native_session(event.index, session);
                    self.capture_drafts[event.index]=None;
                    self.automation_drafts[event.index]=AutomationDraft::default();
                    self.set_status(format!(
                        "{}: PokéIdle and Twitch connected with your saved login",
                        self.account_name(profile)), false);
                },
                Ok(None) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: please sign in to continue",
                        self.account_name(profile)), false);
                    self.launch_firefox_bootstrap(profile, false);
                }
                Err(error) => {
                    self.require_browser_login[event.index] = true;
                    self.set_status(format!(
                        "{}: saved login unavailable: {error}",
                        self.account_name(profile)), true);
                    self.launch_firefox_bootstrap(profile, false);
                }
            }
        }
        // Finish BiDi capture on the UI thread. The authenticated bootstrap
        // is never logged or written to a file.
        while let Ok(event) = self.native_handoff_rx.try_recv() {
            if event.generation!=self.auth_generations[event.index].load(Ordering::Acquire)
                || !self.native_handoff_busy[event.index] {continue;}
            self.native_handoff_busy[event.index] = false;
            let profile = GameProfile::from_index(event.index).expect("known slot");
            match event.result {
                Err(error) => {
                    let pending = error.to_ascii_lowercase();
                    let is_bootstrap = pending.contains("reloaded")
                        || pending.contains("waiting") || pending.contains("pending")
                        || pending.contains("temporary twitch")
                        || pending.contains("hook installed")
                        || pending.contains("login has not produced");
                    if is_bootstrap {
                        self.native_error[event.index] = None;
                    } else {
                        self.native_error[event.index] = Some(error.clone());
                    }
                    self.set_status(format!(
                        "{}: {}: {error}",
                        self.account_name(profile),
                        if is_bootstrap { "waiting for login" } else { "login check" }
                    ), !is_bootstrap);
                },
                Ok(bootstrap) => {
                    // Never close Firefox without a usable authenticated hello
                    // and cookies, or if the slot changed while capture ran.
                    if self.games[event.index].native.is_some()
                        || !self.games[event.index].is_running()
                        || self.games[event.index].monitor.is_none()
                    {
                        self.set_status("Connection interrupted because the Firefox session changed", true);
                        continue;
                    }
                    let validated = crate::native_background::validate_bootstrap(&bootstrap);
                    if let Err(error) = validated {
                        self.native_error[event.index] = Some(error.clone());
                        self.set_status(format!("Could not connect to the game: {error}"), true);
                        continue;
                    }
                    // First prove Rust is authenticated while Firefox's live game
                    // WebSocket still exists, matching pokeidle-manager-moth.
                    // Only then close the game tab and finally Firefox.
                    if self.games[event.index].handoff_closing ||
                        self.native_verifying[event.index] { continue; }
                    let Some(monitor) = self.games[event.index].monitor.clone() else {
                        continue;
                    };
                    self.native_verifying[event.index] = true;
                    self.native_error[event.index] = None;
                    let tx = self.native_verified_tx.clone();
                    let index = event.index;
                    let generation=event.generation;
                    let generation_state=Arc::clone(&self.auth_generations[index]);
                    let cache_warning = event.cache_warning;
                    logging::info(&format!(
                        "{}: checking PokéIdle and Twitch connections before closing Firefox",
                        self.account_name(profile)));
                    let spawned = std::thread::Builder::new()
                        .name(format!("moth-native-handoff-{}", profile.name()))
                        .spawn(move || {
                            let mut attempted_game_close = false;
                            // A successful native welcome must precede the game
                            // tab close. Closing Firefox's original socket first
                            // could invalidate the just-captured hello/token.
                            let result = std::panic::catch_unwind(
                                std::panic::AssertUnwindSafe(|| {
                                    if generation_state.load(Ordering::Acquire)!=generation {
                                        return Err("Login verification canceled: launch changed".into());
                                    }
                                    crate::native_tls::ensure()?;
                                    let native = NativeSession::start_verified(profile, bootstrap)?;
                                    if generation_state.load(Ordering::Acquire)!=generation {
                                        native.stop();
                                        return Err("Login verification canceled: launch changed".into());
                                    }
                                    attempted_game_close = true;
                                    if let Err(error)=monitor.close_game_for_handoff(Duration::from_secs(18)) {
                                        native.stop();
                                        return Err(error);
                                    }
                                    if generation_state.load(Ordering::Acquire)!=generation {
                                        native.stop();
                                        return Err("Login verification canceled: launch changed".into());
                                    }
                                    Ok(native)
                                })
                            ).unwrap_or_else(|_| {
                                Err("Login check failed unexpectedly. See the controller log.".to_owned())
                            });
                            let result = match (result, attempted_game_close) {
                                (Err(error), true) if generation_state.load(Ordering::Acquire)==generation => {
                                    match monitor.restore_game_after_failed_handoff(
                                        Duration::from_secs(20)
                                    ) {
                                        Ok(()) => Err(format!("{error}; game tab reopened")),
                                        Err(recovery) => {
                                            crate::logging::error(&format!(
                                                "Could not recover the connection: {recovery}"
                                            ));
                                            Err(format!("{error}; game tab recovery failed: {recovery}"))
                                        }
                                    }
                                }
                                (other, _) => other,
                            };
                            let _ = tx.send(NativeVerifiedEvent {
                                index, generation, result, cache_warning
                            });
                        });
                    if let Err(error) = spawned {
                        self.native_verifying[index] = false;
                        self.native_error[index] = Some(format!(
                            "Could not start the login check: {error}"
                        ));
                        self.set_status(format!(
                            "{}: could not verify login: {error}",
                            self.account_name(profile)), true);
                    }
                }
            }
        }
        let mut recover = Vec::new();
        for slot in &mut self.games {
            if slot.native.as_ref().is_some_and(|session|
                session.snapshot().phase == "NeedsLogin" ||
                session.twitch_snapshot().phase == "NeedsLogin"
            ) {
                if let Some(session) = slot.native.take() { session.stop(); }
                recover.push(slot.profile);
            }
        }
        for profile in recover {
            // Expired-login recovery is also a session boundary. Cancel any
            // old capture save before Firefox imports/preserves its settings.
            self.invalidate_capture_session(profile.index());
            self.require_browser_login[profile.index()] = true;
            self.set_status(format!(
                "{}: login expired, opening Firefox to sign in again",
                self.account_name(profile)), true);
            self.launch_firefox_bootstrap(profile, false);
        }
        let mut command_feedback = Vec::new();
        for slot in &mut self.games {
            // The shutdown worker has exclusive ownership of this profile.
            if slot.stopping { continue; }
            if let Some(monitor) = slot.monitor.as_ref() {
                while let Some(result) = monitor.take_command_feedback() {
                    command_feedback.push((
                        self.accounts[slot.profile.index()].name.clone(),
                        result,
                    ));
                }
            }
            let exited = match slot.child.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(error) => {
                        logging::warn(&format!(
                            "{} Firefox process check failed: {error}", slot.profile.label()
                        ));
                        false
                    }
                },
                None => false,
            };
            if exited {
                let account_name = self.accounts[slot.profile.index()].name.clone();
                logging::info(&format!("{} Firefox child process exited", account_name));
                slot.child = None;

                // Firefox can outlive the original launcher handle. Keep the
                // existing BiDi monitor for that same profile rather than
                // permanently discarding telemetry from a still-open game.
                match Config::for_profile(slot.profile)
                    .and_then(|config| firefox::find_profile_process(&config))
                {
                    Ok(Some(pid)) => {
                        slot.orphaned_pid = Some(pid);
                        logging::warn(&format!(
                            "{} Firefox launcher exited; profile PID {} is still running, preserving monitor",
                            account_name, pid
                        ));
                    }
                    Ok(None) => {
                        slot.orphaned_pid = None;
                        if let Some(monitor) = slot.monitor.take() {
                            monitor.stop();
                        }
                    }
                    Err(error) => logging::warn(&format!(
                        "{} profile recovery failed (keeping monitor until ownership is known): {}",
                        account_name, error
                    )),
                }
            }
            if let Some(pid) = slot.orphaned_pid {
                if !firefox::process_alive(pid) {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                }
            }
        }
        for (account_name, result) in command_feedback {
            self.set_status(
                format!("{} · {} #{}: {}", account_name, tr("Tab"), result.id, result.message),
                !result.ok,
            );
        }
        // Temporary Firefox authentication is auto-handed off to native.
        // An explicitly opened interactive Firefox must stay open instead.
        for profile in GameProfile::ALL {
            let index = profile.index();
            if !self.accounts[index].enabled || self.games[index].browser_only ||
               self.native_handoff_busy[index] || self.games[index].native.is_some() ||
               self.native_verifying[index] ||
               self.games[index].handoff_closing || self.games[index].stopping ||
               self.games[index].monitor.is_none() {
                continue;
            }
            let health = self.games[index].health();
            // BiDi's captured game hello, not a DOM telemetry sample, is the
            // authentication signal. The reference manager also observes the
            // hello while the login page is still transitioning.
            if health.state != "Running" {
                continue;
            }
            if self.native_handoff_last_attempt[index]
                .is_some_and(|at| at.elapsed() < Duration::from_secs(8)) { continue; }
            self.start_native_handoff(profile);
        }
    }

}

#[cfg(test)]
mod capture_save_reconciliation_tests {
    use super::*;

    #[test]
    fn failed_restock_retries_are_bounded_and_validation_never_spins() {
        let now=Instant::now();
        let mut retry=RestockSaveRetry::default();
        for attempt in 1..5 {
            assert_eq!(retry.failed(true,now),
                Some(Duration::from_secs(1u64<<(attempt-1))));
            assert!(!retry.ready(now));
            assert!(!retry.blocked);
        }
        assert_eq!(retry.failed(true,now),None);
        assert!(retry.blocked);
        assert!(!retry.ready(now+Duration::from_secs(60)));
        // A new edit creates a fresh retry budget.
        retry=RestockSaveRetry::default();
        assert!(retry.ready(now));
        assert_eq!(retry.failed(false,now),None);
        assert!(retry.blocked);
        // Transient Windows DPAPI failures are categorized by the storage
        // layer, even when their message never mentions DPAPI.
        retry=RestockSaveRetry::default();
        assert_eq!(retry.failed(true,now),Some(Duration::from_secs(1)));
    }
    #[test]
    fn old_session_or_request_acknowledgement_cannot_clear_current_save() {
        let event=CaptureSavedEvent {
            index:2,generation:7,request_id:3,config:CatchSettings::default(),
            origin:CaptureSaveOrigin::Toggle,result:Ok(())
        };
        assert!(capture_save_is_current(&event,7,3));
        assert!(!capture_save_is_current(&event,8,3));
        assert!(!capture_save_is_current(&event,7,4));
    }

    #[test]
    fn restock_save_keeps_unsaved_capture_edits_and_updates_only_restock() {
        let mut draft=CatchSettings::default();
        draft.enabled=true;
        draft.interval_ms=750;
        draft.priorities=vec!["beast".into()];
        let mut saved=CatchSettings::default();
        saved.restock_enabled=true;
        saved.ball_id=5;
        saved.potion_id=203;
        saved.revive_id=206;
        let mut editor=Some(draft);
        reconcile_capture_draft(&mut editor,&saved,CaptureSaveOrigin::Restock);
        let result=editor.unwrap();
        assert!(result.enabled);
        assert_eq!(result.interval_ms,750);
        assert_eq!(result.priorities,vec!["beast"]);
        assert_eq!(result.ball_id,5);
        assert_eq!(result.potion_id,203);
        assert_eq!(result.revive_id,206);
        assert!(result.restock_enabled);
    }

    #[test]
    fn editor_save_does_not_discard_newer_unsaved_capture_changes() {
        let mut saved=CatchSettings::default();
        saved.interval_ms=500;
        let mut newer=saved.clone();
        newer.interval_ms=700;
        let mut editor=Some(newer.clone());
        reconcile_capture_draft(&mut editor,&saved,CaptureSaveOrigin::Editor);
        assert_eq!(editor,Some(newer.clone()));
        // Restock reconciliation must also preserve the newer throwing fields.
        reconcile_capture_draft(&mut editor,&saved,CaptureSaveOrigin::Restock);
        assert_eq!(editor.as_ref().unwrap().interval_ms,700);
        reconcile_capture_draft(&mut editor,&newer,CaptureSaveOrigin::Editor);
        assert!(editor.is_none());
    }

    #[test]
    fn switch_save_does_not_discard_editor_and_editor_acknowledgement_does() {
        let mut editor=Some(CatchSettings::default());
        let mut saved=CatchSettings::default();
        saved.enabled=true;
        reconcile_capture_draft(&mut editor,&saved,CaptureSaveOrigin::Toggle);
        assert!(editor.as_ref().unwrap().enabled);
        reconcile_capture_draft(&mut editor,&saved,CaptureSaveOrigin::Editor);
        assert!(editor.is_none());
    }
}
