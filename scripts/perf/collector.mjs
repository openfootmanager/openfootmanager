#!/usr/bin/env node
/**
 * Receives benchmark results from `src/dev/benchUi.ts` and writes them to disk.
 *
 * The webview has no filesystem access and its console does not reach our stdout, so the browser
 * side POSTs its JSON here instead. Runs on port 1421, next to Vite's 1420.
 *
 *   node scripts/perf/collector.mjs [--out DIR] [--expect N] [--label NAME]
 *
 * Exits 0 once `--expect` results have arrived (default: runs until killed).
 *
 * `--label` rejects results from anything but the run being measured. Without it, any stray
 * client still pointed at localhost:1420 -- a browser tab left open from an earlier run, say --
 * satisfies `--expect` and the row silently records someone else's numbers.
 */

import { createServer } from "node:http";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const readFlag = (name, fallback) => {
  const index = args.indexOf(name);
  return index === -1 ? fallback : args[index + 1];
};

const outDir = resolve(readFlag("--out", "scripts/perf/results"));
const onlyLabel = readFlag("--label", "");

// Both of these are counts, so both must be whole numbers in range. A flag given without a value
// (`--expect` at the end of the line) reads as undefined and Number(undefined) is NaN, which would
// make `received >= expected` never true and hang the row until the runner's timeout. Fractional
// values are just as wrong and quieter: `--expect 0.5` satisfies `received >= expected` on the
// first result, so the row stops after one instead of erroring. Fail on the input, not the symptom.
function integerFlag(name, fallback, min, max) {
  const raw = readFlag(name, fallback);
  const value = Number(raw);
  if (!Number.isInteger(value) || value < min || value > max) {
    console.error(`[collector] ${name} needs an integer from ${min} to ${max}, got: ${raw}`);
    process.exit(2);
  }
  return value;
}

const expected = integerFlag("--expect", "0", 0, Number.MAX_SAFE_INTEGER);
// Out-of-range ports otherwise reach server.listen and die with ERR_SOCKET_BAD_PORT.
const port = integerFlag("--port", "1421", 0, 65535);

await mkdir(outDir, { recursive: true });

let received = 0;

const server = createServer((req, res) => {
  // The webview's origin is not ours, so the POST is cross-origin.
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");
  res.setHeader("Access-Control-Allow-Methods", "POST, OPTIONS");

  if (req.method === "OPTIONS") {
    res.writeHead(204).end();
    return;
  }
  if (req.method !== "POST") {
    res.writeHead(405).end();
    return;
  }

  const chunks = [];
  req.on("data", (chunk) => chunks.push(chunk));
  req.on("end", async () => {
    const body = Buffer.concat(chunks).toString("utf8");
    try {
      const result = JSON.parse(body);

      if (onlyLabel && result.label !== onlyLabel) {
        console.warn(
          `[collector] ignoring result labelled "${result.label}" (measuring "${onlyLabel}")`,
        );
        res.writeHead(409).end();
        return;
      }

      // Label comes from the URL, so sanitise it before it becomes a filename.
      const safeLabel = String(result.label ?? "unlabelled").replace(/[^a-zA-Z0-9._-]/g, "_");
      const file = resolve(outDir, `${safeLabel}.json`);
      await writeFile(file, `${JSON.stringify(result, null, 2)}\n`, "utf8");
      received += 1;
      console.log(`[collector] ${received}${expected ? `/${expected}` : ""} → ${file}`);
      res.writeHead(204).end();
      if (expected && received >= expected) {
        server.close(() => process.exit(0));
      }
    } catch (error) {
      console.error("[collector] bad payload:", error.message);
      res.writeHead(400).end();
    }
  });
});

server.listen(port, "127.0.0.1", () => {
  console.log(`[collector] listening on http://127.0.0.1:${port}, writing to ${outDir}`);
});
