CREATE TABLE transfer_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    league_id       TEXT NOT NULL,
    date            TEXT NOT NULL,
    from_team_id    TEXT NOT NULL,
    to_team_id      TEXT NOT NULL,
    player_id       TEXT NOT NULL,
    fee             INTEGER NOT NULL
);
