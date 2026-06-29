import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ComponentPropsWithoutRef, ReactNode } from "react";

import { countryName } from "../lib/countries";
import { resetCountryResourcesCache } from "../components/menu/CreateManagerNationalityField";
import type { ManagerProfile } from "../components/menu/types";
import MainMenu from "./MainMenu";

const navigateMock = vi.fn();
const setGameActiveMock = vi.fn();
const setGameStateMock = vi.fn();
const alertMock = vi.fn();
let latestDatePickerOnChange: ((date: string) => void) | null = null;
const translationState = {
  language: "en",
};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const openUrlMock = vi.fn();
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (...args: unknown[]) => openUrlMock(...args),
}));

// The native file-picker returns whatever the current test stages here.
let dialogOpenResult: string | null = null;
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => dialogOpenResult),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: () => { },
  },
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, unknown>) =>
      typeof fallback === "string" ? fallback : key,
    i18n: { language: translationState.language },
  }),
}));

vi.mock("../store/gameStore", () => ({
  useGameStore: (
    selector: (state: {
      setGameActive: typeof setGameActiveMock;
      setGameState: typeof setGameStateMock;
    }) => unknown,
  ) =>
    selector({
      setGameActive: setGameActiveMock,
      setGameState: setGameStateMock,
    }),
}));

vi.mock("../components/ui", () => ({
  Button: ({
    children,
    iconRight: _iconRight,
    iconLeft: _iconLeft,
    ...props
  }: ComponentPropsWithoutRef<"button"> & {
    iconRight?: unknown;
    iconLeft?: unknown;
  }) => <button {...props}>{children}</button>,
  ThemeToggle: () => <div data-testid="theme-toggle" />,
  DatePicker: ({
    value,
    onChange,
  }: {
    value: string;
    onChange: (date: string) => void;
    error?: boolean;
  }) => {
    latestDatePickerOnChange = onChange;

    return (
      <input
        aria-label="manager-date-of-birth"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    );
  },
  CountryFlag: ({ code }: { code: string }) => (
    <span data-testid={`country-flag-${code.toLowerCase()}`} />
  ),
  Select: ({
    value,
    onChange,
    children,
    "aria-label": ariaLabel,
  }: {
    value?: string | number | readonly string[];
    onChange?: (event: { target: { value: string } }) => void;
    children?: ReactNode;
    "aria-label"?: string;
  }) => (
    <select
      aria-label={ariaLabel}
      value={value}
      onChange={(event) => onChange?.({ target: { value: event.target.value } })}
    >
      {children}
    </select>
  ),
}));

vi.mock("../components/ui/ThemeToggle", () => ({
  ThemeToggle: () => <div data-testid="theme-toggle" />,
}));

vi.mock("../components/menu/SavesList", () => ({
  default: ({
    saves,
    onLoad,
  }: {
    saves?: Array<{ id: string; name: string }>;
    onLoad?: (id: string) => void;
  }) => (
    <div data-testid="saves-list">
      {(saves ?? []).map((save) => (
        <button key={save.id} type="button" onClick={() => onLoad?.(save.id)}>
          {save.name}
        </button>
      ))}
    </div>
  ),
}));

vi.mock("../components/menu/PackageBuildStep", () => ({
  default: ({
    onNext,
    onTogglePackage,
    onInstallPackage,
    installedPackages,
  }: {
    onNext: () => void;
    onTogglePackage: (id: string) => void;
    onInstallPackage: () => void;
    installedPackages: Array<{ id: string }>;
  }) => (
    <div data-testid="package-build-step">
      {installedPackages.map((pkg) => (
        <button
          key={pkg.id}
          type="button"
          onClick={() => onTogglePackage(pkg.id)}
        >
          {`toggle-${pkg.id}`}
        </button>
      ))}
      <button type="button" onClick={onInstallPackage}>
        install-package
      </button>
      <button type="button" onClick={onNext}>
        package-next
      </button>
    </div>
  ),
}));

