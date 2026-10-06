// ==UserScript==
// @name         IdleShell Link Router (pokeidle.io)
// @namespace    moth.pokeidle
// @version      1.0.0
// @description  Intercepts twitch.tv / kick.com links inside the PokéIdle game page and hands them to the IdleShell host, which opens them in background stream panes on every Twitch/Kick account profile instead of a popup window.
// @match        https://pokeidle.io/*
// @match        https://www.pokeidle.io/*
// @grant        unsafeWindow
// @run-at       document-start
// ==/UserScript==

/*
 * This script runs inside the GAME pane (profile AccountA / AccountB).
 * The shell injects a small bootstrap into every isolated world via
 * AddScriptToExecuteOnDocumentCreatedAsync, which defines:
 *
 *     window.__idleshell_openLink   — routes a URL to the shell host
 *     window.__idleshell_hostInfo   — { title, profile } of this pane
 *
 * If those are missing (plain browser), everything below is inert.
 *
 * Strategy: never let a stream URL reach window.open() for real — WebView2
 * would raise NewWindowRequested and spawn an uncontrollable OS popup.
 * Instead we route the URL to the shell over window.chrome.webview.postMessage,
 * and the shell opens (or reuses) hidden Background-mode stream panes, one per
 * configured Twitch/Kick login profile.
 */

(() => {
    'use strict';

    const HOST = typeof unsafeWindow !== 'undefined' ? unsafeWindow : window;

    // Same hosts the shell considers "stream" URLs (StreamHostPattern in AppConfig.cs).
    const STREAM_URL_RE =
        /^https?:\/\/(?:www\.|m\.)?(?:twitch\.tv|kick\.com)\/[^\s"'<>]*/i;

    const post = (payload) => {
        try {
            // Prefer the host function injected by IdleShell itself. This avoids
            // depending on whether chrome.webview is exposed to an extension
            // content script running in the page's MAIN world.
            if (typeof HOST.__idleshell_openLink === 'function')
                return HOST.__idleshell_openLink(payload.url, payload.source || 'userscript');
        } catch (_) {}

        try {
            if (HOST.chrome?.webview &&
                typeof HOST.chrome.webview.postMessage === 'function') {
                HOST.chrome.webview.postMessage(JSON.stringify(payload));
                return true;
            }
        } catch (_) {}
        return false;
    };

    const info = () => {
        try {
            return HOST.__idleshell_hostInfo || { title: location.host, profile: 'unknown' };
        } catch (_) {
            return { title: location.host, profile: 'unknown' };
        }
    };

    const route = (rawUrl, source) => {
        if (typeof rawUrl !== 'string' || !STREAM_URL_RE.test(rawUrl)) return false;
        const url = new URL(rawUrl, location.href).href;
        const h = info();
        // Only claim the event when the host actually received it. If the
        // WebView2 bridge is unavailable (for example, when this userscript
        // executes in a context that cannot see chrome.webview), return false
        // so the browser/native WebView2 fallback can handle the popup or
        // navigation normally.
        return post({
            type: 'link',
            url,
            source,
            pane: h.title,
            profile: h.profile
        });
    };

    // 1. window.open(...) calls from the game or other userscripts.
    const realOpen = HOST.open ? HOST.open.bind(HOST) : null;
    const patchedOpen = function (url, ...rest) {
        if (route(url, 'window.open')) {
            // The host owns the stream URL only after post() succeeds.
            return { closed: false, close() {}, focus() {}, blur() {}, location: { href: String(url) } };
        }
        return realOpen ? realOpen(url, ...rest) : undefined;
    };
    try { HOST.open = patchedOpen; } catch (_) {}
    try { window.open = patchedOpen; } catch (_) {}

    // 2. Anchor clicks (target=_blank or plain) anywhere in the page.
    //    Capture phase + stopPropagation so the game's own handlers don't
    //    double-handle the same click. We only suppress the click if the host
    //    accepted the route; otherwise native WebView2 popup/navigation
    //    handling remains available as the fallback.
    const elementValue = (element) => {
        try {
            for (const name of element.getAttributeNames()) {
                const value = element.getAttribute(name);
                if (value && STREAM_URL_RE.test(value))
                    return value;
            }

            const onclick = element.getAttribute('onclick');
            if (onclick) {
                const match = onclick.match(/https?:\/\/(?:www\.|m\.)?(?:twitch\.tv|kick\.com)\/[^\s"'<>)]*/i);
                if (match) return match[0];
            }
        } catch (_) {}
        return null;
    };

    const findStreamUrl = (event) => {
        try {
            const path = typeof event.composedPath === 'function'
                ? event.composedPath()
                : [event.target];

            for (const item of path) {
                if (!item || item.nodeType !== 1) continue;

                if (item.href && typeof item.href === 'string' &&
                    STREAM_URL_RE.test(item.href))
                    return item.href;

                const value = elementValue(item);
                if (value) return value;
            }
        } catch (_) {}
        return null;
    };

    const clickHandler = (e) => {
        const url = findStreamUrl(e);
        if (!url) return;

        if (route(url, e.type === 'auxclick' ? 'auxclick' : 'click')) {
            e.preventDefault();
            e.stopPropagation();
            if (typeof e.stopImmediatePropagation === 'function')
                e.stopImmediatePropagation();
        }
    };

    const bindClicks = () => {
        document.addEventListener('click', clickHandler, true);
        document.addEventListener('auxclick', clickHandler, true);
    };
    if (document.documentElement) bindClicks();
    else document.addEventListener('DOMContentLoaded', bindClicks, true);

    // 3. Direct assignments like `location.href = 'https://twitch.tv/...'`
    //    cannot be intercepted without breaking navigation generally, but the
    //    shell also watches each game pane's Source property as a backstop
    //    (see MainForm.GamePaneNavigated).
})();
