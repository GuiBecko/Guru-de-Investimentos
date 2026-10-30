-- Dados oficiais de mercado (CDI/Selic, IPCA, Tesouro Direto, B3).
CREATE TABLE indice_diario (
    serie TEXT NOT NULL,
    data  TEXT NOT NULL,
    valor TEXT NOT NULL,
    PRIMARY KEY (serie, data)
);

CREATE TABLE ipca_mensal (
    mes   TEXT PRIMARY KEY, -- YYYY-MM
    valor TEXT NOT NULL
);

CREATE TABLE tesouro_preco (
    tipo        TEXT NOT NULL,
    vencimento  TEXT NOT NULL,
    data        TEXT NOT NULL,
    taxa_compra TEXT NOT NULL,
    pu_compra   TEXT NOT NULL,
    pu_base     TEXT NOT NULL,
    PRIMARY KEY (tipo, vencimento, data)
);

CREATE TABLE cotacao (
    ticker TEXT NOT NULL,
    data   TEXT NOT NULL,
    fechamento_centavos INTEGER NOT NULL CHECK (fechamento_centavos > 0),
    PRIMARY KEY (ticker, data)
);

CREATE TABLE fonte_status (
    fonte         TEXT PRIMARY KEY,
    ultima_data   TEXT,
    atualizado_em TEXT,
    erro          TEXT
);
