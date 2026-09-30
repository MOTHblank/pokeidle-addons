/**
 * SKINS DE POKÉMON — a fantasia que troca o DESENHO de um pokémon e mais nada.
 *
 * ### O que se compra
 *
 * Uma skin é um ITEM da bolsa (faixa `73000 + n`, e `73500 + n` na shiny), e fica lá para sempre —
 * aba Skins. Com ela o jogador veste UM pokémon da espécie e da COR certas; tirar devolve o sprite
 * normal. Quem tem duas
 * unidades veste dois bichos. E, sendo item, a unidade livre pode ir para o Mercado da
 * Comunidade — é o que faz a skin de uma venda limitada continuar existindo depois dela.
 *
 * ### A unidade NUNCA sai da bolsa
 *
 * Vestir não move o item para o pokémon, como a Exp. Share faz com o `held_item_id`. É só uma
 * MARCAÇÃO — "este pokémon usa uma das minhas skins X" —, guardada no jogador
 * (`players.skins_vestidas`, `{ pokemonId: skinId }`), e a regra é uma desigualdade: nunca há
 * mais pokémon vestidos de X do que unidades de X na bolsa.
 *
 * A razão é a história da Exp. Share (ver `ehXpShareHeldMercado` em `content.mjs`): item que
 * mora em DUAS tabelas duplica, e item negociável que duplica espalha a cópia pela comunidade.
 * Com a unidade num lugar só, não há o que partir ao meio. O Mercado anuncia só a unidade
 * LIVRE (`skinsLivres` no sim), e se alguma unidade sair da bolsa por outro caminho a marcação
 * excedente é que cai, nunca a bolsa.
 *
 * A marcação é do DONO, não da linha do pokémon: vender o bicho não leva a skin junto — o
 * comprador recebe a cara de sempre, e a unidade continua na bolsa de quem pagou.
 *
 * ### A espécie tem de bater, e mega é outra espécie
 *
 * `pokeId` é a espécie que VESTE a skin. As megas são espécies de verdade na faixa #3000 (ver
 * `megas.mjs`), então "Dragonite Uchiha" e "Mega Dragonite Uchiha" são duas skins, com duas
 * artes e duas compras. Quem megaevolui um Dragonite vestido vê o bicho sem a fantasia — a skin
 * não sumiu, só não serve mais nele, e volta a ficar livre no inventário.
 *
 * ### Shiny É outra skin
 *
 * Decisão do dono do jogo (25/09/2026): cada SPRITE é uma skin. Um desenho ("Dragonite Uchiha")
 * vira duas — a comum, que só veste Dragonite comum, e a shiny, que só veste Shiny Dragonite —,
 * cada uma com o seu item, o seu preço e a sua compra. A cor tem de bater nos dois sentidos: uma
 * skin shiny num bicho comum faria um pokémon comum PASSAR por shiny na hunt e no PvP.
 *
 * ### Os looktypes
 *
 * `95000 + n` para a skin comum e `95500 + n` para a shiny. A faixa estava vazia no índice de
 * outfits e fica longe das megas (80000/85000) e dos NPCs (909xx). A arte é SPRITE como qualquer
 * outra: atlas WEBP no pack (`public/data/asset-packs`), entrada `kind: 'skin'` no
 * `outfits-index.json`, publicada por `npm run publicar:skins` (`tools/publicar-sprites-skins.mjs`)
 * a partir do `Pokedex Backup/SKINS/<evento>`, e levada à produção pelo `deploy-public-data.ps1`.
 *
 * ### Venda limitada
 *
 * Cada skin pertence a um EVENTO, e o evento tem janela de venda. Fora dela a Loja não vende —
 * nem antes (a vitrine mostra a contagem), nem depois (a skin some da vitrine). Quem comprou
 * continua com ela para sempre. As datas são do servidor; a tela só desenha a contagem.
 */

/** Quanto cada skin custa na Loja de diamantes. */
export const SKIN_PRECO = 30;

export const SKIN_LOOKTYPE_BASE = 95000;
export const SKIN_LOOKTYPE_SHINY_BASE = 95500;

