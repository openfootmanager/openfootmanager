import { describe, expect, it } from "vitest";

import { SCREEN_WRITE_COMMANDS } from "./careerInvoke";
import { CAREER_SCOPED_READ_COMMANDS } from "./seasonReviewService";

/**
 * The career-scoped contract has two halves that must not drift: Rust commands that check the
 * career token, and the TypeScript services that send it. These read both from source.
 */
const rustSources = import.meta.glob("../../src-tauri/src/commands/**/*.rs", {
  eager: true,
  import: "default",
  query: "?raw",
}) as Record<string, string>;

const tsSources = {
  ...(import.meta.glob("../**/*.ts", { eager: true, import: "default", query: "?raw" }) as Record<
    string,
    string
  >),
  ...(import.meta.glob("../**/*.tsx", { eager: true, import: "default", query: "?raw" }) as Record<
    string,
    string
  >),
};

const isTest = (path: string) => /\.test\.tsx?$/.test(path);

/** Names of the `#[tauri::command]` functions whose parameters include `career_token`. */
function rustCommandsTakingACareerToken(): string[] {
  const names: string[] = [];
  for (const source of Object.values(rustSources)) {
    const commands = source.matchAll(
      /#\[tauri::command\][^{;]*?\bfn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([\s\S]*?)\)\s*(?:->[^{]*)?\{/g,
    );
    for (const match of commands) {
      if (/\bcareer_token\s*:/.test(match[2])) names.push(match[1]);
    }
  }
  return names.sort();
}

describe("screen-write commands", () => {
  /**
   * Given the Rust commands that take a career token
   * When they are compared with the TypeScript list
   * Then they match the declared writes and reads that send the token.
   */
  it("are the same commands in Rust and in the TypeScript list", () => {
    expect([...SCREEN_WRITE_COMMANDS, ...CAREER_SCOPED_READ_COMMANDS].sort()).toEqual(
      rustCommandsTakingACareerToken(),
    );
  });

  /**
   * Given every source file outside tests
   * When its bare invoke calls are read
   * Then none names a screen-write command: those go through invokeForCareer, which sends the
   *      token, so a call that forgot it fails here instead of being refused in play.
   */
  it("are never called with a bare invoke", () => {
    const offenders: string[] = [];
    for (const [file, source] of Object.entries(tsSources)) {
      if (isTest(file) || file.endsWith("careerInvoke.ts")) continue;
      for (const command of SCREEN_WRITE_COMMANDS) {
        if (new RegExp(`\\binvoke\\b[^(]*\\(\\s*["'\`]${command}["'\`]`).test(source)) {
          offenders.push(`${file}: ${command}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });
});
