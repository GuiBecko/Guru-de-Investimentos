-- NULL: conversa do Assistente; código da classe: chat do Próximo aporte.
ALTER TABLE conversa ADD COLUMN foco TEXT;

CREATE TABLE ia_gerado (
    tipo TEXT PRIMARY KEY CHECK (tipo IN ('insights', 'noticias')),
    gerado_em TEXT NOT NULL,
    modelo TEXT NOT NULL,
    itens_json TEXT NOT NULL
);
