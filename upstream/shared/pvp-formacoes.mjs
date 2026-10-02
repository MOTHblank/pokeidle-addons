/**
 * O ARMÁRIO DE FORMAÇÕES do PvP — equipes salvas com nome, trocadas com um clique.
 *
 * ### O que é
 *
 * Até `PVP_FORMACOES_MAX` escalações (os mesmos até cinco pokémon da equipe de PvP, NA ORDEM), cada
 * uma com um nome e o placar dela: quantas partidas aquela escalação exata ganhou e perdeu. "Usar"
 * copia a formação para a equipe de PvP (`pvp_time`), que é a que o pareador lê — e, para quem não
 * escolheu equipe própria no Campeonato, a que ele congela.
 *
 * ### O placar é da ESCALAÇÃO, não do clique
 *
 * Uma partida conta para toda formação cuja lista de ids é IGUAL, na mesma ordem, à equipe que
 * entrou em campo (ver `contarNasFormacoes` em `server/pvp-formacoes-db.mjs`). Não existe "formação
 * ativa" guardada em lugar nenhum. Assim o número não depende de o jogador ter clicado em "Usar":
 * quem remonta a mesma escalação à mão também alimenta o placar dela, e quem troca um pokémon e vai
 * para a fila NÃO suja o placar de uma formação que não lutou.
 *
 * A ordem entra na comparação porque ela é a decisão de verdade do PvP: quem abre a luta é o 1º.
 * O mesmo time em outra ordem é outra formação.
 *
 * Mudar a escalação de uma formação ZERA o placar dela — o número antigo seria de outro time.
 * Renomear não zera.
 *
 * ### Privado
 *
 * Formação, nome e placar só viajam para o DONO (`pvp.info` e as respostas do armário). A ficha
 * pública do treinador e o Tracker continuam mostrando só a equipe de PvP atual.
 */

/** Quantas formações cada jogador guarda — "talvez 4 ou 5 times", no pedido. */
export const PVP_FORMACOES_MAX = 5;

/** O teto do nome, em code points. Cabe "Anti-Dragão (Campeonato)" com folga. */
export const FORMACAO_NOME_MAX = 30;

/** Invisíveis, de controle, combinantes e marcas de direção — os mesmos que a Name Tag barra. */
const RE_PROIBIDO = /[\u0000-\u001F\u007F-\u009F̀-ͯ​-‏‪-‮⁠-⁯﻿]/;

/** Pelo menos uma letra ou um número: um nome só de pontuação não identifica nada. */
const RE_LEGIVEL = /[\p{L}\p{N}]/u;

/** Um slot de 1 a `PVP_FORMACOES_MAX`, ou `null`. Só número inteiro: o pacote é JSON à mão. */
export function slotDeFormacao(valor) {
  return Number.isInteger(valor) && valor >= 1 && valor <= PVP_FORMACOES_MAX ? valor : null;
}

/**
 * Tira o espaço das bordas e colapsa o repetido. O corte ANTES do `replace` é o mesmo cuidado da
 * Name Tag: o socket aceita 16 KB por mensagem, e sem ele um nome gigante pagaria duas varreduras
 * inteiras antes de o teto ser consultado.
 */
export function normalizarNomeFormacao(valor) {
  if (typeof valor !== 'string') return '';
  const bruto = valor.length > FORMACAO_NOME_MAX * 4 ? valor.slice(0, FORMACAO_NOME_MAX * 4 + 1) : valor;
  // NFC junta "e" + acento combinante em "é": o combinante solto é barrado logo adiante, e sem isto
  // um "Dragão" colado de um lugar que manda a forma decomposta seria recusado sem motivo visível.
  return bruto.normalize('NFC').replace(/\s+/g, ' ').trim();
}

/**
 * `{ ok: true, nome }` ou `{ ok: false, erro }`, com `erro` como CHAVE de i18n — roda no servidor
 * (que não sabe o idioma) e no cliente, como o `validarApelido`.
 *
 * Mais solto que o apelido de propósito: o nome é privado (só o dono vê), então pontuação e emoji
 * simples passam. O que continua barrado é o que quebra a tela — invisível, controle, direção.
 */
export function validarNomeFormacao(valor) {
  if (typeof valor !== 'string') return { ok: false, erro: 'pvp.formErroNome' };
  const nome = normalizarNomeFormacao(valor);
  if (!nome) return { ok: false, erro: 'pvp.formErroNome' };
  if ([...nome].length > FORMACAO_NOME_MAX) return { ok: false, erro: 'pvp.formErroNomeLongo' };
  if (RE_PROIBIDO.test(nome) || !RE_LEGIVEL.test(nome)) return { ok: false, erro: 'pvp.formErroNome' };
  return { ok: true, nome };
}

/** As duas escalações são a MESMA — mesmos ids, mesma ordem. */
export function mesmaEscalacao(a, b) {
  if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
  return a.every((id, i) => Number(id) === Number(b[i]));
}

/** As partidas de uma formação, somando Ranqueado e Campeonato. */
export const partidasDaFormacao = (f) =>
  (Number(f?.v) || 0) + (Number(f?.d) || 0) + (Number(f?.cv) || 0) + (Number(f?.cd) || 0);

/** A taxa de vitória em % inteiro, ou `null` sem partida nenhuma (0% e "nunca jogou" não são iguais). */
export function taxaDaFormacao(f) {
  const total = partidasDaFormacao(f);
  if (!total) return null;
  return Math.round((((Number(f?.v) || 0) + (Number(f?.cv) || 0)) * 100) / total);
}
