//! Account start, stop, browser sign-in and monitor recovery actions.
//! NativeSession and monitor handles remain owned by ControllerApp.
use super::*;

impl ControllerApp {
    pub(super) fn start_native_handoff(&mut self, profile: GameProfile) {
        let index = profile.index();
        if self.runtimes[index].session.native_handoff_busy || self.runtimes[index].game.stopping ||
            self.runtimes[index].game.shutdown_failed { return; }
        let Some(monitor) = self.runtimes[index].game.monitor.clone() else {
            self.set_status("Open Firefox and sign in to PokéIdle first", true);
            return;
        };
        self.runtimes[index].session.native_handoff_busy = true;
        self.runtimes[index].session.native_handoff_last_attempt = Some(Instant::now());
        self.set_status(format!("{}: signing in to PokéIdle",
            self.account_name(profile)), false);
        let tx = self.native_handoff_tx.clone();
        let generation_state=Arc::clone(&self.runtimes[index].session.auth_generation);
        let generation=generation_state.load(Ordering::Acquire);
        let result = std::thread::Builder::new()
            .name(format!("moth-handoff-{}", profile.name()))
            .spawn(move || {
                let mut result = monitor.native_bootstrap(Duration::from_secs(18));
                // Firefox provides fresh authentication, not authoritative
                // native preferences. Preserve this account's saved Auto Catch
                // configuration, even after the browser login is renewed.
                // Cache and worker must receive the SAME merged bootstrap.
                let mut cache_warning = None;
                if let Ok(value) = &mut result {
                    if crate::native_background::validate_bootstrap(value).is_ok() {
                        let (merged,warning) =
                            crate::native_auth::cache_browser_handoff(
                                profile,value,&generation_state,generation);
                        *value=merged;
                        cache_warning=warning;
                    }
                }
                let _ = tx.send(NativeHandoffEvent { index, generation, result, cache_warning });
            });
        if let Err(error) = result {
            self.runtimes[index].session.native_handoff_busy = false;
            self.set_status(format!("Could not finish sign-in: {error}"), true);
        }
    }

