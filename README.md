# Moth Controller — PokéIdle

Desenvolvido para ser o **gerenciador de múltiplas contas mais eficiente do PokéIdle**, o Moth Controller reúne até **quatro contas simultâneas** em um único aplicativo. Cada conta pode acompanhar **mais de dez canais da Twitch** por conexões leves de chat, com foco em um consumo extremamente baixo de **CPU e memória RAM**.

Acompanhe suas caçadas, configure automações e mantenha as sessões funcionando em segundo plano.

**Menos de 2% de CPU e no máximo 50 MB de RAM — ou seu dinheiro de volta!**


**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)** · [Relatar um problema](https://github.com/MOTHblank/pokeidle-addons/issues)

## Funcionalidades

- **Painel:** acompanhe suas contas, caçadas ativas, Pokémon, adversários e itens em uma visão geral.
- **Hunt Atlas:** compare caçadas, estimativas de experiência, chances de captura e níveis de Outland.
- **Automações:** configure captura automática, reposição de Poké Bolas, poções e revives, além de regras de compra no mercado.
- **Contas:** organize até quatro perfis e acompanhe suas conexões com o PokéIdle e a Twitch.
- **Kick:** sessões nativas independentes por conta, com tráfego HLS de baixo bitrate, presença via WebSocket da Kick e leitura do saldo real de pontos (ganho mostrado apenas quando confirmado por aumento de saldo).

## Como começar

1. [**Baixe o Moth Controller**](https://github.com/MOTHblank/pokeidle-addons/releases/latest) e abra o aplicativo.
2. Acesse **Contas** e configure os perfis que deseja utilizar.
3. Entre no **PokéIdle** e na **Twitch** quando solicitado.
4. Use o **Painel** para acompanhar suas contas, **Automações** para configurar as automações e **Hunt Atlas** para encontrar as melhores caçadas para seus objetivos.

**Navegador recomendado:** [Firefox Developer Edition](https://www.mozilla.org/pt-BR/firefox/developer/).

Para jogar pelo navegador, selecione **Abrir no Firefox** na conta desejada.

## Desenvolvimento

Para compilar o aplicativo a partir do código-fonte, consulte as [instruções do controller](controller/README.md).

## Releases automáticas (GitHub Actions)

Os workflows usam os runners gratuitos do GitHub para repositórios públicos:

- Alterações em `controller/` ou nos workflows acionam a validação Rust em Windows e Linux.
- Para baixar um build de teste sem publicar uma versão, abra [Actions → Build e release do Moth Controller](https://github.com/MOTHblank/pokeidle-addons/actions/workflows/build-controller.yml), clique em **Run workflow** e mantenha **Publicar a release** desmarcado. Os binários ficam nos artefatos da execução por 7 dias.
- Para publicar uma **nova** versão, atualize `version` em `controller/Cargo.toml`, faça o commit em `master` e crie/envie a tag correspondente, por exemplo `v0.1.10`. O workflow compila Windows x86_64 e Linux x86_64, verifica os SHA-256 e cria a [GitHub Release](https://github.com/MOTHblank/pokeidle-addons/releases) com os dois pacotes.
- Também é possível iniciar o workflow manualmente em `master` e marcar **Publicar a release**; releases existentes não são sobrescritas.

Os pacotes publicados são um executável `.exe` para Windows e um `.zip` para Linux (com verificação de dependências e licença MIT). A publicação só ocorre quando os dois builds são concluídos com sucesso.

