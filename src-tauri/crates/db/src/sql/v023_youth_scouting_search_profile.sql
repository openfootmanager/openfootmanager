ALTER TABLE youth_scouting_assignments
ADD COLUMN region TEXT NOT NULL DEFAULT 'Domestic';

ALTER TABLE youth_scouting_assignments
ADD COLUMN objective TEXT NOT NULL DEFAULT 'Balanced';
