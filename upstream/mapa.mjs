
const SEM_CACHE = { cache: 'no-cache' }; // ver a nota em app.js

// Prepara o mapa de tiles de uma hunt para ser desenhado ao vivo, com a câmera andando.
//
// O sprite-lab renderiza mapa inteiro num canvas gigante (public/mapview.mjs). Aqui é outro
// problema: a câmera segue o herói, e os pokémon precisam passar ATRÁS das árvores. Então o
// mapa vira duas coisas:
//
//   · `chao`  — um canvas único, pintado uma vez, com os andares de baixo e a banda de chão
//               do andar da hunt. Nada disso pode cobrir um pokémon, então pode ser um blit só.
//   · `ops`   — a banda de itens (árvores, pedras, paredes) e os andares de cima, cada sprite
//               com a sua profundidade. Estes o campo intercala com os pokémon a cada quadro.
//
// A profundidade é a do cliente deles: `ez(x,y) = (x+y)*4096 + 4x`, com a banda de itens
// somando 8e6 e cada andar somando 1e9 (era um container por andar, empilhado em ordem).
// A ordem de desenho DENTRO de uma tile também é a deles (chão → bordas → bottom → resto →
// top), que é o que faz a beirada de grama e a sombra da árvore caírem no lugar certo.
//
// DE ONDE SAEM OS PIXELS. O tileset deles são 30 páginas de 2046×2040 — 16 MB cada uma depois
// de decodificada. Desenhar direto delas a cada quadro era o lag das cidades: perto da cidade da
// hunt do Eevee um quadro pedia 24 páginas (~400 MB), muito acima do cache de imagens que o
// Chrome reserva a um canvas, e ele redecodificava as páginas A CADA QUADRO — 708 ms por quadro
// numa RTX 5070, contra 7 ms na do Pidgey, que só usa 3 páginas. Hoje as páginas só servem para
// a montagem: cada recorte que o mapa usa é copiado para as FOLHAS do mapa (uma ou duas
// texturas pequenas, ver `empacotar`), e daí em diante tudo sai delas.
//
// E os andares de cima (o telhado, o alto do morro) não se intercalam com ninguém — o pokémon
// anda no andar da hunt. Desenhar sprite a sprite eram até 15 mil `drawImage` por quadro numa
// montanha; eles vêm assados em pedaços (ver `montarTeto`).

const PACK = '/assets/asset-packs';
const localize = (p) => p.replace(/^\/assets-packs/, PACK);

// Itens que o cliente nunca desenha (marcadores invisíveis, bordas de edição).
const OCULTOS = new Set([7124, 1510, 8274, 46638, 46639, 46620, 46621, 1511, 1024]);

/** Chave de profundidade de uma tile. Idêntica à do bundle deles. */
export const ez = (x, y) => (x + y) * 4096 + 4 * x;

/** Acima disto é banda de item: intercala com os pokémon em vez de ir para o `chao`. */
export const BANDA_ITEM = 8e6;
export const POR_ANDAR = 1e9;

/** Quantas tiles de cenário além da área andável entram no canvas (a câmera mostra ~12). */
const MARGEM_TILES = 12;
/** Folga em px para sprites altos (árvore de 96px sobe para fora da tile dela). */
const FOLGA = 128;
/** Teto do canvas de chão; acima disto a margem encolhe em vez de estourar a memória. */
const MAX_PIXELS = 14e6;

/** Lado máximo de uma folha do mapa. 2048 é textura que qualquer GPU aceita. */
const FOLHA_LADO = 2048;
/** Vão entre os recortes na folha — o mesmo `padding` do atlas deles, para nada vazar do vizinho. */
const FOLHA_VAO = 2;
/**
 * Páginas do atlas decodificadas AO MESMO TEMPO na montagem. Cada uma são 16 MB de pixels; o
 * mapa da cidade pede 24, e decodificar todas juntas seria um pico de 400 MB num celular.
 */
