# Quickstart: Your First Package

In this guide you will create a four-team league, validate it, package it into a `.ofm` file, and install it in OpenFoot Manager. It takes about 10 minutes.

You will use `ofm-cli`. If you prefer a visual tool, the in-app World Editor supports all entity types — see [PACKAGE_EDITOR.md](PACKAGE_EDITOR.md).

---

## Prerequisites

**Build the CLI from source** (requires a Rust toolchain and a clone of the repository):

```bash
cargo build -p ofm-cli --release
# Binary is at: target/release/ofm-cli
```

Or add it to your PATH permanently:

```bash
cargo install --path src-tauri/crates/ofm-cli
```

After installation, `ofm-cli --help` should print the command list.

---

## Step 1 — Scaffold the Package

```bash
ofm-cli new my-first-league --author "Your Name" --version 1.0.0 --type database
```

This creates the following structure:

```
my-first-league/
  package.json          ← world metadata (edit this)
  teams/
    teams.json          ← empty teams list (add your teams here)
  players/
    players.json        ← empty (you can ignore this for now)
  staff/
    staff.json          ← empty (coaches, scouts, physios — optional)
  confederations/
    confederations.json ← empty (uses built-in catalog)
  countries/
    countries.json      ← empty (uses built-in catalog)
  competitions/
    competitions.json   ← empty (add your league here)
  names/
    names.json          ← empty (uses built-in name pools)
```

Open `package.json`. It looks like this:

```json
{
  "schema": "world",
  "id": "my-first-league",
  "name": "My First League",
  "version": "1.0.0",
  "author": "Your Name",
  "packageType": "database",
  ...
}
```

Fill in the `name`, `description`, and `gameMinVersion` (e.g. `"0.3.0"`) fields. The `id` becomes the install key — make it unique (lowercase, hyphens only, no spaces).

---

## Step 2 — Add Teams

Open `my-first-league/teams/teams.json` and replace it with:

```json
{
  "schema": "team",
  "items": [
    {
      "id": "northshire-fc",
      "name": "Northshire FC",
      "shortName": "NSH",
      "city": "Northshire",
      "country": "ENG",
      "colors": { "primary": "#003366", "secondary": "#ffffff" },
      "playStyle": "Balanced",
      "stadiumName": "Castle Park",
      "reputationRange": [400, 650],
      "financeRange": [800000, 3000000]
    },
    {
      "id": "westbrook-united",
      "name": "Westbrook United",
      "shortName": "WBU",
      "city": "Westbrook",
      "country": "ENG",
      "colors": { "primary": "#cc0000", "secondary": "#ffffff" },
      "playStyle": "Attacking",
      "stadiumName": "Westbrook Arena",
      "reputationRange": [350, 600],
      "financeRange": [600000, 2500000]
    },
    {
      "id": "eastgate-city",
      "name": "Eastgate City",
      "shortName": "EGC",
      "city": "Eastgate",
      "country": "ENG",
      "colors": { "primary": "#0066cc", "secondary": "#ffcc00" },
      "playStyle": "Pressing",
      "stadiumName": "City Ground",
      "reputationRange": [300, 550],
      "financeRange": [500000, 2000000]
    },
    {
      "id": "southfield-rovers",
      "name": "Southfield Rovers",
      "shortName": "SFR",
      "city": "Southfield",
      "country": "ENG",
      "colors": { "primary": "#006600", "secondary": "#ffffff" },
      "playStyle": "Defensive",
      "stadiumName": "Rovers Stadium",
      "reputationRange": [250, 500],
      "financeRange": [400000, 1500000]
    }
  ]
}
```

The `country` field `"ENG"` refers to England, which is in the built-in catalog — you do not need to define it. For a full list of supported country codes, see [SCHEMA_REFERENCE.md](SCHEMA_REFERENCE.md).

---

## Step 3 — Add a Competition

Open `my-first-league/competitions/competitions.json` and replace it with:

```json
{
  "schema": "competition",
  "id": "my-first-league-cup",
  "name": "Northshire Premier League",
  "type": "League",
  "scope": "Domestic",
  "countryId": "ENG",
  "priority": 10,
  "format": {
    "kind": "LeagueTable",
    "legs": 2
  },
  "participants": {
    "explicit": [
      "northshire-fc",
      "westbrook-united",
      "eastgate-city",
      "southfield-rovers"
    ]
  },
  "seasonStartMonth": 8,
  "seasonStartDay": 1
}
```

The `participants.explicit` list must match the `id` fields of your teams exactly.

> **Tip**: You can also have the game pick teams automatically using a `selector` instead of an explicit list. See [SCHEMA_REFERENCE.md](SCHEMA_REFERENCE.md) for `TopByReputation`, `AllInCountry`, and other selector kinds.

---

## Step 4 — Validate

```bash
ofm-cli validate my-first-league
```

If everything is correct:

```
✓ Valid — 4 teams, 0 players, 1 competitions, 0 countries, 0 confederations
```

### Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `be.error.package.unknownCountry` | Team references a country that doesn't exist | Use a built-in code (e.g. `ENG`, `ES`, `DE`) or add a `country` entity with that id |
| `be.error.package.missingId` | An entity has an empty `id` field | Add a unique id string to the entity |
| `be.error.package.duplicateId` | Two entities of the same type share an id | Rename one of them |
| `be.error.competitionDef.tooFewParticipants` | `explicit` list has fewer than 2 entries | Add more teams |
| `be.error.package.invalidEntity` | JSON/YAML is malformed or missing required fields | Check the schema for required fields |

---

## Step 5 — Build the `.ofm` File

```bash
ofm-cli pack my-first-league
```

This validates first, then creates `my-first-league.ofm` in your current directory. To specify a different output path:

```bash
ofm-cli pack my-first-league --output ~/Downloads/my-first-league.ofm
```

---

## Step 6 — Install In-Game

1. Open OpenFoot Manager
2. On the main menu, click **New Game**
3. On the world selection screen, click the **+** button (Install Package)
4. Select your `.ofm` file
5. Your package appears in the world list — select it and start a new game

Alternatively, you can drop the `.ofm` file into the packages directory and restart the game. See [INSTALLING_PACKAGES.md](INSTALLING_PACKAGES.md) for the exact directory path on each OS.

---

## Next Steps

- Use `ofm-cli schema team` to see a commented template for any entity type
- Use `ofm-cli add team "Club Name" --dir my-first-league` to scaffold a new team file without writing JSON by hand
- Use `ofm-cli info my-first-league.ofm` to inspect an already-built package
- Browse the [examples/mini-league/](examples/mini-league/) directory for a working example with all files in place
- Read [SCHEMA_REFERENCE.md](SCHEMA_REFERENCE.md) for the full field reference
