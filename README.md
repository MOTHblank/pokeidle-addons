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
- **Twitch:** acompanhe os streamers oficiais, favoritos e saldo de pontos por canal (até duas transmissões simultâneas por conta), com login opcional e separado do PokéIdle.
- **Kick:** acompanhe canais e o saldo real de pontos por conta de forma independente.

## Como começar

1. [**Baixe o Moth Controller**](https://github.com/MOTHblank/pokeidle-addons/releases/latest) e abra o aplicativo.
2. Acesse **Contas**, configure seus perfis e faça login no **PokéIdle** quando solicitado.
3. Opcionalmente, conecte suas contas da **Twitch** ou **Kick** em **Contas** para acompanhar pontos e transmissões.
4. Use o **Painel** para acompanhar suas contas, **Automações** para configurar captura/reposição/mercado e **Hunt Atlas** para comparar caçadas.

**Navegador recomendado:** [Firefox Developer Edition](https://www.mozilla.org/pt-BR/firefox/developer/).

Para jogar pelo navegador, selecione **Abrir no Firefox** na conta desejada.

## Desenvolvimento

Para compilar o aplicativo a partir do código-fonte, consulte as [instruções do controller](controller/README.md).

## Releases e verificação

As versões para **Windows (x86_64)** são publicadas em [GitHub Releases](https://github.com/MOTHblank/pokeidle-addons/releases) nos formatos **Setup Windows** e **Portable Windows**, acompanhadas de arquivos `.sha256` e atestações de proveniência do GitHub Actions (não são assinaturas Authenticode).

Para verificar a proveniência com o [GitHub CLI](https://cli.github.com/) (substitua `<versão>` pela tag da release, ex.: `v0.2.10`):

```sh
gh attestation verify Moth-Controller-Setup-<versão>.exe -R MOTHblank/pokeidle-addons
gh attestation verify Moth-Controller-Portable-<versão>-windows-x86_64.exe -R MOTHblank/pokeidle-addons
```

Para compilar ou gerar builds de teste sem publicar uma release, consulte [Actions](https://github.com/MOTHblank/pokeidle-addons/actions/workflows/build-controller.yml) ou `controller/README.md`.

