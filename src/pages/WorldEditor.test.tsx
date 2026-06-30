import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import WorldEditor from "./WorldEditor";

// --- Tauri boundary -------------------------------------------------------
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// Native dialogs return whatever the current test stages here.
let dialogOpenResult: string | string[] | null = null;
let dialogSaveResult: string | null = null;
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => dialogOpenResult),
  save: vi.fn(async () => dialogSaveResult),
}));

// i18n + backend-error resolution: pass-through so assertions read plainly.
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock("../utils/backendI18n", () => ({
  resolveBackendError: (err: unknown) => String(err),
}));

// --- View shims: expose the lifecycle handlers as plain buttons -----------
const SAMPLE = {
  confederations: [],
  countries: [],
  teams: [],
  players: [],
  staff: [],
  names: { pools: {} },
  competitions: [],
};

vi.mock("../components/worldEditor/WorldEditorHome", () => ({
  WorldEditorHome: ({
    errorMsg,
    recentProjects,
    onNewPackage,
    onOpenPackage,
    onOpenRecent,
  }: {
    errorMsg: string | null;
    recentProjects: { path: string; name: string }[];
    onNewPackage: (meta: unknown, sample: unknown) => void;
    onOpenPackage: () => void;
    onOpenRecent: (path: string) => void;
  }) => (
    <div data-testid="home">
      {errorMsg && <div data-testid="home-error">{errorMsg}</div>}
      <button onClick={() => onNewPackage({ id: "pkg", name: "New Pkg" }, null)}>
        new-plain
      </button>
      <button onClick={() => onNewPackage({ id: "pkg", name: "New Pkg" }, SAMPLE)}>
        new-sample
      </button>
      <button onClick={onOpenPackage}>open-package</button>
      <button onClick={() => onOpenRecent("/recent/dir")}>open-recent-dir</button>
      <button onClick={() => onOpenRecent("/recent/world.ofm")}>open-recent-ofm</button>
      <ul data-testid="recent-list">
        {recentProjects.map((p) => (
          <li key={p.path}>{p.name}</li>
        ))}
      </ul>
    </div>
  ),
}));

vi.mock("../components/worldEditor/WorldEditorLayout", () => ({
  WorldEditorLayout: ({
    topBar,
    formPanel,
  }: {
    topBar: React.ReactNode;
    formPanel: React.ReactNode;
  }) => (
    <div data-testid="layout">
      <div>{topBar}</div>
      <div>{formPanel}</div>
    </div>
  ),
}));

vi.mock("../components/worldEditor/WorldEditorTopBar", () => ({
  WorldEditorTopBar: ({
    packageName,
    saveState,
    autoSave,
    isDirty,
    onValidate,
    onBuild,
    onSave,
    onToggleAutoSave,
  }: {
    packageName: string;
    saveState: string;
    autoSave: boolean;
    isDirty: boolean;
    onValidate: () => void;
    onBuild: () => void;
    onSave: () => void;
    onToggleAutoSave: () => void;
  }) => (
    <div data-testid="topbar">
      <span data-testid="pkg-name">{packageName}</span>
      <span data-testid="save-state">{saveState}</span>
      <span data-testid="auto-save">{String(autoSave)}</span>
      <span data-testid="is-dirty">{String(isDirty)}</span>
      <button onClick={onValidate}>tb-validate</button>
      <button onClick={onBuild}>tb-build</button>
      <button onClick={onSave}>tb-save</button>
      <button onClick={onToggleAutoSave}>tb-toggle-autosave</button>
    </div>
  ),
}));

vi.mock("../components/worldEditor/WorldEditorFormPanel", () => ({
  WorldEditorFormPanel: ({
    meta,
    onMetaChange,
    onSaveMetadata,
  }: {
    meta: { name: string };
    onMetaChange: (m: unknown) => void;
    onSaveMetadata: () => void;
  }) => (
    <div data-testid="formpanel">
      <span data-testid="meta-name">{meta.name}</span>
      <button onClick={() => onMetaChange({ ...meta, name: "Edited" })}>fp-edit-meta</button>
      <button onClick={onSaveMetadata}>fp-save-meta</button>
    </div>
  ),
}));

