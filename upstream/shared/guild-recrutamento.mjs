/**
 * Como se ENTRA numa guild — a política, os requisitos e a espera de quem sai.
 *
 * ### O que isto substitui
 *
 * Até aqui, entrar em guild era um só caminho: alguém com mando digitava o seu nick. Quem
 * queria guild anunciava no chat do mundo e torcia para o líder certo estar online no mesmo
 * minuto — a mensagem subia em dez segundos, e quem chegasse depois não via nada. O recrutamento
 * do jogo inteiro morava numa caixa de texto que apaga sozinha.
 *
 * Agora a guild DIZ como ela recebe, e a tela mostra isso numa lista:
 *
 *   · `aberta`     — qualquer um entra, na hora, sem pedir nada a ninguém;
 *   · `requisitos` — qualquer um entra na hora DESDE QUE cumpra o que o líder exigiu;
 *   · `fechada`    — só entra quem foi convidado, mas qualquer um pode se CANDIDATAR, e o
 *                    pedido fica esperando o líder ou o sub-dono.
 *
 * A candidatura existe justamente porque "fechada" não pode virar "invisível": a guild aparece
 * na lista, o jogador bate na porta e o líder responde quando entrar no jogo. É a mensagem de
 * chat que não some.
 *
 * ### Por que os requisitos são nível e PR
 *
 * São os dois números que o jogador já vê na própria ficha e entende como "o quanto eu sou": o
 * nível diz quanto tempo de jogo, o PR (os pontos do PvP ranqueado) diz quanto de PvP.
 *
 * O PR, e **não o `elo`**. O `elo` de `players` é o número da arena aposentada: ele parou de se
 * mover, e hoje quase todo mundo carrega o 1.000 inicial — exigi-lo seria escrever na porta um
 * requisito que ninguém cumpre e ninguém entende, porque nem a ficha do treinador o mostra mais
 * (ver `fichaBlocoElo` em `app.js`). O PR muda toda partida e é o que aparece embaixo do emblema.
 *
 * Qualquer outro candidato (pokédex, pontos de boss) mede a mesma coisa de novo com uma régua
 * que o líder teria de explicar. Dois campos cabem numa linha da lista — e uma lista que ninguém
 * lê não recruta.
 *
 * ### Isto é CLIENTE e SERVIDOR
 *
 * O cliente valida para pintar o "você não alcança" ANTES do clique, e o servidor valida de
 * novo porque é ele que decide. Uma régua só, nos dois lados: se um dia divergirem, o jogador
 * lê "pode entrar" na lista e leva um "não" no clique, que é a pior forma de dizer não.
 */

/** As três, na ordem em que a tela as oferece: da mais aberta para a mais fechada. */
export const POLITICAS_GUILD = Object.freeze(['aberta', 'requisitos', 'fechada']);

/** A que uma guild que nunca escolheu nada responde. Ver `normalizarRecrutamento`. */
export const POLITICA_PADRAO = 'fechada';

/**
 * Quanto tempo quem SAI de uma guild espera antes de entrar em outra.
 *
 * O problema que isto resolve não é o jogador que troca de guild — é o que troca TODO DIA. Com
 * bônus de ranking por guild, sem espera nenhuma, a jogada ótima é pular para a guild que está
 * em primeiro toda manhã e voltar à noite; a guild vira uma assinatura diária e ninguém constrói
 * nada com ninguém. Vinte e quatro horas é curto o bastante para não prender quem se arrependeu
 * de verdade e longo o bastante para a troca custar um dia de bônus.
 *
 * Só pega quem sai POR VONTADE PRÓPRIA. Quem foi expulso entra na outra guild no mesmo minuto —
 * ver `expulsarMembro` em `guild-db.mjs` —, senão a espera viraria um castigo na mão do líder:
 * bastaria expulsar para deixar a pessoa um dia fora de qualquer guild. E quem perdeu a guild
 * porque o dono a apagou também não paga por uma decisão que não foi dele.
 */
export const GUILD_SAIDA_CD_MS = 24 * 60 * 60 * 1000;

