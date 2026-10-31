-- The day the last season ended (`Game::last_season_rollover_on`): every player aged then, and the
-- season's growth applied. A scout's report taken before it is out of date. NULL for a save from before
-- the column until it is loaded, when the day is taken from its last league season.
ALTER TABLE game_meta ADD COLUMN last_season_rollover_on TEXT;