const PAGINAS_POR_VEZ = 4;
/** Lado do pedaço do teto assado, em px de mundo (1 MB cada). */
const PEDACO = 512;
/** Piso de pedaços guardados por mapa, para o zoom mais fechado (ver `montarTeto`). */
const MIN_PEDACOS = 16;
/**
 * Quanto de um quadro pode ir para assar pedaços de teto, em ms. Um estado de telhado novo pede
 * a tela inteira de uma vez — 30 ms numa montanha, no desktop —, e o que não couber aqui sai
 * sprite a sprite neste quadro e fica para o próximo.
 */
const ORCAMENTO_ASSAR_MS = 3;

let packPromise = null;
let tabelasPromise = null;
const cacheMapa = new Map();

const carregarImagem = (src) =>
  new Promise((ok, err) => {
    const img = new Image();
    img.onload = () => ok(img);
    img.onerror = () => err(new Error(`falhou: ${src}`));
    img.src = src;
  });

/** Manifest do atlas de tiles (10.338 assets em 30 páginas .webp). */
function carregarPack() {
  return (packPromise ??= (async () => {
    const version = await fetch(`${PACK}/version.json`).then((r) => r.json());
    const index = await fetch(localize(version.index)).then((r) => r.json());
    const cat = Object.values(index.categories)[0];
    const manifest = await fetch(localize(cat.manifest)).then((r) => r.json());
    const pages = Object.values(manifest.categories)[0].pages.map((p) => localize(p.image));
    return { assets: manifest.assets, pages };
  })());
}

/** offsets.json (deslocamento/elevação), collision.json e draworder.json. */
function carregarTabelas() {
  return (tabelasPromise ??= (async () => {
    const [offsets, collision, draworder] = await Promise.all([
      fetch('/assets/world/offsets.json', SEM_CACHE).then((r) => r.json()),
      fetch('/assets/world/collision.json', SEM_CACHE).then((r) => r.json()),
      fetch('/assets/world/draworder.json', SEM_CACHE).then((r) => r.json()),
    ]);
    return {
      disp: offsets.disp ?? {},
      elev: offsets.elev ?? {},
      bloqueia: new Set(collision.blocking ?? []),
      top: new Set(draworder.top ?? []),
      bottom: new Set(draworder.bottom ?? []),
      borda: new Set(draworder.borders ?? []),
    };
  })());
}

/**
 * Uma página do atlas, decodificada e pronta para recortar.
 *
 * `createImageBitmap` em cima do arquivo decodifica FORA da thread principal e devolve os pixels
 * prontos — um `<img>` decodifica no meio do `drawImage`, travando a montagem. Navegador sem ele
 * (ou que recuse o webp por esse caminho) cai no `<img>` de sempre.
 */
async function decodificarPagina(url, arquivo) {
  if (typeof createImageBitmap === 'function') {
    try {
      return await createImageBitmap(await arquivo);
    } catch {
      // segue para o <img>
    }
  }
  return carregarImagem(url);
}

/**
 * Empacota os recortes nas FOLHAS do mapa: prateleiras, do recorte mais alto para o mais baixo.
 *
 * Escreve em cada recorte a folha e a posição (`folha`, `fx`, `fy`) e devolve o tamanho de cada
 * folha. Quase todo recorte tem 32, 64 ou 128 px, então as prateleiras quase não perdem espaço: a
 * mediana dos mapas cabe em 1 M px (uma folha de 2048×~600) e o maior, Cerulean, em duas.
 */
