/**
 * A NAME TAG e o apelido cosmético que ela grava num pokémon.
 *
 * ### O que ela é, e o que ela deliberadamente NÃO é
 *
 * O apelido é DESENHO, e nada além disso. Ele não toca `speciesId`, `looktype`, tipo, stats,
 * evolução, Pokédex, spawn nem nada que o combate leia — a espécie continua sendo a espécie, e
 * a ficha continua mostrando "#006 Charizard" na linha de espécie mesmo quando o nome grande
 * em cima diz "Brasa". Quem quiser saber o que o bicho é tem a resposta na mesma tela.
 *
 * É por isso que ele mora numa coluna própria (`player_pokemon.apelido`) em vez de sobrescrever
 * o nome: `nome` é DERIVADO da espécie a cada `montarPokemon`, não é armazenado. Um apelido
 * gravado por cima duraria até o próximo login e sumiria sozinho.
 *
 * ### E por que ele atravessa a venda
 *
 * A coluna é da LINHA do pokémon, e o Mercado é `UPDATE ... SET player_id` — o mesmo caminho
 * da Medalha de Guerra e da data de captura. Quem compra leva o apelido junto, de propósito:
 * o comprador está pagando por aquele indivíduo, e um nome que evaporasse na troca de dono
 * tornaria a Name Tag inútil justamente para quem cria bicho para vender.
 *
 * ### A regra do texto
 *
 * Letras latinas (com acento, que é o alfabeto dos três idiomas do jogo), números, espaço e
 * três sinais de pontuação. Começa e termina em letra ou número, sem espaço duplo.
 *
 * O que fica de fora não é frescura de formato:
 *
 *   · emoji e símbolo — a fileira de selos do card (shiny, TM, medalha, +N) já é desenho, e
 *     um nome que também desenha faz o card inteiro virar ruído;
 *   · caractere invisível, combinante e marca de direção — é com eles que se monta o nome que
 *     empurra o resto da linha para fora da tela, ou que não dá para digitar num report;
 *   · vazio e só-pontuação — um pokémon sem nome legível na vitrine é um anúncio que não se
 *     consegue citar.
 *
 * O teto de 16 é o mesmo do nick do treinador, e pelo mesmo motivo: é o que `.cm-nome` do card
 * do Mercado mostra antes de cortar com reticências.
 *
 * ### Moderação
 *
 * Nada aqui filtra ofensa — não existe lista de palavras que resolva isso em três idiomas, e
 * fingir que existe daria uma garantia falsa. O que existe é o aviso na hora de confirmar (o
 * jogador lê antes de gastar), a linha de auditoria que o servidor grava em toda troca (quem
 * pôs, em quem, o quê) e a regra dos Termos. A punição é humana e vem depois.
 */

/** Item Name Tag — a etiqueta de uso único que grava o apelido. Ver `itens-nossos.mjs`. */
export const NAME_TAG_ID = 70016;

/** Quanto ela custa na Loja de diamantes. */
export const NAME_TAG_PRECO = 15;

export const APELIDO_MIN = 2;
export const APELIDO_MAX = 16;

/** O alfabeto aceito: latino com acento (pt/en/es) mais dígitos. */
const LETRA = 'A-Za-z0-9À-ÖØ-öø-ÿ';

/** Começa e termina em letra ou número; no meio, espaço e os três sinais. */
const RE_APELIDO = new RegExp(`^[${LETRA}](?:[${LETRA} '.-]*[${LETRA}])?$`);

/** Invisíveis, combinantes e marcas de direção — ver o cabeçalho. */
const RE_PROIBIDO = /[\u0000-\u001F\u007F-\u009F̀-ͯ​-‏‪-‮⁠-⁯﻿]/;

/**
 * Tira o que sobra das bordas e colapsa espaço repetido.
 *
 * Roda ANTES de validar, nos dois lados: sem isto, um apelido colado com espaço no fim seria
 * recusado por um motivo que o jogador não consegue ver na tela.
 */
export function normalizarApelido(valor) {
  if (typeof valor !== 'string') return '';
  // O CORTE antes do `replace`. O socket aceita frame de 16 KB (`maxPayload`, no gateway), e
  // sem este corte um apelido de 16 KB pagava duas varreduras da string inteira — uma no
  // `\s+` e outra no `trim` — antes de o teto de 16 caracteres sequer ser consultado. Medido:
  // 20 mil recusas de 16 KB custavam 1,7 s; com o corte, 20 ms.
  //
  // O fator 4 é folga deliberada: espaço repetido colapsa, então um nome legítimo pode chegar
  // mais longo do que sai. Quatro vezes o teto cobre qualquer digitação humana e ainda deixa
  // a carga grande cair antes de custar alguma coisa.
  const bruto = valor.length > APELIDO_MAX * 4 ? valor.slice(0, APELIDO_MAX * 4 + 1) : valor;
  return bruto.replace(/\s+/g, ' ').trim();
}

/**
 * `{ ok: true, apelido }` ou `{ ok: false, erro }`, onde `erro` é CHAVE de i18n.
 *
 * Chave, e não frase: esta função roda no servidor (que não sabe o idioma da aba) e no cliente
 * (que sabe). A mesma resposta serve aos dois — a tela traduz, o servidor devolve a chave no
 * `aviso` e o cliente traduz na chegada, como o resto do jogo já faz.
 */
export function validarApelido(valor) {
  // TEXTO, e nada além de texto. O pacote é JSON escrito à mão do outro lado, então `apelido`
  // chega como o que o remetente quiser — e `String(v)` transformava `true` no nome "true",
  // `NaN` em "NaN" e um objeto com `toString` no que ele mandasse. Nenhum desses nomes é
  // perigoso, mas aceitar coerção é aceitar que o campo tem um contrato que ele não tem: o
  // dia em que o valor cru for usado para outra coisa, a surpresa já está gravada no banco.
  if (typeof valor !== 'string') return { ok: false, erro: 'nametag.erroVazio' };
  const apelido = normalizarApelido(valor);
  if (!apelido) return { ok: false, erro: 'nametag.erroVazio' };
  // O comprimento é medido em code points, e não em `.length`: "é" pode chegar como um
  // caractere ou como dois (e + acento combinante), e o segundo caso cabe no limite mas
  // ocuparia o dobro na tela. O combinante já é barrado logo abaixo; contar por code point é
  // o que impede um limite de 16 que na prática é 32.
  const tamanho = [...apelido].length;
  if (tamanho < APELIDO_MIN || tamanho > APELIDO_MAX) {
    return { ok: false, erro: 'nametag.erroTamanho' };
  }
  if (RE_PROIBIDO.test(apelido)) return { ok: false, erro: 'nametag.erroCaractere' };
  if (!RE_APELIDO.test(apelido)) return { ok: false, erro: 'nametag.erroCaractere' };
  return { ok: true, apelido };
}

/**
 * O nome que as TELAS mostram: o apelido, quando existe, senão o da espécie.
 *
 * Uma função só, usada pela ficha, pelo card do Mercado, pela bolha do chat e pelas folhas de
 * escolha. Repetir `pk.apelido ?? pk.nome` em cada lugar é o desenho em que um deles fica para
 * trás — e o lugar esquecido é sempre o que denuncia que o apelido não é "de verdade".
 */
export function nomeExibidoPokemon(pk) {
  const apelido = normalizarApelido(pk?.apelido);
  return apelido || String(pk?.nome ?? '');
}
