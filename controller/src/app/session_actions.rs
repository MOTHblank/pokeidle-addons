//! Account start, stop, browser sign-in and monitor recovery actions.
//! NativeSession and monitor handles remain owned by ControllerApp.
use super::*;

impl ControllerApp {
    pub(super) fn start_native_handoff(&mut self, profile: GameProfile) {
        let index = profile.index();
        if self.native_handoff_busy[index] { return; }
        let Some(monitor) = self.games[index].monitor.clone() else {
            self.set_status("Open Firefox and sign in to PokéIdle first", true);
            return;
        };
        self.native_handoff_busy[index] = true;
        self.native_handoff_last_attempt[index] = Some(Instant::now());
        logging::info(&format!(
            "{} capturing native game + Twitch IRC session via Firefox BiDi",
            self.account_name(profile)));
        self.set_status(format!("{}: signing in to PokéIdle and Twitch",
            self.account_name(profile)), false);
        let tx = self.native_handoff_tx.clone();
        let generation_state=Arc::clone(&self.auth_generations[index]);
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
            self.native_handoff_busy[index] = false;
            self.set_status(format!("Could not finish sign-in: {error}"), true);
        }
    }

    pub(super) fn launch_one(&mut self, profile: GameProfile) {
        self.launch_one_mode(profile, true);
    }

    pub(super) fn launch_one_mode(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        if self.games[index].is_running() || self.cached_native_busy[index] ||
            self.native_handoff_busy[index] {
            return;
        }
        // A new launch must not accept completion events from the old launch.
        self.auth_generations[index].fetch_add(1,Ordering::AcqRel);
        self.games[index].headless = headless;
        self.games[index].browser_only = false;
        if self.require_browser_login[index] {
            self.launch_firefox_bootstrap(profile, false);
            return;
        }
        self.cached_native_busy[index] = true;
        let tx = self.cached_native_tx.clone();
        let generation_state=Arc::clone(&self.auth_generations[index]);
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
                // A cache preflight panic must never strand the UI in
                // "starting" state. Report it, then authenticate via Firefox.
                let result = std::panic::catch_unwind(
                    std::panic::AssertUnwindSafe(|| {
                        crate::native_tls::ensure()?;
                        detected.and_then(|_| {
                            if existing_firefox { return Ok(None); }
                            crate::native_auth::load_for_auth(
                                profile,&generation_state,generation)
                                .and_then(|saved| saved.map(|bootstrap| {
                                    // Never project a cached, unverified connection as
                                    // started. A failed welcome needs a fresh Firefox
                                    // authentication, not infinite background retries.
                                    NativeSession::start_verified(profile, bootstrap)
                                }).transpose())
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
                    index, generation, existing_firefox, result
                });
            });
        if let Err(error) = started {
            self.cached_native_busy[index] = false;
            self.require_browser_login[index] = true;
            self.set_status(format!("Saved login could not be used: {error}"), true);
            self.launch_firefox_bootstrap(profile, false);
        }
    }

    /// Authentication-only browser fallback. Never call as the ordinary
    /// Launch path while reusable native game + Twitch credentials exist.
    pub(super) fn launch_firefox_bootstrap(&mut self, profile: GameProfile, headless: bool) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        if self.games[index].is_running() { return; }

        let config = match crate::config::Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", self.account_name(profile), error), true);
                return;
            }
        };

        // An existing Firefox may own this profile even if the controller was
        // restarted or its Child handle was lost. Never spawn over that session.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                // A previous crash may have left an authenticated, controlled
                // Moth Firefox profile alive. Attach to its existing BiDi
                // endpoint and complete native handoff; never spawn another
                // Firefox on the same profile or kill it before Rust is ready.
                self.games[index].orphaned_pid = Some(pid);
                if self.games[index].monitor.is_none() {
                    self.games[index].monitor = Some(MonitorHandle::start(
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

        self.games[index].child = None;
        self.games[index].monitor = None;
        self.games[index].orphaned_pid = None;
        self.games[index].headless = headless;

        logging::info(&format!(
            "launching {} with Firefox profile {} on BiDi port {}",
            self.account_name(profile),
            config.profile_dir.display(),
            config.remote_debug_port
        ));

        match firefox::launch(&config, headless) {
            Ok((child, monitor)) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = Some(monitor);
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
        if self.native_verifying[index] || self.games[index].handoff_closing {
            self.set_status(format!(
                "{account_name}: Sign-in is still finishing. Open Firefox after it completes."
            ), true);
            return;
        }

        if self.games[index].browser_only && !self.games[index].headless &&
            self.games[index].is_running() {
            self.set_status(format!("{account_name}: Firefox is already open."), false);
            return;
        }

        // Stop native game/IRC or an authentication-only Firefox first. In
        // particular, don't run native and Firefox on the same game account.
        if !self.stop_one(profile) { return; }
        self.games[index].browser_only = true;
        self.launch_firefox_bootstrap(profile, false);
    }

    pub(super) fn repair_monitor(&mut self, profile: GameProfile) {
        // An orphaned Firefox BiDi session cannot be reclaimed by a new
        // session.new. Only an explicit user action restarts this profile;
        // never interrupt a running game automatically on telemetry failure.
        let index = Self::game_index(profile);
        let headless = self.games[index].headless;
        let browser_only = self.games[index].browser_only;
        logging::warn(&format!(
            "{} repairing an exhausted BiDi session by user-requested Firefox restart",
            self.account_name(profile)
        ));
        if self.stop_one(profile) {
            if browser_only {
                // A manual Firefox launch must remain a manual Firefox launch
                // even after an explicit monitor repair/restart.
                self.games[index].browser_only = true;
                self.launch_firefox_bootstrap(profile, false);
            } else {
                self.launch_one_mode(profile, headless);
            }
        }
    }

    pub(super) fn stop_one(&mut self, profile: GameProfile) -> bool {
        let index = Self::game_index(profile);
        // A generation transition cannot overtake an in-progress vault write.
        self.invalidate_capture_session(index);
        // A stopped account must not keep an independent Kick worker alive.
        self.stop_kick_account(index);
        let account_name = self.account_name(profile);
        if self.games[index].handoff_closing {
            // The dedicated shutdown worker already owns and is closing the
            // Firefox process. Do not start native mode after explicit Stop.
            self.native_close_cancelled[index] = true;
            self.set_status(format!(
                "{}: stopping sign-in", account_name), false);
            return true;
        }
        // Cancel in-flight cache lookups and browser handoffs. Results from
        // canceled workers are ignored even if a Stop is pressed mid-login.
        self.cached_native_busy[index] = false;
        self.native_handoff_busy[index] = false;
        self.native_verifying[index] = false;
        let slot = &mut self.games[index];
        if let Some(native) = slot.native.take() {
            native.stop();
            // A failed browser cleanup can leave an authenticated native game
            // plus an owned Firefox launcher. Stop must close BOTH owners.
            if slot.child.is_none() && slot.orphaned_pid.is_none() {
                if let Some(monitor) = slot.monitor.take() { monitor.stop(); }
                self.set_status(format!("{}: PokéIdle and Twitch stopped.", account_name), false);
                return true;
            }
        }

        // Stop also works after restarting the controller, before any attempt
        // to launch that profile has made the orphan process visible.
        if slot.child.is_none() && slot.orphaned_pid.is_none() {
            if let Ok(config) = Config::for_profile(profile) {
                match firefox::find_profile_process(&config) {
                    Ok(Some(pid)) => slot.orphaned_pid = Some(pid),
                    Ok(None) => {}
                    Err(error) => {
                        self.set_status(format!("{}: {}", account_name, error), true);
                        return false;
                    }
                }
            }
        }

        if let Some(mut child) = slot.child.take() {
            logging::info(&format!(
                "stopping {} Firefox PID {}", account_name, child.id()
            ));
            let closed = firefox::stop_child(&mut child, slot.monitor.as_ref())
                .and_then(|_| Config::for_profile(profile))
                .and_then(|config| firefox::stop_profile_processes(&config));
            match closed {
                Ok(()) => {
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    slot.orphaned_pid = None;
                    self.set_status(format!("{} Firefox profile completely closed.", account_name), false);
                    true
                }
                Err(error) => {
                    logging::error(&format!("{} Firefox shutdown failed: {}", account_name, error));
                    slot.child = Some(child);
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else if slot.orphaned_pid.is_some() {
            // Stop was explicitly clicked; never terminate a recovered process
            // automatically when the controller itself exits.
            let result = Config::for_profile(profile)
                .and_then(|config| firefox::stop_profile_processes(&config));
            match result {
                Ok(()) => {
                    slot.orphaned_pid = None;
                    if let Some(monitor) = slot.monitor.take() {
                        monitor.stop();
                    }
                    self.set_status(format!("{} Firefox closed.", account_name), false);
                    true
                }
                Err(error) => {
                    self.set_status(format!("{}: could not close Firefox: {}", account_name, error), true);
                    false
                }
            }
        } else {
            if let Some(monitor) = slot.monitor.take() {
                monitor.stop();
            }
            self.set_status(format!("{} is already stopped.", account_name), false);
            true
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
        let mut all_stopped = true;
        for profile in GameProfile::ALL {
            if !self.stop_one(profile) {
                all_stopped = false;
            }
        }
        if all_stopped {
            self.set_status("All accounts stopped. Firefox login windows were closed.", false);
        } else {
            self.set_status("Some Firefox instances could not be stopped; check the log.", true);
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

    /// Use the same non-automated, visible Firefox for BOTH provider logins.
    /// Never send the Twitch login URL to a monitored BiDi profile.
    pub(super) fn open_unmonitored_sign_in(&mut self, profile: GameProfile, action: ProfileAction) {
        let index = Self::game_index(profile);
        let account_name = self.account_name(profile);
        self.refresh_processes();

        // A native handoff still owns the profile until its cleanup finishes.
        // stop_one intentionally cancels it asynchronously, so do not launch
        // another Firefox against the same locked profile during that window.
        if self.native_verifying[index] || self.games[index].handoff_closing {
            self.set_status(format!(
                "{account_name}: Sign-in is still finishing. Try again after it completes."
            ), true);
            return;
        }

        // Stop the native game/Twitch sockets and any monitored Firefox so
        // authentication runs solely in an ordinary user-controlled browser.
        if !self.stop_one(profile) {
            return;
        }

        let config = match Config::for_profile(profile) {
            Ok(config) => config,
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        };

        // Preserve the existing per-account cookies, but do not share an
        // active Firefox process or debugger with either sign-in provider.
        match firefox::find_profile_process(&config) {
            Ok(Some(pid)) => {
                self.games[index].orphaned_pid = Some(pid);
                self.set_status(
                    format!("{}: Firefox already uses this profile (PID {}). Use Stop before launching.", account_name, pid),
                    true,
                );
                return;
            }
            Ok(None) => {}
            Err(error) => {
                self.set_status(format!("{}: {}", account_name, error), true);
                return;
            }
        }

        let (service, url, status_suffix) = match action {
            ProfileAction::GoogleSignIn => (
                "Google",
                config.url.as_str(),
                " opened Firefox without remote debugging for Google sign-in. After signing in, close that window and launch this account normally from the dashboard.",
            ),
            ProfileAction::Twitch => (
                "Twitch",
                accounts::TWITCH_LOGIN,
                " opened Firefox without remote debugging for Twitch sign-in. After signing in, close that window and launch this account normally from the dashboard.",
            ),
            ProfileAction::Folder => return,
        };

        match firefox::launch_unmonitored(&config, url) {
            Ok(child) => {
                let pid = child.id();
                self.games[index].child = Some(child);
                self.games[index].monitor = None;
                self.games[index].orphaned_pid = None;
                self.games[index].browser_only = true;
                self.games[index].headless = false;
                // After the user closes this window, refresh browser auth
                // before returning to native mode instead of trusting stale
                // cached Twitch or game login credentials.
                self.require_browser_login[index] = true;
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
            ProfileAction::GoogleSignIn | ProfileAction::Twitch => {
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