function empacotar(recortes) {
  const folhas = [];
  let atual = null;
  let x = 0;
  let y = 0;
  let prateleira = 0;
  const abrir = () => {
    atual = { larg: 0, alt: 0 };
    folhas.push(atual);
    x = 0;
    y = 0;
    prateleira = 0;
  };
  for (const r of [...recortes].sort((a, b) => b.h - a.h || b.w - a.w)) {
    if (!atual) abrir();
    if (x + r.w > FOLHA_LADO) {
      y += prateleira + FOLHA_VAO;
      x = 0;
      prateleira = 0;
    }
    if (y + r.h > FOLHA_LADO) abrir();
    r.folha = folhas.length - 1;
    r.fx = x;
    r.fy = y;
    x += r.w + FOLHA_VAO;
    prateleira = Math.max(prateleira, r.h);
    atual.larg = Math.max(atual.larg, r.fx + r.w);
    atual.alt = Math.max(atual.alt, r.fy + r.h);
  }
  return folhas;
}

/**
 * Os ANDARES DE CIMA, assados em pedaços de `PEDACO` px.
 *
 * Tudo o que fica acima do andar da hunt é desenhado depois de todos os pokémon (a profundidade
 * soma 1e9 por andar), então nada precisa se intercalar com eles — dá para assar. O que impede
 * de assar tudo uma vez só é o TELHADO: o herói entra debaixo dele e os andares de cima apagam
 * (`primeiroAndarVisivel` + o fade do campo). Por isso a montagem é preguiçosa e por estado:
 *
 *   · os andares com alfa 1 — sempre um bloco contíguo a partir do de baixo, `[zBase, groundZ)` —
 *     saem de pedaços assados para aquele `zBase`, montados na primeira vez que entram na tela;
 *   · os de cima dele que ainda aparecem são os que estão no meio do fade, e vão sprite a sprite
 *     com o alfa deles, como antes — só durante os 180 ms da transição.
 *
 * Os pedaços ficam num LRU do tamanho de duas telas: quem entra e sai de uma casa reaproveita os
 * dois estados, e a memória não cresce com o tamanho do mapa. Assar tem orçamento por quadro
 * (`ORCAMENTO_ASSAR_MS`): o pedaço que fica para depois é desenhado sprite a sprite, recortado no
 * retângulo dele — sem o recorte, o sprite que atravessa a divisa sairia duas vezes no vizinho
 * já assado.
 *
 * `ops` vem na ordem de desenho, que é andar a andar do de baixo para o de cima — então, num
 * pedaço, os sprites dos andares `>= zBase` são um prefixo da lista dele.
 */
