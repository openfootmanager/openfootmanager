CREATE TABLE competitions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'League',
    scope TEXT NOT NULL DEFAULT 'Domestic',
    season INTEGER NOT NULL,
    region_id TEXT,
    country_id TEXT,
    required_region_ids_json TEXT NOT NULL DEFAULT '[]',
    participant_ids_json TEXT NOT NULL DEFAULT '[]',
    rules_json TEXT NOT NULL DEFAULT '{}',
    fixtures_json TEXT NOT NULL DEFAULT '[]',
    standings_json TEXT NOT NULL DEFAULT '[]',
    knockout_rounds_json TEXT NOT NULL DEFAULT '[]',
    transfer_log_json TEXT NOT NULL DEFAULT '[]',
    transfer_rumours_json TEXT NOT NULL DEFAULT '[]',
    priority INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE national_teams (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    football_nation TEXT NOT NULL,
    region_id TEXT,
    squad_player_ids_json TEXT NOT NULL DEFAULT '[]',
    manager_name TEXT,
    reputation INTEGER NOT NULL DEFAULT 500,
    fixtures_json TEXT NOT NULL DEFAULT '[]'
);
