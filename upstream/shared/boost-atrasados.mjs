// O BOOST DOS ATRASADOS — para quem chegou depois.
//
// Quem entra num servidor que já tem meses chega numa conversa em andamento: os veteranos estão
// centenas de níveis acima, as hunts deles estão fechadas para ele, e o caminho até lá é o mesmo que
// eles fizeram, só que sozinho. O produto encurta esse caminho, e só ele: **XP ×10 (1000%) no
// treinador e nos pokémon enquanto o treinador estiver abaixo do nível-alvo**. Quando o treinador
// chega lá, o boost acaba sozinho.
//
// ### O nível-alvo é a MEDIANA de quem joga há tempo
//
// A mediana do nível de quem jogou nos últimos `diasAtivo` dias e tem conta há pelo menos
// `diasConta` dias (decisão do dono, 06/10/2026: "ativos com conta de 30+ dias"). É recalculada todo
// dia, então quem chegar daqui a um ano vê o alvo daquele dia — o produto acompanha o servidor.
//
// Mediana, e não média: a média é puxada para cima pelos poucos de nível altíssimo. E é a mediana dos
// VETERANOS porque a enxurrada de contas da semana puxaria o número para baixo — quem chega quer
// alcançar quem já está jogando, não quem chegou junto com ele.
//
// ### Justo para todo mundo
//
//   · ninguém passa do alvo com ele: é a metade de cima dos veteranos que continua na frente, e o
//     boost acaba no nível em que metade deles já está. Não fura ranking nenhum.
//   · o preço é PROPORCIONAL ao XP que falta: o salto inteiro (do nível 1) custa `precoCheio`, e quem
//     já tem parte do caminho paga só a parte que falta. Em XP, e não em níveis, porque a curva é
//     cúbica: os últimos níveis antes do alvo são a maior parte do tempo de jogo, e é esse tempo que o
//     produto economiza.
//   · o preço cheio NÃO cresce com o tempo: chegar à metade dos veteranos custa o mesmo hoje e daqui a
//     um ano, mesmo que o alvo de lá seja muito mais alto.
//
// O alvo fica GRAVADO na compra: se a mediana subir amanhã, o boost de quem já comprou continua
// acabando no nível que a tela prometeu. Quem chegou ao alvo e vê que a mediana subiu pode comprar de
// novo — e paga só o pedaço novo.
//
// ### O ×10 tem um tamanho, e ele é o XP até o alvo
//
// O boost multiplica o XP dos POKÉMON também, e quem o desliga é o nível do TREINADOR. Então o
// treinador tem de andar sempre para a frente enquanto ele dura — senão o ×10 dos pokémon não acaba:
//
//   · enquanto o boost vale, desmaiar (hunt, boss, PvP) e desistir do combate NÃO tiram XP do
//     treinador (`aplicarPerdaDeXpTreinador`, em server/game/morte-xp.mjs). Sem isto, quem parasse no
//     nível alvo−1 e morresse de propósito de tempos em tempos devolveria o XP do treinador e ficaria
//     com o ×10 nos pokémon para sempre (o caso que o dono levantou em 06/10/2026);
//   · o ganho que cruza o alvo leva o ×10 só até o XP exato do alvo; o resto do mesmo abate sai no
//     multiplicador normal (`ganhoAteOAlvo`, logo abaixo, que o `calcularXpGanho` do sim usa).
//
// Mora no shared porque a vitrine calcula o MESMO preço que o servidor cobra; quem decide é o
// servidor, que confere de novo antes de debitar.

export const ATRASADOS = {
  /** O multiplicador do XP do treinador e do XP do pokémon: 1000% = dez vezes o normal. */
  mult: 10,
  /** O salto inteiro, do nível 1 até o alvo. */
  precoCheio: 1000,
  /**
   * O piso: um pedaço pequeno demais para valer um produto ainda custa isto. Era 10; o dono subiu
   * para 50 em 06/10/2026, antes de ir ao ar — o boost de 10 💎 a um passo do alvo saía barato
   * demais para o ×10 que ele dá nos pokémon.
   */
  precoMinimo: 50,
  /**
   * O teto de qualquer multiplicador GRAVADO numa compra. O `mult` mora no save, junto com o alvo:
   * se um dia o save vier corrompido (ou editado à mão no banco), o jogo nunca multiplica por mais
   * do que isto. Ver `multAtrasados`.
   */
  multMaximo: 10,
  /** Entra na mediana quem jogou nos últimos `diasAtivo` dias… */
  diasAtivo: 7,
  /** …e tem conta há pelo menos `diasConta` dias. */
  diasConta: 30,
  /** Com menos veteranos ativos que isto a mediana não diz nada, e o produto não aparece. */
  minimoAmostra: 20,
};

