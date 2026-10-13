-- A stand-in manager (an assistant covering a vacant post) is marked, so the club can replace
-- them with a permanent appointment and the market never offers them as a candidate.
ALTER TABLE managers ADD COLUMN interim INTEGER NOT NULL DEFAULT 0;
