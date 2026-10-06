(() => {
  'use strict';

  const marker = '__idleshell_gm_bridge_v1__';

  window.addEventListener('message', (event) => {
    if (event.source !== window) return;

    const msg = event.data;
    if (!msg || msg.__idleshell !== marker || msg.kind !== 'xhr') return;

    try {
      chrome.runtime.sendMessage(
        { kind: 'gm-xhr', id: msg.id, details: msg.details },
        (response) => {
          const error = chrome.runtime.lastError;
          window.postMessage({
            __idleshell: marker,
            kind: 'xhr-response',
            id: msg.id,
            response: error
              ? { ok: false, error: String(error.message || error) }
              : (response || { ok: false, error: 'No response from userscript bridge' })
          }, '*');
        }
      );
    } catch (error) {
      window.postMessage({
        __idleshell: marker,
        kind: 'xhr-response',
        id: msg.id,
        response: { ok: false, error: String(error?.message || error) }
      }, '*');
    }
  }, true);
})();
