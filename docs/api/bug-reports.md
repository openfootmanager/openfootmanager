# OFMHub bug-report endpoint: client contract

What the game sends to OFMHub's bug-report endpoint, and every answer it can get back. Written from
the server code on `feat/launch-prep` at `85109e3` (`app/Http/Controllers/Api/BugReportController.php`,
`app/Http/Middleware/RequireGameVersion.php`, `app/Services/BugBundleValidator.php`,
`config/openfoothub.php` → `bug_reports`). The game client in `src-tauri/src/report/hub.rs` implements it.

## Hosts

| Environment | Base URL | State (2026-10-09) |
|---|---|---|
| Production | `https://openfootmanager.com/api/v1` | deployed, **switched off** (`enabled: false`) |
| Staging | `https://staging.openfootmanager.com/api/v1` | deployed, switched off; the rest of staging sits behind HTTP basic auth, `/api/` does not |
| Planned | `https://api.openfootmanager.com/api/v1` | not created yet; meant to be the hostname compiled into shipped builds, so the marketing site can move without breaking them |

Every response under `/api/` is JSON, including errors from the framework (404, 405, 413, 429) and
unexpected failures. The one exception is a body over the web server's own ceiling (see 413 below).

There is no CORS: no origin is granted `Access-Control-Allow-Origin`. Send from native code (Rust),
not from `fetch` in the webview.

## 1. `GET /bug-reports/config`

Call it when the report dialog opens. Never hard-code the limits; they come from here.

```json
{
  "enabled": true,
  "max_total_bytes": 26214400,
  "max_bundle_bytes": 20971520,
  "max_screenshot_bytes": 5242880,
  "max_package_bytes": 20971520,
  "max_files": 6,
  "accepted_types": {
    "bundle": ["zip"],
    "screenshots": ["png", "jpg", "jpeg", "webp"],
    "packages": ["ofm"]
  }
}
```

- `enabled: false` is the kill switch, flipped on the server without a deploy. Don't offer
  sending; go straight to the fallback (save the bundle locally, open the prefilled GitHub form).
- It needs no header. It's rate-limited to **60 per minute per address**. Beyond that you get a 429
  with the error body below and a `Retry-After` header. It has its own counter, so reading the
  config never uses up the report allowance.

## 2. `POST /bug-reports`

### Request

| Header | Required | Value |
|---|---|---|
| `X-OFM-Version` | **yes** | The game version. Must match `^\d{1,4}\.\d{1,4}\.\d{1,4}(?:[-+][0-9A-Za-z.+-]{1,17})?$`: `0.2.0`, `0.3.0-nightly.20260921`, `1.0.0+abc123`. Stored as the report's game version. |
| `Content-Type` | yes | `multipart/form-data; boundary=…` |
| `Accept` | no | Errors are JSON regardless. |

No account, cookie, CSRF token or API key.

The header is checked **before** the rate limit. A request without it gets 400 and isn't counted
against the sender's allowance.

### Multipart parts

| Part name | Count | Content | Limit | Notes |
|---|---|---|---|---|
| `report` | 0–1 | A JSON **object**, sent as a plain text field or as a file part (either works) | 64 KiB (65,536 bytes) raw; nesting depth 32 | Absent or empty means `{}`. A JSON array or scalar → 422 `invalid`. |
| `bundle` | 0–1 | ZIP | 20 MiB (20,971,520 bytes) | Logs, `diagnostics.json`, and the save when the player opted in. **The save goes inside the bundle, never as its own part.** |
| `screenshots[]` | 0–n | PNG, JPEG or WebP | 5 MiB (5,242,880 bytes) each | Re-encoded to WebP on the server, which strips EXIF. A file that isn't really an image → 422 `invalid`. |
| `packages[]` | 0–n | `.ofm` files | 20 MiB (20,971,520 bytes) each | Stored unopened. Not validated as packages, so a broken one is still accepted as evidence. |

Across all parts:

- **at most 6 files** in total (bundle, screenshots and packages together); more → 422 `too_many_files`;
- **at most 25 MiB (26,214,400 bytes)**, counted as the sum of the file parts; more → 413 `too_large`.

A file's kind comes from the part name, never from its file name or declared MIME type.

#### `report` fields

All optional, all nullable. Unknown keys are ignored.

| Key | Type | Max |
|---|---|---|
| `category` | string | 32 chars |
| `what_happened` | string | 10,000 chars |
| `expected` | string | 10,000 chars |
| `steps` | string | 10,000 chars |
| `frequency` | string | 32 chars |
| `game_context` | string | 10,000 chars |
| `locale` | string | 16 chars |
| `os` | string | 32 chars |
| `arch` | string | 16 chars |
| `webview_version` | string | 64 chars |
| `diagnostics` | object | 64 KiB once re-encoded as JSON |

#### `bundle` rules

The server checks these with the bundle already uploaded. Any violation → 422 `invalid`, with the
reasons listed under `errors.bundle`.

- It must be a readable ZIP of **at most 100 entries**, expanding to **at most 150 MiB
  (157,286,400 bytes)** in total. The bytes actually read are what's measured, not the declared sizes.
