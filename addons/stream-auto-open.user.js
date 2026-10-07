// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      4.2.0
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
    let startupActionHandled = false;
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

    function openChat(service, index, raw) {
        const channel = channelName(raw);
        const url = chatUrl(raw);
        if (!channel || !url) return false;

        const key = chatKey(service, index);
        const current = openChats.get(key);

        if (current && !current.closed) {
            return true;
        }

        try {
            const chat = GM_openInTab(url, {
                active: false,
                insert: true
            });

            if (!chat) return false;

            openChats.set(key, chat);
            return true;
        } catch (_) {
            return false;
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

    function openAllConfiguredChats(config) {
        let opened = 0;

        for (const service of ['twitch', 'kick']) {
            for (let index = 0; index < MAX_STREAMS_PER_SERVICE; index += 1) {
                if (openChat(service, index, config[service][index])) {
                    opened += 1;
                }
            }
        }

        return opened;
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

    function collectLiveChannels() {
        const candidates = new Map();

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

    function openStream(url) {
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
        return openChat(service, index, channel);
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
        openAll.addEventListener('click', () => {
            const count = openAllConfiguredChats(config);
            openAll.textContent = 'Opened ' + count;
            window.setTimeout(() => { openAll.textContent = 'Open all'; }, 1000);
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

                const chat = document.createElement('button');
                chat.type = 'button';
                chat.textContent = 'Chat';
                chat.style.cssText = 'padding:4px 7px;cursor:pointer';

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

                chat.addEventListener('click', () => {
                    const value = channelName(input.value);
                    if (!value) return;
                    config[service][index] = value;
                    input.value = value;
                    saveStreamConfig(config);
                    openChat(service, index, value);
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

    function scanLiveStreams(button) {
        if (scanInProgress) {
            return;
        }

        scanInProgress = true;

        const originalLabel = button.textContent.trim();

        try {
            button.disabled = true;
            button.querySelector('span').textContent =
                'Scanning…';

            const channels = collectLiveChannels();
            let queued = 0;

            for (const channel of channels) {
                if (openStream(channel.url)) {
                    queued += 1;
                }
            }

            button.querySelector('span').textContent =
                queued > 0
                    ? 'Scanned ' + queued + ' live'
                    : 'No live streams found';

            console.info(
                '[Moth] manual live chat scan:',
                channels.length,
                'live channel(s),',
                queued,
                'opened/queued'
            );
        } catch (error) {
            console.error(
                '[Moth] manual live chat scan failed:',
                error
            );
            button.querySelector('span').textContent =
                'Scan failed';
        } finally {
            window.setTimeout(() => {
                button.disabled = false;
                button.querySelector('span').textContent =
                    originalLabel || 'Open Live Streams';
                scanInProgress = false;
            }, 1600);
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

    function consumeStartupAction() {
        if (startupActionHandled || !location.search) {
            return;
        }

        const url = new URL(location.href);
        const action = url.searchParams.get('moth-stream-action');

        if (action !== 'manager' && action !== 'scan') {
            return;
        }

        startupActionHandled = true;
        url.searchParams.delete('moth-stream-action');
        history.replaceState(null, '', url.pathname + url.search + url.hash);

        const run = () => {
            if (action === 'manager') {
                renderManager();
                return;
            }

            const button = document.getElementById(BUTTON_ID);
            if (!button) {
                startupActionHandled = false;
                window.setTimeout(consumeStartupAction, 750);
                return;
            }

            scanLiveStreams(button);
        };

        window.setTimeout(run, 1200);
    }

    function start() {
        ensureButton();

        window.setInterval(() => {
            ensureButton();
            consumeStartupAction();
        }, 3000);

        consumeStartupAction();

        console.info(
            '[Moth] live chat scanner ready'
        );
    }

    start();
})();
