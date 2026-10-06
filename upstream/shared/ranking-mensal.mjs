// O PRÊMIO DO MÊS dos placares de treinador — Top Treinadores, Top Catch, Top Coins e Top Bosses.
//
// No fechamento de cada mês (dia 1º, 00:00 de Brasília — o mesmo relógio da Temporada Global de
// guilds), quem estiver no placar ganha diamante pela posição. E quem é o 1º de cada placar AGORA usa o
// SELO no chat: o Ash (a nossa logo antiga) no Top Treinadores, a Poké Ball no Top Catch, a moeda no Top
// Coins e a coroa de boss no Top Bosses. Pedido do dono em 06/10/2026, "assim como funciona na gvg"; o
// Top Bosses entrou no mesmo dia, e o selo virou AO VIVO (passa para quem ultrapassar) no mesmo dia,
// porque esperar o primeiro fechamento deixava o chat sem ícone nenhum por um mês.
//
// O Top Bosses é o placar "Todos" da aba Bosses: a SOMA das vitórias em todos os bosses
// (`automation.bossKills`), e não o placar de um boss só — esse continua existindo no seletor, sem prêmio.
//
// O que conta é o PLACAR que o jogador vê, na hora do fechamento: nível (com o XP no desempate),
// espécies capturadas, ouro na conta. Os três não zeram — não há como zerar nível ou Pokédex —, então o
// prêmio é de quem SEGUE no topo quando o mês vira. Conta banida na hora do fechamento não recebe, e as
// posições não andam por causa dela: o 5º do placar é o 5º do prêmio.
//
// Mora no shared porque a TELA mostra a escada (a legenda do Ranking) e o selo (o chat), e as duas
// precisam ser a mesma tabela com que o servidor paga.

/**
 * Os placares premiados, pelo id da aba do Ranking. `bosses` é o "Todos" da aba Bosses (ver
 * `BOSS_TODOS`), e não um boss em particular.
 */
export const PLACARES_MENSAIS = ['nivel', 'capturas', 'coins', 'bosses'];

/** A chave do placar "Todos" da aba Bosses: a soma das vitórias em todos os bosses. */
export const BOSS_TODOS = 'todos';

/** As faixas, de cima para baixo: até que posição, quantos diamantes, e se leva o selo do chat. */
export const FAIXAS_RANKING_MENSAL = [
  { ate: 1, diamantes: 100, selo: true },
  { ate: 10, diamantes: 50 },
  { ate: 100, diamantes: 25 },
];

/** Quantas posições cada placar paga — e quantas linhas as três abas mostram. */
export const PREMIADOS_RANKING_MENSAL = FAIXAS_RANKING_MENSAL[FAIXAS_RANKING_MENSAL.length - 1].ate;

/** O prêmio de uma posição: `{ diamantes, selo }`, ou `null` fora das faixas. */
export function premioDaPosicao(pos) {
  const p = Math.floor(Number(pos));
  if (!(p >= 1)) return null;
  const f = FAIXAS_RANKING_MENSAL.find((x) => p <= x.ate);
  return f ? { diamantes: f.diamantes, selo: !!f.selo } : null;
}

/**
 * O teto de emissão do mês: os três placares cheios, sem ninguém de fora. É o número que justifica o
 * motivo `ranking_mensal` em `MOTIVOS_QUE_EMITEM` (server/game/diamantes.mjs).
 */
export const TETO_DIAMANTES_MES = (() => {
  let de = 1;
  let porPlacar = 0;
  for (const f of FAIXAS_RANKING_MENSAL) {
    porPlacar += (f.ate - de + 1) * f.diamantes;
    de = f.ate + 1;
  }
  return porPlacar * PLACARES_MENSAIS.length;
})();

/**
 * O selo do chat de cada placar. O Ash é a primeira logo do jogo (o ícone da aba até a v1.230.3); a
 * Poké Ball e a moeda são as do HUD; a coroa é o ícone do botão Boss do menu. Caminho absoluto = arte
 * nossa (`/img`); relativo = espelho (`/assets/...`), como em todo ícone do cliente.
 */
export const SELOS_RANKING = {
  nivel: { icone: '/img/favicon-32.png' },
  capturas: { icone: 'site/assets/ui/ball-poke.png' },
  coins: { icone: 'site/assets/ui/moeda-ouro.png' },
  bosses: { icone: 'site/assets/ui/menu-bosses.png' },
};
