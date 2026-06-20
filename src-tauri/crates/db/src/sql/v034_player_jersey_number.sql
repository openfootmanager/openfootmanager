ALTER TABLE players ADD COLUMN jersey_number INTEGER
    CHECK (jersey_number IS NULL OR jersey_number BETWEEN 1 AND 99)
    DEFAULT NULL;
