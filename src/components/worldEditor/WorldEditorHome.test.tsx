import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { MemoryRouter } from "react-router-dom";

import { WorldEditorHome } from "./WorldEditorHome";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

/** As `list_installed_packages` returns it: `installedPath` is the `.ofm` file. */
const INSTALLED = [
  {
    id: "brazil-1962",
    name: "Brazil 1962",
    version: "1.0.0",
    author: "Someone",
    description: "",
    teamCount: 17,
    playerCount: 23,
    competitionCount: 2,
    namePoolCount: 1,
    countryCount: 1,
    confederationCount: 0,
    installedPath: "/home/user/.local/share/openfootmanager/packages/brazil-1962.ofm",
    logoDataUrl: null,
  },
];

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation((cmd: string) =>
    cmd === "list_installed_packages" ? Promise.resolve(INSTALLED) : Promise.reject(new Error(cmd)),
  );
});

function renderHome(handlers: {
  onOpenRecent?: (path: string) => void;
  onOpenInstalled?: (path: string) => void;
}) {
  const onOpenRecent = handlers.onOpenRecent ?? vi.fn();
  const onOpenInstalled = handlers.onOpenInstalled ?? vi.fn();
  render(
    <MemoryRouter>
      <WorldEditorHome
        isBusy={false}
        errorMsg={null}
        recentProjects={[]}
        onNewPackage={() => {}}
        onOpenPackageFile={() => {}}
        onOpenPackageFolder={() => {}}
        onOpenRecent={onOpenRecent}
        onOpenInstalled={onOpenInstalled}
      />
    </MemoryRouter>,
  );
  return { onOpenRecent, onOpenInstalled };
}

describe("opening an installed package from the editor home", () => {
  it("opens it as an archive, not as a project directory", async () => {
    // Reported from real use: "Open in Editor" on an installed package showed
    // an empty editor for a package that loads fine in the game. Installed
    // packages are `.ofm` archives and were being handed to the callback that
    // opens a *directory*, which found no entity files and — because finding
    // nothing is not an error — opened a blank project instead of failing.
    const { onOpenRecent, onOpenInstalled } = renderHome({});

    fireEvent.click(await screen.findByRole("button", { name: "worldEditor.openInEditor" }));

    expect(onOpenInstalled).toHaveBeenCalledWith(
      "/home/user/.local/share/openfootmanager/packages/brazil-1962.ofm",
    );
    expect(onOpenRecent).not.toHaveBeenCalled();
  });
});