/**
 * O XP TOTAL para estar no nível `L` — a curva do jogo.
 *
 * É a MESMA fórmula de `xpTotalParaNivel` (server/content.mjs) e de `xpTotalParaNivelCliente`
 * (client/app.js); `tools/teste-boost-atrasados.mjs` confere as três nível a nível. Está repetida
 * aqui porque a Loja não pode importar o `content.mjs` (o `content.mjs` é que importa a Loja).
 */
export const xpTotalDoNivel = (L) => (L <= 1 ? 0 : Math.round((50 / 3) * (L ** 3 - 6 * L ** 2 + 17 * L - 12)));

/**
 * Quanto custa, para quem tem `xpAtual`, ir até o nível `nivelAlvo`. `null` quando ele já chegou.
 *
 * `ceil` no proporcional: o jogador nunca paga menos que a fração do caminho que falta, e o piso
 * evita o boost de 1 💎 para quem está a um passo do alvo.
 */
export function precoAtrasados(xpAtual, nivelAlvo) {
  const alvo = xpTotalDoNivel(Math.floor(Number(nivelAlvo) || 0));
  const atual = Math.max(0, Number(xpAtual) || 0);
  if (!(alvo > 0) || atual >= alvo) return null;
  return Math.max(ATRASADOS.precoMinimo, Math.ceil(ATRASADOS.precoCheio * ((alvo - atual) / alvo)));
}

/**
 * O boost comprado está valendo? Só enquanto o treinador estiver abaixo do alvo gravado na compra.
 *
 * O alvo tem de ser um INTEIRO de verdade: um save com `alvo: Infinity` (ou `1e300`, ou texto) seria
 * um ×10 que nunca acaba, e quem desliga o boost é justamente o alvo. Alvo inválido = boost morto.
 */
export const atrasadosAtivo = (boost, nivel) => {
  const alvo = Number(boost?.alvo);
  return Number.isSafeInteger(alvo) && alvo > (Math.floor(Number(nivel)) || 0);
};

/**
 * O multiplicador de XP que o boost dá agora — 1 quando não há boost ou ele já acabou.
 *
 * O `mult` gravado nunca passa de `multMaximo`, e um valor que não é número (ou é menor que 1) vale o
 * da regra: o save não decide sozinho quanto XP o jogo dá.
 */
export const multAtrasados = (boost, nivel) => {
  if (!atrasadosAtivo(boost, nivel)) return 1;
  const m = Number(boost.mult);
  if (!Number.isFinite(m) || m < 1) return ATRASADOS.mult;
  return Math.min(m, ATRASADOS.multMaximo);
};

/**
 * O XP de um abate, com o boost indo até o XP do alvo e nem um ponto além.
 *
 * `multT`/`multP` são os multiplicadores COMPLETOS do treinador e do pokémon — com o do boost dentro,
 * como `multXpTreinador`/`multXpPokemon` (loja.mjs) devolvem —, e `xp` é o XP do treinador antes do
 * abate. No abate que cruza o alvo, só a fração `f` do ganho que cabe até lá leva o boost; o resto sai
 * no multiplicador sem ele — no treinador e, na mesma proporção, no pokémon. Sem o corte, um abate de
 * boss com todos os bônus empilhados (×10 × XP Boost × VIP × guild × evento…) levaria o treinador
 * vários níveis acima do alvo de uma vez.
 */
export function ganhoAteOAlvo(quantidade, multT, multP, boost, nivel, xp) {
  const mA = multAtrasados(boost, nivel);
  if (mA > 1) {
    const falta = Math.max(0, xpTotalDoNivel(Number(boost.alvo)) - (Number(xp) || 0));
    const cheio = quantidade * multT;
    if (cheio > falta) {
      // `cheio * f` é exatamente o que falta.
      const f = falta / cheio;
      return {
        treinador: Math.round(falta + quantidade * (1 - f) * (multT / mA)),
        pokemon: Math.round(quantidade * multP * f + quantidade * (1 - f) * (multP / mA)),
      };
    }
  }
  return { treinador: Math.round(quantidade * multT), pokemon: Math.round(quantidade * multP) };
}