vi.mock("../components/menu/WorldSelect", () => ({
  default: ({
    onStart,
    onChangeHistoryDepthYears,
    historyDepthYears,
  }: {
    onStart: () => void;
    onChangeHistoryDepthYears: (value: number) => void;
    historyDepthYears: number;
  }) => (
    <div data-testid="world-select">
      <button type="button" onClick={() => onChangeHistoryDepthYears(24)}>
        {`set-history-depth-24:${historyDepthYears}`}
      </button>
      <button type="button" onClick={onStart}>
        start-world
      </button>
    </div>
  ),
}));

const mockedInvoke = vi.mocked(invoke);

// The packages step sits between the create form and the generation/world-select
// step. Its internals (installed-package list, stack validation) are covered by
// their own tests; here we shim it to a simple "advance" control so the manager-
// creation flow tests can drive create -> packages -> generation directly.
async function advanceThroughPackages(): Promise<void> {
  fireEvent.click(await screen.findByText("package-next"));
}

async function openCreateManagerForm(): Promise<void> {
  fireEvent.click(screen.getByText("menu.newGame"));
  await screen.findByPlaceholderText("createManager.placeholderFirst");
}

function fillManagerDetails(): void {
  fireEvent.change(
    screen.getByPlaceholderText("createManager.placeholderFirst"),
    {
      target: { value: "Ada" },
    },
  );
  fireEvent.change(
    screen.getByPlaceholderText("createManager.placeholderLast"),
    {
      target: { value: "Lovelace" },
    },
  );
  fireEvent.change(screen.getByLabelText("manager-date-of-birth"), {
    target: { value: "1980-01-01" },
  });
}

function fillCareerStartDetails(
  startYear = "2026",
  startPhase = "seasonStart",
): void {
  fireEvent.change(screen.getByLabelText("createManager.startYear"), {
    target: { value: startYear },
  });
  fireEvent.change(screen.getByLabelText("createManager.startPhase"), {
    target: { value: startPhase },
  });
}

async function getNationalityTrigger(): Promise<HTMLButtonElement> {
  let trigger: HTMLButtonElement | null = null;

  await waitFor(() => {
    const fieldContainer = document.getElementById(
      "create-manager-field-nationality",
    );
    const candidate = fieldContainer?.querySelector(
      "div.relative > button:not([disabled])",
    );

    trigger = candidate instanceof HTMLButtonElement ? candidate : null;

    expect(trigger).toBeInstanceOf(HTMLButtonElement);
  });

  if (!trigger) {
    throw new Error("Nationality trigger button not found");
  }

  return trigger;
}

async function selectNationality(
  language: string,
  nationalityCode: string,
): Promise<void> {
  const countryLabel = countryName(nationalityCode, language);

  fireEvent.mouseDown(await getNationalityTrigger());
  fireEvent.mouseDown(await screen.findByText(countryLabel));
}

async function searchAndSelectNationality(
  language: string,
  nationalityCode: string,
  searchText: string,
): Promise<void> {
  const countryLabel = countryName(nationalityCode, language);

  fireEvent.mouseDown(await getNationalityTrigger());
  const searchInput = await screen.findByPlaceholderText(
    "createManager.searchNationalities",
  );
  fireEvent.change(
    searchInput,
    {
      target: { value: searchText },
    },
  );
  fireEvent.mouseDown(await screen.findByText(countryLabel));
}

