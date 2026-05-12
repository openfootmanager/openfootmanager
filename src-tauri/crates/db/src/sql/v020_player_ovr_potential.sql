-- V20: Persist computed OVR and potential so they survive save/load without recomputation.
-- Existing players start at 0; the save_manager backfill will populate them once on first load
-- and then resave, making subsequent loads use the real values.
ALTER TABLE players ADD COLUMN ovr INTEGER NOT NULL DEFAULT 0;
ALTER TABLE players ADD COLUMN potential INTEGER NOT NULL DEFAULT 0;
