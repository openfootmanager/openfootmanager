import type { PlayerAdvancedStatsSummary } from "../components/playerProfile/PlayerProfile.helpers";
import type { PlayerRecentMatchEntry } from "../components/playerProfile/PlayerProfileRecentMatchesCard";
import { invokeCommand } from "./tauriClient";

export async function getPlayerStatsOverview(
    playerId: string,
): Promise<PlayerAdvancedStatsSummary> {
    return invokeCommand<PlayerAdvancedStatsSummary>("get_player_stats_overview", {
        playerId,
    });
}

export async function getPlayerMatchHistory(
    playerId: string,
    limit = 5,
): Promise<PlayerRecentMatchEntry[]> {
    return invokeCommand<PlayerRecentMatchEntry[]>("get_player_match_history", {
        playerId,
        limit,
    });
}