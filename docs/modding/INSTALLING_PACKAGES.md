# Installing Packages

This guide is for players who want to install community-created `.ofm` packages.

---

## In-Game Installation (Recommended)

1. Launch OpenFoot Manager
2. Click **New Game** on the main menu
3. On the world selection screen, click the **+** (Install Package) button
4. Navigate to and select the `.ofm` file
5. The package appears in the world list immediately

The in-game installer copies the `.ofm` file to the correct directory automatically.

---

## Manual Installation

You can also drop a `.ofm` file directly into the packages directory and restart the game.

**Directory locations:**

| OS | Path |
|----|------|
| **Linux** | `~/.local/share/openfootmanager/packages/` |
| **macOS** | `~/Library/Application Support/openfootmanager/packages/` |
| **Windows** | `%APPDATA%\openfootmanager\packages\` |

Create the `packages/` directory if it does not exist, place the `.ofm` file inside, then restart the game.

---

## Inspecting a Package Before Installing

Before installing an unfamiliar `.ofm` file, use `ofm-cli info` to see its contents:

```bash
ofm-cli info some-package.ofm
```

Output:

```
┌─────────────────────┬───────────────────────────────────┐
│ id                  │ german-bundesliga-2026             │
│ name                │ German Bundesliga 2026             │
│ version             │ 2.1.0                              │
│ author              │ Community Pack Authors             │
│ type                │ database                           │
│ license             │ CC-BY-4.0                          │
│ min game version    │ 0.3.0                              │
│ base year           │ 2026                               │
│ teams               │ 36                                 │
│ players             │ 720                                │
│ competitions        │ 4                                  │
└─────────────────────┴───────────────────────────────────┘
```

---

## Multiple Packages and Merging

You can have multiple `.ofm` packages installed at once. When you start a new game and select a world, you choose one package. The game does **not** automatically merge packages.

However, packages can be designed to work together. A `patch` package can overlay entities on top of a `database` package — if you load them both in the same game setup (a future feature), the engine merges them with last-wins-by-id semantics.

For now, each new game uses exactly one installed package.

---

## Removing a Package

Delete the `.ofm` file from the packages directory and restart the game. The package will no longer appear in the world selector.

**Existing saves are not affected** — a save file embeds the world data at save time, so deleting the package does not break ongoing careers.

---

## Troubleshooting

**Package doesn't appear after installation**
- Make sure the file has a `.ofm` extension (not `.ofm.zip`)
- Restart the game if you placed the file manually
- Check that `gameMinVersion` in the package is ≤ your current game version

**"Invalid package" error when starting a new game**
- The package may be corrupt; try re-downloading it
- Use `ofm-cli validate <path>` to check if the package is structurally valid

**Game crashes when loading a world**
- Enable debug logging and check the log file for details
- Report the issue on GitHub with the package file attached
