// O FILTRO de tipo e de IV mínimo do Depot e da venda de pokémon do Market NPC — numa peça só, para a
// tela e o servidor nunca discordarem sobre quem passa.
//
// Mora em `shared/` porque o "Vender todo o Depot (Filtrado)" vende exatamente o que a tela mostra:
// a tela filtra com isto, e o servidor, que recebe o MESMO filtro (e não a lista de ids, que num
// depot de milhares de pokémon passaria do teto de 16 KB por mensagem do socket), refaz a conta
// com isto. Uma cópia de cada lado divergiria no primeiro ajuste — e aí a venda levaria um
// pokémon que a tela não mostrou.
import { IV_MIN, IV_MAX } from './nota-pokemon.mjs';

/**
 * O piso de IV digitado, já limpo. `null` = sem filtro.
 *
 * Campo vazio, texto e número fora da faixa caem todos em "sem filtro" ou na borda em vez de
 * esvaziarem a lista: quem digita 999 quer os melhores, não quer uma tela em branco, e um
 * `<input>` de número aceita coisas que o teclado do celular deixa passar.
 */
export function pisoIvDoFiltro(valor) {
  if (valor == null || String(valor).trim() === '') return null;
  const v = Math.round(Number(valor));
  if (!Number.isFinite(v)) return null;
  return Math.min(IV_MAX, Math.max(IV_MIN, v));
}

/** A soma dos IVs: o total pronto, se o objeto já o trouxer, senão a soma dos seis. */
export function ivTotalParaFiltro(pk) {
  if (!pk) return null;
  const direto = pk.iv ?? pk.ivTotal;
  if (direto != null && Number.isFinite(Number(direto))) return Math.round(Number(direto));
  if (!pk.ivs) return null;
  return Object.values(pk.ivs).reduce((soma, v) => soma + (Number(v) || 0), 0);
}

/** O pokémon passa no filtro de TIPO (um dos dois da espécie) e de IV mínimo (a soma)? */
export function passaFiltroTipoIv(pk, tipo = null, ivMin = null) {
  if (tipo && !(pk?.tipos ?? []).includes(tipo)) return false;
  if (ivMin != null && (ivTotalParaFiltro(pk) ?? 0) < ivMin) return false;
  return true;
}
