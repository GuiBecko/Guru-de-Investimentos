//! Separação das linhas em seções pelo título.

use super::super::pdf::{LinhaPdf, esqueleto};

/// Cabeçalho e rodapé repetidos em toda página.
pub(super) fn ruido(l: &LinhaPdf) -> bool {
    let texto = l.texto();
    let e = esqueleto(&texto);
    e.starts_with(&esqueleto("Extrato"))
        || e.starts_with(&esqueleto("Período"))
        || e.starts_with(&esqueleto("Periodo"))
        || e == esqueleto("Investimento")
        || e.starts_with(&esqueleto("Disclaimers"))
        || (!texto.is_empty() && texto.chars().all(|c| c.is_ascii_digit()))
}

/// Esqueleto que preserva `-` e `|`, para que "Fundo de Investimento - X" não
/// case com um índice "- Fundo de Investimento página 2".
pub(super) fn chave(texto: &str) -> String {
    let mut saida = String::new();
    let mut trecho = String::new();
    for c in texto.chars() {
        if c == '-' || c == '|' {
            saida.push_str(&esqueleto(&trecho));
            saida.push(c);
            trecho.clear();
        } else {
            trecho.push(c);
        }
    }
    saida.push_str(&esqueleto(&trecho));
    saida
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Tipo {
    FundoPosicao,
    FundoDetalhe,
    RendaFixa(&'static str),
    RendaVariavel(&'static str),
    Cripto,
    Ignorada,
}

pub(super) struct Secao<'a> {
    pub(super) tipo: Tipo,
    /// Título legível (rótulo canônico + o resto do texto do PDF).
    pub(super) titulo: String,
    /// Texto do título como veio do PDF.
    pub(super) bruto: String,
    pub(super) linhas: Vec<&'a LinhaPdf>,
}

pub(super) const FUNDO_POSICAO: &str = "Fundo de Investimento - Posição - Portfólio de fundos";
pub(super) const FUNDO_DETALHE: &str = "Fundo de Investimento - Detalhamento - ";
pub(super) const RF_DETALHE: &str = "Renda fixa - Detalhamento - ";
pub(super) const RV_POSICAO: &str = "Renda variável - Posição - ";
pub(super) const CRIPTO_POSICAO: &str = "CriptoAtivos - Posição";

/// Seções que não interessam ao leitor (testadas antes das demais).
pub(super) const IGNORADAS: &[&str] = &[
    "Sumário",
    "Distribuição",
    "Fundo de Investimento - Rentabilidade",
    "Fundo de Investimento - Distribuição",
    "Renda fixa - Posição",
    "Conta corrente",
    "Valores em trânsito",
    "CriptoAtivos - Movimentação",
    "Perfil de risco",
    "Fale Conosco",
];

/// Prefixos de seções com ativos: se o leitor não conhece a seção, avisa.
pub(super) const CONHECIDAS: &[&str] = &[
    "Fundo de Investimento - ",
    RF_DETALHE,
    RV_POSICAO,
    CRIPTO_POSICAO,
];

pub(super) const TIPOS_RF: &[(&str, &str)] = &[
    ("CDB", "rf-cdb"),
    ("LCI", "rf-lci"),
    ("LCA", "rf-lca"),
    ("LC", "rf-lc"),
    ("DEBÊNTURE", "rf-deb"),
    ("DEB", "rf-deb"),
    ("CRI", "rf-cri"),
    ("CRA", "rf-cra"),
];

pub(super) const TIPOS_RV: &[(&str, &str)] = &[
    ("Ações", "rv-acao"),
    ("BDR", "rv-bdr"),
    ("FII", "rv-fii"),
    ("Fundos Imobiliários", "rv-fii"),
    ("ETF", "rv-etf"),
];

pub(super) fn comeca_com(texto: &str, prefixo: &str) -> bool {
    chave(texto).starts_with(&chave(prefixo))
}

/// Título legível: o prefixo canônico até o último `-` e o resto do texto do
/// PDF depois do mesmo número de `-`, com o `\0` trocado pelo `a` (o caso comum).
pub(super) fn legivel(bruto: &str, prefixo: &str) -> String {
    let n = prefixo.matches('-').count();
    let cabeca = match prefixo.rfind('-') {
        Some(i) => format!("{} ", &prefixo[..=i]),
        None => String::new(),
    };
    let resto = bruto.splitn(n + 1, '-').nth(n).unwrap_or_default();
    format!("{cabeca}{}", resto.trim().replace('\0', "a"))
}

/// Resto do título depois do `n`-ésimo `-`, sem `\0`.
pub(super) fn depois_do_traco(bruto: &str, n: usize) -> String {
    bruto
        .splitn(n + 1, '-')
        .nth(n)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// `None` quando a linha não é título de seção.
pub(super) fn classificar(texto: &str) -> Option<(Tipo, String)> {
    if let Some(p) = IGNORADAS.iter().find(|p| comeca_com(texto, p)) {
        return Some((Tipo::Ignorada, legivel(texto, p)));
    }
    if comeca_com(texto, FUNDO_POSICAO) {
        return Some((Tipo::FundoPosicao, legivel(texto, FUNDO_POSICAO)));
    }
    if comeca_com(texto, FUNDO_DETALHE) {
        return Some((Tipo::FundoDetalhe, legivel(texto, FUNDO_DETALHE)));
    }
    if comeca_com(texto, RF_DETALHE) {
        let titulo = legivel(texto, RF_DETALHE);
        let resto = depois_do_traco(texto, 2);
        let nome_tipo = resto.split('|').next().unwrap_or_default().trim();
        let nome_tipo = nome_tipo.replace('\0', "").to_uppercase();
        let tipo = TIPOS_RF
            .iter()
            .find(|(n, _)| *n == nome_tipo)
            .map(|(_, t)| Tipo::RendaFixa(t));
        return Some((tipo.unwrap_or(Tipo::Ignorada), titulo));
    }
    if comeca_com(texto, RV_POSICAO) {
        let titulo = legivel(texto, RV_POSICAO);
        let resto = esqueleto(&depois_do_traco(texto, 2));
        let tipo = TIPOS_RV
            .iter()
            .find(|(n, _)| esqueleto(n) == resto)
            .map(|(_, t)| Tipo::RendaVariavel(t));
        return Some((tipo.unwrap_or(Tipo::Ignorada), titulo));
    }
    if comeca_com(texto, CRIPTO_POSICAO) {
        return Some((Tipo::Cripto, legivel(texto, CRIPTO_POSICAO)));
    }
    CONHECIDAS
        .iter()
        .find(|p| comeca_com(texto, p))
        .map(|p| (Tipo::Ignorada, legivel(texto, p)))
}

/// Separa as linhas em seções. Seção conhecida sem leitor vira aviso. O título
/// "Disclaimers" encerra a leitura: as notas que vêm depois repetem títulos.
pub(super) fn secoes<'a>(linhas: &'a [LinhaPdf], avisos: &mut Vec<String>) -> Vec<Secao<'a>> {
    let mut saida: Vec<Secao<'a>> = Vec::new();
    for l in linhas {
        let texto = l.texto();
        let unica = l.celulas.len() == 1;
        if unica && chave(&texto) == esqueleto("Disclaimers") {
            break;
        }
        if ruido(l) {
            continue;
        }
        if unica && let Some((tipo, titulo)) = classificar(&texto) {
            let desconhecida =
                tipo == Tipo::Ignorada && !IGNORADAS.iter().any(|p| comeca_com(&texto, p));
            if desconhecida {
                avisos.push(format!("Seção \"{titulo}\" não reconhecida."));
            }
            saida.push(Secao {
                tipo,
                titulo,
                bruto: texto,
                linhas: Vec::new(),
            });
            continue;
        }
        if let Some(s) = saida.last_mut() {
            s.linhas.push(l);
        }
    }
    saida
}
