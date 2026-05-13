# Openfoot Manager — Alpha Testing Guide

Hey there, and welcome to the **Openfoot Manager** alpha! First off — thank you so much for being here. Seriously. The fact that you're willing to play an unfinished game and help us make it better means the world to us.

This guide will walk you through what the game is, what works, what doesn't (yet), and how you can help us the most.

---

## What is Openfoot Manager?

Openfoot Manager is an **open-source football management simulation game**. Think of it as a love letter to the classic football manager genre — you take charge of a club, manage your squad, set tactics, handle transfers, and guide your team through a full season of competitive league football.

It's built as a desktop app using [Tauri](https://tauri.app/) (Rust backend + React frontend), which means it runs natively on Windows, macOS, and Linux with no browser or internet connection needed.

### What's in this alpha

Here's what you can actually do right now:

- **Create a manager** with your name, date of birth, and nationality
- **Pick a team** from a 16-team generated league
- **Manage your squad** — set formations, pick your starting XI, assign set piece takers
- **Train your players** — choose training focus, intensity, and weekly schedule
- **Hire and release staff** — coaches, physios, scouts, assistant managers
- **Play matches** — live minute-by-minute simulation with tactical controls, or delegate to your assistant
- **Half-time team talks** and **post-match press conferences** that affect morale
- **Read the news** — match reports, league roundups, standings updates
- **Handle your inbox** — messages from staff, board, and various game events
- **Scout players** and **make transfers**
- **Track finances** — wages, budgets, income
- **Complete a full season** and advance to the next one

### What's NOT in this alpha

To set expectations — here are things that are planned but not implemented yet:

- Multiple leagues / promotions / relegations
- Cup competitions
- Player contract negotiations
- Youth academy
- Detailed match tactics (man-marking, set piece routines, etc.)
- Multiplayer
- Sound / music
- Tutorial / guided onboarding beyond the first week

We'll get there. But right now, we need your help finding the bugs and rough edges in what we *do* have.

---

## Getting Started

### Installation

1. Download the installer for your platform from the link we shared with you
2. Run the installer — on Windows you might see a SmartScreen warning since the app isn't signed yet. Click "More info" → "Run anyway"
3. Launch **Openfoot Manager**

### Your first game

1. Click **New Game** on the main menu
2. Fill in your manager details (name, date of birth, nationality)
3. Choose a world database (the default "Random World" is fine)
4. Pick a team from the list — check their stats and pick one that looks fun
5. You'll land on the **Dashboard** — this is your home base

### Key things to know

- The **Continue** button (top-right) advances time by one day. The dropdown arrow next to it lets you skip to the next match day.
- The **sidebar** on the left is your navigation — Squad, Tactics, Training, Schedule, Finances, etc.
- Before a match, you'll be asked how you want to play it: **Go to the Field** (full control), **Watch as Spectator** (watch the AI play), or **Delegate to Assistant** (instant result).
- During matches, you can make substitutions, change formation, change play style, and adjust set piece takers.
- After matches, there's a half-time/full-time team talk and an optional press conference.
- The game **auto-saves** when you exit to the main menu. You can also save manually from the settings.

---

## What We Need From You

### The short version

Play the game. Break it. Tell us what happened.

### The longer version

We're looking for feedback in three categories:

#### 1. Bugs and crashes

This is the top priority. If something breaks, crashes, freezes, or behaves in a way that's clearly wrong — we want to know.

Examples:

- The game crashes when I try to start a match
- My formation changes don't seem to stick between sessions
- A player shows as injured but is still in my starting XI
- The standings don't add up correctly after matchday 5
- I got a message about a match that hasn't happened yet

#### 2. Usability issues

Things that aren't broken, but are confusing, annoying, or hard to use.

Examples:

- I couldn't figure out how to change my starting XI
- The training page is overwhelming, I don't know what anything does
- The text is too small / too large on my screen
- I don't understand what "play style" actually affects
- The inbox is full of messages I don't care about

#### 3. Balance and gameplay feel

This is more subjective, but equally valuable. Does the game *feel* right?

Examples:

- My team wins every match 5-0, it's too easy
- Training doesn't seem to make any difference
- Player morale never changes no matter what I do
- Transfer bids are always rejected
- The AI teams seem way too weak / too strong

---

## How to Submit Feedback

The easiest way to submit feedback is through our **GitHub issue templates**. Just pick the right one and fill it in.

If you want to chat with the team or other players, you can also join the Discord server: https://discord.gg/4ppEDH68

- [**Bug Report (English)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=bug_report.yml) — Something crashed, broke, or behaved incorrectly
- [**Feedback / Suggestion (English)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=feedback.yml) — Usability issues, balance concerns, or ideas
- [**Session Report (English)**](https://github.com/openfootmanager/openfootmanager/issues/new?template=session_report.yml) — A summary of your play session (super valuable!)

### Log files

When you report a bug, **please include your log files**. They contain detailed information about what the game was doing when something went wrong, and they're often the difference between us being able to fix a bug in 5 minutes vs. spending hours trying to reproduce it.

**Where to find your logs:**

- **Windows:** `C:\Users\<YourUsername>\AppData\Roaming\com.sturdyrobot.openfootmanager\logs\`
- **macOS:** `~/Library/Application Support/com.sturdyrobot.openfootmanager/logs/`
- **Linux:** `~/.local/share/com.sturdyrobot.openfootmanager/logs/`

Just zip up the whole `logs` folder and attach it to your report. The logs don't contain any personal information — just game events, commands, and error traces.

---

## Known Issues

Here are things we already know about — you don't need to report these (but feel free to comment on them if they affect your experience):

- **No sound or music** — the game is completely silent for now
- **UI scaling** may not be perfect on all screen sizes — check Settings → Display → UI Scale if things look off
- **Some text is not translated** — i18n is a work in progress
- **Save files from this alpha may not be compatible** with future versions. Don't get too attached to your saves!
- **Performance** may slow down slightly when simulating many days in a row

---

## A Few Final Words

This is a passion project. It's not backed by a big studio or a fat budget. It's built by people who love football and love games, in their spare time.

Your feedback during this alpha isn't just "nice to have" — it's literally shaping what this game becomes. Every bug you report, every "this confused me" comment, every "wouldn't it be cool if..." suggestion — it all matters.

So play the game, have fun with it (we hope!), and don't hold back on the feedback. There are no dumb questions and no feedback too small.

Thanks for being part of this. Let's build something great together.

— The Openfoot Manager Team

---

Alpha version 0.2.0
