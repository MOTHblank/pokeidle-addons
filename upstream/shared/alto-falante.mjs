// O ALTO-FALANTE do Mercado: o vendedor escolhe UM dos seus anúncios e ele vai para os chats
// Mundo e Comércio em destaque, com um link que leva direto ao anúncio.
//
// As duas regras moram aqui porque as duas pontas precisam delas: o servidor cobra e trava, e a
// tela mostra o preço no diálogo e o relógio no botão antes do clique.
//
//   · CUSTA 10.000.000 Coins por anúncio (era 1.000.000 na estreia, v1.164). É um sink de
//     propósito — o Mundo é de todo mundo, e o preço é o que impede de ele virar vitrine.
//   · A espera de 30 minutos é da CONTA, e não do anúncio: quem tem trinta anúncios abertos
//     escolhe um deles, e só depois da espera anuncia outro (ou o mesmo).

/** Quanto custa mandar um anúncio para o chat. */
export const CUSTO_ALTO_FALANTE = 10_000_000;

/** A espera entre dois anúncios no chat, por conta. */
export const ESPERA_ALTO_FALANTE_MS = 30 * 60_000;
