(() => {
  'use strict';

  const safeHeaders = (input) => {
    const out = {};
    if (!input || typeof input !== 'object') return out;

    for (const [key, value] of Object.entries(input)) {
      const k = String(key);
      if (/^(host|origin|referer|user-agent|content-length)$/i.test(k)) continue;
      if (/^(sec-|proxy-)/i.test(k)) continue;
      out[k] = String(value);
    }

    return out;
  };

  const headersToString = (headers) =>
    [...headers.entries()]
      .map(([key, value]) => key + ': ' + value)
      .join('\r\n');

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (!message || message.kind !== 'gm-xhr') return;

    const details = message.details || {};

    (async () => {
      const method = String(details.method || 'GET').toUpperCase();
      const url = String(details.url || '');

      if (!/^https?:$/i.test(new URL(url).protocol))
        throw new Error('GM_xmlhttpRequest only supports http(s) URLs.');

      const init = {
        method,
        headers: safeHeaders(details.headers),
        redirect: 'follow',
        credentials: details.anonymous
          ? 'omit'
          : (details.withCredentials ? 'include' : 'omit'),
        body: ['GET', 'HEAD'].includes(method)
          ? undefined
          : (details.data ?? undefined)
      };

      const controller = new AbortController();
      let timer = null;
      if (Number(details.timeout) > 0)
        timer = setTimeout(() => controller.abort(), Number(details.timeout));

      init.signal = controller.signal;

      try {
        const response = await fetch(url, init);
        const responseText = await response.text();

        sendResponse({
          ok: true,
          status: response.status,
          statusText: response.statusText,
          responseHeaders: headersToString(response.headers),
          responseText,
          finalUrl: response.url
        });
      } finally {
        if (timer !== null) clearTimeout(timer);
      }
    })().catch((error) => {
      sendResponse({
        ok: false,
        error: String(error?.message || error)
      });
    });

    return true;
  });
})();
