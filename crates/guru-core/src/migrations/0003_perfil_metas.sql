CREATE TABLE perfis (
    id INTEGER PRIMARY KEY,
    perfil TEXT NOT NULL,
    pontos INTEGER NOT NULL,
    respostas TEXT NOT NULL,
    avisos TEXT NOT NULL,
    feito_em TEXT NOT NULL
);

CREATE TABLE alocacao_alvo (
    classe TEXT PRIMARY KEY,
    percentual INTEGER NOT NULL CHECK (percentual BETWEEN 0 AND 100)
);

CREATE TABLE objetivo (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    texto TEXT NOT NULL,
    atualizado_em TEXT NOT NULL
);

CREATE TABLE metas (
    id INTEGER PRIMARY KEY,
    nome TEXT NOT NULL,
    valor_alvo_centavos INTEGER NOT NULL CHECK (valor_alvo_centavos > 0),
    prazo TEXT NOT NULL,
    criado_em TEXT NOT NULL,
    atualizado_em TEXT NOT NULL
);

ALTER TABLE investimentos ADD COLUMN meta_id INTEGER REFERENCES metas (id) ON DELETE SET NULL;
