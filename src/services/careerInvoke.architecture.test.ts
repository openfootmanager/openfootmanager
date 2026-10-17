import { describe, expect, it } from "vitest";

import { SCREEN_WRITE_COMMANDS } from "./careerInvoke";

/**
 * The screen-write contract has two halves that must not drift: the Rust commands that check the
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
   * Then the two are the same set: a command that checks the token is sent it, and a listed
   *      command checks it.
   */
  it("are the same commands in Rust and in the TypeScript list", () => {
    expect([...SCREEN_WRITE_COMMANDS].sort()).toEqual(rustCommandsTakingACareerToken());
  });

  /**
   * Given every service file
   * When its bare invoke calls are read
   * Then none names a screen-write command: those go through invokeForCareer, which sends the
   *      token, so a call that forgot it fails here instead of being refused in play.
   */
  it("are never called with a bare invoke", () => {
    const offenders: string[] = [];
    for (const [file, source] of Object.entries(tsSources)) {
      if (!file.startsWith("./") || isTest(file) || file.endsWith("careerInvoke.ts")) {
        continue;
      }
      for (const command of SCREEN_WRITE_COMMANDS) {
        if (new RegExp(`\\binvoke(?:<[^>]*>)?\\(\\s*["'\`]${command}["'\`]`).test(source)) {
          offenders.push(`${file}: ${command}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  /**
   * Given a component, hook or page
   * When its source is read
   * Then it does not invoke a screen-write command by name: the service layer is the only caller.
   */
  it("are only named in the service layer", () => {
    const offenders: string[] = [];
    for (const [file, source] of Object.entries(tsSources)) {
      if (file.startsWith("./") || isTest(file)) continue;
      for (const command of SCREEN_WRITE_COMMANDS) {
        if (new RegExp(`\\binvoke\\w*(?:<[^>]*>)?\\(\\s*["'\`]${command}["'\`]`).test(source)) {
          offenders.push(`${file}: ${command}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });
});
