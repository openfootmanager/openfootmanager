-- The scout's opinion of a player's potential when he wrote the report (a range, both ends
-- inclusive), and the scout's name, kept with the report because a scout who has since left can't
-- be looked up. NULL for a report written before reports carried them: that player shows no
-- opinion until he is scouted again.
ALTER TABLE scouted_players ADD COLUMN potential_low INTEGER;
ALTER TABLE scouted_players ADD COLUMN potential_high INTEGER;
ALTER TABLE scouted_players ADD COLUMN scout_name TEXT;
