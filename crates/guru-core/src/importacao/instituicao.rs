//! Chave de comparação de instituições: a B3 grava o nome da corretora como no registro
//! (`BTG PACTUAL CTVM S/A`) e o catálogo usa `BTG Pactual`; as duas grafias têm a mesma chave.

use super::comparacao::normalizar_emissor;
use crate::investimento::INSTITUICOES;

/// Marca (palavra inteira ou sequência de palavras do nome normalizado) e nome no catálogo.
const MARCAS: &[(&[&str], &str)] = &[
    (&["BTG"], "BTG Pactual"),
    (&["XP"], "XP Investimentos"),
    (&["INTER"], "Banco Inter"),
    (&["RICO"], "Rico"),
    (&["NU"], "Nubank"),
    (&["NUINVEST"], "Nubank"),
    (&["NUBANK"], "Nubank"),
    (&["BANCO", "DO", "BRASIL"], "Banco do Brasil"),
    (&["BB"], "Banco do Brasil"),
    (&["ITAU"], "Itaú"),
    (&["BRADESCO"], "Bradesco"),
    (&["SANTANDER"], "Santander"),
    (&["CAIXA"], "Caixa Econômica Federal"),
    (&["C6"], "C6 Bank"),
];

/// Nome do catálogo normalizado quando o texto traz a marca de uma instituição conhecida (ignorando
/// o que vem depois de `GRUPO`); senão, o próprio texto normalizado.
pub(crate) fn chave_instituicao(texto: &str) -> String {
    let normal = normalizar_emissor(texto);
    // "RICO INVESTIMENTOS - GRUPO XP": a marca do grupo controlador não é a da corretora.
    let palavras: Vec<&str> = normal
        .split(' ')
        .filter(|p| !p.is_empty())
        .take_while(|p| *p != "GRUPO")
        .collect();
    for (marca, nome) in MARCAS {
        if palavras.windows(marca.len()).any(|w| w == *marca) {
            debug_assert!(INSTITUICOES.contains(nome));
            return normalizar_emissor(nome);
        }
    }
    normal
}

/// A instituição é uma das do catálogo (pela marca), e não uma corretora digitada à mão.
pub(crate) fn e_corretora_conhecida(texto: &str) -> bool {
    let chave = chave_instituicao(texto);
    INSTITUICOES.iter().any(|n| normalizar_emissor(n) == chave)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn corretora_conhecida_e_a_do_catalogo() {
        assert!(e_corretora_conhecida("XP Investimentos"));
        assert!(e_corretora_conhecida("BTG PACTUAL CTVM S/A"));
        assert!(e_corretora_conhecida("Nubank"));
        assert!(!e_corretora_conhecida("Genial"));
        assert!(!e_corretora_conhecida("CLEAR CORRETORA - GRUPO XP"));
        assert!(!e_corretora_conhecida("Outra Corretora"));
    }

    #[test]
    fn chave_instituicao_liga_ao_catalogo() {
        assert_eq!(
            chave_instituicao("BTG PACTUAL CTVM S/A"),
            chave_instituicao("BTG Pactual")
        );
        assert_eq!(
            chave_instituicao("XP INVESTIMENTOS CCTVM S/A"),
            chave_instituicao("XP Investimentos")
        );
        assert_eq!(
            chave_instituicao("NU INVEST CORRETORA DE VALORES S.A."),
            chave_instituicao("Nubank")
        );
        assert_eq!(
            chave_instituicao("BANCO DO BRASIL S/A"),
            chave_instituicao("Banco do Brasil")
        );
        assert_eq!(
            chave_instituicao("ITAU CORRETORA DE VALORES S/A"),
            chave_instituicao("Itaú")
        );
        assert_ne!(
            chave_instituicao("CORRETORA DESCONHECIDA LTDA"),
            chave_instituicao("BTG Pactual")
        );
        assert_eq!(
            chave_instituicao("RICO INVESTIMENTOS - GRUPO XP"),
            chave_instituicao("Rico")
        );
        assert_ne!(
            chave_instituicao("CLEAR CORRETORA - GRUPO XP"),
            chave_instituicao("XP Investimentos")
        );
        assert_ne!(
            chave_instituicao("XP Investimentos"),
            chave_instituicao("BTG Pactual")
        );
    }

    #[test]
    fn todo_nome_do_catalogo_tem_marca() {
        for nome in INSTITUICOES {
            assert!(MARCAS.iter().any(|(_, n)| n == nome), "{nome}");
            assert_eq!(chave_instituicao(nome), normalizar_emissor(nome));
        }
    }
}
