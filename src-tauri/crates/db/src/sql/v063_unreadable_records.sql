-- A JSON cell that did not parse when the save was loaded, kept as it was: the load goes on with
-- the field's default, and the next save writes that default back over the cell, so without this
-- copy the original (a player's whole career, say) would be gone. Written only on load
-- (`json_column`), never by a save; one row per cell, the first text seen kept.
CREATE TABLE IF NOT EXISTS unreadable_records (
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    column_name TEXT NOT NULL,
    raw TEXT NOT NULL,
    error TEXT NOT NULL,
    found_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (table_name, row_id, column_name)
);
