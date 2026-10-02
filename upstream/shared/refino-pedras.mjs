/**
 * Quais pedras de evolução servem para REFINAR uma espécie — e como mesclar saldos.
 *
 * Dual-type aceita pedra de qualquer um dos tipos elementares do bicho (Fire Stone OU Feather
 * Stone num Charizard). Os custos somam entre elas: 300 Feather + 200 Fire fecham um degrau de
 * 500. Na tela o jogador ESCOLHE quanto sai de cada uma (`validarUsoPedrasRefino`, conferida de
 * novo no servidor); sem escolha — aba antiga, espécie de uma pedra só — a ordem é a
 * determinística de sempre (maior pilha primeiro, `consumirPedrasRefino`).
 */
import { tipoDaPedraDeEvolucao } from './evolucoes-ramificadas.mjs';
import { PEDRA_POR_TIPO } from './pedras-evolucao.mjs';

/** Tipos elementais cujas pedras entram no refino desta espécie (sem repetir). */
export function tiposPedraRefino(especie) {
  if (!especie) return [];
  const vistos = new Set();
  const out = [];
  for (const t of [tipoDaPedraDeEvolucao(especie), especie.type1, especie.type2]) {
    if (!t || vistos.has(t)) continue;
    vistos.add(t);
    out.push(t);
  }
  return out;
}

/** `{ tipo, nome }[]` — nomes únicos, na ordem dos tipos acima. */
export function nomesPedraRefino(especie) {
  const vistos = new Set();
  const out = [];
  for (const tipo of tiposPedraRefino(especie)) {
    const nome = PEDRA_POR_TIPO[tipo] ?? PEDRA_POR_TIPO.NORMAL;
    if (vistos.has(nome)) continue;
    vistos.add(nome);
    out.push({ tipo, nome });
  }
  return out;
}

/** Resolve `{ tipo, nome, itemId }[]` usando o mapa tipo→pedra do welcome/catálogo. */
export function pedrasRefinoResolvidas(especie, mapaPorTipo) {
  const vistos = new Set();
  const out = [];
  for (const tipo of tiposPedraRefino(especie)) {
    const p = mapaPorTipo?.[tipo] ?? mapaPorTipo?.NORMAL;
    if (!p?.itemId || vistos.has(p.itemId)) continue;
    vistos.add(p.itemId);
    out.push({ tipo, itemId: p.itemId, nome: p.nome ?? p.name ?? 'Stone' });
  }
  return out;
}

export function saldoPedrasRefino(items, pedras) {
  let s = 0;
  for (const p of pedras ?? []) s += Math.max(0, Math.floor(Number(items?.[p.itemId]) || 0));
  return s;
}

/**
 * Gasta `custo` pedras mesclando os tipos válidos. Mutates `items`.
 * @returns {{ itemId, nome, qtd }[]} ou `null` se o saldo não fechar.
 */
export function consumirPedrasRefino(items, pedras, custo) {
  const total = Math.floor(Number(custo) || 0);
  if (total <= 0) return [];
  if (saldoPedrasRefino(items, pedras) < total) return null;

  let falta = total;
  const consumido = [];
  const ordenadas = [...(pedras ?? [])].sort(
    (a, b) => (items[b.itemId] ?? 0) - (items[a.itemId] ?? 0),
  );

  for (const p of ordenadas) {
    if (falta <= 0) break;
    const tem = Math.max(0, Math.floor(Number(items[p.itemId]) || 0));
    if (!tem) continue;
    const usa = Math.min(tem, falta);
    const resta = tem - usa;
    if (resta > 0) items[p.itemId] = resta;
    else delete items[p.itemId];
    consumido.push({ itemId: p.itemId, nome: p.nome, qtd: usa });
    falta -= usa;
  }

  return falta <= 0 ? consumido : null;
}

