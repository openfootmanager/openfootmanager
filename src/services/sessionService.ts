import type {
  GameStateData,
  BoardObjective,
  FixtureData,
  ManagerData,
  ScoutedPlayerView,
  ScoutingAssignment,
  SeasonContextData,
  StandingData,
  TeamData,
  YouthScoutingAssignment,
} from "../store/types";
import { invoke } from "@tauri-apps/api/core";

export function getActiveGame(): Promise<GameStateData> {
  return invoke("get_active_game");
}

export function getActiveSaveId(): Promise<string | null> {
  return invoke("get_active_save_id");
}

export function saveGame(): Promise<void> {
  return invoke("save_game");
}

export function exitToMenu(): Promise<void> {
  return invoke("exit_to_menu");
}

/** Standing row with team name already resolved by the backend. */
export interface StandingRow extends StandingData {
  team_name: string;
}

/** Slim view of the manager's primary competition. */
export interface UserCompetitionSummary {
  competition_id: string;
  competition_name: string;
  name_key: string | null;
  standings: StandingRow[];
  upcoming_fixtures: FixtureData[];
  recent_fixtures: FixtureData[];
}

/**
 * KB-sized session payload returned by `get_session_state` and (in Phase 2)
 * by every hot command (advance_time, squad mutations, etc.).
 *
 * Contains only what the Home screen and header need — no players array,
 * no news bodies, no transfer history. Heavy collections live in slice commands.
 */
export interface SessionState {
  clock: {
    current_date: string;
    start_date: string;
  };
  manager: ManagerData;
  team: TeamData | null;
  season_context: SeasonContextData;
  board_objectives: BoardObjective[];
  scouting_assignments: ScoutingAssignment[];
  youth_scouting_assignments: YouthScoutingAssignment[];
  scouted_players: ScoutedPlayerView[];
  active_competition_ids: string[];
  unread_news_count: number;
  unread_messages_count: number;
  user_competition: UserCompetitionSummary | null;
}
