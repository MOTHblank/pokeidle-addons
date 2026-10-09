// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      6.9.1
// @description  Opens current official Twitch chats as lightweight popouts and delegates KICK streams to the native normal-browser manager; refreshes every 2 minutes.
// @match        https://pokeidle.io/app*
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/stream-auto-open.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/master/addons/stream-auto-open.user.js
// @run-at       document-start
// @grant        GM_openInTab
// @grant        unsafeWindow
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const BUTTON_ID = 'moth-scan-live-streams';
    const INVENTORY_ID = 'btn-bolsa';

    const LIVE_SCAN_INTERVAL_MS = 2 * 60 * 1000;
    const INITIAL_SCAN_DELAY_MS = 30 * 1000;
    const UI_RECHECK_INTERVAL_MS = 30 * 1000;

    let scanInProgress = false;
    const openStreams = new Map();
    let lastKickLive = [];
    let lastKickStateAvailable = false;

    const excludedTwitch = new Set([
        'directory', 'downloads', 'jobs', 'p', 'search',
        'settings', 'subscriptions', 'wallet'
    ]);

    const excludedKick = new Set([
        'categories', 'browse', 'directory', 'following', 'search',
        'settings', 'auth', 'login', 'register', 'signup',
        'video', 'videos'
    ]);

    const sleep = (ms) =>
        new Promise((resolve) => window.setTimeout(resolve, ms));

    const text = (value) =>
        String(value == null ? '' : value)
            .normalize('NFD')
            .replace(/[\u0300-\u036f]/g, '')
            .replace(/\s+/g, ' ')
            .trim()
            .toLowerCase();

    function channelName(raw) {
        const value = String(raw || '').trim();

        if (/^[A-Za-z0-9_-]{1,64}$/.test(value)) {
            return value;
        }

        try {
            const url = new URL(value, location.href);
            const host = url.hostname.toLowerCase().replace(/^www\./, '');

            if (host !== 'twitch.tv' && host !== 'kick.com') {
                return '';
            }

            const parts = url.pathname.split('/').filter(Boolean);

            return parts.length === 1
                ? decodeURIComponent(parts[0])
                : '';
        } catch (_) {
            return '';
        }
    }

    function channelUrl(service, raw) {
        const name = channelName(raw);

        if (!name) {
            return null;
        }

        const host =
            service === 'twitch'
                ? 'www.twitch.tv'
                : service === 'kick'
                    ? 'kick.com'
                    : null;

        return host
            ? 'https://' + host + '/' + encodeURIComponent(name)
            : null;
    }

    function chatUrl(service, raw) {
        const name = channelName(raw);

        if (!name) {
            return null;
        }

        const host =
            service === 'twitch'
                ? 'www.twitch.tv'
                : service === 'kick'
                    ? 'kick.com'
                    : null;

        return host
            ? 'https://' + host + '/popout/' +
                encodeURIComponent(name) + '/chat'
            : null;
    }

    function normalizeLiveLink(service, raw) {
        const url = channelUrl(service, raw);

        if (!url) {
            return null;
        }

        const name = channelName(raw);
        const excluded =
            service === 'twitch' ? excludedTwitch : excludedKick;

        return excluded.has(name.toLowerCase())
            ? null
            : {
                service,
                name,
                url,
                chat: chatUrl(service, name)
            };
    }

    function collectLinks(selectors) {
        const found = new Map();

        for (const selector of selectors) {
            for (const anchor of document.querySelectorAll(selector)) {
                const href =
                    anchor.href ||
                    anchor.getAttribute('href') ||
                    '';

                const host = (() => {
                    try {
                        return new URL(href, location.href)
                            .hostname
                            .toLowerCase()
                            .replace(/^www\./, '');
                    } catch (_) {
                        return '';
                    }
                })();

                const service =
                    host === 'twitch.tv'
                        ? 'twitch'
                        : host === 'kick.com'
                            ? 'kick'
                            : null;

                if (!service) {
                    continue;
                }

                const item = normalizeLiveLink(service, href);

                if (item) {
                    found.set(service + ':' + text(item.name), item);
                }
            }
        }

        return [...found.values()];
    }

    function liveStateRows() {
        return {
            twitch: document.querySelector(
                '#tr-ativos .tr-ativo.twitch'
            ),
            kick: document.querySelector(
                '#tr-ativos .tr-ativo.kick'
            )
        };
    }

    function readLiveChannelsFromBridge() {
        try {
            const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
            const bridge = page.__mothControllerBridgeV1;
            const state = bridge && typeof bridge.streamStateSnapshot === 'function'
                ? bridge.streamStateSnapshot()
                : null;

            if (!state) {
                return null;
            }

            const twitchLives = state.twitch?.lives;
            const kickOfficials = state.kick?.oficiais;
            const hasTwitchState = Array.isArray(twitchLives);
            const hasKickState = Array.isArray(kickOfficials);

            if (!hasTwitchState && !hasKickState) {
                return null;
            }

            const collected = new Map();

            if (hasTwitchState) {
                for (const channel of twitchLives) {
                    const raw = channel?.login || channel?.name;
                    const item = normalizeLiveLink('twitch', raw);
                    if (item) {
                        collected.set('twitch:' + text(item.name), item);
                    }
                }
            }

            const kickLive = [];
            if (hasKickState) {
                for (const channel of kickOfficials) {
                    if (!channel?.aoVivo) {
                        continue;
                    }

                    const raw = channel?.slug || channel?.login;
                    const item = normalizeLiveLink('kick', raw);
                    if (!item) {
                        continue;
                    }

                    collected.set('kick:' + text(item.name), item);
                    kickLive.push({
                        name: item.name,
                        url: item.url
                    });
                }
            }

            return {
                live: collected,
                kickStateAvailable: hasKickState,
                kickLive
            };
        } catch (_) {
            return null;
        }
    }

    async function collectOfficialLiveChannels() {
        // Compatibility fallback for cases where the controller bridge has not
        // attached yet. The normal path reads the game's current live state
        // directly and does not need to open/close the bonus modals.
        const collected = new Map();

        const direct = collectLinks([
            'a.tw-canal.ao-vivo[href*="twitch.tv/"]',
            'a.kk-canal.ao-vivo[href*="kick.com/"]',
            '#tr-ativos .tr-ativo.twitch.tw-aovivo a[href*="twitch.tv/"]',
            '#tr-ativos .tr-ativo.kick.kk-aovivo a[href*="kick.com/"]'
        ]);

        for (const item of direct) {
            collected.set(item.service + ':' + text(item.name), item);
        }

        const target = {
            row: '.tr-ativo.twitch.tw-aovivo',
            links: '#tw-corpo a.tw-canal.ao-vivo[href*="twitch.tv/"]'
        };
        const row = document.querySelector(target.row);

        if (row) {
            try {
                row.click();
            } catch (_) {
                return collected;
            }

            for (let attempt = 0; attempt < 15; attempt += 1) {
                for (const item of collectLinks([target.links])) {
                    collected.set(
                        item.service + ':' + text(item.name),
                        item
                    );
                }

                if ([...collected.values()].some(item => item.service === 'twitch')) {
                    break;
                }

                await sleep(100);
            }

            const close = document.getElementById('modal-fechar');
            if (close) {
                try {
                    close.click();
                } catch (_) {}
                await sleep(80);
            }
        }

        return collected;
    }

    function waitForGameUi(timeoutMs = 15_000) {
        return new Promise((resolve) => {
            const started = Date.now();

            const check = () => {
                const ready =
                    !!document.body &&
                    (
                        document.getElementById(INVENTORY_ID) ||
                        document.querySelector('.menu-topo')
                    );

                if (ready) {
                    resolve(true);
                    return;
                }

                if (Date.now() - started >= timeoutMs) {
                    resolve(false);
                    return;
                }

                window.setTimeout(check, 250);
            };

            check();
        });
    }

    function openStream(item) {
        // KICK channels are reported to the Rust interface, where they are
        // combined into one MultiKick link for the user to open in a regular
        // browser. Neither this userscript nor the controller opens KICK streams.
        if (item.service === 'kick') {
            return false;
        }

        // Twitch still uses its lightweight chat popout.
        const targetUrl = item.chat;

        if (!targetUrl) {
            return false;
        }

        const key = item.service + ':' + text(item.name);
        const current = openStreams.get(key);

        if (current && !current.closed) {
            return true;
        }

        try {
            if (typeof GM_openInTab !== 'function') {
                console.error('[Moth] GM_openInTab is unavailable');
                return false;
            }

            const tab = GM_openInTab(targetUrl, {
                active: false,
                insert: true,
                setParent: true
            });

            if (!tab) {
                return false;
            }

            openStreams.set(key, tab);
            return true;
        } catch (error) {
            console.error(
                '[Moth] failed to open stream:',
                targetUrl,
                error
            );
            return false;
        }
    }

    function rewardedTwitchKeys() {
        // A stream can be actively rewarding even when the current live-stream
        // list does not contain it (for example while PokéIdle is refreshing
        // its stream data). Never close a Twitch tab that the game reports as
        // currently being watched for rewards.
        try {
            const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
            const bridge = page.__mothControllerBridgeV1;
            const snapshot = bridge && typeof bridge.snapshot === 'function'
                ? bridge.snapshot()
                : null;
            const watching = snapshot?.state?.twitch?.assistindoEm;

            if (!Array.isArray(watching)) {
                return new Set();
            }

            return new Set(
                watching
                    .map(channelName)
                    .filter(Boolean)
                    .map(name => 'twitch:' + text(name))
            );
        } catch (_) {
            return new Set();
        }
    }

    function closeStreamsNotLive(liveKeys, protectedKeys = new Set()) {
        for (const [key, tab] of [...openStreams.entries()]) {
            if (
                tab?.closed ||
                (!liveKeys.has(key) && !protectedKeys.has(key))
            ) {
                try {
                    if (!tab?.closed) {
                        tab.close();
                    }
                } catch (_) {}

                openStreams.delete(key);
            }
        }
    }

    function setScanDiagnostics(data) {
        const button = document.getElementById(BUTTON_ID);
        if (!button) return;

        button.dataset.mothScanStatus = data.status || '';
        button.dataset.mothScanReason = data.reason || '';
        button.dataset.mothScanLive = String(data.live ?? 0);
        button.dataset.mothScanOpened = String(data.opened ?? 0);
        button.dataset.mothScanTracked = String(data.tracked ?? 0);
        button.dataset.mothScanKickLive = String(lastKickLive.length);
        button.dataset.mothScanAt = String(Date.now());
    }

    async function runLiveScan(reason = 'manual') {
        if (scanInProgress) {
            return {
                channels: [],
                opened: 0,
                tracked: openStreams.size,
                skipped: true
            };
        }

        scanInProgress = true;
        setScanDiagnostics({ status: 'scanning', reason });

        try {
            if (!(await waitForGameUi())) {
                console.info('[Moth] live scan skipped: game UI not ready');

                setScanDiagnostics({ status: 'not-ready', reason, tracked: openStreams.size });
                return {
                    channels: [],
                    opened: 0,
                    tracked: openStreams.size,
                    notReady: true
                };
            }

            let bridgeData = readLiveChannelsFromBridge();
            let live;

            if (bridgeData) {
                // Fast path: game state already includes the current live
                // channels, including KICK. No modal/UI scraping is needed.
                live = bridgeData.live;
            } else {
                live = new Map();
                for (let attempt = 0; attempt < 10; attempt += 1) {
                    live = await collectOfficialLiveChannels();

                    if (live.size) {
                        break;
                    }

                    await sleep(500);
                }
            }

            const rows = liveStateRows();
            const streamStateIsAvailable =
                bridgeData !== null || !!rows.twitch || !!rows.kick;

            lastKickStateAvailable = bridgeData
                ? bridgeData.kickStateAvailable
                : !!rows.kick;
            lastKickLive = bridgeData
                ? bridgeData.kickLive
                : [...live.values()]
                    .filter(item => item.service === 'kick')
                    .map(item => ({
                        name: item.name,
                        url: item.url
                    }));

            const liveKeys = new Set();
            let opened = 0;

            for (const item of live.values()) {
                const key =
                    item.service + ':' + text(item.name);

                liveKeys.add(key);

                if (item.service === 'kick') {
                    continue;
                }

                if (openStream(item)) {
                    opened += 1;
                }
            }

            // Only close old streams when PokéIdle actually exposed its stream
            // state. A transient server/UI delay must never wipe valid chats.
            if (streamStateIsAvailable) {
                const protectedKeys = rewardedTwitchKeys();
                closeStreamsNotLive(liveKeys, protectedKeys);
            }

            setScanDiagnostics({
                status: 'ok',
                reason,
                live: live.size,
                opened,
                tracked: openStreams.size
            });

            console.info(
                '[Moth] live stream scan:',
                reason,
                live.size,
                'live channel(s),',
                opened,
                'opened/kept,',
                openStreams.size,
                'tracked'
            );
            if (lastKickLive.length) {
                console.info(
                    '[Moth] KICK channels available for Rust MultiKick link:',
                    lastKickLive.map(item => item.name)
                );
            }

            return {
                channels: [...live.values()],
                opened,
                tracked: openStreams.size
            };
        } catch (error) {
            setScanDiagnostics({ status: 'failed', reason, tracked: openStreams.size });
            console.error('[Moth] live stream scan failed:', error);

            return {
                channels: [],
                opened: 0,
                tracked: openStreams.size,
                failed: true
            };
        } finally {
            scanInProgress = false;
        }
    }

    async function scheduledScan() {
        const startedAt = Date.now();
        await runLiveScan('scheduled');
        const elapsed = Date.now() - startedAt;
        window.setTimeout(
            scheduledScan,
            Math.max(0, LIVE_SCAN_INTERVAL_MS - elapsed)
        );
    }

    function ensureButton() {
        const inventory = document.getElementById(INVENTORY_ID);

        if (!inventory || document.getElementById(BUTTON_ID)) {
            return;
        }

        const button = document.createElement('button');
        button.type = 'button';
        button.id = BUTTON_ID;
        button.title = 'Open current live Twitch chat and KICK watch pages';
        button.setAttribute(
            'aria-label',
            'Open current live Twitch and KICK streams'
        );

        const style = window.getComputedStyle(inventory);

        button.style.cssText = [
            'display:inline-flex',
            'align-items:center',
            'justify-content:center',
            'box-sizing:border-box',
            'min-height:' + Math.max(30, inventory.offsetHeight) + 'px',
            'margin:' + (style.margin || '2px 0 0 0'),
            'padding:' + (style.padding || '6px 10px'),
            'font:' + (style.font || 'inherit'),
            'line-height:' + (style.lineHeight || 'normal'),
            'color:' + (style.color || 'inherit'),
            'background:' + (style.background || 'transparent'),
            'border:' + (style.border || '1px solid currentColor'),
            'border-radius:' + (style.borderRadius || '4px'),
            'cursor:pointer'
        ].join(';');

        const label = document.createElement('span');
        label.textContent = 'Open Live Streams';
        button.appendChild(label);

        button.addEventListener('click', async () => {
            button.disabled = true;
            label.textContent = 'Scanning…';

            const result = await runLiveScan('manual');

            label.textContent = result.failed
                ? 'Scan failed'
                : result.notReady
                    ? 'Game not ready'
                    : result.channels.length
                        ? 'Opened/kept ' + result.opened
                        : 'No live streams found';

            window.setTimeout(() => {
                if (!button.isConnected) {
                    return;
                }

                button.disabled = false;
                label.textContent = 'Open Live Streams';
            }, 1600);
        });

        inventory.insertAdjacentElement('afterend', button);
    }

    function start() {
        ensureButton();

        window.setInterval(
            ensureButton,
            UI_RECHECK_INTERVAL_MS
        );

        const scheduleInitialScan = () => {
            window.setTimeout(() => {
                void scheduledScan();
            }, INITIAL_SCAN_DELAY_MS);
        };

        if (document.readyState === 'complete') {
            scheduleInitialScan();
        } else {
            window.addEventListener('load', scheduleInitialScan, { once: true });
        }

        const page = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;
    page.__mothKickScannerV1 = {
        version: 2,
        snapshot() {
            // The native monitor polls this API independently of the 2-minute
            // Twitch tab scan. Read KICK's current game state on every poll so
            // live channels appear/disappear promptly without waiting for a
            // modal refresh or the next scheduled scan.
            const current = readLiveChannelsFromBridge();
            if (current) {
                // Clear stale entries if a refreshed complete state no longer
                // contains KICK data; never keep channels from an old snapshot.
                lastKickLive = current.kickLive;
                lastKickStateAvailable = current.kickStateAvailable;
            }

            return {
                live: lastKickLive.map(item => ({ ...item })),
                stateAvailable: lastKickStateAvailable,
                scannedAt: Date.now()
            };
        }
    };

    console.info('[Moth] live stream scanner v6.9.1 ready · KICK channels are shown as a MultiKick link in Rust · first scan 30s after page load · every 2 minutes thereafter');
    }

    start();
})();
