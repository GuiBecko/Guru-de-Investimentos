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

-- Desde quando a série diária/mensal já foi pedida (para não repetir pedidos sem dados).
CREATE TABLE cobertura_serie (
    serie TEXT PRIMARY KEY,
    desde TEXT NOT NULL
);

-- Cobertura da B3 por data-alvo: pregão achado (NULL = esgotou as 7 tentativas) e
-- tickers pedidos naquela vez (ordenados, separados por vírgula).
CREATE TABLE b3_alvo (
    alvo    TEXT PRIMARY KEY,
    pregao  TEXT,
    tickers TEXT NOT NULL
);
