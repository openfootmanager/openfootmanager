# Bug-report relay (epic #569)

An anonymous player can send the evidence they reviewed to the maintainer without registering.
The relay stores it synchronously in a private inbox; it does not create a GitHub issue or send
mail, and needs no persistent queue worker. The prefilled GitHub issue form remains the permanent
offline fallback. The game's upload client is a separate follow-up.

## POST /api/v1/reports

HTTPS in production. `Content-Type: application/json` is required (`; charset=utf-8` is allowed).
`Accept: application/json` is recommended, but all `/api/*` errors use JSON even without it.
No account, cookie, CSRF token, API key or client-chosen reference code is required.

```json
{
  "schema_version": 1,
  "consent": { "upload": true, "include_save": false },
  "bundle": { "encoding": "base64", "data": "<standard padded Base64 of the exported ZIP>" }
}
```

Only these keys are accepted. `schema_version` must be the JSON integer `1`; `encoding` must be
`base64`. `consent.upload` must be the JSON boolean `true`, recorded by the client only after the
player reviews the files and agrees to upload. `include_save` is optional and defaults to false;
if present it must be a JSON boolean. A ZIP containing a save requires it to be true. True without
a save is permitted (no career may be open); actual archive presence is recorded separately.
Consent is an explicit client assertion, not proof of a UI interaction. The server does not know
the user's home directory and cannot guarantee that the client's redaction removed personal data.
Do not add identifying contact information without another consent design.

The source of the bundle format is OFM's `src-tauri/src/commands/report.rs` (`export_report_bundle`)
and `src-tauri/src/report/bundle.rs` (`write_bundle`). The requested path is absent from the game's
current checkout; the inspected source is branch `feat/report-bundle`, commit `a865ee80`, in
`.claude/worktrees/polished-orbiting-candy`. `collect_diagnostics` returns the following shape:

```json
{
  "app_version": "0.4.0",
  "os": "linux",
  "arch": "x86_64",
  "webview_version": "unknown",
  "log_directory": "~/logs",
  "crash_on_previous_run": false,
  "has_active_save": false,
  "log_files": [{ "name": "OpenFootManager.log", "bytes": 1234 }],
  "save_bytes": null
}
```

These fields describe the preview snapshot; log sizes/names and save availability can change
before export. They are not an archive manifest and need not match archive presence. Version, OS,
architecture and webview are bounded nonempty strings, flags are booleans, log entries have a name
and nonnegative byte count, and `save_bytes` is a nonnegative integer or null. Additional diagnostic
keys are allowed for forward compatibility within the metadata limit. Do not send `BundleSummary`
(`path`, `bytes`, `log_files`, `included_save`, `included_crash`): it describes a local export, not
an upload envelope, and its path belongs on the player's machine.

### ZIP entries

| Path | Required | Meaning |
| --- | --- | --- |
| `report.md` | Yes | Full redacted description/steps/expected/actual text, rendered as plain text in the inbox |
| `diagnostics.json` | Yes | Redacted `DiagnosticsReport` object above |
| `logs/<basename>.log` | No, at most 3 | Recent redacted UTF-8 log text |
| `last-crash.json` | No | Redacted previous-crash JSON object |
| `save/<basename>.db` | No, at most 1 | Unredacted SQLite career copy, only with separate save consent |

No images, extra entries, nested basenames, directory entries, absolute paths, traversal,
backslashes, duplicate names, symlinks, encryption or unsupported compression methods are accepted.
Stored and Deflate compression are accepted. Every entry is read with bounded streaming and CRC
verification. No entry is extracted to a filesystem path, opened as a database, or executed.

### Default limits

Binary MiB/KiB, inclusive ceilings. Central settings: `config/bug_reports.php`.

