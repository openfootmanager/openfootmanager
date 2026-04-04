import { invokeCommand } from "./tauriClient";

export interface AwardEntry {
    player_id: string;
    player_name: string;
    team_id: string;
    team_name: string;
    value: number;
}

export interface SeasonAwards {
    golden_boot: AwardEntry[];
    assist_king: AwardEntry[];
    player_of_year: AwardEntry[];
    clean_sheet_king: AwardEntry[];
    most_appearances: AwardEntry[];
    young_player: AwardEntry[];
}

export async function getSeasonAwards(): Promise<SeasonAwards> {
    return invokeCommand<SeasonAwards>("get_season_awards");
}