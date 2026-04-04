export interface SaveEntry {
    id: string;
    name: string;
    manager_name: string;
    db_filename: string;
    checksum: string;
    created_at: string;
    last_played_at: string;
}

export interface WorldDatabaseInfo {
    id: string;
    name: string;
    description: string;
    team_count: number;
    player_count: number;
    source: string;
    path: string;
}