/**
 * Confere a MISTURA que o jogador escolheu: `uso` = `{ [itemId]: qtd }`. Só passa com as pedras que
 * refinam esta espécie, quantidades inteiras ≥ 0 (número de verdade — nem texto, nem fração, nem
 * lista), nenhuma acima do saldo, e a soma EXATAMENTE igual ao custo do degrau. O servidor chama com
 * o inventário e o custo DELE, nunca com o que a tela diz que eles são.
 * @returns {{ ok: true, plano: { itemId, nome, qtd }[] } | { ok: false, erro: string }}
 */
export function validarUsoPedrasRefino(items, pedras, custo, uso) {
  if (!uso || typeof uso !== 'object' || Array.isArray(uso)) return { ok: false, erro: 'mistura de pedras inválida' };
  const validas = new Map((pedras ?? []).map((p) => [String(p.itemId), p]));
  const total = Math.floor(Number(custo) || 0);
  let soma = 0;
  const plano = [];
  for (const [chave, qtd] of Object.entries(uso)) {
    const p = validas.get(chave);
    if (!p) return { ok: false, erro: 'essa pedra não refina este pokémon' };
    if (typeof qtd !== 'number' || !Number.isInteger(qtd) || qtd < 0) return { ok: false, erro: 'quantidade de pedra inválida' };
    const tem = Math.max(0, Math.floor(Number(items?.[p.itemId]) || 0));
    if (qtd > tem) return { ok: false, erro: `você só tem ${tem}× ${p.nome}` };
    if (qtd > 0) plano.push({ itemId: p.itemId, nome: p.nome, qtd });
    soma += qtd;
  }
  if (total <= 0 || soma !== total) return { ok: false, erro: `a mistura soma ${soma}, e o degrau custa ${total}` };
  return { ok: true, plano };
}

/** Gasta um plano JÁ validado por `validarUsoPedrasRefino`. Mutates `items`. */
export function gastarPlanoPedrasRefino(items, plano) {
  for (const { itemId, qtd } of plano) {
    const resta = Math.max(0, Math.floor(Number(items[itemId]) || 0)) - qtd;
    if (resta > 0) items[itemId] = resta;
    else delete items[itemId];
  }
  return plano.map((x) => ({ ...x }));
}

/**
 * A mistura que a TELA propõe quando o jogador mexe numa pedra: ela fica com `qtd` (cortada no saldo
 * dela e no custo) e o resto do custo sai das OUTRAS, maior pilha primeiro. Sem `itemIdFixo`, é a
 * mistura padrão (a mesma de `consumirPedrasRefino`). Devolve `{ [itemId]: qtd }` — que pode não
 * fechar o custo, se as outras não tiverem o bastante (a tela mostra quanto falta).
 */
export function completarUsoPedrasRefino(items, pedras, custo, itemIdFixo = null, qtdFixa = 0) {
  const saldo = (p) => Math.max(0, Math.floor(Number(items?.[p.itemId]) || 0));
  const total = Math.max(0, Math.floor(Number(custo) || 0));
  const uso = {};
  let falta = total;
  const fixa = itemIdFixo == null ? null : (pedras ?? []).find((p) => String(p.itemId) === String(itemIdFixo));
  if (fixa) {
    const q = Math.max(0, Math.min(saldo(fixa), total, Math.floor(Number(qtdFixa) || 0)));
    uso[fixa.itemId] = q;
    falta -= q;
  }
  const outras = (pedras ?? []).filter((p) => p !== fixa).sort((a, b) => saldo(b) - saldo(a));
  for (const p of outras) {
    const q = Math.min(saldo(p), falta);
    uso[p.itemId] = q;
    falta -= q;
  }
  return uso;
}

/** Prévia do consumo — não altera o inventário real. */
export function simularConsumoPedrasRefino(items, pedras, custo) {
  const copia = { ...(items ?? {}) };
  return consumirPedrasRefino(copia, pedras, custo);
}

export function formatarConsumoPedras(consumido) {
  if (!consumido?.length) return '';
  return consumido.map((c) => `${c.qtd}× ${c.nome}`).join(' + ');
}

export function rotuloPedrasRefino(pedras) {
  if (!pedras?.length) return '';
  if (pedras.length === 1) return pedras[0].nome;
  return pedras.map((p) => p.nome).join(' / ');
}
