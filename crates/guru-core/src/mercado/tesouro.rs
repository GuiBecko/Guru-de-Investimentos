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

fn linha_valida(c: &[&str]) -> Result<PrecoTesouro, ErroFonte> {
    Ok(PrecoTesouro {
        tipo: c[0].to_string(),
        vencimento: data_br(c[1])?,
        data: data_br(c[2])?,
        taxa_compra: decimal_br(c[3])?,
        pu_compra: decimal_br(c[5])?,
        pu_base: decimal_br(c[7])?,
    })
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
    let mut vazias = 0;
    let mut invalidas = 0;
    for linha in linhas {
        if linha.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = linha.split(';').map(str::trim).collect();
        if c.len() != 8 {
            invalidas += 1;
            continue;
        }
        // taxa compra, PU compra e PU base: se algum faltar, o dia não serve.
        if [c[0], c[1], c[2], c[3], c[5], c[7]]
            .iter()
            .any(|x| x.is_empty())
        {
            vazias += 1;
            continue;
        }
        match linha_valida(&c) {
            Ok(p) => saida.push(p),
            Err(_) => invalidas += 1,
        }
    }
    // Linhas ruins são puladas; só falha se o arquivo tinha dados e nenhum serviu.
    if saida.is_empty() && invalidas > 0 && vazias == 0 {
        return Err(ErroFonte::Formato(
            "nenhuma linha válida no CSV do Tesouro".into(),
        ));
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

    #[test]
    fn linha_quebrada_no_meio_e_pulada_e_sem_nenhuma_valida_e_formato() {
        let csv = format!(
            "{CABECALHO}\nTesouro Selic;01/03/2029;28/10/2026;0,05;0,06;20166,29;20151,28;20151,28\nlinha;quebrada\nTesouro Selic;01/03/2029;32/13/2026;0,05;0,06;20166,29;20151,28;20151,28\nTesouro Selic;01/03/2029;27/10/2026;abc;0,06;20157,79;20142,78;20142,78\nTesouro Selic;01/03/2029;26/10/2026;0,05;0,06;20149,00;20134,00;20134,00\n"
        );
        let v = ler_tesouro(csv.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].data, data_br("26/10/2026").unwrap());

        let so_ruins = format!("{CABECALHO}\nlinha;quebrada\n");
        assert!(matches!(
            ler_tesouro(so_ruins.as_bytes()),
            Err(ErroFonte::Formato(_))
        ));
    }

    /// Roda sobre uma cópia real do CSV (caminho em `GURU_TESOURO_REAL`); não faz rede.
    #[test]
    #[ignore = "precisa de GURU_TESOURO_REAL apontando para uma cópia do CSV"]
    fn csv_real_do_tesouro() {
        let caminho = std::env::var("GURU_TESOURO_REAL").expect("defina GURU_TESOURO_REAL");
        let bytes = std::fs::read(caminho).unwrap();
        let texto = std::str::from_utf8(&bytes).unwrap();
        let linhas = texto
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count();
        let lidas = ler_tesouro(&bytes).unwrap().len();
        println!(
            "linhas de dados: {linhas}, lidas: {lidas}, puladas: {}",
            linhas - lidas
        );
        assert!(lidas > 0);
    }
}
