# Linux graphics: WebKitGTK, NVIDIA, and why the UI was slow

Tauri renders through **WebKitGTK** on Linux. WebKitGTK's DMABuf renderer has a long history of
failing on NVIDIA's proprietary driver — blank windows, flicker on resize, Wayland protocol
errors. Working around that badly costs the accelerated compositing path, which is what happened
here.

This document records what was measured, on what, and what the app now does about it. If you are
a player hitting a blank window or a sluggish UI, skip to
[Troubleshooting](#troubleshooting-for-players).

---

## The short version

The app used to force `WEBKIT_DISABLE_DMABUF_RENDERER=1` on **every** Linux machine. That cured
blank windows and cost **13× on compositing**: any animation, transition, modal or dropdown ran at
roughly 4 frames per second while the rest of the UI was fine.

The correct fix is one level cheaper on
[Tauri's ladder](https://v2.tauri.app/develop/debug/linux-graphics/): `__NV_DISABLE_EXPLICIT_SYNC=1`
cures the same crash **and keeps hardware acceleration**.

| | compositing p50 | p95 | dropped frames |
|---|---|---|---|
| `__NV_DISABLE_EXPLICIT_SYNC=1` (DMABuf **on**) | **17 ms / 16 ms** | 18 ms / 18 ms | 1 / 0 |
| `WEBKIT_DISABLE_DMABUF_RENDERER=1` (what shipped) | **224 ms / 372 ms** | 257 / 388 ms | 14 / 9 |
| nothing set | *app does not start* | — | — |

Two independent runs, shown as `run 1 / run 2`. Frame budget on this display is 16 ms (~63 Hz),
so 17 ms is a clean 60 fps and 224 ms is about 4.5 fps.

---

## Test machine

| | |
|---|---|
| OS / session | Fedora 44, KDE Plasma 6, **Wayland** |
| GPUs | Intel HD 630 (`i915`, `/dev/dri/renderD128`) + **NVIDIA GTX 1060 Mobile** (`/dev/dri/renderD129`) — an Optimus laptop |
| NVIDIA driver | 580.173.02 (proprietary) |
| WebKitGTK | 2.52.5 |
| Displays | internal eDP (Intel) + external HDMI (NVIDIA), both connected |

Reproduce with `scripts/perf/run-matrix.sh`; the harness is `src/dev/benchUi.ts`.

---

## What was measured

`src/dev/benchUi.ts` runs three phases, chosen because they fail differently:

- **scroll** — layout and raster.
- **repaint** — many small repaints, by recolouring every visible control each frame.
- **composite** — transform and opacity only on a full-viewport layer. A working compositor
  handles this on the GPU without repainting anything, so it should be the *cheapest* phase.

That last phase is the whole diagnostic. **Only compositing collapsed** — a renderer that paints
fine but cannot composite is a renderer doing its compositing on the CPU, and that is exactly what
disabling the DMABuf renderer leaves you with.

> **Read the scroll and repaint columns with care.** In the run that produced the table below,
> both phases were weaker than intended: `phaseHover` (as it then was) dispatched synthetic
> pointer events, which cannot trigger CSS `:hover` in any engine, and the scroll phase could
> select a non-scrolling ancestor and silently record an idle baseline. Both are fixed in the
> harness now, but the numbers here predate the fix, so treat those two columns as *"nothing
> anomalous"* rather than as positive evidence that raster is healthy. **The composite column is
> unaffected** — it drives its own animation and needs no input — and it is where the 13× sits.
> Also note the benchmark runs on whatever screen is showing a few seconds after launch, in
> practice the main menu, so it compares configurations against each other rather than measuring
> the app's heaviest screens.

## Full results

| configuration | scroll p50 | hover p50 † | composite p50 | composite drop | outcome |
|---|---|---|---|---|---|
| `baseline` — nothing set | — | — | — | — | **`Error 71 (Protocol error) dispatching to Wayland display`**, app exits |
| `explicit-sync` — `__NV_DISABLE_EXPLICIT_SYNC=1` | 16 ms | 16 ms | **17 ms** | 1 | ✅ accelerated, stable |
| `render-intel` — `WEBKIT_WEB_RENDER_DEVICE_FILE=/dev/dri/renderD128` | — | — | — | — | Error 71, app exits |
| `disable-gbm` — `WEBKIT_DMABUF_RENDERER_DISABLE_GBM=1` | — | — | — | — | Error 71, app exits |
| `shipped` — `WEBKIT_DISABLE_DMABUF_RENDERER=1` | 16 ms | 16 ms | **224 ms** | 14 | ⚠️ stable, compositing on the CPU |

Figures are run 1. † The `hover` phase has since been replaced by `repaint` — see the note above;
its column, and `scroll`, carry no positive evidence in this run.

Not run: `force-shm`, `no-compositing`, `xwayland`, the second-display axis, and
`WEBKIT_DMABUF_RENDERER_BUFFER_FORMAT` (its accepted tokens were not established — a wrong value
is silently ignored with `Invalid format ... ignoring`, which would look like a result). The table
is therefore **not exhaustive**; it is enough to pick a default.

### Three things worth extracting

1. **The crash is real and the old workaround was load-bearing.** With nothing set, the app dies
   before painting. Whoever added `WEBKIT_DISABLE_DMABUF_RENDERER=1` was fixing a genuine bug.
   Simply deleting it would have regressed every NVIDIA user from "slow" to "broken".
2. **Pinning the render node does not help.** `WEBKIT_WEB_RENDER_DEVICE_FILE` pointed at the Intel
   iGPU still hits Error 71 — the protocol error comes from the Wayland/NVIDIA explicit-sync
   handshake, not from which GPU allocates buffers. Neither does `DISABLE_GBM`. Both were
   plausible on paper; both are dead ends here.
3. **Compositing is where the collapse is.** 224 ms against 17 ms for the same work is the one
   unambiguous signal in the table, and it explains why the app felt slow "everywhere" — every
   modal, dropdown, hover lift and page transition composites — while profiling the React side
   would have shown nothing wrong. Note this is a claim about compositing being *broken*, not a
   claim that raster is *healthy*: the two phases that would have shown raster problems were not
   measuring properly in this run (see the note above), so they rule nothing in or out.

---

## What the app does now

`src-tauri/src/platform/linux_graphics.rs`, before the webview is built. Profiles are selected
with the `OFM_GPU_PROFILE` environment variable:

| `OFM_GPU_PROFILE` | Behaviour |
|---|---|
| `auto` *(default)* | Detects the GPUs. On NVIDIA, sets `__NV_DISABLE_EXPLICIT_SYNC=1` and leaves the DMABuf renderer **enabled**. On AMD/Intel-only machines, sets **nothing**. |
| `safe` | The old behaviour: `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Slow, but starts on hardware where nothing else does. |
| `off` | Sets nothing at all. The measurement baseline. |

Rules the module keeps:

- **A variable you set yourself is never overridden.** `WEBKIT_DISABLE_DMABUF_RENDERER=0` is
  WebKitGTK's own opt-out (it compares the value against the string `0`), so clobbering it would
  silently ignore you.
- **Nothing is forced on hardware that never had the bug.** Previously every Linux machine, AMD
  and Intel included, was pushed onto CPU compositing.
- **The decision is logged at `Info`**, so a bug report arrives with the answer already in it.

Because `auto` now keeps the DMABuf renderer on, a machine where `__NV_DISABLE_EXPLICIT_SYNC=1`
is *not* enough would fail to start. The app therefore records a sentinel before creating the
window and clears it once the UI is alive; a launch that finds a stale sentinel falls back to
`safe` on its own. See [issue #281](https://github.com/openfootmanager/openfootmanager/issues/281),
which is a different failure in the same subsystem (`EGL_BAD_PARAMETER` rather than Error 71).

---

## Troubleshooting for players

**The window is blank, white, or the app closes immediately.**
Run it with the conservative profile:

```sh
OFM_GPU_PROFILE=safe openfootmanager
```

The UI will be noticeably less smooth, but it will start. Please open an issue with the log line
beginning `Linux graphics:` — that tells us exactly what was detected.

**The UI is sluggish — animations stutter, menus feel heavy.**
You are probably on the conservative path. Try:

```sh
OFM_GPU_PROFILE=auto openfootmanager
```

**Neither works.** Last resort, disables accelerated compositing entirely:

```sh
WEBKIT_DISABLE_COMPOSITING_MODE=1 openfootmanager
```

---

## Notes for maintainers

- **`WEBKIT_DISABLE_DMABUF_RENDERER` no longer means what it used to.** It once selected
  WebKitGTK's WPE/X11 fallback renderer, which was still accelerated. That renderer was removed
  during the 2.43 cycle — the installed 2.52 library does not link `libwpe` at all — so today the
  variable drops you to CPU compositing. Advice written before 2024 that recommends it as a
  harmless fix is out of date.
- **WebKitGTK exposes far more knobs than the ecosystem discusses.** Visible with
  `strings /usr/lib64/libwebkit2gtk-4.1.so.0 | grep WEBKIT_`:
  `WEBKIT_DMABUF_RENDERER_DISABLE_GBM`, `WEBKIT_DMABUF_RENDERER_FORCE_SHM`,
  `WEBKIT_DMABUF_RENDERER_BUFFER_FORMAT`, `WEBKIT_WEB_RENDER_DEVICE_FILE`,
  `WEBKIT_SKIA_ENABLE_CPU_RENDERING`, `WEBKIT_SKIA_{CPU,GPU}_PAINTING_THREADS`,
  `WEBKIT_FORCE_COMPOSITING_MODE`, `WEBKIT_FORCE_VBLANK_TIMER`. None of them helped here, but
  they are the search space for the next machine that misbehaves.
- **The default is chosen from one machine.** Everything above is a single GPU, one driver
  (580.173.02), one compositor (KWin/Wayland) and one WebKitGTK (2.52.5).
  `__NV_DISABLE_EXPLICIT_SYNC=1` turns off the mechanism Wayland compositors use to avoid
  frame-pacing glitches on NVIDIA; Tauri describes it as "often" free, which is a weaker claim
  than "always". It is clearly better *here* than CPU compositing, and that is the comparison the
  default turns on — but if you are on newer hardware or a newer driver, re-run the matrix rather
  than trusting this table. The cost of the default being wrong elsewhere is an app that does not
  start, which the startup marker catches and downgrades on the next launch.
- **This is not a kernel problem** and there is nothing to send upstream to Linux. The layers with
  agency are the app's env-var policy, packaging, and bug reports to WebKitGTK and Tauri.
- **Re-run the matrix after any WebKitGTK or NVIDIA driver update.** The right default is a
  moving target; `scripts/perf/run-matrix.sh` exists so that re-checking is cheap.
