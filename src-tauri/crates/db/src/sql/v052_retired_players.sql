-- People who have stopped playing, frozen at retirement. One row per player, written once:
-- the record is JSON of domain::retired_player::RetiredPlayer, and an existing id is never
-- rewritten, so a field added to the record later cannot make an old row "differ".
CREATE TABLE retired_players (
    player_id    TEXT PRIMARY KEY,
    record_json  TEXT NOT NULL
);
