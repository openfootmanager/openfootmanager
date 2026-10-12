// @vitest-environment node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { scanRust } from "./audit-i18n.mjs";

// The players-only ruling (#661): MCP agent output and the modder CLI stay English, so the audit
// must not report prose found there, while ordinary backend code is still checked.
describe("audit-i18n Rust scan", () => {
  let root;

  function write(relativePath) {
    const file = path.join(root, relativePath);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, 'fn f() -> String { "Welcome to the club, manager".to_string() }\n');
  }

  function reportedFiles() {
    return scanRust(root).map((finding) => finding.file.split(path.sep).join("/"));
  }

  beforeAll(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), "ofm-audit-"));
    write("src-tauri/src/commands/greeting.rs");
    write("src-tauri/src/mcp_server/tools_impl/report.rs");
    write("src-tauri/crates/ofm-cli/src/scaffold.rs");
    write("src-tauri/crates/ofm_core/src/news.rs");
  });

  afterAll(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  it("reports prose in ordinary backend code", () => {
    const files = reportedFiles();
    expect(files).toContain("src-tauri/src/commands/greeting.rs");
    expect(files).toContain("src-tauri/crates/ofm_core/src/news.rs");
  });

  it("skips mcp_server", () => {
    expect(reportedFiles().filter((f) => f.includes("mcp_server"))).toEqual([]);
  });

  it("skips ofm-cli", () => {
    expect(reportedFiles().filter((f) => f.includes("ofm-cli"))).toEqual([]);
  });
});
