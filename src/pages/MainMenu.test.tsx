import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ComponentPropsWithoutRef } from "react";

import { countryName } from "../lib/countries";
import MainMenu from "./MainMenu";

const navigateMock = vi.fn();
const setGameActiveMock = vi.fn();
const setGameStateMock = vi.fn();
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
    init: () => {},
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
}));

vi.mock("../components/menu/SavesList", () => ({
  default: () => <div data-testid="saves-list" />,
}));

vi.mock("../components/menu/WorldSelect", () => ({
  default: ({ onStart }: { onStart: () => void }) => (
    <div data-testid="world-select">
      <button type="button" onClick={onStart}>
        start-world
      </button>
    </div>
  ),
}));

const mockedInvoke = vi.mocked(invoke);

function openCreateManagerForm(): void {
  fireEvent.click(screen.getByText("menu.newGame"));
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

function getNationalityTrigger(): HTMLButtonElement {
  const fieldContainer = document.getElementById(
    "create-manager-field-nationality",
  );
  const trigger = fieldContainer?.querySelector("div.relative > button");

  if (!(trigger instanceof HTMLButtonElement)) {
    throw new Error("Nationality trigger button not found");
  }

  return trigger;
}

function selectNationality(language: string, nationalityCode: string): void {
  const countryLabel = countryName(nationalityCode, language);

  fireEvent.mouseDown(getNationalityTrigger());
  fireEvent.mouseDown(screen.getByText(countryLabel));
}

function searchAndSelectNationality(
  language: string,
  nationalityCode: string,
  searchText: string,
): void {
  const countryLabel = countryName(nationalityCode, language);

  fireEvent.mouseDown(getNationalityTrigger());
  fireEvent.change(
    screen.getByPlaceholderText("createManager.searchNationalities"),
    {
      target: { value: searchText },
    },
  );
  fireEvent.mouseDown(screen.getByText(countryLabel));
}

describe("MainMenu", () => {
  beforeEach(() => {
    navigateMock.mockReset();
    setGameActiveMock.mockReset();
    setGameStateMock.mockReset();
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

      return null;
    });
    // MainMenu defers focus with requestAnimationFrame; defer one microtask so React
    // commits setFormErrors before focus runs (matches real rAF ordering).
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      queueMicrotask(() => cb(0));
      return 0;
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it.each(["es", "de", "fr", "it", "pt", "pt-BR"])(
    "stores the nationality as an ISO code and continues the flow in %s",
    async (language: string) => {
      translationState.language = language;

      render(<MainMenu />);

      openCreateManagerForm();
      fillManagerDetails();
      selectNationality(language, "ES");

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
          }),
        );
      });
      expect(setGameStateMock).toHaveBeenCalledWith({ id: "game-1" });
      expect(navigateMock).toHaveBeenCalledWith("/select-team");
    },
  );

  it("allows changing nationality after the other manager fields are filled", () => {
    render(<MainMenu />);

    openCreateManagerForm();
    fillManagerDetails();

    selectNationality("en", "ES");
    expect(
      screen.getByRole("button", {
        name: /spain/i,
      }),
    ).toBeInTheDocument();

    selectNationality("en", "DE");

    expect(
      screen.getByRole("button", {
        name: /germany/i,
      }),
    ).toBeInTheDocument();
  });

  it("allows selecting England instead of legacy GB", () => {
    render(<MainMenu />);

    openCreateManagerForm();
    fillManagerDetails();
    selectNationality("en", "ENG");

    expect(
      screen.getByRole("button", {
        name: /england/i,
      }),
    ).toBeInTheDocument();
  });

  it("preserves nationality when a stale date picker callback fires after selection", () => {
    render(<MainMenu />);

    openCreateManagerForm();
    fillManagerDetails();

    const staleDatePickerOnChange = latestDatePickerOnChange;

    selectNationality("en", "DE");

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

    openCreateManagerForm();
    fillManagerDetails();
    searchAndSelectNationality("pt", "AT", "austria");

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
        }),
      );
    });
  });

  it("focuses the first invalid field when submitting an empty Create Manager form", async () => {
    render(<MainMenu />);

    openCreateManagerForm();
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

    openCreateManagerForm();
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

    openCreateManagerForm();
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

    selectNationality("en", "ES");
    fireEvent.click(screen.getByText("createManager.chooseWorld"));

    await waitFor(() => {
      expect(screen.getByLabelText("manager-date-of-birth")).toHaveFocus();
    });
    expect(mockedInvoke).not.toHaveBeenCalledWith("list_world_databases");
    expect(screen.queryByTestId("world-select")).not.toBeInTheDocument();
  });
});
