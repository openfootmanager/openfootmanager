# CONTRIBUTING

Thank you for taking the time to read this and for showing your interest in supporting the project.

There are several ways to contribute, whether you are a programmer or just a fan of this type of game. Any meaningful help is appreciated. This game was born from dissatisfaction with market alternatives and was built by a football fan for football fans.

If you do not code, you can still help in many ways:

- Give us a star!
- Join the Discord server: https://discord.gg/2CXaesaukT
- Tweet about the project!
- Refer this project in your project's readme!
- Tell your friends about us!
- Share us on Facebook!
- Donate to the project _(not available yet)_
- Make a video about it!
- Play it!

## How to contribute

If you really want to help us directly, thank you very much! We have a few jobs that you might be interested in:

- **Report a problem**  
  You can report bugs or issues you encounter in the game. Open an Issue and follow the steps to report the problem. Please read carefully the bug reporting issue template before submitting a new bug report. Provide as much information as you can to help us track the bug and solve it as fast as we possibly can. If you want to discuss a problem before filing it, you can also join the Discord server: https://discord.gg/2CXaesaukT

- **Propose enhancements**  
  You can also propose new enhancements or improvements to the game. We're considering new ideas every day, and you can propose yours by opening an Issue and following the steps to propose enhancements. Just make sure to check the Issues page for similar ideas before opening up a new Issue. We don't want to flood the page with duplicated issues. Discord is also a good place for early discussion: https://discord.gg/2CXaesaukT

- **Documentation**  
  Do you think we can improve our documentation somehow? You can propose changes to the text, or write useful tutorials or examples on how to do certain things in the game.

- **Translation**  
  Localization contributions are welcome. You can help improve existing translations and add new locales for both the game UI and documentation.

- **Create new content**  
  You can propose and contribute content for the game, such as images, logos, and database improvements. Please open an Issue first so we can align scope and format.

### How work gets scheduled

Before picking something up, it helps to know how work is organised:

1. [**VISION.md**](VISION.md) says what the project is for, what it will never do, and where it is
   going. Read it first if you are proposing something new — it will tell you whether an idea fits
   before you write any code.
