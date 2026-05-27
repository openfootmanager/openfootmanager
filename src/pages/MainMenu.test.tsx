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
  default: () => <div data-testid="saves-list" />,
}));

vi.mock("../components/menu/WorldSelect", () => ({
  default: ({
    onStart,
    onSelectWorld,
    onChangeHistoryDepthYears,
    historyDepthYears,
    worldDatabases,
  }: {
    onStart: () => void;
    onSelectWorld: (id: string) => void;
    onChangeHistoryDepthYears: (value: number) => void;
    historyDepthYears: number;
    worldDatabases: Array<{ id: string }>;
  }) => (
    <div data-testid="world-select">
      {worldDatabases.map((db) => (
        <button key={db.id} type="button" onClick={() => onSelectWorld(db.id)}>
          {`select-${db.id}`}
        </button>
      ))}
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
    localStorage.clear();
    latestDatePickerOnChange = null;
    translationState.language = "en";
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation(async (command: string) => {
      if (command === "list_world_databases") {
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

      await waitFor(() => {
        expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
      });
      expect(screen.getByTestId("world-select")).toBeInTheDocument();

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

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });

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
    expect(mockedInvoke).not.toHaveBeenCalledWith("list_world_databases");
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
    expect(mockedInvoke).not.toHaveBeenCalledWith("list_world_databases");
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

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });
    expect(screen.getByTestId("world-select")).toBeInTheDocument();
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
    expect(mockedInvoke).not.toHaveBeenCalledWith("list_world_databases");
  });

  it("passes the imported world path directly when starting a new career", async () => {
    mockedInvoke.mockImplementation(async (command: string, args?) => {
      if (command === "list_world_databases") {
        return [
          {
            id: "file:imported-world.json",
            name: "Imported World",
            description: "Imported",
            team_count: 8,
            player_count: 160,
            source: "imported",
            path: "/tmp/imported-world.json",
            history_mode: "reference",
          },
        ];
      }

      if (command === "start_new_game") {
        expect((args as Record<string, unknown>)?.worldSource).toBe("file:/tmp/imported-world.json");
        return { id: "game-1" };
      }

      return null;
    });

    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });

    fireEvent.click(screen.getByText("select-file:imported-world.json"));
    fireEvent.click(screen.getByText("start-world"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith(
        "start_new_game",
        expect.objectContaining({
          worldSource: "file:/tmp/imported-world.json",
        }),
      );
    });

    expect(mockedInvoke).not.toHaveBeenCalledWith("write_temp_database", expect.anything());
    expect(navigateMock).toHaveBeenCalledWith("/select-team");
  });

  it("passes the selected generated history depth when starting a new career", async () => {
    render(<MainMenu />);

    await openCreateManagerForm();
    fillManagerDetails();
    await selectNationality("en", "ES");

    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });

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

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("list_world_databases");
    });

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

    await waitFor(() => {
      expect(screen.getByText("set-history-depth-24:12")).toBeInTheDocument();
    });
    expect(localStorage.getItem("ofm-generated-history-depth-years")).toBe("12");
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
        if (command === "list_world_databases") return [];
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
      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
    });

    it("save-as-new branch: calls save_manager_profile with force and proceeds to world select", async () => {
      await openModal();
      fireEvent.click(screen.getByText("managerProfiles.saveConfirm.saveNew"));
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

      await waitFor(() => {
        expect(screen.getByTestId("world-select")).toBeInTheDocument();
      });
      expect(screen.queryByText("managerProfiles.saveConfirm.title")).not.toBeInTheDocument();
    });
  });
});
