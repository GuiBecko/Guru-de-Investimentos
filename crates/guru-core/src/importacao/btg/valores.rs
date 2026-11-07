//! Conversão dos textos do extrato em datas, números e taxas.

use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::super::pdf::esqueleto;

/// `dd/mm/aa`, ano 20aa.
pub(super) fn data(texto: &str) -> Option<NaiveDate> {
    let t = texto.trim();
    let p: Vec<&str> = t.split('/').collect();
    if t.len() != 8 || p.len() != 3 || p.iter().any(|x| x.len() != 2) {
        return None;
    }
    let num = |s: &str| s.parse::<u32>().ok();
    let ano = 2000 + num(p[2])? as i32;
    NaiveDate::from_ymd_opt(ano, num(p[1])?, num(p[0])?)
}

/// `1.234,56` → `1234.56`; `-` ou vazio → `Ok(None)`.
pub(super) fn valor(texto: &str) -> Result<Option<Decimal>, ()> {
    let t = texto.trim();
    if t.is_empty() || t == "-" {
        return Ok(None);
    }
    let normal = t.replace('.', "").replace(',', ".");
    Decimal::from_str(&normal).map(Some).map_err(|_| ())
}

pub(super) fn valor_obrigatorio(texto: &str) -> Option<Decimal> {
    valor(texto).ok().flatten()
}

/// Taxa de compra → (indexador, taxa, benchmark).
pub(super) fn taxa(texto: &str) -> (Option<&'static str>, Option<Decimal>, Option<&'static str>) {
    let t = texto
        .replace('\0', "")
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let percentual = |s: &str| {
        let s = s.trim();
        s.strip_suffix('%').and_then(valor_obrigatorio)
    };
    if let Some(r) = t.strip_prefix("ipca") {
        let r = r.trim_start();
        let n = r.strip_prefix('+').and_then(|r| {
            let r = r.trim();
            percentual(r.strip_suffix("a.a.").unwrap_or(r))
        });
        if let Some(n) = n {
            return (Some("ipca"), Some(n), None);
        }
    } else if let Some(n) = t.strip_suffix("do cdi").and_then(percentual) {
        return (Some("pos"), Some(n), Some("cdi"));
    } else if let Some(n) = t.strip_suffix("a.a.").and_then(percentual) {
        return (Some("pre"), Some(n), None);
    }
    (None, None, None)
}

pub(super) fn liquidez(texto: &str) -> Option<&'static str> {
    match esqueleto(texto).as_str() {
        "não" | "nao" => Some("vencimento"),
        "sim" => Some("diaria"),
        _ => None,
    }
}

/// CNPJ (só dígitos) depois do último "CNPJ" do texto.
pub(super) fn cnpj(texto: &str) -> Option<String> {
    let i = texto.rfind("CNPJ")?;
    let d: String = texto[i..].chars().filter(char::is_ascii_digit).collect();
    (d.len() == 14).then_some(d)
}

/// Nome do fundo: os trechos " - " de `pular` até o penúltimo (o último é o
/// "Classe CNPJ: …"), sem `*`.
pub(super) fn nome_fundo(texto: &str, pular: usize) -> String {
    let partes: Vec<&str> = texto.split(" - ").collect();
    let fim = partes.len().saturating_sub(1).max(pular);
    partes[pular.min(partes.len())..fim.min(partes.len())]
        .join(" - ")
        .trim()
        .trim_end_matches('*')
        .trim()
        .replace('\0', "a")
}
