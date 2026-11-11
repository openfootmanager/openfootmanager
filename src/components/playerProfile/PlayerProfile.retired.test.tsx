import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { i18nReady } from "../../i18n";
import type { GameStateData, PlayerData, StaffData } from "../../store/gameStore";
import { createPlayer, createSeasonContext, createTeam } from "../../test-utils/factories";
import PlayerProfile from "./PlayerProfile";

// A retired player is not in the world's players: the profile fetches him from the archive. These
// pin what it says of him (#1348): that he has retired, when and from which club; and what it no
// longer offers: scouting and the Actions menu.

beforeAll(async () => {
  await i18nReady;
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../hooks/useRatingTiers", async () => {
  const { cannedRatingTiersModule } = await import("../../test-utils/ratingTiers");
  return cannedRatingTiersModule();
});
vi.mock("../../hooks/useContractWarningStages", () => ({
  useContractRiskLevels: () => () => "stable",
}));
vi.mock("../../hooks/usePlayerKnowledge", () => ({
  usePlayerKnowledge: () => () => ({ kind: "unknown" }),
}));
vi.mock("../../hooks/useVisibleWage", () => ({
  useVisibleWage: () => ({ status: "answered", value: { kind: "hidden" } }),
}));
vi.mock("../../hooks/usePotentialOpinion", () => ({
  usePotentialOpinion: () => ({ status: "answered", value: null }),
}));
vi.mock("../../hooks/usePositionRatings", () => ({
  usePositionRatings: () => ({ status: "answered", value: null }),
}));

function gameWith(player: PlayerData): GameStateData {
  return {
    season_context: createSeasonContext(),
    clock: { current_date: "2026-08-01T00:00:00Z", start_date: "2026-07-01T00:00:00Z" },
    manager: {
      display_name: "Jane Doe",
      id: "manager-1",
      first_name: "Jane",
      last_name: "Doe",
      date_of_birth: "1980-01-01",
      nationality: "GB",
      reputation: 50,
      satisfaction: 50,
      fan_approval: 50,
      team_id: "team-1",
      career_stats: {
        matches_managed: 0,
        wins: 0,
        draws: 0,
        losses: 0,
        trophies: 0,
        best_finish: null,
      },
      career_history: [],
    },
    teams: [createTeam({ id: "team-1", name: "Alpha FC" })],
    players: [player],
    staff: [],
    messages: [],
    news: [],
    league: { id: "league-1", name: "League", season: 1, fixtures: [], standings: [] },
    scouting_assignments: [],
    board_objectives: [],
  } as unknown as GameStateData;
}

const retiree = createPlayer({
  id: "retiree-1",
  match_name: "Old Timer",
  full_name: "Oscar Timer",
  team_id: null,
  retired: true,
  career: [
    {
      season: 2030,
      team_id: "team-1",
      team_name: "Alpha FC",
      appearances: 30,
      goals: 2,
      assists: 1,
    },
  ],
  movement_history: [
    {
      date: "2031-06-30",
      kind: "retired",
      from_team_id: "team-1",
      from_team_name: "Alpha FC",
      to_team_id: null,
      to_team_name: null,
    },
  ],
});

// The club has a scout, so a player it could scout shows the button.
const scout = {
  id: "scout-1",
  first_name: "Sam",
  last_name: "Scout",
  display_name: "Sam Scout",
  date_of_birth: "1980-01-01",
  nationality: "GB",
  role: "Scout",
  attributes: { coaching: 20, judgingAbility: 70, judgingPotential: 70, physiotherapy: 20 },
  team_id: "team-1",
  specialization: "General",
  wage: 1200,
  contract_end: "2027-06-30",
} as unknown as StaffData;

function renderRetiree(player: PlayerData = retiree) {
  return render(
    <PlayerProfile
      player={player}
      gameState={{ ...gameWith(player), staff: [scout] }}
      isOwnClub={false}
      onGameUpdate={() => {}}
      onSelectTeam={() => {}}
    />,
  );
}

async function answer(command: string) {
  if (command === "get_player_stats_overview") {
    const none = { total: 0, per90: null, percentile: null };
    return {
      percentileEligible: false,
      metrics: {
        shots: none,
        shotsOnTarget: none,
        passes: { completed: 0, attempted: 0, accuracy: null, percentile: null },
        tacklesWon: none,
        interceptions: none,
        foulsCommitted: none,
      },
    };
  }
  if (command === "get_player_match_history") return [];
  return null;
}

beforeEach(() => {
  vi.mocked(invoke).mockImplementation(answer);
});

