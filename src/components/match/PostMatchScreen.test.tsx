import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PostMatchScreen, { computeGoalSources } from "./PostMatchScreen";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import type { MatchEvent } from "./types";
import { ThemeProvider } from "../../context/ThemeContext";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: query === "(prefers-color-scheme: dark)",
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, string | number>) => {
      if (key === "schedule.matchday") {
        return `Matchday ${params?.number}`;
      }
      if (key === "match.otherMatches") {
        return "Other Matches";
      }
      if (key === "match.otherMatchesToday") {
        return "Other Matches Today";
      }
      if (key === "match.otherMatchesUnavailable") {
        return "Other match context unavailable for this fixture yet.";
      }
      if (key === "match.viewDetails") {
        return "View details";
      }
      if (key === "match.matchDetails") {
        return "Match Details";
      }
      if (key === "common.close") {
        return "Close";
      }
      if (key === "match.roundSummaryUnavailable") {
        return "Round summary unavailable.";
      }
      if (key === "match.roundSummary") {
        return "Round Summary";
      }
      if (key === "home.leagueTable") {
        return "League Table";
      }
      if (key === "home.topScorers") {
        return "Top Scorers";
      }
      if (key === "home.noGoals") {
        return "No goals scored yet.";
      }
      if (key === "common.none") {
        return "None";
      }
      if (params?.team) {
        return `${key}:${params.team}`;
      }
      return key;
    },
  }),
}));

