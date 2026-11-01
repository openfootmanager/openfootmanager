import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PlayersPageQuery } from "../../services/playersService";
import type { PlayerData, TeamData } from "../../store/gameStore";
import { useSettingsStore } from "../../store/settingsStore";
import ScoutingPlayerSearchCard from "./ScoutingPlayerSearchCard";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => undefined) }));

vi.mock("../../hooks/usePositionGroups", async () => {
  const { cannedPositionsModule } = await import("../../test-utils/positionTable");
  return cannedPositionsModule();
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "scouting.findPlayers") return "Find Players";
      if (key === "scouting.searchPlaceholder") return "Search players";
      if (key === "scouting.player") return "Player";
      if (key === "scouting.pos") return "Pos";
      if (key === "scouting.age") return "Age";
      if (key === "scouting.team") return "Team";
      if (key === "scouting.value") return "Value";
      if (key === "scouting.action") return "Action";
      if (key === "scouting.scoutBtn") return "Scout";
      if (key === "scouting.previousPage") return "Previous page";
      if (key === "scouting.nextPage") return "Next page";
      if (key === "scouting.noPlayersFound") return "No players found";
      if (key === "scouting.noScoutsFree") return "No scouts free";
      if (key === "scouting.scoutingInProgress") {
        return "Scouting in progress";
      }
      if (key === "scouting.showingRange") {
        return `${params?.from}-${params?.to} of ${params?.total}`;
      }
      if (key === "common.all") return "All";
      if (key === "common.freeAgent") return "Free Agent";
      if (key === "common.viewTeam") return "View team";
      if (key === "squad.viewProfile") return "View profile";
      return typeof params?.defaultValue === "string" ? params.defaultValue : key;
    },
    i18n: { language: "en" },
  }),
}));

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
    match_name: "J. Smith",
    full_name: "John Smith",
    date_of_birth: "2000-01-01",
    nationality: "GB",
    position: "Forward",
    natural_position: "Forward",
    alternate_positions: [],
    training_focus: null,
    attributes: {
      pace: 60,
      stamina: 60,
      strength: 60,
      agility: 60,
      passing: 60,
      shooting: 60,
      tackling: 60,
      dribbling: 60,
      defending: 60,
      positioning: 60,
      vision: 60,
      decisions: 60,
      composure: 60,
      aggression: 60,
      teamwork: 60,
      leadership: 60,
      handling: 20,
      reflexes: 20,
      aerial: 60,
    },
    condition: 80,
    morale: 75,
    injury: null,
    team_id: "team-2",
    retired: false,
    contract_end: "2027-06-30",
    wage: 12000,
    market_value: 350000,
    stats: {
      appearances: 0,
      goals: 0,
      assists: 0,
      clean_sheets: 0,
      yellow_cards: 0,
      red_cards: 0,
      avg_rating: 0,
      minutes_played: 0,
    },
    career: [],
    transfer_listed: false,
    loan_listed: false,
    transfer_offers: [],
    traits: [],
    ...overrides,
  };
}

const baseQuery: PlayersPageQuery = {
  search: null,
  position: null,
  positions: [],
  age_min: null,
  age_max: null,
  ovr_min: null,
  ovr_max: null,
  value_min: null,
  value_max: null,
  in_budget: false,
  scope: "scoutable",
  team_id: null,
  status: "all",
  sort_key: "ovr",
  sort_asc: false,
  page: 1,
  page_size: 20,
};

