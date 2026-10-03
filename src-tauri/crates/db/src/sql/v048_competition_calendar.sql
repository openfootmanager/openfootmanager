-- NULL marks old competition data requiring provenance adoption and resave.
ALTER TABLE competitions ADD COLUMN calendar_json TEXT;
