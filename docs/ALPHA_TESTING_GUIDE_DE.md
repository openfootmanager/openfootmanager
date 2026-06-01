# Openfoot Manager — Alpha-Tester-Leitfaden

Hey, willkommen beim Alpha von **Openfoot Manager**! Erstmal — vielen Dank, dass du dabei bist. Ehrlich. Die Tatsache, dass du bereit bist, ein unfertiges Spiel zu spielen und uns zu helfen, es besser zu machen, bedeutet uns unglaublich viel.

Dieser Leitfaden erklärt dir, was das Spiel ist, was funktioniert, was (noch) nicht, und wie du uns am besten helfen kannst.

---

## Was ist Openfoot Manager?

Openfoot Manager ist ein **Open-Source-Fußballmanager-Simulationsspiel**. Stell es dir als Liebesbrief an das klassische Football-Manager-Genre vor — du übernimmst einen Verein, verwaltest deinen Kader, legst Taktiken fest, kümmerst dich um Transfers und führst dein Team durch eine komplette Saison im Ligafußball.

Es ist als Desktop-App mit [Tauri](https://tauri.app/) gebaut (Rust-Backend + React-Frontend), was bedeutet, dass es nativ auf Windows, macOS und Linux läuft — ohne Browser oder Internetverbindung.

### Was in diesem Alpha enthalten ist

Das kannst du jetzt schon machen:

- **Einen Trainer erstellen** mit deinem Namen, Geburtsdatum und Nationalität
- **Ein Team wählen** aus einer generierten Liga mit 16 Mannschaften
- **Deinen Kader verwalten** — Formationen festlegen, Startelf wählen, Standardschützen bestimmen
- **Deine Spieler trainieren** — Trainingsschwerpunkt, Intensität und Wochenplan wählen
- **Staff einstellen und entlassen** — Trainer, Physiotherapeuten, Scouts, Co-Trainer
- **Spiele austragen** — Minute-für-Minute-Simulation mit taktischen Kontrollen, oder an deinen Assistenten delegieren
- **Halbzeit-Ansprachen** und **Pressekonferenzen nach dem Spiel**, die die Moral beeinflussen
- **Nachrichten lesen** — Spielberichte, Spieltagszusammenfassungen, Tabellenaktualisierungen
- **Dein Postfach verwalten** — Nachrichten von Staff, Vorstand und verschiedenen Spielereignissen
- **Spieler beobachten** und **Transfers tätigen**
- **Finanzen verfolgen** — Gehälter, Budgets, Einnahmen
- **Eine komplette Saison abschließen** und zur nächsten wechseln

### Was NICHT in diesem Alpha enthalten ist

Um die Erwartungen richtig zu setzen — hier sind Dinge, die geplant, aber noch nicht umgesetzt sind:

- Mehrere Ligen / Auf- und Abstiege
- Pokalwettbewerbe
- Vertragsverhandlungen mit Spielern
- Jugendakademie
- Detaillierte Taktiken (Manndeckung, Standardsituationen, etc.)
- Mehrspieler
- Sound / Musik
- Tutorial / geführte Einführung über die erste Woche hinaus

Wir kommen dahin. Aber jetzt brauchen wir deine Hilfe, um die Bugs und Ecken und Kanten in dem zu finden, was wir *schon* haben.

---

## Erste Schritte

### Installation

1. Lade den Installer für deine Plattform über den Link herunter, den wir dir geschickt haben
2. Führe den Installer aus — unter Windows kann eine SmartScreen-Warnung erscheinen, da die App noch nicht signiert ist. Klicke auf "Weitere Informationen" → "Trotzdem ausführen"
3. Starte **Openfoot Manager**

### Dein erstes Spiel

1. Klicke auf **New Game** im Hauptmenü
2. Gib deine Trainerdaten ein (Name, Geburtsdatum, Nationalität)
3. Wähle eine Weltdatenbank (die Standard-Option "Random World" ist völlig in Ordnung)
4. Wähle ein Team aus der Liste — schau dir die Statistiken an und nimm eines, das dir Spaß macht
5. Du landest auf dem **Dashboard** — das ist deine Kommandozentrale

### Wichtige Hinweise

- Der **Continue**-Button (oben rechts) lässt die Zeit um einen Tag voranschreiten. Der Dropdown-Pfeil daneben lässt dich zum nächsten Spieltag springen.
- Die **Seitenleiste** links ist deine Navigation — Kader, Taktik, Training, Spielplan, Finanzen, etc.
- Vor einem Spiel wirst du gefragt, wie du es erleben möchtest: **Go to the Field** (volle Kontrolle), **Watch as Spectator** (der KI zuschauen), oder **Delegate to Assistant** (sofortiges Ergebnis).
- Während der Spiele kannst du Auswechslungen vornehmen, die Formation ändern, den Spielstil wechseln und die Standardschützen anpassen.
- Nach den Spielen gibt es eine Halbzeit-/Schlusspfiff-Ansprache und eine optionale Pressekonferenz.
- Das Spiel **speichert automatisch**, wenn du zum Hauptmenü zurückkehrst. Du kannst auch manuell über die Einstellungen speichern.

---

## Was Wir von Dir Brauchen

### Die Kurzversion

Spiel das Spiel. Mach alles kaputt. Erzähl uns, was passiert ist.

### Die ausführliche Version

Wir suchen Feedback in drei Kategorien:

#### 1. Bugs und Abstürze

Das hat oberste Priorität. Wenn etwas kaputt geht, abstürzt, einfriert oder sich offensichtlich falsch verhält — wir wollen es wissen.

Beispiele:
- Das Spiel stürzt ab, wenn ich versuche, ein Spiel zu starten
- Meine Formationsänderungen werden nicht zwischen Sitzungen gespeichert
- Ein Spieler wird als verletzt angezeigt, ist aber trotzdem in meiner Startelf
- Die Tabelle stimmt nach dem 5. Spieltag nicht
- Ich habe eine Nachricht über ein Spiel bekommen, das noch nicht stattgefunden hat

#### 2. Bedienungsprobleme

Dinge, die nicht kaputt sind, aber verwirrend, nervig oder schwer zu benutzen.

Beispiele:
- Ich konnte nicht herausfinden, wie ich meine Startelf ändere
- Die Trainingsseite ist überwältigend, ich weiß nicht, was was bewirkt
- Der Text ist zu klein / zu groß auf meinem Bildschirm
- Ich verstehe nicht, was "Spielstil" tatsächlich beeinflusst
- Das Postfach ist voll mit Nachrichten, die mich nicht interessieren

#### 3. Balance und Spielgefühl

Das ist subjektiver, aber genauso wertvoll. *Fühlt* sich das Spiel richtig an?

Beispiele:
- Mein Team gewinnt jedes Spiel 5:0, es ist zu einfach
- Training scheint keinen Unterschied zu machen
- Die Spielermoral ändert sich nie, egal was ich tue
- Transferangebote werden immer abgelehnt
- Die KI-Teams scheinen viel zu schwach / zu stark

---

## Wie Du Feedback Einreichen Kannst

Am einfachsten geht das über unsere **GitHub Issue-Vorlagen**. Wähle einfach die richtige aus und fülle sie aus — **du kannst in deiner Sprache schreiben!**

Wenn du mit dem Team oder anderen Spielern sprechen möchtest, kannst du auch unserem Discord-Server beitreten: https://discord.gg/2CXaesaukT

- [**Bug-Report**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_de.yml) — Etwas ist abgestürzt, kaputt oder hat sich falsch verhalten
- [**Feedback / Vorschlag**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_de.yml) — Bedienungsprobleme, Balance-Bedenken oder Ideen
- [**Sitzungsbericht**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_de.yml) — Eine Zusammenfassung deiner Spielsitzung (super wertvoll!)

### Log-Dateien

Wenn du einen Bug meldest, **bitte leg deine Log-Dateien bei**. Sie enthalten detaillierte Informationen darüber, was das Spiel gerade tat, als etwas schief ging, und sie sind oft der Unterschied zwischen einem Bug-Fix in 5 Minuten und stundenlangem Reproduzieren.

**Wo du deine Logs findest:**

- **Windows:** `C:\Users\<DeinBenutzername>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Packe einfach den gesamten `logs`-Ordner als ZIP und häng ihn an deinen Bericht an. Die Logs enthalten keine persönlichen Daten — nur Spielereignisse, Befehle und Fehler-Traces.

---

## Bekannte Probleme

Hier sind Dinge, die wir bereits kennen — du musst sie nicht melden (aber kommentiere gerne, wenn sie dein Erlebnis beeinträchtigen):

- **Kein Sound oder Musik** — das Spiel ist aktuell komplett stumm
- **UI-Skalierung** ist möglicherweise nicht auf allen Bildschirmgrößen perfekt — überprüfe Einstellungen → Display → UI Scale, falls etwas komisch aussieht
- **Ein Teil des Textes ist nicht übersetzt** — Internationalisierung ist in Arbeit
- **Speicherdateien aus diesem Alpha sind möglicherweise nicht kompatibel** mit zukünftigen Versionen. Häng nicht zu sehr an deinen Spielständen!
- **Die Performance** kann leicht abnehmen, wenn viele Tage hintereinander simuliert werden

---

## Ein Paar Letzte Worte

Das hier ist ein Leidenschaftsprojekt. Kein großes Studio, kein fettes Budget. Es wird von Leuten gebaut, die Fußball und Spiele lieben, in ihrer Freizeit.

Dein Feedback während dieses Alphas ist nicht nur "nett zu haben" — es formt buchstäblich, was dieses Spiel wird. Jeder Bug, den du meldest, jedes "das hat mich verwirrt", jedes "wäre es nicht cool, wenn..." — alles zählt.

Also spiel, hab Spaß (hoffen wir!), und halt dich nicht zurück mit dem Feedback. Es gibt keine dummen Fragen und kein Feedback ist zu klein.

Danke, dass du dabei bist. Lass uns zusammen etwas Großartiges bauen.

— Das Openfoot Manager Team

---

*Alpha-Version 0.2.0*
