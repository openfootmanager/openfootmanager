import { describe, it, expect, beforeEach } from "vitest";
import { useGameStore } from "./gameStore";
import type { GameStateData } from "./types";

// ---------------------------------------------------------------------------
// Reset store between tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  useGameStore.setState({
    hasActiveGame: false,
    managerName: null,
    gameState: null,
    isDirty: false,
  });
});

// Minimal GameStateData stub — only the fields the store cares about
const makeGameState = (overrides: Partial<GameStateData> = {}): GameStateData => ({
  clock: { current_date: "2026-08-01", start_date: "2026-08-01" },
  manager: {
    id: "mgr1", first_name: "Test", last_name: "Manager",
    date_of_birth: "1985-01-01", nationality: "GB", team_id: "team1",
    satisfaction: 50, fan_approval: 50, reputation: 500,
    career_stats: { matches_managed: 0, wins: 0, draws: 0, losses: 0, trophies: 0, best_finish: null },
    career_history: [],
  },
  teams: [],
  players: [],
  staff: [],
  messages: [],
  league: null,
  news: [],
  board_objectives: [],
  scouting_assignments: [],
  ...overrides,
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useGameStore", () => {
  it("starts with default values", () => {
    const state = useGameStore.getState();
    expect(state.hasActiveGame).toBe(false);
    expect(state.managerName).toBeNull();
    expect(state.gameState).toBeNull();
    expect(state.isDirty).toBe(false);
  });

  describe("setGameActive", () => {
    it("sets hasActiveGame and managerName", () => {
      useGameStore.getState().setGameActive(true, "John Doe");
      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(true);
      expect(state.managerName).toBe("John Doe");
    });

    it("sets managerName to null when not provided", () => {
      useGameStore.getState().setGameActive(true);
      expect(useGameStore.getState().managerName).toBeNull();
    });

    it("can deactivate the game", () => {
      useGameStore.getState().setGameActive(true, "Test");
      useGameStore.getState().setGameActive(false);
      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(false);
      expect(state.managerName).toBeNull();
    });
  });

  describe("setGameState", () => {
    it("stores game state data", () => {
      const gs = makeGameState();
      useGameStore.getState().setGameState(gs);
      expect(useGameStore.getState().gameState).toBe(gs);
    });

    it("prefers football_nation over raw nationality when hydrating entities", () => {
      const gs = makeGameState({
        manager: {
          ...makeGameState().manager,
          nationality: "GB",
          football_nation: "ENG",
        },
        players: [
          {
            id: "p1",
            match_name: "A. Allen",
            full_name: "Adam Allen",
            date_of_birth: "2008-01-01",
            nationality: "GB",
            football_nation: "ENG",
            position: "Goalkeeper",
            natural_position: "Goalkeeper",
            alternate_positions: [],
            training_focus: null,
            attributes: {
              pace: 50,
              stamina: 50,
              strength: 50,
              agility: 50,
              passing: 50,
              shooting: 50,
              tackling: 50,
              dribbling: 50,
              defending: 50,
              positioning: 50,
              vision: 50,
              decisions: 50,
              composure: 50,
              aggression: 50,
              teamwork: 50,
              leadership: 50,
              handling: 50,
              reflexes: 50,
              aerial: 50,
            },
            condition: 100,
            morale: 100,
            injury: null,
            team_id: "team1",
            retired: false,
            contract_end: null,
            wage: 0,
            market_value: 0,
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
          },
        ],
        staff: [
          {
            id: "s1",
            first_name: "Sam",
            last_name: "Coach",
            date_of_birth: "1980-01-01",
            nationality: "British",
            football_nation: "ENG",
            role: "Coach",
            attributes: {
              coaching: 70,
              judging_ability: 70,
              judging_potential: 70,
              physiotherapy: 30,
            },
            team_id: "team1",
            specialization: null,
            wage: 0,
            contract_end: null,
          },
        ],
      });

      useGameStore.getState().setGameState(gs);

      const hydrated = useGameStore.getState().gameState;
      expect(hydrated).not.toBe(gs);
      expect(hydrated?.manager.nationality).toBe("ENG");
      expect(hydrated?.players[0].nationality).toBe("ENG");
      expect(hydrated?.staff[0].nationality).toBe("ENG");
    });

    it("hydrates manager directories consistently and preserves transfer logs", () => {
      type ExtendedGameState = GameStateData & {
        managers: Array<{
          id: string;
          first_name: string;
          last_name: string;
          date_of_birth: string;
          nationality: string;
          football_nation?: string;
          reputation: number;
          satisfaction: number;
          fan_approval: number;
          team_id: string | null;
          career_stats: GameStateData["manager"]["career_stats"];
          career_history: GameStateData["manager"]["career_history"];
        }>;
        league: NonNullable<GameStateData["league"]> & {
          transfer_log: Array<{
            date: string;
            from_team_id: string;
            to_team_id: string;
            player_id: string;
            fee: number;
          }>;
        };
      };

      const gs = {
        ...makeGameState(),
        managers: [
          {
            ...makeGameState().manager,
            id: "mgr-ai-1",
            nationality: "GB",
            football_nation: "ENG",
            team_id: "team2",
          },
        ],
        league: {
          id: "league-1",
          name: "Premier Division",
          season: 2026,
          fixtures: [],
          standings: [],
          transfer_log: [
            {
              date: "2026-08-02",
              from_team_id: "team2",
              to_team_id: "team3",
              player_id: "p9",
              fee: 1200000,
            },
          ],
        },
      } as ExtendedGameState;

      useGameStore.getState().setGameState(gs);

      const hydrated = useGameStore.getState().gameState as ExtendedGameState | null;
      expect(hydrated?.managers[0].nationality).toBe("ENG");
      expect(hydrated?.league.transfer_log).toEqual(gs.league.transfer_log);
    });

    it("marks state as dirty", () => {
      useGameStore.getState().setGameState(makeGameState());
      expect(useGameStore.getState().isDirty).toBe(true);
    });

    it("replaces previous game state", () => {
      const gs1 = makeGameState({ clock: { current_date: "2026-08-01", start_date: "2026-08-01" } });
      const gs2 = makeGameState({ clock: { current_date: "2026-09-01", start_date: "2026-08-01" } });
      useGameStore.getState().setGameState(gs1);
      useGameStore.getState().setGameState(gs2);
      expect(useGameStore.getState().gameState?.clock.current_date).toBe("2026-09-01");
    });
  });

  describe("clearGame", () => {
    it("resets all fields to initial state", () => {
      useGameStore.getState().setGameActive(true, "Test Manager");
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().clearGame();

      const state = useGameStore.getState();
      expect(state.hasActiveGame).toBe(false);
      expect(state.managerName).toBeNull();
      expect(state.gameState).toBeNull();
      expect(state.isDirty).toBe(false);
    });
  });

  describe("isDirty / markClean", () => {
    it("is false initially", () => {
      expect(useGameStore.getState().isDirty).toBe(false);
    });

    it("becomes true after setGameState", () => {
      useGameStore.getState().setGameState(makeGameState());
      expect(useGameStore.getState().isDirty).toBe(true);
    });

    it("resets to false after markClean", () => {
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().markClean();
      expect(useGameStore.getState().isDirty).toBe(false);
    });

    it("resets to false after clearGame", () => {
      useGameStore.getState().setGameState(makeGameState());
      useGameStore.getState().clearGame();
      expect(useGameStore.getState().isDirty).toBe(false);
    });
  });
});
