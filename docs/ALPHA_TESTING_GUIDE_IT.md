# Openfoot Manager — Guida all'Alpha Testing

Ciao e benvenuto nell'alpha di **Openfoot Manager**! Prima di tutto — grazie davvero per essere qui. Sul serio. Il fatto che tu voglia giocare a un gioco ancora incompleto e aiutarci a migliorarlo significa tantissimo per noi.

Questa guida ti accompagnerà attraverso cos'è il gioco, cosa funziona, cosa non funziona ancora, e come puoi aiutarci nel modo più utile possibile.

---

## Cos'è Openfoot Manager?

Openfoot Manager è un **gioco open-source di simulazione manageriale calcistica**. Pensalo come una lettera d'amore al classico genere dei football manager — prendi in mano un club, gestisci la rosa, imposti le tattiche, segui i trasferimenti e guidi la tua squadra attraverso un'intera stagione di campionato.

È sviluppato come app desktop usando [Tauri](https://tauri.app/) (backend Rust + frontend React), quindi gira in modo nativo su Windows, macOS e Linux senza bisogno di browser o connessione internet.

### Cosa c'è in questa alpha

Ecco cosa puoi fare già adesso:

- **Creare un allenatore** con nome, data di nascita e nazionalità
- **Scegliere una squadra** da un campionato generato con 16 team
- **Gestire la rosa** — impostare la formazione, scegliere l'XI titolare, assegnare i battitori sui piazzati
- **Allenare i giocatori** — scegliere focus, intensità e programma settimanale
- **Assumere e licenziare staff** — allenatori, fisioterapisti, osservatori, vice allenatori
- **Giocare le partite** — simulazione live minuto per minuto con controlli tattici, oppure delegare al tuo assistente
- **Discorsi all'intervallo** e **conferenze stampa post-partita** che influenzano il morale
- **Leggere le notizie** — report partita, riepiloghi di giornata, aggiornamenti di classifica
- **Gestire la posta** — messaggi da staff, dirigenza e vari eventi di gioco
- **Osservare giocatori** e **fare trasferimenti**
- **Tenere sotto controllo le finanze** — stipendi, budget, entrate
- **Completare un'intera stagione** e passare a quella successiva

### Cosa NON c'è ancora in questa alpha

Per impostare bene le aspettative — ecco alcune cose previste ma non ancora implementate:

- Campionati multipli / promozioni / retrocessioni
- Coppe
- Trattative contrattuali con i giocatori
- Settore giovanile
- Tattiche di partita più dettagliate (marcature, schemi sui piazzati, ecc.)
- Multiplayer
- Audio / musica
- Tutorial / onboarding guidato oltre la prima settimana

Ci arriveremo. Ma in questo momento abbiamo bisogno del tuo aiuto per trovare bug e spigoli nelle funzionalità che *già* esistono.

---

## Per Iniziare

### Installazione

1. Scarica l'installer per la tua piattaforma dal link che ti abbiamo inviato
2. Esegui l'installer — su Windows potresti vedere un avviso SmartScreen perché l'app non è ancora firmata. Clicca su "Ulteriori informazioni" → "Esegui comunque"
3. Avvia **Openfoot Manager**

### La tua prima partita

1. Clicca su **Nuova partita** nel menu principale
2. Compila i dettagli del tuo allenatore (nome, data di nascita, nazionalità)
3. Scegli un database del mondo (quello predefinito, "Mondo casuale", va benissimo)
4. Scegli una squadra dalla lista — guarda le statistiche e prendine una che ti sembri divertente
5. Atterrerai sulla **Dashboard** — questa è la tua base operativa

### Cose importanti da sapere

- Il pulsante **Continua** (in alto a destra) fa avanzare il tempo di un giorno. La freccia del menu accanto ti permette di saltare direttamente al prossimo giorno partita.
- La **barra laterale** a sinistra è la tua navigazione — Rosa, Tattiche, Allenamento, Calendario, Finanze, ecc.
- Prima di una partita, ti verrà chiesto come vuoi affrontarla: **Vai in campo** (controllo completo), **Guarda da spettatore** (osservi l'IA giocare), oppure **Delega all'assistente** (risultato immediato).
- Durante le partite puoi fare sostituzioni, cambiare formazione, modificare lo stile di gioco e regolare i battitori sui piazzati.
- Dopo le partite ci sono il discorso di metà/fine gara e una conferenza stampa opzionale.
- Il gioco **si salva automaticamente** quando esci al menu principale. Puoi anche salvare manualmente dalle impostazioni.

---

## Cosa Ci Serve Da Te

### La versione breve

Gioca. Rompi tutto. Raccontaci cosa è successo.

### La versione lunga

Cerchiamo feedback in tre categorie:

#### 1. Bug e crash

Questa è la priorità assoluta. Se qualcosa si rompe, va in crash, si blocca o si comporta in modo chiaramente sbagliato — vogliamo saperlo.

Esempi:

- Il gioco va in crash quando provo ad avviare una partita
- I cambi di formazione non restano salvati tra una sessione e l'altra
- Un giocatore risulta infortunato ma compare comunque nel mio XI titolare
- La classifica non torna dopo la giornata 5
- Ho ricevuto un messaggio su una partita che non è ancora stata giocata

#### 2. Problemi di usabilità

Cose che non sono rotte, ma risultano confuse, fastidiose o difficili da usare.

Esempi:

- Non sono riuscito a capire come cambiare il mio XI titolare
- La pagina dell'allenamento è troppo caotica, non capisco a cosa serva ogni cosa
- Il testo è troppo piccolo / troppo grande sul mio schermo
- Non capisco cosa influenzi davvero lo "stile di gioco"
- La posta è piena di messaggi che non mi interessano

#### 3. Bilanciamento e feeling di gioco

Questa parte è più soggettiva, ma è altrettanto preziosa. Il gioco *dà* la sensazione giusta?

Esempi:

- La mia squadra vince tutte le partite 5-0, è troppo facile
- L'allenamento non sembra fare nessuna differenza
- Il morale dei giocatori non cambia mai, qualunque cosa faccia
- Le offerte di trasferimento vengono sempre rifiutate
- Le squadre controllate dall'IA sembrano troppo deboli / troppo forti

---

## Come Inviare Feedback

Il modo più semplice per inviare feedback è tramite i nostri **template di issue su GitHub**. Basta scegliere quello giusto e compilarlo — **puoi scrivere in italiano!**

Se vuoi parlare con il team o con altri giocatori, puoi anche unirti al server Discord: https://discord.gg/2CXaesaukT

- [**Segnalazione Bug (Italiano)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_it.yml) — Qualcosa è andato in crash, si è rotto o si è comportato in modo scorretto
- [**Feedback / Suggerimento (Italiano)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_it.yml) — Problemi di usabilità, bilanciamento o idee
- [**Resoconto di Sessione (Italiano)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_it.yml) — Un riassunto della tua sessione di gioco (super prezioso!)

### File di log

Quando segnali un bug, **per favore allega i tuoi file di log**. Contengono informazioni dettagliate su cosa stava facendo il gioco quando qualcosa è andato storto, e spesso fanno la differenza tra correggere un bug in 5 minuti o passare ore a cercare di riprodurlo.

**Dove trovare i log:**

- **Windows:** `C:\Users\<IlTuoNomeUtente>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Ti basta comprimere in ZIP l'intera cartella `logs` e allegarla alla segnalazione. I log non contengono informazioni personali — solo eventi di gioco, comandi e tracce di errore.

---

## Problemi Noti

Ecco alcune cose che già conosciamo — non serve segnalarle (ma sentiti libero di commentarle se influiscono sulla tua esperienza):

- **Niente audio o musica** — per ora il gioco è completamente silenzioso
- **La scala dell'interfaccia** potrebbe non essere perfetta su tutti gli schermi — controlla Impostazioni → Display → UI Scale se qualcosa appare strano
- **Parte del testo non è ancora tradotta** — l'internazionalizzazione è ancora in corso
- **I salvataggi di questa alpha potrebbero non essere compatibili** con versioni future. Non affezionarti troppo alle tue partite!
- **Le prestazioni** potrebbero calare leggermente simulando molti giorni di fila

---

## Un Ultimo Messaggio

Questo è un progetto di passione. Non c'è dietro un grande studio né un grosso budget. È costruito da persone che amano il calcio e i videogiochi, nel loro tempo libero.

Il tuo feedback durante questa alpha non è solo "utile da avere" — sta letteralmente dando forma a ciò che questo gioco diventerà. Ogni bug che segnali, ogni commento del tipo "questa cosa mi ha confuso", ogni idea del tipo "non sarebbe bello se..." — conta davvero.

Quindi gioca, divertiti (speriamo!), e non trattenerti con il feedback. Non esistono domande stupide e nessun feedback è troppo piccolo.

Grazie per far parte di tutto questo. Costruiamo insieme qualcosa di grande.

— Il team di Openfoot Manager

---

Versione alpha 0.2.0