/**
 * O item de cada skin: `73000 + n` na comum e `73500 + n` na shiny — o mesmo desenho dos
 * looktypes (95000/95500), com 499 desenhos de folga em cada metade.
 *
 * NÃO é a faixa 70100: ela é do ESPELHO. O `tools/importar-itens-hugh.mjs` numera o loot que
 * importa a partir de 70100 (Bear Arm é 70101, Mystic Flower 70151…, e segue crescendo), e uma
 * skin no mesmo id SUBSTITUI o item no catálogo — a Mystic Flower de todo mundo viraria skin, e
 * todo drop dela também. As megas usam 71000–72999 (`itens-nossos.mjs`). O `teste-loja` confere
 * que nenhum item nosso cai num id do espelho.
 */
export const SKIN_ITEM_BASE = 73000;
export const SKIN_ITEM_SHINY_BASE = 73500;

/**
 * Os eventos e as janelas de venda.
 *
 * `abreEm`/`fechaEm` em UTC, e a meia-noite é a de Brasília (UTC−3): o Halloween vende de
 * 01/10 00:00 até 31/10 23:59:59. `fechaEm` é EXCLUSIVO.
 */
export const EVENTOS_SKIN = {
  halloween2026: {
    id: 'halloween2026',
    abreEm: Date.UTC(2026, 9, 1, 3, 0, 0),
    fechaEm: Date.UTC(2026, 10, 1, 3, 0, 0),
  },
};

/**
 * Os DESENHOS — cada um vira duas skins, a comum e a shiny (ver `SKINS` logo abaixo).
 *
 *   id       chave do desenho; a skin comum usa este id e a shiny `<id>_shiny`. Nunca renomeie:
 *            é o que o banco guarda
 *   n        o número do looktype e do item (95000/95500 + n, 73000/73500 + n); nunca reaproveite
 *   pokeId   a espécie que veste (mega = 3000 + dex)
 *   especie  o nome da espécie, para o texto de reserva do servidor (a tela usa o catálogo dela)
 *   estilo   a fantasia — a chave `skin.estilo.<estilo>` no dicionário
 *   quadro   a largura do quadro da folha, a MESMA da arte da espécie no jogo
 *   alto     a altura do quadro, quando ele não é quadrado (o Tyranitar é 64×96)
 */
export const DESENHOS_SKIN = [
  { id: 'venusaur_halloween', n: 1, pokeId: 3, especie: 'Venusaur', estilo: 'halloween', quadro: 64 },
  { id: 'mega_venusaur_robinhood', n: 2, pokeId: 3003, especie: 'Mega Venusaur', estilo: 'robinhood', quadro: 64 },
  { id: 'charizard_ender', n: 3, pokeId: 6, especie: 'Charizard', estilo: 'ender', quadro: 64 },
  { id: 'mega_charizard_dracula', n: 4, pokeId: 3006, especie: 'Mega Charizard Y', estilo: 'dracula', quadro: 128 },
  { id: 'mega_charizard_ender', n: 5, pokeId: 3006, especie: 'Mega Charizard Y', estilo: 'ender', quadro: 128 },
  { id: 'blastoise_raphael', n: 6, pokeId: 9, especie: 'Blastoise', estilo: 'raphael', quadro: 64 },
  { id: 'mega_blastoise_raphael', n: 7, pokeId: 3009, especie: 'Mega Blastoise', estilo: 'raphael', quadro: 64 },
  { id: 'gengar_dracula', n: 8, pokeId: 94, especie: 'Gengar', estilo: 'dracula', quadro: 64 },
  { id: 'mega_gengar_dracula', n: 9, pokeId: 3094, especie: 'Mega Gengar', estilo: 'dracula', quadro: 64 },
  { id: 'gyarados_dragao_chines', n: 10, pokeId: 130, especie: 'Gyarados', estilo: 'dragaoChines', quadro: 96 },
  { id: 'mega_gyarados_dragao_chines', n: 11, pokeId: 3130, especie: 'Mega Gyarados', estilo: 'dragaoChines', quadro: 128 },
  { id: 'dragonite_uchiha', n: 12, pokeId: 149, especie: 'Dragonite', estilo: 'uchiha', quadro: 64 },
  // 96 desde 25/09/2026: a Mega Dragonite ganhou arte nova (96×96, 4 quadros) e o manto foi
  // vestido de novo sobre ela — a folha de 64 era do desenho antigo.
  { id: 'mega_dragonite_uchiha', n: 13, pokeId: 3149, especie: 'Mega Dragonite', estilo: 'uchiha', quadro: 96 },
  { id: 'tyranitar_jason', n: 14, pokeId: 248, especie: 'Tyranitar', estilo: 'jason', quadro: 64, alto: 96 },
  // 64×64, a mesma folha da Mega Tyranitar do jogo, desde 25/09/2026: a fantasia foi refeita
  // SOBRE a arte da mega (jaqueta no tronco, máscara na cara, facão na mão) e nada passa do
  // quadro. A versão de antes era 64×96 porque a máscara ia em cima da cabeça.
  { id: 'mega_tyranitar_jason', n: 15, pokeId: 3248, especie: 'Mega Tyranitar', estilo: 'jason', quadro: 64 },
];

