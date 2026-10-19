-- The day the last season ended and every player aged and developed (`Game::last_season_rollover_on`).
-- A scout's report taken before it is out of date. NULL for a save from before the column: no report
-- in it is called out of date until the next season ends and the column is filled.
ALTER TABLE game_meta ADD COLUMN last_season_rollover_on TEXT;
