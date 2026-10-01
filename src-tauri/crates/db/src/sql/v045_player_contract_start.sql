-- A contract is an interval, not just an end date.
--
-- Nullable with no default: a player carried over from an earlier save has an end
-- date and no recorded start, and inventing one would fabricate employment
-- history. `NULL` means unknown, which the UI renders as unknown.
ALTER TABLE players ADD COLUMN contract_start TEXT;