describe("ScoutingPlayerSearchCard", () => {
  it("renders players and delegates search, filter, selection, scout, and pagination actions", () => {
    const onFilterChange = vi.fn();
    const onSelectPlayer = vi.fn();
    const onSelectTeam = vi.fn();
    const onSendScout = vi.fn();
    const onPreviousPage = vi.fn();
    const onNextPage = vi.fn();

    render(
      <ScoutingPlayerSearchCard
        players={[createPlayer()]}
        teams={[
          createTeam(),
          createTeam({ id: "team-2", name: "Beta FC", manager_id: "manager-2" }),
        ]}
        query={baseQuery}
        alreadyScoutingIds={new Set<string>()}
        availableScoutCount={1}
        sendingPlayerId={null}
        safePage={0}
        totalPages={2}
        totalPlayers={21}
        pageSize={20}
        onFilterChange={onFilterChange}
        onSelectPlayer={onSelectPlayer}
        onSelectTeam={onSelectTeam}
        onSendScout={onSendScout}
        onPreviousPage={onPreviousPage}
        onNextPage={onNextPage}
      />,
    );

    expect(screen.getByText("Find Players")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "John Smith" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Defender" }));
    expect(onFilterChange).toHaveBeenCalledWith({ positions: [{ group: "Defender" }] });

    fireEvent.change(screen.getByPlaceholderText("Search players"), {
      target: { value: "john" },
    });
    expect(onFilterChange).toHaveBeenCalledWith({ search: "john" });

    fireEvent.click(screen.getByRole("button", { name: "John Smith" }));
    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");

    fireEvent.click(screen.getByRole("button", { name: /^Scout$/i }));
    expect(onSendScout).toHaveBeenCalledWith("player-1");

    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(onNextPage).toHaveBeenCalledOnce();

    const playerRow = screen.getByText("John Smith").closest("tr");
    expect(playerRow).not.toBeNull();

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "View team" }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");

    fireEvent.contextMenu(playerRow as HTMLTableRowElement);
    fireEvent.click(screen.getByRole("menuitem", { name: "View profile" }));
    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");
  });

  function renderCard(player: PlayerData) {
    return render(
      <ScoutingPlayerSearchCard
        players={[player]}
        teams={[createTeam(), createTeam({ id: "team-2", name: "Beta FC" })]}
        query={baseQuery}
        alreadyScoutingIds={new Set<string>()}
        availableScoutCount={1}
        sendingPlayerId={null}
        safePage={0}
        totalPages={1}
        totalPlayers={1}
        pageSize={20}
        onFilterChange={vi.fn()}
        onSelectPlayer={vi.fn()}
        onSelectTeam={vi.fn()}
        onSendScout={vi.fn()}
        onPreviousPage={vi.fn()}
        onNextPage={vi.fn()}
      />,
    );
  }

  /**
   * Given a player whose `position` is still the legacy bucket and whose natural position is granular
   * When the search card renders the row
   * Then the badge reads the granular abbreviation.
   */
  it("shows the granular natural position, not the legacy bucket", () => {
    renderCard(createPlayer({ position: "Defender", natural_position: "LeftBack" }));

    expect(screen.getByText("LB")).toBeInTheDocument();
  });

  /**
   * Given a left back
   * When the search card renders the row
   * Then the badge takes the defender colour, not the forward fallback red.
   */
  it("colours a granular defender as a defender", () => {
    renderCard(createPlayer({ position: "LeftBack", natural_position: "LeftBack" }));

    expect(screen.getByText("LB").className).not.toContain("text-red-700");
  });
});

function setListView(view: "table" | "cards") {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, list_view: view },
  });
}

