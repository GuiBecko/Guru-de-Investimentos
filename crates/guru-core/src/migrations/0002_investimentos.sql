CREATE TABLE investimentos (
    id INTEGER PRIMARY KEY,
    tipo TEXT NOT NULL,
    instituicao TEXT NOT NULL,
    emissor TEXT,
    ticker TEXT,
    indexador TEXT,
    taxa TEXT,
    benchmark TEXT,
    data_aplicacao TEXT NOT NULL,
    vencimento TEXT,
    valor_aplicado_centavos INTEGER CHECK (valor_aplicado_centavos IS NULL OR valor_aplicado_centavos > 0),
    quantidade INTEGER CHECK (quantidade IS NULL OR quantidade > 0),
    preco_medio_centavos INTEGER CHECK (preco_medio_centavos IS NULL OR preco_medio_centavos > 0),
    liquidez TEXT,
    data_carencia TEXT,
    ir TEXT,
    observacao TEXT,
    criado_em TEXT NOT NULL,
    atualizado_em TEXT NOT NULL
);

CREATE INDEX investimentos_tipo ON investimentos (tipo);
