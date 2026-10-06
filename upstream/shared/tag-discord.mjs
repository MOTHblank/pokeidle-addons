// TAG DO DISCORD — quem usa a tag "IDLE" do nosso servidor no perfil ganha diamante.
//
// Mora em `shared/` pelo mesmo motivo de `convites.mjs`: o BOT (`src/bot/tag.mjs`) conta o tempo
// e decide quando um período fechou, o JOGO (`sim.mjs`) credita e anuncia, e os dois precisam da
// MESMA régua. Uma cópia do "10 dias" no bot e outra no jogo seria um prêmio prometido num lado e
// pago no outro com outro número.
//
// ### A regra, em cinco frases
//
// Pôs a tag do nosso servidor no perfil pela PRIMEIRA vez: 5 💎 na hora, de boas-vindas — uma vez
// por conta de Discord, para sempre (ver `DIAMANTES_BOAS_VINDAS`). Daí em diante, a cada 10 dias
// SEGUIDOS com a tag, a conta de Discord ganha mais 5 💎.
// Tirou a tag (ou trocou pela de outro servidor, ou saiu do servidor), o relógio ZERA e a
// sequência recomeça do zero quando ela voltar — e o jogador é avisado disso. Só conta conta de
// Discord com mais de 1 ano (a mesma régua dos convites). O diamante cai na conta do jogo
// VINCULADA àquele Discord, e o vínculo é 1 para 1, para sempre.
//
// ### De onde vem a informação
//
// O Discord manda, no objeto de usuário, o campo `primary_guild`:
//
//     { identity_guild_id, identity_enabled, tag, badge }
//
// `identity_enabled` é `true` quando a tag está À MOSTRA, `false` quando a PESSOA a tirou, e
// `null` quando o SISTEMA a limpou (o servidor perdeu o direito à tag, por exemplo por queda de
// boost). O terceiro caso não é culpa de ninguém e por isso não zera: ele PAUSA (ver
// `situacaoDaTag`).
//
// ### O relógio conta o que foi VISTO, e nada além
//
// O tempo de uma sequência é a soma dos intervalos entre duas observações seguidas COM a tag, e
// cada intervalo entra limitado a `LACUNA_MAX_MS`. É isso que fecha o "tira enquanto o bot está
// fora do ar": se o bot passou 3 horas sem olhar, essas 3 horas não contam para ninguém — tirar
// a tag nesse vão não rende nada, porque o vão já não rendia. O preço é perder alguns minutos de
// relógio num restart, e é o lado certo de errar.
//
// ### Tudo aqui é puro
//
// Nada de banco, nada de Discord: `observar` recebe a linha, o que foi visto e a hora, e devolve a
// linha nova, os prêmios que nasceram e o aviso a dar. O banco (`server/tag-discord-db.mjs`) só
// grava o resultado, e `tools/teste-tag-discord.mjs` exercita cada caminho de burla contra esta
// função sem precisar do Discord.
import { contaVelhaOBastante, criadoEmDoSnowflake, IDADE_MINIMA_MS } from './convites.mjs';

export { contaVelhaOBastante, criadoEmDoSnowflake, IDADE_MINIMA_MS };

const MINUTO_MS = 60_000;
const HORA_MS = 60 * MINUTO_MS;
const DIA_MS = 24 * HORA_MS;

/** Quanto tempo SEGUIDO com a tag fecha um período. */
export const PERIODO_DIAS = 10;
export const PERIODO_MS = PERIODO_DIAS * DIA_MS;

/** Quanto cada período paga. Diamante de BRINDE (`MOTIVO.TAG_DISCORD`): não enche a cota de venda. */
export const DIAMANTES_POR_PERIODO = 5;

/**
 * O BÔNUS DE BOAS-VINDAS: quem põe a tag pela PRIMEIRA vez ganha na hora, e só então o relógio dos
 * 10 dias começa (regra do dono, 30/09/2026). É UM por conta de Discord, para sempre: o prêmio tem a
 * chave fixa `(série 0, período 0)` — a mesma chave única do banco que impede pagar um período duas
 * vezes —, e nasce só quando a conta de Discord abre a PRIMEIRA série da vida. Tirar e pôr a tag, ou
 * sair e voltar ao servidor, abre séries novas, e série nova depois da primeira não paga boas-vindas.
 */
export const DIAMANTES_BOAS_VINDAS = 5;

/** A chave do prêmio de boas-vindas em `tag_discord_premios` — a série 0 nunca é uma série de verdade. */
export const SERIE_BOAS_VINDAS = 0;
export const PERIODO_BOAS_VINDAS = 0;

