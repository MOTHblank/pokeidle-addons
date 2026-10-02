// O AVISO DE MUDANÇA — o recado que abre uma vez, no primeiro login depois de um deploy que
// mexeu em algo que o jogador já usava.
//
// É irmão do `discord-pop.mjs` e a mecânica é a mesma: um NÚMERO de versão aqui, o último
// número visto gravado no jogador (`players.aviso_visto`), e o pop-up aparece enquanto o
// segundo for menor que o primeiro. Fechar carimba o número atual e ele não volta.
//
// ### Por que no banco, e não no `localStorage`
//
// "Uma vez" tem de valer para a PESSOA, não para o navegador. No `localStorage` o mesmo
// jogador veria o aviso de novo no celular, na aba anônima e depois de limpar o cache — e um
// recado sobre diamante creditado na conta não pode dar a impressão de que foi creditado três
// vezes.
//
// ### Por que uma versão, e não um booleano
//
// O próximo aviso não vai ser este. Com um número, subir o valor abaixo é tudo o que o
// próximo anúncio precisa: quem já leu o anterior volta a ver, e quem nunca leu nada vê só o
// mais recente. Um booleano obrigaria a limpar a coluna de 5.800 linhas a cada recado.
//
// ### Conta nova não vê
//
// Quem criou a conta depois do deploy nunca pescou, e abrir o jogo pela primeira vez com um
// "fechamos a Pesca" no meio da tela é ruído. O `INSERT` de `players` (em `db.mjs`) já nasce
// com `aviso_visto` no valor de agora, então o recado só alcança quem já estava aqui.

/**
 * AS NOVIDADES, da mais recente para a mais antiga. Cada uma é um slide do modal "Novidades no
 * PokéIdle" (`#aviso-jogo`): a `<section class="novidade" data-aviso="N">` mora no `index.html` e
 * os textos `aviso.*` no `i18n.mjs`. O botão "!" abre o modal na primeira desta lista, e as setas
 * voltam para as antigas.
 *
 * **Novidade nova = uma entrada NO TOPO**, com `aviso` um número acima do anterior. É ela que vira
 * o `AVISO_VERSAO`, e quem ainda não a leu vê o modal sozinho no próximo login.
 *
 *   aviso   o número que o jogador carimba ao fechar (`players.aviso_visto`)
 *   versao  a versão do jogo que trouxe a novidade — vai no título
 *   data    AAAA-MM-DD, o dia em que ela entrou no ar — vai no título, no formato do idioma
 *   ok      a chave do texto do botão de fechar, enquanto ela está na tela
 */
