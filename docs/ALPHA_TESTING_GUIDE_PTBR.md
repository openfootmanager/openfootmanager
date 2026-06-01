# Openfoot Manager — Guia para Testadores Alpha

E aí, tudo bem? Seja muito bem-vindo ao alpha do **Openfoot Manager**! Antes de mais nada — muito obrigado por estar aqui. De verdade. O fato de você topar jogar um jogo inacabado e nos ajudar a melhorá-lo significa muito pra gente.

Este guia vai te explicar o que é o jogo, o que funciona, o que ainda não funciona, e como você pode nos ajudar da melhor forma.

---

## O que é o Openfoot Manager?

O Openfoot Manager é um **jogo de simulação de gerenciamento de futebol open-source**. Pense nele como uma carta de amor ao gênero clássico de football manager — você assume o comando de um clube, gerencia seu elenco, define táticas, cuida de transferências e guia seu time por uma temporada completa de futebol competitivo.

Ele é construído como um app desktop usando [Tauri](https://tauri.app/) (backend em Rust + frontend em React), o que significa que roda nativamente no Windows, macOS e Linux sem precisar de navegador ou conexão com a internet.

### O que tem neste alpha

Aqui está o que você pode fazer agora:

- **Criar um treinador** com seu nome, data de nascimento e nacionalidade
- **Escolher um time** de uma liga gerada com 16 equipes
- **Gerenciar seu elenco** — definir formações, escalar o time titular, designar cobradores de bola parada
- **Treinar seus jogadores** — escolher foco de treino, intensidade e cronograma semanal
- **Contratar e dispensar funcionários** — técnicos auxiliares, fisioterapeutas, olheiros, assistentes
- **Jogar partidas** — simulação minuto a minuto com controles táticos, ou delegar ao seu assistente
- **Palestras no intervalo** e **coletivas de imprensa pós-jogo** que afetam o moral
- **Ler as notícias** — relatórios de partidas, resumos da rodada, atualizações da classificação
- **Gerenciar sua caixa de entrada** — mensagens da diretoria, staff e eventos diversos
- **Observar jogadores** e **fazer transferências**
- **Acompanhar as finanças** — salários, orçamentos, receitas
- **Completar uma temporada inteira** e avançar para a próxima

### O que NÃO tem neste alpha

Pra alinhar expectativas — aqui estão coisas planejadas mas ainda não implementadas:

- Múltiplas ligas / promoções / rebaixamentos
- Competições de copa
- Negociação de contratos de jogadores
- Base / categorias de formação
- Táticas detalhadas (marcação individual, jogadas ensaiadas, etc.)
- Multiplayer
- Som / música
- Tutorial / introdução guiada além da primeira semana

Vamos chegar lá. Mas agora, precisamos da sua ajuda pra encontrar os bugs e as arestas do que já temos.

---

## Primeiros Passos

### Instalação

1. Baixe o instalador para sua plataforma pelo link que compartilhamos com você
2. Execute o instalador — no Windows, pode aparecer um aviso do SmartScreen já que o app ainda não é assinado digitalmente. Clique em "Mais informações" → "Executar assim mesmo"
3. Abra o **Openfoot Manager**

### Seu primeiro jogo

1. Clique em **New Game** no menu principal
2. Preencha os dados do seu treinador (nome, data de nascimento, nacionalidade)
3. Escolha um banco de dados de mundo (o padrão "Random World" tá ótimo)
4. Escolha um time da lista — veja as estatísticas e escolha um que pareça divertido
5. Você vai cair no **Dashboard** — essa é sua central de comando

### Coisas importantes pra saber

- O botão **Continue** (canto superior direito) avança o tempo em um dia. A setinha do lado permite pular direto pro dia de jogo.
- A **barra lateral** à esquerda é sua navegação — Elenco, Táticas, Treino, Calendário, Finanças, etc.
- Antes de uma partida, você escolhe como quer jogar: **Go to the Field** (controle total), **Watch as Spectator** (assistir a IA jogar), ou **Delegate to Assistant** (resultado instantâneo).
- Durante as partidas, você pode fazer substituições, mudar formação, alterar estilo de jogo e ajustar cobradores de bola parada.
- Depois das partidas, tem palestra de intervalo/pós-jogo e uma coletiva de imprensa opcional.
- O jogo **salva automaticamente** quando você volta ao menu principal. Você também pode salvar manualmente pelas configurações.

---

## O Que Precisamos de Você

### A versão curta

Jogue o jogo. Quebre tudo. Conta pra gente o que aconteceu.

### A versão mais longa

Estamos buscando feedback em três categorias:

#### 1. Bugs e crashes

Essa é a prioridade número um. Se algo quebrar, travar, congelar, ou se comportar de um jeito claramente errado — a gente quer saber.

Exemplos:
- O jogo travou quando tentei iniciar uma partida
- Minhas mudanças de formação não ficam salvas entre sessões
- Um jogador aparece como lesionado mas ainda tá no meu time titular
- A classificação não bate depois da rodada 5
- Recebi uma mensagem sobre um jogo que ainda não aconteceu

#### 2. Problemas de usabilidade

Coisas que não estão quebradas, mas são confusas, irritantes ou difíceis de usar.

Exemplos:
- Não consegui descobrir como mudar meu time titular
- A página de treino é confusa demais, não sei o que nada faz
- O texto é pequeno / grande demais na minha tela
- Não entendo o que "estilo de jogo" realmente afeta
- A caixa de entrada tá cheia de mensagens que não me interessam

#### 3. Balanceamento e sensação de gameplay

Isso é mais subjetivo, mas igualmente valioso. O jogo *parece* certo?

Exemplos:
- Meu time ganha todas as partidas de 5x0, tá fácil demais
- Treino não parece fazer diferença nenhuma
- O moral dos jogadores nunca muda, não importa o que eu faça
- Propostas de transferência são sempre rejeitadas
- Os times da IA parecem fracos / fortes demais

---

## Como Enviar Feedback

A forma mais fácil de enviar feedback é pelos nossos **templates de issue no GitHub**. Basta escolher o certo e preencher — **você pode escrever no seu idioma!**

Se quiser falar com a equipe ou com outros jogadores, você também pode entrar no servidor do Discord: https://discord.gg/2CXaesaukT

- [**Relatório de Bug**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_ptbr.yml) — Algo travou, quebrou ou se comportou incorretamente
- [**Feedback / Sugestão**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_ptbr.yml) — Problemas de usabilidade, balanceamento ou ideias
- [**Relatório de Sessão**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_ptbr.yml) — Um resumo da sua sessão de jogo (super valioso!)

### Arquivos de log

Quando você reportar um bug, **por favor inclua seus arquivos de log**. Eles contêm informações detalhadas sobre o que o jogo estava fazendo quando algo deu errado, e geralmente são a diferença entre a gente conseguir corrigir um bug em 5 minutos ou passar horas tentando reproduzir.

**Onde encontrar seus logs:**

- **Windows:** `C:\Users\<SeuUsuario>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Basta compactar (zipar) toda a pasta `logs` e anexar ao seu relato. Os logs não contêm informações pessoais — apenas eventos do jogo, comandos e rastreamento de erros.

---

## Problemas Conhecidos

Aqui estão coisas que a gente já sabe — você não precisa reportar essas (mas fique à vontade pra comentar se elas afetam sua experiência):

- **Sem som ou música** — o jogo é completamente silencioso por enquanto
- **Escala da interface** pode não ficar perfeita em todos os tamanhos de tela — veja Configurações → Display → UI Scale se algo parecer estranho
- **Parte do texto não está traduzido** — a internacionalização é um trabalho em progresso
- **Saves deste alpha podem não ser compatíveis** com versões futuras. Não se apegue muito aos seus saves!
- **Performance** pode cair levemente ao simular muitos dias seguidos

---

## Últimas Palavras

Esse é um projeto de paixão. Não tem um grande estúdio ou um orçamento gordo por trás. É feito por gente que ama futebol e ama jogos, no tempo livre.

Seu feedback durante esse alpha não é só "legal de ter" — ele está literalmente moldando o que esse jogo vai se tornar. Cada bug que você reportar, cada "isso me confundiu", cada "não seria legal se..." — tudo isso importa.

Então jogue, divirta-se (esperamos!), e não segure o feedback. Não existe pergunta boba e nenhum feedback é pequeno demais.

Obrigado por fazer parte disso. Vamos construir algo incrível juntos.

— Time Openfoot Manager

---

*Versão alpha 0.2.0*
