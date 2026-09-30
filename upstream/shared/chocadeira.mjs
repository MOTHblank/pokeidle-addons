/**
 * A CHOCADEIRA e os OVOS — as regras puras, sem sim nem banco.
 *
 * ### O que é
 *
 * O Market NPC vende três ovos, um por potência: o P2, o P3 e o P4. Comprado, o ovo cai em "Itens
 * Raros" da bolsa; clicado, vai para uma CHOCADEIRA livre, e ela conta o tempo sozinha — com o
 * jogador online ou não, porque o que se guarda é a DATA em que o ovo abre, e não um relógio que
 * anda. Quando a data chega, o ovo se abre e o pokémon nasce direto na Coleção.
 *
 * O pokémon é um sorteio LIVRE entre as espécies que têm hunt no Mapa — fora da Outland, e nunca
 * Lendário, Mítico ou Mega (`podeChocarDoOvo`; 694 espécies em 30/09/2026): a mesma chance para um
 * Magikarp e para um pokémon de hunt 100k. A potência é a do ovo; o resto
 * (qualidade, IV, shiny) sai da mesma roleta de uma captura. O nível é o da hunt da espécie, com
 * o MESMO teto de 100 da captura — um ovo não entrega o que a bola não entregaria.
 *
 * ### As chocadeiras
 *
 * Toda conta nasce com UMA (é o Pokémon Box Link do pokesprite). As outras cinco, até seis, saem
 * na Loja VIP por 50 💎 cada — uma compra por espaço, e o espaço é da conta, para sempre. Não é
 * um item: não vai à bolsa, não se vende e não se perde. O que a conta guarda é o NÚMERO delas.
 *
 * O estado inteiro mora em `automation.chocadeira` (o mesmo jsonb do contador de bosses): vai no
 * mesmo flush da bolsa, e é isso que faz o ovo nunca existir nos dois lugares — quem sai da bolsa
 * entra na chocadeira no mesmo UPDATE.
 */
import { DEX_LENDARIO_OU_MITICO } from './mistico.mjs';
import { isOutlandPokeId } from './outland.mjs';
import { isMegaPokeId } from './megas.mjs';

/**
 * Os três ovos. `itemId` na faixa 70000–70099 dos itens nossos. Preço em Coins; o tempo de choca
 * em HORAS (16/32/48, decisão do dono em 30/09/2026 — eram 3/5/7 dias). Um ovo já na chocadeira
 * guarda a DATA em que abre, então mudar estes números só vale para quem choca depois.
 */
export const OVOS = Object.freeze([
  Object.freeze({ id: 'p2', itemId: 70091, potencia: 2, preco: 100_000_000, horas: 16, nome: 'Mystery Egg P2' }),
  Object.freeze({ id: 'p3', itemId: 70092, potencia: 3, preco: 500_000_000, horas: 32, nome: 'Mystery Egg P3' }),
  Object.freeze({ id: 'p4', itemId: 70093, potencia: 4, preco: 1_000_000_000, horas: 48, nome: 'Mystery Egg P4' }),
]);

export const OVO_POR_ID = new Map(OVOS.map((o) => [o.id, o]));
export const OVO_POR_ITEM = new Map(OVOS.map((o) => [o.itemId, o]));
export const ehItemOvo = (itemId) => OVO_POR_ITEM.has(Number(itemId));

/** Quanto tempo um ovo leva para abrir, em ms. */
export const duracaoDoOvo = (ovo) => (ovo?.horas ?? 0) * 3600_000;

/** Toda conta nasce com uma. */
export const CHOCADEIRAS_GRATIS = 1;

/** O teto por conta. */
export const CHOCADEIRAS_MAX = 6;

/** O preço de cada chocadeira a mais, em diamantes. */
export const PRECO_CHOCADEIRA_DIAMANTES = 50;

/**
 * O estado limpo, venha ele do banco ou de onde for: `{ total, ovos: [{ slot, ovo, inicio, fim }] }`.
 *
 * `total` fica entre 1 e 6. Cada ovo tem de estar num espaço que existe (0 … total−1), sem dois no
 * mesmo, ser de um tipo que existe e ter as duas datas. Lixo no jsonb não vira ovo — e também não
 * derruba a carga do jogador: some. Nenhum caminho do jogo grava um ovo torto, então o que cai aqui
 * é corrupção, e a regra de toda normalização do jogo vale (ver `normalizarPasse`).
 */
