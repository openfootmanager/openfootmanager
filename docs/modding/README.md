# OpenFoot Manager — Modding Guide

OpenFoot Manager supports community-created content through the `.ofm` package format. You can create your own leagues, teams, players, and competitions and share them with other players.

---

## What is a `.ofm` Package?

A `.ofm` file is a standard ZIP archive containing JSON or YAML data files and optional image assets. When installed, the game reads these files and makes your content available as a playable world.

A package has three types:

| Type | Use case |
|------|----------|
| `database` | A complete world — leagues, teams, players, competitions |
| `patch` | Partial overlay on top of another world (roster updates, additions) |
| `assets` | Art-only — team logos, kit colors, without changing gameplay data |

---

## The Entity Model

Packages are built from eight entity types. They reference each other by `id`:

```
confederations
    └── countries (reference confederation id)
         └── teams (reference country id)
              └── players (reference team id + country id; youth players set youth: true)
                   staff   (reference team id; coaches, scouts, physios)
                   competitions (reference team ids, country ids, region ids)
names (name pools keyed by country code, for random player name generation)
```

The loader validates all cross-references. If a team references `country: "XYZ"` and no country with `id: "XYZ"` exists in the package **or** in the built-in catalog, validation fails with a clear error message.

### Built-in Catalog

You do not have to redefine everything. The game ships with:
- All standard country codes (`ENG`, `ES`, `DE`, `FR`, `IT`, `PT`, `BR`, `AR`, `NL`, and many more)
- Standard confederation ids (`europe`, `south-america`, `north-america`, `africa`, `asia`, `oceania`)

Your package can reference these ids directly without defining them. You only need to define your own confederations or countries if you are inventing fictional ones.

---

## File Discovery Rules

The loader walks your package directory recursively and classifies files by their `schema` field — **not by directory name or file name**. This means:

- You can organize files however you like
- A file can contain **one entity** (fields at the top level) or **many entities** in an `items` array
- JSON (`.json`) and YAML (`.yaml`, `.yml`) are both supported and can coexist in one package

```json
{ "schema": "team", "id": "my-club", "name": "My Club FC", ... }
```

```json
{
  "schema": "team",
  "items": [
    { "id": "club-a", "name": "Club A", ... },
    { "id": "club-b", "name": "Club B", ... }
  ]
}
```

The only exception is translation files (`translations.{locale}.json`), which are named by convention and are not entity files.

---

## Choosing a Tool

| | CLI (`ofm-cli`) | World Editor (in-app) |
|---|---|---|
| **Best for** | technical users, batch authoring, scripting | visual editing, non-technical authors |
| **Supports** | all entity types | all entity types (incl. staff & youth) |
| **Output** | files on disk → `.ofm` | files on disk → `.ofm` |
| **Requires** | terminal | running OFM installation |

You can mix both tools: use the World Editor to create and edit teams visually, then use the CLI to add competitions and validate.

---

## Documentation Index

| Guide | What it covers |
|-------|----------------|
| [QUICKSTART.md](QUICKSTART.md) | Build and install your first package in 10 minutes |
| [CLI_REFERENCE.md](CLI_REFERENCE.md) | All `ofm-cli` commands, flags, and examples |
| [PACKAGE_EDITOR.md](PACKAGE_EDITOR.md) | In-app World Editor walkthrough |
| [SCHEMA_REFERENCE.md](SCHEMA_REFERENCE.md) | Every entity type with full field documentation |
| [INSTALLING_PACKAGES.md](INSTALLING_PACKAGES.md) | How to install and manage `.ofm` files |
| [examples/mini-league/](examples/mini-league/) | Fully working 4-team example package |
| [examples/academy-showcase/](examples/academy-showcase/) | Club authored end to end: players, youth, staff, and a name pool |
