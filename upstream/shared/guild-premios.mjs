// As recompensas da guild — num arquivo só, porque DOIS lados leem os mesmos números.
//
// O servidor paga (`server/game/guild.mjs` recalcula o bônus diário depois de cada guerra,
// `server/game/guild-global.mjs` paga o diamante do mês) e a tela MOSTRA (a folha 🏆 do
// PvP Guild, a legenda do Ranking e o capítulo `guild` da Poképedia). Com o número escrito nos
// dois lados, o primeiro ajuste de balanceamento deixaria a folha prometendo o que o servidor
// não paga — então TODA tela desenha a partir destes objetos, sem lista escrita à mão.
//
// Os dois prêmios são do TIME, não da lista de membros: quem está na guild mas fora da
// escalação não leva nem o % do dia nem o diamante do mês. Quem corta é `p.guildBonusPct` em
// `server/sim.mjs` (diário) e `playerIdsEscalados` em `server/guild-db.mjs` (mensal).

/**
 * Bônus % de XP de treinador, XP de pokémon e loot por posição no ranking DIÁRIO de GP.
 * Só guilds com GP > 0 — e vale até a guerra seguinte, que zera o GP e refaz o ranking.
 *
 * A escada vai do 1º ao 9º e depois achata: `resto` é o piso de quem pontuou e ficou do 10º
 * para baixo. Não há teto de posição — toda guild com GP > 0 leva pelo menos esse 1%.
 */
export const BONUS_RANKING_GP = { 1: 10, 2: 9, 3: 8, 4: 7, 5: 6, 6: 5, 7: 4, 8: 3, 9: 2, resto: 1 };

/** As posições com % próprio, em ordem. A tela desenha estas linhas e mais a do `resto`. */
export const POSICOES_BONUS_GP = Object.keys(BONUS_RANKING_GP)
  .filter((k) => k !== 'resto')
  .map(Number)
  .sort((a, b) => a - b);

/** A primeira posição que cai no piso — o "TOP >9" da legenda. */
export const PRIMEIRA_POS_RESTO = POSICOES_BONUS_GP.length + 1;

/** O % do ranking diário de uma posição. 0 para quem não está no ranking (GP = 0). */
export function bonusPctPorPosRanking(pos) {
  const p = Number(pos) || 0;
  if (p <= 0) return 0;
  return BONUS_RANKING_GP[p] ?? BONUS_RANKING_GP.resto;
}

/**
 * Diamante por ESCALADO da guild, por posição no ranking GLOBAL, no fechamento do mês.
 * Aqui, ao contrário do diário, há teto: fora destas posições o mês não paga nada.
 */
export const PREMIOS_GLOBAL = { 1: 100, 2: 50, 3: 35, 4: 25, 5: 20, 6: 15, 7: 10, 8: 5 };

/** Quantas guilds o fechamento mensal paga — é o `limite` com que ele lê o ranking Global. */
export const PREMIADOS_GLOBAL = Object.keys(PREMIOS_GLOBAL).length;
