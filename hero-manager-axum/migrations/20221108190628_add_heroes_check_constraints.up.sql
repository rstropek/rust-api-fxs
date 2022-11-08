ALTER TABLE heroes ADD CONSTRAINT abilities_length_check CHECK (array_length(abilities, 1) BETWEEN 1 AND 5);
