-- How many cash-journal posts existed when the last season ended
-- (`Game::last_season_rollover_journal_len`). Posts at this index and after
-- belong to the new season, so a settlement on the rollover calendar day
-- after the cutoff advanced still counts. NULL for a save from before the
-- column, which keeps the date-only cutoff until the next season end.
ALTER TABLE game_meta ADD COLUMN last_season_rollover_journal_len INTEGER;
