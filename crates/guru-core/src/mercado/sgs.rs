use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;

use super::{ErroFonte, data_br};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PontoDiario {
    pub data: NaiveDate,
    pub valor: Decimal,
}

#[derive(Deserialize)]
struct Linha {
    data: String,
    valor: String,
}

/// JSON do SGS: `[{"data":"dd/MM/aaaa","valor":"0.051660"}]`.
pub fn ler_sgs(corpo: &[u8]) -> Result<Vec<PontoDiario>, ErroFonte> {
    if corpo.trim_ascii_start().first() != Some(&b'[') {
        return Err(ErroFonte::Formato(
            "resposta inesperada do Banco Central".into(),
        ));
    }
    let linhas: Vec<Linha> = serde_json::from_slice(corpo)
        .map_err(|e| ErroFonte::Formato(format!("JSON do Banco Central inválido: {e}")))?;
    linhas
        .into_iter()
        .map(|l| {
            let valor = l
                .valor
                .trim()
                .parse::<Decimal>()
                .map_err(|_| ErroFonte::Formato(format!("valor inválido: {}", l.valor)))?;
            Ok(PontoDiario {
                data: data_br(&l.data)?,
                valor,
            })
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        data_br(s).unwrap()
    }

    #[test]
    fn cdi_primeiro_ponto() {
        let v = ler_sgs(include_bytes!("../../tests/fixtures/sgs-12.json")).unwrap();
        assert_eq!(v[0].data, d("01/09/2026"));
        assert_eq!(v[0].valor, "0.051660".parse::<Decimal>().unwrap());
    }

    #[test]
    fn selic_le() {
        let v = ler_sgs(include_bytes!("../../tests/fixtures/sgs-11.json")).unwrap();
        assert!(!v.is_empty());
    }

    #[test]
    fn ipca_le_mes_e_valor() {
        let v = ler_sgs(include_bytes!("../../tests/fixtures/sgs-433.json")).unwrap();
        let p = v.iter().find(|p| p.data == d("01/09/2026")).unwrap();
        assert_eq!(p.valor, "0.82".parse::<Decimal>().unwrap());
    }

    #[test]
    fn html_de_erro_e_formato() {
        let e = ler_sgs(include_bytes!("../../tests/fixtures/sgs-erro.html")).unwrap_err();
        assert_eq!(
            e,
            ErroFonte::Formato("resposta inesperada do Banco Central".into())
        );
    }

    #[test]
    fn lista_vazia_e_valida() {
        assert_eq!(ler_sgs(b"  []").unwrap(), vec![]);
    }

    #[test]
    fn valor_invalido_e_formato() {
        let e = ler_sgs(br#"[{"data":"01/09/2026","valor":"x"}]"#).unwrap_err();
        assert!(matches!(e, ErroFonte::Formato(_)));
    }
}
