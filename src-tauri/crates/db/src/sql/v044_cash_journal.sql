CREATE TABLE IF NOT EXISTS cash_journal (
    id      TEXT PRIMARY KEY,
    club_id TEXT NOT NULL,
    amount  INTEGER NOT NULL,
    kind    TEXT NOT NULL,
    date    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS cash_journal_club_date ON cash_journal (club_id, date);
CREATE INDEX IF NOT EXISTS cash_journal_club_kind ON cash_journal (club_id, kind);
