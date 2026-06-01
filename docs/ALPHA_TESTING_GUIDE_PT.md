# Openfoot Manager — Guia para Testadores Alpha

Olá e bem-vindo ao alpha do **Openfoot Manager**! Antes de mais — muito obrigado por estares aqui. A sério. O facto de estares disposto a jogar um jogo inacabado e a ajudar-nos a melhorá-lo significa imenso para nós.

Este guia vai explicar-te o que é o jogo, o que funciona, o que ainda não funciona, e como nos podes ajudar da melhor forma.

---

## O que é o Openfoot Manager?

O Openfoot Manager é um **jogo de simulação de gestão de futebol open-source**. Pensa nele como uma carta de amor ao género clássico de football manager — assumes o comando de um clube, geres o teu plantel, defines táticas, tratas de transferências e guias a tua equipa ao longo de uma temporada completa de futebol competitivo.

É construído como uma aplicação de desktop com [Tauri](https://tauri.app/) (backend em Rust + frontend em React), o que significa que corre nativamente em Windows, macOS e Linux sem precisar de browser ou ligação à internet.

### O que está neste alpha

Eis o que podes fazer neste momento:

- **Criar um treinador** com o teu nome, data de nascimento e nacionalidade
- **Escolher uma equipa** de uma liga gerada com 16 equipas
- **Gerir o teu plantel** — definir formações, escolher o onze inicial, designar marcadores de bolas paradas
- **Treinar os teus jogadores** — escolher foco de treino, intensidade e calendário semanal
- **Contratar e dispensar staff** — treinadores adjuntos, fisioterapeutas, olheiros, assistentes
- **Jogar jogos** — simulação minuto a minuto com controlos táticos, ou delegar no teu assistente
- **Palestras ao intervalo** e **conferências de imprensa pós-jogo** que afetam a moral
- **Ler as notícias** — relatórios de jogos, resumos de jornada, atualizações da classificação
- **Gerir a tua caixa de entrada** — mensagens do staff, direção e eventos do jogo
- **Observar jogadores** e **fazer transferências**
- **Acompanhar as finanças** — salários, orçamentos, receitas
- **Completar uma temporada inteira** e avançar para a seguinte

### O que NÃO está neste alpha

Para alinhar expectativas — aqui estão coisas planeadas mas ainda não implementadas:

- Múltiplas ligas / promoções / despromoções
- Competições de taça
- Negociação de contratos de jogadores
- Academia / formação de jovens
- Táticas detalhadas (marcação individual, lances de bola parada ensaiados, etc.)
- Multijogador
- Som / música
- Tutorial / introdução guiada para além da primeira semana

Vamos lá chegar. Mas agora, precisamos da tua ajuda para encontrar os bugs e as arestas no que já temos.

---

## Primeiros Passos

### Instalação

1. Descarrega o instalador para a tua plataforma a partir do link que te enviámos
2. Executa o instalador — no Windows pode aparecer um aviso do SmartScreen porque a aplicação ainda não está assinada digitalmente. Clica em "Mais informações" → "Executar mesmo assim"
3. Abre o **Openfoot Manager**

### O teu primeiro jogo

1. Clica em **New Game** no menu principal
2. Preenche os dados do teu treinador (nome, data de nascimento, nacionalidade)
3. Escolhe uma base de dados de mundo (o "Random World" por defeito está ótimo)
4. Escolhe uma equipa da lista — vê as estatísticas e escolhe uma que te pareça divertida
5. Vais parar ao **Dashboard** — este é o teu centro de comando

### Coisas importantes a saber

- O botão **Continue** (canto superior direito) avança o tempo em um dia. A seta ao lado permite saltar para o dia de jogo.
- A **barra lateral** à esquerda é a tua navegação — Plantel, Táticas, Treino, Calendário, Finanças, etc.
- Antes de um jogo, escolhes como queres jogá-lo: **Go to the Field** (controlo total), **Watch as Spectator** (ver a IA jogar), ou **Delegate to Assistant** (resultado instantâneo).
- Durante os jogos, podes fazer substituições, mudar a formação, alterar o estilo de jogo e ajustar os marcadores de bolas paradas.
- Após os jogos, há palestra de intervalo/final e uma conferência de imprensa opcional.
- O jogo **guarda automaticamente** quando voltas ao menu principal. Também podes guardar manualmente nas definições.

---

## O Que Precisamos de Ti

### A versão curta

Joga o jogo. Parte tudo. Conta-nos o que aconteceu.

### A versão mais longa

Procuramos feedback em três categorias:

#### 1. Bugs e crashes

Esta é a prioridade máxima. Se algo se partir, crashar, congelar, ou se comportar de forma claramente errada — queremos saber.

Exemplos:
- O jogo crasha quando tento iniciar um jogo
- As minhas alterações de formação não ficam guardadas entre sessões
- Um jogador aparece como lesionado mas continua no meu onze inicial
- A classificação não bate certo após a jornada 5
- Recebi uma mensagem sobre um jogo que ainda não aconteceu

#### 2. Problemas de usabilidade

Coisas que não estão partidas, mas são confusas, irritantes ou difíceis de usar.

Exemplos:
- Não consegui perceber como mudar o meu onze inicial
- A página de treino é confusa demais, não sei o que cada coisa faz
- O texto é demasiado pequeno / grande no meu ecrã
- Não percebo o que o "estilo de jogo" afeta realmente
- A caixa de entrada está cheia de mensagens que não me interessam

#### 3. Equilíbrio e sensação de jogo

Isto é mais subjetivo, mas igualmente valioso. O jogo *parece* correto?

Exemplos:
- A minha equipa ganha todos os jogos por 5-0, é demasiado fácil
- O treino não parece fazer diferença nenhuma
- A moral dos jogadores nunca muda, faça eu o que fizer
- As propostas de transferência são sempre rejeitadas
- As equipas da IA parecem demasiado fracas / fortes

---

## Como Enviar Feedback

A forma mais fácil de enviar feedback é pelos nossos **templates de issue no GitHub**. Basta escolheres o certo e preencheres — **podes escrever no teu idioma!**

Se quiseres falar com a equipa ou com outros jogadores, também podes juntar-te ao servidor de Discord: https://discord.gg/2CXaesaukT

- [**Relatório de Bug**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_pt.yml) — Algo crashou, partiu-se ou comportou-se incorretamente
- [**Feedback / Sugestão**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_pt.yml) — Problemas de usabilidade, equilíbrio ou ideias
- [**Relatório de Sessão**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_pt.yml) — Um resumo da tua sessão de jogo (super valioso!)

### Ficheiros de log

Quando reportares um bug, **por favor inclui os teus ficheiros de log**. Contêm informação detalhada sobre o que o jogo estava a fazer quando algo correu mal, e muitas vezes são a diferença entre conseguirmos corrigir um bug em 5 minutos ou passarmos horas a tentar reproduzi-lo.

**Onde encontrar os teus logs:**

- **Windows:** `C:\Users\<OTeuUtilizador>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Basta compactar (zip) toda a pasta `logs` e anexá-la ao teu relatório. Os logs não contêm informações pessoais — apenas eventos do jogo, comandos e rastreio de erros.

---

## Problemas Conhecidos

Aqui estão coisas que já sabemos — não precisas de as reportar (mas fica à vontade para comentar se afetarem a tua experiência):

- **Sem som ou música** — o jogo é completamente silencioso por agora
- **A escala da interface** pode não estar perfeita em todos os tamanhos de ecrã — verifica Definições → Display → UI Scale se algo parecer estranho
- **Parte do texto não está traduzido** — a internacionalização é um trabalho em curso
- **Os ficheiros de gravação deste alpha podem não ser compatíveis** com versões futuras. Não te apegues demasiado às tuas gravações!
- **O desempenho** pode diminuir ligeiramente ao simular muitos dias seguidos

---

## Umas Últimas Palavras

Este é um projeto de paixão. Não tem um grande estúdio nem um orçamento gordo por trás. É feito por pessoas que adoram futebol e adoram jogos, no tempo livre.

O teu feedback durante este alpha não é apenas "bom de ter" — está literalmente a moldar aquilo em que este jogo se vai tornar. Cada bug que reportares, cada "isto confundiu-me", cada "não seria fixe se..." — tudo conta.

Por isso joga, diverte-te (esperamos nós!), e não te contenhas no feedback. Não há perguntas parvas e nenhum feedback é pequeno demais.

Obrigado por fazeres parte disto. Vamos construir algo fantástico juntos.

— A equipa Openfoot Manager

---

*Versão alpha 0.2.0*
