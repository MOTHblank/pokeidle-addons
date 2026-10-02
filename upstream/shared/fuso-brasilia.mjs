// Quanto Brasília está ATRÁS do UTC — o deslocamento que decide a hora de toda virada do jogo.
//
// Fixo, e não `America/Sao_Paulo` pelo Intl: o Brasil não tem horário de verão desde 2019, e um
// deslocamento escrito aqui dá a mesma resposta no servidor, na tela e no teste — sem depender da
// base de fusos do Node nem da do navegador de cada jogador. Se o horário de verão voltar, é
// AQUI que se mexe, e num lugar só.
//
// Mora no shared sozinho porque três relógios diferentes já dependiam dele e o escreviam cada um
// por conta: a temporada semanal do PvP (`shared/pvp-rank.mjs`, que ainda reexporta o número com
// o nome antigo), as datas do Campeonato e, desde 2026-10, o fechamento mensal da Temporada
// Global de Guilds (`server/game/guild-global.mjs`).
//
// O fechamento do mês era o único que virava em UTC, e foi assim que o chat anunciou "temporada
// fechada" às 21h de Brasília de 30/09/2026 — meia-noite UTC do dia 1 — para um servidor que
// esperava o prêmio à meia-noite DELE. A regra passou a ser a mesma do PvP semanal: 00:00 em
// Brasília, que a tela do ranking já anunciava com essas palavras.
export const FUSO_BRASILIA_MS = 3 * 60 * 60_000;

/**
 * O mesmo instante, lido como se o relógio fosse o de Brasília.
 *
 * Devolve um `Date` para ser lido com `getUTC*` — e NUNCA com `getHours`/`getDate`, que
 * voltariam a trazer o fuso do processo para dentro da conta. É o deslocamento, não uma data
 * em outro fuso: o instante absoluto da virada continua sendo o mesmo para todo mundo.
 */
export const emBrasilia = (t) => new Date(Number(t) - FUSO_BRASILIA_MS);
