# Imagens nativas do Moth Controller

## Ícones locais

Os ícones usados pelo controlador são extraídos da versão verificada de [pokeidle-upstream](https://github.com/MOTHblank/pokeidle-upstream) e incluídos diretamente no executável com `include_bytes!`.

- **33 arquivos PNG/GIF**, compartilhados por **50 IDs de Poké Bolas e itens**.
- Todos os ícones de **Auto Restock**: cinco Poké Bolas, sete poções/revives e a Golden Potion.
- Outros itens confirmados pelo catálogo do jogo, como pedras, discos e tokens.
- Nenhum sprite de Pokémon ou atlas de Pokémon é incluído. Esses recursos continuam usando o cache dinâmico existente.

Os arquivos ficam em `controller/assets/visual/bundled/`, preservando seus caminhos relativos do upstream. `manifest.json` registra a origem e o SHA-256 de cada recurso; `native_visual_assets.generated.rs` associa os IDs às imagens locais. Vários IDs podem compartilhar um único arquivo.

**O usuário não precisa baixar imagens nem configurar nada.** Ícones incluídos são carregados localmente na primeira exibição e as texturas são reutilizadas. Não há acesso à rede para esses ícones em condições normais. Itens novos ou não incluídos ainda podem usar o download sob demanda do site do jogo; o erro não interrompe a interface. O espelho remoto de `pokeidle-upstream` foi removido do runtime, pois o repositório é privado.

## Atualização dos ícones

Depois de atualizar uma cópia local de `pokeidle-upstream`, execute na raiz de `pokeidle-addons`:

```sh
python scripts/sync-native-visuals.py --upstream-dir ../pokeidle-upstream --local-only
```

O script valida os arquivos disponíveis, mantém os recursos locais cujo checksum não mudou e atualiza `manifest.json` e `native_visual_assets.generated.rs`. Para listar os IDs sem alterar arquivos:

```sh
python scripts/sync-native-visuals.py --upstream-dir ../pokeidle-upstream --local-only --dry-run
```

**Não use `--pokemon-index` para o conjunto de ícones locais:** o carregamento dinâmico de sprites deve permanecer independente.

Revise os direitos de uso e redistribuição dos arquivos de PokéIdle antes de publicar versões que contenham sua arte. O acesso aos arquivos do jogo não implica, por si só, licença para redistribuí-los.

## Cache de itens não incluídos

O cache de rede permanece disponível apenas para arquivos que não foram incluídos no executável:

- Windows: `%LOCALAPPDATA%\\Moth\\PokeIdle\\VisualCache`
- Linux: `$XDG_CACHE_HOME/moth-controller/visual` ou `~/.cache/moth-controller/visual`

A rede, a leitura de arquivos de cache e a decodificação de imagens baixadas são executadas fora da atualização normal da interface. Em caso de falha, o texto do item continua disponível.
