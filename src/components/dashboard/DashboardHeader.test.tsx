import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { MatchModeType } from "../../hooks/useAdvanceTime";
import { resetPositionGroupsCache } from "../../hooks/usePositionGroups";
import { getPositionGroups } from "../../services/positionService";
import type { PlayerData, TeamData } from "../../store/gameStore";
import DashboardHeader, { type DashboardMatchModeMeta } from "./DashboardHeader";

vi.mock("../../services/positionService", () => ({
  getPositionGroups: vi.fn(async () => []),
}));

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
      if (key === "dashboard.matchModeMenu") return "Choose match mode";
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
   * Given the header's Continue button, which has a chevron half that opens the match-mode menu
   * When that half is looked up by what a screen reader announces
   * Then it is named (the translated "Choose match mode") and shows a focus ring.
   */
  it("names the Continue dropdown toggle and gives it a focus ring", () => {
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
        teams={[]}
      />,
    );

    expect(screen.getByRole("button", { name: "Choose match mode" })).toHaveClass(
      "focus-visible:ring-focus",
    );
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
  // `natural_position` can pass. The badge is toned by the group the backend names for the
  // position (the mocked table below), the same way every other badge in the app is.
  it.each([
    ["Goalkeeper", "GK", "bg-accent-100", "Goalkeeper"],
    ["Defender", "DEF", "bg-primary-100", "Defender"],
    ["RightBack", "RB", "bg-primary-100", "Defender"],
    ["CenterBack", "CB", "bg-primary-100", "Defender"],
    ["LeftBack", "LB", "bg-primary-100", "Defender"],
    ["RightWingBack", "RWB", "bg-primary-100", "Defender"],
    ["LeftWingBack", "LWB", "bg-primary-100", "Defender"],
    ["Midfielder", "MID", "bg-positive-tint", "Midfielder"],
    ["DefensiveMidfielder", "DM", "bg-positive-tint", "Midfielder"],
    ["CentralMidfielder", "CM", "bg-positive-tint", "Midfielder"],
    ["AttackingMidfielder", "AM", "bg-positive-tint", "Midfielder"],
    ["RightMidfielder", "RM", "bg-positive-tint", "Midfielder"],
    ["LeftMidfielder", "LM", "bg-positive-tint", "Midfielder"],
    ["Forward", "FWD", "bg-danger-tint", "Forward"],
    ["RightWinger", "RW", "bg-danger-tint", "Forward"],
    ["LeftWinger", "LW", "bg-danger-tint", "Forward"],
    ["Striker", "ST", "bg-danger-tint", "Forward"],
  ])(
    "badges a %s the same way the rest of the app does",
    async (position, abbreviation, expectedClass, group) => {
      resetPositionGroupsCache();
      vi.mocked(getPositionGroups).mockResolvedValue([{ position, group }]);
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

      await waitFor(() =>
        expect(screen.getByText(abbreviation).parentElement).toHaveClass(expectedClass),
      );
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

describe("DashboardHeader search", () => {
  /**
   * Given a narrow header whose search field was opened from its button, with a result showing
   * When a keyboard player moves the focus from the field into the results and presses Enter
   * Then the field stays open while the focus is in the results, and Enter picks the player.
   */
  /**
   * Given a team result and a player result. The real Enter key needs user-event, which isn't a
   * dependency here; jsdom doesn't turn a keydown into a click on its own. So this pins what makes
   * Enter and Space work: each result is a native button (whose activation the HTML spec defines
   * as a click), it is named, and it takes the focus.
   */
  it.each([
    ["team", /^Alpha FC/, "onSelectSearchTeam", "team-1"],
    ["player", /John Striker/, "onSelectSearchPlayer", "player-1"],
  ] as const)(
    "keeps the results open while the focus moves into a %s result, and picks it",
    (_kind, name, handler, id) => {
      const handlers = { onSelectSearchTeam: vi.fn(), onSelectSearchPlayer: vi.fn() };
      const onSearchBlur = vi.fn();
      renderHeader({
        searchOpen: true,
        searchQuery: "al",
        matchedTeams: [createTeam()],
        matchedPlayers: [createPlayer()],
        ...handlers,
        onSearchBlur,
      });
      fireEvent.click(screen.getByRole("button", { name: "common.search" }));
      const field = screen.getByRole("textbox", { name: "common.search" });
      const result = screen.getByRole("button", { name });

      fireEvent.blur(field, { relatedTarget: result });
      result.focus();

      expect(result).toHaveFocus();
      expect(result.tagName).toBe("BUTTON");
      expect(field.closest("[data-search-field]")).not.toHaveClass("hidden");
      expect(onSearchBlur).not.toHaveBeenCalled();
      fireEvent.click(result);
      expect(handlers[handler]).toHaveBeenCalledWith(id);
      const other =
        handler === "onSelectSearchTeam" ? "onSelectSearchPlayer" : "onSelectSearchTeam";
      expect(handlers[other]).not.toHaveBeenCalled();
    },
  );

  /**
   * Given the results showing
   * When the mouse presses a result
   * Then the press doesn't take the focus from the field (WebKit doesn't focus a pressed button,
   * so the field would blur and the results vanish before the click lands).
   */
  it("keeps the focus in the field when a result is pressed with the mouse", () => {
    renderHeader({ searchOpen: true, searchQuery: "jo", matchedPlayers: [createPlayer()] });
    const result = screen.getByRole("button", { name: /John Striker/ });

    expect(fireEvent.mouseDown(result)).toBe(false);
  });

  /**
   * Given the search field opened from its button
   * When the focus leaves the search for something else
   * Then the field folds back to its button and the results close.
   */
  it("folds the field when the focus leaves the search", () => {
    const onSearchBlur = vi.fn();
    renderHeader({ onSearchBlur });
    fireEvent.click(screen.getByRole("button", { name: "common.search" }));
    const field = screen.getByRole("textbox", { name: "common.search" });

    fireEvent.blur(field, { relatedTarget: null });

    expect(field.closest("[data-search-field]")).toHaveClass("hidden");
    expect(onSearchBlur).toHaveBeenCalledTimes(1);
  });
});

describe("DashboardHeader in a narrow window (layout rule L5)", () => {
  /**
   * Given a header too narrow for the search field (a container query hides it there)
   * When the player presses the search button
   * Then the field opens and takes the focus, so search is one press away at any width.
   */
  it("opens a collapsed search field from its button and focuses it", () => {
    renderHeader();
    const field = screen.getByRole("textbox", { name: "common.search" });
    expect(field.closest("[data-search-field]")).toHaveClass("hidden");

    fireEvent.click(screen.getByRole("button", { name: "common.search" }));
    expect(field.closest("[data-search-field]")).not.toHaveClass("hidden");
    expect(field).toHaveFocus();
  });

  /**
   * Given the date with and without its weekday
   * When the header renders
   * Then the full date shows from 42rem of header width and the short one below it. jsdom can't
   * evaluate a container query, so this pins the breakpoint classes; the page harness's
   * screenshots show the switch.
   */
  it("swaps the full date for the short one below 42rem", () => {
    renderHeader();
    expect(screen.getByText("Tuesday, 1 July 2031")).toHaveClass("hidden", "@2xl/header:inline");
    expect(screen.getByText("1 July 2031")).toHaveClass("@2xl/header:hidden");
    expect(screen.getByText("1 July 2031")).not.toHaveClass("hidden");
  });

  /**
   * Given the Continue menu open, with its match modes and the skip action
   * When a keyboard user moves through it
   * Then every item shows the focus ring.
   */
  it("draws the focus ring on every Continue menu item", () => {
    renderHeader({ showContinueMenu: true });
    const items = ["Live", "Spectator", "Delegate", "continueMenu.skipToMatchDay"].map((name) =>
      screen.getByRole("button", { name: new RegExp(name) }),
    );
    for (const item of items) expect(item).toHaveClass("focus-visible:ring-focus");
  });
});
