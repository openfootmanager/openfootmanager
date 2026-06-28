# World Editor Guide

The World Editor (formerly the Package Editor) is a built-in, full-page screen for creating and editing `.ofm` packages without writing JSON by hand. It supports all entity types: metadata, confederations, countries, teams, players, youth players, staff, name pools, and competitions.

---

## Opening the Editor

From the main menu, click **World Editor**. You can start from a blank package, open an existing package folder, or open one of the bundled sample packages (see [Sample Packages](#sample-packages)).

---

## Home View

The home view offers:

**New Package**
Click to open a directory picker. Choose an **empty** folder where your package will be created. The editor writes `package.json` and the standard subdirectory structure to that folder immediately.

**Open Package**
Click to open a directory picker. Choose an **existing** package folder (one that already has a `package.json`). The editor loads all entities and validation state.

**Sample Packages**
The home view also lists the bundled sample packages. Opening one copies it into an editable working directory so you can explore a complete, valid package as a starting point.

---

## Edit View

The edit view is a three-column, master-detail layout: a **sidebar** of grouped sections on the left, an **entity list** in the middle, and the **edit form** on the right. The sidebar sections are grouped as:

- **Metadata**
- **World** — Confederations, Countries
- **Clubs** — Teams, Players, Youth, Staff, Names
- **Competitions** — Competitions

Each section shows the count of defined entities in its label (e.g. "Teams (4)"). Selecting a section lists its entities in the middle column; selecting a row opens that entity's form on the right without leaving the section.

Changes are written to disk each time you save or edit an entity. Use **Validate** to check everything, and **Build .ofm** when you're ready to distribute.

---

### Metadata Section

Fill in the package's top-level fields:

| Field | Description |
|-------|-------------|
| **Package ID** | Stable slug used as the install key. Use lowercase letters, numbers, and hyphens only. Example: `bundesliga-2026`. Must be unique across installed packages. |
| **Package Name** | Human-readable display name shown in the world selector. |
| **Description** | Short description shown to players before they start a game. |
| **Version** | Semantic version string. Follow `MAJOR.MINOR.PATCH`. Example: `1.0.0`. |
| **Base Year** | The in-game season year. Players see this when selecting a world. Example: `2026`. |
| **Author** | Your name or username. |
| **License** | [SPDX license identifier](https://spdx.org/licenses/). Use `CC-BY-4.0` for attribution-required, `CC0-1.0` for public domain, or `All Rights Reserved` for proprietary. |
| **Package Type** | `database` for a full world, `patch` for partial updates, `assets` for art-only packages. |
| **Min Game Version** | Minimum OFM version required to use this package. Use `0.3.0` if unsure. Leave empty for no requirement. |

---

### Confederations Section

Confederations define the top-level regional groupings that countries belong to. You only need to add confederations if you are creating fictional groupings not in the built-in catalog.

**Built-in confederation ids:** `europe`, `south-america`, `north-america`, `africa`, `asia`, `oceania`

**Add Confederation** — Opens the confederation form with empty fields.

Each confederation has two fields:
- **Confederation ID** — Stable slug, e.g. `"fictland-union"`. Referenced by countries.
- **Confederation Name** — Display name, e.g. `"Fictland Football Union"`.

---

### Countries Section

Countries define the national identities used in teams and players. Standard football country codes (`ENG`, `ES`, `DE`, `FR`, `IT`, `PT`, `BR`, `AR`, etc.) are built-in and do not need to be defined here.

Add countries when you need fictional countries or country codes not in the built-in catalog.

**Add Country** — Opens the country form.

Each country has three fields:
- **Country ID** — The code used in `team.country` and `player.nationality`. Example: `"NOR"`.
- **Country Name** — Display name. Example: `"Northshire Republic"`.
- **Confederation** — The confederation this country belongs to. Select from confederations defined in the package, or type a built-in id.

---

### Teams Section

The Teams section shows a card list of all clubs in the package. Each card shows the team's primary color swatch, name, and location.

**Add Team** — Opens the team form with empty fields.

**Edit (pencil icon)** — Opens the team form pre-filled with that team's data.

**Delete (trash icon)** — Removes the team immediately from the list.

Each team has the following fields:

| Field | Description |
|-------|-------------|
| **Team ID** | Stable slug used to reference this team in players and competitions. Auto-generated from the name if left empty. Example: `fc-northshire`. |
| **Team Name** | Full display name. Example: `FC Northshire`. |
| **Short Name** | 2–5 character abbreviation for standings tables. Example: `NSH`. |
| **City** | City the team is based in. |
| **Country** | Football country code. Must match a built-in code or a country defined in your package. |
| **Play Style** | Team's tactical tendency. One of: `Balanced`, `Attacking`, `Defensive`, `Counter`, `Pressing`. |
| **Stadium Name** | Optional home stadium name. |
| **Primary Color** | Primary kit color as a hex string. A color swatch updates as you type. |
| **Secondary Color** | Secondary kit color as a hex string. |
| **Reputation Min / Max** | Range (0–1000) from which the engine draws a random reputation at world generation. Higher = more prestigious. |
| **Finance Min / Max** | Budget range in euros drawn randomly at world generation. |

---

### Players Section

The Players section shows a list of all explicitly authored first-team players. Each entry shows the player's position, name, and club.

Most packages do not need hand-authored players — the engine generates full squads automatically. Use this section when you want specific real-world players (or fictional stars) with precise attributes.

**Add Player** — Opens the player form.

Each player has the following fields:

| Field | Description |
|-------|-------------|
| **Player ID** | Stable slug. Auto-generated from name if empty. |
| **First Name / Last Name** | Used for display and name generation. |
| **Club** | Team ID this player starts at. |
| **Nationality** | Country code. Example: `ENG`, `ES`. |
| **Position** | See [Position Values](SCHEMA_REFERENCE.md#position-values) in the Schema Reference. |
| **Preferred Foot** | Right, Left, or Both. Defaults to Right. |
| **Date of Birth** | ISO date `YYYY-MM-DD`. Used to compute age at game start. |
| **Photo** | Optional player photo asset, bundled into the package. |
| **Overall** | Single ability rating (1–99). The engine generates a realistic attribute spread from this. |
| **Attributes** | Toggle to switch from Overall mode to explicit attribute control. Shows 19 sliders grouped into Physical, Technical, Mental, and Goalkeeper categories. |

When **Attributes** mode is on, the Overall field is hidden. When **Overall** mode is on, attribute sliders are hidden. The engine uses whichever is set.

---

### Youth Section

The Youth section authors academy/youth-squad players. It uses the same form as the Players section, but saved players are flagged `youth: true` and join the club's youth squad rather than its first team. Use it to seed promising academy prospects with specific attributes.

---

### Staff Section

The Staff section authors non-playing staff — assistant managers, coaches, scouts, and physios.

**Add Staff Member** — Opens the staff form.

| Field | Description |
|-------|-------------|
| **Staff ID** | Stable slug. Auto-generated from name if empty. |
| **First Name / Last Name** | Display name. |
| **Role** | Assistant Manager, Coach, Scout, or Physio. |
| **Specialization** | For coaches: Fitness, Technique, Tactics, Defending, Attacking, Goalkeeping, or Youth. Optional. |
| **Club** | Team ID this staff member belongs to. Leave empty for an unattached / free agent. |
| **Nationality** | Country code. |
| **Date of Birth** | ISO date `YYYY-MM-DD`. |
| **Attributes** | Toggle to set Coaching, Judging Ability, Judging Potential, and Physiotherapy (1–99). |

---

### Names Section

The Names section manages name pools — lists of first and last names per country code used by the engine when generating random players.

Name pools are keyed by country code (e.g. `ENG`, `ES`, `DE`). The engine uses these when generating players for teams of that nationality.

**Add Pool** — Opens the name pool form to create a new country's pool.

**Edit (pencil icon)** — Opens the form for an existing pool.

**Delete (trash icon)** — Removes the pool.

Each name pool form has three fields:
- **Country Code** — The key for this pool. Example: `ENG`. For new pools, this is editable; for existing pools, changing it renames the pool.
- **First Names** — One name per line. Example: `James\nOliver\nHarry`.
- **Last Names** — One name per line. Example: `Smith\nJones\nBrown`.

> **Note:** If no names package is defined, the engine uses its built-in English name pools for all teams.

---

### Competitions Section

The Competitions section lists all competition definitions. Each entry shows the competition's type abbreviation, name, and scope.

**Add Competition** — Opens the competition form.

Each competition form has the following sections:

**Basic Info:**
- **Competition ID** — Stable slug. Example: `eng-premier-league`.
- **Competition Name** — Display name. Example: `Premier League`.
- **Type** — Category: League, Cup, ContinentalClub, InternationalClub, InternationalNation, or FriendlyCup.
- **Scope** — Geographic scope: Domestic, Regional, Continental, or International.
- **Country ID** — Required for Domestic competitions.
- **Priority** — Scheduling priority. Higher numbers are scheduled first.

**Format:**
- **Format Kind** — `LeagueTable`, `Knockout`, or `GroupAndKnockout`.

**Participants — Explicit Mode:**
Enter team IDs directly, one per line. All listed teams must exist in the package.

**Participants — Selector Mode:**
Choose a selector kind and its parameters:
- `topByReputation` — Top N clubs by reputation in a country; set Country and Count.
- `allInCountry` — All clubs from a country; set Country.
- `allInRegion` — All clubs from a region; set Region.
- `championsOf` — Winners or top finishers of another competition; set Source Competition.

For a complete field reference including berths, season timing, and all selector parameters, see [SCHEMA_REFERENCE.md](SCHEMA_REFERENCE.md).

---

## Save, Validate, and Build

These actions appear at the bottom of the edit view:

**Save** (top-right corner)
Writes all current entities and metadata to disk. No validation is performed.

**Validate**
Saves the current state first, then runs the same validation the CLI uses. Results appear as a red error list below the tabs. A green success message is shown when there are no errors.

**Build .ofm**
Saves and validates the package, then opens a save dialog so you can choose where to write the `.ofm` file. If validation fails, the archive is not created and the errors are shown instead.

---

## Workflow for a Complete Package

1. Open (or create) your package in the World Editor
2. Fill in Metadata
3. Add Confederations and Countries (if needed for fictional settings)
4. Add Teams
5. Add Players (optional — the engine generates squads automatically)
6. Set up Name Pools (optional — built-in pools are used if absent)
7. Define Competitions
8. Click **Validate** to check everything
9. Click **Build .ofm** when ready to distribute

You can also work alongside the CLI:

```bash
ofm-cli validate /path/to/my-package
```
