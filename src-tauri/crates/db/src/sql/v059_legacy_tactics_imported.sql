-- Whether the player's old browser library of custom tactics has been copied into this career
-- (`Game::legacy_tactics_imported`). The copy runs once, so a tactic deleted afterwards is not
-- copied back from the browser's key. 0 for a save from before the mark: it is copied in once more
-- on the next load, then marked.
ALTER TABLE game_meta ADD COLUMN legacy_tactics_imported INTEGER NOT NULL DEFAULT 0;
