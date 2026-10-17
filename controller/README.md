# Moth Controller — Desenvolvimento

Aplicativo desktop do [PokéIdle Addons](../README.md), desenvolvido em Rust com egui/eframe. Gerencia até quatro perfis independentes do PokéIdle e conexões com a Twitch.

**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)**

## Uso

Em **Contas**, configure seus perfis e faça login no PokéIdle e na Twitch quando solicitado. Os cartões mostram o estado das conexões e os controles de cada sessão.

- **Painel:** visão geral da atividade das contas e detalhes das sessões.
- **Hunt Atlas:** análise e comparação de caçadas.
- **Automações:** captura automática, reposição de itens e regras do mercado.
- **Abrir no Firefox:** abre o jogo no navegador para interação manual.

## Kick — monitoramento nativo

Em **Contas**, expanda **Kick · monitoramento nativo** no cartão de uma conta.
Informe os nomes dos canais separados por vírgula e um **token de acesso da API
Kick Developer** obtido por autorização legítima no serviço. Use **Salvar canais**
para manter apenas a lista de nomes e **Monitorar Kick** para iniciar o worker
nativo. Cada conta tem seu próprio monitor; **Parar Kick** o encerra.

O worker usa somente o endpoint oficial de leitura
`GET https://api.kick.com/public/v1/channels?slug=...` (com Bearer token),
verifica se cada canal está ao vivo a cada 90 segundos e apresenta contagens de
espectadores quando a API as oferece. Requisições têm timeout; erros temporários
geram retries espaçados, e falhas de autenticação interrompem a consulta. Não
é necessário Streamlink, VLC, MPV ou Python.

**Segurança:** tokens são usados apenas em memória, não são escritos no
`kick-channels.json`, no cache de login do jogo, em logs ou URLs. Após reiniciar
o aplicativo, é necessário inserir o token novamente. O monitor não usa
cookies da Twitch, não conecta o player de vídeo, não envia mensagens de chat,
não relata eventos de visualização e **não confirma ou acumula pontos**.

A API Kick Developer exige um token emitido para um aplicativo registrado.
Este recurso é independente da autenticação em Twitch/PokéIdle; não aceite
cookies de sessão nem tokens extraídos de navegadores como substitutos do
token da API oficial.

## Compilação

No diretório `controller/`, execute:

```sh
cargo build --release
```

O executável compilado fica em `target/release/`.

No Windows, o script `build-windows.ps1` também permite compilar para um diretório separado enquanto uma versão anterior do programa está em execução:

```powershell
.\build-windows.ps1
```

## Desenvolvimento e recursos

Para executar as verificações locais:

```sh
cargo check
cargo test
```

Os ícones de itens utilizados pela interface são incorporados ao executável. Os sprites de Pokémon são carregados sob demanda. Consulte a [documentação dos recursos visuais](assets/visual/README.md) para detalhes de implementação e manutenção.

Mantenha credenciais e dados de autenticação fora dos commits e registros de diagnóstico.
