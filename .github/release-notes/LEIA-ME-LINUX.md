# Moth Controller — Linux (x86_64)

Esta versão do Moth Controller é um executável nativo para **Linux de 64 bits (x86_64)**. Ela não é um instalador e não inclui o Firefox.

## Requisitos

- Linux com ambiente gráfico **X11 ou Wayland** e bibliotecas compatíveis com Ubuntu 22.04 ou mais recentes.
- **Firefox Developer Edition** (preferencial) ou Firefox, instalado separadamente.
- **Violentmonkey** instalado no perfil do Firefox de cada conta.
- Pacotes **curl** e **xdg-utils** instalados na distribuição.
- Bibliotecas de sistema para X11/Wayland, OpenGL e GTK 3 (geralmente já incluídas nos desktops Linux).

Em Ubuntu e derivados, caso faltem dependências de execução, confira os pacotes `libgtk-3-0`, `libx11-6`, `libxkbcommon0`, `libwayland-client0` e `libgl1`.

## Como executar

Extraia o arquivo `.tar.gz`, abra um terminal na pasta extraída e execute:

```bash
chmod +x moth-controller
./moth-controller
```

Se o Firefox não for encontrado automaticamente, informe o caminho do executável:

```bash
MOTH_FIREFOX=/caminho/para/firefox ./moth-controller
```

Os perfis são guardados em `$XDG_DATA_HOME/Moth/PokeIdle/` ou, por padrão, em `~/.local/share/Moth/PokeIdle/`. Os perfis do Windows não são importados automaticamente.

**Limitações conhecidas:** não há ícone de bandeja no Linux; não há pacote .deb, .rpm, Flatpak ou AppImage nesta versão. A execução em diferentes distribuições ainda precisa ser validada. O modo headless do Firefox é opcional.

Para mais informações, consulte o arquivo `controller/README.md` no repositório: https://github.com/MOTHblank/pokeidle-addons