/**
 * As SKINS — o que se compra, se guarda na bolsa e se veste. Duas por desenho, a comum e a shiny.
 *
 *   desenho  o id do desenho de origem (`DESENHOS_SKIN`)
 *   shiny    true = só veste o pokémon SHINY da espécie; false = só o comum
 *   looktype a arte DESTA skin (uma só: a skin shiny é a arte shiny)
 */
export const SKINS = DESENHOS_SKIN.flatMap((d) => [false, true].map((shiny) => {
  const id = shiny ? `${d.id}_shiny` : d.id;
  return {
    ...d,
    id,
    desenho: d.id,
    shiny,
    evento: 'halloween2026',
    preco: SKIN_PRECO,
    itemId: (shiny ? SKIN_ITEM_SHINY_BASE : SKIN_ITEM_BASE) + d.n,
    looktype: (shiny ? SKIN_LOOKTYPE_SHINY_BASE : SKIN_LOOKTYPE_BASE) + d.n,
    // O retrato parado (quadro de frente, sem moldura): é o ícone do ITEM na bolsa e no Mercado.
    // Mora com os outros ícones de item nossos, em `img/itens/` — gerado por `tools/gerar-skins.py`.
    icone: `/img/itens/skins/${id}.png`,
  };
}));

export const SKIN_POR_ID = new Map(SKINS.map((s) => [s.id, s]));

/** O looktype é de uma skin (das 95000 ou das 95500)? Quem pergunta é o `sprites.mjs`, para o recorte. */
export const ehLooktypeSkin = (lt) => {
  const n = Number(lt);
  return n > SKIN_LOOKTYPE_BASE && n < SKIN_LOOKTYPE_BASE + 1000;
};
export const SKIN_POR_ITEM = new Map(SKINS.map((s) => [s.itemId, s]));

export const ehItemSkin = (itemId) => SKIN_POR_ITEM.has(Number(itemId));

/** O teto de pokémon vestidos que a marcação guarda — defesa contra pacote forjado, não regra de jogo. */
export const MAX_SKINS_VESTIDAS = 500;

/** O texto do estilo em português — o nome de reserva do servidor, que não sabe o idioma da aba. */
export const ESTILO_PT = {
  halloween: 'Halloween',
  robinhood: 'Robin Hood',
  ender: 'Ender',
  dracula: 'Drácula',
  raphael: 'Raphael',
  dragaoChines: 'Dragão Chinês',
  uchiha: 'Uchiha',
  jason: 'Jason',
};

export const nomeSkinPt = (s) => `${s.shiny ? 'Shiny ' : ''}${s.especie} ${ESTILO_PT[s.estilo] ?? s.estilo}`;

/** O mesmo em inglês — o nome do ITEM no catálogo do servidor, que é em inglês como o resto. */
const ESTILO_EN = { ...ESTILO_PT, dracula: 'Dracula', dragaoChines: 'Chinese Dragon' };
export const nomeSkinEn = (s) => `${s.shiny ? 'Shiny ' : ''}${s.especie} ${ESTILO_EN[s.estilo] ?? s.estilo}`;

