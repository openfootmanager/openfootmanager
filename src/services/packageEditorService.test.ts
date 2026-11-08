import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { validatePackageProject } from "./packageEditorService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

describe("validatePackageProject", () => {
  beforeEach(() => invokeMock.mockReset());

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
});
