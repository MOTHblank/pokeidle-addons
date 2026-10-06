// A NOTA da Guerra de Guilds — o que decide a colocação e, por ela, o GP.
//
// ### Como chegou aqui
//
// Até 06/10/2026 a colocação era só a ordem de queda: a última guild de pé era a 1ª, a penúltima a
// 2ª, e assim por diante. Com ~100 guilds na arena isso premiava DURAR, e não brigar: guild de dois
// membros que passava minutos longe da briga terminava no top 20, na frente de guilds de dez que
// derrubaram dezenas de pokémon e caíram lutando (em 05/10/2026 a Rocket da Shopee, 2 membros e 10
// abates, foi a 12ª, e a xibiu, 10 membros e 79 abates, a 19ª).
//
// A primeira resposta (v1.234.0, nunca publicada) colocou só por PONTOS de combate (abates e dano),
// com a ordem de queda como último desempate. O dono recusou: assim a última guild VIVA podia perder a
// guerra (em 05/10 a BRUTALITY, a última de pé, terminaria em 3º), e sobreviver tem de pesar MAIS. A
// decisão dele (06/10/2026): **60% sobrevivência, 40% combate**, e o combate como CONTADOR de pokémon
// derrubados pela guild, na mesma conta da sobrevivência — sem dano (a primeira versão do 60/40 usava
// os pontos de abate e dano na proporção da melhor guild, e ele achou a conta difícil de entender).
//
// ### A nota, de 0 a 100
//
//   · SOBREVIVÊNCIA, até `PESO_SOBREVIVENCIA` (60): 60 ÷ √(posição de queda), contando da última de
//     pé. A última leva 60, a 2ª 42,4, a 3ª 34,6, a 5ª 26,8, a 10ª 19, a 20ª 13,4, a 50ª 8,5.
//   · COMBATE, até `PESO_COMBATE` (40): 40 ÷ √(posição da guild em pokémon derrubados) — todo pokémon
//     inimigo que QUALQUER membro derruba conta para a guild. A que mais derrubou leva 40, a 2ª 28,3, a
//     3ª 23,1, a 10ª 12,6. Guilds com o mesmo número de abates dividem a posição; quem não derrubou
//     nenhum leva 0.
//   · A NOTA é a soma das duas, cada uma arredondada a uma casa (a conta da tela fecha), e a
//     colocação é a ordem da nota. Empate: quem caiu por último, depois quem derrubou mais.
//
// ### Por que a raiz, e não uma reta
//
// Numa reta (60 para a 1ª, 0 para a última, o mesmo degrau em cada posição) cada posição de
// sobrevivência vale 0,6 com 99 guilds: sobreviver à penúltima não paga quase nada, e o combate
// decide o topo — exatamente a reclamação. Com o tempo de pé no lugar da posição é pior: a penúltima
// cai no instante em que a guerra acaba e empata com a vencedora. A raiz põe o peso no TOPO, onde a
// disputa é de verdade, e deixa o meio da tabela decidido pelas duas coisas juntas. Medida nas 9
// guerras reais de 26/09 a 05/10/2026: a última de pé termina em 1º nas 9, e as guilds de 1 a 3
// membros no top 20 caem de 1–6 por guerra (ordem de queda) para 0–4 (em 7 das 9, no máximo 2).
//
// O GP continua saindo da colocação (`gpPorColocacao`: N guilds → 1º +N … último +1).
//
// Mora no shared porque a tela lê os mesmos números: o painel do PvP Guild mostra a nota com a conta
// aberta e a Wiki a documenta. Com o número escrito nos dois lados, o primeiro ajuste deixaria a
// tela prometendo uma conta que o servidor não faz.

/** Até quanto da nota vale ficar de pé — a última guild viva leva tudo isto. */
export const PESO_SOBREVIVENCIA = 60;

/** Até quanto da nota vale derrubar — a guild que mais derrubou pokémon leva tudo isto. */
export const PESO_COMBATE = 40;

/** A nota de sobrevivência de quem caiu na posição `queda` (1 = a última de pé): 60 ÷ √queda. */
export function notaSobrevivencia(queda) {
  const q = Math.max(1, Math.floor(Number(queda) || 1));
  return PESO_SOBREVIVENCIA / Math.sqrt(q);
}

/**
 * A nota de combate de quem ficou na posição `pos` em abates (1 = a que mais derrubou): 40 ÷ √pos.
 * Quem não derrubou nenhum (`abates` 0) leva 0, qualquer que seja a posição.
 */
export function notaCombate(pos, abates) {
  if (!((Number(abates) || 0) > 0)) return 0;
  const p = Math.max(1, Math.floor(Number(pos) || 1));
  return PESO_COMBATE / Math.sqrt(p);
}

/**
 * A posição de cada guild em abates: 1 + quantas derrubaram MAIS — guilds com o mesmo número dividem
 * a posição (30, 30 e 10 abates = 1º, 1º e 3º). Devolve `abates` (os da guild) → posição.
 */
export function posicaoEmAbates(placar) {
  const todos = (placar ?? []).map((g) => Math.max(0, Math.floor(Number(g.abates) || 0)));
  return (abates) => 1 + todos.filter((k) => k > abates).length;
}

/**
 * O placar da simulação recolocado pela NOTA.
 *
 * Entra o `placar` de `simularGuerra`, em que `pos` é a ordem de queda (e, no teto de tempo, a das
 * sobreviventes pela saúde, na frente das que caíram). Sai uma lista nova, na ordem nova, com:
 *
 *   · `queda` — a posição de sobrevivência (1 = a última de pé), que é o `pos` que entrou;
 *   · `posAbates` — a posição em pokémon derrubados (empate divide);
 *   · `sob`, `com` e `nota` — as notas com uma casa; a nota é a soma das duas JÁ arredondadas;
 *   · `pos` — a colocação final, reescrita de 1 a N.
 */
export function colocarNaGuerra(placar) {
  const posDe = posicaoEmAbates(placar);
  return (placar ?? [])
    .map((g, i, todos) => {
      const abates = Math.max(0, Math.floor(Number(g.abates) || 0));
      const queda = Math.max(1, Math.floor(Number(g.pos) || todos.length));
      const posAbates = posDe(abates);
      // Em décimos inteiros: a soma de dois números de uma casa não pode virar 72,39999.
      const sob = Math.round(notaSobrevivencia(queda) * 10);
      const com = Math.round(notaCombate(posAbates, abates) * 10);
      return { ...g, queda, posAbates, sob: sob / 10, com: com / 10, nota: (sob + com) / 10, decimos: sob + com, abatesN: abates };
    })
    .sort((a, b) => b.decimos - a.decimos || a.queda - b.queda || b.abatesN - a.abatesN)
    .map(({ decimos, abatesN, ...g }, i) => ({ ...g, pos: i + 1 }));
}
