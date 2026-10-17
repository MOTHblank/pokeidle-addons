# Moth Controller — PokéIdle

Aplicativo leve para gerenciar até **quatro contas do PokéIdle**. O Moth Controller mantém as conexões com o jogo e com a Twitch em segundo plano, de forma nativa. O Firefox é utilizado quando é necessário fazer login e, opcionalmente, para jogar pela interface do navegador.

**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)** · [Relatar um problema](https://github.com/MOTHblank/pokeidle-addons/issues)

## Funcionalidades

- **Dashboard (Painel):** visão geral das contas, caçadas ativas, informações sobre seus Pokémon e adversários, itens e detalhes das sessões.
- **Hunt Atlas:** comparação de caçadas, estimativas de experiência, planejamento de capturas e níveis de Outland.
- **Automation (Automações):** configuração de captura automática, reposição de itens e regras de compra no mercado.
- **Accounts (Contas):** gerenciamento de quatro perfis independentes e do estado das conexões com o PokéIdle e a Twitch.
- **Imagens do jogo:** ícones de Poké Bolas, poções, revives e itens compatíveis já vêm no executável. Sprites de Pokémon continuam sob demanda, com cache local.

## Como começar

1. **Baixe e abra** o Moth Controller.
2. Em **Accounts (Contas)**, configure os perfis que deseja utilizar.
3. Faça login no **PokéIdle** e na **Twitch** quando o aplicativo solicitar.
4. Após a conexão, acesse **Dashboard** para acompanhar as sessões, **Automation** para ajustar as automações e **Hunt Atlas** para comparar caçadas.

O aplicativo nativo **não exige Violentmonkey, userscripts nem que o Firefox permaneça aberto** durante o funcionamento normal em segundo plano. A opção **Open in Firefox (Abrir no Firefox)** continua disponível para jogar pelo navegador.

## Compilar a partir do código-fonte

No diretório `controller/`, execute:

```sh
cargo build --release
```

Para informações técnicas, consulte as [instruções do controller](controller/README.md) e a [documentação dos recursos visuais](controller/assets/visual/README.md).
