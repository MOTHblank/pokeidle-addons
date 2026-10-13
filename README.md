# pokeidle-addons

Addons para **PokéIdle** e um controlador nativo leve para manter até quatro contas em perfis separados do Firefox.

O controlador funciona no Windows e no Linux e usa o Firefox instalado normalmente, com **um perfil isolado por conta**. Os userscripts continuam sendo executados pelo **Violentmonkey**; o controlador não possui um motor próprio de userscripts.

## Transparência e desenvolvimento

Nenhuma parte do código deste projeto está escondida: todo o código-fonte está disponível neste repositório para que você possa revisá-lo. Você também pode pedir a uma IA de sua confiança para analisar o código e ajudar a identificar possíveis problemas de segurança ou comportamento.

Este aplicativo ainda está em desenvolvimento, então nem tudo funciona perfeitamente. Caso encontre algum problema, abra uma [Issue](https://github.com/MOTHblank/pokeidle-addons/issues) descrevendo o que aconteceu. Sugestões de funcionalidades que estejam faltando também são bem-vindas. E, se você sabe programar, contribuições e pull requests são bem-vindos.

## Requisitos

Instale estes componentes antes de configurar o controlador:

1. **Firefox Developer Edition**  
   [Baixar Firefox Developer Edition](https://www.mozilla.org/pt-BR/firefox/developer/)

   O controlador procura primeiro o Firefox Developer Edition. A própria edição Developer usa caminho e perfil próprios, o que permite mantê-la separada de outras instalações do Firefox.

2. **Firemin (somente Windows)**  
   [Baixar Firemin](https://rizonesoft.com/downloads/firemin/)

   O Firemin é opcional no Windows, mas pode ajudar a reduzir a memória do Firefox em sessões longas. Configure-o para monitorar o Firefox Developer Edition. **Não é necessário no Linux.**


## Primeira configuração
Video configurando uma conta: https://www.youtube.com/watch?v=8nO0gfs0jPQ

Abra o **Moth Controller**.

A interface começa em **PT-BR**. No menu lateral existe um seletor **EN / PT-BR** para trocar o idioma a qualquer momento.

### 1. Configure as contas

Abra **Contas**.

Existem quatro slots independentes:

- **Game 1**
- **Game 2**
- **Game 3**
- **Game 4**

Os dois primeiros vêm habilitados por padrão.

Para cada conta:

1. Marque **Habilitar**.
2. Dê um nome para a conta, por exemplo **Main**, **Alt**, **Shiny Hunter** ou o nome que preferir.
3. Use **Abrir jogo** para configurar a conta em uma janela visível do Firefox.
4. Clique em **Instalar Violentmonkey** e instale a extensão naquele perfil do Firefox.
5. Em **Gerenciar addons**, clique em **Instalar todos (primeira configuração)**. Confirme os instaladores no Violentmonkey. Depois disso, as atualizações são feitas pelo Violentmonkey; não repita a instalação.
6. Abra o PokéIdle e faça login. Use **Twitch** para entrar na conta correta em cada perfil.
7. O guia de configuração é ocultado quando o jogo e todos os scripts são verificados. Se você optar por instalar apenas alguns addons, clique em **Concluir configuração** para ocultar o guia. Você pode reabri-lo depois.


**Não misture logins entre perfis.** Game 1 deve permanecer com as sessões da conta 1; Game 2 com as da conta 2; e assim por diante.


## Userscripts

### Scripts disponíveis em Gerenciar addons

**Gerenciar addons** mostra quais scripts estão em execução, versões antigas e scripts não detectados. **Reparar selecionados** abre instaladores somente dos itens marcados; **Instalar todos (primeira configuração)** abre todos os userscripts recomendados para o perfil selecionado. A abertura do instalador não significa que a instalação foi concluída: confirme-a no Violentmonkey.

**Auto Catch+**  
Captura automática configurável e estatísticas de captura. O Auto Catch também possui reposição automática independente para **bolas, poções e Revives**.

**Performance+**  
Reduz trabalho de renderização e loops da interface do PokéIdle. É recomendado para todos os perfis.

**Live Stream Scanner**  
Abre chat de streamers online a cada cinco minutos. Para a KICK (que barra o acesso automatizado), um link multistream fica disponível na interface.

**Controller Bridge**  
É o principal userscript de integração com o aplicativo Rust. Ele fornece ao controlador dados do jogo.

**Twitch Low Resource Mode**  
Mantém as páginas da Twitch no menor consumo prático. Pop-outs de chat da Twitch permanecem intocados; a redução de recursos é aplicada somente à Twitch.

**PokéIdle Hunt Atlas**  
Substitui a tela **Mapa** por uma interface de busca e comparação de hunts. 

**Moth Watch (separado)**  
Não é mais distribuído por este repositório nem instalado pelo Controller. Para uso opcional, consulte [pokeidle-addons-premium](https://github.com/MOTHblank/pokeidle-addons-premium). A configuração e as atualizações devem ser feitas separadamente no Violentmonkey.

## Como configurar os userscripts

Os scripts salvam suas próprias configurações no armazenamento do **perfil Firefox**. Isso significa que uma configuração feita na Conta 1 não altera a Conta 2.

### Auto Catch+

Abra o painel do **Auto Catch+** dentro do PokéIdle e escolha:

- ativação da captura automática;
- bolas que podem ser usadas;
- reservas mínimas;
- intervalo de operação.

Na seção **Auto-restock**, bolas, poções e **Revives são categorias independentes**. Cada uma possui seu próprio item, limite **at** e limite **to**. É possível reabastecer poções e Revives ao mesmo tempo; um não substitui o outro.

Comece com as reservas conservadoras e ajuste depois de confirmar que a captura está funcionando como esperado.

### Performance+

Abra o painel **Performance+** no PokéIdle.

Ele vem habilitado por padrão. O perfil de desempenho pode ser ajustado ali. Para contas usadas apenas para ficar AFK, mantenha-o ativo.


### Moth Watch (instalação separada)

A partir da próxima versão, o **Moth Watch não é instalado, atualizado nem distribuído** pelo Controller ou por este repositório. A versão premium possui [código e instruções próprias](https://github.com/MOTHblank/pokeidle-addons-premium). No Firefox, remova ou substitua a cópia anterior do userscript para evitar duas instâncias executando juntas. A automação de compra por regras permanece desligada por padrão em perfis novos, e não há compra automática de Pokémon.

## Usando o Moth Controller

### Dashboard

O **Dashboard** mostra o estado de todos os perfis habilitados.

Cada conta mostra:

- estado do Firefox;
- estado do login;
- caça atual;
- nível e XP;
- recursos;
- Auto Catch;
- abas abertas;
- streams e bônus;
- diagnóstico do Bridge.

Quando quatro contas estão habilitadas, o painel mantém as caixas sem esmagá-las: em uma janela larga são usadas duas colunas; em janelas menores elas passam para uma coluna e o conteúdo ganha rolagem vertical.

### Iniciar e parar contas

**Iniciar habilitadas** inicia todas as contas marcadas como habilitadas no Gerenciador de Contas.

Cada cartão individual também possui seu próprio botão **Iniciar / Parar**.

**Parar tudo** fecha todos os Firefox controlados pelo aplicativo.

O controlador mantém um monitor BiDi separado para cada perfil.

### Contas

Use **Contas** para:

- alterar o nome dos quatro slots;
- habilitar/desabilitar uma conta;
- abrir o jogo;
- abrir Twitch;
- abrir KICK;
- instalar os userscripts principais;
- abrir a pasta do perfil.

Desabilitar uma conta a remove do Dashboard e de **Iniciar habilitadas**, mas não apaga o perfil nem seus logins.

### Logs

**Logs** abre o log do controlador em:

    %LOCALAPPDATA%\Moth\PokeIdle\moth-controller.log

## Modelo de recursos

O objetivo do projeto é manter o número de processos do navegador baixo:

- um Firefox isolado por conta;
- Twitch em modo de chat leve quando possível;

O controlador apenas coordena processos, perfis e comunicação. O navegador continua sendo o Firefox oficial e o Violentmonkey continua sendo responsável pela execução dos userscripts.

## Solução de problemas

### O controlador não encontra o Firefox

Instale o Firefox Developer Edition. Ele é o executável preferido pelo controlador.

Caso o Firefox esteja instalado em um local incomum, defina a variável de ambiente:

    MOTH_FIREFOX=C:\caminho\para\firefox.exe

e abra novamente o controlador.

### O Dashboard não mostra os dados do jogo

Confirme que **Controller Bridge** está instalado e habilitado no perfil correto.

Depois abra ou recarregue o PokéIdle naquele perfil e aguarde o Bridge enviar os dados.

### Uma conta aparece misturada com outra

Não reutilize o perfil do Firefox pessoal nem copie cookies entre slots.

Cada conta deve usar exclusivamente:

    Profiles\Game1
    Profiles\Game2
    Profiles\Game3
    Profiles\Game4

### Os userscripts não aparecem

Primeiro confirme que o **Violentmonkey foi instalado naquele perfil específico**. Instalar a extensão no seu Firefox pessoal não instala a extensão nos quatro perfis isolados do controlador.

Depois use **Contas → Gerenciar addons → Instalar todos (primeira configuração)** somente na primeira instalação. **Não é necessário repetir esse passo a cada atualização.** O Violentmonkey pode baixar automaticamente as novas versões de cada script, desde que a verificação automática de atualizações esteja habilitada nesse perfil do Firefox. Recarregue a aba do jogo depois que uma atualização for instalada. Em **Gerenciador de contas → Gerenciar addons**, confira as versões detectadas durante a execução e as versões publicadas no GitHub. Addons antigos que ainda não informam sua versão aparecem como **não verificados**, e não como atualizados.

---

## English

A collection of **PokéIdle** addons plus a lightweight native controller for keeping up to four accounts in isolated Firefox profiles.

The controller supports Windows and Linux. The controller uses normal installed Firefox with **one isolated browser profile per account**. Userscripts continue to run through **Violentmonkey**; the controller does not contain its own userscript engine.

### Transparency and development

No part of this project's code is hidden: the complete source code is available in this repository for anyone to review. You can also ask an AI you trust to review the code and help identify potential security or behavioral problems.

This application is still under development, so not everything will work perfectly yet. When you encounter a problem, open an [Issue](https://github.com/MOTHblank/pokeidle-addons/issues) describing what happened. Suggestions for missing features are welcome as well. And if you know how to program, contributions and pull requests are welcome.

### Requirements

1. [Download Firefox Developer Edition](https://www.mozilla.org/firefox/developer/)
2. [Download Firemin](https://rizonesoft.com/downloads/firemin/)

Violentmonkey must be installed separately in every isolated controller profile you enable.

### First setup

Open **Moth Controller**, then open **Accounts**.

Enable and name **Game 1** through **Game 4** as needed. The first two slots are enabled by default.

Each slot gets its own Firefox profile:

    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game1
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game2
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game3
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game4

Use the account's **Twitch**, **KICK**, and game entry points from that same profile. Never mix account sessions between profiles.

Install Violentmonkey inside each enabled profile, then use **Accounts → Manage Addons → Install all (first setup)**. Confirm each installer inside Violentmonkey.

### Core userscripts

The **Manage Addons** section offers these userscripts for the selected profile:

- Auto Catch+ — including independent ball, potion and revive restocking
- Performance+
- Live Stream Scanner
- Controller Bridge
- Twitch Low Resource Mode
- PokéIdle Hunt Atlas

**Install all (first setup)** opens every installer only when needed. **Repair selected** opens installers just for the checked userscripts; an opened installer is not confirmation of installation. Violentmonkey's automatic update setting (enabled separately in each Firefox profile) handles newer `@version` releases. **Check now** refreshes GitHub version information without touching the game or opening tabs. **Manage Addons** compares the versions reported by running userscripts with those published on GitHub. A dashboard warning links to the affected account when scripts need attention. Reload the game manually after an update; the controller deliberately does not interrupt gameplay. First-time setup guidance disappears once the complete live setup is verified or the user explicitly finishes setup with their chosen scripts, and can be reopened later.

### Controller usage

**Dashboard** shows enabled accounts and their live state.

**Accounts** manages the four profile slots, account names, browser/logins and addon installation.

**Launch enabled** starts every enabled account.

**Stop all** closes every controlled Firefox instance.

**Hunt Atlas** remains an in-game userscript installed through **Accounts → Addons**. The Rust controller no longer provides a separate Hunt Atlas window.

**Moth Watch** moved to [pokeidle-addons-premium](https://github.com/MOTHblank/pokeidle-addons-premium). The standard controller will not discover, install, or update it; use Violentmonkey separately if you choose to install the optional script.

The dashboard uses vertical scrolling rather than shrinking account cards. Wide windows use two columns; narrow windows use one.

---

**MOTHblank** · [Google Play](https://play.google.com/store/apps/developer?id=MOTHblank) · [X](https://x.com/MOTHblank) · [WhatsApp / Pix](https://wa.me/+5537999933376)


## Versão para Linux (experimental)

A partir da **v0.1.3**, a página de [Releases](https://github.com/MOTHblank/pokeidle-addons/releases) também oferece um arquivo `moth-controller-v0.1.3-linux-x86_64.tar.gz`. Ele contém o executável nativo e instruções em português.

Extraia o arquivo e execute `chmod +x moth-controller && ./moth-controller` em um terminal dentro da pasta extraída.

É necessário ter uma sessão gráfica **X11 ou Wayland**, **Firefox Developer Edition** (ou Firefox), **Violentmonkey**, `curl`, `xdg-utils` e as bibliotecas gráficas correspondentes à distribuição. O executável é compilado em Ubuntu 22.04 (x86_64) para ampliar a compatibilidade com distribuições Linux atuais, mas ainda não foi validado em todas elas.

Os dados ficam em `$XDG_DATA_HOME/Moth/PokeIdle` ou, por padrão, em `~/.local/share/Moth/PokeIdle`. Caso o Firefox não seja detectado, defina `MOTH_FIREFOX=/caminho/para/firefox`. Os perfis do Windows e Linux são separados; no Linux ainda não existe integração com a bandeja do sistema. Consulte [controller/README.md](controller/README.md) para mais informações.