/**
 * O maior intervalo entre duas observações que ainda conta inteiro.
 *
 * O bot olha a lista de membros de 5 em 5 minutos (e na hora, quando o Discord avisa uma troca).
 * Meia hora cobre uma ou duas varreduras perdidas num restart sem virar um vão em que a tag
 * poderia ter saído e voltado sem ninguém ver.
 */
export const LACUNA_MAX_MS = 30 * MINUTO_MS;

/** De quanto em quanto tempo o bot relê a lista inteira de membros. */
export const VARREDURA_MS = 5 * MINUTO_MS;

/** Quanto vale o código de vínculo que o botão mostra. Curto: ele só serve para ser digitado já. */
export const VINCULO_VALIDADE_MS = 30 * MINUTO_MS;

/**
 * A trava da varredura contra a queda em massa: se MAIS da metade de quem estava com a tag
 * "tirasse" a tag na mesma varredura (e pelo menos este tanto de gente), é o Discord que
 * respondeu errado — lista pela metade, campo sumido —, não um mutirão. Nessa hora a varredura
 * PAUSA essa gente em vez de zerar o relógio de todo mundo.
 */
export const QUEDA_EM_MASSA_MIN = 5;
export const QUEDA_EM_MASSA_FRACAO = 0.5;

/**
 * O que um objeto de usuário do Discord diz sobre a NOSSA tag.
 *
 *   'com'          a tag do nosso servidor, à mostra
 *   'sem'          sem tag nenhuma, ou a nossa desligada pela própria pessoa
 *   'outra'        a tag de OUTRO servidor
 *   'pausa'        a nossa, limpa pelo sistema (`identity_enabled: null`) — não conta e não zera
 *   'desconhecida' o objeto não trouxe o campo — não dá para concluir nada
 *
 * `'desconhecida'` existe porque nem todo payload do Discord carrega `primary_guild`. Tratar a
 * AUSÊNCIA do campo como "sem tag" zeraria o relógio de quem só apareceu num evento incompleto.
 * A varredura decide diferente (ver `campoAusente`): lá, com o campo vindo nos outros membros,
 * a ausência num deles é "sem".
 */
export function situacaoDaTag(usuario, guildId, { campoAusente = 'desconhecida' } = {}) {
  if (!usuario || typeof usuario !== 'object') return 'desconhecida';
  if (!('primary_guild' in usuario)) return campoAusente;
  const pg = usuario.primary_guild;
  if (!pg || pg.identity_guild_id == null) return 'sem';
  if (String(pg.identity_guild_id) !== String(guildId)) return 'outra';
  if (pg.identity_enabled === true) return 'com';
  if (pg.identity_enabled === false) return 'sem';
  return 'pausa';
}

/** As situações que ZERAM uma sequência em andamento. */
export const SITUACOES_QUE_ZERAM = new Set(['sem', 'outra', 'saiu']);

/** Uma linha de quem nunca foi visto. Mesmo formato que `server/tag-discord-db.mjs` lê e grava. */
export const LINHA_VAZIA = Object.freeze({
  estado: 'sem',
  serie: 0,
  acumuladoMs: 0,
  periodos: 0,
  vistoEm: null,
  desde: null,
  resets: 0,
  removeuEm: null,
  motivoReset: null,
});

/**
 * Aplica UMA observação à linha de uma conta de Discord.
 *
 * `situacao` é o que `situacaoDaTag` devolveu — ou `'saiu'` (a pessoa deixou o servidor) — já
 * com a régua da idade aplicada por quem chama (`'nova'` = está com a tag, mas a conta de Discord
 * tem menos de 1 ano). Ver `situacaoComIdade`.
 *
 * Devolve `{ linha, premios, aviso }`:
 *
 *   premios  `[{ serie, periodo, boasVindas? }]` — o que nasceu AGORA: os períodos que fecharam e,
 *            na primeira série da vida, as boas-vindas (`serie 0, periodo 0`). `(serie, periodo)`
 *            é a chave do prêmio no banco, e é ela que impede pagar o mesmo prêmio duas vezes;
 *   aviso    `null` ou `{ tipo, ... }` — o que o jogador precisa ler:
 *              'iniciou'   primeira sequência da vida, o relógio começou;
 *              'removeu'   a tag saiu com uma sequência em andamento: zerou;
 *              'reiniciou' a tag voltou depois de ter saído: sequência nova, do zero;
 *              'nova'      está com a tag, mas a conta de Discord é nova demais.
 *
 * ### Por que a sequência é um número (`serie`)
 *
 * Cada vez que o relógio recomeça, a série sobe. O prêmio é único por (série, período), então o
 * período 1 da série 3 e o período 1 da série 4 são prêmios diferentes — e cada um só existe
 * depois de 10 dias INTEIROS com a tag naquela série. Tirar e pôr de novo não adianta nada: a
 * série nova começa do zero.
 */