function montarTeto(ops, { ox, oy, larg, alt, groundZ, minZ }) {
  const colunas = Math.ceil(larg / PEDACO);
  const linhas = Math.ceil(alt / PEDACO);
  const coluna = (px) => Math.floor((px - ox) / PEDACO);
  const linha = (py) => Math.floor((py - oy) / PEDACO);

  // Cada sprite entra em todo pedaço que ele toca, na ordem de desenho.
  const porPedaco = new Array(colunas * linhas);
  for (const o of ops) {
    const c0 = Math.max(0, coluna(o.x));
    const c1 = Math.min(colunas - 1, coluna(o.x + o.w - 1));
    const l0 = Math.max(0, linha(o.y));
    const l1 = Math.min(linhas - 1, linha(o.y + o.h - 1));
    for (let l = l0; l <= l1; l++) for (let c = c0; c <= c1; c++) (porPedaco[l * colunas + c] ??= []).push(o);
  }

  // Onde cada andar começa e termina na lista — o fade pula um andar apagado inteiro de uma vez.
  const andares = [];
  for (let k = 0; k < ops.length; k++) {
    const ult = andares[andares.length - 1];
    if (ult?.z === ops[k].z) ult.fim = k + 1;
    else andares.push({ z: ops[k].z, ini: k, fim: k + 1 });
  }

  /** `${zBase}:${pedaço}` → canvas, ou `null` quando aquele pedaço não tem nada naquele estado. */
  const assados = new Map();

  const assar = (zBase, idx) => {
    const lista = porPedaco[idx];
    let n = 0;
    while (n < lista.length && lista[n].z >= zBase) n++;
    if (!n) return null;
    const c = idx % colunas;
    const l = (idx - c) / colunas;
    const bx = ox + c * PEDACO;
    const by = oy + l * PEDACO;
    const cv = document.createElement('canvas');
    cv.width = Math.min(PEDACO, larg - c * PEDACO);
    cv.height = Math.min(PEDACO, alt - l * PEDACO);
    const g = cv.getContext('2d');
    g.imageSmoothingEnabled = false;
    for (let k = 0; k < n; k++) {
      const o = lista[k];
      g.drawImage(o.img, o.sx, o.sy, o.w, o.h, o.x - bx, o.y - by, o.w, o.h);
    }
    return cv;
  };

  /** O pedaço que não coube no orçamento: o mesmo conteúdo do `assar`, direto na tela. */
  const semAssar = (ctx, zBase, idx, x0, y0, x1, y1) => {
    const c = idx % colunas;
    const l = (idx - c) / colunas;
    ctx.save();
    ctx.beginPath();
    ctx.rect(ox + c * PEDACO, oy + l * PEDACO, Math.min(PEDACO, larg - c * PEDACO), Math.min(PEDACO, alt - l * PEDACO));
    ctx.clip();
    for (const o of porPedaco[idx]) {
      if (o.z < zBase) break;
      if (o.x > x1 || o.y > y1 || o.x + o.w < x0 || o.y + o.h < y0) continue;
      ctx.drawImage(o.img, o.sx, o.sy, o.w, o.h, o.x, o.y, o.w, o.h);
    }
    ctx.restore();
  };

  return {
    /** Desenha os andares de cima na janela `x0..x1 × y0..y1` (px de mundo), com o alfa de cada andar. */
    desenhar(ctx, x0, y0, x1, y1, alfaAndar) {
      let zBase = groundZ;
      while (zBase > minZ && (alfaAndar.get(zBase - 1) ?? 1) >= 1) zBase--;

      if (zBase < groundZ) {
        const c0 = Math.max(0, coluna(x0));
        const c1 = Math.min(colunas - 1, coluna(x1));
        const l0 = Math.max(0, linha(y0));
        const l1 = Math.min(linhas - 1, linha(y1));
        // pelo menos um pedaço por quadro, para a fila andar mesmo numa máquina lenta
        const prazo = performance.now() + ORCAMENTO_ASSAR_MS;
        let assou = false;
        for (let l = l0; l <= l1; l++) {
          for (let c = c0; c <= c1; c++) {
            const idx = l * colunas + c;
            if (!porPedaco[idx]) continue;
            const chave = `${zBase}:${idx}`;
            let cv = assados.get(chave);
            if (cv === undefined) {
              if (assou && performance.now() > prazo) {
                semAssar(ctx, zBase, idx, x0, y0, x1, y1);
                continue;
              }
              cv = assar(zBase, idx);
              assou = true;
            } else assados.delete(chave);
            assados.set(chave, cv); // o recém-usado vai para o fim da fila
            if (cv) ctx.drawImage(cv, ox + c * PEDACO, oy + l * PEDACO);
          }
        }
        const limite = Math.max(MIN_PEDACOS, 2 * (c1 - c0 + 1) * (l1 - l0 + 1));
        while (assados.size > limite) {
          const [chaveVelha, velho] = assados.entries().next().value;
          assados.delete(chaveVelha);
          // Zerar devolve a memória na hora — o Safari conta o canvas no teto dele até o GC passar.
          if (velho) velho.width = velho.height = 0;
        }
      }

      let alfa = 1;
      for (const a of andares) {
        if (a.z >= zBase) continue;
        const v = alfaAndar.get(a.z) ?? 1;
        if (v <= 0.002) continue;
        if (v !== alfa) ctx.globalAlpha = alfa = v;
        for (let k = a.ini; k < a.fim; k++) {
          const o = ops[k];
          if (o.x > x1 || o.y > y1 || o.x + o.w < x0 || o.y + o.h < y0) continue;
          ctx.drawImage(o.img, o.sx, o.sy, o.w, o.h, o.x, o.y, o.w, o.h);
        }
      }
      if (alfa !== 1) ctx.globalAlpha = 1;
    },
  };
}

