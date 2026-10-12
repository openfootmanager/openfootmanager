-- Whether this career draws its World Cups the way it did before they were seeded from
-- the game (`Game::legacy_world_cup_draw`). 0 for a new game; the loader sets it to 1 for a
-- save written before the format that introduced it, and writes it back.
ALTER TABLE game_meta ADD COLUMN legacy_world_cup_draw INTEGER NOT NULL DEFAULT 0;
