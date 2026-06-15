ALTER TABLE game_meta ADD COLUMN save_format_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE game_meta ADD COLUMN world_format_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE game_meta ADD COLUMN app_version TEXT NOT NULL DEFAULT '';
ALTER TABLE game_meta ADD COLUMN source_world_id TEXT NOT NULL DEFAULT '';
ALTER TABLE game_meta ADD COLUMN source_world_kind TEXT NOT NULL DEFAULT '';
ALTER TABLE game_meta ADD COLUMN active_region_ids_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE game_meta ADD COLUMN active_competition_ids_json TEXT NOT NULL DEFAULT '[]';
