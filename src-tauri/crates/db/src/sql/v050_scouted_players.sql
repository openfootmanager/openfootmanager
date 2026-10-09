-- What a finished scouting job revealed about a player: the date, and the attributes
-- as the scout saw them (JSON of PlayerAttributes). One row per player, replaced on a rescout.
CREATE TABLE scouted_players (
    player_id        TEXT PRIMARY KEY,
    scouted_on       TEXT NOT NULL,
    attributes_json  TEXT NOT NULL
);
