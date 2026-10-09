CREATE TABLE IF NOT EXISTS competition_edition_archive (
    competition_id TEXT NOT NULL,
    season INTEGER NOT NULL,
    record_json TEXT NOT NULL,
    PRIMARY KEY (competition_id, season)
);
