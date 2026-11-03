-- Read mail older than half a year, moved out of `messages` so the live inbox stays small. One
-- row per message, written once: the record is JSON of domain::message::InboxMessage, and an
-- existing id is never rewritten. `date` is kept as a column so the archive can be read newest first.
CREATE TABLE archived_messages (
    message_id   TEXT PRIMARY KEY,
    date         TEXT NOT NULL,
    record_json  TEXT NOT NULL
);
CREATE INDEX idx_archived_messages_date ON archived_messages (date DESC, message_id DESC);
