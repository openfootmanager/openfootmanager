/**
 * UI rendering benchmark — development tooling, never part of a normal run.
 *
 * Loaded only when the URL carries `?ofmbench`, via a dynamic import in `main.tsx`, so it stays
 * out of the app's main chunk.
 *
 * The point of this file is to answer one question with a number instead of an impression: is the
 * app slow because of WebKitGTK's compositing path, or because of our own React work? The same
 * script runs in Chromium and in the Tauri webview against the same dev server, so the two are
 * directly comparable. See `docs/LINUX_GRAPHICS.md`.
 *
 * Three phases, chosen because they fail differently:
 *
 * - `scroll`    — layout and raster. Slow here means painting is expensive.
 * - `repaint`   — many small repaints, by recolouring every visible control each frame.
 * - `composite` — transform/opacity only, which a working compositor handles on the GPU without
 *                 repainting anything. Slow *here specifically* is the signature of a broken or
 *                 software compositing path, which is exactly the WebKitGTK/NVIDIA failure mode.
 *
 * Known limitation: the run starts a few seconds after mount, so it measures whatever screen is
 * showing — in practice the main menu, which is lighter than a squad list or a live match. That
 * is fine for comparing two rendering configurations against each other, which is what this is
 * for, and misleading if read as "how heavy is the app".
 */

export interface PhaseResult {
  name: string;
  frames: number;
  durationMs: number;
  fps: number;
  /** Frame-to-frame deltas in milliseconds. */
  p50: number;
  p95: number;
  p99: number;
  max: number;
  /** Frames that took longer than 1.5x the measured display frame budget. */
  dropped: number;
}

export interface BenchResult {
  label: string;
  userAgent: string;
  /** Measured display refresh interval in ms, from an idle warm-up. */
  frameBudgetMs: number;
  devicePixelRatio: number;
  viewport: { width: number; height: number };
  phases: PhaseResult[];
  /** Long-animation-frame entries, where the browser supports the API (WebKit may not). */
  longAnimationFrames: number;
  startedAt: string;
}

const QUANTILES = { p50: 0.5, p95: 0.95, p99: 0.99 } as const;

function quantile(sorted: number[], q: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.floor(q * sorted.length));
  return Math.round(sorted[index] * 100) / 100;
}

/** Collects frame-to-frame deltas until `stop()` is called. */
function recordFrames(): { stop: () => number[] } {
  const deltas: number[] = [];
  let previous = performance.now();
  let running = true;

  const tick = (now: number) => {
    if (!running) return;
    deltas.push(now - previous);
    previous = now;
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);

  return {
    stop: () => {
      running = false;
      return deltas;
    },
  };
}

