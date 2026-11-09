-- NULL marks a pre-v066 row until GameDatabase backfills it through the normal competition
-- loader and the canonical club_league_table_in selector. A new player explicitly saves {}.
-- Keeping the marker in the row also makes an interrupted upgrade safe to retry.
ALTER TABLE players ADD COLUMN league_stats_by_competition TEXT;
