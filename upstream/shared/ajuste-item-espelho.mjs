/**
 * Ajustes NOSSOS por cima do espelho de `public/data/items.json` — preço e efeito.
 *
 * ### Por que aqui, e não editando o items.json
 *
 * A mesma razão de `game/itens-nossos.mjs` e do `AJUSTE_HUNT_LEVEL`: `public/data/` é espelho
 * regenerável e está no `.gitignore`. Um preço corrigido lá funciona na máquina de quem
 * editou e some no primeiro `npm run fetch` — inclusive em produção, onde o espelho é baixado
 * do zero, e sem deixar rastro de que o ajuste existia.
 *
 * ### Por que em `shared/`
 *
 * O catálogo tem DUAS entradas e as duas leem o espelho: o servidor mescla em `content.mjs`
 * (é ele quem cobra e quem aplica o efeito) e o cliente baixa `/assets/items.json` direto no
 * carregamento (é ele quem escreve o preço e o "restaura X% do HP" no card do Market). Um
 * ajuste em só um dos lados vira card mentindo sobre o que o servidor vai fazer, então os dois
 * passam por esta mesma função.
 */

/** Revive comum — id do espelho. */
export const REVIVE_ID = 205;

/** Max Revive — id do espelho. */
export const MAX_REVIVE_ID = 206;

/**
 * itemId do espelho → `npcPrice` que vale de verdade.
 *
 * O Max Revive devolve 100% do HP: é o efeito mais forte do jogo e o único que apaga por
 * inteiro o custo de ter caído. A 2.500 ele era só o dobro do Revive comum, e por isso não
 * havia razão para comprar o comum. A 5.000 a escolha volta a existir.
 */
export const AJUSTE_NPC_PRICE = new Map([
  [MAX_REVIVE_ID, 5000],
]);

/**
 * itemId do espelho → `revivePct` que vale de verdade.
 *
 * O Revive comum vinha do espelho devolvendo METADE do HP máximo, e isso apagava a poção do
 * jogo: levantar já era ficar com meia barra, o suficiente para voltar à hunt e seguir
 * caçando. Quem tinha Revive na bolsa não tinha razão nenhuma para comprar Potion.
 *
 * A 10% ele volta a ser só o que o nome diz — o item que tira o pokémon do chão — e a poção
 * passa a ser o que recompõe a barra depois. O Max Revive segue devolvendo 100%: é ele quem
 * paga para não precisar da poção.
 */
export const AJUSTE_REVIVE_PCT = new Map([
  [REVIVE_ID, 0.1],
]);

/** O item com preço e efeito ajustados — o MESMO objeto quando não há ajuste para ele. */
export function ajustarItemEspelho(item) {
  const preco = AJUSTE_NPC_PRICE.get(item?.id);
  const pct = AJUSTE_REVIVE_PCT.get(item?.id);
  const mudaPreco = preco != null && item.npcPrice !== preco;
  const mudaPct = pct != null && item.revivePct !== pct;
  if (!mudaPreco && !mudaPct) return item;
  return {
    ...item,
    ...(mudaPreco ? { npcPrice: preco } : {}),
    ...(mudaPct ? { revivePct: pct } : {}),
  };
}