export const NOVIDADES = [
  // O BÔNUS NA KICK: vincular a Kick (para sempre — uma conta da Kick por treinador) e trocar os
  // PONTOS DO CANAL dos Streamers Oficiais por horas de +15% em XP, Capture Boost e Secret Lure,
  // que se somam. Com o convite de Streamer Parceiro da Kick no meio do slide, como o da Twitch
  // na 18. Junto: os diálogos virando cena de jogo, a barraca do Mercado, o shiny que brilha na
  // Equipe, a mensagem fixada do chat, o Mimikyu em Alola e os links da Mega/evolução na ficha.
  { aviso: 22, versao: '1.206.0', data: '2026-10-01', ok: 'aviso.v22Ok' },
  // A COMISSÃO DO MERCADO: em gema, o POKÉMON passa a pagar por faixa marginal (13% / 10% / 7,5%)
  // no lugar dos 15% planos, e o Coins sobe de 10% para 15% — os dois lados da mesma conta, e os
  // dois no slide, porque descobrir o aumento na hora de vender seria pior. Junto: a ficha do Boss
  // dizendo como capturar pelo MysticTicket, a recarga de golpe que pausa fora de campo (o exploit
  // relatado no Discord), a busca de treinador, a fila do PvP que sobrevive ao deploy e o resto.
  { aviso: 21, versao: '1.202.0', data: '2026-10-01', ok: 'aviso.v21Ok' },
  // O MYSTICTICKET e os MYSTERY EGGS: o ticket raríssimo dos Bosses que abre a Arena Mística com
  // um Lendário ou Mítico nível 100 (uma bola só), e os ovos P2/P3/P4 do Market NPC que chocam em
  // 16/32/48 h na Chocadeira. Junto: a Tag IDLE do Discord com os botões novos do #reedem-codes,
  // os troféus de campeonato, o contador de favoritos do Mercado e o Banshee Mismagius só GHOST.
  { aviso: 20, versao: '1.182.0', data: '2026-09-30', ok: 'aviso.v20Ok' },
  // O CRAFT DE CASAS no Professor Carvalho: 5 casas da mesma raridade viram 1 da seguinte, só até
  // a Rara. Junto: o Boss automático que empilha os drops e fecha com o resumo da série, os golpes
  // novos de Electivire/Lucario/Mr. Mime/Drifblim/Camerupt e o Tracker com combate turno a turno,
  // temporadas e 30 dias de histórico. (O item "nota acima de 4 só em Gemas" saiu do slide em
  // 30/09/2026, junto com a regra.)
  { aviso: 19, versao: '1.179.0', data: '2026-09-29', ok: 'aviso.v19Ok' },
  // O BÔNUS TWITCH: vincular a Twitch e assistir a um Streamer Oficial (o faasii ou qualquer canal
  // da aba Contas Twitch do painel) dá +15% de XP Treinador e XP Pokémon. Com o convite para virar
  // Streamer Parceiro no meio do slide — o público dele está justamente lendo. Junto: as 117 formas
  // shiny novas e o histórico do PvP que abre a ficha da partida.
  { aviso: 18, versao: '1.175.0', data: '2026-09-28', ok: 'aviso.v18Ok' },
  // A ESCALAÇÃO SECRETA do PvP Ranqueado: a ficha de outro treinador (e o Tracker) mostra QUAIS
  // pokémon ele leva, mas não em que ordem entram. Junto: os pacotes de 1.000 e 2.000 diamantes,
  // o meta por rank no Tracker, a confirmação do Professor Carvalho, a mega indo para a Coleção
  // e o /guia novo. A campanha das redes já passou do prazo (27/09), e esta volta a ser a regra
  // de sempre: entrada no topo, `aviso` um acima do maior.
  { aviso: 17, versao: '1.172.0', data: '2026-09-28', ok: 'aviso.v17Ok' },
  // ATENÇÃO à ordem destas duas: o número do `aviso` NÃO segue a versão aqui, e é de propósito.
  //
  // A CAMPANHA DAS REDES é a que abre o modal, porque ela tem PRAZO — o post está no ar agora, e
  // uma novidade de produto (a Name Tag, que fica na Loja para sempre) não pode empurrá-la para
  // trás de uma seta justamente na semana em que ela vale. Como quem abre o modal é `NOVIDADES[0]`
  // e quem decide o carimbo é o `aviso` mais alto, pôr a campanha na frente exige dar a ELA o 16.
  //
  // O efeito colateral é que o slide de v1.162.0 carrega o carimbo 15 e o de v1.161.0 o 16. Não
  // atrapalha nada — `versao` e `data` do título vêm de cada entrada, e o carimbo só serve para
  // "já li isto". Quem acrescentar a PRÓXIMA novidade continua fazendo o de sempre: entrada no
  // topo, com `aviso` um acima do maior daqui (17).
  //
  // A CAMPANHA DAS REDES: seguir no Instagram e no X, com o resgate por ticket no Discord.
  { aviso: 16, versao: '1.161.0', data: '2026-09-23', ok: 'aviso.v15Ok' },
  // A NAME TAG: a etiqueta de 15 💎 que dá um nome COSMÉTICO a um pokémon, na seção Cosméticos
  // — prateleira nova da Loja de diamantes. O nome aparece na ficha, em campo, na praça, no
  // chat e no Mercado, e atravessa a venda junto com o bicho.
  { aviso: 15, versao: '1.162.0', data: '2026-09-23', ok: 'aviso.v16Ok' },
  // A GUILD SEM TETO DE MEMBROS e o TIME de 10 que o dono escala para a Guerra de Guilds — com
  // o painel da guild refeito em faixa + abas. Junto: as automações de cuidado ligadas por
  // padrão, a TAG da guild no chat, as duas dicas que o chat dá sozinho a cada 30 min e o
  // ícone de Coins do Mercado.
  { aviso: 14, versao: '1.154.0', data: '2026-09-22', ok: 'aviso.v14Ok' },
  // AS MEGA EVOLUÇÕES: as 35 formas na aba Mega da Pokédex (#3000), os dois fragmentos que caem
  // em boss, as 70 pedras negociáveis e a bancada do Professor que transforma uma coisa na
  // outra. Junto: o shiny do Totodile e os radares do bot no Discord.
  { aviso: 13, versao: '1.146.0', data: '2026-09-22', ok: 'aviso.v13Ok' },
  // CONVIDE & GANHE (a escada de convites do Discord, com o bot contando quem entra por
  // quem) e o CAMPEONATO AMADOR — o segundo torneio do mes, sem shiny e sem P5, com o
  // Mundial virando mensal e o Torneio abrindo numa lista de campeonatos.
  { aviso: 12, versao: '1.137.0', data: '2026-09-21', ok: 'aviso.v12Ok' },
  // O PASSE DE BATALHA (a trilha de 30 dias, grátis e VIP), o PvP amistoso entre amigos, a análise
  // da última Guerra de Guilds e o resto da leva: histórico do Mercado com P·IV·Q·N, o filtro de
  // Outland, boosts sem limite diário e o menu com Torneio e RMT.
  { aviso: 11, versao: '1.128.0', data: '2026-09-18', ok: 'aviso.v11Ok' },
  // O MODO ECONOMIA (o palco sem cena) com a Tela sempre acesa, o PvP Ranqueado semanal e tudo o
  // que entrou desde a v1.120.0: Oferenda Rápida, caixas mais baratas, a diária do Mercado a
  // 100.000 e a tipagem das 38 espécies da Pokédex.
  { aviso: 10, versao: '1.127.0', data: '2026-09-18', ok: 'aviso.v10Ok' },
  // As CAIXAS DO MARKET — o ralo de Coins — e a Área de Treinamento, a bancada de testes do PvP.
  { aviso: 9, versao: '1.119.0', data: '2026-09-17', ok: 'aviso.v9Ok' },
  // Um mês de jogo: o agradecimento com o que a comunidade movimentou em USDT, e o que entrou
  // desde a novidade 7 — a Coleção, o celular refeito, a guerra mais longa e as correções.
  { aviso: 8, versao: '1.116.0', data: '2026-09-17', ok: 'aviso.v8Ok' },
  // O Campeonato abrindo as inscrições, a Oferenda de Pokémon e as correções que foram juntas.
  { aviso: 7, versao: '1.104.0', data: '2026-09-16', ok: 'aviso.v7Ok' },
  { aviso: 6, versao: '1.99.0', data: '2026-09-15', ok: 'aviso.v6Ok' },
  // Era a 4 (v1.94.0, retenção de 8 h o dia todo). Virou a 5 no mesmo dia, com a retenção pelo
  // horário: quem já tinha fechado a 4 precisa ler a regra certa.
  { aviso: 5, versao: '1.96.0', data: '2026-09-15', ok: 'aviso.v4Ok' },
  { aviso: 3, versao: '1.88.0', data: '2026-09-14', ok: 'aviso.v3Ok' },
  { aviso: 2, versao: '1.86.0', data: '2026-09-13', ok: 'aviso.ok' },
  { aviso: 1, versao: '1.69.0', data: '2026-09-11', ok: 'aviso.pescaOk' },
];

/** Versão do aviso atual — sempre a da novidade mais recente. */
export const AVISO_VERSAO = NOVIDADES[0].aviso;

/** @param {number|undefined|null} vista o último aviso que o jogador fechou */
export function avisoDeveMostrar(vista) {
  return (Number(vista) || 0) < AVISO_VERSAO;
}
