// A PRANCHETA DA SESSÃO de hunt, por dentro: os contadores, o que cada evento soma, o custo do que
// foi gasto e o que volta do disco. Puro — sem DOM e sem `estado` —, para o teste rodar no Node
// (`tools/teste-sessao-hunt.mjs`). O desenho mora no app.js (`pintarSessao`); o ritmo (XP/h,
// ouro/h), em `hunt-analyser.mjs`, porque o Pocket e o Hunt Analyser mostram os mesmos números.
//
// A sessão é coisa do CLIENTE: o servidor não conta nada disso. Ela soma os eventos que já chegam
// para a cena — o abate (`morte`), o boss no chão (`bossMorto`, também no boss automático), a
// bola (`bola`), a poção (`cura`) e o revive (`revive`), cada um com o id do que foi gasto.
import { MS_MINIMO_RITMO } from './hunt-analyser.mjs';
import { MYSTIC_TICKET_ID } from '../shared/mistico.mjs';

/**
 * Os DROPS RAROS que a prancheta conta, na ordem dos slots. `id` e `nome` são os do catálogo do
 * servidor (o nome é a rede para um drop que chegue sem id); o app.js ainda troca o id dos três
 * primeiros fragmentos pelo que o `welcome` mandar, como sempre fez.
 *
 *   · fragmentos de Mega Stone (`FRAGMENTO_MEGA_ID`/`_SHINY_ID`, `server/game/itens-nossos.mjs`)
 *     e as peças de TM (`PIECE_ELEMENTAL`/`PIECE_AOE`, `server/game/tm.mjs`) caem de BOSS — por
 *     isso o `bossMorto` soma drops agora, e não só o Boss Token;
 *   · o MysticTicket também é drop de boss (`shared/mistico.mjs`).
 */
export const DROPS_SESSAO = [
  { campo: 'bossTokens', id: 70000, nome: 'Bronze Boss Token' },
  { campo: 'fragChave', id: 70011, nome: 'Key Fragment' },
  { campo: 'fragShiny', id: 70012, nome: 'Shiny Stone Fragment' },
  { campo: 'fragBicicleta', id: 70013, nome: 'Bicycle Fragment' },
  { campo: 'fragMega', id: 70014, nome: 'Mega Stone Fragment' },
  { campo: 'fragMegaShiny', id: 70015, nome: 'Mega Shiny Stone Fragment' },
  { campo: 'tmPeca', id: 59194, nome: 'TM Disk Piece' },
  { campo: 'tmPecaAoe', id: 40530, nome: 'AoE TM Disk Piece' },
  { campo: 'ticketMistico', id: MYSTIC_TICKET_ID, nome: 'MysticTicket' },
];

/** Os três gastos, cada um um mapa `id → quantas` (a bola pelo `ballId`, o resto pelo `itemId`). */
export const GASTOS_SESSAO = ['bolas', 'pocoes', 'revives'];

/** Teto de tipos diferentes por mapa: o jogo tem cinco bolas e nove remédios; isto é só a cerca. */
const MAX_TIPOS = 64;

export const sessaoNova = (agora = Date.now()) => ({
  inicio: agora,
  xpTreinador: 0,
  xpPokemon: 0,
  gold: 0,
  abates: 0,
  capturas: 0,
  shiniesVistos: 0,
  shinies: 0,
  ...Object.fromEntries(DROPS_SESSAO.map((d) => [d.campo, 0])),
  ...Object.fromEntries(GASTOS_SESSAO.map((g) => [g, {}])),
});

/**
 * Um id de bola/item — do evento (número) ou de chave de mapa vinda do disco (texto só de dígitos):
 * inteiro positivo, ou `null`. Nada de `Number()` solto: ele aceita `[7]` como 7 e `''` como 0.
 */
const idValido = (v) => {
  if (typeof v === 'string' ? !/^\d{1,9}$/.test(v) : typeof v !== 'number') return null;
  const n = Number(v);
  return Number.isInteger(n) && n > 0 && n < 1e9 ? n : null;
};

/** O mapa de gasto que voltou do disco, saneado: só ids válidos e quantidades inteiras ≥ 0. */
function sanearMapa(m) {
  const limpo = {};
  if (!m || typeof m !== 'object' || Array.isArray(m)) return limpo;
  let n = 0;
  for (const [k, v] of Object.entries(m)) {
    const id = idValido(k);
    const qtd = Math.floor(Number(v));
    if (id == null || !Number.isFinite(qtd) || qtd <= 0) continue;
    limpo[id] = qtd;
    if (++n >= MAX_TIPOS) break;
  }
  return limpo;
}

/**
 * A prancheta gravada, de volta do `localStorage`. Campo a campo e sempre do tipo certo: o que
 * está no disco foi escrito por uma versão qualquer do jogo, e um contador que ainda não existia
 * lá (os drops novos, os gastos) não pode chegar aqui como `NaN` nem como `undefined`. O tempo
 * volta como DURAÇÃO (`ms`): o período com o jogo fechado não entra no relógio.
 */