function summarise(name: string, deltas: number[], frameBudgetMs: number): PhaseResult {
  // The first delta straddles the setup work rather than the interaction, so drop it.
  const samples = deltas.slice(1);
  const sorted = [...samples].sort((a, b) => a - b);
  const durationMs = samples.reduce((total, delta) => total + delta, 0);
  const threshold = frameBudgetMs * 1.5;

  return {
    name,
    frames: samples.length,
    durationMs: Math.round(durationMs),
    fps: durationMs > 0 ? Math.round((samples.length / durationMs) * 1000 * 10) / 10 : 0,
    p50: quantile(sorted, QUANTILES.p50),
    p95: quantile(sorted, QUANTILES.p95),
    p99: quantile(sorted, QUANTILES.p99),
    max: Math.round(Math.max(0, ...samples) * 100) / 100,
    dropped: samples.filter((delta) => delta > threshold).length,
  };
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Block until the window is actually on screen.
 *
 * Browsers throttle — and eventually freeze — `requestAnimationFrame` in background tabs, so a
 * run started while hidden measures the throttling, not the renderer. Chromium in particular
 * stops firing frames entirely, which reads as a hung benchmark.
 */
function waitForVisible(onWaiting: () => void): Promise<void> {
  const ready = () => document.visibilityState === "visible";
  if (ready()) return Promise.resolve();

  onWaiting();
  return new Promise((resolve) => {
    const check = () => {
      if (!ready()) return;
      document.removeEventListener("visibilitychange", check);
      // One extra beat so the compositor has resumed before we start timing.
      setTimeout(resolve, 500);
    };
    document.addEventListener("visibilitychange", check);
  });
}

/** A small fixed banner so a human driving this by hand can see what is happening. */
function statusBanner() {
  const el = document.createElement("div");
  el.setAttribute("data-ofm-bench", "status");
  Object.assign(el.style, {
    position: "fixed",
    top: "0",
    left: "0",
    right: "0",
    zIndex: "2147483646",
    padding: "6px 12px",
    font: "600 13px/1.4 monospace",
    color: "#001b10",
    background: "#ffd60a",
    textAlign: "center",
    pointerEvents: "none",
  } satisfies Partial<CSSStyleDeclaration>);
  document.body.appendChild(el);
  return {
    set: (text: string) => {
      el.textContent = text;
    },
    remove: () => el.remove(),
  };
}

/**
 * Infer the display's frame interval from the fastest frames actually observed.
 *
 * An idle warm-up looks like the obvious way to get this, and it is what this did at first — but
 * it samples while the app is still settling and the compositor is throttled, which produced a
 * "budget" of 290 ms on a 60 Hz panel and made every dropped-frame count meaningless. The fastest
 * decile across the whole run is a far better proxy for the refresh interval: whatever else the
 * renderer struggles with, the frames it *does* deliver on time reveal the display's cadence.
 */
function inferFrameBudget(allDeltas: number[]): number {
  if (allDeltas.length === 0) return 16.67;
  const sorted = [...allDeltas].sort((a, b) => a - b);
  // Floor at 4 ms so a burst of coalesced callbacks cannot imply a 500 Hz display.
  return Math.max(4, quantile(sorted, 0.1));
}

/**
 * The largest element that actually scrolls, falling back to the document.
 *
 * The computed-overflow check is essential, not defensive. Overflowing ancestors nest, so an
 * `overflow: visible` wrapper always reports *more* overflow than the real scroll container
 * inside it and would win a naive size comparison — and writing `scrollTop` to it is silently
 * discarded, so the phase would record an idle baseline under the label "scroll".
 */
function findScrollTarget(): Element {
  const scrolls = (element: Element) => {
    const overflowY = getComputedStyle(element).overflowY;
    return overflowY === "auto" || overflowY === "scroll" || overflowY === "overlay";
  };

  let best: Element | null = null;
  let bestOverflow = 0;
  for (const element of document.querySelectorAll("*")) {
    const overflow = element.scrollHeight - element.clientHeight;
    if (overflow > bestOverflow && overflow > 200 && scrolls(element)) {
      bestOverflow = overflow;
      best = element;
    }
  }

  const root = document.scrollingElement ?? document.documentElement;
  return best ?? root;
}

/** A phase's raw frame deltas, summarised once the whole run has established a frame budget. */
interface RawPhase {
  name: string;
  deltas: number[];
}

async function phaseScroll(): Promise<RawPhase> {
  const target = findScrollTarget();
  const distance = Math.max(1, target.scrollHeight - target.clientHeight);
  const durationMs = 3000;

  const recorder = recordFrames();
  const start = performance.now();
  await new Promise<void>((resolve) => {
    const step = () => {
      const elapsed = performance.now() - start;
      const progress = Math.min(1, elapsed / durationMs);
      // Triangle wave: scroll down then back up, so we exercise both directions.
      const position = progress < 0.5 ? progress * 2 : (1 - progress) * 2;
      target.scrollTop = position * distance;
      if (progress >= 1) {
        resolve();
        return;
      }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  });

  return { name: "scroll", deltas: recorder.stop() };
}

/**
 * Force many small repaints across the visible UI.
 *
 * This started out dispatching synthetic `pointerover`/`pointerenter` at each control to emulate
 * hovering. That measured nothing: CSS `:hover` is driven by real input and no engine updates it
 * from a scripted event, so the app's Tailwind `hover:` styling never applied and the phase
 * recorded an idle baseline. Changing an inline colour is the honest way to make the renderer
 * repaint a lot of small boxes, which is what the phase was always meant to cost.
 */
async function phaseRepaint(): Promise<RawPhase> {
  const controls = Array.from(
    document.querySelectorAll<HTMLElement>("button, a, [role='button'], [role='tab'], input"),
  ).filter((element) => {
    const rect = element.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  });

  const recorder = recordFrames();

  if (controls.length === 0) {
    await wait(2000);
    return { name: "repaint", deltas: recorder.stop() };
  }

  const original = controls.map((element) => element.style.backgroundColor);
  const durationMs = 3000;
  const start = performance.now();

  try {
    await new Promise<void>((resolve) => {
      const step = () => {
        const elapsed = performance.now() - start;
        if (elapsed >= durationMs) {
          resolve();
          return;
        }
        // Alternate every element's background each frame — a genuine repaint, no layout.
        const on = Math.floor(elapsed / 16) % 2 === 0;
        for (const element of controls) {
          element.style.backgroundColor = on ? "rgba(16,185,129,0.25)" : "rgba(255,214,10,0.25)";
        }
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    });
    return { name: "repaint", deltas: recorder.stop() };
  } finally {
    recorder.stop();
    controls.forEach((element, index) => {
      element.style.backgroundColor = original[index];
    });
  }
}

/**
 * Animate transform and opacity on a full-viewport overlay. These are the two properties a
 * compositor can animate without repainting, so on a healthy GPU path this should be the
 * *cheapest* phase. If it is the most expensive one, compositing is not being accelerated.
 */
async function phaseComposite(): Promise<RawPhase> {
  const overlay = document.createElement("div");
  overlay.setAttribute("data-ofm-bench", "composite");
  Object.assign(overlay.style, {
    position: "fixed",
    inset: "0",
    zIndex: "2147483647",
    pointerEvents: "none",
    background: "linear-gradient(135deg, rgba(16,185,129,0.35), rgba(255,214,10,0.35))",
    willChange: "transform, opacity",
  } satisfies Partial<CSSStyleDeclaration>);
  document.body.appendChild(overlay);

  const recorder = recordFrames();
  const durationMs = 3000;
  const start = performance.now();

  try {
    await new Promise<void>((resolve) => {
      const step = () => {
        const elapsed = performance.now() - start;
        const progress = elapsed / durationMs;
        if (progress >= 1) {
          resolve();
          return;
        }
        const angle = progress * Math.PI * 4;
        overlay.style.transform = `translate3d(${Math.sin(angle) * 40}px, ${
          Math.cos(angle) * 40
        }px, 0) scale(${1 + Math.sin(angle) * 0.05})`;
        overlay.style.opacity = String(0.4 + Math.sin(angle) * 0.3);
        requestAnimationFrame(step);
      };
      requestAnimationFrame(step);
    });
    return { name: "composite", deltas: recorder.stop() };
  } finally {
    // A full-viewport overlay at the top of the stacking order is not something to leave behind
    // if this throws — it would sit over the app, invisible to the user as a benchmark artefact.
    recorder.stop();
    overlay.remove();
  }
}

/** Counts long-animation-frame entries, where the API exists. Absent in WebKit; that is fine. */
function observeLongAnimationFrames(): () => number {
  let count = 0;
  const supported =
    typeof PerformanceObserver !== "undefined" &&
    PerformanceObserver.supportedEntryTypes?.includes("long-animation-frame");

  if (!supported) return () => -1;

  const observer = new PerformanceObserver((list) => {
    count += list.getEntries().length;
  });
  observer.observe({ type: "long-animation-frame", buffered: true });

  return () => {
    observer.disconnect();
    return count;
  };
}

/** Run every phase and return the result. Exported so it can also be driven from a console. */
export async function runBench(label: string): Promise<BenchResult> {
  const startedAt = new Date().toISOString();
  const stopObserving = observeLongAnimationFrames();

  // Let the app finish mounting and the compositor come up to speed. Measuring too early was
  // worth two bogus runs, so this is deliberately generous.
  await wait(2500);

  const raw: RawPhase[] = [];
  raw.push(await phaseScroll());
  await wait(250);
  raw.push(await phaseRepaint());
  await wait(250);
  raw.push(await phaseComposite());

  const frameBudgetMs = inferFrameBudget(raw.flatMap((phase) => phase.deltas.slice(1)));
  const phases = raw.map((phase) => summarise(phase.name, phase.deltas, frameBudgetMs));

  return {
    label,
    userAgent: navigator.userAgent,
    frameBudgetMs,
    devicePixelRatio: window.devicePixelRatio,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    phases,
    longAnimationFrames: stopObserving(),
    startedAt,
  };
}

/**
 * Entry point used by `main.tsx`. Runs the benchmark, then hands the result to whoever is
 * listening: the collector in `scripts/perf/collector.mjs` if one is reachable, the console
 * always, and `window.__ofmBenchResult` for manual inspection.
 */
export async function autoRunBench(label: string): Promise<void> {
  const params = new URLSearchParams(window.location.search);
  const collector = params.get("collector") ?? "http://localhost:1421/result";
  const status = statusBanner();

  status.set(`ofm-bench "${label}" — waiting for the window to be visible…`);
  await waitForVisible(() => {
    status.set(`ofm-bench "${label}" — bring this window to the front to start`);
  });

  status.set(`ofm-bench "${label}" — running, do not touch the window (~11s)`);

  let result: BenchResult;
  try {
    result = await runBench(label);
  } catch (error) {
    // Without this the banner sits on "running…" forever and nothing is posted, which is
    // indistinguishable from a hang — and a hang is one of the things being investigated.
    status.set(`ofm-bench "${label}" FAILED — ${String(error)}`);
    console.error("[ofm-bench] failed:", error);
    return;
  }

  (window as unknown as { __ofmBenchResult?: BenchResult }).__ofmBenchResult = result;
  console.info("[ofm-bench]", JSON.stringify(result));

  const summary = result.phases
    .map((phase) => `${phase.name} p50 ${phase.p50}ms p95 ${phase.p95}ms drop ${phase.dropped}`)
    .join("  |  ");
  status.set(`ofm-bench "${label}" done — ${summary}`);

  try {
    await fetch(collector, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(result),
    });
  } catch {
    // No collector running — the banner, console output and window global cover a manual run.
  }
}
