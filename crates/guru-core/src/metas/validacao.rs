use chrono::{Datelike, NaiveDate};

use super::modelo::NovaMeta;
use crate::investimento::{ErroCampo, MAX_CENTAVOS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaValida {
    pub nome: String,
    pub valor_alvo_centavos: i64,
    /// `YYYY-MM`.
    pub prazo: String,
    pub ano: i32,
    pub mes: u32,
}

/// `YYYY-MM` estrito.
fn mes(valor: &str) -> Option<(i32, u32)> {
    if valor.len() != 7 {
        return None;
    }
    let d = NaiveDate::parse_from_str(&format!("{valor}-01"), "%Y-%m-%d").ok()?;
    Some((d.year(), d.month()))
}

/// `editando`: um prazo já vencido é aceito (a meta pode continuar existindo depois do prazo).
pub fn validar_meta(
    nova: &NovaMeta,
    hoje: NaiveDate,
    editando: bool,
) -> Result<MetaValida, Vec<ErroCampo>> {
    let mut erros = Vec::new();
    let mut erro = |campo: &str, mensagem: &str| {
        erros.push(ErroCampo {
            campo: campo.to_string(),
            mensagem: mensagem.to_string(),
        });
    };

    let nome = nova.nome.trim().to_string();
    if nome.is_empty() || nome.chars().count() > 60 {
        erro("nome", "Informe o nome da meta.");
    }
    let valor = nova
        .valor_alvo_centavos
        .filter(|v| *v > 0 && *v <= MAX_CENTAVOS);
    if valor.is_none() {
        erro("valorAlvoCentavos", "Informe o valor da meta.");
    }
    let prazo_texto = nova.prazo.as_deref().map(str::trim).unwrap_or_default();
    let prazo = mes(prazo_texto);
    match prazo {
        None => erro("prazo", "Informe o prazo."),
        Some(p) if !editando && p < (hoje.year(), hoje.month()) => {
            erro("prazo", "O prazo precisa ser este mês ou depois.");
        }
        Some(_) => {}
    }

    match (valor, prazo) {
        (Some(valor_alvo_centavos), Some((ano, mes))) if erros.is_empty() => Ok(MetaValida {
            nome,
            valor_alvo_centavos,
            prazo: prazo_texto.to_string(),
            ano,
            mes,
        }),
        _ => Err(erros),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn hoje() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
    }

    fn boa() -> NovaMeta {
        NovaMeta {
            nome: "Reserva".into(),
            valor_alvo_centavos: Some(1_000_000),
            prazo: Some("2027-06".into()),
        }
    }

    fn msgs(r: Result<MetaValida, Vec<ErroCampo>>) -> Vec<(String, String)> {
        r.unwrap_err()
            .into_iter()
            .map(|e| (e.campo, e.mensagem))
            .collect()
    }

    fn par(c: &str, m: &str) -> (String, String) {
        (c.to_string(), m.to_string())
    }

    #[test]
    fn meta_boa_passa_e_corta_espacos() {
        let mut n = boa();
        n.nome = "  Reserva  ".into();
        let v = validar_meta(&n, hoje(), false).unwrap();
        assert_eq!(v.nome, "Reserva");
        assert_eq!((v.ano, v.mes), (2027, 6));
        assert_eq!(v.prazo, "2027-06");
    }

    #[test]
    fn nome_vazio_ou_longo_demais() {
        let mut n = boa();
        n.nome = "   ".into();
        assert_eq!(
            msgs(validar_meta(&n, hoje(), false)),
            vec![par("nome", "Informe o nome da meta.")]
        );
        n.nome = "a".repeat(61);
        assert_eq!(
            msgs(validar_meta(&n, hoje(), false)),
            vec![par("nome", "Informe o nome da meta.")]
        );
        n.nome = "a".repeat(60);
        assert!(validar_meta(&n, hoje(), false).is_ok());
    }

    #[test]
    fn valor_zero_ausente_negativo_ou_acima_do_teto() {
        for v in [Some(0), None, Some(-5), Some(MAX_CENTAVOS + 1)] {
            let mut n = boa();
            n.valor_alvo_centavos = v;
            assert_eq!(
                msgs(validar_meta(&n, hoje(), false)),
                vec![par("valorAlvoCentavos", "Informe o valor da meta.")]
            );
        }
        let mut n = boa();
        n.valor_alvo_centavos = Some(MAX_CENTAVOS);
        assert!(validar_meta(&n, hoje(), false).is_ok());
    }

    #[test]
    fn prazo_vazio_ou_malformado() {
        for p in [None, Some("".to_string()), Some("  ".to_string())] {
            let mut n = boa();
            n.prazo = p;
            assert_eq!(
                msgs(validar_meta(&n, hoje(), false)),
                vec![par("prazo", "Informe o prazo.")]
            );
        }
        for p in ["2027-13", "2027-6", "27-06", "2027-06-01", "abc"] {
            let mut n = boa();
            n.prazo = Some(p.into());
            assert_eq!(
                msgs(validar_meta(&n, hoje(), false)),
                vec![par("prazo", "Informe o prazo.")],
                "{p}"
            );
        }
    }

    #[test]
    fn prazo_passado_so_ao_editar() {
        let mut n = boa();
        n.prazo = Some("2026-09".into());
        assert_eq!(
            msgs(validar_meta(&n, hoje(), false)),
            vec![par("prazo", "O prazo precisa ser este mês ou depois.")]
        );
        assert!(validar_meta(&n, hoje(), true).is_ok());
        n.prazo = Some("2026-10".into());
        assert!(validar_meta(&n, hoje(), false).is_ok());
    }

    #[test]
    fn acumula_erros_de_varios_campos() {
        let n = NovaMeta::default();
        let campos: Vec<String> = msgs(validar_meta(&n, hoje(), false))
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert_eq!(campos, ["nome", "valorAlvoCentavos", "prazo"]);
    }
}
