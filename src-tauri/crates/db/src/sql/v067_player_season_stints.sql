-- A player's season in progress split by club (`Player::season_stints`): the totals he arrived at
-- his club with and the clubs he has left since the season began, which the rollover turns into
-- one career row per club. NULL for a row written before the column, which reads as no stints:
-- the whole season then goes to the club that holds him, as it always did.
ALTER TABLE players ADD COLUMN season_stints TEXT;
