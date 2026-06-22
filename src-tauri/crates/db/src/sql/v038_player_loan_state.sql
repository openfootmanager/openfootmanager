ALTER TABLE players ADD COLUMN loan_offers TEXT NOT NULL DEFAULT '[]';
ALTER TABLE players ADD COLUMN active_loan TEXT;
