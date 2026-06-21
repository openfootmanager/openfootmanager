ALTER TABLE teams ADD COLUMN kit_pattern TEXT NOT NULL DEFAULT 'Solid'
    CHECK (kit_pattern IN ('Solid', 'Stripes', 'Hoops', 'HalfAndHalf', 'Diagonal'));
