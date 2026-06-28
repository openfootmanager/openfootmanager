# ofm-cli Reference

`ofm-cli` is a standalone command-line tool for creating, editing, validating, and packaging `.ofm` content. It is built from the same `ofm_core` library that the game itself uses, so validation results are authoritative.

---

## Installation

**Build from source** (requires Rust ≥ 1.75):

```bash
cargo build -p ofm-cli --release
# Binary: target/release/ofm-cli
```

**Install to PATH:**

```bash
cargo install --path src-tauri/crates/ofm-cli
```

Or download a pre-built binary from the [Releases](https://github.com/sturdy-robot/openfootmanager/releases) page.

---

## Commands

### `ofm-cli new <name>`

Scaffold a new package directory with stub files and a manifest.

```bash
ofm-cli new my-league
ofm-cli new my-league --author "Your Name" --version 1.0.0 --type database --dir ./output/my-league
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `name` | Package name (used as default directory name and `name` field in the manifest) |

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--dir <path>` | `./<name>` | Where to create the package directory |
| `--author <string>` | _(empty)_ | Author name, written to manifest |
| `--version <string>` | `1.0.0` | Semantic version string |
| `--type <type>` | `database` | Package type: `database`, `patch`, or `assets` |

**Creates:**

```
<name>/
  package.json
  teams/teams.json
  players/players.json
  staff/staff.json
  confederations/confederations.json
  countries/countries.json
  competitions/competitions.json
  names/names.json
```

Each stub file has `{ "schema": "<entity>", "items": [] }` and is ready to receive entries.

---

### `ofm-cli schema <entity>`

Print an annotated JSON template for an entity type. Use this to learn the fields and constraints before writing your own files.

```bash
ofm-cli schema team
ofm-cli schema competition
ofm-cli schema world
```

**Valid entity names:** `world`, `team`, `player`, `staff`, `confederation`, `country`, `competition`, `names`

The output is a JSONC-style template (with `//` comments) — not valid JSON, but meant for reading. Copy-paste the fields you need into your own files.

---

### `ofm-cli add <entity> [name]`

Scaffold a single entity file inside an existing package directory.

```bash
# Create a new file with one entry
ofm-cli add team "FC Lisbon" --dir my-league

# Append to an existing file
ofm-cli add team "SL Benfica" --dir my-league --append-to fc-lisbon.json

# Output YAML instead of JSON
ofm-cli add player "Luis Figo" --dir my-league --format yaml
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `entity` | Entity type: `team`, `player`, `staff`, `confederation`, `country`, `competition`, `names` |
| `name` | (optional) Display name — pre-fills the `name` field and generates a slug for `id` |

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--dir <path>` | `.` (current directory) | Package directory |
| `--append-to <file>` | _(create new file)_ | Append the entry to an existing file in the entity subdirectory instead of creating a new file |
| `--format <fmt>` | `json` | Output format: `json` or `yaml` |

**Notes:**
- Without `--append-to`, a new file is created in the entity's subdirectory named after the slug (e.g. `teams/fc-lisbon.json`)
- With `--append-to`, the entry is appended to `<entity-dir>/<file>` (the file name is relative to the entity subdirectory)
- `world` is not a valid entity type for `add` — use `new` to create or edit `package.json` directly
- All fields default to documented placeholder values; edit the file afterward

---

### `ofm-cli validate <path>`

Validate a package directory or a `.ofm` archive. Reports all errors with file location and i18n code.

```bash
ofm-cli validate my-league
ofm-cli validate my-league.ofm
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `path` | Directory or `.ofm` file to validate |

**Exit codes:** `0` = valid, `1` = one or more errors

**Output examples:**

```
✓ Valid — 18 teams, 360 players, 2 competitions, 1 countries, 0 confederations
```

```
✗ 3 error(s):
  teams/bundesliga.json  be.error.package.unknownCountry (entity=fc-koeln, country=DEU)
  competitions/cups.json be.error.competitionDef.selectorMissingCountry (competition=dfb-cup)
  package.json           be.error.package.missingId (kind=world)
```

See the [Error Codes Reference](#error-codes) below for a full list of error codes.

---

### `ofm-cli pack <dir>`

Validate a package directory and, if valid, build a `.ofm` archive.

```bash
ofm-cli pack my-league
ofm-cli pack my-league --output ~/releases/my-league-1.0.0.ofm
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `dir` | Package directory to pack |

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--output <path>` | `<id>.ofm` in current directory | Output file path |

**Notes:**
- Runs `validate` first; if there are errors the archive is not created and a full error list is printed
- The default output name reads the `id` field from `package.json`
- The archive includes all files in the directory (data + assets). Non-data files (`.md`, `.txt`, images) are preserved but not parsed by the loader

---

### `ofm-cli info <file.ofm>`

Read and display metadata from a `.ofm` archive without extracting it.

```bash
ofm-cli info my-league.ofm
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `file` | Path to a `.ofm` file |

**Output:** A table with id, name, version, author, type, license, min game version, base year, description, team count, player count, and competition count.

---

## Error Codes Reference

### Package-Level Errors (`be.error.package.*`)

| Code | Cause | Parameters |
|------|-------|------------|
| `readFailed` | File could not be opened or parsed | _(file path in output)_ |
| `missingSchema` | File has no top-level `schema` field | _(file path)_ |
| `unknownSchema` | `schema` value is not recognized | `schema` |
| `invalidEntity` | Entity body is malformed or missing required fields | `schema` |
| `missingId` | Entity has an empty `id` field | `kind` (entity type) |
| `duplicateId` | Two entities of the same type share an `id` | `kind`, `id` |
| `unknownConfederation` | Country references a confederation id that does not exist | `country`, `confederation` |
| `unknownCountry` | Team or player references a country that does not exist | `entity`, `country` |
| `unknownTeam` | A player or staff member references a team `id` that does not exist | `entity`, `team` |
| `unknownRegion` | `defaultActiveRegions` lists a region id that does not exist | `id`, `field` |
| `zipSlip` | Archive contains a path traversal attack | _(entry name)_ |
| `symlinkDetected` | Archive contains a symlink | _(entry name)_ |
| `tooManyFiles` | Archive exceeds 10,000 files | — |
| `archiveTooLarge` | Uncompressed content exceeds 1 GB | — |

### Competition-Level Errors (`be.error.competitionDef.*`)

| Code | Cause |
|------|-------|
| `emptyId` | Competition has an empty `id` |
| `emptyName` | Competition has an empty `name` |
| `duplicateId` | Two competitions share an `id` |
| `unknownCountry` | `countryId` references a country that does not exist |
| `unknownRegion` | `regionId` references a region that does not exist |
| `unknownTeam` | `explicit` participant list contains an unknown team id |
| `noParticipants` | `participants` has neither `explicit` nor `selector` |
| `bothParticipantSources` | `participants` has both `explicit` and `selector` |
| `tooFewParticipants` | `explicit` list has fewer than 2 entries |
| `selectorMissingCountry` | Selector `kind` requires `country` but it is absent |
| `selectorMissingRegion` | Selector `kind` requires `region` but it is absent |
| `selectorMissingSource` | `championsOf` selector is missing `sourceCompetition` |
| `selectorCountTooSmall` | `topByReputation` selector `count` is less than 2 |
| `selectorCycle` | `championsOf` dependencies form a cycle |
| `invalidSeasonMonth` | `seasonStartMonth` is not in range 1–12 |
| `invalidSeasonDay` | `seasonStartDay` is not in range 1–31 |
| `berthRequiresLeague` | Berth rule type requires a league-format competition |
| `berthRequiresCup` | Berth rule type requires a cup-format competition |
| `berthInvalidRange` | Berth `from`/`to` values are invalid |
| `unsupportedVersion` | `formatVersion` is higher than this build supports |
