/**
 * Domínios aceitos em cadastro, login e OAuth — bloqueia caixas temporárias e provedores aleatórios.
 *
 * São DUAS listas, e o que as separa é o tempo: `DOMINIOS_EMAIL_CADASTRO` governa endereço
 * NOVO (criar conta, o OAuth que vira conta, trocar o e-mail da conta) e `DOMINIOS_EMAIL_LOGIN`
 * governa quem JÁ tem conta e só quer entrar. Um domínio que sai do cadastro precisa continuar
 * no login, senão a mudança não fecha a porta para os próximos — ela tranca do lado de fora
 * quem já estava dentro, e essa pessoa não tem o que fazer a respeito.
 *
 * Foi exatamente o caso do `icloud.com` (23/09/2026). O **Ocultar Meu E-mail** da Apple gera
 * endereços `@icloud.com` aleatórios que caem todos na mesma caixa e não têm padrão nenhum a
 * cortar — duas dessas são indistinguíveis de duas pessoas, e nenhuma canonização resolve (o
 * assunto está por extenso em `email-canonico.mjs`). Ou seja: uma conta Apple valia contas
 * infinitas aqui dentro. Daqui em diante ninguém cadastra com `@icloud.com`, e as contas que
 * já existiam entram como sempre — senha, Google e Discord.
 */
export const DOMINIOS_EMAIL_CADASTRO = Object.freeze([
  'gmail.com',
  'hotmail.com',
  'outlook.com',
  'live.com',
  'yahoo.com',
  'yahoo.com.br',
]);

/**
 * Domínios que valem só para ENTRAR — fechados para conta nova, abertos para quem já tem.
 *
 * Quem entra aqui nunca mais sai: apagar uma linha desta lista é despejar jogador que se
 * cadastrou quando o domínio era aceito. Se um dia for mesmo preciso, o caminho é o mesmo de
 * antes — `admin.trocarEmailUsuario`, um a um, com o jogador avisado.
 */
export const DOMINIOS_EMAIL_SO_LOGIN = Object.freeze([
  'icloud.com',
]);

/** Tudo que o login aceita: os do cadastro mais o legado. */
export const DOMINIOS_EMAIL_LOGIN = Object.freeze([
  ...DOMINIOS_EMAIL_CADASTRO,
  ...DOMINIOS_EMAIL_SO_LOGIN,
]);

const FORMATO = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;

/** O domínio de um endereço bem formado, ou `null` se nem e-mail é. */
function dominioDe(email) {
  const norm = String(email ?? '').trim().toLowerCase();
  if (!FORMATO.test(norm)) return null;
  return norm.split('@')[1];
}

/** Endereço NOVO: criar conta, OAuth que cria conta, trocar o e-mail da conta. */
export function emailPermitidoCadastro(email) {
  const dominio = dominioDe(email);
  return dominio !== null && DOMINIOS_EMAIL_CADASTRO.includes(dominio);
}

/** Endereço que JÁ é de uma conta: login com senha e OAuth de quem volta. Aceita o legado. */
export function emailPermitidoLogin(email) {
  const dominio = dominioDe(email);
  return dominio !== null && DOMINIOS_EMAIL_LOGIN.includes(dominio);
}
