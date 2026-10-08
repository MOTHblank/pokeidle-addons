// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      6.3.1
// @description  Opens current official Twitch chats as lightweight popouts and delegates KICK streams to the native normal-browser manager; refreshes every 10 minutes.
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

    const MAX_LIVE_STREAMS_PER_SERVICE = 10;
    const LIVE_SCAN_INTERVAL_MS = 10 * 60 * 1000;
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

    async function collectOfficialLiveChannels() {
        const collected = new Map();

        const direct = collectLinks([
            'a.tw-canal.ao-vivo[href]',
            'a.kk-canal.ao-vivo[href]'
        ]);

        for (const item of direct) {
            collected.set(item.service + ':' + text(item.name), item);
        }

        const targets = [
            {
                service: 'twitch',
                row: '.tr-ativo.twitch.tw-aovivo',
                body: '#tw-corpo',
                links: '#tw-corpo a.tw-canal.ao-vivo[href]'
            },
            {
                service: 'kick',
                row: '.tr-ativo.kick.kk-aovivo',
                body: '#kk-corpo',
                links: '#kk-corpo a.kk-canal.ao-vivo[href]'
            }
        ];

        for (const target of targets) {
            const row = document.querySelector(target.row);

            if (!row) {
                continue;
            }

            try {
                row.click();
            } catch (_) {
                continue;
            }

            for (let attempt = 0; attempt < 15; attempt += 1) {
                const links = collectLinks([target.links]);

                for (const item of links) {
                    collected.set(
                        item.service + ':' + text(item.name),
                        item
                    );
                }

                if (links.length) {
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
        // KICK is intentionally not opened from this userscript. Its pages
        // must stay outside the Moth-controlled Firefox profile so KICK sees
        // a normal browser. Rust opens/closes the reported live KICK windows.
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

            let live = new Map();

            for (let attempt = 0; attempt < 10; attempt += 1) {
                live = await collectOfficialLiveChannels();

                if (live.size) {
                    break;
                }

                await sleep(500);
            }

            const rows = liveStateRows();
            const streamStateIsAvailable =
                !!rows.twitch || !!rows.kick;

            lastKickStateAvailable = !!rows.kick && !!streamStateIsAvailable;
            lastKickLive = [...live.values()]
                .filter(item => item.service === 'kick')
                .map(item => ({
                    name: item.name,
                    url: item.url
                }));

            const liveKeys = new Set();
            const perService = {
                twitch: 0,
                kick: 0
            };

            let opened = 0;

            for (const item of live.values()) {
                if (perService[item.service] >= MAX_LIVE_STREAMS_PER_SERVICE) {
                    continue;
                }

                const key =
                    item.service + ':' + text(item.name);

                liveKeys.add(key);
                perService[item.service] += 1;

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
        await runLiveScan('scheduled');
        window.setTimeout(scheduledScan, LIVE_SCAN_INTERVAL_MS);
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
        version: 1,
        snapshot() {
            return {
                live: lastKickLive.map(item => ({ ...item })),
                stateAvailable: lastKickStateAvailable,
                scannedAt: Number(document.getElementById(BUTTON_ID)?.dataset?.mothScanAt || 0) || 0
            };
        }
    };

    console.info('[Moth] live stream scanner ready · KICK is delegated to native browser profile · first scan 30s after page load · every 10 minutes thereafter');
    }

    start();
})();
