# Schema Reference

Every data file in a `.ofm` package must have a top-level `"schema"` field that identifies what kind of entity the file contains. This document describes every schema, every field, and valid values.

All field names use **camelCase** in JSON/YAML (e.g. `shortName`, `playStyle`, `seasonStartMonth`).

---

## File Format Rules

- Supported formats: `.json`, `.yaml`, `.yml`
- A file can contain **one entity** (fields at the top level alongside `schema`) or **many entities** in an `items` array
- JSON comments are not supported (use YAML for commented data)
- The loader walks the package directory recursively — directory names do not matter
- At most **one `world` entity** is allowed per package

**Single entity:**
```json
{ "schema": "team", "id": "my-club", "name": "My Club FC", "city": "London", "country": "ENG", ... }
```

**Multiple entities (items array):**
```json
{
  "schema": "team",
  "items": [
    { "id": "club-a", "name": "Club A", ... },
    { "id": "club-b", "name": "Club B", ... }
  ]
}
```

---

## `world` — Package Manifest

The package manifest. Place this in `package.json` (or any file with `"schema": "world"`). Only one world entity is allowed per package.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | yes | — | Stable slug used as the install key. Lowercase letters, numbers, and hyphens only. Example: `"bundesliga-2026"`. Must be unique. |
| `name` | string | yes | — | Human-readable display name. Example: `"Bundesliga 2026"`. |
| `description` | string | no | `""` | Short description shown in the world selector. |
| `version` | string | yes | — | Semantic version. Example: `"1.0.0"`. Increment on each update. |
| `author` | string | no | `""` | Author name or username. |
| `license` | string | no | `""` | [SPDX license identifier](https://spdx.org/licenses/). Examples: `"CC-BY-4.0"`, `"CC0-1.0"`, `"MIT"`. |
| `packageType` | string | no | `"database"` | One of: `"database"`, `"patch"`, `"assets"`. |
| `gameMinVersion` | string | no | `""` | Minimum OFM version required. Semver string. Example: `"0.3.0"`. Empty = no requirement. |
| `formatVersion` | integer | no | `1` | Schema format version. Always `1` for the current release. |
| `baseYear` | integer or null | no | `null` | Season year displayed in the world selector. Example: `2026`. |
| `defaultActiveRegions` | array of strings | no | `[]` | Region ids that are enabled by default when starting a new game with this package. |
| `defaultActiveCompetitions` | array of strings | no | `[]` | Competition ids that are enabled by default when starting a new game with this package. |
| `fallbackLeague` | object or null | no | `null` | Overrides for the league auto-generated when a `database` package declares teams but **no** competitions. See below. |

### `fallbackLeague` overrides

When a `database` package defines teams but no competitions, the engine synthesizes a single-division league over all teams so the world is still playable (and raises a notice). This object lets you shape that league. Every field is optional; an omitted field uses the built-in default.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string or null | localized "Default League" | Display name, used verbatim. |
| `legs` | integer or null | `2` | Rounds each pair plays: `1` (single) or `2` (double round-robin). Other values are ignored. |
| `scope` | string or null | `"Domestic"` | One of `"Domestic"`, `"Regional"`, `"Continental"`, `"International"`. |

If you define your own competitions, this object is ignored.

**Minimal valid example:**
```json
{
  "schema": "world",
  "id": "my-league-2026",
  "name": "My League 2026",
  "version": "1.0.0",
  "packageType": "database",
  "gameMinVersion": "0.3.0"
}
```

---

## `team` — Club Definition

Defines a football club.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | no | _(auto-UUID)_ | Stable slug used to reference this team in players and competitions. Auto-generated from `name` if empty. Example: `"manchester-city"`. |
| `name` | string | yes | — | Full display name. Example: `"Manchester City"`. |
| `shortName` | string | no | `""` | 2–5 character abbreviation for standings tables. Example: `"MCI"`. |
| `city` | string | yes | — | City the team is based in. |
| `country` | string | yes | — | Football country code. See [Country Codes](#country-codes) below. Example: `"ENG"`. |
| `colors.primary` | string | yes | — | Primary kit color as a hex string. Example: `"#1c6bba"`. |
| `colors.secondary` | string | yes | — | Secondary kit color as a hex string. Example: `"#ffffff"`. |
| `playStyle` | string | no | `"Balanced"` | Team's tactical tendency. One of: `"Balanced"`, `"Attacking"`, `"Defensive"`, `"Counter"`, `"Pressing"`. |
| `stadiumName` | string | no | `""` | Home stadium name. |
| `reputationRange` | [integer, integer] or null | no | `null` | `[min, max]` reputation (0–1000). The game draws a random value in this range at world generation. Higher = more prestigious. |
| `financeRange` | [integer, integer] or null | no | `null` | `[min, max]` budget in euros. The game draws a random value in this range at world generation. |
| `logo` | string or null | no | `null` | Relative path to the team's logo image inside the package. Example: `"assets/logos/manchester-city.png"`. |

**Example:**
```json
{
  "schema": "team",
  "id": "manchester-city",
  "name": "Manchester City",
  "shortName": "MCI",
  "city": "Manchester",
  "country": "ENG",
  "colors": { "primary": "#1c6bba", "secondary": "#ffffff" },
  "playStyle": "Pressing",
  "stadiumName": "Etihad Stadium",
  "reputationRange": [850, 1000],
  "financeRange": [50000000, 200000000]
}
```

---

## `player` — Player Definition

Defines a specific player. Reference teams and countries by their `id`.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | no | _(auto-UUID)_ | Stable slug. Auto-generated from `name` if empty. |
| `name` | string | no | `""` | Full display name (used as fallback if `firstName`/`lastName` are empty). |
| `firstName` | string | no | `""` | First name. Used for display and name pool lookup. |
| `lastName` | string | no | `""` | Last name. |
| `club` | string | no | `""` | Team `id` this player starts at. Must match an existing team. |
| `nationality` | string | no | `""` | Country `id` (ISO alpha-2 or football code). Example: `"ENG"`, `"ES"`. |
| `position` | string | no | `"Goalkeeper"` | Player position. See [Position Values](#position-values). |
| `footedness` | string or null | no | `"Right"` | Preferred foot: `"Right"`, `"Left"`, or `"Both"`. |
| `dateOfBirth` | string or null | no | `null` | ISO 8601 date: `"YYYY-MM-DD"`. Example: `"1990-05-15"`. |
| `age` | integer or null | no | `null` | Player age at world generation start. Used if `dateOfBirth` is absent. |
| `youth` | boolean | no | `false` | If `true`, the player joins the club's youth/academy squad instead of the first team. |
| `photo` | string or null | no | `null` | Relative path to a player photo asset bundled in the package. |
| `overall` | integer (1–99) or null | no | `null` | Overall ability rating. The engine generates a realistic attribute spread from this value. |
| `attributes` | object or null | no | `null` | Explicit attribute overrides (18 attributes). Overrides the `overall`-based generation for the specified attributes. |

> **Tip**: You only need to specify `overall` *or* `attributes` — not both. For most authored players, `overall` is sufficient. Use `attributes` for precise control.

**Position Values:**

| Value | Role |
|-------|------|
| `"Goalkeeper"` | Goalkeeper |
| `"CenterBack"` | Centre-back |
| `"LeftBack"` | Left back |
| `"RightBack"` | Right back |
| `"LeftWingBack"` | Left wing-back |
| `"RightWingBack"` | Right wing-back |
| `"DefensiveMidfielder"` | Defensive midfielder |
| `"CentralMidfielder"` | Central midfielder |
| `"AttackingMidfielder"` | Attacking midfielder |
| `"LeftMidfielder"` | Left midfielder |
| `"RightMidfielder"` | Right midfielder |
| `"LeftWinger"` | Left winger |
| `"RightWinger"` | Right winger |
| `"Striker"` | Striker |
| `"Defender"` | Generic defender (engine assigns specific role) |
| `"Midfielder"` | Generic midfielder |
| `"Forward"` | Generic forward |

**Example:**
```json
{
  "schema": "player",
  "id": "john-smith",
  "firstName": "John",
  "lastName": "Smith",
  "club": "northshire-fc",
  "nationality": "ENG",
  "position": "CentralMidfielder",
  "dateOfBirth": "1998-03-22",
  "overall": 72
}
```

---

## `staff` — Coaching & Backroom Staff

Defines a non-playing staff member (manager assistant, coach, scout, or physio). Place these inside `staff/*.json` in the `"items"` array. Reference teams by their `id`.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | no | _(auto-UUID)_ | Stable slug. Auto-generated from the name if empty. |
| `firstName` | string | no | `""` | First name. |
| `lastName` | string | no | `""` | Last name. |
| `club` | string | no | `""` | Team `id` this staff member belongs to. Empty = unattached / free agent. |
| `nationality` | string | no | `""` | Country `id` (ISO alpha-2 or football code). |
| `role` | string | no | `"Coach"` | One of `"AssistantManager"`, `"Coach"`, `"Scout"`, `"Physio"`. |
| `specialization` | string or null | no | `null` | Coaching focus (coaches only): `"Fitness"`, `"Technique"`, `"Tactics"`, `"Defending"`, `"Attacking"`, `"GoalKeeping"`, `"Youth"`. |
| `dateOfBirth` | string or null | no | `null` | ISO 8601 date: `"YYYY-MM-DD"`. |
| `age` | integer or null | no | `null` | Age at world generation start. Used if `dateOfBirth` is absent. |
| `attributes` | object or null | no | `null` | Explicit attribute overrides: `coaching`, `judgingAbility`, `judgingPotential`, `physiotherapy` (each 1–99). |

**Example:**
```json
{
  "schema": "staff",
  "id": "alex-ferguson",
  "firstName": "Alex",
  "lastName": "Ferguson",
  "club": "northshire-fc",
  "nationality": "ENG",
  "role": "AssistantManager",
  "specialization": null,
  "dateOfBirth": "1941-12-31"
}
```

---

## `confederation` — Region Definition

Defines a football confederation or regional grouping. You only need this if you are creating fictional confederations not in the built-in catalog.

**Built-in confederation ids:** `europe`, `south-america`, `north-america`, `africa`, `asia`, `oceania`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | yes | — | Stable slug. Example: `"europa-fictiva"`. |
| `name` | string | yes | — | Display name. Example: `"Europa Fictiva"`. |

**Example:**
```json
{ "schema": "confederation", "id": "fictland-union", "name": "Fictland Football Union" }
```

---

## `country` — Country Definition

Defines a country. You only need this for fictional countries. Standard football country codes (`ENG`, `ES`, `DE`, `FR`, `IT`, `PT`, `BR`, `AR`, etc.) are built-in and do not need to be re-declared.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | yes | — | Country code or slug. Used in `team.country`, `player.nationality`, and competition country references. |
| `name` | string | yes | — | Display name. Example: `"England"`. |
| `confederation` | string | no | `""` | Confederation `id` this country belongs to. Must match a built-in or package-defined confederation. |

**Example (fictional country):**
```json
{ "schema": "country", "id": "NOR", "name": "Northshire Republic", "confederation": "europe" }
```

---

## Country Codes

The following codes are in the built-in catalog and can be used in `team.country` and `player.nationality` without defining a `country` entity:

| Code | Country | Code | Country | Code | Country |
|------|---------|------|---------|------|---------|
| `ENG` | England | `SCO` | Scotland | `WAL` | Wales |
| `NIR` | Northern Ireland | `ES` | Spain | `DE` | Germany |
| `FR` | France | `IT` | Italy | `PT` | Portugal |
| `NL` | Netherlands | `BE` | Belgium | `BR` | Brazil |
| `AR` | Argentina | `CO` | Colombia | `MX` | Mexico |
| `US` | United States | `CN` | China | `JP` | Japan |
| `KR` | South Korea | `SA` | Saudi Arabia | `NG` | Nigeria |
| `SN` | Senegal | `GH` | Ghana | `EG` | Egypt |
| `MA` | Morocco | `ZA` | South Africa | `AU` | Australia |

This is not the complete list. To see all supported codes, run `ofm-cli schema country` or check the `nations` module in the source.

---

## `competition` — Competition Definition

Competitions are the most complex entity. A competition defines a league, cup, or continental tournament and the rules for selecting participants.

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Stable slug. Example: `"eng-premier-league"`. Must be unique. |
| `name` | string | Display name. Example: `"Premier League"`. |
| `type` | string | Competition category. See [Competition Types](#competition-types). |
| `scope` | string | Geographic scope. See [Competition Scopes](#competition-scopes). |
| `format` | object | Format rules. See [Format](#format). |
| `participants` | object | How participants are chosen. See [Participants](#participants). |

### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `countryId` | string or null | `null` | Country this competition belongs to (required for `Domestic` scope). |
| `regionId` | string or null | `null` | Region this competition belongs to (required for `Regional`/`Continental` scope). |
| `requiredRegionIds` | array of strings | `[]` | Region ids that must be active for this competition to appear. |
| `priority` | integer | `0` | Scheduling priority. Higher-priority competitions are scheduled first. |
| `berths` | array | `[]` | Qualification spots this competition awards to other competitions. See [Berths](#berths). |
| `seasonStartMonth` | integer (1–12) | `8` | Month the season begins. |
| `seasonStartDay` | integer (1–31) | `1` | Day of the month the season begins. |
| `nameKey` | string or null | `null` | i18n key for a translated competition name. |

### Competition Types

| Value | Use |
|-------|-----|
| `"League"` | Domestic league table |
| `"Cup"` | Domestic knockout cup |
| `"ContinentalClub"` | Club-level continental tournament (e.g. Champions League) |
| `"InternationalClub"` | Club-level international tournament |
| `"InternationalNation"` | National team tournament |
| `"FriendlyCup"` | Pre-season or friendly tournament |

### Competition Scopes

| Value | Use |
|-------|-----|
| `"Domestic"` | Belongs to one country; set `countryId` |
| `"Regional"` | Belongs to a region; set `regionId` |
| `"Continental"` | Continent-wide; set `regionId` |
| `"International"` | Global tournament |

### Format

```json
"format": {
  "kind": "LeagueTable",
  "legs": 2
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kind` | string | — | `"LeagueTable"`, `"Knockout"`, or `"GroupAndKnockout"` |
| `legs` | integer | `2` | _(LeagueTable / groups phase only)_ Number of legs per round-robin cycle. `1` = one-leg, `2` = home and away. |
| `groupSize` | integer | `4` | _(GroupAndKnockout only)_ Clubs per group. |
| `qualifiersPerGroup` | integer | `2` | _(GroupAndKnockout only)_ Clubs advancing from each group. |
| `bestThirdQualifiers` | integer | `0` | _(GroupAndKnockout only)_ Best third-placed teams that also advance (like the 2026 World Cup format). |

### Participants

Exactly one of `explicit` or `selector` must be set.

**Explicit list:**
```json
"participants": {
  "explicit": ["team-id-1", "team-id-2", "team-id-3", "team-id-4"]
}
```
All team ids must exist in the package. Minimum 2 teams.

**Selector:**
```json
"participants": {
  "selector": {
    "kind": "topByReputation",
    "country": "ENG",
    "count": 20
  }
}
```

| Selector Kind | Description | Required fields |
|---------------|-------------|-----------------|
| `"topByReputation"` | Top N clubs by reputation in a country | `country`, `count` (≥ 2) |
| `"allInCountry"` | All clubs from a country | `country` |
| `"allInRegion"` | All clubs from a region | `region` |
| `"championsOf"` | Top N finishers of another competition | `sourceCompetition`, optionally `count` |

Additional selector fields:

| Field | Description |
|-------|-------------|
| `excludeCompetitions` | Array of competition ids whose participants are excluded (e.g. to fill a second division without repeating first division clubs) |

### Berths

Berths define automatic qualification spots that a competition awards to other competitions. For example, a league's top two finishers qualify for a continental cup.

```json
"berths": [
  {
    "targetCompetition": "continental-cup",
    "rule": { "type": "LeagueFinishers", "from": 1, "to": 2 }
  },
  {
    "targetCompetition": "relegation-play-off",
    "rule": { "type": "LeagueFinishers", "from": 17, "to": 20 }
  }
]
```

| Berth Rule Type | Description |
|-----------------|-------------|
| `"LeagueFinishers"` | League finishers from position `from` to `to` (1-based, inclusive) |
| `"CupWinner"` | The winner of this cup |

### Full Example

A domestic league using `topByReputation` selector:

```json
{
  "schema": "competition",
  "id": "eng-premier-league",
  "name": "Premier League",
  "type": "League",
  "scope": "Domestic",
  "countryId": "ENG",
  "priority": 100,
  "format": {
    "kind": "LeagueTable",
    "legs": 2
  },
  "participants": {
    "selector": {
      "kind": "topByReputation",
      "country": "ENG",
      "count": 20
    }
  },
  "berths": [
    { "targetCompetition": "eng-champions-cup", "rule": { "type": "LeagueFinishers", "from": 1, "to": 4 } }
  ],
  "seasonStartMonth": 8,
  "seasonStartDay": 1
}
```

A group + knockout continental tournament:

```json
{
  "schema": "competition",
  "id": "continental-champions-cup",
  "name": "Continental Champions Cup",
  "type": "ContinentalClub",
  "scope": "Continental",
  "regionId": "europe",
  "priority": 50,
  "format": {
    "kind": "GroupAndKnockout",
    "legs": 1,
    "groupSize": 4,
    "qualifiersPerGroup": 2
  },
  "participants": {
    "selector": {
      "kind": "championsOf",
      "sourceCompetition": "eng-premier-league",
      "count": 1
    }
  }
}
```

---

## `names` — Name Pools

Provides first and last name lists for random player name generation. Keyed by ISO 3166-1 alpha-2 country code.

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `version` | integer | no | `1` | Always `1`. |
| `description` | string | no | `""` | Optional description of the name pool. |
| `pools` | object | yes | — | Map of country code → name lists. |
| `pools.<code>.first_names` | array of strings | yes | — | First name list for this country. |
| `pools.<code>.last_names` | array of strings | yes | — | Last name list for this country. |

> **Note:** The name pool fields use **snake_case** (`first_names`, `last_names`), unlike most other entities which use camelCase. This matches the internal serialization format.

**Example:**
```json
{
  "schema": "names",
  "version": 1,
  "description": "Fictional name pools for Northshire",
  "pools": {
    "NOR": {
      "first_names": ["Aldo", "Bram", "Celia", "Dana"],
      "last_names": ["Northwick", "Ashfield", "Moorgate"]
    }
  }
}
```

The built-in name pools already cover common footballing countries. You only need to define name pools for countries your package introduces, or if you want to customize the names generated for existing countries.
