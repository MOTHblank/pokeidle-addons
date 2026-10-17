# Moth Controller

O Moth Controller é o aplicativo nativo para desktop do [PokéIdle Addons](../README.md). Ele gerencia até quatro perfis do jogo com autenticações independentes e conexões nativas com a Twitch.

**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)**

## Como usar

Configure os perfis em **Accounts (Contas)** e faça login no PokéIdle e na Twitch quando solicitado. Os cartões das contas mostram o estado das conexões e os controles das sessões. O Firefox ou o Firefox Developer Edition é utilizado para autenticação quando necessário. Use **Open in Firefox (Abrir no Firefox)** para jogar pelo navegador.

Após a autenticação, as conexões com o jogo e com a Twitch funcionam em segundo plano, sem a necessidade de manter uma janela do navegador aberta. Não é necessário instalar gerenciadores de userscripts ou extensões de automação.

O **Dashboard (Painel)** mostra a atividade e os detalhes das contas; o **Hunt Atlas** compara caçadas e resultados de capturas; e **Automation (Automações)** reúne as configurações de captura automática, reposição de itens e regras do mercado. As opções específicas de cada conta são configuradas no aplicativo.

## Imagens do jogo

Os ícones de Poké Bolas, poções, revives e itens compatíveis são **incluídos diretamente no executável** e carregados localmente. Sprites de Pokémon e imagens de itens não incluídos continuam usando cache sob demanda. **Não há etapa nem botão para baixar manualmente os ícones.** Se uma imagem estiver indisponível, o aplicativo continuará funcionando.

Consulte a [documentação dos recursos visuais nativos](assets/visual/README.md) para detalhes técnicos sobre o cache e as ferramentas de desenvolvimento.

## Compilação

Dentro do diretório `controller/`, execute:

```sh
cargo build --release
```

No Windows, se uma versão anterior do executável ainda estiver em execução, o script opcional `build-windows.ps1` compila o programa em outro diretório, sem sobrescrever o arquivo em uso.

Durante o desenvolvimento, utilize `cargo check` e `cargo test` a partir de `controller/`. Não inclua credenciais nem dados de autenticação nos commits.
