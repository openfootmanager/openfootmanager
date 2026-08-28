---
name: add-ui-string
description: Add or change any text a player can see, in every locale the game ships in. Covers the full procedure — en.json first, real translations for the rest, INTENTIONAL_SAME.json only where a term genuinely does not translate, then the two vitest gates. Use for frontend strings and for Rust-side message keys.
when_to_use: Adding a label, button, tooltip, error message, news headline, inbox message, aria-label, or any other user-visible text. Also when changing existing wording, renaming a translation key, or when localeCoverage.test.ts or frontendKeyCoverage.test.ts fails.
argument-hint: "[what the string says or the key you are adding]"
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(npx vitest run src/i18n), Bash(npx vitest run src/utils), Bash(npm run audit:i18n)
---

# Adding a user-facing string

OpenFoot Manager ships in **12 locales**. A string that exists only in English is a broken
build, not a TODO. This is the project's most frequently violated rule, so follow the steps in
order and finish with the tests.

## The 12 locales

Source of truth: `SUPPORTED_LANGUAGES` in `src/i18n/index.ts`.

| Code | Language | | Code | Language |
|---|---|---|---|---|
| `en` | English (source) | | `ru` | Russian |
| `es` | Spanish | | `pt-BR` | Brazilian Portuguese |
| `pt` | Portuguese | | `zh-CN` | Simplified Chinese |
| `fr` | French | | `cs` | Czech |
| `de` | German | | `tr` | Turkish |
| `it` | Italian | | `id` | Indonesian |

Files: `src/i18n/locales/<code>.json`.

If `SUPPORTED_LANGUAGES` and this table ever disagree, `src/i18n/index.ts` wins — read it. It has
grown before and will again: `id` was the twelfth, added in August 2026.

The rest of the repository's docs say "every locale" rather than a number, on purpose: when `id`
was added, a dozen files were left claiming eleven. Keep it that way.

---

## Procedure

### 1. Find the right key, don't invent a new one

```bash
# Is this string, or something close, already translated?
grep -rn "the exact english text" src/i18n/locales/en.json
```

Keys are nested and namespaced by feature (`squad.*`, `tactics.*`, `transfers.*`, `news.*`,
`settings.*`). Put the new key where its siblings live. Reusing an existing key beats adding a
near-duplicate — but do **not** reuse a key across contexts where a translator would need
different wording (a noun label and a button verb are different keys even when English collapses
them).

### 2. Add it to `en.json` first

English is the source. Every other locale is validated against its key set.

Use interpolation for anything dynamic — never build a sentence by concatenating translated
fragments, because word order differs by language:

```jsonc
// wrong — unassemblable in German or Turkish
"signedFor": "signed for",
// right
"signedFor": "{{player}} signed for {{team}} for {{fee}}"
```

Pluralisation uses i18next suffixes (`_one`, `_other`, and the extra forms `ru` and `cs` need).
If a count is involved, check how an existing pluralised key in `en.json` is written and match it.

### 3. Translate into the other 11 — properly

Add the same key path to `cs`, `de`, `es`, `fr`, `id`, `it`, `pt`, `pt-BR`, `ru`, `tr`, `zh-CN`.

- **Keep every interpolation placeholder identical.** `{{player}}` stays `{{player}}`; only the
  surrounding text and the word order change.
- `pt` and `pt-BR` are genuinely different — European vs Brazilian vocabulary (*relvado* vs
  *gramado*, *equipa* vs *time*). Don't copy one into the other.
- Football has established vocabulary in each language. Use the term a fan of that language would
  use, not a literal translation of the English.
- `zh-CN` is Simplified Chinese. The font stack in `src/App.css` has CJK fallbacks — don't remove
  them.
- If you genuinely cannot produce a confident translation for a locale, say so in your summary
  rather than shipping English text under a non-English key. The test will catch it anyway.

### 4. Match how the file already addresses the manager

