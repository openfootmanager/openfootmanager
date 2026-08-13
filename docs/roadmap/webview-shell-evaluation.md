# Should we move off WebKitGTK on Linux?

**Status: parked, deliberately.** Written after the August 2026 rendering investigation
([`docs/LINUX_GRAPHICS.md`](../LINUX_GRAPHICS.md)). Revisit if the conditions at the bottom are
met.

---

## Why the question came up

Tauri renders through WebKitGTK on Linux, and WebKitGTK's DMABuf renderer has a long history of
failing on NVIDIA's proprietary driver. On the reference machine the app would not start at all
without a workaround, and the workaround that was shipped cost **13× on compositing**. That is a
bad enough experience — and a broad enough class of hardware — to ask whether the engine itself
is the problem.

It is worth being precise about what was actually wrong, because it changes the answer:
**the collapse was in compositing, and it was ours to fix.** One environment-variable choice was
pushing compositing onto the CPU — 224 ms per composited frame against 17 ms with it corrected.
Changing it restored 60 fps without touching the engine.

Note this is narrower than "WebKitGTK's rendering was fine". The two phases that would have shown
raster or layout problems were not measuring properly in that run (see the note in
`docs/LINUX_GRAPHICS.md`), so they rule nothing in or out. What the data supports is that the
problem we had was a configuration problem, not that the engine is blameless everywhere.

That does not clear WebKitGTK — the underlying NVIDIA fragility is real, poorly documented, and
ours to keep working around — but it means "replace the webview" is not the lowest-cost fix, and
was not needed to solve the reported problem.

## The options

### 1. Stay on WebKitGTK *(current, recommended)*

- **Cost:** zero. Already done.
- **Risk:** each WebKitGTK or NVIDIA release can change which workaround is correct. Mitigated by
  `scripts/perf/run-matrix.sh`, which makes re-checking cheap, and by the startup marker, which
  downgrades automatically on hardware where the fast path does not work.
- **Ceiling:** we inherit WebKitGTK's bugs and cannot fix them on our own schedule.

### 2. Chromium via CEF

- On [Tauri's radar](https://github.com/tauri-apps/wry/issues/1064), with **no ETA**. Not
  something we can adopt; something we would have to wait for or build.
- **Cost if it existed:** a ~150 MB bundled runtime against WebKitGTK's zero (it is a system
  library). For a game distributed as .deb/.rpm/AppImage that is a real regression in download
  size for every Linux user, to fix a problem that affects a subset of them.
- **Benefit:** the rendering engine most web content is actually tested against, and the one our
  frontend is developed against day to day.

### 3. Servo / Verso

- A Tauri–Igalia collaboration, still a proof of concept. Servo is years from feature parity with
  a production browser engine.
- Adopting it would mean discovering, and working around, a fresh set of engine bugs — trading a
  known problem for an unknown one. Not a serious option for a game we want people to play now.

### 4. WPE WebKit

- Non-GTK WebKit, same engine family. Would share WebKit's rendering behaviour and therefore most
  of its NVIDIA exposure, while giving up GTK's desktop integration.
- Little upside for this specific problem.

### 5. "Browser mode" — Rust backend as a local server

The cheapest escape hatch by a wide margin, and **useful on its own merits** regardless of this
question: run the existing backend as a local HTTP server and let the player open the game in
whatever browser they already have.

- Sidesteps the entire WebKitGTK question for anyone who hits it, with no bundled runtime.
- Also unlocks: playing on a tablet on the same network, easier frontend debugging, and a natural
  fit with the MCP server's existing "agents play the game" story.
- **Cost:** the IPC layer would need an HTTP transport alongside `invoke`, and file dialogs,
  the asset protocol and save-directory access would all need rethinking. Not trivial, but far
  smaller than swapping engines, and it is additive rather than a migration.

## Recommendation

Stay on WebKitGTK. The measured problem is fixed, the fix is automatic and self-healing, and
neither alternative engine is adoptable today at any price we would want to pay.

If the Linux rendering situation degrades again, the first move is **not** a new engine — it is
re-running `scripts/perf/run-matrix.sh` and checking whether a different rung of the ladder is now
correct. That is a fifteen-minute check with a documented method.

Treat **browser mode** as the real contingency, and consider it on its own merits rather than as
an escape from WebKitGTK.

## Revisit when

- Tauri ships a supported non-WebKitGTK backend on Linux (not a proof of concept), **or**
- the matrix stops finding *any* configuration that is both stable and accelerated on common
  NVIDIA hardware, **or**
- Linux crash and "blank window" reports keep arriving after the automatic fallback ships, which
  would mean the detection is not covering the real distribution of hardware.
