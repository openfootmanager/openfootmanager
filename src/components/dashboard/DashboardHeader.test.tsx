import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { MatchModeType } from "../../hooks/useAdvanceTime";
import type { PlayerData, TeamData } from "../../store/gameStore";
import DashboardHeader, { type DashboardMatchModeMeta } from "./DashboardHeader";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Honours defaultValue, as the real t does. translatePositionAbbreviation
    // relies on it to fall back to the position code, so a mock that ignored
    // it would render raw translation keys as badge text.
    t: (key: string, options?: { defaultValue?: string }) => {
      if (key === "dashboard.noResults") return "No results";
      if (key === "dashboard.searchPlayers") return "Players";
      if (key === "dashboard.searchTeams") return "Teams";
      if (key === "dashboard.searchPlaceholder") return "Search";
      if (key === "dashboard.saveGame") return "Save game";
      if (key === "common.save") return "Save";
      if (key === "dashboard.continue") return "Continue";
      if (key === "common.back") return "Back";
      if (key === "squad.viewProfile") return "View profile";
      if (key === "common.viewTeam") return "View team";
      return options?.defaultValue ?? key;
    },
  }),
}));

vi.mock("../ui", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../ui")>();

  return {
    ...actual,
    ThemeToggle: () => <div data-testid="theme-toggle" />,
  };
});

function createTeam(overrides: Partial<TeamData> = {}): TeamData {
  return {
    id: "team-1",
    name: "Alpha FC",
    short_name: "ALP",
    country: "GB",
    city: "London",
    stadium_name: "Alpha Ground",
    stadium_capacity: 30000,
    finance: 500000,
    manager_id: "manager-1",
    reputation: 50,
    wage_budget: 50000,
    transfer_budget: 250000,
    season_income: 0,
    season_expenses: 0,
    formation: "4-4-2",
    play_style: "Balanced",
    training_focus: "General",
    training_intensity: "Balanced",
    training_schedule: "Balanced",
    founded_year: 1900,
    colors: { primary: "#000000", secondary: "#ffffff" },
    starting_xi_ids: [],
    form: [],
    history: [],
    ...overrides,
  };
}

function createPlayer(overrides: Partial<PlayerData> = {}): PlayerData {
  return {
    id: "player-1",
    full_name: "John Striker",
    match_name: "J. Striker",
    team_id: "team-1",
    position: "Forward",
    natural_position: "Forward",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    market_value: 1000000,
    condition: 100,
    ...overrides,
  } as PlayerData;
}

function createModeMeta(): Record<MatchModeType, DashboardMatchModeMeta> {
  return {
    live: {
      desc: "Live",
      icon: <span>L</span>,
      label: "Live",
    },
    spectator: {
      desc: "Spectator",
      icon: <span>S</span>,
      label: "Spectator",
    },
    delegate: {
      desc: "Delegate",
      icon: <span>D</span>,
      label: "Delegate",
    },
  };
}

