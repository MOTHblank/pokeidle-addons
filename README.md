# pokeidle-addons

Addons para **PokéIdle** e um controlador nativo leve para manter até quatro contas em perfis separados do Firefox.

O projeto foi feito para Windows. O controlador usa o Firefox instalado normalmente, com **um perfil isolado por conta**. Os userscripts continuam sendo executados pelo **Violentmonkey**; o controlador não possui um motor próprio de userscripts.

## Instalação

Instale estes componentes antes de configurar o controlador:

1. **Firefox Developer Edition**  
   [Baixar Firefox Developer Edition](https://www.mozilla.org/pt-BR/firefox/developer/)

   O controlador procura primeiro o Firefox Developer Edition. A própria edição Developer usa caminho e perfil próprios, o que permite mantê-la separada de outras instalações do Firefox.

2. **Firemin**  
   [Baixar Firemin](https://rizonesoft.com/downloads/firemin/)

   O Firemin é opcional, mas recomendado para reduzir o uso de memória do Firefox quando os perfis ficam abertos por longos períodos. Deixe o Firemin em execução e configure-o para monitorar o Firefox Developer Edition.

3. **Violentmonkey para Firefox**  
   [Instalar Violentmonkey](https://addons.mozilla.org/pt-BR/firefox/addon/violentmonkey/)

   O Violentmonkey é obrigatório para os userscripts deste repositório. **Cada perfil isolado do controlador é independente**, portanto o Violentmonkey precisa estar instalado em cada conta que você pretende usar.

4. **PokéIdle**  
   [Abrir PokéIdle](https://pokeidle.io/app)

## Primeira configuração

Abra o **Moth Controller**.

A interface começa em **PT-BR**. No menu lateral existe um seletor **EN / PT-BR** para trocar o idioma a qualquer momento.

### 1. Configure as contas

Abra **Contas**.

Existem quatro slots independentes:

- **Game 1**
- **Game 2**
- **Game 3**
- **Game 4**

Os dois primeiros vêm habilitados por padrão para manter compatibilidade com a configuração antiga.

Para cada conta:

1. Marque **Habilitada**.
2. Dê um nome para a conta, por exemplo **Main**, **Alt**, **Shiny Hunter** ou o nome que preferir.
3. Feche a janela ou continue configurando os outros slots.

Os nomes e o estado habilitado são salvos automaticamente em:

    %LOCALAPPDATA%\Moth\PokeIdle\accounts.json

Os dados do navegador de cada slot ficam separados em:

    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game1
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game2
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game3
    %LOCALAPPDATA%\Moth\PokeIdle\Profiles\Game4

Cada diretório é uma sessão Firefox independente. Cookies, localStorage, login do PokéIdle e login das plataformas de streaming não são compartilhados entre contas.

### 2. Faça login em cada conta

Para cada slot habilitado:

1. Use o **Twitch** na janela de Contas para abrir o login da Twitch naquele perfil.
2. Use o **KICK** para abrir o KICK naquele mesmo perfil.
3. Abra o jogo nesse perfil e faça login no **PokéIdle**.
4. Termine o login de todas as plataformas antes de passar para a próxima conta.

**Não misture logins entre perfis.** Game 1 deve permanecer com as sessões da conta 1; Game 2 com as da conta 2; e assim por diante.

### 3. Instale o Violentmonkey em cada perfil

O perfil do Firefox criado pelo controlador é separado do seu Firefox pessoal.

Faça o seguinte para cada conta habilitada:

1. Abra o perfil pelo controlador.
2. Abra a página do [Violentmonkey](https://addons.mozilla.org/pt-BR/firefox/addon/violentmonkey/).
3. Instale a extensão.
4. Volte ao PokéIdle.
5. Na janela **Contas**, use **Addons**.

O botão **Addons** abre os instaladores dos scripts principais no perfil selecionado. A primeira instalação passa pela confirmação normal do Violentmonkey; depois as próprias URLs de update permitem que o Violentmonkey mantenha os scripts atualizados.

## Userscripts

### Scripts instalados pelo botão Addons

O botão **Addons** instala todos os userscripts recomendados para o perfil selecionado:

**Auto Catch+**  
Captura automática configurável e estatísticas de captura. O Auto Catch também possui reposição automática independente para **bolas, poções e Revives**.

**Performance+**  
Reduz trabalho de renderização e loops da interface do PokéIdle. É recomendado para todos os perfis.

**Live Stream Scanner**  
Verifica os streams atuais e abre automaticamente os chats da Twitch e as páginas KICK necessárias. A primeira verificação acontece após aproximadamente 30 segundos e depois uma vez por hora.

**Controller Bridge**  
É o principal userscript de integração com o aplicativo Rust. Ele fornece ao controlador dados do jogo, caça, estado de login, recursos, abas, bônus e mercado.

**Twitch + KICK Low Resource Mode**  
Mantém Twitch e KICK no menor consumo prático. Chats da Twitch permanecem leves; no KICK, a página normal do canal é mantida para que o player real continue disponível para o tempo de exibição/Channel Points.

**PokéIdle Hunt Atlas**  
Substitui a tela **Mapa** por uma interface de busca e comparação de hunts. O userscript é instalado automaticamente pelo botão **Addons**.

**PokéIdle Moth Watch**  
Monitoramento e compra configurável do mercado. O userscript também é instalado automaticamente pelo botão **Addons**.

## Como configurar os userscripts

Os scripts salvam suas próprias configurações no armazenamento do **perfil Firefox**. Isso significa que uma configuração feita na Conta 1 não altera a Conta 2.

### Auto Catch+

Abra o painel do **Auto Catch+** dentro do PokéIdle e escolha:

- ativação da captura automática;
- bolas que podem ser usadas;
- reservas mínimas;
- opções de reposição;
- intervalo de operação.

Comece com as reservas conservadoras e ajuste depois de confirmar que a captura está funcionando como esperado.

### Performance+

Abra o painel **Performance+** no PokéIdle.

Ele vem habilitado por padrão. O perfil de desempenho pode ser ajustado ali. Para contas usadas apenas para ficar AFK, mantenha-o ativo.

### Live Stream Scanner

Normalmente não exige configuração manual.

O objetivo do script é abrir automaticamente os streams encontrados pelo jogo. Para a Twitch, o fluxo leve usa **chat**. Para o KICK, ele abre a página real do canal porque o tempo de exibição é necessário para recursos como Channel Points.

O scanner faz a primeira verificação depois de aproximadamente 30 segundos e repete a cada hora.

### Twitch + KICK Low Resource Mode

Normalmente não exige configuração manual. Ele começa a atuar quando o script encontra uma página compatível da Twitch ou do KICK.

**Não desative esse script nos perfis usados para os streams automáticos** caso o objetivo seja minimizar o consumo de recursos.

### Controller Bridge

Não possui configuração de usuário normal. Deixe-o ativo nos perfis controlados pelo Moth Controller.

Sem o Bridge, o controlador não consegue receber o estado do jogo.

### Moth Watch

Abra a interface **Moth Watch** do userscript no PokéIdle para configurar:

- porcentagem máxima do preço observado;
- porcentagem para compra automática;
- intervalo de varredura;
- mercado de itens;
- mercado de Pokémon;
- compra automática de itens;
- compra automática de Pokémon;
- reservas de ouro;
- reservas de gemas;
- limites por compra.

A compra automática começa desativada por padrão. Ative-a somente depois de conferir as regras de preço e as reservas da conta.

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

### Hunt Atlas

O **Hunt Atlas** no controlador é uma visão nativa baseada nos dados recebidos pelo **Controller Bridge**.

Ele permite selecionar cada conta e consultar as hunts observadas, ordenar por XP/h e viajar diretamente para uma hunt.

O Atlas nativo do controlador não substitui o userscript completo do Hunt Atlas. Para a experiência completa da tela **Mapa**, instale o userscript separado.

### Moth Watch

A seção nativa **Moth Watch** mostra dados de mercado recebidos pelo Bridge e permite consultar e enviar compras pelo perfil selecionado.

### Logs

**Logs** abre o log do controlador em:

    %LOCALAPPDATA%\Moth\PokeIdle\moth-controller.log

## Modelo de recursos

O objetivo do projeto é manter o número de processos do navegador baixo:

- um Firefox isolado por conta;
- Twitch em modo de chat leve quando possível;
- KICK com página real somente quando necessário para manter o tempo de exibição/Channel Points;
- nenhum navegador oculto embutido;
- nenhum WebView2;
- nenhuma implementação própria de userscript.

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

Depois use **Contas → Addons**.

---

## English

A collection of **PokéIdle** addons plus a lightweight native controller for keeping up to four accounts in isolated Firefox profiles.

The project targets Windows. The controller uses normal installed Firefox with **one isolated browser profile per account**. Userscripts continue to run through **Violentmonkey**; the controller does not contain its own userscript engine.

### Installation

1. [Download Firefox Developer Edition](https://www.mozilla.org/firefox/developer/)
2. [Download Firemin](https://rizonesoft.com/downloads/firemin/)
3. [Install Violentmonkey for Firefox](https://addons.mozilla.org/en-US/firefox/addon/violentmonkey/)
4. [Open PokéIdle](https://pokeidle.io/app)

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

Install Violentmonkey inside each enabled profile, then use **Accounts → Addons** to open the core addon installers.

### Core userscripts

The **Addons** button installs all recommended userscripts for the selected profile:

- Auto Catch+ — including independent ball, potion and revive restocking
- Performance+
- Live Stream Scanner
- Controller Bridge
- Twitch + KICK Low Resource Mode
- PokéIdle Hunt Atlas
- PokéIdle Moth Watch

There is no separate installation flow. **Accounts → Addons** opens the complete addon set in the selected Firefox profile.

### Controller usage

**Dashboard** shows enabled accounts and their live state.

**Accounts** manages the four profile slots, account names, browser/logins and addon installation.

**Launch enabled** starts every enabled account.

**Stop all** closes every controlled Firefox instance.

**Hunt Atlas** is a native controller view using data from Controller Bridge. The full Hunt Atlas map replacement remains a separate userscript.

**Moth Watch** is the native market view using market data received through the Bridge.

The dashboard uses vertical scrolling rather than shrinking account cards. Wide windows use two columns; narrow windows use one.

---

**MOTHblank** · [Google Play](https://play.google.com/store/apps/developer?id=MOTHblank) · [X](https://x.com/MOTHblank) · [WhatsApp / Pix](https://wa.me/+5537999933376)