export function normalizarChocadeira(bruto) {
  const b = bruto && typeof bruto === 'object' && !Array.isArray(bruto) ? bruto : {};
  const total = Math.max(
    CHOCADEIRAS_GRATIS,
    Math.min(CHOCADEIRAS_MAX, Math.floor(Number(b.total)) || CHOCADEIRAS_GRATIS),
  );
  const usados = new Set();
  const ovos = [];
  for (const o of Array.isArray(b.ovos) ? b.ovos : []) {
    const slot = Math.floor(Number(o?.slot));
    const def = OVO_POR_ID.get(String(o?.ovo ?? ''));
    const inicio = Number(o?.inicio);
    const fim = Number(o?.fim);
    if (!def || !Number.isInteger(slot) || slot < 0 || slot >= total || usados.has(slot)) continue;
    if (!Number.isFinite(inicio) || !Number.isFinite(fim) || !(inicio > 0) || !(fim > inicio)) continue;
    usados.add(slot);
    ovos.push({ slot, ovo: def.id, inicio, fim });
  }
  ovos.sort((a, b) => a.slot - b.slot);
  return { total, ovos };
}

/** O ovo de um espaço, ou `null`. */
export const ovoNoSlot = (estado, slot) => estado.ovos.find((o) => o.slot === slot) ?? null;

/** O primeiro espaço livre, ou `null`. */
export function primeiroSlotLivre(estado) {
  for (let s = 0; s < estado.total; s++) if (!ovoNoSlot(estado, s)) return s;
  return null;
}

/**
 * Pode pôr um ovo neste espaço? Devolve a CHAVE do motivo da recusa (i18n do cliente), ou `null`.
 *
 * Quem chama confere a bolsa (se o ovo existe lá) — aqui só o que depende da chocadeira.
 */
export function motivoRecusaColocar(estado, slot, ovoId) {
  if (!OVO_POR_ID.has(String(ovoId))) return 'chocadeira.ovoInvalido';
  const s = Number(slot);
  if (!Number.isInteger(s) || s < 0 || s >= CHOCADEIRAS_MAX) return 'chocadeira.slotInvalido';
  if (s >= estado.total) return 'chocadeira.slotTrancado';
  if (ovoNoSlot(estado, s)) return 'chocadeira.slotOcupado';
  return null;
}

/** Os ovos que já podem abrir neste instante. */
export const ovosProntos = (estado, agora) => estado.ovos.filter((o) => o.fim <= agora);

/** O instante do PRÓXIMO ovo a abrir, ou `Infinity` — o tick só olha a chocadeira quando chega nele. */
export function proximoFim(estado) {
  let menor = Infinity;
  for (const o of estado.ovos) if (o.fim < menor) menor = o.fim;
  return menor;
}

/**
 * O número da Pokédex por trás de um id do jogo. As cópias de Orre (#13000+) carregam o dex nos
 * três últimos dígitos (as 139 conferidas contra o nome em 30/09/2026); o resto é o próprio id.
 */
const dexBase = (id) => (id >= 13000 ? id % 1000 : id);

/**
 * Pode chocar deste ovo? Regras do dono (30/09/2026). NUNCA:
 *
 *   · um LENDÁRIO nem um MÍTICO — esses só saem da Arena Mística, com o MysticTicket, e os Bosses
 *     são todos lendários. Hoje nenhuma hunt tem um (o editor de spawns barra os lendários, ver
 *     `spawns-filtro.mjs`), mas ele deixa MÍTICO entrar numa hunt — e o ovo não pode passar a
 *     chocar um no dia em que alguém puser, sem ninguém perceber. A cópia de Orre de um lendário
 *     também fica de fora, pelo dex;
 *   · uma variante da OUTLAND (#2001+) — os pokémon da área endgame saem de lá;
 *   · uma MEGA (#3000+) — a mega nasce da pedra, não de ovo. Nenhuma hunt tem mega hoje; a trava é
 *     pelo mesmo motivo da do mítico.
 */
export function podeChocarDoOvo(id) {
  const n = Number(id);
  return !isOutlandPokeId(n) && !isMegaPokeId(n) && !DEX_LENDARIO_OU_MITICO.has(dexBase(n));
}

/**
 * As espécies que um ovo pode chocar: toda espécie com hunt JOGÁVEL de qualquer região do Mapa,
 * MENOS a Outland inteira (a área, e não só as variantes que ela tem hoje) e menos o que
 * `podeChocarDoOvo` barra — sem repetir, pela ordem do id. `huntsJogaveis` é a lista do content
 * (`{ area, especies: [{ pokeId }] }`). Cada uma sai com a mesma chance — ver `sortearEspecieDoOvo`.
 */
export function especiesDoOvo(huntsJogaveis) {
  const ids = new Set();
  for (const h of huntsJogaveis ?? []) {
    if (h?.area === 'outland') continue;
    for (const e of h.especies ?? []) ids.add(Number(e.pokeId));
  }
  return [...ids]
    .filter((id) => Number.isInteger(id) && id > 0 && podeChocarDoOvo(id))
    .sort((a, b) => a - b);
}

/** Uma espécie da lista, todas com a mesma chance. */
export function sortearEspecieDoOvo(ids, aleatorio = Math.random) {
  if (!ids?.length) return null;
  return ids[Math.floor(aleatorio() * ids.length)] ?? ids[ids.length - 1];
}
