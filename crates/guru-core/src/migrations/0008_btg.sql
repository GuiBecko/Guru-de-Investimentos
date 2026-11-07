ALTER TABLE investimentos ADD COLUMN saldo_centavos INTEGER;
ALTER TABLE investimentos ADD COLUMN saldo_data TEXT;
ALTER TABLE importacao ADD COLUMN fonte TEXT NOT NULL DEFAULT 'b3';
