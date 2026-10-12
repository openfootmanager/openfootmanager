ALTER TABLE competitions ADD COLUMN name_key TEXT;

-- Rows written before this column existed had World Cup names re-derived on load. Derive them
-- once here, with the same predicates as ofm_core::world_cup (id prefix, or kind and scope),
-- so later loads read the column verbatim. Every other row stays NULL.
UPDATE competitions SET name_key = 'tournaments.competitions.worldCupQualifying'
    WHERE id LIKE 'world-cup-qualifying-%';
UPDATE competitions SET name_key = 'tournaments.competitions.worldCupPlayoff'
    WHERE id LIKE 'world-cup-playoff-%';
UPDATE competitions SET name_key = 'tournaments.competitions.worldCup'
    WHERE name_key IS NULL AND kind = 'InternationalNation' AND scope = 'International';