describe("DashboardHeader", () => {
  it("offers context menu actions for player search results", () => {
    const onSelectSearchPlayer = vi.fn();
    const onSelectSearchTeam = vi.fn();
    const teams = [createTeam()];

    render(
      <DashboardHeader
        title="Dashboard"
        currentDate="2026-08-10"
        currentDateShort="2026-08-10"
        hasProfileHistory={false}
        hasMatchToday={false}
        isAdvancing={false}
        isUnemployed={false}
        isSaving={false}
        matchMode="live"
        matchedPlayers={[createPlayer()]}
        matchedTeams={teams}
        modeMeta={createModeMeta()}
        onBack={vi.fn()}
        onContinue={vi.fn()}
        onSave={vi.fn()}
        onSearchBlur={vi.fn()}
        onSearchFocus={vi.fn()}
        onSearchQueryChange={vi.fn()}
        onSelectMatchMode={vi.fn()}
        onSelectSearchPlayer={onSelectSearchPlayer}
        onSelectSearchTeam={onSelectSearchTeam}
        onSkipToMatchDay={vi.fn()}
        onToggleContinueMenu={vi.fn()}
        saveFlash={false}
        searchOpen
        searchQuery="jo"
        seasonComplete={false}
        showContinueMenu={false}
        teams={teams}
      />,
    );

    fireEvent.contextMenu(screen.getByTestId("dashboard-search-player-player-1"));
    fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));

    expect(onSelectSearchTeam).toHaveBeenCalledWith("team-1");
    expect(onSelectSearchPlayer).not.toHaveBeenCalled();
  });

  /**
   * Given a profile open, so there is somewhere to go back to
   * When the manager uses the header's Back, the only Back on screen
   * Then it is found by its name and it goes back
   */
  it("offers a named Back that goes back when there is history", () => {
    const onBack = vi.fn();
    const teams = [createTeam()];

    render(
      <DashboardHeader
        title="J. Smith"
        currentDate="2026-08-10"
        currentDateShort="2026-08-10"
        hasProfileHistory
        hasMatchToday={false}
        isAdvancing={false}
        isUnemployed={false}
        isSaving={false}
        matchMode="live"
        matchedPlayers={[]}
        matchedTeams={[]}
        modeMeta={createModeMeta()}
        onBack={onBack}
        onContinue={vi.fn()}
        onSave={vi.fn()}
        onSearchBlur={vi.fn()}
        onSearchFocus={vi.fn()}
        onSearchQueryChange={vi.fn()}
        onSelectMatchMode={vi.fn()}
        onSelectSearchPlayer={vi.fn()}
        onSelectSearchTeam={vi.fn()}
        onSkipToMatchDay={vi.fn()}
        onToggleContinueMenu={vi.fn()}
        saveFlash={false}
        searchOpen={false}
        searchQuery=""
        seasonComplete={false}
        showContinueMenu={false}
        teams={teams}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(onBack).toHaveBeenCalledTimes(1);
  });

  // `position` is held at a legacy bucket that matches none of these rows, so only a read of
  // `natural_position` can pass. Every position positionBadgeVariant names explicitly. The dashboard used to
  // keep its own four-bucket copy, so each granular position rendered red here
  // while the rest of the app rendered it blue or green.
  it.each([
    ["Goalkeeper", "GK", "bg-accent-100"],
    ["Defender", "DEF", "bg-primary-100"],
    ["RightBack", "RB", "bg-primary-100"],
    ["CenterBack", "CB", "bg-primary-100"],
    ["LeftBack", "LB", "bg-primary-100"],
    ["RightWingBack", "RWB", "bg-primary-100"],
    ["LeftWingBack", "LWB", "bg-primary-100"],
    ["Midfielder", "MID", "bg-positive-tint"],
    ["DefensiveMidfielder", "DM", "bg-positive-tint"],
    ["CentralMidfielder", "CM", "bg-positive-tint"],
    ["AttackingMidfielder", "AM", "bg-positive-tint"],
    ["RightMidfielder", "RM", "bg-positive-tint"],
    ["LeftMidfielder", "LM", "bg-positive-tint"],
    ["Forward", "FWD", "bg-danger-tint"],
    ["RightWinger", "RW", "bg-danger-tint"],
    ["LeftWinger", "LW", "bg-danger-tint"],
    ["Striker", "ST", "bg-danger-tint"],
  ])(
    "badges a %s the same way the rest of the app does",
    (position, abbreviation, expectedClass) => {
      const teams = [createTeam()];

      render(
        <DashboardHeader
          title="Dashboard"
          currentDate="2026-08-10"
          currentDateShort="2026-08-10"
          hasProfileHistory={false}
          hasMatchToday={false}
          isAdvancing={false}
          isUnemployed={false}
          isSaving={false}
          matchMode="live"
          matchedPlayers={[createPlayer({ position: "Midfielder", natural_position: position })]}
          matchedTeams={teams}
          modeMeta={createModeMeta()}
          onBack={vi.fn()}
          onContinue={vi.fn()}
          onSave={vi.fn()}
          onSearchBlur={vi.fn()}
          onSearchFocus={vi.fn()}
          onSearchQueryChange={vi.fn()}
          onSelectMatchMode={vi.fn()}
          onSelectSearchPlayer={vi.fn()}
          onSelectSearchTeam={vi.fn()}
          onSkipToMatchDay={vi.fn()}
          onToggleContinueMenu={vi.fn()}
          saveFlash={false}
          searchOpen
          searchQuery="jo"
          seasonComplete={false}
          showContinueMenu={false}
          teams={teams}
        />,
      );

      expect(screen.getByText(abbreviation)).toHaveClass(expectedClass);
    },
  );
});

function renderHeader(overrides: Partial<React.ComponentProps<typeof DashboardHeader>> = {}) {
  const teams = [createTeam()];
  return render(
    <DashboardHeader
      title="Squad"
      currentDate="Tuesday, 1 July 2031"
      currentDateShort="1 July 2031"
      hasProfileHistory={false}
      hasMatchToday={false}
      isAdvancing={false}
      isUnemployed={false}
      isSaving={false}
      matchMode="live"
      matchedPlayers={[]}
      matchedTeams={[]}
      modeMeta={createModeMeta()}
      onBack={vi.fn()}
      onContinue={vi.fn()}
      onSave={vi.fn()}
      onSearchBlur={vi.fn()}
      onSearchFocus={vi.fn()}
      onSearchQueryChange={vi.fn()}
      onSelectMatchMode={vi.fn()}
      onSelectSearchPlayer={vi.fn()}
      onSelectSearchTeam={vi.fn()}
      onSkipToMatchDay={vi.fn()}
      onToggleContinueMenu={vi.fn()}
      saveFlash={false}
      searchOpen={false}
      searchQuery=""
      seasonComplete={false}
      showContinueMenu={false}
      teams={teams}
      {...overrides}
    />,
  );
}

describe("DashboardHeader in a narrow window (layout rule L5)", () => {
  /**
   * Given a header too narrow for the search field (a container query hides it there)
   * When the player presses the search button
   * Then the field opens and takes the focus, so search is one press away at any width.
   */
  it("opens a collapsed search field from its button and focuses it", () => {
    renderHeader();
    const field = screen.getByPlaceholderText("Search");
    expect(field.closest("[data-search-field]")).toHaveClass("hidden");

    fireEvent.click(screen.getByRole("button", { name: "common.search" }));
    expect(field.closest("[data-search-field]")).not.toHaveClass("hidden");
    expect(field).toHaveFocus();
  });

  /**
   * Given the date with and without its weekday
   * When the header renders
   * Then both are there: the short one shows when the header is narrow, the full one when wide.
   */
  it("carries the date with and without the weekday", () => {
    renderHeader();
    expect(screen.getByText("Tuesday, 1 July 2031")).toBeInTheDocument();
    expect(screen.getByText("1 July 2031")).toBeInTheDocument();
  });
});
