-- Tactics the player saved, kept with the career (not the club). `position` is the order they
-- were made in; `record_json` is domain::custom_tactic::CustomTactic.
CREATE TABLE custom_tactics (
    tactic_id    TEXT PRIMARY KEY,
    position     INTEGER NOT NULL,
    record_json  TEXT NOT NULL
);