vi.mock("../components/worldEditor/WorldEditorListContent", () => ({
  WorldEditorListContent: () => <div data-testid="listcontent" />,
}));
vi.mock("../components/worldEditor/WorldEditorSidebar", () => ({
  WorldEditorSidebar: () => <div data-testid="sidebar" />,
}));

const PROJECT_DATA = {
  meta: { id: "pkg", name: "Original" },
  confederations: [],
  countries: [],
  teams: [],
  players: [],
  staff: [],
  names: { pools: {} },
  competitions: [],
  issues: [],
};

const mockedInvoke = vi.mocked(invoke);

/** Open a project via "New (plain)" and wait for the editor layout. */
async function openProject(): Promise<void> {
  fireEvent.click(screen.getByText("new-plain"));
  await screen.findByTestId("layout");
}

function lastSavePayload(): Record<string, unknown> {
  const calls = mockedInvoke.mock.calls.filter((c) => c[0] === "save_package_project");
  return calls[calls.length - 1]?.[1] as Record<string, unknown>;
}

describe("WorldEditor", () => {
  beforeEach(() => {
    localStorage.clear();
    dialogOpenResult = null;
    dialogSaveResult = null;
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "create_world_project":
          return "/new/project/dir";
        case "extract_ofm_for_editing":
          return "/extracted/dir";
        case "read_package_project":
          return PROJECT_DATA;
        case "save_package_project":
        case "build_ofm":
        default:
          return null;
      }
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the Home view with no project open and calls no backend", () => {
    render(<WorldEditor />);
    expect(screen.getByTestId("home")).toBeInTheDocument();
    expect(screen.queryByTestId("layout")).not.toBeInTheDocument();
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("creates a new package and transitions to the editor layout", async () => {
    render(<WorldEditor />);
    await openProject();

    expect(mockedInvoke).toHaveBeenCalledWith(
      "create_world_project",
      expect.objectContaining({ slug: "pkg" }),
    );
    expect(mockedInvoke).toHaveBeenCalledWith("read_package_project", { dir: "/new/project/dir" });
    expect(screen.getByTestId("pkg-name")).toHaveTextContent("Original");
  });

  it("seeds sample data when creating a package from a sample", async () => {
    render(<WorldEditor />);
    fireEvent.click(screen.getByText("new-sample"));
    await screen.findByTestId("layout");

    expect(mockedInvoke).toHaveBeenCalledWith(
      "save_package_project",
      expect.objectContaining({ dir: "/new/project/dir", teams: SAMPLE.teams }),
    );
  });

  it("records opened projects in the recents list", async () => {
    render(<WorldEditor />);
    await openProject();
    expect(localStorage.getItem("worldEditor.recentProjects")).toContain("/new/project/dir");
  });

  it("opens a recent directory path directly", async () => {
    render(<WorldEditor />);
    fireEvent.click(screen.getByText("open-recent-dir"));
    await screen.findByTestId("layout");

    expect(mockedInvoke).toHaveBeenCalledWith("read_package_project", { dir: "/recent/dir" });
    expect(mockedInvoke).not.toHaveBeenCalledWith("extract_ofm_for_editing", expect.anything());
  });

  it("extracts a recent .ofm path before reading it", async () => {
    render(<WorldEditor />);
    fireEvent.click(screen.getByText("open-recent-ofm"));
    await screen.findByTestId("layout");

    expect(mockedInvoke).toHaveBeenCalledWith("extract_ofm_for_editing", {
      ofmPath: "/recent/world.ofm",
    });
    expect(mockedInvoke).toHaveBeenCalledWith("read_package_project", { dir: "/extracted/dir" });
  });

  it("opens a package picked from the native file dialog", async () => {
    dialogOpenResult = "/picked/world.ofm";
    render(<WorldEditor />);
    fireEvent.click(screen.getByText("open-package"));
    await screen.findByTestId("layout");

    expect(mockedInvoke).toHaveBeenCalledWith("extract_ofm_for_editing", {
      ofmPath: "/picked/world.ofm",
    });
  });

  it("stays on Home and surfaces the error when opening fails", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "read_package_project") throw "read-failed";
      if (command === "extract_ofm_for_editing") return "/extracted/dir";
      return null;
    });
    render(<WorldEditor />);
    fireEvent.click(screen.getByText("open-recent-dir"));

    expect(await screen.findByTestId("home-error")).toHaveTextContent("read-failed");
    expect(screen.queryByTestId("layout")).not.toBeInTheDocument();
  });

  it("persists the latest edited state (not the load-time snapshot) on manual save", async () => {
    render(<WorldEditor />);
    await openProject();

    // Edit metadata, then save with no override — persist must read the live
    // stateRef, not a stale closure from when the project loaded.
    fireEvent.click(screen.getByText("fp-edit-meta"));
    expect(screen.getByTestId("meta-name")).toHaveTextContent("Edited");
    fireEvent.click(screen.getByText("tb-save"));

    await waitFor(() => {
      const payload = lastSavePayload();
      expect(payload).toBeDefined();
      expect((payload.meta as { name: string }).name).toBe("Edited");
    });
    await waitFor(() => expect(screen.getByTestId("save-state")).toHaveTextContent("saved"));
  });

  it("surfaces an error and flags save-state when a save fails", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      switch (command) {
        case "create_world_project":
          return "/new/project/dir";
        case "read_package_project":
          return PROJECT_DATA;
        case "save_package_project":
          throw "save-failed";
        default:
          return null;
      }
    });
    render(<WorldEditor />);
    await openProject();

    fireEvent.click(screen.getByText("tb-save"));

    expect(await screen.findByText("save-failed")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId("save-state")).toHaveTextContent("error"));
  });

  it("re-reads issues after a validate (persist then read)", async () => {
    render(<WorldEditor />);
    await openProject();
    const readsBefore = mockedInvoke.mock.calls.filter((c) => c[0] === "read_package_project").length;

    fireEvent.click(screen.getByText("tb-validate"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("save_package_project", expect.anything());
      const readsAfter = mockedInvoke.mock.calls.filter(
        (c) => c[0] === "read_package_project",
      ).length;
      expect(readsAfter).toBe(readsBefore + 1);
    });
  });

  it("builds an .ofm after persisting when a destination is chosen", async () => {
    dialogSaveResult = "/out/world.ofm";
    render(<WorldEditor />);
    await openProject();

    fireEvent.click(screen.getByText("tb-build"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("build_ofm", {
        dir: "/new/project/dir",
        output: "/out/world.ofm",
      });
    });
    expect(await screen.findByText("worldEditor.buildSuccess")).toBeInTheDocument();
  });

  it("does not build when the destination dialog is cancelled", async () => {
    dialogSaveResult = null;
    render(<WorldEditor />);
    await openProject();

    fireEvent.click(screen.getByText("tb-build"));

    await waitFor(() => expect(screen.getByTestId("save-state")).toBeInTheDocument());
    expect(mockedInvoke).not.toHaveBeenCalledWith("build_ofm", expect.anything());
  });

  it("toggles auto-save and persists the preference", async () => {
    render(<WorldEditor />);
    await openProject();
    expect(screen.getByTestId("auto-save")).toHaveTextContent("true");

    fireEvent.click(screen.getByText("tb-toggle-autosave"));

    expect(screen.getByTestId("auto-save")).toHaveTextContent("false");
    expect(localStorage.getItem("worldEditor.autoSave")).toBe("false");
  });
});
