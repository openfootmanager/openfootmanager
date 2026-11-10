import { expect, it } from "vitest";
import { rememberRecentProject } from "./recentProjects";

/** Given a validated project, when it is reopened, then its last validation is retained. */
it("keeps validation when reopening a project", () => {
  const lastValidation = { errors: 2, warnings: 1, checkedAt: "2026-01-02" };
  const records = [{ path: "/p", name: "World", openedAt: "2026-01-01", lastValidation }];
  expect(rememberRecentProject(records, "/p", "Renamed")[0].lastValidation).toEqual(lastValidation);
  expect(
    JSON.parse(localStorage.getItem("worldEditor.recentProjects") ?? "[]")[0].lastValidation,
  ).toEqual(lastValidation);
});
