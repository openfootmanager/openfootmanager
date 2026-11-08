import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { savePackageProject, validatePackageProject } from "./packageEditorService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

describe("validatePackageProject", () => {
  beforeEach(() => {
    invokeMock.mockReset();
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
   * Given a save request typed against the caller's collections
   * When a collection has the wrong type or a key does not exist
   * Then it does not compile: the request is a typed DTO, not a bag of unknowns
   */
  it("rejects a collection of the wrong type and an unknown collection at compile time", () => {
    interface Shape {
      meta: { id: string };
      confederations: string[];
      countries: string[];
      teams: { id: string }[];
      players: string[];
      staff: string[];
      names: string[];
      competitions: string[];
    }
    const neverCalled = () => {
      // @ts-expect-error teams is a list of clubs, not a number
      void savePackageProject<Shape>({ dir: "/p", collections: { teams: 42 } });
      // @ts-expect-error there is no `team` collection
      void savePackageProject<Shape>({ dir: "/p", collections: { team: [] } });
      void savePackageProject<Shape>({ dir: "/p", collections: { teams: [{ id: "fc-a" }] } });
    };
    expect(typeof neverCalled).toBe("function");
  });
});
