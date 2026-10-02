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
 * ### O Absorb Bulb — a lamparina que aquece UM ovo e quebra
 *
 * 3 💎 aquecem o ovo que está numa chocadeira: ele passa a chocar 25% mais rápido, e quando ele
 * abre a lamparina quebra junto. Sem teto — quem quiser aquecer todos os ovos de todas as levas
 * paga 3 💎 por ovo, sempre.
 *
 * ### A lamparina NUNCA existe como objeto, e é isso que a torna impossível de duplicar
 *
 * Ela não é item da bolsa, não tem contador na conta e não fica "guardada" em lugar nenhum: os
 * 3 💎 SÃO o ato de aquecer, e o que sobra disso é um campo `luz` dentro do próprio ovo. Não há
 * estoque para somar duas vezes, nada para transferir no Mercado, nada que uma corrida de dois
 * cliques possa multiplicar — o pior caso de uma falha aqui é um ovo aquecido que não foi cobrado,
 * nunca uma lamparina a mais no mundo.
 *
 * E é por isso que só se acende COM O OVO DENTRO (foi o pedido, e fecha o resto sozinho): sem ovo
 * não existe onde a marca more, então não há como comprar antes e usar depois.
 *
 * O `fim` do ovo aquecido é recalculado a partir do `inicio` — nunca do `fim` atual. A conta é
 * idempotente: um efeito aplicado duas vezes dá o mesmo instante, e um ovo não pode ser acelerado
 * em cima de si mesmo. A trava de verdade, ainda assim, é `motivoRecusaAcender`, que recusa o ovo
 * que já está aquecido antes de qualquer débito.
 *
 * O estado inteiro mora em `automation.chocadeira` (o mesmo jsonb do contador de bosses): vai no
 * mesmo flush da bolsa, e é isso que faz o ovo nunca existir nos dois lugares — quem sai da bolsa
 * entra na chocadeira no mesmo UPDATE. A lamparina "quebrar" não é um passo à parte: ela mora no
 * ovo, e o ovo sai da chocadeira ao chocar (`chocarOvosProntos`, no sim).
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

/** Quanto do tempo de choca sobra com o ovo aquecido — 25% mais rápido (16h → 12h). */
export const BULBO_FATOR = 0.75;

/**
 * Quanto tempo um ovo leva para abrir, em ms. `comLuz` desconta o Absorb Bulb daquele espaço.
 *
 * `Math.round` porque o resultado vira uma DATA gravada no jsonb: `0.75` de uma hora é exato, mas
 * um fator futuro que não seja poderia deixar fração de milissegundo num carimbo de tempo.
 */
export const duracaoDoOvo = (ovo, comLuz = false) =>
  Math.round((ovo?.horas ?? 0) * 3600_000 * (comLuz ? BULBO_FATOR : 1));

/** Toda conta nasce com uma. */
export const CHOCADEIRAS_GRATIS = 1;

/** O teto por conta. */
export const CHOCADEIRAS_MAX = 6;

/** O preço de cada chocadeira a mais, em diamantes. */
export const PRECO_CHOCADEIRA_DIAMANTES = 50;

/**
 * O preço de aquecer UM ovo, em diamantes. Sem teto: a lamparina quebra quando o ovo abre, então
 * o próximo ovo custa outros 3 💎 — é o que faz dela uma venda que se repete, e não uma só.
 */
export const PRECO_BULBO_DIAMANTES = 3;

/**
 * O estado limpo, venha ele do banco ou de onde for:
 * `{ total, ovos: [{ slot, ovo, inicio, fim, luz }] }`.
 *
 * `total` fica entre 1 e 6. Cada ovo tem de estar num espaço que existe (0 … total−1), sem dois no
 * mesmo, ser de um tipo que existe e ter as duas datas. Lixo no jsonb não vira ovo — e também não
 * derruba a carga do jogador: some. Nenhum caminho do jogo grava um ovo torto, então o que cai aqui
 * é corrupção, e a regra de toda normalização do jogo vale (ver `normalizarPasse`).
 *
 * `luz` é a lamparina DAQUELE ovo, e sai daqui como booleano de verdade: o que chega do banco pode
 * ser `1`, `"true"` ou lixo, e um valor meio-verdade num campo que vale 3 💎 é exatamente o tipo de
 * coisa que vira exploit mais tarde. Ela não aparece em lugar nenhum fora do ovo — some com ele
 * quando ele choca, que é a lamparina "quebrando".
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
    const ovo = { slot, ovo: def.id, inicio, fim };
    if (o?.luz === true) ovo.luz = true;
    ovos.push(ovo);
  }
  ovos.sort((a, b2) => a.slot - b2.slot);
  return { total, ovos };
}

/** O ovo de um espaço, ou `null`. */
export const ovoNoSlot = (estado, slot) => estado.ovos.find((o) => o.slot === slot) ?? null;

/** O ovo deste espaço está aquecido? */
export const temLuz = (estado, slot) => ovoNoSlot(estado, Number(slot))?.luz === true;

/**
 * Quando um ovo posto AGORA abre. É a única conta de data de choca do jogo — o sim usa ao pôr o
 * ovo, e a loja ao aquecer um que já está lá. Sempre a partir do `inicio`, nunca do `fim` atual.
 */
export const fimDoOvo = (inicio, def, comLuz) => inicio + duracaoDoOvo(def, comLuz);

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

/**
 * Pode aquecer o ovo deste espaço? Devolve a CHAVE do motivo da recusa, ou `null`.
 *
 * É a trava do dinheiro, e ela roda ANTES de qualquer débito (ver `validarCompra`, na loja). As
 * quatro recusas, na ordem em que importam:
 *
 *   · **espaço que não existe** ou **trancado** — não há ovo lá, e não pode haver;
 *   · **espaço vazio** — a lamparina não existe fora de um ovo. É o que impede comprar antes e usar
 *     depois, e com isso some a ideia de "ter lamparinas" que alguém pudesse duplicar;
 *   · **ovo JÁ aquecido** — a recusa que impede cobrar duas vezes pelo mesmo ovo. Ela não depende
 *     da idempotência da conta do `fim`: essa evita acelerar duas vezes, esta evita PAGAR duas vezes;
 *   · **ovo que já venceu** — ele abre no próximo tique do sim (segundos), e aquecê-lo seria
 *     3 💎 por um desconto que não chega a existir.
 */
export function motivoRecusaAcender(estado, slot, agora = Date.now()) {
  // Mais estrito que `motivoRecusaColocar`, e de propósito: ali o "slot" errado só gasta um ovo
  // que já é do jogador, aqui ele gasta DIAMANTE. `Number(null)`, `Number([])` e `Number([0])` são
  // todos 0 — e "não mandei espaço nenhum" não pode virar "acenda o espaço 0". Número ou texto
  // numérico passam (o cliente manda os dois); o resto é pedido torto.
  const cru = typeof slot === 'string' ? slot.trim() : slot;
  const s = typeof cru === 'number' || (typeof cru === 'string' && cru !== '') ? Number(cru) : NaN;
  if (!Number.isInteger(s) || s < 0 || s >= CHOCADEIRAS_MAX) return 'chocadeira.slotInvalido';
  if (s >= estado.total) return 'chocadeira.luzSemChocadeira';
  const ovo = ovoNoSlot(estado, s);
  if (!ovo) return 'chocadeira.luzSemOvo';
  if (ovo.luz === true) return 'chocadeira.jaTemLuz';
  if (ovo.fim <= agora) return 'chocadeira.luzOvoPronto';
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
