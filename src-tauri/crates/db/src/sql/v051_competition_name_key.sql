ALTER TABLE competitions ADD COLUMN name_key TEXT;

-- Rows written before this column existed had World Cup names re-derived on load. Derive them
-- once here so later loads read the column verbatim. Only rows the game itself generates are
-- matched: the qualifying and playoff competitions by id prefix, the finals by the markers the
-- generator sets (priority 10000 and the name "World Cup <year>"). A package competition of the
-- same kind and scope, or even the same name, keeps NULL. Every other row stays NULL.
UPDATE competitions SET name_key = 'tournaments.competitions.worldCupQualifying'
    WHERE id LIKE 'world-cup-qualifying-%';
UPDATE competitions SET name_key = 'tournaments.competitions.worldCupPlayoff'
    WHERE id LIKE 'world-cup-playoff-%';
UPDATE competitions SET name_key = 'tournaments.competitions.worldCup'
    WHERE name_key IS NULL AND kind = 'InternationalNation' AND scope = 'International'
        AND priority = 10000 AND name GLOB 'World Cup [0-9]*';
