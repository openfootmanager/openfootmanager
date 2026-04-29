# Definition Files

OpenFootManager uses **JSON definition files** to drive world generation. These files control the name pools, team templates, and other data used when creating a new game. You can customize or replace them to create your own leagues, nationalities, and more.

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

### `default_teams.json` — Team Templates or blueprint

Controls the teams created during world generation.
the two following modes are supported:
- an explicit `teams` list,
- or a structural blueprint via `structure.nations[].leagues[]`.
These two modes are supposed to be combined together.

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
| `teams` | `TeamDef[]` | Yes | `[]` | Array of explicit team definitions |
| `structure` | `object` | No | `null` | Blueprint used when `teams` is empty |

#### TeamDef

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | `string` | **Yes** | — | Full team name |
| `short_name` | `string` | No | Auto-generated from initials | 2-3 letter abbreviation |
| `city` | `string` | **Yes** | — | City name |
| `country` | `string` | **Yes** | — | Team location / football identity code |
| `league` | `string` | No | `"<country> Premier Division"` | Domestic league label |
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

#### Structural blueprint mode

```json
{
  "version": 2,
  "description": "4 nations, 2 leagues each, 10 clubs per league",
  "structure": {
    "nations": [
      {
        "name": "England",
        "code": "ENG",
        "leagues": [
          { "name": "Premier Division", "club_count": 10 },
          { "name": "Championship", "club_count": 10 }
        ]
      }
    ]
  },
  "teams": []
}
```

Expansion behavior:
- `teams` non-empty: explicit teams are used.
- `teams` empty + `structure` present: teams are generated from the structure.
- generated teams inherit `country` from `nation.name` and `league` into `domestic_league`.

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
  "description": "A hand-crafted world with multiple countries",
  "countries": [
    {
      "code": "ENG",
      "name": "England",
      "league_names": ["England Premier Division"]
    },
    {
      "code": "ESP",
      "name": "Spain",
      "league_names": ["Spain Premier Division"]
    }
  ],
  "clubs": [
    {
      "id": "club_arsenal",
      "name": "Arsenal FC",
      "country": "England",
      "city": "London",
      "team_ids": ["team_arsenal_first", "team_arsenal_u21"]
    }
  ],
  "teams": [ /* full Team objects */ ],
  "players": [ /* full Player objects */ ],
  "staff": [ /* full Staff objects */ ]
}
```

`countries` and `clubs` are optional for backward compatibility and usage simplicity. If they are missing, they will be derived from `teams` on load.

These files are placed in:
- `<app-resources>/databases/` for bundled worlds
- `<app-data>/databases/` for user-imported worlds

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