export function observar(anterior, situacao, agora) {
  const l = { ...LINHA_VAZIA, ...(anterior ?? {}) };
  const premios = [];
  let aviso = null;

  if (situacao === 'desconhecida' || !Number.isFinite(agora)) return { linha: l, premios, aviso };

  if (situacao === 'com') {
    if (l.estado === 'com' && l.vistoEm != null) {
      // Duas observações seguidas COM a tag: o intervalo entra, limitado. Observação fora de
      // ordem (hora menor que a última) não conta nada e não volta o relógio.
      const lacuna = agora - l.vistoEm;
      if (lacuna > 0) l.acumuladoMs += Math.min(lacuna, LACUNA_MAX_MS);
    } else if (l.desde == null) {
      // Sequência NOVA. Quem já teve uma que zerou fica sabendo que esta recomeçou do zero.
      // A PRIMEIRA da vida desta conta de Discord (`serie` ainda 0) paga as boas-vindas na hora.
      const primeiraDaVida = l.serie === 0;
      l.serie += 1;
      l.desde = agora;
      l.acumuladoMs = 0;
      l.periodos = 0;
      aviso = l.removeuEm != null ? { tipo: 'reiniciou' } : { tipo: 'iniciou' };
      if (primeiraDaVida) premios.push({ serie: SERIE_BOAS_VINDAS, periodo: PERIODO_BOAS_VINDAS, boasVindas: true });
    }
    // Senão, a sequência estava PAUSADA (pelo sistema): ela retoma de onde parou, e o vão da
    // pausa não entra — `vistoEm` era nulo.
    l.estado = 'com';
    if (l.vistoEm == null || agora > l.vistoEm) l.vistoEm = agora;
    while (l.acumuladoMs >= (l.periodos + 1) * PERIODO_MS) {
      l.periodos += 1;
      premios.push({ serie: l.serie, periodo: l.periodos });
    }
    return { linha: l, premios, aviso };
  }

  if (SITUACOES_QUE_ZERAM.has(situacao)) {
    if (l.desde != null) {
      // Havia sequência: ZERA. O que ela já tinha pago continua pago; o que faltava se perde.
      aviso = { tipo: 'removeu', motivo: situacao, perdidoMs: l.acumuladoMs };
      l.resets += 1;
      l.removeuEm = agora;
      l.motivoReset = situacao;
      l.acumuladoMs = 0;
      l.periodos = 0;
      l.desde = null;
    }
    l.estado = situacao;
    l.vistoEm = null;
    return { linha: l, premios, aviso };
  }

  if (situacao === 'pausa') {
    l.estado = 'pausa';
    l.vistoEm = null;
    return { linha: l, premios, aviso };
  }

  if (situacao === 'nova') {
    if (l.estado !== 'nova') aviso = { tipo: 'nova' };
    l.estado = 'nova';
    l.vistoEm = null;
    return { linha: l, premios, aviso };
  }

  return { linha: l, premios, aviso };
}

/**
 * A régua da idade em cima da situação: com a tag e conta de Discord de menos de 1 ano vira
 * `'nova'` (a tag está lá, mas não conta). As outras situações passam direto — tirar a tag zera
 * igual, tenha a conta a idade que tiver.
 */
export function situacaoComIdade(situacao, discordId, agora = Date.now()) {
  if (situacao === 'com' && !contaVelhaOBastante(discordId, agora)) return 'nova';
  return situacao;
}

/** Quando uma conta de Discord passa a valer (1 ano de vida), em ms. `null` se o id for ruim. */
export function liberaEm(discordId) {
  const criada = criadoEmDoSnowflake(discordId);
  return criada == null ? null : criada + IDADE_MINIMA_MS;
}

/** Quanto falta para o próximo prêmio, a partir do acumulado. */
export const faltaParaProximo = (linha) =>
  Math.max(0, ((linha?.periodos ?? 0) + 1) * PERIODO_MS - (linha?.acumuladoMs ?? 0));

/** "3d 4h", "5h 12min", "8min" — para as mensagens do bot. */
export function duracaoCurta(ms) {
  const m = Math.max(0, Math.floor(ms / MINUTO_MS));
  const d = Math.floor(m / (24 * 60));
  const h = Math.floor((m % (24 * 60)) / 60);
  const min = m % 60;
  if (d) return h ? `${d}d ${h}h` : `${d}d`;
  if (h) return min ? `${h}h ${min}min` : `${h}h`;
  return `${min}min`;
}
