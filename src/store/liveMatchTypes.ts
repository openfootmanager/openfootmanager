// The live match as the backend sends it (the engine's snapshot, events and step results).
// They live in the store, not beside the match screens, so a service can name them too.

export type DangerBand = "Speculative" | "Decent" | "BigChance";
export type SaveQuality = "Routine" | "Strong" | "WorldClass";
export type FoulSeverity = "Soft" | "Hard" | "Reckless";
export type GoalContext = "Opener" | "Equaliser" | "Extends" | "Consolation";

// Serde externally-tagged representation of the Rust EventDetail enum.
export type EventDetail =
  | { Shot: { danger: DangerBand } }
  | { Save: { quality: SaveQuality } }
  | { Foul: { severity: FoulSeverity } }
  | { Goal: { context: GoalContext } };

export interface MatchEvent {
  minute: number;
  event_type: string;
  side: "Home" | "Away";
  zone: string;
  player_id: string | null;
  secondary_player_id: string | null;
  detail?: EventDetail | null;
}

export interface EnginePlayerData {
  id: string;
  name: string;
  position: string;
  ovr: number;
  condition: number;
  pace: number;
  stamina: number;
  strength: number;
  agility: number;
  passing: number;
  shooting: number;
  tackling: number;
  dribbling: number;
  defending: number;
  positioning: number;
  vision: number;
  decisions: number;
  composure: number;
  aggression: number;
  teamwork: number;
  leadership: number;
  handling: number;
  reflexes: number;
  aerial: number;
  traits: string[];
  role: string;
}

export interface EngineTeamData {
  id: string;
  name: string;
  formation: string;
  play_style: string;
  players: EnginePlayerData[];
}

export interface SetPieceTakers {
  free_kick_taker: string | null;
  corner_taker: string | null;
  penalty_taker: string | null;
  captain: string | null;
}

export interface SubstitutionRecord {
  minute: number;
  side: "Home" | "Away";
  player_off_id: string;
  player_on_id: string;
}

export interface PenaltyShootoutSnapshot {
  home_taken: number;
  away_taken: number;
  home_scored: number;
  away_scored: number;
  sudden_death: boolean;
  /**
   * The user's own side's order, before and during the shootout; the opponent's is always null
   * (hidden until each kick), and both are null when the user manages neither club.
   */
  home_order?: ShootoutOrder | null;
  away_order?: ShootoutOrder | null;
}

/**
 * One side's shootout order: everyone eligible to kick, in the order they kick. The first
 * `kickers` kick and the rest sit out; `kickers` is fixed by the side with fewer players.
 */
export interface ShootoutOrder {
  player_ids: string[];
  kickers: number;
}

export interface MatchSnapshot {
  phase: string;
  current_minute: number;
  home_score: number;
  away_score: number;
  possession: "Home" | "Away";
  ball_zone: string;
  home_team: EngineTeamData;
  away_team: EngineTeamData;
  home_bench: EnginePlayerData[];
  away_bench: EnginePlayerData[];
  home_possession_pct: number;
  away_possession_pct: number;
  events: MatchEvent[];
  home_subs_made: number;
  away_subs_made: number;
  max_subs: number;
  home_set_pieces: SetPieceTakers;
  away_set_pieces: SetPieceTakers;
  substitutions: SubstitutionRecord[];
  allows_extra_time: boolean;
  home_yellows: Record<string, number>;
  away_yellows: Record<string, number>;
  sent_off: string[];
  penalty_shootout?: PenaltyShootoutSnapshot | null;
  /** Engine marks out of ten by player id; filled once the match is over. */
  player_ratings?: Record<string, number>;
}

export interface MinuteResult {
  minute: number;
  phase: string;
  events: MatchEvent[];
  home_score: number;
  away_score: number;
  possession: "Home" | "Away";
  ball_zone: string;
  is_finished: boolean;
}
