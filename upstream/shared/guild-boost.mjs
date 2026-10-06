// O BOOST DA GUILD — +10% de XP do treinador e do pokémon para a guild INTEIRA, por 7 dias, ligado
// pelo BANCO DA GUILD: os membros doam diamante, e cada vez que o banco junta 1.000 💎 o boost liga
// (ou ganha mais 7 dias) e o banco zera para a próxima rodada.
//
// Num arquivo só porque DOIS lados leem os mesmos números. O servidor cobra e aplica
// (`server/guild-db.mjs` debita a doação e liga o boost na mesma transação; `server/game/loja.mjs`
// monta o multiplicador) e a tela mostra (a aba Boost do painel da guild, o diálogo de doação, a
// linha da ficha do treinador e a faixa do painel). Com o número escrito nos dois lados, o primeiro
// ajuste deixaria a barra prometendo o que o servidor não paga.
//
// ### Por que um banco, e não uma compra
//
// Até 03/10/2026 o boost era um produto da Loja: UM membro pagava os 1.000 💎 e a guild inteira
// ganhava. Saiu antes de ir ao ar — o peso caía sempre no mesmo bolso. Com o banco, cada um põe o
// que pode (de 1 💎 até o que falta), e o boost é da guild também na hora de pagar.
//
// ### O banco nunca passa da meta
//
// A doação é cortada no que FALTA para encher: com 950 no banco, quem pede para doar 100 paga 50 e
// fica com o resto. Assim cada doação pertence a uma rodada só — e é isso que deixa devolver o banco
// aberto, doador por doador, se a guild for apagada (ver `devolverBancoDaGuild`). Quem quer bancar
// mais uma semana doa de novo depois que o banco zera.
//
// ### O boost é da GUILD, não de quem doou
//
// Fica gravado na linha da guild (`guilds.boost_ate`) e vale para quem é membro AGORA: quem entra
// enquanto ele dura leva, quem sai perde. É a leitura literal de "todos que estiverem na guild" — e
// a doação também é da guild: não volta para quem sai.
//
// ### Todos os membros, e não só o time
//
// O bônus do RANKING é dos escalados (ver `carregarGuild` no sim) porque é prêmio de guerra: sem o
// corte, a guild campeã venderia +10% a quem nunca lutou. O boost não é prêmio, é o que a guild
// juntou — e vale para a guild toda.
//
// ### Multiplica, não soma
//
// Como todo bônus de XP do jogo (`multXpTreinador` em `server/game/loja.mjs`): o +10% entra em cima
// do que o jogador já tem — XP Boost, VIP, evento, Twitch, Kick. Somar faria o boost da guild valer
// menos justamente para quem já gastou no próprio.
//
// ### Encher de novo estende
//
// Banco cheio com o boost ainda correndo soma 7 dias a partir do fim atual, nunca reinicia — a regra
// de todo boost da Loja ("não acumula: comprar de novo estende o prazo").

export const BOOST_GUILD = Object.freeze({
  /** Quanto o boost soma no XP do treinador E no do pokémon. */
  pct: 10,
  dias: 7,
  /** A arte inteira (48×48: a aba Boost, o diálogo, a comemoração) e a pequena (32×32: o que sai a 16-20 px). */
  icone: '/img/itens/boost-guild.png',
  iconeMini: '/img/itens/boost-guild-32.png',
});

export const BOOST_GUILD_MS = BOOST_GUILD.dias * 24 * 3600_000;
export const BOOST_GUILD_MULT = 1 + BOOST_GUILD.pct / 100;

export const BANCO_GUILD = Object.freeze({
  /** Quanto o banco junta para ligar (ou estender) o boost: 7 dias. */
  meta: 1000,
  /** O `ref` das linhas do ledger: a doação (`loja`) e a devolução da guild apagada (`loja_estorno`). */
  ref: 'guild_banco',
  /** Os atalhos de doação da tela, além do "Completar". */
  atalhos: [10, 50, 100, 250],
  /** Quantas linhas o histórico do banco traz (doações e bancos cheios, os mais recentes). */
  historico: 25,
  /**
   * Quantas doações o LOG COMPLETO (dono e sub-dono, "Ver tudo" na aba) traz no máximo — as mais
   * recentes. O teto é do socket, que leva a resposta inteira de uma vez: 5.000 linhas são ~200 kB,
   * e uma guild que enche um banco por semana leva anos para chegar nelas. A soma por membro é
   * sempre da vida inteira, com teto ou sem (ver `logBancoDaGuild`).
   */
  logMax: 5000,
});

/** O boost está valendo? `ate` é o carimbo da guild, em ms (ou nada). */
export const boostGuildAtivo = (ate, agora) => (Number(ate) || 0) > agora;

/** O multiplicador de XP — treinador e pokémon — que a guild dá agora: 1,10 ou 1. */
export const multBoostGuild = (ate, agora) => (boostGuildAtivo(ate, agora) ? BOOST_GUILD_MULT : 1);

/**
 * Até quando o boost vai depois de MAIS um banco cheio: soma a partir do fim atual, nunca reinicia.
 *
 * O servidor faz a mesma conta em SQL, dentro da transação da doação (ver `doarBancoGuild`) — lá ela
 * é atômica contra dois membros doando no mesmo segundo. Esta aqui é a da TELA, que mostra o "depois"
 * no diálogo antes de o jogador confirmar.
 */
export const fimDoBoostGuild = (ate, agora) => Math.max(Number(ate) || 0, agora) + BOOST_GUILD_MS;

/** O que o banco tem, sempre dentro de 0…meta−1 (um banco cheio já zerou). */
export const saldoDoBanco = (saldo) => Math.min(BANCO_GUILD.meta - 1, Math.max(0, Math.floor(Number(saldo) || 0)));

/** Quanto falta para encher o banco: de 1 a `meta`. */
export const faltaNoBanco = (saldo) => BANCO_GUILD.meta - saldoDoBanco(saldo);

/**
 * O pedido de doação é válido? Inteiro de verdade (nada de "50", 50.5, NaN ou Infinity), de 1 até a
 * meta. O teto real é o que FALTA — e esse só o banco sabe, dentro da transação.
 */
export const doacaoValida = (qtd) =>
  typeof qtd === 'number' && Number.isSafeInteger(qtd) && qtd >= 1 && qtd <= BANCO_GUILD.meta;
