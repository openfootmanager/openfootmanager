// Shared types for match simulation components — mirrors Rust engine types

import type { TFunction } from "i18next";

// The live match's wire types live in the store, so services can use them; re-exported here
// for the match screens.
export type {
  DangerBand,
  SaveQuality,
  FoulSeverity,
  GoalContext,
  EventDetail,
  MatchEvent,
  EnginePlayerData,
  EngineTeamData,
  MatchSnapshot,
  MinuteResult,
  ShootoutOrder,
} from "../../store/liveMatchTypes";

export interface RoundResultSummary {
  fixture_id: string;
  home_team_id: string;
  home_team_name: string;
  away_team_id: string;
  away_team_name: string;
  home_goals: number;
  away_goals: number;
}

export interface CupTieResult extends RoundResultSummary {
  home_penalties: number | null;
  away_penalties: number | null;
  advancing_team_id: string | null;
  advancing_team_name: string | null;
}

export interface CupRoundSummary {
  competition_id: string;
  competition_name: string;
  round_name: string | null;
  is_complete: boolean;
  pending_fixture_count: number;
  results: CupTieResult[];
}

export interface StandingDelta {
  team_id: string;
  team_name: string;
  previous_position: number;
  current_position: number;
  points: number;
  points_delta: number;
}

export interface NotableUpset {
  fixture_id: string;
  favorite_team_id: string;
  favorite_team_name: string;
  favorite_strength: number;
  underdog_team_id: string;
  underdog_team_name: string;
  underdog_strength: number;
  strength_gap: number;
  home_goals: number;
  away_goals: number;
}

export interface TopScorerDelta {
  player_id: string;
  player_name: string;
  team_id: string;
  previous_rank: number;
  current_rank: number;
  previous_goals: number;
  current_goals: number;
}

export interface RoundSummary {
  matchday: number;
  is_complete: boolean;
  pending_fixture_count: number;
  completed_results: RoundResultSummary[];
  standings_delta: StandingDelta[];
  notable_upset: NotableUpset | null;
  top_scorer_delta: TopScorerDelta[];
}

export type SimSpeed = "paused" | "slow" | "normal" | "fast" | "instant";

// A "persistable" speed is a playback pace the user might want to resume from
// after half-time. `paused` is a temporary action; `instant` is a one-shot
// skip-to-end-of-half — persisting it would silently skip the next half.
// Only "slow" / "normal" / "fast" describe a pace worth locking in.
export function isPersistableSpeed(s: SimSpeed): s is "slow" | "normal" | "fast" {
  return s !== "paused" && s !== "instant";
}

export type MatchDayStage =
  | "prematch"
  | "first_half"
  | "halftime"
  | "second_half"
  | "extra_time_halftime"
  | "extra_time_second_half"
  | "penalty_shootout"
  | "postmatch"
  | "digest"
  | "press";

export type TeamTalkTone =
  | "calm"
  | "motivational"
  | "assertive"
  | "aggressive"
  | "praise"
  | "disappointed";

export interface TeamTalkOption {
  id: TeamTalkTone;
  label: string;
  description: string;
  icon: string;
}

const TEAM_TALK_OPTION_DEFINITIONS: Array<{
  id: TeamTalkTone;
  icon: string;
}> = [
  { id: "calm", icon: "calm" },
  { id: "motivational", icon: "motivational" },
  { id: "assertive", icon: "assertive" },
  { id: "aggressive", icon: "aggressive" },
  { id: "praise", icon: "praise" },
  { id: "disappointed", icon: "disappointed" },
];

export function getTeamTalkOptions(t: TFunction): TeamTalkOption[] {
  return TEAM_TALK_OPTION_DEFINITIONS.map(({ id, icon }) => ({
    id,
    icon,
    label: t(`match.teamTalkOptions.${id}.label`),
    description: t(`match.teamTalkOptions.${id}.description`),
  }));
}

export const SPEED_MS: Record<SimSpeed, number> = {
  paused: 0,
  slow: 2000,
  normal: 800,
  fast: 200,
  // 100ms, not 10ms. Each tick is two IPC round trips plus a full React render, which cannot
  // finish in 10ms — so the old value did not simulate faster, it just queued ticks back to back
  // and left the compositor no idle frames. In the live match, speed comes from MINUTES_PER_TICK
  // instead. A penalty shootout steps one kick per tick and so is genuinely paced by this value;
  // 100ms per kick is fast without being a blur.
  instant: 100,
};

/**
 * How many match minutes each tick advances.
 *
 * Simulating several minutes per round trip is much cheaper than several round trips: the backend
 * returns one `MinuteResult` per minute either way, but the expensive part — fetching the full
 * snapshot and re-rendering — happens once per tick rather than once per minute.
 *
 * Live match only. A penalty shootout must show every kick, so it always steps one at a time.
 */
export const MINUTES_PER_TICK: Record<SimSpeed, number> = {
  paused: 0,
  slow: 1,
  normal: 1,
  fast: 1,
  instant: 10,
};

export const FORMATIONS = [
  "4-4-2",
  "4-3-3",
  "3-5-2",
  "4-5-1",
  "4-2-3-1",
  "3-4-3",
  "5-3-2",
  "4-1-4-1",
];

export const PLAY_STYLES = [
  "Balanced",
  "Attacking",
  "Defensive",
  "Possession",
  "Counter",
  "HighPress",
] as const;

/** One swap the manager has lined up: the player coming off and the bench player replacing them. */
export interface SubstitutionSwap {
  offId: string;
  onId: string;
}

/**
 * How a batch of swaps went: all applied, or the first refused swap, the engine's reason and how
 * many swaps before it were made.
 */
export type SubstitutionOutcome =
  | { applied: true }
  | { applied: false; refused: SubstitutionSwap; reason: string; appliedCount: number };

/** What the engine said to a role change: taken, or refused with its translation key. */
export type RoleChangeOutcome = { applied: true } | { applied: false; reason: string };
