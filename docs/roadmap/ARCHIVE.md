# Roadmap archive — shipped releases

This is the historical record of what each shipped release of Openfoot Manager set out to do,
preserved verbatim from the roadmap issue
([#11](https://github.com/openfootmanager/openfootmanager/issues/11)) before it was restructured
into an index.

Two things to know when reading it:

- **The `-alpha` / `-beta` suffixes are historical.** Openfoot Manager now uses an odd/even
  convention — odd minors (`0.3.x`) are the unstable development stream, even minors (`0.4.x`) are
  stable releases — and version numbers no longer carry a maturity tag. See
  [CONTRIBUTING.md](../../CONTRIBUTING.md) for the current scheme.
- **Struck-through items were dropped or deferred at the time**, with the reason usually noted
  inline. Anything that was still outstanding when the roadmap was restructured was migrated into a
  release tracking issue or into [VISION.md](../../VISION.md) — it is not recorded here, because
  this file is history rather than a worklist.

For what is being built now, see the [roadmap index](https://github.com/openfootmanager/openfootmanager/issues/11).

---

## 0.1.x-alpha

First release of the game, where the foundations are laid out. In this release, we work on the essential features for the game. A lot of features that are not considered a priority will be left out of this release.

This can be considered an alpha release, so this is the first step into making a stable game. This release should not represent the final product.

This release **must** include:

- [x] Match Simulation
	- [x] Text-based Match Simulation
	- [x] Goals, fouls, penalty shootouts, free kicks, yellow card, send-off, substitutions and the computer-controlled opponent should also be able to make decisions (substitutions, change formation, etc)
- [x] Create a new game session
	- [x] Create your manager and provide manager details
	- [x] Select the team that you want to control
	- [x] Select the players that you want to play the game
	- [x] Change formations
	- [x] See dashboard of next games and practice
	- [x] Player stamina and form
- [x] Save and load game
- [x] Database of players, with random player generation or even using an existing player database
- [x] Database of teams and ~*possibly* of regions~
- [x] Only one championship (the one that the player is playing)
	- [x] Match scheduling initial implementation
- [x] Simple GUI

The following items are optional for this release, which might be implemented if we can fit into the initial foundations:

- [x] Finances
	- [x]  Player transfers
	- [x] Player contracts (wages, bonuses, etc)
- [x] Player progression
- [ ] ~Select region to play the championship~
- [x] Start supporting translations

All the other potential features are not considered priority at this point. The initial release will give us the confidence to implement more features.

**Other things that were implemented in 0.1.x:**

- [x] Scouting (initial implementation)
- [x] Assistant manager AI
- [x] Inbox message system
- [x] Board of directors approval
- [x] Fan approval system
- [x] Youth academy
- [x] Sponsors
	- [x] Get sponsor contracts to improve the club's financial gains
	- [x] Accomplish sponsor-imposed challenges to receive greater bonuses (partial)
- [x] News articles
	- [x] You should get a summary of what happened in the world of soccer/football in the latest weeks, with a system that generates "news articles" that should resemble what you find in a newspaper and sports magazines

## 0.2.x-alpha

If any of the optional items of the previous release were not implemented, this release should now include them.

This release is where things are substantially improved. The game should be more robust at this point, and whe should be able to identify the issues that need our attention. Above that, these items **must** be implemented:

- [x] ~Manager approval~ (Implemented in 0.1.x)
	- ~This is like a termometer that tells you how much approval you have of the club's board of directors. If your termometer gets too low, you can be fired at any time.~
	- [x] Firing may result in the end of the game, or new job opportunities might pop up in your screen (low priority to implement this last option)
- [x] Job opportunities
	- The ability to change teams as soon as opportunities appear
	- Maybe give the player the opportunity to manage a national team
- [x] Youth academy
	- [x] Find new young players for your team
	- [x] Promote academy players
	- [x] Wonderkid prospection
- [x] ~Assistant manager~ (implemented in 0.1.x)
	- [x] An AI-controlled manager that should provide initial suggestions
	- [x] Start implementation of player scouts (low priority)
- [x] Financial improvements
	- [x] Set up marketing campaigns for the team
	- [x] Sell shirts and club merchandise to improve your financial situation
	- [x] Get loans and get more pressure from the board of directors
	- [x] Enhance facilities to improve your team's performance
- [x] Lively world
    - [x] The world should feel more alive, with news from other teams, random events and the user being able to see other matches and results from the round
    - [x] Transfer window and pre-season
    - [x] News articles cover players and narratives
- [x] More star players: game currently lacks players that are very good.
- [x] Tune OVR
- [x] Granular player positions: go beyond GK, DF, MF, FW.
- [x] Player contract renewal and termination system

The user now has to please the demands of the club's board of directors, and try not to get fired from the job. And you can now watch your squad improve as you play the game, with the facility enhancement system.

## 0.3.0-beta

Following this release should come major improvements to the game's core gameplay mechanics. We can move on to the Beta tag, and in this release we can focus on the visual aspects of the game and more desirable features:

- [x] Championship types (Cups, Leagues)
	- [x] Support multiple regions (North America, Central America, South America, Europe, Asia, Ocenia)
	- [x] Create multiple leagues (e.g. Premier League, Serie A, Bundesliga, La Liga, Ligue 1, Bundesliga, Brasileirão Série A, etc.)
	- [x] Create cups (e.g. Copa del Rey, Copa do Brasil, FA Cup, etc.)
	- [x] Regional major competitions (e.g. Champions League, Europa League, Copa Libertadores da América, Sudamericana, Asian Champions Cup, etc.)
	- [x] International club competitions (e.g. Club World Cup, etc.)
	- [x] International nation competitions (e.g. FIFA World Cup, Euro, Copa América, Olympic games)
	- [x] International friendly matches
	- [x] International club friendly cups
- [ ] Customization
	- [ ] ~Support new theme, styling, and customization options~ (deferred)
	- [x] Support player profile pictures
	- [x] Support club logo images
	- [x] Support news headers images
- [ ] ~News overhaul~ (deferred to 0.3.1-beta)
	- [ ] ~Review the current UX for news~
	- [ ] ~Support variety of news: create multiple alternatives for certain stories (title and headers for news generation)~
	- [ ] ~Make news feel way more realistic and opinionated~
	- [ ] ~Rivalry news~
	- [ ] ~Make press conferences matter more~
	- [ ] ~Managers (AI and user) can trash talk on the news~
- [x] Match simulation revamp
	- [x] Add Phase blueprints: more in-depth mechanisms for build up
	- [x] Match testing framework  UI and CLI
	- [x] Player functions on the field
	- [x] Add live match commentary/events during simulation
	- [x] Generate descriptive text for key moments such as goals, fouls, shots, saves, cards, substitutions, injuries, and tactical shifts
	- [x] Support multiple commentary variants for the same type of event to avoid repetition
	- [x] Make commentary context-aware based on players, clubs, rivalries, scoreline, competition importance, match minute, and recent form
	- [x] Add dramatic commentary for high-impact moments, such as late goals, derbies, finals, comebacks, missed penalties, and red cards
	- [x] Support different commentary tones, such as neutral, excited, sarcastic, dramatic, or biased toward a team
	- [x] Improve match immersion by showing a timeline/feed of important match events
	- [x] Allow commentary to influence or connect with the news system after the match
	- [x] Add post-match summaries based on the generated match events
	- [x] Support localization-ready commentary templates for future translations
	- [x] Example commentaries:

		```text
		GOOOAL! Haaland does it again! He finds space inside the box and buries it past the keeper.

		IT'S A FOUL! Marquinhos goes in hard on Mbappé, and PSG win a free kick in a dangerous position.

		THAT WAS CLOSE! Neymar fires from outside the box, the ball whistles just past the left post, and Courtois could only watch.

		WHAT A SAVE! Alisson stretches at full length to deny Vinícius Jr. from close range.

		RED CARD! The referee has seen enough. Sergio Ramos is off after a reckless challenge.

		LATE DRAMA! In the 89th minute, Palmeiras find the equalizer and the stadium erupts.
		```

- [ ] ~Dev QA: Use AI agents to automate QA (avoid regressions, test new features, test UX)~ (Deferred to 0.3.1-beta)
- [x] World Editor: let users create their own worlds easily with a World Editor and a CLI tool.
    - [x] ofm packages: make installable packages in `.ofm` formats
    - [x] Packages are stackable, so users can combine packages to form databases
    - [x] Players
    - [x] Staff
    - [x] Teams
    - [x] Competitions
    - [x] Name pools
    - [x] Support YAML files for definitions
- [ ] UI reworks
    - [x] Redesign Dashboard
    - [x] Redesign Pre-match simulation screen
    - [x] Redesign Post-match simulation screen
    - [x] Redesign tactics board
    - [x] Redesign match
    - [ ] ~Redesign training screen~ (Deferred to 0.3.1-beta)
    - [x] Redesign loan and transfers screen
    - [x] Redesign the Schedule screen
- [x] Support loan bids

In this release, the user must be able to choose the region and team that they want to play. The championships are now expanded, so you can play more than one cup at the same time.
