// ==UserScript==
// @name         PokéIdle Live Stream Scanner
// @namespace    moth.pokeidle
// @version      3.2.0
// @description  Adds Open Live Streams under Open Inventory; clicking it scans the current PokéIdle page for live Twitch/KICK channels and opens them in the current Firefox profile.
// @match        https://pokeidle.io/app*
// @run-at       document-start
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
        try {
            const popup = window.open(
                url,
                '_blank',
                'noopener,noreferrer'
            );
            return !!popup;
        } catch (_) {
            return false;
        }
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
                'queued'
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
    }

    function start() {
        ensureButton();

        window.setInterval(() => {
            ensureButton();
        }, 3000);

        console.info(
            '[Moth] manual live stream scanner ready'
        );
    }

    start();
})();
