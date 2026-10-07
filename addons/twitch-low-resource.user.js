// ==UserScript==
// @name         Twitch + KICK Low Resource Mode
// @namespace    moth.pokeidle
// @version      2.0.0
// @description  Keeps Twitch/KICK stream tabs at the lowest practical resource usage. Twitch chat pop-outs stay untouched; KICK channel pages keep the real player alive for watch-time Channel Points while the rest of the page is culled.
// @match        https://www.twitch.tv/*
// @match        https://www.twitch.tv/*/*
// @match        https://player.twitch.tv/*
// @match        https://m.twitch.tv/*
// @match        https://kick.com/*
// @match        https://*.kick.com/*
// @grant        none
// @updateURL    https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/twitch-low-resource.user.js
// @downloadURL  https://raw.githubusercontent.com/MOTHblank/pokeidle-addons/rust-rewrite/addons/twitch-low-resource.user.js
// @run-at       document-idle
// @noframes
// ==/UserScript==

(() => {
    'use strict';

    const sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

    const isTwitch = /(^|\.)twitch\.tv$/i.test(location.hostname);
    const isKick = /(^|\.)kick\.com$/i.test(location.hostname);

    // Twitch popout chat is already the minimum viable Twitch surface.
    const isTwitchChatPopout =
        isTwitch && /\/popout\/[^/]+\/chat(?:[/?]|$)/i.test(location.pathname);

    if (isTwitchChatPopout) {
        console.info('[Moth] Twitch chat-only tab left untouched');
        return;
    }

    if (isTwitch) {
        runTwitch();
        return;
    }

    if (isKick) {
        runKick();
    }

    function textOf(el) {
        try {
            return String(
                el?.closest('label')?.textContent ||
                el?.getAttribute?.('aria-label') ||
                el?.parentElement?.textContent ||
                el?.textContent ||
                ''
            ).replace(/\s+/g, ' ').trim();
        } catch (_) {
            return '';
        }
    }

    function numericQuality(text) {
        const match = String(text || '').match(/(\d{3,4})p/i);
        return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
    }

    function closeKickMenu() {
        try {
            document.dispatchEvent(new KeyboardEvent('keydown', {
                key: 'Escape',
                code: 'Escape',
                bubbles: true,
                cancelable: true
            }));
        } catch (_) {}
    }

    function simulateClick(el) {
        if (!el) return;

        try {
            el.focus();
        } catch (_) {}

        for (const type of [
            'pointerover',
            'pointerenter',
            'pointerdown',
            'mousedown',
            'pointerup',
            'mouseup',
            'click'
        ]) {
            try {
                el.dispatchEvent(new PointerEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    composed: true,
                    pointerId: 1,
                    pointerType: 'mouse',
                    isPrimary: true
                }));
            } catch (_) {
                try {
                    el.click();
                } catch (_) {}
            }
        }
    }

    function findMainVideo() {
        const videos = [...document.querySelectorAll('video')]
            .filter(video => {
                const rect = video.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            })
            .sort((a, b) => {
                const ar = a.getBoundingClientRect();
                const br = b.getBoundingClientRect();
                return (br.width * br.height) - (ar.width * ar.height);
            });

        return videos[0] || null;
    }

    function runTwitch() {
        const TARGET_QUALITY_RE = /^160p(?:\D|$)/i;
        const QUALITY_OPTION_SELECTOR =
            '[data-a-target="player-settings-menu"] input[type="radio"],' +
            '[data-a-target="player-settings-submenu-quality-option"]';

        let qualityBusy = false;
        let lastQualityAttempt = 0;
        let qualityConfiguredAt = 0;
        let lastUrl = location.href;

        const isLowestOption = (el) => TARGET_QUALITY_RE.test(textOf(el));

        const closeQualityMenu = async () => {
            closeKickMenu();
            await sleep(100);

            const menu = document.querySelector('[data-a-target="player-settings-menu"]');
            if (menu) {
                try {
                    document.querySelector('[data-a-target="player-settings-button"]')?.click();
                } catch (_) {}
            }
        };

        const forceLowestQuality = async () => {
            if (qualityBusy) return;
            if (Date.now() - qualityConfiguredAt < 60000) return;
            if (Date.now() - lastQualityAttempt < 15000) return;

            const gear =
                document.querySelector('[data-a-target="player-settings-button"]');

            const video = document.querySelector('video');

            if (!gear || !video) return;

            lastQualityAttempt = Date.now();
            qualityBusy = true;

            try {
                gear.click();
                await sleep(250);

                let qualityButton =
                    document.querySelector(
                        '[data-a-target="player-settings-menu-item-quality"]'
                    );

                if (!qualityButton) {
                    qualityButton = [...document.querySelectorAll(
                        '[data-a-target="player-settings-menu"] button'
                    )].find(el => /quality/i.test(el.textContent || ''));
                }

                if (!qualityButton) return;

                qualityButton.click();
                await sleep(250);

                const options = [...document.querySelectorAll(QUALITY_OPTION_SELECTOR)];

                if (!options.length) return;

                let target = options.find(isLowestOption);

                if (!target) {
                    const scored = options
                        .map((el, index) => ({
                            el,
                            index,
                            height: numericQuality(textOf(el))
                        }))
                        .filter(item => Number.isFinite(item.height))
                        .sort(
                            (a, b) =>
                                a.height - b.height ||
                                a.index - b.index
                        );

                    target = scored[0]?.el;
                }

                if (!target) return;

                (target.closest('label') || target.parentElement || target).click();
                await sleep(250);

                qualityConfiguredAt = Date.now();
                console.info('[Moth] Twitch low-resource quality:', textOf(target));
            } catch (error) {
                console.warn('[Moth] Twitch low-resource quality failed:', error);
            } finally {
                await closeQualityMenu();
                qualityBusy = false;
            }
        };

        const addTwitchCss = () => {
            if (document.getElementById('moth-twitch-low-resource-css')) return;

            const style = document.createElement('style');
            style.id = 'moth-twitch-low-resource-css';
            style.textContent = [
                '[data-a-target="chat-room-component-layout"] { display:none !important; }',
                '[data-a-target="side-nav"] { display:none !important; }',
                '[data-a-target="recommendations-container"],',
                '[data-a-target="home-recommendations"] { content-visibility:auto !important; }'
            ].join('\n');

            (document.head || document.documentElement)?.appendChild(style);
        };

        const tick = () => {
            addTwitchCss();

            if (location.href !== lastUrl) {
                lastUrl = location.href;
                lastQualityAttempt = 0;
                qualityConfiguredAt = 0;
            }

            void forceLowestQuality();
        };

        addTwitchCss();
        void forceLowestQuality();
        setInterval(tick, 15000);

        console.info('[Moth] Twitch low-resource mode active');
    }

    function runKick() {
        const excludedRoutes = new Set([
            '',
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
            'videos',
            'subscriptions',
            'dashboard'
        ]);

        // The scanner opens /channel-slug directly. Do not turn unrelated KICK
        // account/discovery routes into a blank player-only page.
        const parts = location.pathname.split('/').filter(Boolean);
        if (parts.length !== 1 || excludedRoutes.has(parts[0].toLowerCase())) {
            return;
        }

        let lastVideoKey = '';
        let qualityBusy = false;
        let qualityDoneKey = '';
        let lastQualityAttempt = 0;
        let culledPlayer = null;
        let lastUrl = location.href;

        const MAIN_PLAYER_SMALL_WIDTH = 260;
        const MAIN_PLAYER_SMALL_HEIGHT = 146;

        const addKickCss = () => {
            if (document.getElementById('moth-kick-low-resource-css')) return;

            const style = document.createElement('style');
            style.id = 'moth-kick-low-resource-css';
            style.textContent = [
                'html, body { overflow:hidden !important; }',
                'body.moth-kick-low-resource-active > * { visibility:hidden !important; }',
                'body.moth-kick-low-resource-active .moth-kick-player-root,',
                'body.moth-kick-low-resource-active .moth-kick-player-root * { visibility:visible !important; }',
                '.moth-kick-player-root {',
                '  position:fixed !important;',
                '  left:auto !important;',
                '  top:auto !important;',
                '  right:4px !important;',
                '  bottom:4px !important;',
                '  width:' + MAIN_PLAYER_SMALL_WIDTH + 'px !important;',
                '  height:' + MAIN_PLAYER_SMALL_HEIGHT + 'px !important;',
                '  min-width:' + MAIN_PLAYER_SMALL_WIDTH + 'px !important;',
                '  min-height:' + MAIN_PLAYER_SMALL_HEIGHT + 'px !important;',
                '  max-width:' + MAIN_PLAYER_SMALL_WIDTH + 'px !important;',
                '  max-height:' + MAIN_PLAYER_SMALL_HEIGHT + 'px !important;',
                '  margin:0 !important;',
                '  padding:0 !important;',
                '  overflow:hidden !important;',
                '  background:#000 !important;',
                '  contain:layout paint !important;',
                '  z-index:2147483646 !important;',
                '}',
                '.moth-kick-player-root video,',
                '#video-player {',
                '  width:100% !important;',
                '  height:100% !important;',
                '  max-width:none !important;',
                '  max-height:none !important;',
                '  min-width:0 !important;',
                '  min-height:0 !important;',
                '  object-fit:cover !important;',
                '  opacity:0.02 !important;',
                '  pointer-events:none !important;',
                '}',
                '.moth-kick-low-resource-hidden { visibility:hidden !important; }',
                '[data-testid*="chat" i],',
                '[data-testid*="sidebar" i],',
                '[data-testid*="recommend" i],',
                'header, nav, aside, [role="navigation"] { visibility:hidden !important; }',
                '* { animation:none !important; transition:none !important; scroll-behavior:auto !important; }'
            ].join('\n');

            (document.head || document.documentElement)?.appendChild(style);
        };

        const findPlayerRoot = (video) => {
            if (!video) return null;

            const videoRect = video.getBoundingClientRect();
            let best = null;

            for (let node = video.parentElement, depth = 0; node && depth < 7; node = node.parentElement, depth += 1) {
                const rect = node.getBoundingClientRect();

                if (
                    rect.width >= videoRect.width &&
                    rect.height >= videoRect.height &&
                    rect.width <= Math.max(videoRect.width * 2.5, 640) &&
                    rect.height <= Math.max(videoRect.height * 2.5, 360)
                ) {
                    best = node;
                    break;
                }
            }

            return best || video.parentElement;
        };

        const cullKickPage = (video) => {
            const root = findPlayerRoot(video);
            if (!root) return false;

            if (culledPlayer !== root) {
                culledPlayer?.classList.remove('moth-kick-player-root');
                culledPlayer = root;
                culledPlayer.classList.add('moth-kick-player-root');
            }

            document.body?.classList.add('moth-kick-low-resource-active');

            return true;
        };

        const playerKey = (video) => {
            const src = (video.currentSrc || video.src || '').split('?')[0];
            return location.pathname + '::' + (src || 'no-src');
        };

        const findKickSettingsButton = (video) => {
            if (!video) return null;

            const rect = video.getBoundingClientRect();
            const center = (node) => {
                const r = node.getBoundingClientRect();
                return {
                    x: r.left + r.width / 2,
                    y: r.top + r.height / 2
                };
            };
            const overVideo = (node) => {
                const p = center(node);
                return (
                    p.x >= rect.left &&
                    p.x <= rect.right &&
                    p.y >= rect.top &&
                    p.y <= rect.bottom + 70
                );
            };

            const explicit = [...document.querySelectorAll(
                'button[aria-haspopup="menu"]'
            )].find(button =>
                /settings/i.test(
                    button.getAttribute('aria-label') || ''
                ) && overVideo(button)
            );

            if (explicit) return explicit;

            const menuish = [...document.querySelectorAll(
                'button[aria-haspopup="menu"]'
            )]
                .filter(overVideo)
                .filter(button => {
                    const aria = button.getAttribute('aria-label') || '';
                    return !/profile|account|avatar|user/i.test(aria);
                })
                .sort((a, b) => {
                    const ar = center(a);
                    const br = center(b);
                    return (
                        Math.hypot(rect.right - ar.x, rect.bottom - ar.y) -
                        Math.hypot(rect.right - br.x, rect.bottom - br.y)
                    );
                });

            return menuish[0] || null;
        };

        const chooseLowestKickQuality = async (video) => {
            if (!video || qualityBusy) return;
            if (Date.now() - lastQualityAttempt < 15000) return;

            const key = playerKey(video);
            if (!key || key === qualityDoneKey) return;

            const gear = findKickSettingsButton(video);
            if (!gear) return;

            lastQualityAttempt = Date.now();
            qualityBusy = true;

            try {
                simulateClick(gear);
                await sleep(300);

                const menuItems = [...document.querySelectorAll(
                    '[role="menuitemradio"]'
                )];

                if (!menuItems.length) return;

                const videoQualities = menuItems
                    .map((item, index) => ({
                        item,
                        index,
                        label: textOf(item),
                        height: numericQuality(textOf(item))
                    }))
                    .filter(entry => Number.isFinite(entry.height));

                if (!videoQualities.length) return;

                videoQualities.sort(
                    (a, b) =>
                        a.height - b.height ||
                        a.index - b.index
                );

                const target =
                    videoQualities.find(entry => entry.height === 160)?.item ||
                    videoQualities[0]?.item;

                if (!target) return;

                simulateClick(target);
                await sleep(350);

                qualityDoneKey = key;
                console.info('[Moth] KICK lowest video quality:', textOf(target));
            } catch (error) {
                console.warn('[Moth] KICK quality selection failed:', error);
            } finally {
                closeKickMenu();
                qualityBusy = false;
            }
        };

        const keepKickWatching = (video) => {
            if (!video) return;

            try {
                video.muted = true;
                video.defaultMuted = true;
                video.volume = 0;
                video.playsInline = true;

                if (video.paused && !video.ended) {
                    const promise = video.play();
                    if (promise?.catch) {
                        promise.catch(() => {});
                    }
                }
            } catch (_) {}
        };

        const tick = () => {
            addKickCss();

            if (location.href !== lastUrl) {
                lastUrl = location.href;
                lastVideoKey = '';
                qualityDoneKey = '';
                lastQualityAttempt = 0;
            }

            const video = findMainVideo();
            if (!video) return;

            lastVideoKey = playerKey(video);
            keepKickWatching(video);
            cullKickPage(video);
            void chooseLowestKickQuality(video);
        };

        addKickCss();
        setInterval(tick, 5000);
        tick();

        console.info('[Moth] KICK low-resource watch mode active · real player retained for Channel Points');
    }
})();
