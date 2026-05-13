# Openfoot Manager — Guide pour les Testeurs Alpha

Salut et bienvenue dans l'alpha d'**Openfoot Manager** ! Avant tout — merci énormément d'être là. Sérieusement. Le fait que tu sois prêt à jouer à un jeu inachevé et à nous aider à l'améliorer, ça compte énormément pour nous.

Ce guide va t'expliquer ce qu'est le jeu, ce qui marche, ce qui ne marche pas (encore), et comment tu peux nous aider au mieux.

---

## C'est quoi Openfoot Manager ?

Openfoot Manager est un **jeu de simulation de gestion de football open-source**. Vois-le comme une lettre d'amour au genre classique du football manager — tu prends les rênes d'un club, tu gères ton effectif, tu définis les tactiques, tu gères les transferts et tu guides ton équipe à travers une saison complète de football compétitif.

C'est une application de bureau construite avec [Tauri](https://tauri.app/) (backend Rust + frontend React), ce qui veut dire qu'elle tourne nativement sur Windows, macOS et Linux sans navigateur ni connexion internet.

### Ce qu'il y a dans cet alpha

Voici ce que tu peux faire maintenant :

- **Créer un entraîneur** avec ton nom, ta date de naissance et ta nationalité
- **Choisir une équipe** dans une ligue générée de 16 équipes
- **Gérer ton effectif** — définir les formations, choisir ton onze de départ, désigner les tireurs de coups de pied arrêtés
- **Entraîner tes joueurs** — choisir l'axe d'entraînement, l'intensité et le planning hebdomadaire
- **Recruter et libérer du staff** — entraîneurs, kinés, recruteurs, adjoints
- **Jouer des matchs** — simulation minute par minute avec contrôles tactiques, ou déléguer à ton adjoint
- **Causeries de mi-temps** et **conférences de presse d'après-match** qui affectent le moral
- **Lire les actualités** — comptes rendus de matchs, résumés de journée, mises à jour du classement
- **Gérer ta boîte de réception** — messages du staff, de la direction et des événements du jeu
- **Observer des joueurs** et **faire des transferts**
- **Suivre les finances** — salaires, budgets, revenus
- **Terminer une saison complète** et passer à la suivante

### Ce qui N'EST PAS dans cet alpha

Pour bien poser les attentes — voici les choses prévues mais pas encore implémentées :

- Plusieurs ligues / promotions / relégations
- Compétitions de coupe
- Négociations de contrats de joueurs
- Centre de formation / académie des jeunes
- Tactiques détaillées (marquage individuel, coups de pied arrêtés travaillés, etc.)
- Multijoueur
- Son / musique
- Tutoriel / introduction guidée au-delà de la première semaine

On y arrivera. Mais pour l'instant, on a besoin de ton aide pour trouver les bugs et les aspérités dans ce qu'on a *déjà*.

---

## Pour Commencer

### Installation

1. Télécharge l'installateur pour ta plateforme depuis le lien qu'on t'a partagé
2. Lance l'installateur — sur Windows tu peux voir un avertissement SmartScreen car l'app n'est pas encore signée. Clique sur "Informations complémentaires" → "Exécuter quand même"
3. Lance **Openfoot Manager**

### Ta première partie

1. Clique sur **New Game** dans le menu principal
2. Remplis les infos de ton entraîneur (nom, date de naissance, nationalité)
3. Choisis une base de données de monde (le "Random World" par défaut convient très bien)
4. Choisis une équipe dans la liste — regarde les stats et prends celle qui te plaît
5. Tu atterris sur le **Dashboard** — c'est ton QG

### Ce qu'il faut savoir

