/** Engine-shaped match fixtures shared by the live match screen tests. */
import type { EnginePlayerData, EngineTeamData, MatchSnapshot } from "../components/match/types";

export const makePlayer = (overrides: Partial<EnginePlayerData> = {}): EnginePlayerData => ({
  id: "player-1",
  name: "Player One",
  position: "Midfielder",
  ovr: 70,
  condition: 78,
  pace: 70,
  stamina: 70,
  strength: 70,
  agility: 70,
  passing: 70,
  shooting: 70,
  tackling: 70,
  dribbling: 70,
  defending: 70,
  positioning: 70,
  vision: 70,
  decisions: 70,
  composure: 70,
  aggression: 60,
  teamwork: 70,
  leadership: 60,
  handling: 20,
  reflexes: 20,
  aerial: 60,
  traits: [],
  role: "Standard",
  ...overrides,
});

/** A full 4-4-2 XI, so the pitch uses its slot-aligned layout as it does in a real match. */
export function makeXi(prefix: string): EnginePlayerData[] {
  const positions = [
    "Goalkeeper",
    ...Array<string>(4).fill("Defender"),
    ...Array<string>(4).fill("Midfielder"),
    ...Array<string>(2).fill("Forward"),
  ];
  return positions.map((position, i) =>
    makePlayer({ id: `${prefix}-${i}`, name: `${prefix} Player${i}`, position }),
  );
}

export const makeTeam = (overrides: Partial<EngineTeamData> = {}): EngineTeamData => ({
  id: "home",
  name: "Alpha FC",
  formation: "4-4-2",
  play_style: "Balanced",
  players: makeXi("Home"),
  ...overrides,
});

export function createSnapshot(overrides: Partial<MatchSnapshot> = {}): MatchSnapshot {
  return {
    phase: "FirstHalf",
    current_minute: 30,
    home_score: 0,
    away_score: 0,
    possession: "Home",
    ball_zone: "Midfield",
    home_team: makeTeam(),
    away_team: makeTeam({ id: "away", name: "Beta United", players: makeXi("Away") }),
    home_bench: [],
    away_bench: [],
    home_possession_pct: 50,
    away_possession_pct: 50,
    events: [],
    home_subs_made: 0,
    away_subs_made: 0,
    max_subs: 5,
    home_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    away_set_pieces: {
      free_kick_taker: null,
      corner_taker: null,
      penalty_taker: null,
      captain: null,
    },
    substitutions: [],
    allows_extra_time: false,
    home_yellows: {},
    away_yellows: {},
    sent_off: [],
    player_ratings: {},
    player_goals: {},
    player_assists: {},
    ...overrides,
  };
}