- File extensions must be one of `.log`, `.txt`, `.json`, `.db`. Directories are allowed.
- No absolute paths, backslashes, NUL bytes or empty path segments. No segment may **start with a
  dot**, which rules out `..` as well as hidden files like `.DS_Store`. Windows-reserved names are
  refused too. No case-insensitive duplicates.

### Success

**201 Created**, for a new report:

```json
{ "reference": "OFM-7K2QX9DM" }
```

**200 OK** has the same body: it's the reference of a report the server **already accepted**. You
get it when a request from the same address, with the same fields and the same files byte for byte,
arrives within 24 hours. So a retry after a timeout is safe: it returns the first report instead of
making a duplicate. Treat 200 and 201 alike.

#### The reference code

- The format is `OFM-` plus **8 Crockford Base32 characters**: `0-9` and `A-Z` without `I L O U`.
  Regex: `^OFM-[0-9A-HJKMNP-TV-Z]{8}$`. The `OFM-` prefix is part of the code; show it as is.
- It's drawn by the server with `random_int`. It's never derived from the row ID, and the row ID is
  never sent.
- It's stored in a **UNIQUE** column. If a code collides at insert, the server draws a fresh one
  and retries, so the client never sees the collision.
- It's an **identifier, not a credential.** There's no endpoint that reads, checks, updates or
  deletes a report by its reference.
- **What to show the player:** the code, plus the message that the report can be deleted on request
  by emailing `bugreport@openfootmanager.com` with it. That's what the privacy page says.

### Errors

Every error has this body:

```json
{ "message": "Human-readable English, for logs — do not show it untranslated", "code": "stable_machine_code" }
```

A validation failure (422 `invalid`) also lists the reasons per field:

```json
{
  "message": "The report must be a JSON object.",
  "errors": { "report": ["The report must be a JSON object."] },
  "code": "invalid"
}
```

Branch on `status` and `code`, never on `message`.

| Status | `code` | When | What the client should do |
|---|---|---|---|
| 400 | `version_required` | `X-OFM-Version` is missing or isn't a version | A bug in the client: don't retry; fall back |
| 404 | `not_found` | Wrong path | Fall back |
| 405 | `method_not_allowed` | Wrong method | Fall back |
| 413 | `too_large` | The file parts total over 25 MiB, **or** the whole request is over PHP's `post_max_size` (56 MiB on the host) | Drop attachments (screenshots or packages first) and resend, or fall back |
| 413 | *(HTML, not JSON)* | The request is over the web server's own ceiling (60 MB, `LimitRequestBody`); this is refused before Laravel runs | Treat any non-JSON response as "relay unavailable" → fall back. Staying within the 25 MiB cap never reaches it. |
| 422 | `invalid` | A field is too long or the wrong type, `report` isn't a JSON object, a file part is over its own limit, the bundle breaks a rule above, or a screenshot isn't an image | Show "the report could not be accepted"; fall back. Don't retry unchanged |
| 422 | `too_many_files` | More than 6 files | Trim, then resend |
| 422 | `blocked` | A file's SHA-256 is on the server's blocklist | Fall back; don't retry |
| 429 | `rate_limited` | **Route throttle:** over 10 POSTs per hour from this address. Every POST that has a valid version header counts, accepted or not | Wait for `Retry-After` (seconds) or fall back. The header is always present |
| 429 | `rate_limited` | **Daily cap:** this address already has 5 accepted reports in the last 24 hours (a rolling window, not a reset at midnight). A retry of an already-accepted report is answered 200 and not refused | Wait for `Retry-After`: the seconds until the oldest of those reports leaves the window. The header is always present |
| 503 | `disabled` | The kill switch is on (`enabled: false`) | Fall back now; don't retry |
| 503 | `capacity` | Global ceilings: 200 reports in the last 24 hours from everyone, or 2 GiB of stored files | Fall back now; don't retry. Nothing the sender did caused it |
| 503 | `unavailable` | The server can't process screenshots (image library missing) | Resend without screenshots, or fall back |
| 5xx | `server_error` | Anything unexpected | Fall back; a later retry is allowed |

Network failure, a timeout, or any non-JSON body means "relay unavailable": fall back.

A client that **timed out** on a POST doesn't know whether the report arrived. Retrying the **identical**
request is safe, because the 24-hour dedupe returns the original reference with 200.

## Retention and access (for the privacy wording in the game)

- Reports and their files are **kept until handled**, and nothing is deleted on a timer. Once a
  report is handled, an admin may delete its files or the whole report. That deletion is a
  deliberate, audit-logged action.
- The sender's IP is stored only as an HMAC-SHA256 keyed with the server's secret, never in plain.
  It's used for the per-address caps and dedupe.
- **Every OFMHub admin** may download a report's files: the bundle, the save and packages.
  Moderators triage the text and can view the re-encoded screenshots, but can't download files.
- An admin can file a report as a public GitHub issue. Only the player's own words go public, plus
  an environment line (OS and architecture, WebView version, locale) and the reference code. Never
  logs, diagnostics, saves, packages or screenshots.