| Limit | Default | Environment setting |
| --- | --- | --- |
| Entire JSON request | 24 MiB (25,165,824 bytes) | `BUG_REPORT_MAX_REQUEST_BYTES` |
| Decoded ZIP | 16 MiB (16,777,216 bytes) | `BUG_REPORT_MAX_BUNDLE_BYTES` |
| Total expanded ZIP entries | 64 MiB (67,108,864 bytes) | `BUG_REPORT_MAX_EXPANDED_BYTES` |
| `report.md` | 64 KiB | fixed config value |
| `diagnostics.json` | 64 KiB | fixed config value |
| `last-crash.json` | 1 MiB | fixed config value |
| Upload attempts per IP | 3 per minute AND 10 per hour | `BUG_REPORT_RATE_PER_MINUTE`, `BUG_REPORT_RATE_PER_HOUR` |
| Upload attempts across all IPs | 100 per day | `BUG_REPORT_RATE_GLOBAL_PER_DAY` |

Limits apply before expensive ZIP validation. All POST attempts, including malformed, invalid,
oversized and storage-failed attempts, consume the rate budget. Windows start on the first attempt;
these are Laravel cache-backed rate windows, not midnight resets. `429` includes `Retry-After`
in seconds. Concurrent requests can slightly overshoot framework rate counters; these are abuse
controls, not a disk quota. Configure edge request/body/rate limits as well.

The game targets 3 logs and 16 MiB raw logs, but deliberately keeps an oversized newest log and
does not cap saves. The relay is stricter: bundles above its limits must use the offline path or
be rebuilt without the optional save. Base64 adds about one third to compressed size. Clients
should check the local ZIP size before allocating an upload and must not silently remove a save
the player chose or reuse consent for a changed bundle.

### Success

Only after the private ZIP and database record are stored:

```http
HTTP/1.1 201 Created
Content-Type: application/json
Cache-Control: no-store, private

{"code":"7K2M9Q4R"}
```

The server generates 8 cryptographically random Crockford Base32 characters
(`0123456789ABCDEFGHJKMNPQRSTVWXYZ`, 40 bits). A UNIQUE database column protects concurrent inserts;
collisions retry with a fresh code, up to five attempts. Codes do not encode database IDs or volume.
The code is an identifier, **not a credential**: there is no public lookup, status, download or
code-authorized deletion endpoint. Removal requests go to the maintainer. The later game client
records successful codes in app-data `reports.json`, not AppSettings or the rotating log, with
no status tracking. Treat an interrupted response as uncertain: retrying may create a second report;
v1 has no idempotency key. Keep the local bundle until a success response arrives.

### Errors

```json
{
  "error": {
    "code": "validation_failed",
    "message": "The report is invalid.",
    "fields": { "consent.upload": ["Explicit upload consent is required."] }
  }
}
```

`fields` exists only for 422. Messages are explanatory English text; clients translate by stable
`error.code`, not by message. Never display a request body or server trace as an error.

| HTTP | `error.code` | Cause / client action |
| --- | --- | --- |
| 400 | `invalid_json` | Malformed JSON or a non-object root; correct the envelope |
| 413 | `payload_too_large` | Request, compressed ZIP, expanded ZIP or individual entry exceeds a ceiling |
| 415 | `unsupported_media_type` | Use `application/json` |
| 422 | `validation_failed` | Consent, schema, Base64, ZIP structure/integrity or diagnostic shape is invalid |
| 429 | `rate_limited` | Wait at least `Retry-After`; offer the offline fallback |
| 503 | `service_unavailable` | Private storage unavailable, DB write failed or collisions exhausted; retry later |
| 500 | `internal_error` | Unexpected failure; use the fallback and keep the bundle |
| 403 / 404 / 405 | `forbidden` / `not_found` / `method_not_allowed` | Unsupported or disallowed API request |

Framework exceptions under `/api/*` use this envelope, including maintenance/oversized-PHP-body
errors and routing misses. Debug mode does not reveal traces to API clients. PHP warnings and errors
from the webserver/proxy before Laravel starts cannot be normalized by the application.

## Maintainer inbox

- `GET /admin/bug-reports` (`admin.bug-reports.index`): newest-first, 20 per page, reference code,
  full description, build/platform summary, actual save/crash flags, size, receipt time and download.
  Inbox responses also use `Cache-Control: no-store, private`.
