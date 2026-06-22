ALTER TABLE teams ADD COLUMN player_roles_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE teams ADD COLUMN tactics_phase_json TEXT NOT NULL DEFAULT '{}';
