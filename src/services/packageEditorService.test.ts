import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import {
  getWorldsLibrary,
  savePackageProject,
  validatePackageProject,
} from "./packageEditorService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** Given recent paths, when the library is requested, then the IPC receives every path. */
it("passes recent paths to the worlds library command", async () => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({ projects: [], installed: [] });
  await getWorldsLibrary(["/a", "/b"]);
  expect(invokeMock).toHaveBeenCalledWith("get_worlds_library", { paths: ["/a", "/b"] });
});

/** Given the library command fails, when it is requested, then the error reaches the caller. */
it("propagates a worlds library error", async () => {
  invokeMock.mockReset();
  invokeMock.mockRejectedValue("be.error.invalidPath");
  await expect(getWorldsLibrary(["/a"])).rejects.toBe("be.error.invalidPath");
});

describe("validatePackageProject", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    localStorage.clear();
  });

  /** Given stored recents, when Validate returns, then the caller owns the only persistence update. */
  it("does not write recent projects while fetching validation", async () => {
    localStorage.setItem("worldEditor.recentProjects", JSON.stringify([{ path: "/p", name: "P", openedAt: "old" }]));
    const write = vi.spyOn(Storage.prototype, "setItem");
    invokeMock.mockResolvedValue({ issues: [], warnings: [] });
    try {
      await validatePackageProject("/p");
      expect(write).not.toHaveBeenCalled();
    } finally {
      write.mockRestore();
    }
  });

  /** Given diagnostics omit warnings, when Validate returns, then its issues still reach the caller. */
  it("returns diagnostics when warnings are absent", async () => {
    invokeMock.mockResolvedValue({ issues: [{ code: "x" }] });
    await expect(validatePackageProject("/p")).resolves.toEqual({ issues: [{ code: "x" }], warnings: [] });
  });

  /** Given an old recent record, when Validate reports findings, then the stored record is updated. */
  it("saves validation findings on an old recent project", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-10T12:00:00.000Z"));
    localStorage.setItem(
      "worldEditor.recentProjects",
      JSON.stringify([
        { path: "/p", name: "Old world", openedAt: "2026-01-01" },
        { path: "/other", name: "Other", openedAt: "2026-01-02" },
      ]),
    );
    invokeMock.mockResolvedValue({
      issues: [{ code: "x" }],
      warnings: [{ code: "y" }, { code: "z" }],
    });

    try {
      await validatePackageProject("/p");

      const records = JSON.parse(localStorage.getItem("worldEditor.recentProjects") ?? "[]");
      expect(records[0].lastValidation).toEqual({
        errors: 1,
        warnings: 2,
        checkedAt: "2026-11-10T12:00:00.000Z",
      });
      expect(records[1]).toEqual({ path: "/other", name: "Other", openedAt: "2026-01-02" });
    } finally {
      vi.useRealTimers();
    }
  });

  /** Given storage cannot be read, when Validate succeeds, then its findings still reach the editor. */
  it("returns findings when recent-project storage is unavailable", async () => {
    const read = vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage unavailable");
    });
    invokeMock.mockResolvedValue({ issues: [], warnings: [] });
    try {
      await expect(validatePackageProject("/p")).resolves.toEqual({ issues: [], warnings: [] });
    } finally {
      read.mockRestore();
    }
  });

  /** Given stored JSON is not a list, when Validate succeeds, then its findings still reach the editor. */
  it("returns findings when recent-project storage is not a list", async () => {
    localStorage.setItem("worldEditor.recentProjects", "{}");
    invokeMock.mockResolvedValue({ issues: [], warnings: [] });
    await expect(validatePackageProject("/p")).resolves.toEqual({ issues: [], warnings: [] });
  });

  /**
   * Given a project folder
   * When the editor validates it
   * Then it asks the diagnostics-only command, never the one that returns every entity
   */
  it("asks for diagnostics only", async () => {
    invokeMock.mockResolvedValue({ issues: [], warnings: [] });

    await validatePackageProject("/projects/demo");

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("validate_package_project", { dir: "/projects/demo" });
  });

  /**
   * Given the backend reports an issue and a warning
   * When the editor validates
   * Then both reach the caller unchanged, with their params
   */
  it("returns the issues and warnings the backend reported", async () => {
    const issue = {
      code: "be.error.package.duplicateId",
      file: "teams/teams.json",
      params: { id: "fc-a" },
    };
    const warning = {
      code: "be.warning.package.countryWithoutRegion",
      file: "countries/qq.json",
      params: {},
    };
    invokeMock.mockResolvedValue({ issues: [issue], warnings: [warning] });

    await expect(validatePackageProject("/p")).resolves.toEqual({
      issues: [issue],
      warnings: [warning],
    });
  });

  /**
   * Given the backend refuses the folder
   * When the editor validates
   * Then the translation-key error propagates for the caller to show
   */
  it("propagates a refusal", async () => {
    invokeMock.mockImplementation(() => Promise.reject("be.error.package.notAProjectDirectory"));

    let refusal: unknown;
    try {
      await validatePackageProject("/nope");
    } catch (error) {
      refusal = error;
    }

    expect(refusal).toBe("be.error.package.notAProjectDirectory");
  });

  /**
   * Given the save request is typed against the package's real collections
   * When a collection has the wrong type or a key does not exist
   * Then it does not compile: the request is a typed DTO, whatever the caller names
   */
  it("rejects a collection of the wrong type and an unknown collection at compile time", () => {
    const neverCalled = () => {
      // @ts-expect-error teams is a list of clubs, not a number
      void savePackageProject({ dir: "/p", collections: { teams: 42 } });
      // @ts-expect-error there is no `team` collection
      void savePackageProject({ dir: "/p", collections: { team: [] } });
      void savePackageProject({ dir: "/p", collections: { teams: [] } });
    };
    expect(typeof neverCalled).toBe("function");
  });
});