describe("the profile of a retired player", () => {
  /**
   * Given a player who retired on 30 June 2031 from Alpha FC
   * When his profile opens
   * Then the name card says he has retired, when, and his last club, and does not call him a
   *      free agent
   */
  it("says he has retired, when, and his last club", () => {
    renderRetiree();

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Retired")).toBeInTheDocument();
    expect(within(card).getByText(/Retired on .*2031/)).toBeInTheDocument();
    expect(within(card).getByText("Last club: Alpha FC")).toBeInTheDocument();
    expect(within(card).queryByText("Free Agent")).toBeNull();
  });

  /**
   * Given the same retired player
   * When his profile opens
   * Then nothing offers to scout him, and there is no Actions menu: there is no transfer or
   *      contract to act on
   */
  it("offers no scouting and no actions", () => {
    // The same page for a free agent he could scout does offer it: the absence below means something.
    const freeAgent = { ...retiree, id: "free-1", retired: false, movement_history: [] };
    const { unmount } = renderRetiree(freeAgent);
    expect(screen.getByRole("button", { name: /scout/i })).toBeInTheDocument();
    unmount();

    renderRetiree();

    expect(screen.queryByRole("button", { name: /scout/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /actions/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /offer contract/i })).toBeNull();
  });

  /**
   * Given a retired player who left no retirement movement and no career
   * When his profile opens
   * Then it still says he has retired, without a date or a club it cannot know
   */
  it("says only that he has retired when the archive kept no date or club", () => {
    renderRetiree({ ...retiree, movement_history: [], career: [] });

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Retired")).toBeInTheDocument();
    expect(within(card).queryByText(/Last club/)).toBeNull();
    expect(within(card).queryByText(/Retired on/)).toBeNull();
  });

  /**
   * Given the same retired player
   * When his Career tab is opened
   * Then his career history is there to read
   */
  it("keeps his career history readable", () => {
    renderRetiree();

    fireEvent.click(screen.getByRole("tab", { name: "Career" }));

    expect(
      within(screen.getByRole("tabpanel", { name: "Career" })).getAllByText("Alpha FC").length,
    ).toBeGreaterThan(0);
  });

  /**
   * Given a retirement the way the game writes it: a club id and no club name, and a career
   * When his profile opens
   * Then the last club is the name his career gives that club
   */
  it("names the last club from his career when the movement carries only the club's id", () => {
    renderRetiree({
      ...retiree,
      movement_history: [
        { date: "2031-06-30", kind: "retired", from_team_id: "team-1", from_team_name: null },
      ],
    });

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Last club: Alpha FC")).toBeInTheDocument();
  });

  /**
   * Given a retirement movement that names a club and a career whose latest row is another club's
   * When his profile opens
   * Then the movement's club is the last club: it is where he retired from
   */
  it("prefers the club the retirement movement names over his career's latest row", () => {
    renderRetiree({
      ...retiree,
      career: [
        {
          season: 2030,
          team_id: "team-2",
          team_name: "Beta FC",
          appearances: 1,
          goals: 0,
          assists: 0,
        },
      ],
    });

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Last club: Alpha FC")).toBeInTheDocument();
  });

  /**
   * Given a career whose latest row is the one the game writes for a season spent without a club
   *       (team_name "Free Agent", no club id), after a season at Alpha FC
   * When his profile opens
   * Then the last club is Alpha FC: a free agent is not a club, and the card must not say so in
   *      English in every language
   */
  it("skips a free-agent season when naming his last club", () => {
    renderRetiree({
      ...retiree,
      movement_history: [{ date: "2031-06-30", kind: "retired", from_team_id: null }],
      career: [
        {
          season: 2029,
          team_id: "team-1",
          team_name: "Alpha FC",
          appearances: 20,
          goals: 1,
          assists: 0,
        },
        {
          season: 2030,
          team_id: "",
          team_name: "Free Agent",
          appearances: 0,
          goals: 0,
          assists: 0,
        },
      ],
    });

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Last club: Alpha FC")).toBeInTheDocument();
    expect(within(card).queryByText(/Free Agent/)).toBeNull();
  });

  /**
   * Given a career that is only free-agent seasons, and no named club on the movement
   * When his profile opens
   * Then he has no last club to show: just "Retired"
   */
  it("shows no last club when every season of his career was spent without one", () => {
    renderRetiree({
      ...retiree,
      movement_history: [],
      career: [
        {
          season: 2030,
          team_id: "",
          team_name: "Free Agent",
          appearances: 0,
          goals: 0,
          assists: 0,
        },
      ],
    });

    const card = screen.getByRole("region", { name: "Oscar Timer" });
    expect(within(card).getByText("Retired")).toBeInTheDocument();
    expect(within(card).queryByText(/Last club/)).toBeNull();
  });
});