// Sprites com patternX/patternY variam conforme a coordenada no mundo — é o que faz uma
// mesma grama render texturas diferentes lado a lado.
function quadroDe(a, px, py, centro) {
  if (!a.patternX && !a.patternY) return 0;
  const pX = a.patternX || 1;
  const pY = a.patternY || 1;
  const animLen = a.animLen || 1;
  const cx = centro[0] + Math.round(px / 32);
  const cy = centro[1] + Math.round(py / 32);
  const idx = ((((cy % pY) + pY) % pY) * pX + (((cx % pX) + pX) % pX)) * animLen;
  return idx < a.frames.length ? idx : 0;
}

/**
 * Monta o mapa da hunt.
 *
 * @param slug  hunt
 * @param caixa {minTx, minTy, cols, rows, groundZ} — a área andável que o servidor mandou
 * @returns {{chao, ox, oy, ops, limites}}  `ox/oy` = px de mundo do canto do canvas de chão
 */
/**
 * Larga os mapas que estão em memória.
 *
 * Só o MODO ECONOMIA chama. Cada mapa montado é um canvas de dezenas de MB (ver o comentário
 * no fim de `prepararMapa`), mais as folhas e os pedaços de teto dele — tudo isso existe para
 * desenhar a cena, e mais nada no jogo lê este cache. Com a cena desligada, é a maior mordida de
 * memória que dá para devolver ao navegador. As páginas do atlas nem ficam: cada montagem
 * decodifica as que precisa e as solta no fim.
 *
 * Não custa download na volta: o `prepararMapa` refaz o canvas a partir dos arquivos, que o
 * navegador já tem no cache de HTTP. O que se paga é a remontagem, e ela já acontece de
 * qualquer jeito a cada troca de área.
 */
export function soltarMapas() {
  cacheMapa.clear();
}

/**
 * Uma cópia TINGIDA de um pedaço de imagem: o matiz e a saturação da cor por cima, a luz do
 * original por baixo (o modo `color` do canvas), e o recorte da transparência preservado — sem o
 * `destination-in`, o `color` pintaria também os pixels vazios em volta do sprite.
 */
function tingir(fonte, sx, sy, w, h, { cor, forca }) {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const x = c.getContext('2d');
  x.imageSmoothingEnabled = false;
  x.drawImage(fonte, sx, sy, w, h, 0, 0, w, h);
  x.globalCompositeOperation = 'color';
  x.globalAlpha = forca;
  x.fillStyle = cor;
  x.fillRect(0, 0, w, h);
  x.globalAlpha = 1;
  x.globalCompositeOperation = 'destination-in';
  x.drawImage(fonte, sx, sy, w, h, 0, 0, w, h);
  return c;
}

/**
 * `opcoes.tinta` (`{ cor, forca }`) tinge o mapa INTEIRO — o chão e cada sprite do cenário — uma
 * vez, na montagem. É o roxo da Arena Mística: o mapa servido é o mesmo de sempre, e as criaturas
 * (que não são do mapa) continuam na cor delas. Os sprites saem das folhas do mapa, então basta
 * tingir as folhas: a conta do `color` é pixel a pixel, e o vão entre os recortes impede que um
 * contamine o outro.
 */
