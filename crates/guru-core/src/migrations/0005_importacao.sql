ALTER TABLE investimentos ADD COLUMN codigo_b3 TEXT;

CREATE TABLE importacao (
    id INTEGER PRIMARY KEY,
    data TEXT NOT NULL,
    arquivo TEXT NOT NULL,
    linhas INTEGER NOT NULL,
    aplicadas INTEGER NOT NULL
);
