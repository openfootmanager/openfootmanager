-- The first date a completed home fixture is paid a gate for (`Game::gate_settled_from`). NULL for
-- a save from before per-fixture gates: its first settlement fixes the date, reaching back only as
-- far as the old weekly gate did, so fixtures it had already played are not paid again.
ALTER TABLE game_meta ADD COLUMN gate_settled_from TEXT;
