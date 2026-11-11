import { expect, it, vi } from "vitest";
import { rememberRecentProject, updateRecentValidation } from "./recentProjects";

/** Given a validated project, when it is reopened, then its last validation is retained. */
it("keeps validation when reopening a project", () => {
  const lastValidation = { errors: 2, warnings: 1, checkedAt: "2026-01-02" };
  const records = [{ path: "/p", name: "World", openedAt: "2026-01-01", lastValidation }];
  expect(rememberRecentProject(records, "/p", "Renamed")[0].lastValidation).toEqual(lastValidation);
  expect(
    JSON.parse(localStorage.getItem("worldEditor.recentProjects") ?? "[]")[0].lastValidation,
  ).toEqual(lastValidation);
});

/** Given stale storage and a failed write, when validation completes, then current recents retain the result. */
it("keeps current projects and validation when storage is full", () => {
  localStorage.setItem(
    "worldEditor.recentProjects",
    JSON.stringify([{ path: "/a", name: "A", openedAt: "old" }]),
  );
  const write = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("full");
  });
  try {
    const current = [
      { path: "/b", name: "B", openedAt: "now" },
      { path: "/a", name: "A", openedAt: "old" },
    ];
    const result = updateRecentValidation(current, "/b", {
      errors: 2,
      warnings: 1,
      checkedAt: "today",
    });
    expect(result).toEqual([
      { ...current[0], lastValidation: { errors: 2, warnings: 1, checkedAt: "today" } },
      current[1],
    ]);
  } finally {
    write.mockRestore();
  }
});
