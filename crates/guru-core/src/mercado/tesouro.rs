use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{ErroFonte, data_br};

const CABECALHO: &str = "Tipo Titulo;Data Vencimento;Data Base;Taxa Compra Manha;Taxa Venda Manha;PU Compra Manha;PU Venda Manha;PU Base Manha";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecoTesouro {
    pub tipo: String,
    pub vencimento: NaiveDate,
    pub data: NaiveDate,
    pub taxa_compra: Decimal,
    pub pu_compra: Decimal,
    pub pu_base: Decimal,
}

fn decimal_br(s: &str) -> Result<Decimal, ErroFonte> {
    s.trim()
        .replace(',', ".")
        .parse()
        .map_err(|_| ErroFonte::Formato(format!("número inválido: {s}")))
}

/// CSV do Tesouro Transparente (`;`, vírgula decimal, sem aspas).
pub fn ler_tesouro(csv: &[u8]) -> Result<Vec<PrecoTesouro>, ErroFonte> {
    let texto = std::str::from_utf8(csv)
        .map_err(|_| ErroFonte::Formato("CSV do Tesouro não é UTF-8".into()))?;
    let texto = texto.strip_prefix('\u{feff}').unwrap_or(texto);
    let mut linhas = texto.lines();
    if linhas.next().map(str::trim) != Some(CABECALHO) {
        return Err(ErroFonte::Formato(
            "cabeçalho do Tesouro diferente do esperado".into(),
        ));
    }
    let mut saida = Vec::new();
    for linha in linhas {
        if linha.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = linha.split(';').map(str::trim).collect();
        if c.len() != 8 {
            return Err(ErroFonte::Formato(format!(
                "linha do Tesouro malformada: {linha}"
            )));
        }
        // taxa compra, PU compra e PU base: se algum faltar, o dia não serve.
        if [c[0], c[1], c[2], c[3], c[5], c[7]]
            .iter()
            .any(|x| x.is_empty())
        {
            continue;
        }
        saida.push(PrecoTesouro {
            tipo: c[0].to_string(),
            vencimento: data_br(c[1])?,
            data: data_br(c[2])?,
            taxa_compra: decimal_br(c[3])?,
            pu_compra: decimal_br(c[5])?,
            pu_base: decimal_br(c[7])?,
        });
    }
    Ok(saida)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_selic_2029() {
        let v = ler_tesouro(include_bytes!("../../tests/fixtures/tesouro.csv")).unwrap();
        let p = v
            .iter()
            .find(|p| {
                p.tipo == "Tesouro Selic"
                    && p.vencimento == data_br("01/03/2029").unwrap()
                    && p.data == data_br("28/10/2026").unwrap()
            })
            .unwrap();
        assert_eq!(p.pu_base, "20151.28".parse::<Decimal>().unwrap());
        assert_eq!(p.pu_compra, "20166.29".parse::<Decimal>().unwrap());
        assert_eq!(p.taxa_compra, "0.05".parse::<Decimal>().unwrap());
    }

    #[test]
    fn cabecalho_trocado_e_formato() {
        let e = ler_tesouro(b"Titulo;Venc\n").unwrap_err();
        assert!(matches!(e, ErroFonte::Formato(_)));
    }

    #[test]
    fn linha_com_campo_vazio_e_pulada() {
        let csv = format!(
            "{CABECALHO}\nTesouro Selic;01/03/2029;28/10/2026;;;;;\nTesouro Selic;01/03/2029;27/10/2026;0,05;0,06;20157,79;20142,78;20142,78\n"
        );
        let v = ler_tesouro(csv.as_bytes()).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].data, data_br("27/10/2026").unwrap());
    }
}
