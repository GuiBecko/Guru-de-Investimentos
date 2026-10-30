CREATE TABLE ia_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    modelo TEXT NOT NULL,
    instrucoes TEXT NOT NULL,
    apenas_percentuais INTEGER NOT NULL,
    nomes_instituicoes INTEGER NOT NULL,
    busca_web INTEGER NOT NULL,
    chave_final TEXT
);
INSERT INTO ia_config VALUES (1, 'claude-opus-5-5', '', 1, 0, 1, NULL);

CREATE TABLE ia_contexto_parte (
    parte TEXT PRIMARY KEY,
    ligada INTEGER NOT NULL,
    texto_editado TEXT
);

CREATE TABLE conversa (
    id INTEGER PRIMARY KEY,
    titulo TEXT NOT NULL,
    modelo TEXT NOT NULL,
    contexto TEXT NOT NULL,
    fontes_internas TEXT NOT NULL,
    busca_web INTEGER NOT NULL,
    criada_em TEXT NOT NULL,
    atualizada_em TEXT NOT NULL
);

CREATE TABLE mensagem (
    id INTEGER PRIMARY KEY,
    conversa_id INTEGER NOT NULL REFERENCES conversa (id) ON DELETE CASCADE,
    papel TEXT NOT NULL CHECK (papel IN ('user', 'assistant')),
    conteudo_json TEXT NOT NULL,
    texto TEXT NOT NULL,
    fontes_json TEXT NOT NULL,
    aviso TEXT,
    criada_em TEXT NOT NULL
);

CREATE TABLE ia_uso (
    mes TEXT NOT NULL,
    modelo TEXT NOT NULL,
    entrada INTEGER NOT NULL,
    saida INTEGER NOT NULL,
    cache_leitura INTEGER NOT NULL,
    cache_escrita INTEGER NOT NULL,
    PRIMARY KEY (mes, modelo)
);
