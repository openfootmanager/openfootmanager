CREATE TABLE transfer_rumours (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    league_id TEXT NOT NULL,
    rumour_id TEXT NOT NULL,
    date TEXT NOT NULL,
    player_id TEXT NOT NULL,
    player_name TEXT NOT NULL,
    team_id TEXT NOT NULL,
    team_name TEXT NOT NULL,
    UNIQUE (league_id, rumour_id)
);

CREATE INDEX idx_transfer_rumours_league_date
    ON transfer_rumours (league_id, date DESC, id DESC);
