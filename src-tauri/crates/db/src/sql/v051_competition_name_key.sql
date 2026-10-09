-- NULL marks rows written before the key was stored, or competitions authored without one.
ALTER TABLE competitions ADD COLUMN name_key TEXT;