export function sanearSessao(g, agora = Date.now()) {
  if (!g || typeof g !== 'object') return null;
  const s = sessaoNova(agora);
  for (const k of Object.keys(s)) {
    if (k === 'inicio') continue;
    if (GASTOS_SESSAO.includes(k)) s[k] = sanearMapa(g[k]);
    else s[k] = Math.max(0, Number(g[k]) || 0);
  }
  s.inicio = agora - Math.max(0, Number(g.ms) || 0);
  return s;
}

/** Quantas unidades de `id` (ou, na falta do id, do `nome`) vieram nos drops de um evento. */
export const somaDrops = (drops, id, nome) =>
  (Array.isArray(drops) ? drops : []).reduce(
    (t, d) => t + (d?.itemId === id || d?.nome === nome ? Number(d.qtd) || 1 : 0),
    0,
  );

function somarDrops(s, drops, ids) {
  for (const d of DROPS_SESSAO) {
    s[d.campo] = (s[d.campo] || 0) + somaDrops(drops, ids?.[d.campo] ?? d.id, d.nome);
  }
}

function somarGasto(mapa, v) {
  const id = idValido(v);
  if (id == null) return false;
  if (!(id in mapa) && Object.keys(mapa).length >= MAX_TIPOS) return false;
  mapa[id] = (mapa[id] || 0) + 1;
  return true;
}

const num = (v) => Number(v) || 0;

/**
 * Soma um evento da cena na prancheta. Devolve `true` se algum contador mudou (o app.js só grava
 * e repinta nesse caso). `ids` troca o id de um drop (`{ fragChave: 70011 }`) pelo do servidor.
 *
 * Cada bola, poção e revive chega como UM evento por unidade gasta — é assim que o servidor os
 * emite (`arremessarBola`, `usarPocao`, `usarRevive` no sim) —, então somar 1 por evento é contar
 * exatamente o que saiu da bolsa.
 */
export function contarNaSessao(s, e, ids = null) {
  if (!s || !e) return false;
  switch (e.k) {
    case 'morte':
      if (e.quem !== 'selvagem') return false;
      s.abates++;
      s.xpTreinador += num(e.xpTreinador ?? e.xp);
      s.xpPokemon += num(e.xpPokemon ?? e.xp);
      s.gold += num(e.ouro) + num(e.ouroVendaAuto);
      somarDrops(s, e.drops, ids);
      return true;
    case 'bossMorto':
      s.xpTreinador += num(e.xpTreinador ?? e.xp);
      s.xpPokemon += num(e.xpPokemon ?? e.xp);
      s.gold += num(e.valor);
      somarDrops(s, e.drops, ids);
      return true;
    case 'bola':
      if (e.shiny) s.shiniesVistos++;
      somarGasto((s.bolas ??= {}), e.ballId);
      return true;
    case 'capturado':
      s.capturas++;
      if (e.pokemon?.shiny) s.shinies++;
      return true;
    case 'cura':
      return somarGasto((s.pocoes ??= {}), e.itemId);
    case 'revive':
      return somarGasto((s.revives ??= {}), e.itemId);
    default:
      return false;
  }
}

/**
 * O CUSTO do que a sessão gastou, a preço de LOJA — o que custaria repor. `precoBola(id)` e
 * `precoItem(id)` vêm do app.js (o `priceGold` do catálogo de bolas e o `npcPrice` do item). Item
 * sem preço de loja (a Beast Ball, que era de diamante; a Medicine) entra na lista com custo 0 e
 * fica fora do total — melhor não somar do que inventar um preço.
 *
 * Cada grupo vem na ordem do id, que é a ordem da loja (Poké, Great, Super, Ultra; Small, Great,
 * Ultra, Hyper, Ultimate): a lista não fica trocando de lugar enquanto os números sobem.
 */
export function custosDaSessao(s, { precoBola, precoItem }) {
  const grupo = (mapa, preco) => Object.entries(mapa ?? {})
    .map(([k, q]) => {
      const id = Number(k);
      const qtd = Math.max(0, Math.floor(Number(q) || 0));
      const unidade = Math.max(0, Number(preco(id)) || 0);
      return { id, qtd, unidade, custo: unidade * qtd };
    })
    .filter((x) => x.qtd > 0)
    .sort((a, b) => a.id - b.id);
  const bolas = grupo(s?.bolas, precoBola);
  const pocoes = grupo(s?.pocoes, precoItem);
  const revives = grupo(s?.revives, precoItem);
  const total = [...bolas, ...pocoes, ...revives].reduce((t, x) => t + x.custo, 0);
  return { bolas, pocoes, revives, total };
}

/** Um valor da sessão por hora — a mesma conta do `ritmoDaSessao` (piso de 1 minuto). */
export function porHora(valor, s, agora = Date.now()) {
  if (!s?.inicio) return 0;
  const horas = Math.max(agora - s.inicio, MS_MINIMO_RITMO) / 3_600_000;
  return Math.round((Number(valor) || 0) / horas);
}
