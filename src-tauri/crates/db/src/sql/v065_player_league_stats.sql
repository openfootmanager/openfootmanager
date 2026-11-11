-- A player's league matches alone this season (`Player::league_stats`), which the division's
-- awards rank by. NULL for a row written before the column: the load reads that player's season so
-- far as his league season, so an upgraded save keeps its award standings until the season ends.
ALTER TABLE players ADD COLUMN league_stats TEXT;