export async function prepararMapa(slug, caixa, onProgress, opcoes = {}) {
  const tinta = opcoes.tinta ?? null;
  const chave = `${slug}:${caixa.minTx},${caixa.minTy},${caixa.cols},${caixa.rows}${tinta ? `:${tinta.cor}:${tinta.forca}` : ''}`;
  if (cacheMapa.has(chave)) return cacheMapa.get(chave);

  const p = (async () => {
    onProgress?.('carregando o mapa…');
    const [{ assets, pages }, tab, mapa] = await Promise.all([
      carregarPack(),
      carregarTabelas(),
      fetch(`/assets/world/maps/${slug}.json`).then((r) => {
        if (!r.ok) throw new Error(`mapa "${slug}" não está no espelho`);
        return r.json();
      }),
    ]);

    const meta = mapa._meta ?? {};
    const centro = meta.center ?? [0, 0];
    const groundZ = caixa.groundZ ?? meta.groundZ ?? 7;

    // Recorte: a área andável mais uma faixa de cenário em volta.
    let margem = MARGEM_TILES;
    const dims = (m) => ({
      larg: (caixa.cols + 2 * m) * 32 + 2 * FOLGA,
      alt: (caixa.rows + 2 * m) * 32 + 2 * FOLGA,
    });
    while (margem > 2 && dims(margem).larg * dims(margem).alt > MAX_PIXELS) margem -= 2;

    const x0 = caixa.minTx - margem;
    const y0 = caixa.minTy - margem;
    const x1 = caixa.minTx + caixa.cols - 1 + margem;
    const y1 = caixa.minTy + caixa.rows - 1 + margem;
    const { larg, alt } = dims(margem);
    const ox = x0 * 32 - FOLGA;
    const oy = y0 * 32 - FOLGA;

    const tiles = (mapa.tiles ?? []).filter((t) => t[0] >= x0 && t[0] <= x1 && t[1] >= y0 && t[1] <= y1);

    // Índice de COBERTURA: quais tiles de andar de cima têm chão próprio.
    //
    // Sem isto, numa caverna, num telhado ou dentro da pirâmide do Abra o andar de cima é
    // desenhado por último e some com o mapa inteiro. O cliente deles resolve escondendo os
    // andares acima do jogador quando algum deles o cobre — é o `computeFirstVisibleFloor`.
    // Varre o mapa todo (não só o recorte) porque a checagem olha para (tx+n, ty+n).
    let minZ = groundZ;
    const cobre = new Set();
    for (const t of mapa.tiles ?? []) {
      if (t[2] < minZ) minZ = t[2];
      if (t[2] < groundZ && t[3]) cobre.add(`${t[0]},${t[1]},${t[2]}`);
    }

    // ---------------------------------------------------- lista de sprites
    //
    // Uma passada só: cada sprite vira uma op com a sua profundidade. Depois a lista é
    // ordenada e cortada em duas — o que fica abaixo dos pokémon vai para o canvas de chão.
    //
    // Cada op aponta para o RECORTE do atlas que desenha — um por quadro do manifest, então quem
    // usa o mesmo quadro divide o recorte. É a lista deles (algumas centenas, contra milhares de
    // ops) que vai para as folhas do mapa.
    const ops = [];
    const recortes = new Map();

    const colocar = (id, px, py, elevAcum, prof, z) => {
      const a = assets[String(id)];
      if (!a) return 0; // 1.116 ids não existem no atlas; o cliente deles também pula
      const f = a.frames[quadroDe(a, px, py, centro)];
      const d = tab.disp[id];
      let r = recortes.get(f);
      if (!r) recortes.set(f, (r = { pagina: f.page, sx: f.x, sy: f.y, w: f.w, h: f.h }));
      ops.push({
        prof,
        z,
        r,
        w: f.w,
        h: f.h,
        x: px - (a.width - 32) - (d ? d[0] : 0),
        y: py - (a.height - 32) - (d ? d[1] : 0) - elevAcum,
      });
      return tab.elev[id] ?? 0;
    };

    // Porte fiel do buildTile deles: banda (chão/item), sub-ordem e elevação acumulada.
    const montarTile = (t) => {
      const off = (t[2] - groundZ) * 32;
      const px = 32 * t[0] + off;
      const py = 32 * t[1] + off;
      const base = ez(t[0], t[1]) + (groundZ - t[2]) * POR_ANDAR;

      let elev = 0;
      let ultima = base - 1;

      const por = (id) => {
        if (OCULTOS.has(id)) return;
        const a = assets[String(id)];
        const chao = a?.isGround === true;
        const banda = chao || (tab.borda.has(id) && !tab.top.has(id)) ? 'g' : 'i';
        const sub = chao ? -1 : tab.top.has(id) ? 0.6 : tab.bottom.has(id) ? 0 : 0.3;
        const prof = Math.max(base + sub + (banda === 'g' ? 0 : BANDA_ITEM), ultima + 0.001);
        ultima = prof;
        elev = Math.min(32, elev + colocar(id, px, py, elev, prof, t[2]));
      };

      const ehTop = (id) => tab.top.has(id);
      const ehBottom = (id) => tab.bottom.has(id) && !ehTop(id);
      const ehChao = (id) => assets[String(id)]?.isGround === true;
      const ehBorda = (id) => tab.borda.has(id) && !ehTop(id) && !ehChao(id);
      const bottomPuro = (id) => ehBottom(id) && !ehBorda(id) && !ehChao(id);
      const meio = (id) => !ehTop(id) && !ehBottom(id);
      const alto = (id) => tab.bloqueia.has(id) || (tab.elev[id] ?? 0) > 0;
      const invertidos = t[4].length > 1 ? [...t[4]].reverse() : t[4];

      if (t[3]) por(t[3]);
      for (const it of t[4]) if (ehBottom(it[0]) && ehChao(it[0])) por(it[0]);
      for (const it of invertidos) if (ehBorda(it[0])) por(it[0]);
      for (const it of t[4]) if (bottomPuro(it[0])) por(it[0]);
      for (const it of t[4]) if (meio(it[0]) && alto(it[0])) por(it[0]);
      for (const it of t[4]) if (meio(it[0]) && !alto(it[0])) por(it[0]);
      for (const it of invertidos) if (ehTop(it[0])) por(it[0]);
    };

    for (const t of tiles) montarTile(t);
    ops.sort((a, b) => a.prof - b.prof);

    // ------------------------------------------------------- folhas do mapa
    //
    // Os recortes vão para folhas próprias, página por página do atlas: só `PAGINAS_POR_VEZ`
    // decodificadas de cada vez, e cada uma é solta assim que os recortes dela foram copiados.
    // Os arquivos baixam todos juntos (são ~1 MB cada); o que se escalona é a decodificação.
    const folhas = empacotar(recortes.values()).map(({ larg: w, alt: h }) => {
      const cv = document.createElement('canvas');
      cv.width = w;
      cv.height = h;
      const g = cv.getContext('2d');
      g.imageSmoothingEnabled = false;
      return { cv, g };
    });
    const porPagina = new Map();
    for (const r of recortes.values()) {
      if (!porPagina.has(r.pagina)) porPagina.set(r.pagina, []);
      porPagina.get(r.pagina).push(r);
    }
    const lista = [...porPagina.keys()];
    const arquivos = new Map();
    if (typeof createImageBitmap === 'function') {
      for (const i of lista) {
        const arq = fetch(pages[i]).then((r) => {
          if (!r.ok) throw new Error(`falhou: ${pages[i]}`);
          return r.blob();
        });
        arq.catch(() => {}); // quem dá o `await` é o lote dela; até lá, a falha não é "não tratada"
        arquivos.set(i, arq);
      }
    }
    for (let k = 0; k < lista.length; k += PAGINAS_POR_VEZ) {
      onProgress?.(`páginas do atlas: ${k}/${lista.length}…`);
      const lote = lista.slice(k, k + PAGINAS_POR_VEZ);
      const imgs = await Promise.all(lote.map((i) => decodificarPagina(pages[i], arquivos.get(i))));
      lote.forEach((i, j) => {
        for (const r of porPagina.get(i)) folhas[r.folha].g.drawImage(imgs[j], r.sx, r.sy, r.w, r.h, r.fx, r.fy, r.w, r.h);
        imgs[j].close?.();
      });
    }

    onProgress?.('montando o cenário…');
    for (const o of ops) {
      o.img = folhas[o.r.folha].cv;
      o.sx = o.r.fx;
      o.sy = o.r.fy;
    }

    // ------------------------------------------------------- assar o chão
    const chao = document.createElement('canvas');
    chao.width = larg;
    chao.height = alt;
    const ctx = chao.getContext('2d');
    ctx.imageSmoothingEnabled = false;

    // Tudo que tem profundidade abaixo da banda de itens do andar da hunt é cenário de fundo:
    // andares de baixo inteiros e a banda de chão daqui. Vira um blit só por quadro.
    let corte = 0;
    while (corte < ops.length && ops[corte].prof < BANDA_ITEM) corte++;
    for (let i = 0; i < corte; i++) {
      const o = ops[i];
      ctx.drawImage(o.img, o.sx, o.sy, o.w, o.h, o.x - ox, o.y - oy, o.w, o.h);
    }

    const dinamicas = ops.slice(corte);

    let chaoFinal = chao;
    if (tinta) {
      chaoFinal = tingir(chao, 0, 0, larg, alt, tinta);
      const tingidas = new Map(folhas.map(({ cv }) => [cv, tingir(cv, 0, 0, cv.width, cv.height, tinta)]));
      for (const o of dinamicas) o.img = tingidas.get(o.img);
    }

    // A banda de itens do andar da hunt vem primeiro na lista e é a única que se intercala com os
    // pokémon; dali em diante é andar de cima (a profundidade soma 1e9 por andar), que vai assado.
    let nBanda = 0;
    while (nBanda < dinamicas.length && dinamicas[nBanda].z >= groundZ) nBanda++;
    const teto =
      nBanda < dinamicas.length ? montarTeto(dinamicas.slice(nBanda), { ox, oy, larg, alt, groundZ, minZ }) : null;

    return {
      slug,
      chao: chaoFinal,
      ox,
      oy,
      larg,
      alt,
      groundZ,
      minZ,
      /**
       * Os sprites que não foram para o chão, na ordem de desenho: `ops[0..nBanda)` é a banda de
       * itens do andar da hunt, e o resto são os andares de cima — que o `teto` desenha assados.
       * A lista inteira continua aqui para a casa com escada, onde o pokémon anda no andar de
       * cima e precisa se intercalar com ele.
       */
      ops: dinamicas,
      nBanda,
      teto,
      // limites da câmera, em px de mundo
      limites: { x0: x0 * 32, y0: y0 * 32, x1: (x1 + 1) * 32, y1: (y1 + 1) * 32 },

      /**
       * Andar mais alto que ainda deve ser desenhado com o herói em (tx, ty).
       *
       * Idêntico ao `computeFirstVisibleFloor` deles: sobe andar a andar a partir do de cima;
       * o primeiro que tiver chão em (tx+n, ty+n) está cobrindo o herói, e a partir dali
       * (inclusive) tudo some. Sem cobertura nenhuma, desenha até o topo.
       */
      primeiroAndarVisivel(tx, ty) {
        for (let z = groundZ - 1; z >= minZ; z--) {
          const n = groundZ - z;
          if (cobre.has(`${tx + n},${ty + n},${z}`)) return z + 1;
        }
        return minZ;
      },
    };
  })();

  cacheMapa.set(chave, p);
  // um mapa que falhou não fica envenenando o cache
  p.catch(() => cacheMapa.delete(chave));
  // só guardamos dois mapas: cada um tem um canvas de dezenas de MB
  if (cacheMapa.size > 2) cacheMapa.delete(cacheMapa.keys().next().value);
  return p;
}
