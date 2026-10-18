# Moth Controller — Desenvolvimento

Aplicativo desktop do [PokéIdle Addons](../README.md), desenvolvido em Rust com egui/eframe. Gerencia até quatro perfis independentes do PokéIdle e conexões com a Twitch.

**[Baixar a versão mais recente](https://github.com/MOTHblank/pokeidle-addons/releases/latest)**

## Uso

Em **Contas**, configure seus perfis e faça login no PokéIdle e na Twitch quando solicitado. Os cartões mostram o estado das conexões e os controles de cada sessão.

- **Painel:** visão geral da atividade das contas e detalhes das sessões.
- **Hunt Atlas:** análise e comparação de caçadas.
- **Automações:** captura automática, reposição de itens e regras do mercado.
- **Abrir no Firefox:** abre o jogo no navegador para interação manual.

## Kick — tela dedicada em Contas

Acesse **Contas → Kick**. Os quatro perfis aparecem em um seletor compacto:
escolher outra conta não interrompe as demais. Os cartões principais do
gerenciador PokéIdle contêm somente um atalho **Gerenciar Kick →**, sem
duplicar toda a configuração.

1. **Login:** abra a Kick pelo link da interface e autentique-se normalmente
   no navegador. Para o worker nativo, é necessário fornecer manualmente o
   Bearer token da sua própria conta Kick (não um token Developer API).
   A interface não implementa OAuth nem captura automática de sessão.
   Escolha **Salvar login com proteção do sistema** para guardá-lo apenas no
   cofre protegido pelo sistema operacional; o campo sempre fica mascarado.
2. **Canais oficiais do PokéIdle:** Moth Controller lê automaticamente
   `state.kick.oficiais` do WebSocket autenticado de `pokeidle.io/app`,
   usando `slug`/`login` quando `aoVivo` é verdadeiro. Os canais aparecem
   em **Canais oficiais ao vivo** no painel Kick. O browser não precisa
   permanecer aberto: a sessão nativa fornece os mesmos dados. Durante o
   login pelo Firefox, o scanner antigo pode servir como fallback se estiver
   disponível. O monitor adiciona novos streamers e desconecta os que saíram
   do ar sem reiniciar os demais canais.
3. **Canais manuais:** é opcional adicionar nomes ou URLs usando
   **+ Adicionar canal**. Até 12 canais simultâneos por conta; os oficiais
   ao vivo têm prioridade. É possível remover canais manuais pelos chips.
   O arquivo `kick-channels.json` guarda apenas nomes informados manualmente,
   nunca credenciais nem o catálogo volátil de lives oficiais.
3. **Iniciar transmissões:** inicia as sessões da conta selecionada. Se
   existir um token protegido salvo, basta clicar novamente em iniciar nas
   próximas execuções. **Parar transmissões** encerra as conexões. Para
   remover o token do cofre, utilize **Esquecer login salvo**; isto também
   encerra as transmissões da conta.
4. **Atividade:** cartões responsivos mostram cada canal, estado ao vivo,
   conexão WebSocket, fluxo HLS, saldo de pontos e ganho observado. A
   seção **Detalhes da conexão** concentra bytes, segmentos, bitrate e erros.

**Interpretação dos pontos:** apenas a diferença positiva entre duas
consultas numéricas válidas do saldo da própria conta é contabilizada como
ganho observado. Um WebSocket conectado ou HLS recebendo dados **não
comprova** que a Kick creditou pontos. A coleta usa endpoints não documentados
e deve ser validada pelo usuário em uma conta real.

**Arquitetura:** cada canal roda nativamente em Rust, sem Streamlink, VLC,
MPV ou Python. O worker obtém dados e playback URL da API web Kick, estabelece
uma sessão no viewer WebSocket e recebe mídia HLS escolhendo a menor taxa
anunciada, descartando bytes sem decodificar. Não há mensagens de chat.
Por transmitir os bytes do vídeo, **o consumo de banda pode ser considerável**.

**Segurança:** o login Kick é isolado das credenciais PokéIdle/Twitch.
Windows usa um arquivo criptografado com DPAPI específico do perfil Kick;
Linux usa um item separado no Secret Service. Não existe fallback para
arquivo em texto simples. O token pode expirar e exigir substituição.
Não compartilhe tokens em logs, issues ou repositórios.

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
