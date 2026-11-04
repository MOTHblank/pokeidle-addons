# Moth Controller — PokéIdle

Desenvolvido para ser o **gerenciador de múltiplas contas mais eficiente do PokéIdle**, o Moth Controller reúne até **quatro contas simultâneas** em um único aplicativo, com sessões nativas leves e foco em baixo consumo de **CPU e memória RAM**.

Acompanhe suas caçadas, configure automações e mantenha as sessões funcionando em segundo plano.

**Menos de 2% de CPU e no máximo 50 MB de RAM — ou seu dinheiro de volta!**


**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)** · [Relatar um problema](https://github.com/MOTHblank/pokeidle-addons/issues)

## Funcionalidades

- **Painel:** acompanhe suas contas, caçadas ativas, Pokémon, adversários e itens em uma visão geral.
- **Hunt Atlas:** compare caçadas, estimativas de experiência, chances de captura e níveis de Outland.
- **Automações:** configure captura automática, reposição de Poké Bolas, poções e revives, além de regras de compra no mercado.
- **Contas:** organize até quatro perfis e acompanhe suas sessões do PokéIdle.
- **Twitch:** acompanhe os streamers oficiais, o saldo confirmado de pontos por canal, favoritos por conta e até duas transmissões por conta. O login é opcional, fica separado do PokéIdle e pode aproveitar a sessão Twitch já existente no perfil Firefox da conta. Não abre chats nem mantém IRC. Os bônus do jogo (XP, Capture Boost e Secret Lure) continuam dependendo de resgates específicos de pontos do canal.
- **Kick:** sessões nativas independentes por conta, com tráfego HLS de baixo bitrate, presença via WebSocket da Kick e leitura do saldo real de pontos (ganho mostrado apenas quando confirmado por aumento de saldo).

## Como começar

1. [**Baixe o Moth Controller**](https://github.com/MOTHblank/pokeidle-addons/releases/latest) e abra o aplicativo.
2. Acesse **Contas** e configure os perfis que deseja utilizar.
3. Entre no **PokéIdle** quando solicitado. Para acompanhar pontos da Twitch, use o login opcional de **Twitch** em **Contas**; ele nunca é necessário para iniciar o PokéIdle.
4. Use o **Painel** para acompanhar suas contas, **Automações** para configurar as automações e **Hunt Atlas** para encontrar as melhores caçadas para seus objetivos.

**Navegador recomendado:** [Firefox Developer Edition](https://www.mozilla.org/pt-BR/firefox/developer/).

Para jogar pelo navegador, selecione **Abrir no Firefox** na conta desejada.

## Desenvolvimento

Para compilar o aplicativo a partir do código-fonte, consulte as [instruções do controller](controller/README.md).

## Releases automáticas (GitHub Actions)

Os workflows usam os runners gratuitos do GitHub para repositórios públicos:

- Alterações comuns de código não disparam testes/builds de release. O workflow de release roda quando a versão do Controller muda, quando há um pedido explícito de release ou por execução manual.
- Para baixar um build de teste sem publicar uma versão, abra [Actions → Build e release do Moth Controller](https://github.com/MOTHblank/pokeidle-addons/actions/workflows/build-controller.yml), clique em **Run workflow** e mantenha **Publicar a release** desmarcado. Os binários ficam nos artefatos da execução por 7 dias.
- Para publicar uma **nova** versão, atualize `version` em `controller/Cargo.toml` (e sincronize `controller/Cargo.lock`) e faça o commit em `master`. O workflow identifica a mudança de versão, compila Windows x86_64 e Linux x86_64, gera atestados de proveniência e SHA-256, e cria a [GitHub Release](https://github.com/MOTHblank/pokeidle-addons/releases) com os dois pacotes. Também aceita uma tag `vX.Y.Z` correspondente ao `Cargo.toml`.
- Também é possível iniciar o workflow manualmente em `master` e marcar **Publicar a release**; releases existentes não são sobrescritas.

Os pacotes publicados são **Setup Windows**, **Portable Windows** e um `.zip` para Linux, acompanhados de SHA-256. A publicação só ocorre quando os builds Windows e Linux são concluídos com sucesso, incluindo as **atestações de proveniência geradas pelo GitHub Actions**. Essas atestações não são assinaturas Authenticode do Windows.

### Como verificar a proveniência

Após baixar os arquivos de uma release, instale o [GitHub CLI](https://cli.github.com/) e execute (substitua a versão pelo nome da release):

```sh
gh attestation verify Moth-Controller-Setup-v0.2.3.exe -R MOTHblank/pokeidle-addons
gh attestation verify Moth-Controller-Portable-v0.2.3-windows-x86_64.exe -R MOTHblank/pokeidle-addons
gh attestation verify moth-controller-v0.2.3-linux-x86_64.zip -R MOTHblank/pokeidle-addons
```

A verificação confere a integridade do arquivo e a identidade do workflow/repositório que gerou a compilação. Consulte também os arquivos `.sha256` anexos para uma conferência simples de hash.

