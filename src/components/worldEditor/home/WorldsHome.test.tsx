import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SamplePackage } from "../../menu/PackageEditor/sampleData";
import type { WorldsLibrary } from "../../../store/generated/WorldsLibrary";
import type { RecentProject } from "../../../store/packageEditorTypes";
import WorldsHome from "./WorldsHome";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

vi.mock("react-i18next", () => ({
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params?.name !== undefined ? `${key}:${params.name}` : key,
    i18n: { language: "en" },
  }),
}));

const row = (path: string, name: string, overrides = {}) => ({
  path,
  name,
  version: null,
  clubs: 32,
  players: 740,
  countries: 1,
  error: null,
  ...overrides,
});

const LIBRARY: WorldsLibrary = {
  projects: [row("/w/nordic", "Nordic Leagues"), row("/w/classic", "Classic 1962")],
  installed: [row("/p/brazil.ofm", "Brazil 1962", { version: "1.0" })],
};
const RECENT: RecentProject[] = [
  { path: "/w/nordic", name: "Nordic Leagues", openedAt: "2026-11-09T10:00:00Z" },
  { path: "/w/classic", name: "Classic 1962", openedAt: "2026-11-02T10:00:00Z" },
];
const SAMPLES = [
  { meta: { id: "minimal", name: "Minimal world", description: "1 country · 32 clubs" } },
] as unknown as SamplePackage[];

function renderHome(overrides: Partial<Parameters<typeof WorldsHome>[0]> = {}) {
  const handlers = {
    onNewWorld: vi.fn(),
    onOpenFile: vi.fn(),
    onOpenFolder: vi.fn(),
    onStartFromSample: vi.fn(),
    onOpenProject: vi.fn(),
    onOpenInstalled: vi.fn(),
    onBack: vi.fn(),
  };
  render(
    <WorldsHome
      isBusy={false}
      errorMsg={null}
      recentProjects={RECENT}
      samples={SAMPLES}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation((cmd: string) =>
    cmd === "get_worlds_library" ? Promise.resolve(LIBRARY) : Promise.reject(new Error(cmd)),
  );
});

describe("WorldsHome", () => {
  /**
   * Given two projects the editor has opened and one installed package
   * When the home opens
   * Then the backend is asked for the library of those projects' folders, and "Your worlds" lists
   *      the projects and then the installed package.
   */
  it("asks for the library of the recent projects and lists every world", async () => {
    renderHome();

    await screen.findByRole("button", { name: "worldEditor.home.editCopyNamed:Brazil 1962" });
    expect(invoke).toHaveBeenCalledWith("get_worlds_library", {
      paths: ["/w/nordic", "/w/classic"],
    });
    expect(
      screen.getByRole("heading", { level: 1, name: "worldEditor.home.yourWorlds" }),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("row")).toHaveLength(4);
  });

  /**
   * Given the library is shown
   * When Projects, then Installed, is chosen, and then a search typed
   * Then the table keeps only that kind, and the search narrows by name or path within it.
   */
  it("filters by kind and searches by name or path", async () => {
    renderHome();
    await screen.findByRole("button", { name: "worldEditor.home.editCopyNamed:Brazil 1962" });

    fireEvent.click(screen.getByRole("radio", { name: "worldEditor.home.filterProjects" }));
    expect(screen.getAllByRole("row")).toHaveLength(3);
    // The new count is said aloud.
    expect(screen.getByRole("status")).toHaveTextContent("worldEditor.home.resultCount");
    expect(screen.queryByRole("button", { name: /editCopyNamed/ })).toBeNull();

    fireEvent.click(screen.getByRole("radio", { name: "worldEditor.home.filterAll" }));
    fireEvent.change(screen.getByRole("searchbox", { name: "worldEditor.home.search" }), {
      target: { value: "brazil" },
    });
    expect(screen.getAllByRole("row")).toHaveLength(2);
    expect(
      screen.getByRole("button", { name: "worldEditor.home.editCopyNamed:Brazil 1962" }),
    ).toBeInTheDocument();
  });

  /**
   * Given a project and an installed package
   * When "Open" and "Edit a copy" are pressed
   * Then the project opens as a folder and the installed package as its .ofm archive: the two are
   *      opened by different means, and an archive handed to the folder reader opens an empty editor.
   */
  it("opens a project as a folder and an installed package as an archive", async () => {
    const handlers = renderHome();

    fireEvent.click(
      await screen.findByRole("button", { name: "worldEditor.home.openNamed:Nordic Leagues" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "worldEditor.home.editCopyNamed:Brazil 1962" }),
    );

    expect(handlers.onOpenProject).toHaveBeenCalledWith("/w/nordic");
    expect(handlers.onOpenInstalled).toHaveBeenCalledWith("/p/brazil.ofm");
    expect(handlers.onOpenProject).toHaveBeenCalledTimes(1);
    expect(handlers.onOpenInstalled).toHaveBeenCalledTimes(1);
  });

  /**
   * Given the sidebar
   * When each of its controls is pressed
   * Then it starts a new world, opens a file, opens a folder, starts from the example, or goes back.
   */
  it("starts, opens and goes back from the sidebar", async () => {
    const handlers = renderHome();
    await screen.findByRole("button", { name: "worldEditor.home.editCopyNamed:Brazil 1962" });
    const nav = screen.getByRole("complementary", { name: "worldEditor.title" });

    fireEvent.click(within(nav).getByRole("button", { name: "worldEditor.home.newWorld" }));
    fireEvent.click(within(nav).getByRole("button", { name: "worldEditor.home.openFile" }));
    fireEvent.click(within(nav).getByRole("button", { name: "worldEditor.home.openFolder" }));
    fireEvent.click(within(nav).getByRole("button", { name: /Minimal world/ }));
    fireEvent.click(within(nav).getByRole("button", { name: "worldEditor.home.backToMenu" }));

    expect(handlers.onNewWorld).toHaveBeenCalledTimes(1);
    expect(handlers.onOpenFile).toHaveBeenCalledTimes(1);
    expect(handlers.onOpenFolder).toHaveBeenCalledTimes(1);
    expect(handlers.onStartFromSample).toHaveBeenCalledWith(SAMPLES[0]);
    expect(handlers.onBack).toHaveBeenCalledTimes(1);
  });

  /**
   * Given no project opened and nothing installed
   * When the home opens
   * Then the table says how to start, instead of an empty grid.
   */
  it("says how to start when there are no worlds", async () => {
    invoke.mockImplementation(() => Promise.resolve({ projects: [], installed: [] }));
    renderHome({ recentProjects: [] });

    expect(await screen.findByText("worldEditor.home.emptyLibrary")).toBeInTheDocument();
  });

  it("shows the editor's error and still lists the worlds", async () => {
    renderHome({ errorMsg: "Could not open it" });

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not open it");
    await waitFor(() => expect(screen.getAllByRole("row").length).toBeGreaterThan(1));
  });
});
