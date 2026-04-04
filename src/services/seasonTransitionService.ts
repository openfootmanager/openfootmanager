import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export interface EndOfSeasonSummary {
    season: number;
    league_name: string;
    champion_id: string;
    champion_name: string;
    user_position: number;
    user_points: number;
    user_won: number;
    user_drawn: number;
    user_lost: number;
    user_goals_for: number;
    user_goals_against: number;
    golden_boot_player: string;
    golden_boot_goals: number;
    poty_player: string;
    poty_rating: number;
    total_teams: number;
}

export interface AdvanceToNextSeasonResponse {
    game: GameStateData;
    summary: EndOfSeasonSummary;
}

export async function advanceToNextSeason(): Promise<AdvanceToNextSeasonResponse> {
    return invokeCommand<AdvanceToNextSeasonResponse>("advance_to_next_season");
}