describe("PostMatchScreen", (): void => {
  it("renders the Team Talk tab by default for a manager", (): void => {
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getAllByText("match.postMatchTeamTalk").length).toBeGreaterThan(0);
    expect(screen.getByText("match.matchReport")).toBeInTheDocument();
    expect(screen.getByText("match.playerRatings")).toBeInTheDocument();
  });

  /**
   * Given a finished match whose snapshot carries the engine's ratings, one player unrated
   * When the ratings panel renders
   * Then it shows the engine's marks, titles them ratings, and shows a dash for the unrated player
   */
  it("shows the engine's ratings and a dash where the engine gave none", (): void => {
    const snapshot = {
      ...makeSnapshot(),
      player_ratings: { p1: 7.4 },
      events: [
        {
          minute: 30,
          event_type: "Goal",
          side: "Away" as const,
          zone: "x",
          player_id: "p2",
          secondary_player_id: null,
        },
      ],
    };
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getAllByText(/^match\.ratings:/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/^match\.performance/)).not.toBeInTheDocument();
    expect(screen.getAllByText("7.4").length).toBeGreaterThan(0);
    const bobRow = screen
      .getAllByText("Bob")
      .find((node) => node.previousElementSibling?.textContent === "–");
    expect(bobRow).toBeDefined();
  });

  /**
   * Given a starter substituted off before the engine's minutes threshold and an unused substitute
   * When the ratings panel renders
   * Then the starter is listed with a dash and the unused substitute is not listed
   */
  it("lists a player who was on the pitch but unrated, and hides one who never played", (): void => {
    const base = makeSnapshot();
    const benchPlayer = (id: string, name: string) => ({ ...base.home_team.players[0], id, name });
    const snapshot = {
      ...base,
      player_ratings: { p1: 7.0 },
      home_bench: [benchPlayer("early-off", "Early Off"), benchPlayer("unused", "Unused Sub")],
      substitutions: [
        { minute: 6, side: "Home" as const, player_off_id: "early-off", player_on_id: "p1" },
      ],
    };
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    const earlyOffRow = screen
      .getAllByText("Early Off")
      .find((node) => node.previousElementSibling?.textContent === "–");
    expect(earlyOffRow).toBeDefined();
    expect(screen.queryByText("Unused Sub")).not.toBeInTheDocument();
  });

  it("switches to Match Report tab and shows scorers section", (): void => {
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByText("match.matchReport"));

    expect(screen.getByText("match.scorers")).toBeInTheDocument();
    expect(screen.getByText("match.quickStats")).toBeInTheDocument();
  });

  /**
   * Given an away club called Beta FC whose short name is BTA, not its name's first three letters
   * When the match report lists a key event of the away side
   * Then the event is badged with the club's short name, as the score header shows it.
   */
  it("badges a key event with its club's short name", (): void => {
    const snapshot = makeSnapshot();
    const gameState = makeGameState();
    gameState.teams = gameState.teams.map((team) =>
      team.id === snapshot.away_team.id ? { ...team, short_name: "BTA" } : team,
    );
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={gameState}
          userSide="Home"
          isSpectator={false}
          importantEvents={[{ ...makeEvent("Goal", "Away"), minute: 37 }]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByRole("tab", { name: "match.matchReport" }));

    // The score header also shows the short name (in the crest), so read the events card alone.
    const eventsCard = screen.getByText("match.matchEvents").parentElement as HTMLElement;
    expect(within(eventsCard).getByText("BTA")).toBeInTheDocument();
  });

  it("renders Match Report tab by default for a spectator", (): void => {
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide={null}
          isSpectator={true}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("match.scorers")).toBeInTheDocument();
  });

  it("calls onContinue when manager clicks Continue", (): void => {
    const onContinue = vi.fn();
    const onFinish = vi.fn();
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={onContinue}
          onFinish={onFinish}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByText("match.continue"));
    expect(onContinue).toHaveBeenCalledTimes(1);
    expect(onFinish).not.toHaveBeenCalled();
  });

  it("calls onFinish when spectator clicks Continue to Dashboard", (): void => {
    const onContinue = vi.fn();
    const onFinish = vi.fn();
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide={null}
          isSpectator={true}
          importantEvents={[]}
          onContinue={onContinue}
          onFinish={onFinish}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByText("match.continueDashboard"));
    expect(onFinish).toHaveBeenCalledTimes(1);
    expect(onContinue).not.toHaveBeenCalled();
  });

  it("resolves a level score via the shootout and shows the pens score", (): void => {
    // Regression: shootout kicks used to be folded into the match score, so a
    // 1-1 tie won on penalties displayed as 5-4 and the verdict came from the
    // inflated score. Now the score stays level and the shootout decides.
    const snapshot = {
      ...makeSnapshot(),
      home_score: 1,
      away_score: 1,
      penalty_shootout: {
        home_taken: 6,
        away_taken: 6,
        home_scored: 5,
        away_scored: 4,
        sudden_death: true,
      },
    };
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("match.victory")).toBeInTheDocument();
    expect(screen.queryByText("match.draw")).not.toBeInTheDocument();
    expect(screen.getByText(/match\.pen 5–4/)).toBeInTheDocument();
  });

  it("keeps a level score without a shootout as a draw", (): void => {
    const snapshot = {
      ...makeSnapshot(),
      home_score: 1,
      away_score: 1,
    };
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("match.draw")).toBeInTheDocument();
    expect(screen.queryByText("match.victory")).not.toBeInTheDocument();
    expect(screen.queryByText(/match\.pen \d/)).not.toBeInTheDocument();
  });

  it("shows a defeat verdict when the user loses the shootout", (): void => {
    const snapshot = {
      ...makeSnapshot(),
      home_score: 1,
      away_score: 1,
      penalty_shootout: {
        home_taken: 5,
        away_taken: 5,
        home_scored: 3,
        away_scored: 4,
        sudden_death: false,
      },
    };
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={snapshot}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={() => {}}
          onFinish={() => {}}
        />
      </ThemeProvider>,
    );

    expect(screen.getByText("match.defeat")).toBeInTheDocument();
  });

  it("calls onFinish when manager clicks Skip", (): void => {
    const onContinue = vi.fn();
    const onFinish = vi.fn();
    render(
      <ThemeProvider>
        <PostMatchScreen
          snapshot={makeSnapshot()}
          gameState={makeGameState()}
          userSide="Home"
          isSpectator={false}
          importantEvents={[]}
          onContinue={onContinue}
          onFinish={onFinish}
        />
      </ThemeProvider>,
    );

    fireEvent.click(screen.getByText("match.skip"));
    expect(onFinish).toHaveBeenCalledTimes(1);
    expect(onContinue).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// computeGoalSources unit tests
// ---------------------------------------------------------------------------

function makeEvent(event_type: string, side: "Home" | "Away", zone = "Midfield"): MatchEvent {
  return { minute: 1, event_type, side, zone, player_id: null, secondary_player_id: null };
}

describe("computeGoalSources", (): void => {
  it("counts open-play goals", (): void => {
    const events: MatchEvent[] = [makeEvent("Goal", "Home")];
    expect(computeGoalSources(events, "Home")).toEqual({
      openPlay: 1,
      corners: 0,
      freekicks: 0,
      penalties: 0,
    });
  });

  it("counts corner goals for the correct side only", (): void => {
    // Away earns a corner, but Home scores — should be open play for Home, not corner
    const events: MatchEvent[] = [makeEvent("Corner", "Away"), makeEvent("Goal", "Home")];
    expect(computeGoalSources(events, "Home")).toEqual({
      openPlay: 1,
      corners: 0,
      freekicks: 0,
      penalties: 0,
    });
    // Away scoring after their own corner is a corner goal
    const events2: MatchEvent[] = [makeEvent("Corner", "Away"), makeEvent("Goal", "Away")];
    expect(computeGoalSources(events2, "Away")).toEqual({
      openPlay: 0,
      corners: 1,
      freekicks: 0,
      penalties: 0,
    });
  });

  it("only counts attacking-third free kicks as set-piece windows", (): void => {
    // Home FK in HomeDefense (their own third) should NOT open a set-piece window
    const defensive = makeEvent("FreeKick", "Home", "HomeDefense");
    const goal = makeEvent("Goal", "Home");
    expect(computeGoalSources([defensive, goal], "Home")).toEqual({
      openPlay: 1,
      corners: 0,
      freekicks: 0,
      penalties: 0,
    });
    // Home FK in AwayDefense (their attacking third) SHOULD open a window
    const attacking = makeEvent("FreeKick", "Home", "AwayDefense");
    expect(computeGoalSources([attacking, goal], "Home")).toEqual({
      openPlay: 0,
      corners: 0,
      freekicks: 1,
      penalties: 0,
    });
  });

  it("clears the set-piece window on defensive events", (): void => {
    const events: MatchEvent[] = [
      makeEvent("Corner", "Home"),
      makeEvent("Clearance", "Away"),
      makeEvent("Goal", "Home"),
    ];
    expect(computeGoalSources(events, "Home")).toEqual({
      openPlay: 1,
      corners: 0,
      freekicks: 0,
      penalties: 0,
    });
  });

  it("counts penalty goals", (): void => {
    const events: MatchEvent[] = [makeEvent("PenaltyGoal", "Home")];
    expect(computeGoalSources(events, "Home")).toEqual({
      openPlay: 0,
      corners: 0,
      freekicks: 0,
      penalties: 1,
    });
  });
});
