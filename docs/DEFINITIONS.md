# Definition Files

OpenFootManager uses **JSON or YAML definition files** to drive world generation. These files control the name pools and the per-nation inputs to procedural club generation. You can customize or replace them to change the nations a generated world contains, the cities its clubs are named after, and the names its players are given.

> **JSON or YAML?** Every definition file may be written in either format — pick whichever you prefer; YAML is often easier to hand-author. Files are recognised by their `.json`, `.yaml`, or `.yml` extension (and standalone imports are sniffed by content). All examples below show JSON, but the same fields apply to YAML.

## Definition files or a package?

Two different jobs, and picking the wrong one is the usual source of confusion:

| You want to… | Use |
|---|---|
| Change how the engine *generates* a world — which nations exist, their cities, how many divisions, what names players get | a **definition file** (this document) |
| Ship specific, hand-authored clubs, players or competitions | an **`.ofm` package** ([modding guide](modding/README.md)) |

Definition files configure generation. Packages supply content. A curated list of clubs is content, so it belongs in a package — see the [`classic-sixteen`](modding/examples/classic-sixteen/) example.

## File Locations

The game searches for each definition file in this order and uses the first one it finds:

1. **Your data directory** — `<app-data>/data/` — writable, and the one to use for your own edits:
   - Linux: `~/.local/share/com.sturdyrobot.openfootmanager/data/`
   - macOS: `~/Library/Application Support/com.sturdyrobot.openfootmanager/data/`
   - Windows: `%APPDATA%\com.sturdyrobot.openfootmanager\data\`
2. **Bundled data** — `<app-resources>/data/`, shipped beside the installed game. Read-only — treat it as the reference copy to read and copy from, not to edit in place.
3. **Built into the binary** — the same files as (2), compiled in, so the game always has a working set even if the install is incomplete.

A file that is present but does not parse is skipped with a warning, and the next tier is used. That way a half-edited override never stops the game starting.

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

### `default_nations.json` — Generation Nations

Controls **which nations a generated world contains** and how their leagues are
shaped. This is the seed procedural generation grows from: each nation produces
`clubsPerDivision × tiers` clubs, named from its city pool using its naming
style.

Adding a nation used to mean editing Rust and recompiling. It is now an edit to
this file.

```json
{
  "version": 1,
  "description": "My custom generation nations",
  "clubsPerDivision": 4,
  "colorPalette": [
    { "primary": "#dc2626", "secondary": "#ffffff" }
  ],
  "genericCities": ["Northtown", "Eastford"],
  "nations": [
    {
      "code": "JP",
      "style": "Generic",
      "tiers": 1,
      "strength": 3,
      "cities": ["Tokyo", "Osaka", "Nagoya", "Sapporo"]
    }
  ]
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `version` | `number` | No | `0` | Schema version |
| `description` | `string` | No | `""` | Human-readable description |
| `clubsPerDivision` | `number` | No | `20` | Clubs generated in each division |
| `colorPalette` | `{primary, secondary}[]` | No | `[]` | Kit colour pairs drawn from at random |
| `genericCities` | `string[]` | No | `[]` | City names used for a country with no curated pool |
| `nations` | `NationGen[]` | **Yes** | — | The nations to generate |

#### NationGen

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `code` | `string` | **Yes** | — | Football country code — see [Country Codes](#country-codes) |
| `cities` | `string[]` | **Yes** | — | City pool club names are built from |
| `style` | `string` | **Yes** | — | Naming culture (below) |
| `tiers` | `number` | **Yes** | — | Divisions: `1`, or `2` for a nation with a second tier |
| `strength` | `number` | **Yes** | — | `1`–`5`; seeds the reputation band its clubs are drawn in |

**`style`** is one of `English`, `Scottish`, `Spanish`, `Italian`, `German`,
`French`, `Portuguese`, `Dutch`, `Nordic`, `Balkan`, `LatinAmerican`,
`Brazilian`, `Generic`. Unlike most fields, an unrecognised value is a **parse
error** rather than a silent default — the file is skipped and the shipped one
used, with a warning in the log. Use `Generic` for a culture the list does not
cover yet.

**Notes:**
- Give a nation at least as many cities as it generates clubs, or names start
  repeating with numeric suffixes (`Tokyo FC 2`).
- Use a `code` that is in the [nation catalog](#country-codes). An uncatalogued
  code still generates, but the world cannot tell which region it belongs to and
  files it under Europe.
- Each club gets 22 players (2 GK, 7 DEF, 7 MID, 6 FWD) and 4 staff
  (AssistantManager, Coach, Scout, Physio); 12 free-agent staff are generated on
  top, regardless of club count.
- Player nationalities are weighted 60% toward the club's country and 40% drawn
  from the available name pools.

> **Looking for `default_teams.json`?** It was retired. As a definition file it
> *replaced* procedural generation rather than adding to it, so shipping one
> silently cut a ~440-club world down to its own contents. Hand-authored clubs
> belong in an `.ofm` package, which merges and stacks properly — the clubs it
> used to hold now ship as the
> [`classic-sixteen`](modding/examples/classic-sixteen/) example package.

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
| `countryId` | `string` | No | Country code. Competitions sharing a `countryId` form a promotion/relegation pyramid, ordered by `priority` (lower = higher division). Adjacent tiers swap by default. A domestic league that is the primary `PositionRange` berth target of sibling feeders is filled from those feeders instead, and is not chained into the linear ladder. |
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

1. **Start from the shipped copy** — take `default_names.json` or
   `default_nations.json` from the game's bundled `data/` directory (tier 2 in
   [File Locations](#file-locations)). They are the real files, so you are
   editing something known to work rather than starting from a blank page.
2. **Edit** — add your own nations, cities and name pools. Any text editor; JSON
   or YAML.
3. **Place** — put the edited file in **your** data directory (tier 1), *not*
   back into the bundled one. On Linux that is
   `~/.local/share/com.sturdyrobot.openfootmanager/data/`. Create the folder if
   it is not there. World databases still go in `databases/`.
4. **Test** — start a new game and verify your changes appear. If they do not,
   check the log: a file that fails to parse is skipped with a warning and the
   shipped one is used instead.

### Tips

- Keep at least 10 first names and 10 last names per nationality for good variety.
- Team count should be even (4, 8, 12, 16, 20...).
- Colors should be valid CSS hex colors.
- If a file has a JSON syntax error, the game silently uses defaults — check your JSON with a validator if things don't appear.
