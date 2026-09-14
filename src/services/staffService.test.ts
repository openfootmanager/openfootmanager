import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { hireStaff, releaseStaff } from "./staffService";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("staffService", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it("calls the hire staff backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(hireStaff("staff-1")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("hire_staff", {
      staffId: "staff-1",
    });
  });

  it("calls the release staff backend command", async () => {
    const response = { manager: { id: "manager-1" } };
    mockedInvoke.mockResolvedValueOnce(response);

    await expect(releaseStaff("staff-2")).resolves.toBe(response);
    expect(mockedInvoke).toHaveBeenCalledWith("release_staff", {
      staffId: "staff-2",
    });
  });
});
