# Moth Controller — Desenvolvimento

Aplicativo desktop do [PokéIdle Addons](../README.md), desenvolvido em Rust com egui/eframe. Gerencia até quatro perfis independentes do PokéIdle e conexões com a Twitch.

**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)**

## Uso

Em **Contas**, configure seus perfis e faça login no PokéIdle e na Twitch quando solicitado. Os cartões mostram o estado das conexões e os controles de cada sessão.

- **Painel:** visão geral da atividade das contas e detalhes das sessões.
- **Hunt Atlas:** análise e comparação de caçadas.
- **Automações:** captura automática, reposição de itens e regras do mercado.
- **Abrir no Firefox:** abre o jogo no navegador para interação manual.

## Kick — sessões leves, transmissão HLS e pontos

Em **Contas → Kick · transmissões nativas**, configure os canais
(separados por vírgula), informe **o Bearer token da sua própria conta Kick**
(não o token da API pública de desenvolvedores) e clique em
**Iniciar transmissões**. O app continua independente da Twitch e do PokéIdle.

A implementação nativa usa os endpoints da interface web da Kick, estudados no
projeto Baillora/Kick_Channel_Points_Miner:

- `GET https://kick.com/api/v2/channels/{slug}` identifica se o canal está
  ao vivo, o canal, o usuário, o ID da live e o playback_url.
- `GET https://websockets.kick.com/viewer/v1/token`, com headers
  `X-Chatroom` e `X-User-Id`, solicita um token temporário de conexão.
- `wss://websockets.kick.com/viewer/v1/connect` mantém a sessão via
  handshake do canal, pings e eventos de watch enquanto há bytes HLS chegando.
  Não há envio de mensagens de chat.
- Uma thread HLS solicita o playlist M3U8, escolhe o menor bitrate anunciado,
  recebe segmentos e descarta os bytes, sem decodificação, cache em disco ou
  player externo. A interface mostra os segmentos e bytes transferidos.
- `GET https://kick.com/api/v2/channels/{slug}/points` consulta
  periodicamente o saldo associado à conta. Apenas **aumentos numéricos
  consecutivos confirmados pelo endpoint** aparecem como `ganho observado`.
  Resposta ausente, rejeitada ou não numérica é *sem confirmação*, nunca
  uma contagem zerada inventada.

**Importante:** a conexão WS e os segmentos HLS comprovam tráfego de rede,
não que pontos serão atribuídos. A integração depende de endpoints privados
e não documentados que podem mudar sem aviso. Se o saldo não aumentar, o
painel deve refletir isso em vez de declarar sucesso. O sistema não contorna
CAPTCHAs nem desafios da Kick e não garante elegibilidade para recompensas.

**Credenciais:** o token do usuário fica só em memória e é apagado do campo
ao iniciar. São salvos apenas os nomes dos canais no `kick-channels.json`,
nunca o token ou as URLs HLS temporárias. Não inclua credenciais em issues,
diagnósticos ou repositórios. Ao reiniciar o app, informe o token de novo.

**Uso de recursos:** o menor bitrate disponível é selecionado, e o vídeo
não é decodificado. Contudo os bytes dos segmentos completos são transferidos,
então o uso de banda depende do canal e **não é comparável à Twitch IRC**.
Pode haver 12 canais por conta; configure somente o necessário.

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
