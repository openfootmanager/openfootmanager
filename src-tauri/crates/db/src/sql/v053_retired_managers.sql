-- Unemployed managers who have left the active pool, frozen whole. One row per manager,
-- written once: the record is JSON of domain::manager::Manager, and an existing id is never
-- rewritten, so a field added to Manager later cannot make an old row "differ".
CREATE TABLE retired_managers (
    manager_id   TEXT PRIMARY KEY,
    record_json  TEXT NOT NULL
);
