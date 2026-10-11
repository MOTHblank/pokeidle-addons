// ==UserScript==
// @name         PokéIdle Performance+
// @namespace    moth.pokeidle
// @version      5.4.0
// @description  Performance+ pauses map rendering, hides visual noise and chat, and keeps capture controls visible.
// @match        https://pokeidle.io/*
// @match        https://www.pokeidle.io/*
// @grant        unsafeWindow
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/performance.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/performance.user.js
// @run-at       document-start
// ==/UserScript==

(() => {
    'use strict';

    const STORAGE_KEY = 'moth-pokeidle-performance-v7';
    const PREVIOUS_KEY = 'moth-pokeidle-performance-v6';
    const LEGACY_KEY = 'moth-pokeidle-performance-v5';
    const NATIVE_OPT_OWNER_KEY = 'moth-pokeidle-performance-native-opt-owner';
    const NATIVE_OPT_BASE_KEY = 'moth-pokeidle-performance-native-opt-base';
    const TRIGGER_ID = 'moth-performance-trigger';
    const TELEMETRY_ID = 'moth-performance-telemetry';
    const STYLE_ID = 'moth-performance-style';

    const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
    const q = (selector, root = document) => root.querySelector(selector);

    const nativeSetInterval = page.setInterval.bind(page);
    const nativeClearInterval = page.clearInterval.bind(page);

    /*
     * Performance suspends campo.mjs map rendering entirely. This avoids
     * drawing a hidden scene and avoids native Economy mode, which disables
     * delivery of the defeated-Pokémon capture prompt.
     */
    const PROFILES = {
        performance: {
            label: 'Performance',
            adaptive: false,
            scene: 'paused',
            idleScene: 'paused',
            backgroundScene: 'paused',
            idleSeconds: 0,
            ui: 'performance',
            idleUi: 'performance',
            effects: 'minimal',
            nativeOptimized: true
        }
    };

    const DEFAULTS = {
        enabled: true
    };

    function loadSettings() {
        let saved = {};
        let currentSettings = null;
        try {
            currentSettings = localStorage.getItem(STORAGE_KEY);
        } catch {}
        const isUpgrade = currentSettings === null;

        try {
            saved = JSON.parse(
                currentSettings ||
                localStorage.getItem(PREVIOUS_KEY) ||
                localStorage.getItem(LEGACY_KEY) ||
                '{}'
            );
        } catch {}

        // Older releases could persist OFF or the removed Full profile.
        // Start enabled once on upgrade, then preserve the user's new toggle choice.
        const loaded = {
            ...DEFAULTS,
            ...saved,
            enabled: isUpgrade
                ? true
                : typeof saved.enabled === 'boolean'
                    ? saved.enabled
                    : DEFAULTS.enabled
        };
        delete loaded.profile;

        if (isUpgrade) {
            try {
                localStorage.setItem(STORAGE_KEY, JSON.stringify(loaded));
            } catch {}
        }

        return loaded;
    }

    let settings = loadSettings();
    let lastActivity = Date.now();
    let interactionBoostUntil = 0;
    let currentMode = 'full';
    let currentUi = 'off';
    let chatCollapsedByAddon = false;

    // Exact campo.mjs loop hook state.
    const nativeRAF = page.requestAnimationFrame?.bind(page);
    const nativeCancelRAF = page.cancelAnimationFrame?.bind(page);
    const throttledRequests = new Map();
    const suspendedCampoRequests = new Map();
    const rafState = new WeakMap();
    const rafClassification = new WeakMap();
    let nextSyntheticRafId = -1;
    let campoHookInstalled = false;
    let campoLoopFound = false;
    let npcLoopFound = false;
    let campoRunsWindow = 0;
    let campoSkippedWindow = 0;
    let campoRunsPerSecond = 0;
    let campoSkippedPerSecond = 0;

    // Load monitor.
    const longTasks = [];
    const jankFrames = [];
    let browserFps = 0;
    let frameCount = 0;
    let frameWindowStart = performance.now();
    let previousMonitorFrame = 0;
    let longTaskMsPerSecond = 0;
    let jankPerSecond = 0;
    let pressure = 'light';
    let adaptiveDecision = '30';
    let adaptiveCandidate = '30';
    let adaptiveCandidateStreak = 0;

    let uiTimerRunsWindow = 0;
    let uiTimerSkippedWindow = 0;
    let uiTimerRunsPerSecond = 0;
    let uiTimerSkippedPerSecond = 0;

    let npcRunsWindow = 0;
    let npcSkippedWindow = 0;
    let npcRunsPerSecond = 0;
    let npcSkippedPerSecond = 0;

    function saveSettings() {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    }

    function profile() {
        return PROFILES.performance;
    }

    function isNativeEconomy() {
        return document.documentElement.classList.contains('modo-economia');
    }

    function autoCatchEnabled() {
        const button = q('#moth-ac-toggle');

        if (button) {
            return button.textContent.trim().toUpperCase() === 'ON';
        }

        try {
            return Boolean(
                JSON.parse(
                    localStorage.getItem('moth-pokeidle-autocatch-v4') || '{}'
                ).enabled
            );
        } catch {
            return false;
        }
    }

    // ------------------------------------------------------------------
    // Native optimized mode cooperation
    // ------------------------------------------------------------------

    function nativeOptimizedEnabled() {
        try {
            return (
                localStorage.getItem(
                    'cfg-otimizado'
                ) === '1'
            );
        } catch {
            return false;
        }
    }

    function forceNativeOptimized() {
        try {
            const owned =
                localStorage.getItem(
                    NATIVE_OPT_OWNER_KEY
                ) === '1';

            if (owned) {
                localStorage.setItem(
                    'cfg-otimizado',
                    '1'
                );

                return;
            }

            if (
                localStorage.getItem(
                    'cfg-otimizado'
                ) === '1'
            ) {
                /*
                 * The player already enabled PokéIdle's own optimization.
                 * Leave it entirely under their control.
                 */
                return;
            }

            const previous =
                localStorage.getItem(
                    'cfg-otimizado'
                );

            localStorage.setItem(
                NATIVE_OPT_BASE_KEY,
                previous === null
                    ? '__null__'
                    : previous
            );

            localStorage.setItem(
                NATIVE_OPT_OWNER_KEY,
                '1'
            );

            localStorage.setItem(
                'cfg-otimizado',
                '1'
            );
        } catch {}
    }

    function restoreNativeOptimized() {
        try {
            if (
                localStorage.getItem(
                    NATIVE_OPT_OWNER_KEY
                ) !== '1'
            ) {
                return;
            }

            const previous =
                localStorage.getItem(
                    NATIVE_OPT_BASE_KEY
                );

            if (
                previous ===
                '__null__'
            ) {
                localStorage.removeItem(
                    'cfg-otimizado'
                );
            } else if (
                previous !==
                null
            ) {
                localStorage.setItem(
                    'cfg-otimizado',
                    previous
                );
            }

            localStorage.removeItem(
                NATIVE_OPT_OWNER_KEY
            );

            localStorage.removeItem(
                NATIVE_OPT_BASE_KEY
            );
        } catch {}
    }

    function syncNativeOptimized() {
        const shouldForce =
            settings.enabled &&
            profile()
                .nativeOptimized &&
            !isNativeEconomy();

        if (shouldForce) {
            forceNativeOptimized();
        } else {
            restoreNativeOptimized();
        }
    }

    /*
     * Restore the user's own native preference on navigation/reload. The
     * next page load re-enables it at document-start if Performance+ still
     * needs it.
     */
    page.addEventListener(
        'pagehide',
        restoreNativeOptimized
    );

    // Upstream sprites.mjs paints active/team Pokémon into DOM canvases via
    // a 220ms setInterval. CSS animation rules do not stop that canvas drawing.
    // Let each sprite canvas paint one frame, then skip clear/draw operations
    // while Performance is active; Full mode resumes the original calls.
    const spriteCanvasDrawCount = new WeakMap();

    function installPokemonSpriteFreezeHook() {
        const contextType = page.CanvasRenderingContext2D;
        const prototype = contextType?.prototype;
        if (!prototype?.drawImage || !prototype?.clearRect) {
            return false;
        }
        if (prototype.drawImage.__mothPerformanceSpriteFreeze) {
            return true;
        }

        const originalDrawImage = prototype.drawImage;
        const originalClearRect = prototype.clearRect;
        const isActiveOrPartySprite = canvas =>
            Boolean(canvas?.closest?.('#ativo-card, #time-corpo'));

        const guardedDrawImage = function (...args) {
            const canvas = this.canvas;
            if (!isActiveOrPartySprite(canvas)) {
                return originalDrawImage.apply(this, args);
            }

            const previousFrames = spriteCanvasDrawCount.get(canvas) || 0;
            if (settings.enabled && currentMode === 'paused' && previousFrames > 0) {
                return;
            }

            const result = originalDrawImage.apply(this, args);
            spriteCanvasDrawCount.set(canvas, previousFrames + 1);
            return result;
        };
        Object.defineProperty(guardedDrawImage, '__mothPerformanceSpriteFreeze', {
            value: true
        });

        const guardedClearRect = function (...args) {
            const canvas = this.canvas;
            if (
                isActiveOrPartySprite(canvas) &&
                settings.enabled &&
                currentMode === 'paused' &&
                (spriteCanvasDrawCount.get(canvas) || 0) > 0
            ) {
                return;
            }
            return originalClearRect.apply(this, args);
        };

        try {
            prototype.drawImage = guardedDrawImage;
            prototype.clearRect = guardedClearRect;
            return prototype.drawImage === guardedDrawImage &&
                prototype.clearRect === guardedClearRect;
        } catch (error) {
            console.warn('[PokéIdle Performance+] Pokémon sprite freeze hook failed', error);
            return false;
        }
    }

    installPokemonSpriteFreezeHook();

    // ------------------------------------------------------------------
    // Upstream-aware requestAnimationFrame throttling
    // ------------------------------------------------------------------

    function scheduleComesFromCampo() {
        try {
            return /(?:https:\/\/pokeidle\.io)?\/campo\.mjs(?:\?|:)/i
                .test(
                    String(
                        new Error()
                            .stack ||
                        ''
                    )
                );
        } catch {
            return false;
        }
    }

    function classifyRafCallback(
        callback
    ) {
        if (
            typeof callback !==
                'function'
        ) {
            return null;
        }

        if (
            rafClassification.has(
                callback
            )
        ) {
            return rafClassification.get(callback);
        }

        const name =
            String(
                callback.name ||
                ''
            );

        let kind =
            null;

        /*
         * app.js has a permanent rAF in the Pokémon Center solely to keep
         * the nurse/TM/depot/house buttons visually attached to NPCs.
         */
        if (
            name ===
                'seguirEnfermeira' ||
            String(
                callback
            ).includes(
                'seguirBotaoCentro'
            )
        ) {
            kind =
                'npc';

            npcLoopFound =
                true;
        } else if (
            scheduleComesFromCampo()
        ) {
            /*
             * Every Campo instance has its own bound loop. v4 only found
             * the first one, allowing replay/war/PvP Campo instances to
             * run uncapped. Classify every distinct callback once.
             */
            kind =
                'campo';

            campoLoopFound =
                true;
        }

        /*
         * Cap every page-world rAF callback, including loops not identified
         * as campo/NPC. This limits JS-driven updates, not compositor refresh.
         */
        rafClassification.set(
            callback,
            kind || 'page'
        );

        if (
            kind ===
                'campo'
        ) {
            console.info(
                '[PokéIdle Performance+] hooked a campo.mjs renderer'
            );
        } else if (
            kind ===
                'npc'
        ) {
            console.info(
                '[PokéIdle Performance+] hooked Center NPC positioning loop'
            );
        }

        return kind;
    }

    function intervalForMode(
        mode
    ) {
        if (
            mode ===
                'full'
        ) {
            return 0;
        }

        const fps =
            Number(mode);

        if (
            !Number.isFinite(
                fps
            ) ||
            fps <= 0
        ) {
            return 0;
        }

        return (
            1000 /
            fps
        );
    }

    function intervalForRafKind(
        kind
    ) {
        if (
            !settings.enabled
        ) {
            return 0;
        }

        if (
            kind ===
                'campo'
        ) {
            return intervalForMode(
                currentMode
            );
        }

        if (
            kind ===
                'npc'
        ) {
            if (
                document.hidden
            ) {
                return 500;
            }

            if (currentMode === 'paused') {
                return 1000;
            }

            if (
                Date.now() <
                interactionBoostUntil
            ) {
                return 1000 / 30;
            }

            if (
                currentUi ===
                    'off'
            ) {
                return 0;
            }

            if (
                currentUi ===
                    'lite'
            ) {
                return 1000 / 20;
            }

            return 100;
        }

        // Keep decorative page-world loops nearly idle while the map renderer is paused.
        if (currentMode === 'paused') {
            return 1000;
        }

        return intervalForMode(currentMode);
    }

    function installCampoHook() {
        if (
            campoHookInstalled ||
            !nativeRAF ||
            !nativeCancelRAF
        ) {
            return campoHookInstalled;
        }

        function mothRequestAnimationFrame(
            callback
        ) {
            const kind =
                classifyRafCallback(
                    callback
                );

            if (!kind) {
                return nativeRAF(
                    callback
                );
            }

            const publicId = nextSyntheticRafId--;
            if (kind === 'campo' && settings.enabled && currentMode === 'paused') {
                const pending = { actualId: null, cancelled: false, suspended: true };
                throttledRequests.set(publicId, pending);
                suspendedCampoRequests.set(publicId, callback);
                return publicId;
            }

            const pending = {
                actualId: null,
                cancelled: false
            };

            const gate =
                timestamp => {
                    if (
                        pending.cancelled
                    ) {
                        throttledRequests.delete(
                            publicId
                        );

                        return;
                    }

                    if (kind === 'campo' && settings.enabled && currentMode === 'paused') {
                        pending.actualId = null;
                        pending.suspended = true;
                        suspendedCampoRequests.set(publicId, callback);
                        return;
                    }

                    const interval =
                        intervalForRafKind(
                            kind
                        );

                    let callbackState =
                        rafState.get(
                            callback
                        );

                    if (
                        !callbackState
                    ) {
                        callbackState = {
                            lastRun:
                                -Infinity
                        };

                        rafState.set(
                            callback,
                            callbackState
                        );
                    }

                    if (
                        interval > 0 &&
                        timestamp -
                            callbackState
                                .lastRun <
                            interval
                    ) {
                        if (
                            kind ===
                                'campo'
                        ) {
                            campoSkippedWindow++;
                        } else if (
                            kind ===
                                'npc'
                        ) {
                            npcSkippedWindow++;
                        }

                        pending.actualId =
                            nativeRAF(
                                gate
                            );

                        return;
                    }

                    callbackState.lastRun =
                        timestamp;

                    if (
                        kind ===
                            'campo'
                    ) {
                        campoRunsWindow++;
                    } else if (
                        kind ===
                            'npc'
                    ) {
                        npcRunsWindow++;
                    }

                    throttledRequests.delete(
                        publicId
                    );

                    callback.call(
                        page,
                        timestamp
                    );
                };

            pending.actualId =
                nativeRAF(
                    gate
                );

            throttledRequests.set(
                publicId,
                pending
            );

            return publicId;
        }

        function mothCancelAnimationFrame(
            id
        ) {
            const pending =
                throttledRequests.get(
                    id
                );

            if (!pending) {
                return nativeCancelRAF(
                    id
                );
            }

            pending.cancelled =
                true;

            if (
                pending.actualId !=
                null
            ) {
                nativeCancelRAF(
                    pending.actualId
                );
            }

            suspendedCampoRequests.delete(id);
            throttledRequests.delete(
                id
            );
        }

        try {
            page.requestAnimationFrame =
                mothRequestAnimationFrame;

            page.cancelAnimationFrame =
                mothCancelAnimationFrame;

            campoHookInstalled =
                page.requestAnimationFrame ===
                mothRequestAnimationFrame;
        } catch (error) {
            console.warn(
                '[PokéIdle Performance+] rAF hook failed',
                error
            );

            campoHookInstalled =
                false;
        }

        return campoHookInstalled;
    }

    // ------------------------------------------------------------------
    // Hot presentation interval throttling
    // ------------------------------------------------------------------

    const uiTimerClassification =
        new WeakMap();

    let uiTimerHookInstalled =
        false;

    function classifyUiTimer(
        callback,
        delay
    ) {
        if (
            typeof callback !==
                'function'
        ) {
            return null;
        }

        if (
            uiTimerClassification.has(
                callback
            )
        ) {
            return (
                uiTimerClassification.get(
                    callback
                ) ||
                null
            );
        }

        const name =
            String(
                callback.name ||
                ''
            );

        const source =
            String(
                callback
            );

        let kind =
            null;

        if (
            name ===
                'tiqueCentroSaida'
        ) {
            kind =
                'exitHud';
        } else if (
            name ===
                'pintarCaidos'
        ) {
            kind =
                'fallen';
        } else if (
            name ===
                'pintarPillDeFila' ||
            name ===
                'tiqueFilaNoModal'
        ) {
            kind =
                'pvpTicker';
        } else if (
            name ===
                'tiqueCentro'
        ) {
            kind =
                'centerTicker';
        } else if (
            Number(delay) ===
                100 &&
            source.includes(
                'reposicionarGolpesPainel'
            ) &&
            source.includes(
                'golpesCooldownFim'
            )
        ) {
            kind =
                'moveHud';
        }

        uiTimerClassification.set(
            callback,
            kind ||
                false
        );

        return kind;
    }

    function uiTimerMinimum(
        kind
    ) {
        if (
            !settings.enabled ||
            currentUi ===
                'off'
        ) {
            return 0;
        }

        /*
         * Any fresh interaction temporarily restores native UI cadence.
         * Performance+ should disappear when the player starts using the
         * interface, then save work again after the short boost window.
         */
        if (
            Date.now() <
            interactionBoostUntil
        ) {
            return 0;
        }

        if (
            kind ===
                'moveHud'
        ) {
            if (
                currentUi ===
                    'lite'
            ) {
                return 250;
            }

            /*
             * The panel is hidden by our aggressive/performance CSS. One
             * slow sweep keeps its state fresh for instant restoration.
             */
            return currentUi ===
                'performance'
                ? 2000
                : 1000;
        }

        if (
            kind ===
                'exitHud'
        ) {
            if (
                currentUi ===
                    'lite'
            ) {
                return 160;
            }

            return currentUi ===
                'performance'
                ? 500
                : 250;
        }

        if (
            kind ===
                'fallen'
        ) {
            /*
             * Auto Catch consumes the upstream fallen-body DOM, so never
             * slow this producer while Auto Catch is active.
             */
            if (
                autoCatchEnabled()
            ) {
                return 0;
            }

            return currentUi ===
                'performance'
                ? 750
                : currentUi ===
                    'aggressive'
                    ? 500
                    : 250;
        }

        if (
            kind ===
                'pvpTicker'
        ) {
            return currentUi ===
                'performance'
                ? 1500
                : currentUi ===
                    'aggressive'
                    ? 1000
                    : 500;
        }

        if (
            kind ===
                'centerTicker'
        ) {
            return currentUi ===
                'performance'
                ? 1500
                : 1000;
        }

        return 0;
    }

    function installUiTimerHook() {
        if (
            uiTimerHookInstalled
        ) {
            return true;
        }

        function mothSetInterval(
            callback,
            delay,
            ...args
        ) {
            const kind =
                classifyUiTimer(
                    callback,
                    delay
                );

            if (!kind) {
                return nativeSetInterval(
                    callback,
                    delay,
                    ...args
                );
            }

            let lastRun =
                -Infinity;

            const wrapped =
                function (
                    ...callbackArgs
                ) {
                    const now =
                        performance.now();

                    const minimum =
                        uiTimerMinimum(
                            kind
                        );

                    if (
                        minimum > 0 &&
                        now -
                            lastRun <
                            minimum
                    ) {
                        uiTimerSkippedWindow++;

                        return;
                    }

                    lastRun =
                        now;

                    uiTimerRunsWindow++;

                    return callback.apply(
                        this,
                        callbackArgs
                    );
                };

            return nativeSetInterval(
                wrapped,
                delay,
                ...args
            );
        }

        try {
            page.setInterval =
                mothSetInterval;

            /*
             * IDs are native interval IDs; clearInterval needs no mapping.
             */
            page.clearInterval =
                nativeClearInterval;

            uiTimerHookInstalled =
                page.setInterval ===
                mothSetInterval;
        } catch (error) {
            console.warn(
                '[PokéIdle Performance+] UI timer hook failed',
                error
            );

            uiTimerHookInstalled =
                false;
        }

        return uiTimerHookInstalled;
    }

    // Install both hooks before app.js/campo.mjs schedule their permanent loops.
    installCampoHook();
    installUiTimerHook();

    // ------------------------------------------------------------------
    // Runtime pressure measurement
    // ------------------------------------------------------------------

    try {
        const observer =
            new PerformanceObserver(list => {
                const now =
                    performance.now();

                for (
                    const entry of
                    list.getEntries()
                ) {
                    longTasks.push({
                        time: now,
                        duration:
                            entry.duration
                    });
                }
            });

        observer.observe({
            type: 'longtask',
            buffered: false
        });
    } catch {}

    function monitorFrame(now) {
        frameCount++;

        if (previousMonitorFrame) {
            const gap =
                now -
                previousMonitorFrame;

            if (gap > 34) {
                jankFrames.push({
                    time: now,
                    gap
                });
            }
        }

        previousMonitorFrame = now;

        const elapsed =
            now -
            frameWindowStart;

        if (elapsed >= 1000) {
            browserFps =
                frameCount *
                1000 /
                elapsed;

            frameCount = 0;
            frameWindowStart = now;
        }

        nativeRAF?.(monitorFrame);
    }

    nativeRAF?.(monitorFrame);

    function prunePerformanceSamples() {
        const now =
            performance.now();

        const cutoff =
            now - 5000;

        while (
            longTasks.length &&
            longTasks[0].time < cutoff
        ) {
            longTasks.shift();
        }

        while (
            jankFrames.length &&
            jankFrames[0].time < cutoff
        ) {
            jankFrames.shift();
        }

        const longMs =
            longTasks.reduce(
                (sum, item) =>
                    sum +
                    item.duration,
                0
            );

        longTaskMsPerSecond =
            longMs / 5;

        jankPerSecond =
            jankFrames.length / 5;

        if (
            document.hidden
        ) {
            pressure = 'background';
        } else if (
            longTaskMsPerSecond >= 90 ||
            jankPerSecond >= 5 ||
            (
                browserFps > 0 &&
                browserFps < 42
            )
        ) {
            pressure = 'heavy';
        } else if (
            longTaskMsPerSecond >= 35 ||
            jankPerSecond >= 2.5 ||
            (
                browserFps > 0 &&
                browserFps < 53
            )
        ) {
            pressure = 'medium';
        } else {
            pressure = 'light';
        }
    }

    function updateAdaptiveDecision() {
        const p =
            profile();

        if (!p.adaptive) {
            return;
        }

        const idleSeconds =
            (
                Date.now() -
                lastActivity
            ) /
            1000;

        if (
            document.hidden
        ) {
            adaptiveDecision =
                p.backgroundScene;

            adaptiveCandidate =
                adaptiveDecision;

            adaptiveCandidateStreak =
                0;

            return;
        }

        if (
            p.idleSeconds > 0 &&
            idleSeconds >=
                p.idleSeconds
        ) {
            adaptiveDecision =
                p.idleScene;

            adaptiveCandidate =
                adaptiveDecision;

            adaptiveCandidateStreak =
                0;

            return;
        }

        if (
            Date.now() <
            interactionBoostUntil
        ) {
            adaptiveDecision =
                p.scene;

            adaptiveCandidate =
                adaptiveDecision;

            adaptiveCandidateStreak =
                0;

            return;
        }

        const desired =
            pressure ===
                'heavy'
                ? '12'
                : pressure ===
                    'medium'
                    ? '20'
                    : p.scene;

        if (
            desired ===
            adaptiveDecision
        ) {
            adaptiveCandidate =
                desired;

            adaptiveCandidateStreak =
                0;

            return;
        }

        if (
            adaptiveCandidate ===
                desired
        ) {
            adaptiveCandidateStreak++;
        } else {
            adaptiveCandidate =
                desired;

            adaptiveCandidateStreak =
                1;
        }

        const currentFps =
            Number(
                adaptiveDecision
            ) ||
            0;

        const desiredFps =
            Number(desired) ||
            0;

        /*
         * Reduce quickly under sustained load, recover more slowly so the
         * renderer does not oscillate around a threshold.
         */
        const required =
            desiredFps <
                currentFps
                ? 2
                : 4;

        if (
            adaptiveCandidateStreak >=
            required
        ) {
            adaptiveDecision =
                desired;

            adaptiveCandidateStreak =
                0;
        }
    }

    // ------------------------------------------------------------------
    // UI performance levels
    // ------------------------------------------------------------------

    function collapseChat() {
        const button =
            q('#btn-chat-toggle');

        if (!button) {
            return;
        }

        if (
            button.getAttribute(
                'aria-expanded'
            ) !== 'false'
        ) {
            button.click();
            chatCollapsedByAddon = true;
        }
    }

    function restoreChat() {
        if (!chatCollapsedByAddon) {
            return;
        }

        const button =
            q('#btn-chat-toggle');

        if (
            button?.getAttribute(
                'aria-expanded'
            ) === 'false'
        ) {
            button.click();
        }

        chatCollapsedByAddon = false;
    }

    function isIdleForProfile(p) {
        if (document.hidden) {
            return true;
        }

        if (p.idleSeconds <= 0) {
            return false;
        }

        return (
            Date.now() -
            lastActivity
        ) >=
            p.idleSeconds * 1000;
    }

    function calculateMode() {
        if (!settings.enabled) {
            return 'full';
        }

        const p = profile();
        if (document.hidden) {
            return p.backgroundScene;
        }
        if (isIdleForProfile(p)) {
            return p.idleScene;
        }
        return p.scene;
    }

    function calculateUiLevel() {
        if (!settings.enabled) {
            return 'off';
        }

        const p = profile();
        return isIdleForProfile(p) ? p.idleUi : p.ui;
    }

    function syncChatPolicy() {
        /*
         * Performance+ no longer changes chat expansion state at all.
         * Collapsing a core communication panel is too intrusive and can
         * leave the interface looking broken after profile/script changes.
         */
        restoreChat();
    }

    function resumeSuspendedCampoLoops() {
        const pendingRequests = [...suspendedCampoRequests.entries()];
        suspendedCampoRequests.clear();

        for (const [id, callback] of pendingRequests) {
            const pending = throttledRequests.get(id);
            if (!pending || pending.cancelled) {
                continue;
            }

            throttledRequests.delete(id);
            pending.cancelled = true;
            try {
                callback.call(page, performance.now());
            } catch (error) {
                console.warn('[PokéIdle Performance+] could not resume map renderer', error);
            }
        }
    }

    function applyMode() {
        const previousMode = currentMode;
        currentMode = calculateMode();
        if (previousMode === 'paused' && currentMode !== 'paused') {
            resumeSuspendedCampoLoops();
        }

        currentUi =
            calculateUiLevel();

        const p = profile();
        const html =
            document.documentElement;

        html.classList.toggle(
            'moth-perf-reduced',
            settings.enabled &&
                p.effects === 'reduced'
        );

        html.classList.toggle(
            'moth-perf-minimal',
            settings.enabled &&
                p.effects === 'minimal'
        );

        for (
            const level of [
                'lite',
                'aggressive',
                'performance'
            ]
        ) {
            html.classList.toggle(
                'moth-ui-' + level,
                settings.enabled &&
                    currentUi === level
            );
        }

        html.classList.toggle(
            'moth-scene-sleep',
            settings.enabled &&
                currentMode === '2'
        );

        syncNativeOptimized();
        syncChatPolicy();
        updateUI();
    }

    function recordActivity() {
        lastActivity = Date.now();

        /*
         * Adaptive mode immediately returns to its responsive ceiling
         * after interaction, then reevaluates load a few seconds later.
         */
        interactionBoostUntil =
            Date.now() + 2500;

        if (
            profile().adaptive
        ) {
            adaptiveDecision =
                profile().scene;

            adaptiveCandidate =
                adaptiveDecision;

            adaptiveCandidateStreak =
                0;
        }

        applyMode();
    }

    for (
        const eventName of [
            'pointerdown',
            'keydown',
            'wheel',
            'touchstart'
        ]
    ) {
        window.addEventListener(
            eventName,
            recordActivity,
            {
                passive: true,
                capture: true
            }
        );
    }

    document.addEventListener(
        'visibilitychange',
        () => {
            if (
                !document.hidden
            ) {
                lastActivity =
                    Date.now();

                interactionBoostUntil =
                    Date.now() +
                    1500;

                if (
                    profile()
                        .adaptive
                ) {
                    adaptiveDecision =
                        profile().scene;

                    adaptiveCandidate =
                        adaptiveDecision;

                    adaptiveCandidateStreak =
                        0;
                }
            }

            applyMode();
        }
    );

    // ------------------------------------------------------------------
    // CSS reductions
    // ------------------------------------------------------------------

    function installStyles() {
        if (q('#' + STYLE_ID)) {
            return;
        }

        const style =
            document.createElement(
                'style'
            );

        style.id = STYLE_ID;

        style.textContent = `
            /*
             * Trace-backed target:
             * reposicionarGolpesPainel produced repeated style/layout work.
             * In an idle game the move HUD is informational, not an input.
             */
            html.moth-ui-lite #golpes-painel,
            html.moth-ui-aggressive #golpes-painel,
            html.moth-ui-performance #golpes-painel {
                display: none !important;
            }

            /*
             * Aggressive UI Lite stops painting frequently-changing
             * informational feeds. Capture controls/history are NOT here.
             */
            html.moth-ui-aggressive #hud-log,
            html.moth-ui-performance #hud-log,
            html.moth-ui-aggressive .evento-faixa,
            html.moth-ui-performance .evento-faixa,
            html.moth-ui-aggressive #musica-player,
            html.moth-ui-performance #musica-player {
                display: none !important;
            }

            /*
             * UI Max must never remove interactive/core game UI.
             * Chat, team, active Pokémon and automation controls stay fully
             * usable. Only redundant live indicators are removed.
             */
            html.moth-ui-performance #ir-centro-conta {
                display: none !important;
            }

            /* Map canvas and decorative overlays are hidden; the actual
               campo.mjs rAF loop is suspended above, not merely made invisible. */
            html.moth-ui-performance #palco > :not(#caidos) {
                visibility: hidden !important;
                pointer-events: none !important;
            }

            html.moth-ui-performance #palco {
                background: transparent !important;
            }

            /* Capture is the sole scene surface kept interactive. */
            html.moth-ui-performance #caidos:not(.hidden),
            html.moth-ui-performance #caidos:not(.hidden) * {
                visibility: visible !important;
                pointer-events: auto !important;
            }

            html.moth-ui-performance #p-chat {
                display: none !important;
            }

            /* Freeze CSS-driven animations for active and party Pokémon. */
            html.moth-ui-performance :is(#ativo-card, #time-corpo),
            html.moth-ui-performance :is(#ativo-card, #time-corpo) *,
            html.moth-ui-performance :is(#ativo-card, #time-corpo) *::before,
            html.moth-ui-performance :is(#ativo-card, #time-corpo) *::after {
                animation-play-state: paused !important;
                animation-duration: 0s !important;
                transition: none !important;
            }

            html.moth-perf-reduced :is(
                .ab-confete *,
                .recompensa *,
                .shiny-anuncio *,
                .lendario-anuncio *,
                .evento-faixa *,
                .pvp-fila-pill *
            ) {
                animation-duration: .001ms !important;
                animation-iteration-count: 1 !important;
                transition-duration: .001ms !important;
            }

            /*
             * Do not put a wildcard rule on every #app descendant: PokéIdle
             * creates/reuses DOM constantly, and forcing every new node and
             * pseudo-element through a giant override selector becomes its
             * own style-recalculation tax. Native Optimized already removes
             * scene hit effects; target the remaining decorative surfaces.
             */
            html.moth-perf-minimal :is(
                .ab-confete,
                .ab-confete *,
                .recompensa,
                .recompensa *,
                .shiny-anuncio,
                .shiny-anuncio *,
                .lendario-anuncio,
                .lendario-anuncio *,
                .evento-faixa,
                .evento-faixa *,
                .pvp-fila-pill,
                .pvp-fila-pill *,
                .rc-flash,
                .rc-particulas,
                .la-relampago,
                .la-raios,
                .la-faiscas
            ) {
                animation-duration: .001ms !important;
                animation-iteration-count: 1 !important;
                transition-duration: .001ms !important;
                backdrop-filter: none !important;
                filter: none !important;
                text-shadow: none !important;
                box-shadow: none !important;
            }

            html.moth-perf-minimal :is(
                .la-relampago,
                .la-raios,
                .la-faiscas,
                .rc-flash,
                .rc-particulas,
                .ab-confete
            ) {
                display: none !important;
            }

            html.moth-scene-sleep #cena {
                opacity: .55 !important;
            }

            #${TRIGGER_ID} {
                display: inline-flex !important;
                align-items: center !important;
                justify-content: center !important;
                min-width: 0 !important;
                width: auto !important;
                height: 25px !important;
                padding: 3px 7px !important;
                margin-left: auto !important;
                border: 1px solid rgba(255,255,255,.16) !important;
                border-radius: 999px !important;
                background: rgba(40,31,35,.75) !important;
                color: #ddd !important;
                font: 700 9px/1 system-ui,sans-serif !important;
                cursor: pointer !important;
                opacity: 1 !important;
                visibility: visible !important;
                position: relative !important;
                z-index: 20 !important;
            }

            #${TRIGGER_ID}.working {
                color: #8fe0a5 !important;
                border-color: rgba(100,210,130,.35) !important;
            }

            #${TRIGGER_ID}.warning {
                color: #ffd27c !important;
                border-color: rgba(255,190,80,.38) !important;
            }

            #${TELEMETRY_ID} {
                display: none !important;
            }

        `;

        (
            document.head ||
            document.documentElement
        ).appendChild(style);
    }

    function labelForMode(mode) {
        if (
            mode ===
                'full'
        ) {
            return 'Full';
        }

        const fps =
            Number(mode);

        return Number.isFinite(
            fps
        )
            ? fps +
              'fps'
            : mode;
    }

    function labelForUi(level) {
        return {
            off: 'UI Full',
            lite: 'UI Lite',
            aggressive: 'UI Low',
            performance: 'UI Max'
        }[level] || level;
    }

    function setText(el, value) {
        if (!el) {
            return;
        }

        const next =
            String(value);

        if (
            el.textContent !==
            next
        ) {
            el.textContent =
                next;
        }
    }

    function setClass(el, className, enabled) {
        if (
            el &&
            el.classList.contains(
                className
            ) !== Boolean(enabled)
        ) {
            el.classList.toggle(
                className,
                Boolean(enabled)
            );
        }
    }

    // ------------------------------------------------------------------
    // Light control UI
    // ------------------------------------------------------------------

    function ensureTelemetry() {
        let telemetry = q('#' + TELEMETRY_ID);

        if (!telemetry) {
            telemetry = document.createElement('div');
            telemetry.id = TELEMETRY_ID;
            telemetry.hidden = true;
            telemetry.setAttribute('aria-hidden', 'true');
            (document.body || document.documentElement)?.appendChild(telemetry);
        }

        for (const id of ['mpp-browser-fps', 'mpp-scene-runs']) {
            if (q('#' + id, telemetry)) continue;

            const value = document.createElement('span');
            value.id = id;
            value.textContent = '0';
            telemetry.appendChild(value);
        }

        return telemetry;
    }

    function ensureTrigger() {
        if (!document.body) {
            return false;
        }

        installStyles();
        installCampoHook();
        ensureTelemetry();

        const header =
            q('#p-auto .painel-cab-auto') ||
            q('#p-auto .painel-cab');

        if (!header) {
            return false;
        }

        let trigger = q('#' + TRIGGER_ID);
        if (!trigger) {
            trigger = document.createElement('button');
            trigger.type = 'button';
            trigger.id = TRIGGER_ID;
            trigger.title = 'Toggle Performance+ on or off';
            trigger.addEventListener('click', event => {
                event.preventDefault();
                event.stopPropagation();

                settings.enabled = !settings.enabled;
                saveSettings();
                applyMode();
            });
        }

        const collapse = q('#btn-auto-toggle', header);
        if (collapse) {
            if (trigger.parentElement !== header) {
                header.insertBefore(trigger, collapse);
            }
        } else if (trigger.parentElement !== header) {
            header.appendChild(trigger);
        }

        return true;
    }

    function updateUI() {
        const trigger =
            q('#' + TRIGGER_ID);

        ensureTelemetry();
        setText(
            q('#mpp-browser-fps'),
            document.hidden || currentMode === 'paused'
                ? 0
                : Math.round(campoRunsPerSecond)
        );
        setText(
            q('#mpp-scene-runs'),
            Math.round(campoRunsPerSecond)
        );

        if (!trigger) {
            return;
        }

        let triggerText;
        let working = false;
        let warning = false;

        if (!settings.enabled) {
            triggerText = 'Performance+ OFF';
        } else if (isNativeEconomy()) {
            triggerText = 'Performance+ ON · Native Eco';
            warning = true;
        } else if (
            !campoHookInstalled ||
            !uiTimerHookInstalled
        ) {
            triggerText = 'Performance+ ON · Hook!';
            warning = true;
        } else if (!campoLoopFound) {
            triggerText = 'Performance+ ON · Loading…';
            warning = true;
        } else {
            triggerText = 'Performance+ ON';
            working = true;
        }

        setText(trigger, triggerText);
        trigger.setAttribute('aria-pressed', String(settings.enabled));
        trigger.title = settings.enabled
            ? 'Turn Performance+ off'
            : 'Turn Performance+ on';

        setClass(trigger, 'working', working);
        setClass(trigger, 'warning', warning);
    }

    // ------------------------------------------------------------------
    // Tick / bootstrap
    // ------------------------------------------------------------------

    nativeSetInterval(() => {
        campoRunsPerSecond =
            campoRunsWindow;

        campoSkippedPerSecond =
            campoSkippedWindow;

        uiTimerRunsPerSecond =
            uiTimerRunsWindow;

        uiTimerSkippedPerSecond =
            uiTimerSkippedWindow;

        npcRunsPerSecond =
            npcRunsWindow;

        npcSkippedPerSecond =
            npcSkippedWindow;

        campoRunsWindow = 0;
        campoSkippedWindow = 0;
        uiTimerRunsWindow = 0;
        uiTimerSkippedWindow = 0;
        npcRunsWindow = 0;
        npcSkippedWindow = 0;

        prunePerformanceSamples();
        updateAdaptiveDecision();
        applyMode();
    }, 1000);

    function bootstrap() {
        installStyles();
        installCampoHook();
        ensureTrigger();

        lastActivity =
            Date.now();

        applyMode();

        nativeSetInterval(() => {
            installCampoHook();
            installUiTimerHook();
            ensureTrigger();
        }, 1500);

        console.info(
            '[PokéIdle Performance+] v5.4.0 loaded'
        );
    }

    if (
        document.readyState ===
        'loading'
    ) {
        document.addEventListener(
            'DOMContentLoaded',
            bootstrap,
            {
                once: true
            }
        );
    } else {
        bootstrap();
    }
})();