describe("MainMenu", () => {
  beforeEach(() => {
    navigateMock.mockReset();
    setGameActiveMock.mockReset();
    setGameStateMock.mockReset();
    alertMock.mockReset();
    openUrlMock.mockReset();
    dialogOpenResult = null;
    localStorage.clear();
    latestDatePickerOnChange = null;
    translationState.language = "en";
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_installed_packages") {
        return [];
      }

      if (command === "start_new_game") {
        return { id: "game-1" };
      }

      if (command === "get_manager_profiles") {
        return [];
      }

      if (command === "save_manager_profile") {
        return { id: "profile-1", first_name: "Test", last_name: "Manager", date_of_birth: "1980-01-01", nationality: "GB", created_at: new Date().toISOString(), last_used_at: null };
      }

      if (command === "touch_manager_profile") {
        return true;
      }

      return null;
    });
    // MainMenu defers focus with requestAnimationFrame; defer one microtask so React
    // commits setFormErrors before focus runs (matches real rAF ordering).
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      queueMicrotask(() => cb(0));
      return 0;
    });
    vi.stubGlobal("alert", alertMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    // Restore any vi.spyOn (e.g. the console.error spy in the save-load test) so
    // it doesn't silently swallow errors in later cases. Plain vi.fn() mocks are
    // unaffected and are reset in beforeEach.
    vi.restoreAllMocks();
    resetCountryResourcesCache();
  });

  it.each(["es", "de", "fr", "it", "pt", "pt-BR"])(
    "stores the nationality as an ISO code and continues the flow in %s",
    async (language: string) => {
      translationState.language = language;

      render(<MainMenu />);

      await openCreateManagerForm();
      fillManagerDetails();
      fillCareerStartDetails("2028", "midSeason");
      await selectNationality(language, "ES");

      const localizedCountryName = countryName("ES", language);
      expect(
        screen.getByRole("button", {
          name: new RegExp(localizedCountryName, "i"),
        }),
      ).toBeInTheDocument();

      fireEvent.click(screen.getByText("createManager.chooseWorld"));

      await advanceThroughPackages();
      expect(await screen.findByTestId("world-select")).toBeInTheDocument();

      fireEvent.click(screen.getByText("start-world"));

      await waitFor(() => {
        expect(mockedInvoke).toHaveBeenCalledWith(
          "start_new_game",
          expect.objectContaining({
            firstName: "Ada",
            lastName: "Lovelace",
            dob: "1980-01-01",
            nationality: "ES",
            startupOptions: expect.objectContaining({
              startYear: 2028,
              startPhase: "midSeason",
              historyDepthYears: 12,
            }),
          }),
        );
      });
      expect(setGameStateMock).toHaveBeenCalledWith({ id: "game-1" });
      expect(navigateMock).toHaveBeenCalledWith("/select-team");
    },
  );

  it("allows changing nationality after the other manager fields are filled", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();

    await selectNationality("en", "ES");
    expect(
      screen.getByRole("button", {
        name: /spain/i,
      }),
    ).toBeInTheDocument();

    await selectNationality("en", "DE");

    expect(
      screen.getByRole("button", {
        name: /germany/i,
      }),
    ).toBeInTheDocument();
  });

  it("allows selecting England instead of legacy GB", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ENG");

    expect(
      screen.getByRole("button", {
        name: /england/i,
      }),
    ).toBeInTheDocument();
  });

  it("preserves nationality when a stale date picker callback fires after selection", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();

    const staleDatePickerOnChange = latestDatePickerOnChange;

    await selectNationality("en", "DE");

    expect(
      screen.getByRole("button", {
        name: /germany/i,
      }),
    ).toBeInTheDocument();

    act(() => {
      staleDatePickerOnChange?.("1980-01-01");
    });

    expect(
      screen.getByRole("button", {
        name: /germany/i,
      }),
    ).toBeInTheDocument();
  });

  it("allows searching localized countries without accents before selecting them", async () => {
    translationState.language = "pt";

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await searchAndSelectNationality("pt", "AT", "austria");

    expect(
      screen.getByRole("button", {
        name: /áustria/i,
      }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await advanceThroughPackages();

    fireEvent.click(screen.getByText("start-world"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "start_new_game",
        expect.objectContaining({
          nationality: "AT",
          startupOptions: expect.objectContaining({
            historyDepthYears: 12,
          }),
        }),
      );
    });
  });

  it("focuses the first invalid field when submitting an empty Create Manager form", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("createManager.placeholderFirst"),
      ).toHaveFocus();
    });
    expect(screen.queryByTestId("package-build-step")).not.toBeInTheDocument();
  });

  it("focuses the next invalid field in order when earlier fields are valid", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderFirst"),
      { target: { value: "Ada" } },
    );
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("createManager.placeholderLast"),
      ).toHaveFocus();
    });
  });

  it("shows min-age feedback for an underage DOB, blocks progression, and focuses the DOB field on submit", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderFirst"),
      { target: { value: "Ada" } },
    );
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderLast"),
      { target: { value: "Lovelace" } },
    );
    fireEvent.change(screen.getByLabelText("manager-date-of-birth"), {
      target: { value: "2010-06-15" },
    });

    expect(screen.getByText("validation.minAge")).toBeInTheDocument();

    await selectNationality("en", "ES");
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(screen.getByLabelText("manager-date-of-birth")).toHaveFocus();
    });
    expect(screen.queryByTestId("package-build-step")).not.toBeInTheDocument();
    expect(screen.queryByTestId("world-select")).not.toBeInTheDocument();
  });

  it("allows a manager who is 30 by the selected start year to continue", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderFirst"),
      { target: { value: "Ada" } },
    );
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderLast"),
      { target: { value: "Lovelace" } },
    );
    fireEvent.change(screen.getByLabelText("manager-date-of-birth"), {
      target: { value: "2008-01-01" },
    });
    fillCareerStartDetails("2038", "seasonStart");
    await selectNationality("en", "ES");

    expect(screen.queryByText("validation.minAge")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await advanceThroughPackages();
    expect(await screen.findByTestId("world-select")).toBeInTheDocument();
  });

  it("uses the selected start phase when evaluating manager age", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderFirst"),
      { target: { value: "Ada" } },
    );
    fireEvent.change(
      screen.getByPlaceholderText("createManager.placeholderLast"),
      { target: { value: "Lovelace" } },
    );
    fireEvent.change(screen.getByLabelText("manager-date-of-birth"), {
      target: { value: "2008-08-01" },
    });
    fillCareerStartDetails("2038", "seasonStart");

    expect(screen.getByText("validation.minAge")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("createManager.startPhase"), {
      target: { value: "midSeason" },
    });

    await waitFor(() => {
      expect(screen.queryByText("validation.minAge")).not.toBeInTheDocument();
    });
  });

  it("blocks progression when the start year is before 2020 and focuses the year field", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    fillCareerStartDetails("2019", "seasonStart");
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(screen.getByLabelText("createManager.startYear")).toHaveFocus();
    });
    expect(screen.getByText("validation.minStartYear")).toBeInTheDocument();
    expect(screen.queryByTestId("package-build-step")).not.toBeInTheDocument();
  });

  it("passes the activated world package ids when starting a new career", async () => {
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_installed_packages") {
        return [
          {
            id: "premier-league",
            name: "Premier League",
            description: "Imported world",
            packageType: "database",
            teamCount: 20,
            playerCount: 400,
            issues: [],
          },
        ];
      }

      if (command === "start_new_game") {
        return { id: "game-1" };
      }

      return null;
    });

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    // The packages step loads installed packages; activate one before advancing.
    fireEvent.click(await screen.findByText("toggle-premier-league"));
    await advanceThroughPackages();

    fireEvent.click(screen.getByText("start-world"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "start_new_game",
        expect.objectContaining({
          packageIds: ["premier-league"],
        }),
      );
    });

    expect(navigateMock).toHaveBeenCalledWith("/select-team");
  });

  it("installs a world package from a picked .ofm file and makes it activatable", async () => {
    dialogOpenResult = "/tmp/custom-world.ofm";
    let installed = false;
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_installed_packages") {
        return installed
          ? [
              {
                id: "custom-world",
                name: "Custom World",
                description: "",
                packageType: "database",
                teamCount: 8,
                playerCount: 160,
                issues: [],
              },
            ]
          : [];
      }
      if (command === "install_package") {
        installed = true;
        return { id: "custom-world", name: "Custom World", packageType: "database", issues: [] };
      }
      if (command === "start_new_game") {
        return { id: "game-1" };
      }
      return null;
    });

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    // No packages installed yet, so the file has to be imported first.
    expect(screen.queryByText("toggle-custom-world")).not.toBeInTheDocument();
    fireEvent.click(await screen.findByText("install-package"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("install_package", {
        path: "/tmp/custom-world.ofm",
      });
    });

    // After install the list reloads and the imported world can be activated.
    fireEvent.click(await screen.findByText("toggle-custom-world"));
    await advanceThroughPackages();
    fireEvent.click(await screen.findByText("start-world"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "start_new_game",
        expect.objectContaining({ packageIds: ["custom-world"] }),
      );
    });
  });

  it("does not install anything when the file picker is cancelled", async () => {
    dialogOpenResult = null; // user dismissed the picker
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    fireEvent.click(await screen.findByText("install-package"));

    // Give any (incorrect) install call a chance to fire, then assert none did.
    await waitFor(() => expect(screen.getByTestId("package-build-step")).toBeInTheDocument());
    expect(mockedInvoke).not.toHaveBeenCalledWith("install_package", expect.anything());
  });

  it("surfaces a message when a save fails to load", async () => {
    vi.spyOn(console, "error").mockImplementation(() => { });

    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_installed_packages") {
        return [];
      }
      if (command === "get_saves") {
        return [
          {
            id: "save-1",
            name: "My Save",
            manager_name: "Ada Lovelace",
            db_filename: "save-1.db",
            checksum: "abc",
            created_at: "2026-01-01T00:00:00Z",
            last_played_at: "2026-02-01T00:00:00Z",
          },
        ];
      }
      if (command === "load_game") {
        throw "be.error.saveLoad.incompatibleVersion?saveVersion=99&supported=29";
      }
      return null;
    });

    render(<MainMenu />);

    fireEvent.click(screen.getByText("menu.loadGame"));

    const saveButton = await screen.findByText("My Save");
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(alertMock).toHaveBeenCalledWith("menu.loadGameFailed");
    });
    expect(navigateMock).not.toHaveBeenCalledWith("/dashboard");
  });

  it("passes the selected generated history depth when starting a new career", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await advanceThroughPackages();

    fireEvent.click(screen.getByText("set-history-depth-24:12"));
    fireEvent.click(screen.getByText("start-world"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "start_new_game",
        expect.objectContaining({
          startupOptions: expect.objectContaining({
            historyDepthYears: 24,
          }),
        }),
      );
    });
  });

  it("persists generated history depth changes to localStorage", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await advanceThroughPackages();

    expect(localStorage.getItem("ofm-generated-history-depth-years")).toBe("12");

    fireEvent.click(screen.getByText("set-history-depth-24:12"));

    expect(localStorage.getItem("ofm-generated-history-depth-years")).toBe("24");
  });

  it("restores the stored generated history depth preference", async () => {
    localStorage.setItem("ofm-generated-history-depth-years", "24");

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));
    await advanceThroughPackages();

    await waitFor(() => {
      expect(screen.getByText("set-history-depth-24:24")).toBeInTheDocument();
    });
  });

  it("falls back to the default generated history depth when storage is invalid", async () => {
    localStorage.setItem("ofm-generated-history-depth-years", "99");

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));
    await advanceThroughPackages();

    await waitFor(() => {
      expect(screen.getByText("set-history-depth-24:12")).toBeInTheDocument();
    });
    expect(localStorage.getItem("ofm-generated-history-depth-years")).toBe("12");
  });

  it("opens the Discord invite in the system browser when the Discord link is clicked", async () => {
    render(<MainMenu />);

    const discordButton = await screen.findByRole("button", { name: "menu.openDiscord" });
    fireEvent.click(discordButton);

    expect(openUrlMock).toHaveBeenCalledTimes(1);
    expect(openUrlMock).toHaveBeenCalledWith("https://discord.gg/2CXaesaukT");
  });

  it("opens the GitHub repository in the system browser when the GitHub link is clicked", async () => {
    render(<MainMenu />);

    const githubButton = await screen.findByRole("button", { name: "menu.openGithub" });
    fireEvent.click(githubButton);

    expect(openUrlMock).toHaveBeenCalledTimes(1);
    expect(openUrlMock).toHaveBeenCalledWith("https://github.com/openfootmanager/openfootmanager");
  });

  describe("profile confirm modal", () => {
    const mockProfile: ManagerProfile = {
      id: "profile-1",
      first_name: "Test",
      last_name: "Manager",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      created_at: "2024-01-01T00:00:00.000Z",
      last_used_at: null,
    };

    beforeEach(() => {
      mockedInvoke.mockImplementation(async (command: string) => {
        if (command === "list_installed_packages") return [];
        if (command === "get_manager_profiles") return [mockProfile];
        if (command === "touch_manager_profile") return true;
        if (command === "save_manager_profile") {
          return { ...mockProfile, id: "profile-2", last_used_at: new Date().toISOString() };
        }
        if (command === "update_manager_profile") {
          return { ...mockProfile, first_name: "Modified" };
        }
        if (command === "delete_manager_profile") return true;
        if (command === "start_new_game") return { id: "game-1" };
        return null;
      });
    });

    async function selectAndModify(): Promise<void> {
      render(<MainMenu />);
      await openCreateManagerForm();
      fireEvent.click(await screen.findByText("Test Manager"));
      fireEvent.change(screen.getByPlaceholderText("createManager.placeholderFirst"), {
        target: { value: "Modified" },
      });
    }

    async function openModal(): Promise<void> {
      await selectAndModify();
      fireEvent.click(screen.getByText("createManager.chooseWorld"));
      await screen.findByText("managerProfiles.saveConfirm.title");
    }

    it("shows the confirm modal when the form differs from the loaded profile", async () => {
      await openModal();
      expect(screen.getByText("managerProfiles.saveConfirm.title")).toBeInTheDocument();
    });

    it("update branch: calls update_manager_profile and proceeds to world select", async () => {
      await openModal();
      fireEvent.click(screen.getByText("managerProfiles.saveConfirm.update"));
      await waitFor(() => {
        expect(mockedInvoke).toHaveBeenCalledWith(
          "update_manager_profile",
          expect.objectContaining({ id: "profile-1", firstName: "Modified" }),
        );
      });
      await advanceThroughPackages();
      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
    });

    it("save-as-new branch: calls save_manager_profile with force and proceeds to world select", async () => {
      await openModal();
      fireEvent.click(screen.getByText("managerProfiles.saveConfirm.saveNew"));
      await advanceThroughPackages();
      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
      await waitFor(() => {
        expect(mockedInvoke).toHaveBeenCalledWith(
          "save_manager_profile",
          expect.objectContaining({ firstName: "Modified", force: true }),
        );
      });
    });

    it("skip branch: proceeds to world select without saving profile changes", async () => {
      await openModal();
      fireEvent.click(screen.getByText("managerProfiles.saveConfirm.skip"));
      await advanceThroughPackages();
      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
    });

    it("cancel: dismisses the modal without navigating away from the form", async () => {
      await openModal();
      fireEvent.click(screen.getByText("menu.cancel"));
      await waitFor(() => {
        expect(screen.queryByText("managerProfiles.saveConfirm.title")).not.toBeInTheDocument();
      });
      expect(screen.queryByTestId("world-select")).not.toBeInTheDocument();
    });

    it("deleting the loaded profile clears it and skips the confirm modal on submit", async () => {
      render(<MainMenu />);
      await openCreateManagerForm();
      fireEvent.click(await screen.findByText("Test Manager"));

      fireEvent.click(screen.getByLabelText("menu.delete"));
      fireEvent.click(screen.getByText("menu.delete"));

      await waitFor(() => {
        expect(mockedInvoke).toHaveBeenCalledWith("delete_manager_profile", { id: "profile-1" });
      });

      fireEvent.click(screen.getByText("createManager.chooseWorld"));
      await advanceThroughPackages();

      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
      expect(screen.queryByText("managerProfiles.saveConfirm.title")).not.toBeInTheDocument();
    });
  });
});
