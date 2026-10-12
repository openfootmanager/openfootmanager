import { describe, expect, it } from "vitest";

import hubSource from "../../src-tauri/src/report/hub.rs?raw";

import { UPLOAD_FAILURE_KINDS } from "./reportService";

const camelCase = (variant: string) => variant.charAt(0).toLowerCase() + variant.slice(1);

/** The variants of `pub enum UploadFailureKind` in the Rust source, in camelCase as serialised. */
function rustKinds(): string[] {
  const source = hubSource;
  const body = /pub enum UploadFailureKind \{([^}]*)\}/.exec(source)?.[1] ?? "";
  return body
    .split(",")
    .map((variant) => variant.trim())
    .filter((variant) => /^[A-Z]\w*$/.test(variant))
    .map(camelCase);
}

describe("UPLOAD_FAILURE_KINDS", () => {
  /**
   * Given the Rust enum the backend serialises failures from
   * When its variants are read from the source
   * Then every one has a TypeScript twin, and the TypeScript list has nothing the enum lacks
   */
  it("matches the Rust enum variant for variant", () => {
    const rust = rustKinds();

    expect(rust.length).toBeGreaterThan(5);
    expect([...UPLOAD_FAILURE_KINDS].sort()).toEqual([...rust].sort());
  });
});