    pub(super) fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    pub(super) fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        if self.runtimes[index].game.stopping || self.runtimes[index].game.shutdown_failed {
            self.set_status(format!(
                "{}: finish or retry Firefox Stop before starting this account",
                self.account_name(profile)), true);
            return;
        }
        if self.runtimes[index].game.is_running() || self.runtimes[index].session.cached_native_busy ||
            self.runtimes[index].session.native_handoff_busy {
            return;
        }
        // A new launch must not accept completion events from the old launch.
        self.runtimes[index].session.auth_generation.fetch_add(1,Ordering::AcqRel);
        self.runtimes[index].game.headless = headless;
        self.runtimes[index].game.browser_only = false;
        if self.runtimes[index].session.require_browser_login {
            self.launch_firefox_bootstrap(profile, false);
            return;
        }
        self.runtimes[index].session.cached_native_busy = true;
        let tx = self.cached_native_tx.clone();
        let generation_state=Arc::clone(&self.runtimes[index].session.auth_generation);
        let generation=generation_state.load(Ordering::Acquire);
        let started = std::thread::Builder::new()
            .name(format!("moth-cache-{}", profile.name()))
            .spawn(move || {
                // A managed Firefox profile from a previous crash may still
                // have an active game socket. Determine ownership BEFORE using
                // saved credentials, or the same account may connect twice.
                let detected = Config::for_profile(profile)
                    .and_then(|config| firefox::find_profile_process(&config));
                let existing_firefox = matches!(detected.as_ref(), Ok(Some(_)));
                let mut browser_required = false;
                // A cache preflight panic must never strand the UI in
                // "starting" state. A panic or transient transport failure is
                // not evidence that the player's browser login expired.
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| {
                        crate::native_tls::ensure()?;
                        detected.and_then(|_| {
                            if existing_firefox { return Ok(None); }
                            let saved = match crate::native_auth::load_for_auth(
                                profile,&generation_state,generation)
                            {
                                Ok(saved) => saved,
                                Err(error) => {
                                    browser_required =
                                        !error.starts_with("Cached login canceled:");
                                    return Err(error);
                                }
                            };
                            let Some(bootstrap) = saved else {
                                browser_required = true;
                                return Ok(None);
                            };

                            // Twitch migration is handled inside the native
                            // session from its dedicated protected vault and
                            // the Firefox cookie DB. It never determines whether
                            // PokéIdle itself needs browser authentication.
                            match NativeSession::start_verified(profile, bootstrap) {
                                Ok(native) => Ok(Some(native)),
                                Err(error) => {
                                    browser_required =
                                        crate::native_background::authentication_rejected(&error);
                                    Err(error)
                                }
                            }
                        })
                    })
                ).unwrap_or_else(|_| {
                    Err("Native cache verification panicked; see sanitized controller log".into())
                });
                let result=match result {
                    Ok(Some(native)) if generation_state.load(Ordering::Acquire)!=generation => {
                        native.stop();
                        Err("Cached login canceled: launch changed".into())
                    }
                    other=>other,
                };
                let _ = tx.send(CachedNativeEvent {
                    index, generation, existing_firefox, browser_required, result
                });
            });
        if let Err(error) = started {
            self.runtimes[index].session.cached_native_busy = false;
            // Failure to create the verification worker says nothing about
            // authentication. Keep Firefox closed and preserve the cached login.
            self.set_status(format!("Could not verify saved login: {error}"), true);
        }
    }

    /// Authentication-only browser fallback. Never call as the ordinary
    /// Launch path while reusable native game credentials exist.
    pub(super) fn launch_firefox_bootstrap(&mut self, profile: GameProfile, headless: bool) {
        self.launch_firefox_bootstrap_impl(profile, headless, true);
    }

    fn launch_firefox_bootstrap_impl(
        &mut self, profile: GameProfile, headless: bool, discover_existing: bool
    ) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.runtimes[index].game.is_running() || self.runtimes[index].game.stopping { return; }

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
                return;
            }
        };

        // An existing Firefox may own this profile even if the controller was
        // restarted or its Child handle was lost. Never spawn over that session.
        if discover_existing {
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                // A previous crash may have left an authenticated, controlled
                // Moth Firefox profile alive. Attach to its existing BiDi
                // endpoint and complete native handoff; never spawn another
                // Firefox on the same profile or kill it before Rust is ready.
                self.runtimes[index].game.orphaned_pid = Some(pid);
                if self.runtimes[index].game.monitor.is_none() {
                    self.runtimes[index].game.monitor = Some(MonitorHandle::start(
                        config.remote_debug_port, headless));
                }
                logging::info(&format!(
                    "{} adopting existing Moth Firefox profile PID {} for native handoff",
                    account_name, pid
                ));
                self.set_status(format!(
                    "{}: using your existing Firefox login to connect (process {})",
                    account_name, pid
                ), false);
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }
        }

        self.runtimes[index].game.child = None;
        self.runtimes[index].game.monitor = None;
        self.runtimes[index].game.orphaned_pid = None;
        self.runtimes[index].game.headless = headless;

        logging::info(&format!(
            "launching {} with Firefox profile {} on BiDi port {}",
            self.account_name(profile),
            config.profile_dir.display(),
            config.remote_debug_port
        ));

        match firefox::launch(&config, headless) {
            Ok((child, monitor)) => {
                let pid = child.id();
                self.runtimes[index].game.child = Some(child);
                self.runtimes[index].game.monitor = Some(monitor);
                self.set_status(
                    format!(
                        "{}: Firefox started in {} for sign-in",
                        self.account_name(profile),
                        if headless { "the background" } else { "a window" }
                    ),
                    false,
                );
                logging::info(&format!(
                    "{} Firefox spawned with PID {}",
                    self.account_name(profile),
                    pid
                ));
            }
            Err(error) => {
                logging::error(&format!("{} launch failed: {}", self.account_name(profile), error));
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
            }
        }
    }

    /// Explicit interactive browser mode, separate from the temporary native
    /// authentication bootstrap. Do not open two game sockets for one account.
    pub(super) fn open_in_firefox(&mut self, profile: GameProfile) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);

        // A handoff worker might already be closing the Firefox game tab or
        // shutting down its process. Never race it with a new browser session.
        if self.runtimes[index].session.native_verifying || self.runtimes[index].game.handoff_closing ||
            self.runtimes[index].game.stopping {
            self.set_status(format!(
                "{account_name}: Wait until this account finishes stopping before opening Firefox."
            ), true);
            return;
        }

        if self.runtimes[index].game.browser_only && !self.runtimes[index].game.headless &&
            self.runtimes[index].game.is_running() {
            self.set_status(format!("{account_name}: Firefox is already open."), false);
            return;
        }

        // Do not start a second profile owner until shutdown is fully confirmed.
        self.request_stop_then(profile, AfterStop::Firefox);
    }

    pub(super) fn repair_monitor(&mut self, profile: GameProfile) {
        // An orphaned Firefox BiDi session cannot be reclaimed by a new
        // session.new. Only an explicit user action restarts this profile;
        // never interrupt a running game automatically on telemetry failure.
        let index = Self::game_index(profile);
        let headless = self.runtimes[index].game.headless;
        let browser_only = self.runtimes[index].game.browser_only;
        logging::warn(&format!(
            "{} repairing an exhausted BiDi session by user-requested Firefox restart",
            self.account_name(profile)
        ));
        if self.runtimes[index].game.stopping {
            self.set_status(format!(
                "{}: already stopping; retry Firefox repair when it completes",
                self.account_name(profile)), true);
            return;
        }
        self.request_stop_then(profile, if browser_only {
            AfterStop::Firefox
        } else {
            AfterStop::Native { headless }
        });
    }

    /// A restart is a continuation of the *same explicit action*, not an
    /// independent Start. It may run only after the previous owner is gone.
    fn request_stop_then(&mut self, profile: GameProfile, action: AfterStop) {
        let index = profile.index();
        if self.runtimes[index].game.stopping {
            self.set_status(format!(
                "{}: Firefox is still stopping; please try again afterward",
                self.account_name(profile)), true);
            return;
        }
        if self.stop_one(profile) {
            self.runtimes[index].session.after_stop = Some(action);
        }
    }

    pub(super) fn run_after_stop(&mut self, profile: GameProfile, action: AfterStop) {
        let index = profile.index();
        // A late completion must not take ownership over a newer Stop or launch.
        if self.runtimes[index].game.stopping || self.runtimes[index].game.is_running() {
            return;
        }
        match action {
            AfterStop::Native { headless } => self.launch_one_mode(profile, headless),
            AfterStop::Firefox => {
                self.runtimes[index].game.browser_only = true;
                // Stop worker just verified this dedicated profile is free;
                // do not repeat blocking OS process enumeration on egui.
                self.launch_firefox_bootstrap_impl(profile, false, false);
            }
            AfterStop::Unmonitored(action) =>
                self.launch_unmonitored_sign_in(profile, action),
        }
    }

    /// Stop is idempotent while a shutdown worker owns the process. Never
    /// perform WMI/process discovery, BiDi close, or process waits on egui.
    /// Returns whether shutdown was accepted (not whether it has finished).
    pub(super) fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        // An explicit Stop must not later look like an unexpected outage.
        self.last_native_phase[index] = None;
        self.disconnection_alerts[index] = alerts::DisconnectAlertState::default();
        // An explicit Stop cancels any previously requested restart.
        self.runtimes[index].session.after_stop = None;
        if self.runtimes[index].game.stopping {
            self.set_status(format!("{}: stopping", self.account_name(profile)), false);
            return true;
        }
        self.invalidate_capture_session(index);
        // An explicit retry is the only path that can release a failed Stop.
        // Keep Start blocked while the new cleanup worker is active.
        self.runtimes[index].game.shutdown_failed = false;
        self.stop_kick_account(index);
        self.runtimes[index].session.cached_native_busy = false;
        self.runtimes[index].session.native_handoff_busy = false;
        self.runtimes[index].session.native_verifying = false;
        let account_name = self.account_name(profile);

        if self.runtimes[index].game.handoff_closing {
            // The authenticated handoff's existing worker owns Firefox. The
            // same completion path will settle this explicit Stop.
            self.runtimes[index].game.stopping = true;
            self.runtimes[index].session.native_close_cancelled = true;
            self.set_status(format!("{}: stopping sign-in", account_name), false);
            return true;
        }

        if let Some(native) = self.runtimes[index].game.native.take() {
            native.stop();
        }

        let ownership = std::sync::Arc::new(std::sync::Mutex::new(Some(
            FirefoxStopOwnership {
                child: self.runtimes[index].game.child.take(),
                orphaned_pid: self.runtimes[index].game.orphaned_pid.take(),
                monitor: self.runtimes[index].game.monitor.take(),
            }
        )));
        self.runtimes[index].game.stopping = true;
        let tx = self.firefox_stopped_tx.clone();
        let generation = self.runtimes[index].session.auth_generation.load(Ordering::Acquire);
        let worker_ownership = Arc::clone(&ownership);
        let started = std::thread::Builder::new()
            .name(format!("moth-stop-{}", profile.name()))
            .spawn(move || {
                let mut ownership = worker_ownership.lock()
                    .expect("Firefox shutdown ownership lock poisoned")
                    .take().expect("Firefox shutdown must have one owner");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(child) = ownership.child.as_mut() {
                        firefox::stop_child(child, ownership.monitor.as_ref())?;
                    }
                    // Include a discovered orphan from a previous controller
                    // crash, even when there was no local Child handle.
                    Config::for_profile(profile)
                        .and_then(|config| firefox::stop_profile_processes(&config))?;
                    if let Some(monitor) = ownership.monitor.as_ref() {
                        monitor.stop();
                    }
                    Ok::<(), String>(())
                })).unwrap_or_else(|_| {
                    Err("Firefox shutdown worker panicked".into())
                });
                let _ = tx.send(FirefoxStoppedEvent {
                    index, generation, ownership, result
                });
            });
        match started {
            Ok(worker) => {
                track_firefox_shutdown(worker);
                self.set_status(format!("{}: stopping in the background", account_name), false);
                true
            }
            Err(error) => {
                // A failed spawn must restore the exact process ownership rather
                // than drop its Child and allow an overlapping replacement.
                let mut ownership = ownership.lock().expect("shutdown ownership available")
                    .take().expect("worker never acquired ownership");
                self.runtimes[index].game.child = ownership.child.take();
                self.runtimes[index].game.orphaned_pid = ownership.orphaned_pid.take();
                self.runtimes[index].game.monitor = ownership.monitor.take();
                self.runtimes[index].game.stopping = false;
                self.runtimes[index].game.shutdown_failed = true;
                self.set_status(format!(
                    "{}: could not schedule Firefox shutdown: {}", account_name, error
                ), true);
                false
            }
        }
    }

    pub(super) fn launch_enabled(&mut self) {
        let enabled: Vec<GameProfile> = GameProfile::ALL.into_iter()
            .filter(|profile| self.account_enabled(*profile)).collect();
        let disabled: Vec<&str> = GameProfile::ALL.into_iter()
            .filter(|profile| !self.account_enabled(*profile))
            .map(GameProfile::label).collect();
        logging::info(&format!(
            "Launch enabled requested for {} accounts; disabled profiles skipped: {}",
            enabled.len(),
            if disabled.is_empty() { "none".to_string() } else { disabled.join(", ") }
        ));
        if enabled.is_empty() {
            self.set_status(
                "No accounts enabled. Enable a profile in Account Manager before launching.",
                true,
            );
            return;
        }
        for profile in enabled {
            self.launch_one(profile);
        }
        // Keep any launch error visible rather than overwriting it with a
        // success-shaped summary when a profile is already in use.
        if !self.status_error {
            self.set_status("Starting enabled accounts. Firefox will open only if sign-in is needed.", false);
        }
    }

    pub(super) fn stop_all(&mut self) {
        // All four workers are queued independently; no per-account process
        // wait can delay starting shutdown for the remaining accounts.
        let mut accepted = true;
        for profile in GameProfile::ALL {
            accepted &= self.stop_one(profile);
        }
        if !accepted {
            self.set_status("Some Firefox shutdown workers could not start. Retry Stop.", true);
        } else if self.runtimes.iter().map(|runtime| &runtime.game).any(|slot| slot.stopping) {
            self.set_status("Stopping all accounts in the background.", false);
        }
    }

    pub(super) fn open_logs(&mut self) {
        let path = match crate::config::data_root() {
            Ok(root) => root.join("moth-controller.log"),
            Err(error) => {
                self.set_status(error, true);
                return;
            }
        };
        if let Some(parent) = path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.set_status(format!("Could not create log folder: {error}"), true);
                return;
            }
        }
        if !path.exists() {
            if let Err(error) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                self.set_status(format!("Could not create log: {error}"), true);
                return;
            }
        }
        let opener = if cfg!(windows) { "notepad.exe" } else { "xdg-open" };
        match std::process::Command::new(opener).arg(&path).spawn() {
            Ok(_) => self.set_status(format!("Opened controller log · {}", path.display()), false),
            Err(error) => self.set_status(format!("Could not open log: {error}"), true),
        }
    }

    /// Open a non-automated, visible Firefox for an explicit PokéIdle/Google sign-in.
    pub(super) fn open_unmonitored_sign_in(&mut self, profile: GameProfile, action: ProfileAction) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        self.refresh_processes();

        // A native handoff still owns the profile until its cleanup finishes.
        // stop_one intentionally cancels it asynchronously, so do not launch
        // another Firefox against the same locked profile during that window.
        if self.runtimes[index].session.native_verifying || self.runtimes[index].game.handoff_closing ||
            self.runtimes[index].game.stopping {
            self.set_status(format!(
                "{account_name}: Wait until this account finishes stopping before signing in."
            ), true);
            return;
        }

        // The follow-up is tied to the explicit Stop generation and only runs
        // after the worker has verified the profile is fully released.
        self.request_stop_then(profile, AfterStop::Unmonitored(action));
    }

    fn launch_unmonitored_sign_in(&mut self, profile: GameProfile, action: ProfileAction) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.runtimes[index].game.stopping || self.runtimes[index].game.is_running() { return; }
        let config = match Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        };

        // The dedicated Stop worker already confirmed this profile has no
        // living Firefox owner. A second WMI scan here would stall the UI.
        // Firefox itself still enforces the profile lock if another program
        // opens the same profile between cleanup and this new launch.
        let (service, url, status_suffix) = match action {
            ProfileAction::GoogleSignIn => (
                "Google",
                config.url.as_str(),
                " opened Firefox without remote debugging for Google sign-in. After signing in, close that window and launch this account normally from the dashboard.",
            ),
            ProfileAction::Folder => return,
        };

        let launched = match action {
            ProfileAction::GoogleSignIn => firefox::launch_unmonitored(&config, url),
            ProfileAction::Folder => return,
        };

        match launched {
            Ok(child) => {
                let pid = child.id();
                self.runtimes[index].game.child = Some(child);
                self.runtimes[index].game.monitor = None;
                self.runtimes[index].game.orphaned_pid = None;
                self.runtimes[index].game.browser_only = true;
                self.runtimes[index].game.headless = false;
                // Google/PokéIdle sign-in may change the game credentials.
                self.runtimes[index].session.require_browser_login = true;
                logging::info(&format!(
                    "{} Firefox spawned for {} sign-in without remote debugging (PID {})",
                    account_name, service, pid
                ));
                self.set_status(format!("{account_name}{status_suffix}"), false);
            }
            Err(error) => {
                logging::error(&format!(
                    "{} {} sign-in launch failed: {}",
                    account_name, service, error
                ));
                self.set_status(format!("{}: {}", account_name, error), true);
            }
        }
    }

    pub(super) fn profile_action(&mut self, profile: GameProfile, action: ProfileAction) {
        let result = match action {
            ProfileAction::GoogleSignIn => {
                self.open_unmonitored_sign_in(profile, action);
                return;
            }
            ProfileAction::Folder => accounts::open_profile_folder(profile).map(|_| "Opened profile folder".to_string()),
        };

        match result {
            Ok(message) => self.set_status(format!("{} · {}", self.account_name(profile), message), false),
            Err(error) => self.set_status(error, true),
        }
     }
}
