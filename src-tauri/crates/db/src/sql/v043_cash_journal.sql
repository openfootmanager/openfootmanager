CREATE TABLE IF NOT EXISTS cash_journal (
    id                   TEXT PRIMARY KEY,
    club_id              TEXT NOT NULL,
    amount               INTEGER NOT NULL,
    kind                 TEXT NOT NULL,
    date                 TEXT NOT NULL,
    envelope_generation  INTEGER NOT NULL DEFAULT 0,
    meta_json            TEXT NOT NULL DEFAULT '{}',
    reverses_id          TEXT
);
CREATE INDEX IF NOT EXISTS cash_journal_club_date ON cash_journal (club_id, date);
CREATE INDEX IF NOT EXISTS cash_journal_club_kind ON cash_journal (club_id, kind);
CREATE INDEX IF NOT EXISTS cash_journal_club_gen ON cash_journal (club_id, envelope_generation);

CREATE TABLE IF NOT EXISTS transfer_reservations (
    id              TEXT PRIMARY KEY,
    club_id         TEXT NOT NULL,
    player_id       TEXT NOT NULL,
    offer_id        TEXT NOT NULL,
    amount          INTEGER NOT NULL,
    created_on      TEXT NOT NULL,
    settles_on      TEXT,
    kind            TEXT NOT NULL DEFAULT 'transfer_fee',
    status          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS transfer_reservations_club_status
    ON transfer_reservations (club_id, status);

ALTER TABLE teams ADD COLUMN envelope_generation INTEGER NOT NULL DEFAULT 0;