- Le bouton **Continue** (en haut à droite) avance le temps d'un jour. La flèche déroulante à côté te permet de sauter au jour de match.
- La **barre latérale** à gauche est ta navigation — Effectif, Tactiques, Entraînement, Calendrier, Finances, etc.
- Avant un match, tu choisis comment tu veux le jouer : **Go to the Field** (contrôle total), **Watch as Spectator** (regarder l'IA jouer), ou **Delegate to Assistant** (résultat instantané).
- Pendant les matchs, tu peux faire des remplacements, changer de formation, changer de style de jeu et modifier les tireurs de coups de pied arrêtés.
- Après les matchs, il y a une causerie de mi-temps/fin de match et une conférence de presse optionnelle.
- Le jeu **sauvegarde automatiquement** quand tu retournes au menu principal. Tu peux aussi sauvegarder manuellement depuis les paramètres.

---

## Ce Dont On a Besoin

### La version courte

Joue au jeu. Casse tout. Dis-nous ce qui s'est passé.

### La version longue

On cherche du feedback dans trois catégories :

#### 1. Bugs et crashes

C'est la priorité numéro un. Si quelque chose plante, crashe, freeze, ou se comporte d'une manière clairement incorrecte — on veut le savoir.

Exemples :
- Le jeu crashe quand j'essaie de lancer un match
- Mes changements de formation ne sont pas conservés entre les sessions
- Un joueur apparaît comme blessé mais il est toujours dans mon onze
- Le classement ne colle pas après la journée 5
- J'ai reçu un message à propos d'un match qui n'a pas encore eu lieu

#### 2. Problèmes d'ergonomie

Des choses qui ne sont pas cassées, mais qui sont confuses, agaçantes ou difficiles à utiliser.

Exemples :
- Je n'ai pas réussi à trouver comment changer mon onze de départ
- La page d'entraînement est trop chargée, je ne comprends pas ce que font les options
- Le texte est trop petit / trop grand sur mon écran
- Je ne comprends pas ce que le "style de jeu" affecte concrètement
- La boîte de réception est pleine de messages qui ne m'intéressent pas

#### 3. Équilibre et ressenti de jeu

C'est plus subjectif, mais tout aussi précieux. Est-ce que le jeu *semble* juste ?

Exemples :
- Mon équipe gagne tous les matchs 5-0, c'est trop facile
- L'entraînement ne semble faire aucune différence
- Le moral des joueurs ne change jamais quoi que je fasse
- Les offres de transfert sont toujours refusées
- Les équipes IA semblent beaucoup trop faibles / fortes

---

## Comment Envoyer du Feedback

La façon la plus simple d'envoyer du feedback est via nos **templates d'issues GitHub**. Choisis le bon et remplis-le — **tu peux écrire dans ta langue !**

Si tu veux discuter avec l'équipe ou d'autres joueurs, tu peux aussi rejoindre le serveur Discord : https://discord.gg/4ppEDH68

- [**Rapport de Bug**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report_fr.yml) — Quelque chose a crashé, s'est cassé ou s'est mal comporté
- [**Feedback / Suggestion**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback_fr.yml) — Problèmes d'ergonomie, d'équilibre ou idées
- [**Rapport de Session**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report_fr.yml) — Un résumé de ta session de jeu (super précieux !)

### Fichiers de log

Quand tu signales un bug, **merci d'inclure tes fichiers de log**. Ils contiennent des informations détaillées sur ce que faisait le jeu quand quelque chose a mal tourné, et c'est souvent la différence entre corriger un bug en 5 minutes ou passer des heures à essayer de le reproduire.

**Où trouver tes logs :**

- **Windows :** `C:\Users\<TonNomUtilisateur>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS :** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux :** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Compresse (zip) simplement tout le dossier `logs` et joins-le à ton rapport. Les logs ne contiennent aucune information personnelle — juste des événements de jeu, des commandes et des traces d'erreurs.

---

## Problèmes Connus

Voici des choses qu'on connaît déjà — pas besoin de les signaler (mais n'hésite pas à commenter si elles affectent ton expérience) :

- **Pas de son ni de musique** — le jeu est complètement silencieux pour l'instant
- **La mise à l'échelle de l'interface** peut ne pas être parfaite sur toutes les tailles d'écran — vérifie Paramètres → Display → UI Scale si quelque chose semble bizarre
- **Une partie du texte n'est pas traduite** — l'internationalisation est un travail en cours
- **Les sauvegardes de cet alpha peuvent ne pas être compatibles** avec les versions futures. Ne t'attache pas trop à tes parties !
- **Les performances** peuvent baisser légèrement quand tu simules beaucoup de jours d'affilée

---

## Quelques Derniers Mots

C'est un projet passion. Il n'y a pas de gros studio ou de budget conséquent derrière. C'est fait par des gens qui aiment le foot et les jeux vidéo, sur leur temps libre.

Ton feedback pendant cet alpha n'est pas juste un "bonus sympa" — il façonne littéralement ce que ce jeu va devenir. Chaque bug que tu signales, chaque "ça m'a perdu", chaque "ce serait cool si..." — tout ça compte.

Alors joue, amuse-toi (on espère !), et n'hésite pas sur le feedback. Il n'y a pas de question bête et aucun retour n'est trop petit.

Merci de faire partie de l'aventure. Construisons quelque chose de génial ensemble.

— L'équipe Openfoot Manager

---

*Version alpha 0.2.0*