/** A janela de venda de uma skin (`{abreEm, fechaEm}`), ou `null` se o evento não existe. */
export const janelaDaSkin = (s) => EVENTOS_SKIN[s?.evento] ?? null;

/** 'antes' | 'aberta' | 'encerrada' — onde a venda desta skin está agora. */
export function situacaoDaVenda(s, agora, janela = janelaDaSkin(s)) {
  if (!janela) return 'encerrada';
  if (agora < janela.abreEm) return 'antes';
  if (agora >= janela.fechaEm) return 'encerrada';
  return 'aberta';
}

/** A skin serve neste pokémon? Espécie E cor (a skin shiny só no shiny, a comum só no comum). */
export const skinServe = (s, pk) =>
  !!s && !!pk && Number(pk.speciesId) === s.pokeId && !!pk.shiny === s.shiny;

/**
 * A skin que este pokémon está VESTINDO de verdade, ou `null`.
 *
 * `pk.skin` diz qual o dono pôs; esta função é a que decide se ela aparece. Espécie e cor têm de
 * bater — um Dragonite Uchiha que megaevoluiu continua com `skin` no objeto até o servidor
 * limpar, e desenhar a folha do Dragonite em cima de uma Mega Dragonite seria a arte errada.
 */
export function skinVestida(pk) {
  const s = pk?.skin ? SKIN_POR_ID.get(pk.skin) : null;
  return skinServe(s, pk) ? s : null;
}

/** O looktype que a skin desenha num pokémon, ou `null` sem skin. */
export function looktypeDaSkin(pk) {
  return skinVestida(pk)?.looktype ?? null;
}

/**
 * A marcação LIMPA: `{ pokemonId: skinId }` só com o que pode ser verdade agora.
 *
 * Cai quem não é mais do jogador (vendido, anunciado, oferendado), quem mudou de espécie (evoluiu,
 * megaevoluiu), a skin de outra cor, a que não existe mais no catálogo e o EXCEDENTE — mais pokémon vestidos de
 * uma skin do que unidades dela na bolsa. No excedente fica o pokémon de id menor, o mais antigo:
 * a ordem tem de ser a mesma em toda chamada, senão a skin pularia de bicho a cada releitura.
 *
 * Devolve um objeto NOVO; não mexe em nada. `pokemons` é o `Map` id → pokémon do jogador.
 */
export function normalizarVestidas(vestidas, pokemons, items) {
  const limpa = {};
  const usadas = new Map();
  const ids = Object.keys(vestidas ?? {}).map(Number).filter(Number.isFinite).sort((a, b) => a - b);
  for (const id of ids) {
    const skinId = vestidas[id];
    const s = SKIN_POR_ID.get(skinId);
    const pk = pokemons?.get(id);
    if (!skinServe(s, pk)) continue;
    const n = usadas.get(skinId) ?? 0;
    if (n >= (Number(items?.[s.itemId]) || 0)) continue;
    usadas.set(skinId, n + 1);
    limpa[id] = skinId;
  }
  return limpa;
}

/** Quantas unidades de cada skin estão vestidas agora — `Map` itemId → n. */
export function unidadesEmUso(vestidas) {
  const m = new Map();
  for (const skinId of Object.values(vestidas ?? {})) {
    const s = SKIN_POR_ID.get(skinId);
    if (s) m.set(s.itemId, (m.get(s.itemId) ?? 0) + 1);
  }
  return m;
}

/**
 * O recorte do `encaixar` (Loja, bolsa, folha da skin) só conta pixel SÓLIDO — alfa acima disto.
 *
 * As skins do Ender têm uma aura roxa semitransparente (alfa até ~200) que chega à borda do
 * quadro: contada no recorte, ela deixava o Mega Charizard Y Ender miúdo no card e punha dois
 * riscos roxos em cima do Charizard Ender. A aura continua desenhada — só não decide o tamanho.
 * Nas skins sem aura o recorte é o mesmo com qualquer limite.
 */
export const ALFA_RECORTE_SKIN = 200;