- `GET /admin/bug-reports/{bugReport:code}/bundle` (`admin.bug-reports.bundle`): authenticated
  attachment download, `application/zip`, no-store, `X-Content-Type-Options: nosniff`.

Both routes require the existing `can:admin` gate, not package moderators. Existing package
moderation's `Report` and `/admin/reports` remain separate. `BugReport` records consent time and
actual archive flags; no IP address, account association, public URL or client-side status is saved.
Private files use only server-generated UUID paths on the dedicated `bug_reports` local disk:
`storage/app/bug-reports`, `serve=false`, private visibility. Keep the whole directory outside
`public_html` and all public storage links. DB and storage backups contain private diagnostics too.

## Deployment decisions before enabling the game client

1. **Hosting limits:** confirm body limits at the CDN/Apache/PHP layers, disk and temporary-directory
   quotas, execution timeout and memory budget. Start with `post_max_size >= 24M`, a webserver body
   ceiling of 24 MiB, and a measured PHP memory budget (at least 128 MiB, preferably 256 MiB).
   Test the largest incompressible and expanded bundles on the real shared host before adopting
   these defaults. JSON uses `post_max_size`, not multipart `upload_max_filesize`. Set
   `display_errors=Off`, production debug off and TLS. Raising ZIP limits also requires raising
   the JSON ceiling and reassessing memory. `ext-zip` is already a project dependency.
2. **Rate identity:** use a persistent cache (the existing database cache works without a worker).
   Never use `array`/`null` in production. Set `TRUSTED_PROXIES` to actual proxy IPs/CIDRs only; use
   `*` only if clients cannot reach the origin directly. Otherwise forwarded headers can forge IPs.
   Choose global/IP budgets for shared NAT users and set an edge rate limit. Cache outages deny
   requests with a JSON failure; requests do not bypass rate limiting.
3. **Retention:** there is deliberately no automatic deletion until a retention policy is chosen.
   Decide retention duration, user deletion procedure, backup expiry and a total disk budget.
   Daily acceptance limits do not bound lifetime storage. Eloquent deletion removes the private
   bundle before its row; direct SQL deletion bypasses cleanup. If scheduling is later required,
   use cron plus `artisan schedule:run`, not a persistent queue worker. Monitor orphan files after
   interrupted processes; normal request failures clean up their files.
4. **Admin access:** assign the existing `admin` role only to designated diagnostic maintainers;
   moderators cannot view bundles. Decide whether this should become a narrower gate as the
   admin team grows. Diagnostics can still contain personal data, especially optional saves.
   Do not copy private attachments to GitHub without the player's separate agreement.

## Staging handoff to the maintainer (HostGator)

There is no remote Git repository or automated deployment in this workflow. Changes stay on the
local `feat/bug-report-relay` branch. The maintainer transfers the reviewed application changes
to HostGator using their own SSH workflow. Contributors must never deploy, run rsync, or connect
by SSH to any host. The commands below describe maintainer prerequisites; they were not run on
staging by this work.

- **Application and assets:** transfer the changed application files and the locally built
  `public/build` assets/manifest, including the new inbox page. Preserve staging's `.env`,
  `APP_KEY`, database, uploads and other runtime data. No new Composer dependencies are introduced;
  confirm the host satisfies the existing lockfile with `composer check-platform-reqs --no-dev`,
  including PHP, ZIP, GD and the configured database driver. Keep the app outside `public_html`
  and expose only Laravel's `public` directory.
- **Migration:** back up the staging database, then run `php artisan migrate --force` from the
  application directory using the compatible PHP CLI. The new migration is
  `2026_10_03_000000_create_bug_reports_table.php`; it creates the separate `bug_reports` table
  and UNIQUE reference-code index. Existing migrations must also be current. With
  `CACHE_STORE=database`, the existing cache migration must have created `cache` and `cache_locks`.
