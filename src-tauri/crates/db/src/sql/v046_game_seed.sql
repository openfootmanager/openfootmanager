-- The seed the game's own dice are derived from (`Game::seed`). A `u64` stored as
-- the `i64` with the same bits; 0 means a save from before games had one, which
-- the loader replaces with a seed derived from the save's id.
ALTER TABLE game_meta ADD COLUMN seed INTEGER NOT NULL DEFAULT 0;
