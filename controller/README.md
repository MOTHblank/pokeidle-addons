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

1. **Login pelo Firefox normal:** em **Contas → Kick** clique em
   **Entrar pelo Firefox normal**. Para evitar os desafios de autenticação
   causados por um navegador instrumentado, Moth abre uma **janela comum,
   sem Remote Agent, BiDi, WebDriver ou scripts injetados**, usando um
   perfil limpo e persistente dedicado à Kick para cada conta
   (`Kick-Profiles/Game1` etc.). O perfil Kick não compartilha o Firefox
   monitorado do PokéIdle: o login pode ocorrer com a sessão nativa do jogo
   já em execução. **Entre na Kick normalmente e feche essa janela do
   Firefox quando terminar.** Após a saída do navegador, Moth lê somente
   o cookie `session_token` pertencente a `kick.com` no banco local
   `cookies.sqlite` daquele perfil, valida o formato e guarda a sessão
   apenas no cofre protegido pelo SO (DPAPI / Secret Service).
   **Nenhum token precisa ser copiado manualmente.** O Moth não lê cookies
   de outros domínios nem mantém o navegador aberto após o login.
   Se o cookie estiver ausente, for incompatível ou tiver expirado, a
   interface mostra o erro em vez de declarar o login bem-sucedido.
   A compatibilidade com a autenticação interna não documentada da Kick
   depende de um teste real. **Avançado → Token manual** continua disponível.
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
4. **Iniciar transmissões:** inicia as sessões da conta selecionada. Se
   existir um token protegido salvo, basta clicar novamente em iniciar nas
   próximas execuções. **Parar transmissões** encerra as conexões. Para
   remover o token do cofre, utilize **Esquecer login salvo**; isto também
   encerra as transmissões da conta.
5. **Canais Kick:** lista compacta com **uma linha por canal**, incluindo
   os streamers oficiais que estejam **offline**. Exibe o nome, **AO VIVO /
   OFFLINE / Não verificado**, o **saldo de pontos individual da conta
   naquele canal** e o botão **Abrir ↗**. Os dados de WebSocket/HLS ficam
   em **Detalhes**. O saldo vem somente de uma resposta autenticada da Kick;
   **— significa indisponível, nunca zero**. Dados antigos são sinalizados.
   Com o monitor iniciado, um único worker de baixa frequência consulta os
   canais oficiais sem transmissão, sem abrir HLS ou criar uma thread por
   streamer offline. A lista oficial completa vem da sessão nativa do
   PokéIdle; no fallback do navegador podem aparecer somente os ao vivo.

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