- **Environment:** merge the following into staging's existing configuration, adjusting only
  after confirming host limits. Set `APP_ENV=production`, `APP_DEBUG=false`, the staging HTTPS
  `APP_URL`, and a persistent `CACHE_STORE` (database is supported). Leave `TRUSTED_PROXIES` empty
  for a direct origin, or set only the actual proxy IPs/CIDRs. Do not replace the existing app key
  or database credentials.

  ```dotenv
  BUG_REPORT_MAX_REQUEST_BYTES=25165824
  BUG_REPORT_MAX_BUNDLE_BYTES=16777216
  BUG_REPORT_MAX_EXPANDED_BYTES=67108864
  BUG_REPORT_RATE_PER_MINUTE=3
  BUG_REPORT_RATE_PER_HOUR=10
  BUG_REPORT_RATE_GLOBAL_PER_DAY=100
  TRUSTED_PROXIES=
  ```

- **Storage:** provision `storage/app/bug-reports` outside the webroot, writable by the PHP
  process with private permissions. Never symlink it into public storage. Ensure the existing
  `storage/framework`, `storage/logs`, `bootstrap/cache` and PHP temporary directory are writable;
  the ZIP validator uses temporary files and removes them when a request completes. Allocate
  and monitor disk space for both diagnostics and database/backup growth.
- **Configuration caches:** after transferring routes/config and merging environment settings,
  refresh cached configuration and routes with `php artisan config:cache` and
  `php artisan route:cache`; clear stale compiled views with `php artisan view:clear`.
- **Cron and workers:** this relay requires **no cron entry and no persistent queue worker**.
  `routes/console.php` on this branch adds no scheduled relay task. Retention is currently manual;
  adding `schedule:run` alone would not delete reports. If a retention policy and pruning command
  are later implemented, the maintainer can schedule that command using HostGator cron.
- **Staging checks:** designate the maintainer's existing account as `admin`, then verify a
  consented anonymous POST returns JSON 201/code; missing consent returns JSON 422; spent limits
  return JSON 429/`Retry-After`; and the administrator can view and download the private bundle.
  Confirm guests and moderators cannot access diagnostics, and private files have no direct
  web URL. Test configured maximum-size uploads on the actual host before enabling the game's
  later upload client. Delete test diagnostics through Eloquent so file cleanup runs.

## Acceptance scenarios and tests

`tests/Feature/BugReportRelayTest.php` names each test as Given/When/Then. Scenarios cover anonymous
consented persistence/reload, consent rejection and separate save consent, source-shaped logs/crash,
metadata/transport errors, every size boundary, malformed/hostile archives, collisions/retry
exhaustion, disk/DB failures, IP/global throttling, private errors in debug, public code misuse,
admin inbox/download/pagination, private inbox caching and missing files. These were written and observed failing before
implementation. No game files are changed by this relay slice.

### Local verification on the implementation branch

The clean `main` baseline is `ac5f0c2`. PHP 8.5.11 / PHPUnit 13.2.4:

| Check | Result |
| --- | --- |
| `vendor/bin/phpunit` | 170 tests / 741 assertions pass, including 41 relay GWT scenarios |
| Existing tests on clean baseline | 129 tests / 566 assertions pass |
| `npm run lint` | passes |
| `npm run build` | passes |
| `node_modules/.bin/tsc --noEmit` | passes |
| `npm run format:check` | fails on the same 17 existing files as clean `main`; new page passes |
| `vendor/bin/pint --test` | fails on the same 21 existing files/fixers as clean `main` |

Both baseline and implementation have two existing PHP 8.5 deprecations for
`PDO::MYSQL_ATTR_SSL_CA` in `config/database.php`. New PHP files and modified PHP files other
than `routes/web.php` pass scoped Pint; that route file retains its existing formatting to keep
the change small. This is local SQLite coverage, not proof of shared-host memory/timeout limits
or browser accessibility. The inbox uses semantic links, headings and focus styles; no browser
accessibility audit was run. The migration must be applied during deployment.
