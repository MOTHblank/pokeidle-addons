// A ponte da Shiny WishList. Os scripts que leem o jogo rodam dentro da página (world: MAIN), e a CSP dela
// (connect-src) barraria um fetch deles para o nosso site. Este roda no mundo isolado, fora dessa regra, e só busca
// um endereço fixo: a lista pública, sem cookie. É dado, não código: PIN novo não pede versão nova da extensão.
(function () {
  'use strict';
  const URL_LISTA = 'https://pokeidle-gg.pages.dev/api/wishlist';

  addEventListener('message', async (ev) => {
    if (ev.source !== window || ev.data?.tipo !== 'pih:wishlist:pedir') return;
    const responder = (conteudo) => postMessage({ tipo: 'pih:wishlist:lista', id: ev.data.id, ...conteudo }, location.origin);
    try {
      const r = await fetch(URL_LISTA, { credentials: 'omit', cache: 'no-store', signal: AbortSignal.timeout(10000) });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      responder({ dados: await r.json() });
    } catch (e) {
      responder({ erro: String(e?.message ?? e) });
    }
  });
})();