/** O cartaz de recrutamento: uma linha, que é o que cabe na lista sem empurrar a próxima guild. */
export const MAX_DESCRICAO_GUILD = 80;

/**
 * Tetos dos requisitos. Não são o máximo do jogo — são o máximo que faz sentido EXIGIR.
 *
 * Existem para um dedo escorregado (ou um cliente forjado) não gravar "nível mínimo 2 bilhões",
 * que é uma guild que aparece na lista aceitando candidatura e recusa todo mundo em silêncio.
 */
export const REQ_LEVEL_MAX = 9999;
export const REQ_PR_MAX = 9999;

const inteiro = (v, max) => {
  const n = Math.floor(Number(v));
  if (!Number.isFinite(n) || n <= 0) return 0;
  return Math.min(n, max);
};

/**
 * O cartaz limpo: uma linha só, sem espaço dobrado e sem exceder o teto.
 *
 * As quebras de linha viram espaço em vez de serem recusadas — colar um texto de três linhas é
 * o que a pessoa vai fazer, e recusar isso com "inválido" seria cobrar dela um formato que a
 * tela nunca explicou.
 */
export function descricaoGuildLimpa(raw) {
  return String(raw ?? '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, MAX_DESCRICAO_GUILD);
}

/**
 * O que a guild grava, a partir do que o formulário mandou.
 *
 * Os requisitos são ZERADOS fora da política `requisitos` de propósito: guardá-los escondidos
 * faria a guild voltar de "aberta" para "com restrição" carregando um nível mínimo que o líder
 * escreveu há três meses e não lembra mais. O que não está na tela não decide nada.
 */
export function normalizarRecrutamento(raw) {
  const r = raw && typeof raw === 'object' ? raw : {};
  const politica = POLITICAS_GUILD.includes(r.politica) ? r.politica : POLITICA_PADRAO;
  const comReq = politica === 'requisitos';
  return {
    politica,
    reqLevel: comReq ? inteiro(r.reqLevel, REQ_LEVEL_MAX) : 0,
    reqPr: comReq ? inteiro(r.reqPr, REQ_PR_MAX) : 0,
    descricao: descricaoGuildLimpa(r.descricao),
  };
}

/**
 * O que falta a este jogador para entrar nesta guild, ou `null` se não falta nada.
 *
 * Devolve UM motivo, o primeiro que barra, e não a lista inteira: a frase que a tela precisa
 * dizer é "você precisa de nível 150", e não um relatório. Quem cumpre o nível e não o PR vê o
 * PR na tentativa seguinte.
 *
 * @param {{politica?: string, reqLevel?: number, reqPr?: number}} guild
 * @param {{level?: number, pr?: number}} jogador — `pr` são os pontos do PvP ranqueado
 * @returns {null | { campo: 'level'|'pr', minimo: number, seu: number }}
 */
export function faltaParaEntrar(guild, jogador) {
  if (guild?.politica !== 'requisitos') return null;
  const level = Math.floor(Number(jogador?.level) || 0);
  const pr = Math.floor(Number(jogador?.pr) || 0);
  const minLevel = Math.floor(Number(guild?.reqLevel) || 0);
  const minPr = Math.floor(Number(guild?.reqPr) || 0);
  if (minLevel > 0 && level < minLevel) return { campo: 'level', minimo: minLevel, seu: level };
  if (minPr > 0 && pr < minPr) return { campo: 'pr', minimo: minPr, seu: pr };
  return null;
}

/**
 * O que o botão da lista faz nesta guild, para este jogador.
 *
 * Um lugar só decide isto porque a lista e o clique precisam concordar: a lista pinta o botão a
 * partir daqui, o servidor recusa a partir daqui, e não há um terceiro lugar onde a regra possa
 * envelhecer sozinha.
 *
 * @returns {'entrar'|'candidatar'|'requisito'} — `requisito` é "aparece, mas você não alcança".
 */
export function acaoDeEntrada(guild, jogador) {
  if (guild?.politica === 'aberta') return 'entrar';
  if (guild?.politica === 'requisitos') {
    return faltaParaEntrar(guild, jogador) ? 'requisito' : 'entrar';
  }
  return 'candidatar';
}