2. The [**roadmap index** (#11)](https://github.com/openfootmanager/openfootmanager/issues/11) lists
   every release and links to its tracking issue.
3. Each **release tracking issue** holds that release's theme, its goals, its explicit non-goals, and
   the checklist that has to pass before it ships. This is the best place to look for "what is being
   worked on right now".
4. Individual issues carry a **milestone** (the stable release they are due in) and an `area:*`
   label. Issues labelled `good first issue` are a good entry point, and `release-blocker` marks the
   ones holding up the next stable cut.

Discussion about something not yet on the roadmap belongs in
[Discussions](https://github.com/openfootmanager/openfootmanager/discussions) (the `Ideas` category)
or on Discord.

## Submitting code

The most traditional way to contribute is to submit new code. **Openfoot Manager** is a GPLv3 licensed project, read the [LICENSE.md](LICENSE.md) before submitting your code.

Your code must be GPLv3 compliant, which means you understand that any code submitted here is original or also GPL-compliant, and must not depend on patents or copyrighted third-party content. Your code is subject to a free and open source license that will be available to the entire open source community.

Once you understand that concept, you're welcome to submit new code.

### Installing dependencies

This project uses **Rust** for the backend and **Node.js/npm** for the frontend.

1. Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.
2. Ensure you have [Node.js](https://nodejs.org/) (v18+) installed.
3. Install Tauri prerequisites for your OS following the [official Tauri guide](https://v2.tauri.app/start/prerequisites/).

After cloning the repository, install the frontend dependencies:

```bash
npm install
```

To run the debug version of the project (starts both the Vite server and the Tauri app):

```bash
npm run tauri dev
```

### Understanding the code

The backend is split into multiple Rust crates:

- `domain`: Pure business logic and models.
- `engine`: Match simulation engine.
- `db`: Database access and persistence handling.
- `ofm_core`: Coordinates state, the game clock, and data flow.

The frontend is built with React, TypeScript, and TailwindCSS in the `src/` directory.

### Fork and Pull

We work with a [Fork & Pull](https://docs.github.com/en/github/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests#fork--pull) method. Fork this repo, write your code in a feature branch (make sure it is up to date with the `develop` branch), and open a **Pull Request** targeting `develop`, describing your changes and referencing the **Issue** that inspired your code when applicable.

If you're working on a new feature that has no prior **Issue** related to it, please open an **Issue** describing the feature and then reference it in your new **Pull Request**.

### Code conventions

- **Rust**:
  - Run `cargo fmt` to format your Rust code.
  - Run `cargo clippy` to catch common mistakes and improve code quality. Address all warnings before submitting a PR.
  - Use descriptive variable names and leverage Rust's strong type system.
  - Write docstrings for public functions and complex logic.

- **Frontend (TypeScript/React)**:
  - Keep components modular.
  - Use TailwindCSS for styling instead of raw CSS where possible.
  - Ensure type safety across the application (avoid `any` types).

### Tests

Whenever you add a new feature (backend or frontend), include tests to ensure it behaves as expected.

Write unit tests in the same file as your code using the `#[cfg(test)]` module, as is standard in the Rust community.

Run all relevant tests before opening a Pull Request:

```bash
npm test
```

```bash
cd src-tauri
cargo test --workspace
```

If your change affects both layers, run both test suites.

One trap worth knowing: Tauri command tests live in the `openfootmanager_lib` **lib** target. Use `cargo test --lib` to run them — `cargo test --bin` matches zero tests and exits successfully, which looks like a pass but checks nothing.

### Versioning and release streams

We follow the odd/even convention used by projects like GNOME and PCSX2:

- **Odd minor versions (`0.3.x`)** are the *unstable* stream. They live on `develop` and are
  published as nightlies.
- **Even minor versions (`0.4.x`)** are *stable* releases, cut from the `release` branch.

So a user on `0.3.x` knows they are on an unstable build just from the number.

The base version lives in three files that must stay in sync:

- `src-tauri/tauri.conf.json` (the source of truth the build reads)
- `src-tauri/Cargo.toml`
- `package.json`

**Do not bump these per build.** They only change when a release stream branches
(`0.3` → `0.4`). Everything that varies build to build — the channel, the commit hash, the
build date — is injected by `vite.config.ts` as `__APP_VERSION__`, `__APP_CHANNEL__`,
`__APP_COMMIT__` and `__APP_BUILD_DATE__`, and formatted by `formatAppVersion()` in
`src/lib/appVersion.ts`:

| build | shown in the UI and window title |
| --- | --- |
| stable | `v0.4.0` |
| nightly | `v0.3.0-nightly · f164fcd` |
| local | `v0.3.0-dev · f164fcd` |

The channel is set by CI via the `OFM_CHANNEL` environment variable; locally it defaults to
`dev`. It is deliberately not translated — it is part of a semver identifier, not prose.

Version numbers do not carry a maturity tag. There is no `0.4.0-beta` — the odd/even number already
says whether a build is stable, and "this project is pre-1.0" is said in
[VISION.md](VISION.md#what-10-means), not in every version string. Older roadmap entries used
`-alpha`/`-beta` suffixes; those are historical.

### What "stable" means

An even release is a promise, so it has a gate. Before a stable version is cut from `release`, all of
the following must hold:

- **No open `release-blocker` issues** in that release's milestone.
- **A full season plays start to finish** — create a career, play through to the end of the season,
  including a transfer window and the season rollover, without hitting a blocking defect.
- **Saves round-trip.** Save and load at several points in that playthrough, and confirm nothing is
  lost. See [`docs/SAVE_SYSTEM_DESIGN.md`](docs/SAVE_SYSTEM_DESIGN.md); a save written by a stable
  release must keep loading for the life of that stable line.
- **Translations are complete** — `src/i18n/localeCoverage.test.ts` green across all 11 locales, and
  no untranslated player-facing strings introduced since the last cut (see [Translations](#translations)).
- **The suite is green** — `npm test`, `cargo test --workspace`, and both clippy gates
  (`cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings`, and
  the same with `--features mcp`).

The full-season playthrough is currently a manual check. An automated scenario suite driving the game
through the MCP server is planned for 0.4.0, and joins this list when it lands.

Nightlies carry none of these promises, which is the entire point of having two streams: `develop` can
move fast because `release` is the thing that has to be trustworthy.

### Release workflows

- `publish-nightly` runs on every push to `develop` and upserts a single **rolling** `nightly`
  release: one entry that is replaced in place, so the releases page never fills with
  indistinguishable builds. It is never deleted up front, so a failed build leaves the last
  good nightly intact.
- `publish` runs on pushes to `release` and creates `v__VERSION__` as a non-prerelease, which
  is what gives the releases page its "Latest" badge.
- The `*-release-manifest` workflows generate the download manifest consumed by the website.
  Because upserting a release does not re-fire `release: published`, the nightly build
  dispatches `nightly-release-manifest.yml` explicitly when it finishes.

### Translations

OpenFoot Manager ships in **11 locales**. Any string a player can read must exist in all of them, not only English.

Two tests enforce this, and it is worth knowing precisely what each one catches, because between them they leave a gap:

- `src/i18n/localeCoverage.test.ts` — every locale file has every key `en.json` has, and no locale simply copies the English text. Add a key to `en.json` and stop, and CI fails here.
- `src/i18n/frontendKeyCoverage.test.ts` — every literal `t("…")` key used in `src/` exists in `en.json`, so a typo'd key fails too.

Neither catches English text hardcoded straight into a component, because it never becomes a key at all. `npm run audit:i18n` scans for those, but it is a heuristic reporter and **always exits 0** — read its output, don't rely on its exit code.

The locale list lives in `SUPPORTED_LANGUAGES` in `src/i18n/index.ts`, and the files are in `src/i18n/locales/`. On the Rust side, never emit English prose for the player: emit a translation key (like `be.error.noTeamAssigned`) and add that key to every locale file.

## Contributing with AI agents

AI coding agents are welcome here, and the repository is set up so they start with the project's actual conventions instead of guessing.

**Read these first** (your agent will pick them up automatically):

- [`CLAUDE.md`](CLAUDE.md) — commands, the project's six non-negotiable rules, and an index into `docs/`
- [`AGENTS.md`](AGENTS.md) — the same rules, for tools that don't read `CLAUDE.md`
- `src/CLAUDE.md` and `src-tauri/CLAUDE.md` — frontend and backend specifics, loaded when working in those trees

**Claude Code users** also get project skills and review agents in `.claude/`:

| Skill | Use it when |
|-------|-------------|
| `/add-ui-string` | Adding or changing any text a player can see |
| `/new-ui-surface` | Building a new component, panel, tab, or screen |
| `/add-domain-field` | Adding a field that must survive save/load |
| `/add-tauri-command` | Exposing new backend behaviour to the frontend |
| `/add-mcp-tool` | Adding a tool for AI agents that play the game |
| `/preflight` | Before opening a PR — the full local check sequence |

| Review agent | What it checks |
|--------------|----------------|
| `ofm-architecture-reviewer` | Crate boundaries, layering, save compatibility, SOLID |
| `i18n-auditor` | Untranslated strings, missing locales, translation quality |
| `ui-accessibility-reviewer` | Design tokens, theme parity, focus, keyboard, labelling |

**The bar is the same.** An AI-assisted PR is held to exactly the standards above: tests that would have failed before the change, all 11 locales, clean `cargo clippy`, and a description that explains *why*. "The agent wrote it" is not a review comment we can act on.

**A note on `.claude/settings.json`.** The shared settings pre-approve only read-only `git` inspection commands. Build and test commands are deliberately left prompting, even though approving them would be more convenient: `npm test`, `cargo test`, and `cargo clippy` all execute code from the working tree — test files, `vite.config.ts`, `build.rs`, proc macros. On a fork-and-pull project you will sometimes check out someone else's branch to review it, and a checked-in allowlist would run their code without asking you first. If you want those commands approved on your own machine, put them in `.claude/settings.local.json`, which is gitignored.

**Please disclose it.** The pull request template has a checkbox. This is a GPLv3 project, and knowing how a contribution was produced matters for licensing and for review. Read your own diff before you open the PR — you are the author, and you are vouching for it.
