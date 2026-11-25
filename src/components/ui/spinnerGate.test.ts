import { describe, expect, it } from "vitest";

const SOURCES = import.meta.glob(
  ["../../**/*.ts", "../../**/*.tsx", "!../../**/*.test.ts", "!../../**/*.test.tsx"],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

const PRIMITIVE = "./Spinner.tsx";

/** The files that spin without going through the primitive: the one place `animate-spin` lives. */
function spinOffenders(files: Record<string, string>): string[] {
  return Object.entries(files)
    .filter(([file]) => file !== PRIMITIVE)
    .filter(([, source]) => /animate-spin/.test(source))
    .map(([file]) => file);
}

describe("spinner gate", () => {
  /**
   * Given every production source file under `src/`
   * When it is scanned for `animate-spin`
   * Then only the Spinner primitive carries it, so a loading ring cannot skip its status text.
   */
  it("routes every spin through the Spinner primitive", () => {
    expect(spinOffenders(SOURCES)).toEqual([]);
  });

  /**
   * Given a file with a planted bare spin, the primitive itself, and a quiet file
   * When they are scanned
   * Then only the planted file is reported: the gate can fail.
   */
  it("flags a planted violation and allows the primitive", () => {
    const planted = {
      "../../components/Bare.tsx": '<div className="motion-safe:animate-spin" />',
      [PRIMITIVE]: 'const RING = "motion-safe:animate-spin";',
      "../../components/Quiet.tsx": "<div />",
    };

    expect(spinOffenders(planted)).toEqual(["../../components/Bare.tsx"]);
  });
});
