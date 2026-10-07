// ==UserScript==
// @name         PokéIdle Live Chat Presence
// @namespace    moth.pokeidle
// @version      2.0.0
// @description  Joins every Twitch/Kick channel that PokéIdle currently marks as live so the configured account is present in chat.
// @match        https://pokeidle.io/app*
// @grant        unsafeWindow
// @run-at       document-start
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const page =
        typeof unsafeWindow !== 'undefined'
            ? unsafeWindow
            : window;

    // The upstream client is an SPA and rebuilds stream cards dynamically.
    const SCAN_INTERVAL_MS = 5000;
    const SCAN_DEBOUNCE_MS = 350;
    const MISSING_RESET_MS = 20000;
    const MAX_AUTO_OPEN_PER_SCAN = 20;
    const MAX_ANCESTORS_TO_INSPECT = 8;

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

    // Per-page state. A URL is eligible to open again only after it has
    // disappeared long enough to represent a real live/offline transition.
    const streamState = new Map();
    let scanTimer = 0;
    let scheduledScan = 0;
    let observer = null;

    function q(selector, root) {
        return (root || document).querySelector(selector);
    }

    function qa(selector, root) {
        return Array.from((root || document).querySelectorAll(selector));
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
            const url = new URL(String(raw || ''), location.href);

            if (!/^https?:$/i.test(url.protocol)) {
                return null;
            }

            const host = url.hostname.toLowerCase().replace(/^www\./, '');

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

            // /videos/... and other non-channel routes are excluded by the
            // one-segment rule. Keep the canonical channel URL only.
            return 'https://' + host + '/' + encodeURIComponent(channel);
        } catch {
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
            'aria-live',
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
            .replace(/(?:^|\s)(?:•|·|[-–—])(?:\s|$)/g, ' ')
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

            if (/^(?:\d+\s+)?(?:live|online|ao vivo|ao-vivo|en vivo|en-vivo)(?:\s+\d+)?$/i.test(text)) {
                return true;
            }
        }

        return null;
    }

    function hasLiveMarker(anchor) {
        let node = anchor;

        for (let depth = 0; node && depth <= MAX_ANCESTORS_TO_INSPECT; depth += 1) {
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

    function collectCandidates() {
        const candidates = new Map();

        for (const anchor of qa('a[href]')) {
            const url = normalizeChannelUrl(anchor.href || anchor.getAttribute('href'));

            if (!url) {
                continue;
            }

            const live = hasLiveMarker(anchor);

            if (!candidates.has(url)) {
                candidates.set(url, {
                    url,
                    live,
                    anchor
                });
                continue;
            }

            // Multiple links for one channel are common in the upstream
            // card. Treat the channel as live if any copy carries the marker.
            if (live) {
                candidates.get(url).live = true;
            }
        }

        return candidates;
    }

    function openThroughIdleShell(url) {
        try {
            if (
                typeof page.__idleshell_openLink === 'function' &&
                page.__idleshell_openLink(url, 'stream-auto-open')
            ) {
                return true;
            }
        } catch (_) {}

        try {
            if (page.chrome?.webview?.postMessage) {
                page.chrome.webview.postMessage(
                    JSON.stringify({
                        type: 'link',
                        url,
                        source: 'stream-auto-open'
                    })
                );
                return true;
            }
        } catch (_) {}

        return false;
    }

    function openStream(url) {
        if (openThroughIdleShell(url)) {
            return true;
        }

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

    function processCandidates() {
        scheduledScan = 0;

        const now = Date.now();
        const candidates = collectCandidates();
        let opened = 0;

        for (const [url, candidate] of candidates) {
            const previous = streamState.get(url) || {
                live: false,
                opened: false,
                lastSeenAt: 0
            };

            const wasLive = previous.live;
            const isNewOrReset =
                previous.lastSeenAt === 0 ||
                now - previous.lastSeenAt > MISSING_RESET_MS;

            previous.lastSeenAt = now;

            if (!candidate.live) {
                previous.live = false;
                previous.opened = false;
                streamState.set(url, previous);
                continue;
            }

            previous.live = true;

            if (
                (!wasLive || isNewOrReset) &&
                !previous.opened &&
                opened < MAX_AUTO_OPEN_PER_SCAN
            ) {
                if (openStream(url)) {
                    previous.opened = true;
                    opened += 1;
                    console.info(
                        '[IdleShell] joined live chat:',
                        url
                    );
                }
            }

            streamState.set(url, previous);
        }

        // Forget channels that vanished from the DOM long enough to be
        // considered a new live transition when they return.
        for (const [url, state] of streamState) {
            if (
                now - state.lastSeenAt > MISSING_RESET_MS &&
                !candidates.has(url)
            ) {
                streamState.delete(url);
            }
        }

        if (opened > 0) {
            console.info(
                '[IdleShell] live chat scan joined',
                opened,
                'stream(s)'
            );
        }
    }

    function scheduleScan(delay = SCAN_DEBOUNCE_MS) {
        if (scheduledScan) {
            window.clearTimeout(scheduledScan);
        }

        scheduledScan = window.setTimeout(
            processCandidates,
            delay
        );
    }

    function installObserver() {
        if (!document.documentElement || observer) {
            return;
        }

        observer = new MutationObserver(() => {
            scheduleScan();
        });

        observer.observe(document.documentElement, {
            childList: true,
            subtree: true,
            attributes: true,
            attributeFilter: [
                'class',
                'data-live',
                'data-is-live',
                'data-online',
                'data-stream-live',
                'data-streaming',
                'data-status',
                'data-state',
                'aria-label',
                'title',
                'href'
            ]
        });
    }

    function start() {
        processCandidates();
        installObserver();

        if (!scanTimer) {
            scanTimer = window.setInterval(
                processCandidates,
                SCAN_INTERVAL_MS
            );
        }

        console.info(
            '[IdleShell] live chat presence active (PokéIdle upstream 1.240.1)'
        );
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start, {
            once: true
        });
    } else {
        start();
    }

    // The upstream SPA can replace the <html> subtree during a hard
    // navigation/login transition, so re-install the observer if needed.
    window.setInterval(() => {
        if (!observer && document.documentElement) {
            installObserver();
            scheduleScan(0);
        }
    }, 10000);
})();
