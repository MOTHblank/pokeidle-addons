// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      5.2.0
// @description  Adds Open Live Streams under Open Inventory; clicking it scans the current PokéIdle page for live Twitch/KICK channels and opens them in the current Firefox profile.
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

    const LIVE_VALUE_RE =
        /^(?:1|true|yes|on|live|online|ao[_ -]?vivo|en[_ -]?vivo)$/i;

    const LIVE_TEXT_RE =
        /^(?:live|online|ao vivo|ao-vivo|en vivo|en-vivo|assistir agora|watch now|ver ao vivo|assistir)$/i;

    const NEGATIVE_LIVE_TEXT_RE =
        /^(?:offline|off-line|encerrad[oa]|ended|not live|nao ao vivo)$/i;

    const EXCLUDED_TWITCH_PATHS = new Set([
        'directory',
        'downloads',
        'jobs',
        'p',
        'search',
        'settings',
        'subscriptions',
        'wallet'
    ]);

    const EXCLUDED_KICK_PATHS = new Set([
        'categories',
        'browse',
        'directory',
        'following',
        'search',
        'settings',
        'auth',
        'login',
        'register',
        'signup',
        'video',
        'videos'
    ]);

    let scanInProgress = false;
    const APP_READY_TIMEOUT_MS = 15_000;
    const MAX_STREAMS_PER_SERVICE = 10;
    const STREAMS_KEY = 'moth-pokeidle-streams-v1';
    const openChats = new Map();

    function loadStreamConfig() {
        try {
            const saved = JSON.parse(localStorage.getItem(STREAMS_KEY) || '{}');
            return {
                twitch: Array.from({ length: MAX_STREAMS_PER_SERVICE }, (_, i) => String(saved?.twitch?.[i] || '')),
                kick: Array.from({ length: MAX_STREAMS_PER_SERVICE }, (_, i) => String(saved?.kick?.[i] || ''))
            };
        } catch (_) {
            return {
                twitch: Array(MAX_STREAMS_PER_SERVICE).fill(''),
                kick: Array(MAX_STREAMS_PER_SERVICE).fill('')
            };
        }
    }

    function saveStreamConfig(config) {
        try {
            localStorage.setItem(STREAMS_KEY, JSON.stringify(config));
        } catch (_) {}
    }

    function channelName(raw) {
        const value = String(raw || '').trim();
        if (/^[A-Za-z0-9_-]{1,64}$/.test(value)) {
            return value;
        }

        const url = normalizeChannelUrl(value);
        if (!url) return '';
        return decodeURIComponent(new URL(url).pathname.slice(1));
    }

    function chatUrl(raw) {
        const url = normalizeChannelUrl(raw);
        if (!url) return null;
        const parsed = new URL(url);
        const channel = encodeURIComponent(
            decodeURIComponent(parsed.pathname.slice(1))
        );

        return parsed.hostname === 'twitch.tv'
            ? 'https://www.twitch.tv/popout/' + channel + '/chat'
            : 'https://kick.com/popout/' + channel + '/chat';
    }

    function chatKey(service, index) {
        return 'moth-' + service + '-' + (index + 1);
    }

    async function openChat(service, index, raw, active = false) {
        const channel = channelName(raw);
        const url = chatUrl(raw);

        if (!channel || !url) {
            return {
                opened: false,
                error: 'Invalid ' + service + ' channel'
            };
        }

        const key = chatKey(service, index);
        const current = openChats.get(key);

        if (current && !current.closed) {
            return {
                opened: true,
                alreadyOpen: true,
                url
            };
        }

        try {
            const options = {
                active,
                insert: true
            };

            let control;

            // Prefer the explicitly granted legacy API. It is synchronous and avoids
            // turning a user click into an asynchronous popup attempt.
            if (typeof GM_openInTab === 'function') {
                control = GM_openInTab(url, options);
            } else if (typeof GM !== 'undefined' && typeof GM.openInTab === 'function') {
                control = await GM.openInTab(url, options);
            } else {
                throw new Error(
                    'Violentmonkey tab API is unavailable. Reinstall/update the Moth Stream Scanner addon.'
                );
            }

            if (!control) {
                throw new Error('Violentmonkey did not create the chat tab.');
            }

            openChats.set(key, control);

            return {
                opened: true,
                alreadyOpen: false,
                url
            };
        } catch (error) {
            console.error('[Moth] could not open chat with Violentmonkey:', {
                service,
                channel,
                url,
                error
            });

            // This is only a fallback for a broken/missing VM tab API.
            // It opens the CHAT URL, never the PokéIdle URL.
            try {
                const fallback = window.open(
                    url,
                    '_blank',
                    'noopener,noreferrer'
                );

                if (fallback) {
                    return {
                        opened: true,
                        alreadyOpen: false,
                        fallback: true,
                        url
                    };
                }
            } catch (_) {}

            return {
                opened: false,
                error: error?.message || String(error)
            };
        }
    }

    function closeChat(service, index) {
        const key = chatKey(service, index);
        const chat = openChats.get(key);
        if (!chat) return false;

        try { chat.close(); } catch (_) {}
        openChats.delete(key);
        return true;
    }

    async function openAllConfiguredChats(config) {
        let opened = 0;
        const errors = [];

        for (const service of ['twitch', 'kick']) {
            for (let index = 0; index < MAX_STREAMS_PER_SERVICE; index += 1) {
                const value = config[service][index];
                if (!value) continue;

                const result = await openChat(service, index, value);

                if (result.opened) {
                    opened += 1;
                } else if (result.error) {
                    errors.push(
                        service.toUpperCase() + ' ' + (index + 1) + ': ' + result.error
                    );
                }
            }
        }

        return { opened, errors };
    }

    function closeAllChats() {
        let closed = 0;

        for (const key of Array.from(openChats.keys())) {
            try { openChats.get(key)?.close(); } catch (_) {}
            openChats.delete(key);
            closed += 1;
        }

        return closed;
    }


    function qa(selector, root) {
        return Array.from(
            (root || document).querySelectorAll(selector)
        );
    }

    function normalizeText(value) {
        return String(value == null ? '' : value)
            .normalize('NFD')
            .replace(/[\u0300-\u036f]/g, '')
            .replace(/\s+/g, ' ')
            .trim()
            .toLowerCase();
    }

    function normalizeChannelUrl(raw) {
        try {
            const url = new URL(
                String(raw || ''),
                location.href
            );

            if (!/^https?:$/i.test(url.protocol)) {
                return null;
            }

            const host = url.hostname
                .toLowerCase()
                .replace(/^www\./, '');

            if (host !== 'twitch.tv' && host !== 'kick.com') {
                return null;
            }

            const segments = url.pathname
                .split('/')
                .map(part => part.trim())
                .filter(Boolean);

            if (segments.length !== 1) {
                return null;
            }

            const channel = segments[0];

            if (!channel || channel.startsWith(':')) {
                return null;
            }

            const excluded =
                host === 'twitch.tv'
                    ? EXCLUDED_TWITCH_PATHS
                    : EXCLUDED_KICK_PATHS;

            if (excluded.has(channel.toLowerCase())) {
                return null;
            }

            return (
                'https://' +
                host +
                '/' +
                encodeURIComponent(channel)
            );
        } catch (_) {
            return null;
        }
    }

    function readLiveValue(value) {
        const text = normalizeText(value);

        if (!text) {
            return null;
        }

        if (NEGATIVE_LIVE_TEXT_RE.test(text)) {
            return false;
        }

        return LIVE_VALUE_RE.test(text)
            ? true
            : null;
    }

    function inspectAttributes(element) {
        if (!element || element.nodeType !== 1) {
            return null;
        }

        const attributes = [
            'data-live',
            'data-is-live',
            'data-online',
            'data-stream-live',
            'data-streaming',
            'data-status',
            'data-state',
            'aria-label',
            'title'
        ];

        for (const name of attributes) {
            const value = element.getAttribute(name);
            const result = readLiveValue(value);

            if (result !== null) {
                return result;
            }

            if (
                /^(?:aria-label|title)$/.test(name) &&
                LIVE_TEXT_RE.test(normalizeText(value))
            ) {
                return true;
            }
        }

        return null;
    }

    function inspectClasses(element) {
        if (!element || !element.classList) {
            return null;
        }

        const classes = Array.from(element.classList)
            .map(normalizeText)
            .filter(Boolean);

        for (const token of classes) {
            if (
                /^(?:live|is-live|live-now|live-stream|stream-live|online|is-online|ao-vivo|aovivo|en-vivo|envivo)$/.test(token)
            ) {
                return true;
            }

            if (
                /(?:offline|is-offline|ended|encerrad[oa])/.test(token)
            ) {
                return false;
            }
        }

        return null;
    }

    function normalizedBadgeText(value) {
        return normalizeText(value)
            .replace(/^[^a-z0-9à-ÿ]+/i, '')
            .replace(/[^a-z0-9à-ÿ]+$/i, '')
            .trim();
    }

    function inspectBadgeText(container) {
        if (!container) {
            return null;
        }

        const badgeCandidates = qa(
            'b,strong,small,span,i,[role="status"],[class*="badge"],[class*="status"],[class*="live"],[class*="online"]',
            container
        );

        for (const node of badgeCandidates.slice(0, 80)) {
            const text = normalizedBadgeText(node.textContent);

            if (!text || text.length > 40) {
                continue;
            }

            if (NEGATIVE_LIVE_TEXT_RE.test(text)) {
                return false;
            }

            if (LIVE_TEXT_RE.test(text)) {
                return true;
            }

            if (
                /^(?:\d+\s+)?(?:live|online|ao vivo|ao-vivo|en vivo|en-vivo)(?:\s+\d+)?$/i.test(text)
            ) {
                return true;
            }
        }

        return null;
    }

    function hasLiveMarker(anchor) {
        let node = anchor;

        for (
            let depth = 0;
            node && depth <= 8;
            depth += 1
        ) {
            const attrResult = inspectAttributes(node);

            if (attrResult !== null) {
                return attrResult;
            }

            const classResult = inspectClasses(node);

            if (classResult !== null) {
                return classResult;
            }

            const badgeResult = inspectBadgeText(node);

            if (badgeResult !== null) {
                return badgeResult;
            }

            node = node.parentElement;
        }

        return false;
    }

    function collectRenderedOfficialChannels(selectors) {
        const channels = [];
        const seen = new Set();

        for (const selector of selectors) {
            for (const anchor of qa(selector)) {
                const url = normalizeChannelUrl(anchor.href || anchor.getAttribute('href'));
                if (!url || seen.has(url)) continue;

                seen.add(url);
                channels.push({
                    url,
                    anchor
                });
            }
        }

        return channels;
    }

    function collectLiveChannels() {
        const direct = collectRenderedOfficialChannels([
            'a.tw-canal.ao-vivo[href]',
            'a.kk-canal.ao-vivo[href]'
        ]);

        const candidates = new Map(
            direct.map((channel) => [channel.url, channel])
        );

        // Keep the generic fallback for other PokéIdle builds/pages that expose
        // live stream links directly in the DOM.
        for (const anchor of qa('a[href]')) {
            const url = normalizeChannelUrl(
                anchor.href ||
                anchor.getAttribute('href')
            );

            if (!url || !hasLiveMarker(anchor)) {
                continue;
            }

            candidates.set(url, {
                url,
                anchor
            });
        }

        return Array.from(candidates.values());
    }

    function sleep(ms) {
        return new Promise((resolve) => window.setTimeout(resolve, ms));
    }

    async function waitForGameUi(timeoutMs = APP_READY_TIMEOUT_MS) {
        const started = Date.now();

        while (Date.now() - started < timeoutMs) {
            if (
                document.body &&
                (
                    document.getElementById('tr-ativos') ||
                    document.querySelector('.menu-topo') ||
                    document.getElementById(INVENTORY_ID)
                )
            ) {
                return true;
            }

            await sleep(250);
        }

        return false;
    }

    async function collectLiveChannelsFromOfficialModals() {
        const collected = new Map(
            collectRenderedOfficialChannels([
                'a.tw-canal.ao-vivo[href]',
                'a.kk-canal.ao-vivo[href]'
            ]).map((channel) => [channel.url, channel])
        );

        const targets = [
            {
                trigger: 'button.tr-ativo.twitch',
                body: '#tw-corpo',
                links: '#tw-corpo a.tw-canal.ao-vivo[href]'
            },
            {
                trigger: 'button.tr-ativo.kick',
                body: '#kk-corpo',
                links: '#kk-corpo a.kk-canal.ao-vivo[href]'
            }
        ];

        for (const target of targets) {
            const trigger = document.querySelector(target.trigger);

            if (!trigger) {
                continue;
            }

            // The Twitch/KICK row itself tells us whether that service has a live
            // state right now. Skip opening the modal when it is definitely offline.
            if (
                target.trigger.includes('.twitch') &&
                !trigger.classList.contains('tw-aovivo') &&
                !trigger.classList.contains('tw-ativo')
            ) {
                continue;
            }

            if (
                target.trigger.includes('.kick') &&
                !trigger.classList.contains('kk-aovivo') &&
                !trigger.classList.contains('kk-ativo')
            ) {
                continue;
            }

            let body = document.querySelector(target.body);

            if (!body) {
                try {
                    trigger.click();
                } catch (_) {
                    continue;
                }

                for (let attempt = 0; attempt < 20; attempt += 1) {
                    await sleep(100);
                    body = document.querySelector(target.body);

                    if (body) {
                        break;
                    }
                }
            }

            if (!body) {
                continue;
            }

            for (let attempt = 0; attempt < 20; attempt += 1) {
                const found = collectRenderedOfficialChannels([target.links]);

                for (const channel of found) {
                    collected.set(channel.url, channel);
                }

                if (found.length) {
                    break;
                }

                await sleep(150);
            }

            const close = document.getElementById('modal-fechar');

            if (close && !document.querySelector('#modal')?.classList.contains('hidden')) {
                try {
                    close.click();
                } catch (_) {}

                await sleep(100);
            }
        }

        return Array.from(collected.values());
    }

    async function openStream(url) {
        const parsed = new URL(url);
        const service = parsed.hostname.replace(/^www\./, '') === 'twitch.tv'
            ? 'twitch'
            : 'kick';

        const channel = channelName(url);
        if (!channel) return false;

        const config = loadStreamConfig();
        let index = config[service].findIndex(
            value => normalizeText(value) === normalizeText(channel)
        );

        if (index < 0) {
            index = config[service].findIndex(value => !value);

            if (index >= 0) {
                config[service][index] = channel;
                saveStreamConfig(config);
            }
        }

        if (index < 0) return false;

        const result = await openChat(service, index, channel);
        return result.opened;
    }

    function closeManager() {
        document.getElementById('moth-stream-manager')?.remove();
    }

    function renderManager() {
        if (document.getElementById('moth-stream-manager')) return;

        const config = loadStreamConfig();
        const panel = document.createElement('div');
        panel.id = 'moth-stream-manager';
        panel.style.cssText = [
            'position:fixed',
            'right:18px',
            'top:70px',
            'z-index:2147483647',
            'width:430px',
            'max-height:80vh',
            'overflow:auto',
            'box-sizing:border-box',
            'padding:12px',
            'background:#111',
            'color:#eee',
            'border:1px solid #555',
            'border-radius:8px',
            'box-shadow:0 8px 30px rgba(0,0,0,.45)',
            'font:13px/1.3 system-ui,sans-serif'
        ].join(';');

        const header = document.createElement('div');
        header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:8px;font-weight:700';
        header.textContent = 'Moth Stream Chats';

        const closeManagerButton = document.createElement('button');
        closeManagerButton.type = 'button';
        closeManagerButton.textContent = '×';
        closeManagerButton.style.cssText = 'background:none;border:0;color:#fff;font-size:22px;cursor:pointer';
        closeManagerButton.addEventListener('click', closeManager);
        header.appendChild(closeManagerButton);
        panel.appendChild(header);

        const note = document.createElement('div');
        note.textContent = 'Chat-only: no stream video is loaded. These connections use this game profile.';
        note.style.cssText = 'margin-bottom:10px;opacity:.72';
        panel.appendChild(note);

        const actions = document.createElement('div');
        actions.style.cssText = 'display:flex;gap:6px;margin-bottom:10px';

        const openAll = document.createElement('button');
        openAll.type = 'button';
        openAll.textContent = 'Open all';
        openAll.addEventListener('click', async () => {
            openAll.disabled = true;

            const result = await openAllConfiguredChats(config);

            openAll.textContent = result.errors.length
                ? 'Opened ' + result.opened + ' · ' + result.errors.length + ' error(s)'
                : 'Opened ' + result.opened;

            if (result.errors.length) {
                console.error('[Moth] open-all errors:', result.errors);
            }

            window.setTimeout(() => {
                openAll.textContent = 'Open all';
                openAll.disabled = false;
            }, 1800);
        });

        const closeAll = document.createElement('button');
        closeAll.type = 'button';
        closeAll.textContent = 'Close all';
        closeAll.addEventListener('click', () => {
            const count = closeAllChats();
            closeAll.textContent = 'Closed ' + count;
            window.setTimeout(() => { closeAll.textContent = 'Close all'; }, 1000);
        });

        for (const button of [openAll, closeAll]) {
            button.style.cssText = 'padding:5px 9px;cursor:pointer';
            actions.appendChild(button);
        }

        panel.appendChild(actions);

        for (const service of ['twitch', 'kick']) {
            const section = document.createElement('section');

            const title = document.createElement('div');
            title.textContent = service === 'twitch'
                ? 'Twitch · 10 chat slots'
                : 'KICK · 10 chat slots';
            title.style.cssText = 'font-weight:700;margin:8px 0 5px';
            section.appendChild(title);

            for (let index = 0; index < MAX_STREAMS_PER_SERVICE; index += 1) {
                const row = document.createElement('div');
                row.style.cssText = 'display:flex;gap:4px;margin:3px 0';

                const number = document.createElement('span');
                number.textContent = String(index + 1).padStart(2, '0');
                number.style.cssText = 'width:22px;opacity:.6;padding-top:5px';

                const input = document.createElement('input');
                input.type = 'text';
                input.placeholder = 'channel';
                input.value = config[service][index];
                input.style.cssText = 'flex:1;min-width:0;padding:5px';

                const save = document.createElement('button');
                save.type = 'button';
                save.textContent = 'Save';
                save.style.cssText = 'padding:4px 7px;cursor:pointer';

                const chat = document.createElement('a');
                chat.textContent = 'Chat';
                chat.target = '_blank';
                chat.rel = 'noopener noreferrer';
                chat.style.cssText = 'display:inline-block;box-sizing:border-box;padding:4px 7px;cursor:pointer;text-decoration:none;color:inherit;border:1px solid currentColor;border-radius:2px';

                chat.href = chatUrl(config[service][index]) || '#';
                                const close = document.createElement('button');
                close.type = 'button';
                close.textContent = '×';
                close.title = 'Close chat';
                close.style.cssText = 'padding:4px 8px;cursor:pointer';

                save.addEventListener('click', () => {
                    const value = channelName(input.value);
                    if (!value) return;
                    config[service][index] = value;
                    input.value = value;
                    saveStreamConfig(config);
                });

                chat.addEventListener('mousedown', (event) => {
                    if (event.button !== 0) return;

                    const value = channelName(input.value);

                    if (!value) {
                        chat.textContent = 'Invalid';
                        window.setTimeout(() => { chat.textContent = 'Chat'; }, 1200);
                        return;
                    }

                    config[service][index] = value;
                    input.value = value;
                    saveStreamConfig(config);
                    chat.href = chatUrl(value) || '#';
                });

                                close.addEventListener('click', () => {
                    closeChat(service, index);
                });

                row.append(number, input, save, chat, close);
                section.appendChild(row);
            }

            panel.appendChild(section);
        }

        document.body.appendChild(panel);
    }

    async function runLiveScan() {
        const ready = await waitForGameUi();

        if (!ready) {
            console.info(
                '[Moth] live chat scan: PokéIdle UI did not become ready'
            );

            return {
                channels: [],
                queued: 0,
                notReady: true
            };
        }

        // The stream rows are server-driven. Give the current game a few short
        // opportunities to render them instead of requiring the Twitch/KICK row
        // to exist at the exact instant the button is clicked.
        let channels = [];

        for (let attempt = 0; attempt < 10; attempt += 1) {
            channels = await collectLiveChannelsFromOfficialModals();

            if (channels.length) {
                break;
            }

            await sleep(500);
        }

        let queued = 0;

        for (const channel of channels) {
            if (await openStream(channel.url)) {
                queued += 1;
            }
        }

        console.info(
            '[Moth] live chat scan:',
            channels.length,
            'live channel(s),',
            queued,
            'opened/queued'
        );

        return {
            channels,
            queued
        };
    }

    async function scanLiveStreams(button) {
        if (scanInProgress) {
            return;
        }

        scanInProgress = true;

        const originalLabel = button?.querySelector('span')?.textContent?.trim() ||
            'Open Live Streams';

        try {
            if (button) {
                button.disabled = true;
                const label = button.querySelector('span');
                if (label) {
                    label.textContent = 'Scanning…';
                }
            }

            const result = await runLiveScan();

            if (button) {
                const label = button.querySelector('span');
                if (label) {
                    label.textContent = result.notReady
                        ? 'Game not ready'
                        : result.queued > 0
                            ? 'Scanned ' + result.queued + ' live'
                            : result.channels.length > 0
                                ? 'Found live, open failed'
                                : 'No live streams found';
                }
            }

            return result;
        } catch (error) {
            console.error(
                '[Moth] live chat scan failed:',
                error
            );

            if (button) {
                const label = button.querySelector('span');
                if (label) {
                    label.textContent = 'Scan failed';
                }
            }

            return {
                channels: [],
                queued: 0
            };
        } finally {
            if (button) {
                window.setTimeout(() => {
                    button.disabled = false;
                    const label = button.querySelector('span');
                    if (label) {
                        label.textContent =
                            originalLabel || 'Open Live Streams';
                    }
                }, 1600);
            }

            scanInProgress = false;
        }
    }

    function ensureButton() {
        if (!document.documentElement) {
            return;
        }

        const inventory = document.getElementById(INVENTORY_ID);

        if (!inventory) {
            return;
        }

        const existing = document.getElementById(BUTTON_ID);

        if (existing) {
            return;
        }

        const button = document.createElement('button');
        button.type = 'button';
        button.id = BUTTON_ID;
        button.title =
'Open all live Twitch/KICK streams';
        button.setAttribute(
            'aria-label',
            'Open all live Twitch and KICK streams'
        );

        const computed = typeof window.getComputedStyle === 'function'
            ? window.getComputedStyle(inventory)
            : null;

        button.style.display = computed?.display === 'inline'
            ? 'inline-block'
            : (computed?.display || 'inline-flex');
        button.style.alignItems = 'center';
        button.style.justifyContent = 'center';
        button.style.boxSizing = 'border-box';
        button.style.visibility = 'visible';
        button.style.opacity = '1';
        button.style.pointerEvents = 'auto';
        button.style.cursor = 'pointer';
        button.style.minHeight = inventory.offsetHeight > 0
            ? inventory.offsetHeight + 'px'
            : '30px';
        button.style.margin = computed?.margin || '2px 0 0 0';
        button.style.padding = computed?.padding || '6px 10px';
        button.style.font = computed?.font || 'inherit';
        button.style.lineHeight = computed?.lineHeight || 'normal';
        button.style.color = computed?.color || 'inherit';
        button.style.background = computed?.background || 'transparent';
        button.style.border = computed?.border || '1px solid currentColor';
        button.style.borderRadius = computed?.borderRadius || '4px';

        const label = document.createElement('span');
        label.textContent = 'Open Live Streams';
        button.appendChild(label);

        button.addEventListener('click', () => {
            scanLiveStreams(button);
        });

        inventory.insertAdjacentElement(
            'afterend',
            button
        );

        const managerButton = document.createElement('button');
        managerButton.type = 'button';
        managerButton.id = 'moth-manage-streams';
        managerButton.textContent = 'Manage Chats';
        managerButton.title = 'Manage up to 10 Twitch + 10 KICK chat-only connections';
        managerButton.style.cssText = button.style.cssText;
        managerButton.style.marginLeft = '4px';
        managerButton.addEventListener('click', renderManager);
        inventory.insertAdjacentElement('afterend', managerButton);
    }

    function start() {
        ensureButton();

        window.setInterval(() => {
            ensureButton();
        }, 3000);

        console.info(
            '[Moth] live chat scanner ready'
        );
    }

    start();
})();
