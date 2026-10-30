use std::io::Read;

use chrono::NaiveDate;

use super::ErroFonte;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cotacao {
    pub ticker: String,
    pub data: NaiveDate,
    pub fechamento_centavos: i64,
}

fn campo(linha: &[u8], de: usize, ate: usize) -> Result<&str, ErroFonte> {
    std::str::from_utf8(&linha[de - 1..ate])
        .map_err(|_| ErroFonte::Formato("campo não ASCII no COTAHIST".into()))
}

/// Posições 1-based do layout da B3; só registros `01` do mercado à vista (`010`).
pub fn ler_cotahist_txt(txt: &[u8]) -> Result<Vec<Cotacao>, ErroFonte> {
    let mut saida = Vec::new();
    for linha in txt.split(|&b| b == b'\n') {
        let linha = linha.strip_suffix(b"\r").unwrap_or(linha);
        if !linha.starts_with(b"01") {
            continue;
        }
        if linha.len() < 121 {
            return Err(ErroFonte::Formato("registro curto no COTAHIST".into()));
        }
        if campo(linha, 25, 27)? != "010" {
            continue;
        }
        let data = NaiveDate::parse_from_str(campo(linha, 3, 10)?, "%Y%m%d")
            .map_err(|_| ErroFonte::Formato("data inválida no COTAHIST".into()))?;
        let fechamento_centavos = campo(linha, 109, 121)?
            .trim()
            .parse::<i64>()
            .map_err(|_| ErroFonte::Formato("preço inválido no COTAHIST".into()))?;
        // Fechamento zero ou negativo não é cotação: o banco recusaria e derrubaria a rodada.
        if fechamento_centavos <= 0 {
            continue;
        }
        saida.push(Cotacao {
            ticker: campo(linha, 13, 24)?.trim().to_string(),
            data,
            fechamento_centavos,
        });
    }
    Ok(saida)
}

/// Lê o primeiro arquivo `.TXT` do ZIP.
pub fn ler_cotahist_zip(zip: &[u8]) -> Result<Vec<Cotacao>, ErroFonte> {
    let invalido = || ErroFonte::Formato("arquivo da B3 não é um ZIP válido".into());
    let mut arquivo = zip::ZipArchive::new(std::io::Cursor::new(zip)).map_err(|_| invalido())?;
    for i in 0..arquivo.len() {
        let mut entrada = arquivo.by_index(i).map_err(|_| invalido())?;
        if entrada
            .name()
            .map_err(|_| invalido())?
            .to_ascii_lowercase()
            .ends_with(".txt")
        {
            let mut bytes = Vec::new();
            entrada.read_to_end(&mut bytes).map_err(|_| invalido())?;
            return ler_cotahist_txt(&bytes);
        }
    }
    Err(ErroFonte::Formato("ZIP da B3 sem arquivo .TXT".into()))
}

#[cfg(test)]
mod testes {
    use super::*;

    const TXT: &[u8] = include_bytes!("../../tests/fixtures/COTAHIST_D28102026.TXT");
    const ZIP: &[u8] = include_bytes!("../../tests/fixtures/COTAHIST_D28102026.ZIP");

    #[test]
    fn so_mercado_a_vista() {
        let v = ler_cotahist_txt(TXT).unwrap();
        let tickers: Vec<&str> = v.iter().map(|c| c.ticker.as_str()).collect();
        assert_eq!(
            tickers,
            ["ITUB4", "BOVA11", "B3SA3", "HGLG11", "M1TA34", "XFIX11"]
        );
    }

    #[test]
    fn itub4_fechamento() {
        let v = ler_cotahist_txt(TXT).unwrap();
        let c = v.iter().find(|c| c.ticker == "ITUB4").unwrap();
        assert_eq!(c.data, NaiveDate::from_ymd_opt(2026, 10, 28).unwrap());
        assert_eq!(c.fechamento_centavos, 3978);
    }

    #[test]
    fn zip_igual_ao_txt() {
        assert_eq!(
            ler_cotahist_zip(ZIP).unwrap(),
            ler_cotahist_txt(TXT).unwrap()
        );
    }

    #[test]
    fn nao_zip_e_formato() {
        assert!(matches!(
            ler_cotahist_zip(b"<html>"),
            Err(ErroFonte::Formato(_))
        ));
    }

    #[test]
    fn fechamento_zero_ou_negativo_e_pulado() {
        let texto = std::str::from_utf8(TXT).unwrap();
        let itub4 = texto.lines().find(|l| l.contains("ITUB4")).unwrap();
        let (antes, depois) = (&itub4[..108], &itub4[121..]);
        let zerado = format!("{antes}{:013}{depois}", 0);
        let negativo = format!("{antes}{:>13}{depois}", "-100");
        let bom = format!("{antes}{:013}{depois}", 3978);
        let arquivo = format!("{zerado}\n{negativo}\n{bom}\n");
        let v = ler_cotahist_txt(arquivo.as_bytes()).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].fechamento_centavos, 3978);
    }
}