Most of these languages choose between a familiar and a polite second person — `du`/`Sie`,
`tu`/`vous`, `ty`/`vy`, `kamu`/`Anda`, 你/您 — and that choice is not yours to make one string at a
time. Each file has already made it. A string in the wrong register reads to a native speaker the
way a stranger using your first name does: not wrong, exactly, but written by someone who wasn't
paying attention.

**Read four or five neighbouring values in the namespace you are editing, and copy their form.**

Do not instead count pronouns across the whole file and follow the majority. Those counts are
dominated by third-person text *about* players rather than text *to* the manager, and the markers
are ambiguous in both directions: Spanish `su` is "his" far more often than polite "your", and
German `Sie` is also "they" and "it".

Register follows **who is speaking**, which is why one file holds both forms correctly. In
`de.json`, a journalist's question under `match.press.*` and a letter from the board under
`be.msg.*` both use `Sie`; a menu label, and the dialogue options the manager picks under
`be.msg.playerEvent.options.*`, use `du`. Neither is a bug.

⚠️ **A polite form can force you to guess the manager's gender.** Formal Czech takes a plural
auxiliary but keeps the participle singular and gendered: `uspořádal jste` says the manager is a
man, `uspořádala jste` says she is a woman, and the game does not know. Rephrase so that nobody is
the subject — `Tisková konference dnes už proběhla`, *a press conference has already taken place
today*. Any locale that agrees a verb or adjective with the person being addressed can spring this.

### 5. `INTENTIONAL_SAME.json` — only for terms that truly don't translate

`src/i18n/INTENTIONAL_SAME.json` allowlists keys whose value is legitimately identical to English:
proper nouns, competition names, position abbreviations like `GK`. Entries are keyed by locale
code, or `global` for all of them.

This is an escape hatch for linguistics, **not** for unfinished work. If you find yourself adding
several keys at once, you are using it wrong.

### 6. Backend strings are keys, not prose

Rust never emits English text for the player. It emits a **translation key**, and the frontend
resolves it:

- `src/utils/backendI18n.ts` — the main mapping
- `src/utils/backendI18nPlayerEvents.ts` — player event messages
- `src/utils/backendI18n.legacy.ts` — keys kept for old saves

So a new inbox message or news headline generated in `ofm_core` means: emit the key on the Rust
side, map it in `backendI18n.ts` if the mapping isn't automatic, and add the key to every locale
file. `src/utils/backendI18n.localeCoverage.test.ts` covers this half.

### 7. Run the gates

```bash
npx vitest run src/i18n        # localeCoverage + frontendKeyCoverage + index
npx vitest run src/utils       # backendI18n coverage, if you touched backend keys
```

- `localeCoverage.test.ts` — every locale has every `en.json` key, and no locale silently copies
  the English string (outside `INTENTIONAL_SAME.json`).
- `frontendKeyCoverage.test.ts` — every literal `t("…")` key in `src/` exists in `en.json`. It
  parses the TypeScript AST, so typo'd keys fail too.

Then the advisory sweep:

```bash
npm run audit:i18n
```

**This command always exits 0.** It is a heuristic reporter over both `src/` and `src-tauri/`;
read its output and check whether any candidate it lists is a string you just added. A clean run
is not a pass — the vitest gates are.

---

## Checklist

- [ ] Key added to `src/i18n/locales/en.json`, in the right namespace
- [ ] Real translations added to all 11 other locales
- [ ] Interpolation placeholders identical across every locale
- [ ] `pt` and `pt-BR` translated separately
- [ ] Form of address matches the neighbouring keys, and no wording assumes the manager’s gender
- [ ] `INTENTIONAL_SAME.json` touched only for genuinely untranslatable terms
- [ ] Backend keys mapped in `src/utils/backendI18n*.ts` if applicable
- [ ] `npx vitest run src/i18n` green
- [ ] `npm run audit:i18n` output read, not just run
- [ ] Any new `aria-label` uses a translated string, not a hardcoded one
