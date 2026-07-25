# VISION

This document explains what Openfoot Manager is for, what it refuses to become, and where it is
ultimately going. It is deliberately not a feature list — the [Roadmap](https://github.com/openfootmanager/openfootmanager/issues/11)
tracks features, and it changes every release. What is written here should change very rarely.

If you are contributing and unsure whether an idea belongs in the game, this is the document that
should answer you.

---

## What Openfoot Manager is

Openfoot Manager is a free and open source football management simulation. You are the manager: you
pick the squad, set the tactics, negotiate the transfers, answer to the board, and live with the
results.

It runs on your machine. There is no account to create, no server to connect to, no telemetry
phoning home. Your saves and your data are yours, in files you can read, on a disk you own. If this
project disappeared tomorrow, your game would keep working.

It is licensed under the [GPLv3](LICENSE.md), and it always will be.

---

## This is not a Football Manager clone

This is the most common misreading of the project, so it is worth stating plainly and early.

Openfoot Manager is inspired by the genre — Football Manager, Championship Manager, and the
much-missed Bygfoot are all part of why this exists. But it is not an attempt to reproduce any of
them, and "an open source FM" is not the goal. The design centre is genuinely somewhere else:

- **The world is authored data, not a licensed database.** We ship no licensed real-world content.
  Instead we ship an editor and a package format (`.ofm`) so that the world — clubs, players,
  competitions, name pools, history — is something you build, share, and stack. That is a different
  product, not a cheaper version of the same one. Commercial manager games sell you this year's
  database; we hand you the tools and get out of the way.
- **Depth is opt-in.** The commercial games assume you want maximum complexity, and if you don't,
  the genre has nothing for you. We think that assumption costs the genre most of its potential
  players. Complexity should be a dial you set, not a toll you pay.
- **It is a platform for what-if football.** Reconstructing 1970 Santos, running a fictional world
  with its own century of history, or dragging a fourth-division club to a continental title matter
  as much here as simulating the current Premier League season. Arguably more.
- **We are small, and we choose differently because of it.** We can't out-simulate a studio with a
  hundred people and a licensing budget. We can be the game that is yours — moddable, offline,
  translated, accessible, and free — which is something they structurally cannot be.

Where we deliberately diverge, we will say so rather than treating the commercial games as a
specification we are behind on.

---

## What we are not building

These are scope decisions, not technical limitations, and not "not yet". They are listed so nobody
has to guess, and so a contributor can tell before writing code whether a PR will be accepted.

- **No multiplayer or online mode.** Not planned and not in scope. A great deal of this genre has
  always been single-player, and the experience Openfoot Manager is designed around — your career,
  your world, at your pace — would gain little from being networked relative to what it would cost
  in complexity, infrastructure, and the offline guarantee above. This also answers the related
  question of a web or browser version of the game: the game is a local desktop application by
  design.
- **No licensed real-world data.** We will not ship copyrighted club names, kits, badges, or player
  databases. The community authors and shares worlds instead.
- **No accounts, no servers, no telemetry, no online requirement.**
- **No monetization of any kind.** No lootboxes, no card packs, no pay-to-win, no waiting timers,
  no cosmetics store, no "supporter edition" with extra features. Not now and not later.

---

## Core pillars

These are commitments rather than features. Each one obliges us to do something, and each one is
allowed to slow a release down.

### Free and open source, permanently

GPLv3, and no path is left open to change that. Every contribution stays available to the community
that built it. This is not a demo of a commercial product and there is no upsell waiting.

### Community-driven

The roadmap is public, design happens in the open — [Discussions](https://github.com/openfootmanager/openfootmanager/discussions)
and [Discord](https://discord.gg/2CXaesaukT) — and disagreement in the open is a feature. Most
importantly, the world itself is community-authored: the default database is meant to be built by
the people who play the game, not handed down by whoever maintains the engine.

### Offline-first, and yours

No accounts, no servers, no telemetry. It runs on modest hardware, because a management game should
not need a graphics card. This pillar is precisely what the no-multiplayer decision protects: single
player and offline is the design, not a stage we are passing through.

### Accessibility

Keyboard paths and screen-reader support are requirements, not polish applied at the end. A
management game is text, tables, and menus — there is no excuse for it to be unusable with a screen
reader, and accessibility bugs are treated as bugs rather than enhancement requests.

### Internationalization

Every user-facing string is translated into all 11 supported locales, and coverage is enforced in
CI (`src/i18n/localeCoverage.test.ts`) — a change that adds an untranslated string does not merge.
Football is not an English-speaking sport, and translation here is a merge gate, not an afterthought
someone gets to later.

### Moddable and data-driven

The world is data you can author and share. Every football fact hardcoded into the engine is debt we
intend to pay down, because each one is a world somebody cannot build. The `.ofm` package system,
the world editor, and the CLI exist to make authoring a first-class activity rather than a hack.

### Approachable depth

Complexity is the player's choice. Neither the newcomer who wants to play a season this evening nor
the veteran who wants to configure everything should feel the game was built for somebody else. When
those two goals conflict, the answer is usually a setting, not a compromise that fails both.

---

## What 1.0 means

1.0 is a **quality bar, not a feature bar**. It means:

- You can play a multi-season career, in any region, from start to end, without hitting a wall.
- You can author the world you play in, and share it.
- The game is fully translated, and usable with a keyboard and a screen reader.
- Your saves survive upgrading the game.

No single feature gates 1.0. A game that can do the four things above is finished enough to be
called 1.0, whatever else it does or doesn't have.

---

## Where we are going

These are the ambitions — wanted, but deliberately **not scheduled**. Each is large enough to be a
release theme in its own right, which is why none of them carries a version number here. When one
becomes the next thing we build, it gets a release issue on the roadmap.

- **Reconstruct any era.** Play Pelé's Santos, Guardiola's Barcelona of the 2010s, the Galácticos of
  the 2000s. This is the payoff of the package and data-driven systems, and the reason those systems
  exist at all — not modding as a side feature, but the game's central promise.
- **A community-built default database with a lore of its own.** Rather than a thin randomly
  generated world, a coherent fictional football universe with its own history, its own dynasties and
  its own legends — authored, argued over, and owned by the community.
- **History and news you control.** Data-driven narrative generation, so a world's past and the press
  that covers it are authorable rather than hardcoded. A world you build should be able to carry its
  own century of results and its own rivalries.
- **National-team management.** Choose it at the start of a career, or be invited into it mid-career
  once your reputation earns the call.
- **Choose your complexity.** An arcade-leaning game for an evening, a balanced default, or a full
  FM-depth simulation for people who want every dial — the same game, metered to the commitment you
  actually want to make.
- **Women's football.**
- **Challenge modes.** Authored scenarios with a stated goal: take a club from the third division to
  the first, rescue a season already going wrong, inherit a squad in crisis.
- **Watch replays of past matches.**
- **Dynamic rivalries** that emerge from what actually happened in your save, rather than from a
  table someone wrote in advance.

This list is the single source of truth for the project's unscheduled ambitions. The roadmap links
here rather than restating it.

---

## How we ship

Openfoot Manager uses an odd/even release convention: **odd** minor versions (`0.3.x`, `0.5.x`) are
the unstable development stream published as nightlies, and **even** minor versions (`0.4.x`,
`0.6.x`) are stable releases. You can always tell which you are running from the version number
alone.

The mechanics — branches, version files, channels, and the checklist a stable release has to pass —
are in [CONTRIBUTING.md](CONTRIBUTING.md).

## How an idea becomes work

1. **Discuss it.** [Discussions](https://github.com/openfootmanager/openfootmanager/discussions)
   (the `Ideas` category) or Discord. Early, rough, and unfinished is fine.
2. **Open an issue** once the shape is clear enough to describe.
3. **It gets attached to a release** on the [Roadmap](https://github.com/openfootmanager/openfootmanager/issues/11)
   and a milestone, if it is something we are committing to.

Ideas are not rejected for being ambitious. They are turned down for conflicting with the pillars or
the scope decisions above — and if that happens, this document should have told you in advance.

## How to help

Code is one way and not the most needed one. Translations, bug reports, session reports, world
packages, documentation, and simply telling people the game exists all move it forward. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the specifics, and come say hello on
[Discord](https://discord.gg/2CXaesaukT).