describe("ScoutingPlayerSearchCard cards view", () => {
  afterEach(() => setListView("table"));

  function renderCards(over: Partial<Parameters<typeof ScoutingPlayerSearchCard>[0]> = {}) {
    const handlers = {
      onSelectPlayer: vi.fn(),
      onSelectTeam: vi.fn(),
      onSendScout: vi.fn(),
    };
    render(
      <ScoutingPlayerSearchCard
        players={[
          createPlayer(),
          createPlayer({ id: "free", full_name: "Free Man", team_id: null }),
        ]}
        teams={[createTeam({ id: "team-2", name: "Beta FC" })]}
        query={baseQuery}
        alreadyScoutingIds={new Set<string>()}
        availableScoutCount={1}
        sendingPlayerId={null}
        safePage={0}
        totalPages={1}
        totalPlayers={2}
        pageSize={20}
        onFilterChange={vi.fn()}
        onPreviousPage={vi.fn()}
        onNextPage={vi.fn()}
        {...handlers}
        {...over}
      />,
    );
    return handlers;
  }

  /**
   * Given list_view is "cards"
   * When the search renders two players, one at a club and one a free agent
   * Then each is a card, there is no table, the free agent is labelled so, and no rating or wage
   *      is shown, as in the table.
   */
  it("draws each player as a card with no rating and no wage", () => {
    setListView("cards");
    renderCards();

    expect(screen.queryByRole("table")).toBeNull();
    const card = screen.getByRole("article", { name: "John Smith" });
    expect(within(card).getByText("Beta FC")).toBeInTheDocument();
    expect(within(card).queryByText("–")).toBeNull();
    expect(screen.getByRole("article", { name: "Free Man" })).toHaveTextContent("Free Agent");
  });

  /**
   * Given the cards view
   * When the card's name, its club and its Scout button are chosen
   * Then the player opens, the club is selected without opening him, and the scout is sent.
   */
  it("opens a player, selects his club and sends a scout from the card", () => {
    setListView("cards");
    const { onSelectPlayer, onSelectTeam, onSendScout } = renderCards();
    const card = screen.getByRole("article", { name: "John Smith" });

    fireEvent.click(within(card).getByRole("button", { name: "Beta FC" }));
    expect(onSelectTeam).toHaveBeenCalledWith("team-2");
    expect(onSelectPlayer).not.toHaveBeenCalled();

    fireEvent.click(within(card).getByRole("button", { name: "John Smith" }));
    expect(onSelectPlayer).toHaveBeenCalledWith("player-1");

    fireEvent.click(within(card).getByRole("button", { name: /^Scout$/i }));
    expect(onSendScout).toHaveBeenCalledWith("player-1");
  });

  /**
   * Given a player already being scouted, and a search with no free scout
   * When the cards render
   * Then the first card says so in progress, with no Scout button.
   */
  it("shows the scout's progress instead of the Scout button", () => {
    setListView("cards");
    renderCards({ alreadyScoutingIds: new Set(["player-1"]) });
    const card = screen.getByRole("article", { name: "John Smith" });

    expect(within(card).getByText("Scouting in progress")).toBeInTheDocument();
    expect(within(card).queryByRole("button", { name: /^Scout$/i })).toBeNull();
  });

  /**
   * Given the table view
   * When the player picks Cards in the view switch
   * Then the search becomes cards, and an empty search still says no player was found.
   */
  it("switches to cards, and keeps the empty message in both views", () => {
    renderCards();
    fireEvent.click(screen.getByRole("radio", { name: "common.listViewCards" }));
    expect(screen.getAllByRole("article")).toHaveLength(2);
    expect(screen.queryByRole("table")).toBeNull();
  });

  /**
   * Given the cards view, which has no column headers
   * When the sort field is changed to Age, and then the direction is flipped
   * Then the search is asked to sort by it: the same sort_key / sort_asc the query carries.
   */
  it("sorts the cards with the shared sort control", () => {
    setListView("cards");
    const onFilterChange = vi.fn();
    renderCards({ onFilterChange });

    fireEvent.click(screen.getByRole("combobox", { name: "common.sortBy" }));
    fireEvent.click(screen.getByRole("option", { name: "common.age" }));
    expect(onFilterChange).toHaveBeenCalledWith({ sort_key: "age", sort_asc: false });

    fireEvent.click(screen.getByRole("button", { name: "common.sortAscending" }));
    expect(onFilterChange).toHaveBeenCalledWith({ sort_key: "ovr", sort_asc: true });
  });

  /**
   * Given the cards view
   * When a card's Scout button and the no-scout note are drawn
   * Then the button shows keyboard focus with the offset ring, and the note takes a role token.
   */
  it("gives the scout action a focus ring and role-token colours", () => {
    setListView("cards");
    renderCards();
    expect(screen.getAllByRole("button", { name: /^Scout$/i })[0]).toHaveClass(
      "focus-visible:ring-focus",
      "focus-visible:ring-offset-2",
    );
  });

  it("shows the no-free-scout note in a role token and the type role size", () => {
    setListView("cards");
    renderCards({ availableScoutCount: 0 });
    const note = screen.getAllByText("No scouts free")[0];
    expect(note).toHaveClass("text-fg-muted", "text-caption");
    expect(note.className).not.toMatch(/text-gray|text-xs/);
  });

  it.each(["table", "cards"] as const)("says no player was found in the %s view", (view) => {
    setListView(view);
    renderCards({ players: [], totalPlayers: 0 });
    expect(screen.getByText("No players found")).toBeInTheDocument();
  });
});
