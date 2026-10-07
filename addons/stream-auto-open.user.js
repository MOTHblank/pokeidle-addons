// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      6.1.1
// @description  Opens current official Twitch/KICK live chats in background tabs and refreshes the list once per hour.
// @match        https://pokeidle.io/app*
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/stream-auto-open.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/stream-auto-open.user.js
// @run-at       document-start
// @grant        GM_openInTab
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const BUTTON_ID = 'moth-scan-live-streams';
    const INVENTORY_ID = 'btn-bolsa';

    const MAX_LIVE_CHATS_PER_SERVICE = 10;
    const LIVE_SCAN_INTERVAL_MS = 60 * 60 * 1000;
    const INITIAL_SCAN_DELAY_MS = 30 * 1000;
    const UI_RECHECK_INTERVAL_MS = 30 * 1000;

    let scanInProgress = false;
    const openChats = new Map();

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

    function openChat(item) {
        if (!item.chat) {
            return false;
        }

        const key = item.service + ':' + text(item.name);
        const current = openChats.get(key);

        if (current && !current.closed) {
            return true;
        }

        try {
            if (typeof GM_openInTab !== 'function') {
                console.error('[Moth] GM_openInTab is unavailable');
                return false;
            }

            const tab = GM_openInTab(item.chat, {
                active: false,
                insert: true,
                setParent: true
            });

            if (!tab) {
                return false;
            }

            openChats.set(key, tab);
            return true;
        } catch (error) {
            console.error(
                '[Moth] failed to open chat:',
                item.chat,
                error
            );
            return false;
        }
    }

    function closeChatsNotLive(liveKeys) {
        for (const [key, tab] of [...openChats.entries()]) {
            if (tab?.closed || !liveKeys.has(key)) {
                try {
                    if (!tab?.closed) {
                        tab.close();
                    }
                } catch (_) {}

                openChats.delete(key);
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
        button.dataset.mothScanAt = String(Date.now());
    }

    async function runLiveScan(reason = 'manual') {
        if (scanInProgress) {
            return {
                channels: [],
                opened: 0,
                tracked: openChats.size,
                skipped: true
            };
        }

        scanInProgress = true;
        setScanDiagnostics({ status: 'scanning', reason });

        try {
            if (!(await waitForGameUi())) {
                console.info('[Moth] live scan skipped: game UI not ready');

                setScanDiagnostics({ status: 'not-ready', reason, tracked: openChats.size });
                return {
                    channels: [],
                    opened: 0,
                    tracked: openChats.size,
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

            const liveKeys = new Set();
            const perService = {
                twitch: 0,
                kick: 0
            };

            let opened = 0;

            for (const item of live.values()) {
                if (perService[item.service] >= MAX_LIVE_CHATS_PER_SERVICE) {
                    continue;
                }

                const key =
                    item.service + ':' + text(item.name);

                liveKeys.add(key);
                perService[item.service] += 1;

                if (openChat(item)) {
                    opened += 1;
                }
            }

            // Only close old chats when PokéIdle actually exposed its stream
            // state. A transient server/UI delay must never wipe valid chats.
            if (streamStateIsAvailable) {
                closeChatsNotLive(liveKeys);
            }

            setScanDiagnostics({
                status: 'ok',
                reason,
                live: live.size,
                opened,
                tracked: openChats.size
            });

            console.info(
                '[Moth] live chat scan:',
                reason,
                live.size,
                'live channel(s),',
                opened,
                'opened/kept,',
                openChats.size,
                'tracked'
            );

            return {
                channels: [...live.values()],
                opened,
                tracked: openChats.size
            };
        } catch (error) {
            setScanDiagnostics({ status: 'failed', reason, tracked: openChats.size });
            console.error('[Moth] live chat scan failed:', error);

            return {
                channels: [],
                opened: 0,
                tracked: openChats.size,
                failed: true
            };
        } finally {
            scanInProgress = false;
        }
    }

    async function scheduledScan() {
        await runLiveScan('hourly');
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
        button.title = 'Open current live Twitch/KICK chats';
        button.setAttribute(
            'aria-label',
            'Open current live Twitch and KICK chats'
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

        console.info('[Moth] live chat scanner ready · first scan 30s after page load · hourly thereafter');
    }

    start();
})();
