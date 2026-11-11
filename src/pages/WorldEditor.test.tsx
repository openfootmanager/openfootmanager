import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import WorldEditor from "./WorldEditor";
import { ThemeContext } from "../context/ThemeContext";
import { emptyMeta, emptyNamesDefinition } from "../components/menu/PackageEditor/helpers";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("react-i18next", async (importOriginal) => ({
  ...(await importOriginal<typeof import("react-i18next")>()),
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

const path = "/projects/old-world";
const recentKey = "worldEditor.recentProjects";
const diagnostic = { code: "test.finding", file: "world.json", params: {} };

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

/** Given an old recent project, when it is validated and reopened, then its findings and check time persist. */
it("remembers validation findings when an old recent project is reopened", async () => {
  localStorage.setItem(recentKey, JSON.stringify([{ path, name: "Old world", openedAt: "old" }]));
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "list_installed_packages") return [];
    // The home lists the recent project from the backend's reading of its folder.
    if (command === "get_worlds_library") {
      return {
        projects: [
          {
            path,
            name: "Old world",
            version: null,
            clubs: 1,
            players: 1,
            countries: 1,
            error: null,
          },
        ],
        installed: [],
      };
    }
    if (command === "read_package_project") {
      return {
        meta: { ...emptyMeta(), name: "Old world" },
        confederations: [],
        countries: [],
        teams: [],
        players: [],
        staff: [],
        names: emptyNamesDefinition(),
        competitions: [],
        issues: [],
        warnings: [],
      };
    }
    if (command === "save_package_project") return { saved: [], filesWritten: 0 };
    if (command === "validate_package_project") {
      return { issues: [diagnostic, diagnostic], warnings: [diagnostic] };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  const page = () => (
    <ThemeContext.Provider value={{ theme: "light", isDark: false, toggleTheme: () => {} }}>
      <MemoryRouter>
        <WorldEditor />
      </MemoryRouter>
    </ThemeContext.Provider>
  );
  const first = render(page());
  // The home's rows come once the library has answered; the row's button is named for its world
  // (the test's `t` returns the key alone).
  fireEvent.click(await screen.findByRole("button", { name: /worldEditor.home.openNamed/ }));
  const validate = await screen.findByRole("button", { name: /worldEditor.validate/ });
  const before = Date.now();
  fireEvent.click(validate);
  await waitFor(() => {
    const records = JSON.parse(localStorage.getItem(recentKey) ?? "[]");
    expect(records[0].lastValidation).toMatchObject({ errors: 2, warnings: 1 });
  });
  const after = Date.now();
  first.unmount();

  const reopened = render(page());
  fireEvent.click(await screen.findByRole("button", { name: /worldEditor.home.openNamed/ }));
  await screen.findByRole("button", { name: /worldEditor.validate/ });
  const records = JSON.parse(localStorage.getItem(recentKey) ?? "[]");
  expect(records).toHaveLength(1);
  expect(records[0].path).toBe(path);
  expect(records[0].lastValidation).toEqual({
    errors: 2,
    warnings: 1,
    checkedAt: expect.any(String),
  });
  const checkedAt = Date.parse(records[0].lastValidation.checkedAt);
  expect(checkedAt).toBeGreaterThanOrEqual(before);
  expect(checkedAt).toBeLessThanOrEqual(after);
  reopened.unmount();
});
