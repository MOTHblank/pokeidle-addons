# Moth Controller — Linux (x86_64)

O Moth Controller roda como **executável Rust nativo**, com interface X11 ou Wayland. Não necessita de Wine ou de um navegador incorporado. O Firefox é necessário somente para a autenticação ou para abrir manualmente o jogo; a sessão nativa pode continuar depois que o navegador fecha.

## Dependências

- Linux x86_64 com ambiente gráfico (binário compilado para compatibilidade com Ubuntu 22.04 ou mais recente; outras distribuições precisam de validação).
- Firefox ou Firefox Developer Edition, instalado separadamente e disponível no `PATH`.
- `curl` (rede/integração Kick) e `xdg-utils` (abrir links e logs).
- `libsqlite3-0`: biblioteca SQLite de execução usada para importar o cookie Kick de um perfil Firefox.
- `libsecret-tools` e **um Secret Service desbloqueado** (por exemplo, GNOME Keyring ou KWallet com suporte Secret Service) para persistir credenciais PokéIdle, Twitch e Kick com segurança.
- Bibliotecas de execução X11/Wayland, OpenGL e GTK 3.

No Ubuntu 22.04 ou Debian compatível, normalmente é suficiente instalar:

```bash
sudo apt-get update
sudo apt-get install -y firefox curl xdg-utils libsqlite3-0 libsecret-tools gnome-keyring libgtk-3-0 libgl1
```

O pacote Firefox pode ser fornecido por Snap ou por um repositório de sua distribuição. Se o lançador instalado não aparecer no `PATH`, configure `MOTH_FIREFOX` com o caminho do executável. O usuário deve estar em uma sessão de desktop com D-Bus e chaveiro desbloqueado. Não é necessário nem recomendado executar Moth Controller como root.

## Execução

Extraia o arquivo Linux `.tar.gz` em um diretório de sua escolha:

```bash
./check-linux-deps.sh
./moth-controller
```

O verificador é apenas diagnóstico: não instala pacotes, não inicia o Firefox e não lê senhas ou tokens. Para usar outro Firefox:

```bash
MOTH_FIREFOX=/caminho/para/firefox ./moth-controller
```

O nome de comando `MOTH_FIREFOX=firefox` também é aceito se o executável estiver no `PATH`.

## Arquivos e segurança

Perfis, preferências, histórico e logs ficam em `$XDG_DATA_HOME/Moth/PokeIdle/` ou, por padrão, em `~/.local/share/Moth/PokeIdle/`. O cache de imagens fica em `$XDG_CACHE_HOME/moth-controller/visual/` ou em `~/.cache/moth-controller/visual/`. Os perfis Windows não são importados automaticamente.

**Credenciais** são persistidas pelo Secret Service; não há fallback para salvar segredos em texto simples. Se o chaveiro estiver ausente ou bloqueado, o login poderá ter de ser repetido após reiniciar a aplicação. Para efetivar a captura de sessão, a integração Firefox BiDi deve funcionar normalmente no Firefox instalado.

## Compilação Linux

Dentro de `controller/`, instale Rust e as bibliotecas de desenvolvimento necessárias. No Ubuntu 22.04:

```bash
sudo apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev \
  libgtk-3-dev libudev-dev libx11-dev libxi-dev libxcursor-dev \
  libxrandr-dev libxinerama-dev libxkbcommon-dev libwayland-dev \
  libgl1-mesa-dev libegl1-mesa-dev
cargo test --locked
cargo build --release --locked
```

O `libsqlite3-dev` é **necessário na compilação** porque a importação do cookie Kick usa `-lsqlite3`. O `libsqlite3-0` é suficiente na máquina que só executará o binário.

**Limitações:** sem ícone de bandeja no Linux; ainda não distribuído como `.deb`, `.rpm`, Flatpak ou AppImage. Ambientes Wayland/X11, navegadores empacotados em Snap/Flatpak, desktops sem Secret Service e distribuições diferentes ainda exigem testes de integração reais.
