# Definition Files

OpenFootManager uses **JSON or YAML definition files** to drive world generation. These files control the name pools, team templates, and other data used when creating a new game. You can customize or replace them to create your own leagues, nationalities, and more.

> **JSON or YAML?** Every definition file may be written in either format — pick whichever you prefer; YAML is often easier to hand-author. Files are recognised by their `.json`, `.yaml`, or `.yml` extension (and standalone imports are sniffed by content). All examples below show JSON, but the same fields apply to YAML.

## File Locations

The game searches for definition files in the following order:

1. **Bundled data** — `<app-resources>/data/` (ships with the game)
2. **Hardcoded fallback** — built into the binary (always available)

If a file cannot be found or parsed, the game silently falls back to the hardcoded defaults.

## File Types

### `default_names.json` — Name Pools

Controls the first and last names used when generating players and staff.

```json
{
  "version": 1,
  "description": "My custom name pools",
  "pools": {
    "ENG": {
      "first_names": ["James", "Harry", "Jack"],
      "last_names": ["Smith", "Johnson", "Brown"]
    },
    "ES": {
      "first_names": ["Sergio", "Pablo", "Carlos"],
      "last_names": ["Garcia", "Rodriguez", "Martinez"]
    }
  }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `number` | No | Schema version (currently `1`) |
| `description` | `string` | No | Human-readable description |
| `pools` | `object` | **Yes** | Map of nationality code → name pool |
| `pools.<CODE>.first_names` | `string[]` | **Yes** | List of first names for this nationality |
| `pools.<CODE>.last_names` | `string[]` | **Yes** | List of last names for this nationality |

**Notes:**
- Codes should be uppercase short nationality codes. Most use ISO 3166-1 alpha-2 (for example `"ES"`, `"BR"`), but football nations may use project-owned codes such as `"ENG"`, `"SCO"`, `"WAL"`, and `"NIR"`.
- Legacy `"GB"` pools are still accepted and used as a fallback for British football nations when a dedicated pool is missing.
- You can add as many or as few nationalities as you like.
- The generator picks names from the pool matching the player's nationality. If a nationality has no pool entry, a random pool is used as fallback.
- More names = more variety. The default pools have 20 first names and 20 last names each.

---

### `default_teams.json` — Team Templates

Controls the teams created during world generation.

```json
{
  "version": 1,
  "description": "My custom league",
  "teams": [
    {
      "name": "London FC",
      "short_name": "LFC",
      "city": "London",
      "country": "ENG",
      "colors": {
        "primary": "#dc2626",
        "secondary": "#ffffff"
      },
      "play_style": "Possession",
      "stadium_name": "London Arena",
      "reputation_range": [600, 900],
      "finance_range": [3000000, 10000000]
    }
  ]
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `version` | `number` | No | `0` | Schema version |
| `description` | `string` | No | `""` | Human-readable description |
| `teams` | `TeamDef[]` | **Yes** | — | Array of team definitions |

#### TeamDef

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | **Yes** | — | Full team name |
| `short_name` | `string` | No | Auto-generated from initials | 2-3 letter abbreviation |
| `city` | `string` | **Yes** | — | City name |
| `country` | `string` | **Yes** | — | Team location / football identity code |
| `colors.primary` | `string` | **Yes** | — | Primary color (hex, e.g. `"#dc2626"`) |
| `colors.secondary` | `string` | **Yes** | — | Secondary color (hex) |
| `play_style` | `string` | No | `"Balanced"` | One of: `Attacking`, `Defensive`, `Possession`, `Counter`, `HighPress`, `Balanced` |
| `stadium_name` | `string` | No | `"<city> Arena"` | Stadium name |
| `reputation_range` | `[min, max]` | No | `[300, 900]` | Random reputation range (0-1000) |
| `finance_range` | `[min, max]` | No | `[500000, 10000000]` | Random starting finance range |

**Notes:**
- The number of teams determines the league size. Must be an **even** number ≥ 2 for schedule generation.
- Each team gets 22 players (2 GK, 7 DEF, 7 MID, 6 FWD) and 4 staff (AssistantManager, Coach, Scout, Physio).
- Player nationalities are weighted 60% toward the team's country, 40% random from available pools.
- 12 free-agent staff are also generated regardless of team count.

---

## Country Codes

Nationality and team-country fields use short uppercase codes. Most are **ISO 3166-1 alpha-2**, but football nations can use dedicated codes where needed. Common codes:

| Code | Country |
|------|---------|
| `ENG` | England |
| `SCO` | Scotland |
| `WAL` | Wales |
| `NIR` | Northern Ireland |
| `IE` | Republic of Ireland |
| `GB` | Legacy British umbrella code, still accepted for compatibility |
| `ES` | Spain |
| `DE` | Germany |
| `FR` | France |
| `IT` | Italy |
| `NL` | Netherlands |
| `PT` | Portugal |
| `BR` | Brazil |
| `AR` | Argentina |
| `BE` | Belgium |
| `HR` | Croatia |
| `SE` | Sweden |

For the full ISO list, see [ISO 3166-1 alpha-2 on Wikipedia](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2). Football-specific codes are defined by the game itself.

---

## World Database Files

In addition to definition files (which control _generation_), the game also supports **world database files** — pre-built worlds saved as JSON. These are a complete snapshot of teams, players, and staff.

World databases can be:
- **Exported** from an existing game via Settings → Export World Database
- **Imported** when creating a new game via the "Import" option

World database format matches the internal `WorldData` structure:

```json
{
  "name": "My Custom World",
  "description": "A hand-crafted league with 20 teams",
  "teams": [ /* full Team objects */ ],
  "players": [ /* full Player objects */ ],
  "staff": [ /* full Staff objects */ ]
}
```

These files are placed in:
- `<app-resources>/databases/` for bundled worlds
- `<app-data>/databases/` for user-imported worlds

---

## Competition Definitions

You can define your own leagues, cups, and international tournaments — a Turkish
league and cup, an Asian Champions Cup, a reconfigured World Cup, anything — and
have the game build them when a new career starts.

Competition definitions can be supplied two ways:

1. **Embedded in a world** — a `competitionDefinitions` section inside the world
   manifest/package. Ship a world with its own curated competitions.
2. **Standalone files** — a separate JSON file selected during new-game setup and
   layered onto the chosen world.

Unlike the other definition files, competition definitions are **validated
strictly**: if anything is wrong (an unknown team, a duplicate id, a circular
qualification link…), the import is rejected and the game shows you the exact
list of problems. Nothing loads half-broken.

### File shape

```json
{
  "formatVersion": 1,
  "competitions": [
    {
      "id": "tr-super-lig",
      "name": "Süper Lig",
      "type": "League",
      "scope": "Domestic",
      "countryId": "TR",
      "priority": 50,
      "format": { "kind": "LeagueTable" },
      "participants": {
        "selector": { "kind": "topByReputation", "country": "TR", "count": 18 }
      }
    },
    {
      "id": "tr-cup",
      "name": "Turkish Cup",
      "type": "Cup",
      "scope": "Domestic",
      "countryId": "TR",
      "priority": 51,
      "format": { "kind": "Knockout" },
      "participants": { "selector": { "kind": "allInCountry", "country": "TR" } }
    },
    {
      "id": "asian-champions-cup",
      "name": "Asian Champions Cup",
      "type": "ContinentalClub",
      "scope": "Continental",
      "requiredRegionIds": ["asia"],
      "priority": 200,
      "format": { "kind": "GroupAndKnockout", "groupSize": 4, "qualifiersPerGroup": 2, "legs": 1 },
      "participants": { "selector": { "kind": "championsOf", "sourceCompetition": "tr-super-lig", "count": 2 } }
    }
  ]
}
```

### Competition fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `string` | Yes | Unique id across the file. |
| `name` | `string` | Yes | Display name (shown as-is; not translated). |
| `type` | `string` | Yes | `League`, `Cup`, `ContinentalClub`, `InternationalClub`, `InternationalNation`, or `FriendlyCup`. |
| `scope` | `string` | Yes | `Domestic`, `Regional`, `Continental`, or `International`. |
| `countryId` | `string` | No | Country code. Competitions sharing a `countryId` form a promotion/relegation pyramid, ordered by `priority` (lower = higher division). |
| `regionId` | `string` | No | Region id this competition belongs to. |
| `requiredRegionIds` | `string[]` | No | Regions that must be active for this competition to be simulated. |
| `priority` | `number` | No | Sort order in lists; also the tier within a country pyramid. |
| `format` | `object` | Yes | See **Format**. |
| `participants` | `object` | Yes | See **Participants**. |

### Format

`format.kind` is one of `LeagueTable`, `Knockout`, or `GroupAndKnockout`.

| Field | Applies to | Default | Description |
|-------|-----------|---------|-------------|
| `kind` | all | — | The competition shape. |
| `legs` | LeagueTable, GroupAndKnockout | `2` | Round-robin legs (1 = single, 2 = home & away). |
| `groupSize` | GroupAndKnockout | `4` | Clubs per group. |
| `qualifiersPerGroup` | GroupAndKnockout | `2` | Clubs advancing from each group. |
| `bestThirdQualifiers` | GroupAndKnockout | `0` | Extra best next-placed finishers that advance (the 2026 World Cup's "best thirds"). |

To make a continental cup **knockout-only**, use `{ "kind": "Knockout" }`. To
make a 16-team World Cup, define an `InternationalNation` competition with a
`GroupAndKnockout` format and the field you want.

### Participants

Provide **exactly one** of `explicit` or `selector`.

```json
"participants": { "explicit": ["team-id-a", "team-id-b"] }
```

| Selector `kind` | Fields | Resolves to |
|-----------------|--------|-------------|
| `topByReputation` | `country`, `count` | The strongest `count` clubs of a country. |
| `allInCountry` | `country` | Every club of a country. |
| `allInRegion` | `region` | Every club of a region. |
| `championsOf` | `sourceCompetition`, `count` | The top `count` finishers of another competition (continental qualification). |

`selector.excludeCompetitions` (a list of competition ids) removes clubs already
placed elsewhere — e.g. a second division excludes the first division's clubs:

```json
"participants": {
  "selector": { "kind": "topByReputation", "country": "TR", "count": 18, "excludeCompetitions": ["tr-super-lig"] }
}
```

### Validation

Every problem is reported at once (not just the first). Common errors: unknown
team/country/region, duplicate id, missing or doubled participant source, a
`championsOf` selector pointing at an unknown competition or forming a cycle,
group settings on a non-group format, and an unsupported `formatVersion`.

---

## World Packages

A **world package** is a *folder* of definition files that together describe a
complete world — confederations, countries, clubs, players, and competitions —
instead of a single monolithic world database. The loader walks the folder
**recursively** and classifies every `.json`/`.yaml`/`.yml` file by a top-level
**`schema`** field, never by which sub-folder it sits in, so you can organise
files however you like. Entities link to one another by stable string `id`s,
resolved after every file is read.

### File `schema` types

Each file sets `"schema": "<type>"` and then the entity's fields. A single file
may hold one entity, or many of the same type under an `items` array.

| `schema` | Purpose | Key fields |
| --- | --- | --- |
| `world` | Package metadata (at most one) | `name`, `description`, `defaultActiveRegions`, `defaultActiveCompetitions`, `baseYear` |
| `confederation` | A region/confederation | `id` (region id), `name` |
| `country` | A country in a confederation | `id` (country code), `name`, `confederation` (a confederation id) |
| `team` | A club | `id`, `name`, `city`, `country` (a country id), `colors`, optional `shortName`, `stadiumName`, `reputationRange`, `financeRange` |
| `player` | A hand-authored player | `id`, `club` (a team id), `nationality` (a country id), `position`; ability as a single `overall` *or* an explicit `attributes` block |
| `competition` | A competition (same shape as a Competition Definition, above) | `id`, `name`, `type`, `format`, `participants`, … |
| `names` | Name pools (same shape as `default_names`) | per-nationality first/last name lists |

```json
// confederations.json
{ "schema": "confederation", "id": "europe", "name": "Europe" }

// countries.json — bulk form
{
  "schema": "country",
  "items": [
    { "id": "TR", "name": "Türkiye", "confederation": "europe" }
  ]
}

// galatasaray.json
{
  "schema": "team",
  "id": "ts-gs", "name": "Galatasaray", "city": "Istanbul", "country": "TR",
  "colors": { "primary": "#A90432", "secondary": "#FBB03B" }
}

// star-player.yaml
schema: player
id: gs-icardi
name: M. Icardi
club: ts-gs
nationality: AR
position: Striker
overall: 84
```

### Importing a package

In the new-game screen, use **Import World Package** and pick the package
folder. The game validates the whole package up front — unknown confederations,
countries, or clubs, missing or duplicate ids, malformed files — and lists every
problem at once. A package only becomes selectable once it is valid; nothing
loads half-broken. If the package has no `world` name, the folder name is used.

---

## Creating Your Own

1. **Start simple** — Copy `default_names.json` and `default_teams.json` from the `data/` directory.
2. **Edit** — Add your own teams, cities, name pools. Use any text editor.
3. **Place** — Put your files in the game's `data/` directory (for definition files) or `databases/` directory (for world databases).
4. **Test** — Start a new game and verify your changes appear.

### Tips

- Keep at least 10 first names and 10 last names per nationality for good variety.
- Team count should be even (4, 8, 12, 16, 20...).
- Colors should be valid CSS hex colors.
- If a file has a JSON syntax error, the game silently uses defaults — check your JSON with a validator if things don't appear.
