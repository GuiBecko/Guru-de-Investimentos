use chrono::Datelike;
use rust_decimal::Decimal;

use super::catalogo::{Forma, Indexador};
use super::modelo::InvestimentoValido;

/// Formato pt-BR: vírgula decimal e sem zeros à direita (`102.50` vira `102,5`).
fn formatar(taxa: Decimal) -> String {
    taxa.normalize().to_string().replace('.', ",")
}

pub fn nome(v: &InvestimentoValido) -> String {
    let emissor = v.emissor.as_deref().unwrap_or_default();
    match v.tipo.forma {
        Forma::RendaFixa if v.tipo.e_tesouro() => {
            format!("{} {}", v.tipo.rotulo, v.vencimento.map_or(0, |d| d.year()))
        }
        Forma::RendaFixa => format!("{} {}", v.tipo.rotulo, emissor),
        Forma::RendaVariavel => v.ticker.clone().unwrap_or_default(),
        Forma::Valor => emissor.to_string(),
    }
}

pub fn rendimento(v: &InvestimentoValido) -> Option<String> {
    if v.tipo.forma != Forma::RendaFixa {
        return None;
    }
    let taxa = formatar(v.taxa?);
    Some(match v.indexador? {
        Indexador::Pos => format!("{taxa}% do {}", v.benchmark?.rotulo()),
        Indexador::Pre => format!("{taxa}% ao ano"),
        Indexador::Ipca => format!("IPCA + {taxa}% ao ano"),
        Indexador::Selic => format!("Selic + {taxa}% ao ano"),
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::investimento::{NovoInvestimento, validar};
    use chrono::NaiveDate;

    fn valido(novo: NovoInvestimento) -> crate::investimento::InvestimentoValido {
        let hoje = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        validar(&novo, hoje).unwrap()
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    fn cdb(taxa: &str) -> NovoInvestimento {
        NovoInvestimento {
            tipo: "rf-cdb".into(),
            instituicao: "Banco Inter".into(),
            emissor: s("Banco Inter"),
            indexador: s("pos"),
            taxa: s(taxa),
            benchmark: s("cdi"),
            data_aplicacao: s("2025-03-15"),
            vencimento: s("2028-03-15"),
            valor_aplicado_centavos: Some(2_000_000),
            liquidez: s("vencimento"),
            ir: s("regressiva"),
            ..Default::default()
        }
    }

    fn tesouro(indexador: &str, taxa: &str) -> NovoInvestimento {
        NovoInvestimento {
            tipo: if indexador == "selic" {
                "td-selic"
            } else {
                "td-pre"
            }
            .into(),
            instituicao: "XP Investimentos".into(),
            indexador: s(indexador),
            taxa: s(taxa),
            data_aplicacao: s("2025-03-15"),
            vencimento: s("2029-03-01"),
            valor_aplicado_centavos: Some(100_000),
            liquidez: s("d1"),
            ir: s("regressiva"),
            ..Default::default()
        }
    }

    #[test]
    fn nome_e_rendimento() {
        let v = valido(cdb("102"));
        assert_eq!(nome(&v), "CDB Banco Inter");
        assert_eq!(rendimento(&v).as_deref(), Some("102% do CDI"));
        assert_eq!(
            rendimento(&valido(cdb("102.50"))).as_deref(),
            Some("102,5% do CDI")
        );

        let v = valido(tesouro("selic", "0.0905"));
        assert_eq!(nome(&v), "Tesouro Selic 2029");
        assert_eq!(rendimento(&v).as_deref(), Some("Selic + 0,0905% ao ano"));

        let v = valido(tesouro("pre", "12.5"));
        assert_eq!(rendimento(&v).as_deref(), Some("12,5% ao ano"));

        let mut ipca = cdb("6.2");
        ipca.indexador = s("ipca");
        ipca.benchmark = None;
        assert_eq!(
            rendimento(&valido(ipca)).as_deref(),
            Some("IPCA + 6,2% ao ano")
        );

        let acao = NovoInvestimento {
            tipo: "rv-acao".into(),
            instituicao: "Rico".into(),
            ticker: s("itub4"),
            data_aplicacao: s("2025-03-15"),
            quantidade: Some(10),
            preco_medio_centavos: Some(3_000),
            ..Default::default()
        };
        let v = valido(acao);
        assert_eq!(nome(&v), "ITUB4");
        assert_eq!(rendimento(&v), None);
    }

    #[test]
    fn nome_de_forma_valor_e_o_emissor() {
        let v = valido(NovoInvestimento {
            tipo: "fundo".into(),
            instituicao: "XP Investimentos".into(),
            emissor: s("Fundo Multi"),
            data_aplicacao: s("2025-03-15"),
            valor_aplicado_centavos: Some(5_000),
            liquidez: s("d30"),
            ..Default::default()
        });
        assert_eq!(nome(&v), "Fundo Multi");
        assert_eq!(rendimento(&v), None);
    }
}
