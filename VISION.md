# The Openfoot Manager Vision

I’ve been quietly dreaming about and building this game for over a decade. What started as myself's 14-year-old mockup called "Soccer City" during the 2010 World Cup has somehow grown into a living, breathing project.

But as more contributors jump in and the community grows, I realize this is no longer *my* project anymore. This has become a real community effort.

With so many ideas flying around, so many new people joining the project, we need a shared understanding of what Openfoot Manager **actually is**, what **it is not**, and **where it is ultimately going**.

Keep in mind that this isn't a rigid feature list (our [Roadmap](https://github.com/openfootmanager/openfootmanager/issues/11) takes care of that). This document is a general guidance.

If you are contributing and unsure whether an idea belongs in the game, this document should give you the answer.

---

## What Openfoot Manager Is

**Openfoot Manager** is a free, open-source football management simulation. You are the manager: you pick the squad, set the tactics, negotiate the transfers, answer to the board, and live with the results. We've seen this formula plenty of times before.

The difference here is that, this game is actually **yours**. It runs **locally** on your machine. There is no account to create, no server to connect to, and, honestly, I don't care about getting your data through telemetry.

Your saves and your data belong to **you**, in files **you** can read, on a disk **you own**. If this project disappeared off the internet tomorrow (I'm not saying that it will), your game would keep working exactly as it did yesterday.

It is licensed under the [GPLv3](LICENSE.md), and it always will be. This means that: if you want to create a version of the game that needs a server, needs an account, and that tracks people data, you can actually do that, but you need to make it open source as well. That's it.

---

## We Are Not a Football Manager Clone

A lot of people think that because the project is named **Openfoot Manager**, that means that it is an open-source Football Manager clone. That couldn't be farther from the truth.

Openfoot Manager is heavily inspired by the genre. Games like *Football Manager*, *Championship Manager*, *Brasfoot*, and *Bygfoot* are the reason this project exists.

But **we are not** trying to clone any of them. Just because feature X or Y exists in FM, or any other manager game, does not mean that we have to replicate it exactly as-is.

Our design philosophy is way different:

* **The world is authored data, not a licensed database.** We don't ship licensed real-world content. Instead, we ship an in-game editor and a package format (`.ofm`). The world (the clubs, players, history, and name pools) is something **you** build, share, and stack. Commercial games sell you this year's database; we give you all the tools and get out of the way.
* **Depth is opt-in.** Commercial games often assume you want maximum complexity, and if you don't, the game feels like a second job. We want to give you a choice: Do you want full realism? Or just a fun and quick experience?
* **Make your alternate reality come true.** Reconstructing the 1970s Santos squad, running a fictional world with a century of custom history, or dragging a fourth-division club to a continental title matters just as much here as simulating the current Premier League season. Your game, your rules.

Whenever we deliberately diverge from how commercial games do things, we will say so, rather than treating those games as a specification we are "behind" on.

---

## What We Are Not Building

A lot of people come to this project hoping for a few things that are not really in scope for this project. But I need to make it clear from the start, so you don't come with wrong expectations:

* **No multiplayer or online mode.** This is strictly a single-player, offline sandbox. OFM is designed around your career, your world, at your pace. This experience would gain very little from networking, but would cost us everything in complexity, infrastructure, security, and data protection. For this same reason, there are no plans for a web/browser version. It's not completely off the books. I might decide to support it one day, but it's simply not worth it right now.
* **No licensed real-world data.** We will not ship copyrighted club names, badges, or player databases. We build the engine; the community authors the worlds.
* **No accounts, no servers, no telemetry.** I really don't care about your data. If we ever implement telemetry one day, it will be to collect data about the game itself, NOT YOURS. Some software actually use telemetry to catch bugs and potential failures, and it is a valid usage, but if we ever do this, it will be opt-in, with your consent, and totally transparent. We don't want anything else other than game data.
* **No monetization of any kind.** No lootboxes, no pay-to-win mechanics, no waiting timers, no cosmetic stores, and no "supporter editions" with gated features. Of course, it costs money to make the game, so you can consider donating directly to the developers, but only if you want to (and if you can). That's not a hard requirement, you don't need to pay us, but any support is appreciated.

---

## Our Core Pillars

These are our commitments. Each one obliges us to act a certain way, and each one is allowed to slow a release down if we aren't meeting the standard.

### 1. Free and Open Source, Permanently

GPLv3. No path is left open to change that. Every contribution stays available to the community that built it. This isn't a demo for a commercial product, and there is no premium upsell waiting at the end.

### 2. A Game for Everyone (Accessibility & i18n)

A few weeks ago, I found out that blind and visually impaired players were using screen readers to play OFM, and that humbled me deeply. **Keyboard paths and screen-reader support are requirements, not polish applied at the end.** A management game is made of text, tables, and menus: there is no excuse for it to be unusable with a screen reader. Accessibility bugs are treated as critical bugs, not feature requests.

Similarly, football is a global game. Every user-facing string must be translated into our 11 supported locales. If a PR adds an untranslated string, it doesn't merge. Translation is a core requirement, not an afterthought.

### 3. Moddable and Data-Driven

Every football fact hardcoded into the Rust engine is technical debt we intend to pay down, because every hardcoded rule is a world somebody cannot build. The `.ofm` package system, the world editor, and the CLI exist to make modding a first-class feature, not a hack.

### 4. Approachable Depth

The newcomer who wants to casually play a season this evening, and the veteran who wants to micromanage every training schedule, should both feel at home. When those two playstyles conflict, the answer is usually a toggle in the settings, not a messy compromise that fails both.

---

## What 1.0 Means

1.0 is a **quality bar, not a feature bar**. Hitting 1.0 simply means:

1. You can play a multi-season career, in any region, from start to end, without hitting a wall.
2. You can author the world you play in and easily share it.
3. The game is fully translated and completely usable with a keyboard and a screen reader.
4. Your saves survive upgrading the game.

No single feature gates 1.0. A game that can do those four things flawlessly is finished enough to be called 1.0, regardless of what else it does or doesn't have yet.

---

## Where We Are Going (The Ambitions)

These are our long-term goals. They are highly desired, but deliberately **not scheduled**. Each is massive, which is why they don't carry a version number here. When one becomes our immediate focus, it gets a release issue on the roadmap.

* **Women's football:** Full support for women's leagues and tournaments.
* **Reconstruct any era:** Play Pelé's Santos, Guardiola's 2010s Barcelona, or the Galácticos of the 2000s. This is the ultimate payoff of our `.ofm` system. Modding isn't a side feature; it's the game's central promise.
* **History and news you control:** A data-driven narrative engine so a custom world's past, rivalries, and press coverage are fully authorable.
* **National-team management:** Start your career internationally, or earn the prestigious call-up after building your reputation at a club.
* **Challenge modes:** Authored scenarios with specific goals—take a club from the 3rd division to the 1st, or rescue a squad in the middle of a relegation crisis.
* **Match replays:** Watch replays of your past historic matches.
* **Dynamic rivalries:** Grudges that emerge organically from what *actually* happens in your save, rather than from a static database table.
* **Player personalities and synergy:** Players will have distinct personalities that demand tailored managerial approaches, influencing locker room morale and on-pitch synergy.

---

## How We Ship

Openfoot Manager uses an odd/even release convention:

* **Odd** minor versions (`0.3.x`, `0.5.x`) are the unstable development stream published as nightlies.
* **Even** minor versions (`0.4.x`, `0.6.x`) are stable releases.

You can always tell which you are running just by looking at the version number. The mechanics behind this (branches, channels, and our stable release checklist) live in [CONTRIBUTING.md](CONTRIBUTING.md).

## How an Idea Becomes Work

1. **Discuss it:** Drop it in [Discussions](https://github.com/openfootmanager/openfootmanager/discussions) (the `Ideas` category) or on [Discord](https://discord.gg/2CXaesaukT). Early, rough, and unfinished thoughts are more than welcome.
2. **Open an issue:** Once the idea's shape is clear enough to describe.
3. **It hits the Roadmap:** If it's something we are committing to, it gets attached to a release on the [Roadmap](https://github.com/openfootmanager/openfootmanager/issues/11) and a milestone.

Ideas are never rejected for being too ambitious. They are only turned down if they conflict with the core pillars or scope boundaries above.

## How to Help

Writing code is just one way to help, and honestly, it's not always the most needed one. Translating strings, reporting bugs, sharing your `.ofm` world packages, writing documentation, or simply telling people the game exists all move this project forward.

Check out [CONTRIBUTING.md](CONTRIBUTING.md) for the specifics, and come say hello on [Discord](https://discord.gg/2CXaesaukT). We’re glad you’re here.