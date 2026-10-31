import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ThemeProvider } from "../../context/ThemeContext";
import HalfTimeBreak from "./HalfTimeBreak";
import { makeGameState, makeSnapshot } from "./matchScreenFixtures";
import type { MatchEvent } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
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
  initReactI18next: { type: "3rdParty", init: () => {} },
  useTranslation: () => ({ t: (key: string) => key }),
}));

const awayGoal: MatchEvent = {
  minute: 31,
  event_type: "Goal",
  side: "Away",
  zone: "Midfield",
  player_id: null,
  secondary_player_id: null,
};

describe("HalfTimeBreak", () => {
  /**
   * Given an away club called Beta FC whose short name is BTA, not its name's first three letters
   * When the half-time break lists a first-half goal of the away side
   * Then the event row is badged with the club's short name, as the score header shows it.
   */
  it("badges a first-half event with its club's short name", () => {
    const snapshot = makeSnapshot();
    const gameState = makeGameState();
    gameState.teams = gameState.teams.map((team) =>
      team.id === snapshot.away_team.id ? { ...team, short_name: "BTA" } : team,
    );

    render(
      <ThemeProvider>
        <HalfTimeBreak
          snapshot={snapshot}
          gameState={gameState}
          userSide="Home"
          isSpectator
          importantEvents={[awayGoal]}
          onResume={() => {}}
          onUpdateSnapshot={() => {}}
        />
      </ThemeProvider>,
    );

    // The score header also shows the short name (in the crest), so read the first-half card alone.
    const eventsCard = screen.getByText("match.firstHalfEvents").parentElement as HTMLElement;
    expect(within(eventsCard).getByText("BTA")).toBeInTheDocument();
  });

  describe("the team talk", () => {
    beforeEach(() => {
      invokeMock.mockReset();
    });

    function renderAsManager() {
      render(
        <ThemeProvider>
          <HalfTimeBreak
            snapshot={makeSnapshot()}
            gameState={makeGameState()}
            userSide="Home"
            isSpectator={false}
            importantEvents={[]}
            onResume={() => {}}
            onUpdateSnapshot={() => {}}
          />
        </ThemeProvider>,
      );
    }

    async function chooseAndDeliverATalk() {
      const choices = screen.getAllByRole("button", { name: /match\.teamTalkOptions\..*\.label/ });
      fireEvent.click(choices[0]);
      fireEvent.click(screen.getByRole("button", { name: "match.deliverTeamTalk" }));
    }

    /**
     * Given the manager at half time, and a backend that refuses the talk (the career changed)
     * When they deliver a talk
     * Then the failure is shown, the talk is not marked delivered, and the choices stay so they
     * can try again: no morale was applied, so hiding the choices would lose the chance to talk.
     */
    it("keeps the choices and says so when the talk is refused", async () => {
      invokeMock.mockRejectedValue("be.error.careerChanged");
      renderAsManager();

      await chooseAndDeliverATalk();

      expect(await screen.findByRole("alert")).toHaveTextContent("match.teamTalkFailed");
      expect(screen.queryByText("match.delivered")).toBeNull();
      expect(screen.getByRole("button", { name: "match.deliverTeamTalk" })).toBeInTheDocument();
    });

    /**
     * Given the manager at half time and a backend that applies the talk
     * When they deliver a talk
     * Then it is marked delivered with each player's morale change, and no failure is shown.
     */
    it("marks the talk delivered and lists the morale changes when it works", async () => {
      invokeMock.mockResolvedValue([
        { player_id: "p1", player_name: "Pat Player", old_morale: 50, new_morale: 58, delta: 8 },
      ]);
      renderAsManager();

      await chooseAndDeliverATalk();

      await waitFor(() => expect(screen.getByText("match.delivered")).toBeInTheDocument());
      expect(screen.getByText("Pat Player")).toBeInTheDocument();
      expect(screen.queryByRole("alert")).toBeNull();
      expect(screen.queryByRole("button", { name: "match.deliverTeamTalk" })).toBeNull();
    });
  });
});
