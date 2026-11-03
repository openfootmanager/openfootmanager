-- What a staff member is called when it differs from their name ("Pep" for Josep Guardiola i Sala).
-- NULL means nothing in particular: the display name is then their first and last name.
ALTER TABLE staff ADD COLUMN known_as TEXT;
