CREATE UNIQUE INDEX IF NOT EXISTS idx_players_team_jersey
    ON players(team_id, jersey_number)
    WHERE jersey_number IS NOT NULL;
