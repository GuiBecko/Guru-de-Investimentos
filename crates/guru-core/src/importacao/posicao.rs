//! Leitor da posição da Área do Investidor B3: transforma as abas genéricas do XLSX em linhas tipadas.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::xlsx::{Aba, Celula, ler_xlsx};
use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbaRv {
    Acao,
    Bdr,
    Etf,
    Fii,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinhaB3 {
    RendaVariavel {
        aba: AbaRv,
        ticker: String,
        produto: String,
        instituicao: String,
        quantidade: i64,
        preco_fechamento: Option<Decimal>,
        valor_atualizado: Option<Decimal>,
    },
    RendaFixa {
        produto: String,
        tipo: Option<String>,
        instituicao: String,
        emissor: String,
        codigo: String,
        indexador: Option<String>,
        benchmark: Option<String>,
        emissao: Option<NaiveDate>,
        vencimento: NaiveDate,
        quantidade: Decimal,
        valor_curva: Option<Decimal>,
    },
    Tesouro {
        produto: String,
        tipo: String,
        isin: Option<String>,
        instituicao: String,
        vencimento: NaiveDate,
        valor_aplicado: Option<Decimal>,
        valor_atualizado: Option<Decimal>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Posicao {
    pub linhas: Vec<LinhaB3>,
    pub avisos: Vec<String>,
    pub linhas_lidas: usize,
}

pub fn ler_posicao(bytes: &[u8]) -> Result<Posicao, CoreError> {
    let abas = ler_xlsx(bytes)?;
    ler_posicao_abas(&abas)
}

const MSG_SEM_ABA: &str = "Não encontrei nenhuma aba da posição da B3 neste arquivo.";

/// Minúsculas e sem acentos, só o que aparece em nomes de aba e de coluna em português.
fn dobrar(texto: &str) -> String {
    texto
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            outro => outro,
        })
        .collect()
}

enum TipoAba {
    Rv(AbaRv),
    RendaFixa,
    Tesouro,
}

fn tipo_da_aba(nome: &str) -> Option<TipoAba> {
    match dobrar(nome).as_str() {
        "acoes" => Some(TipoAba::Rv(AbaRv::Acao)),
        "bdr" => Some(TipoAba::Rv(AbaRv::Bdr)),
        "etf" => Some(TipoAba::Rv(AbaRv::Etf)),
        "fundo de investimento" => Some(TipoAba::Rv(AbaRv::Fii)),
        "renda fixa" => Some(TipoAba::RendaFixa),
        "tesouro direto" => Some(TipoAba::Tesouro),
        _ => None,
    }
}

/// Colunas da aba, localizadas pelo cabeçalho (primeira linha).
struct Colunas {
    nomes: Vec<String>,
}

impl Colunas {
    fn da_aba(aba: &Aba) -> Self {
        let nomes = match aba.linhas.first() {
            Some(cab) => cab
                .iter()
                .map(|c| match c {
                    Celula::Texto(t) | Celula::Numero(t) => dobrar(t),
                    Celula::Vazia => String::new(),
                })
                .collect(),
            None => Vec::new(),
        };
        Colunas { nomes }
    }

    fn indice(&self, nome: &str) -> Option<usize> {
        let alvo = dobrar(nome);
        self.nomes.iter().position(|n| *n == alvo)
    }

    /// Índices das colunas obrigatórias; as que faltam viram avisos.
    fn exigir(&self, aba: &str, nomes: &[&str], avisos: &mut Vec<String>) -> Option<Vec<usize>> {
        let mut indices = Vec::new();
        let mut falta = false;
        for nome in nomes {
            match self.indice(nome) {
                Some(i) => indices.push(i),
                None => {
                    avisos.push(format!("Aba {aba}: falta a coluna {nome}."));
                    falta = true;
                }
            }
        }
        (!falta).then_some(indices)
    }
}

fn texto_bruto(linha: &[Celula], i: usize) -> Option<&str> {
    match linha.get(i) {
        Some(Celula::Texto(t) | Celula::Numero(t)) => {
            let t = t.trim();
            (!t.is_empty() && t != "-").then_some(t)
        }
        _ => None,
    }
}

fn texto(linha: &[Celula], i: usize) -> String {
    texto_bruto(linha, i).unwrap_or_default().to_string()
}

fn texto_opc(linha: &[Celula], i: Option<usize>) -> Option<String> {
    i.and_then(|i| texto_bruto(linha, i)).map(str::to_string)
}

/// Número de uma célula: `Numero` vem como texto bruto; `Texto` aceita o formato pt-BR.
fn decimal(linha: &[Celula], i: usize) -> Result<Option<Decimal>, ()> {
    let Some(bruto) = texto_bruto(linha, i) else {
        return Ok(None);
    };
    let limpo = match linha.get(i) {
        Some(Celula::Texto(_)) if bruto.contains(',') => bruto.replace('.', "").replace(',', "."),
        _ => bruto.to_string(),
    };
    limpo
        .parse::<Decimal>()
        .or_else(|_| Decimal::from_scientific(&limpo))
        .map(Some)
        .map_err(|_| ())
}

fn decimal_opc(linha: &[Celula], i: Option<usize>) -> Option<Decimal> {
    i.and_then(|i| decimal(linha, i).ok().flatten())
}

fn data(linha: &[Celula], i: usize) -> Result<Option<NaiveDate>, ()> {
    match texto_bruto(linha, i) {
        None => Ok(None),
        Some(t) => NaiveDate::parse_from_str(t, "%d/%m/%Y")
            .map(Some)
            .map_err(|_| ()),
    }
}

/// Linhas de dados: terminam no primeiro `Produto` vazio, o que deixa de fora as linhas de total.
fn linhas_de_dados(aba: &Aba, produto: usize) -> impl Iterator<Item = (usize, &Vec<Celula>)> {
    aba.linhas
        .iter()
        .enumerate()
        .skip(1)
        .take_while(move |(_, l)| texto_bruto(l, produto).is_some())
        .map(|(n, l)| (n + 1, l))
}

fn tipo_renda_fixa(produto: &str) -> Option<String> {
    let prefixo = produto.split(" - ").next()?;
    let codigo = match dobrar(prefixo).as_str() {
        "cdb" => "rf-cdb",
        "lci" => "rf-lci",
        "lca" => "rf-lca",
        "lc" => "rf-lc",
        "deb" | "debenture" => "rf-deb",
        "cri" => "rf-cri",
        "cra" => "rf-cra",
        _ => return None,
    };
    Some(codigo.to_string())
}

/// Indexador e benchmark do catálogo a partir do texto da B3.
fn indexador(texto: Option<&str>) -> (Option<String>, Option<String>) {
    let par = |i: &str, b: Option<&str>| (Some(i.to_string()), b.map(str::to_string));
    match texto.map(dobrar).as_deref() {
        Some("di" | "cdi") => par("pos", Some("cdi")),
        Some("selic") => par("pos", Some("selic")),
        Some("prefixado" | "pre") => par("pre", None),
        Some("ipca") => par("ipca", None),
        _ => (None, None),
    }
}

/// Título do catálogo a partir do `Produto`, sem o ano final.
fn tipo_tesouro(produto: &str) -> Option<&'static str> {
    let dobrado = dobrar(produto);
    let sem_ano = match dobrado.rsplit_once(' ') {
        Some((resto, ano)) if ano.len() == 4 && ano.chars().all(|c| c.is_ascii_digit()) => resto,
        _ => dobrado.as_str(),
    };
    match sem_ano {
        "tesouro selic" => Some("td-selic"),
        "tesouro prefixado" => Some("td-pre"),
        "tesouro ipca+" => Some("td-ipca"),
        _ => None,
    }
}

fn ler_renda_variavel(aba: &Aba, tipo: AbaRv, out: &mut Posicao) {
    let cols = Colunas::da_aba(aba);
    let Some(ix) = cols.exigir(
        &aba.nome,
        &["Produto", "Código de Negociação", "Quantidade"],
        &mut out.avisos,
    ) else {
        return;
    };
    let (c_prod, c_ticker, c_qtd) = (ix[0], ix[1], ix[2]);
    let c_inst = cols.indice("Instituição");
    let c_preco = cols.indice("Preço de Fechamento");
    let c_valor = cols.indice("Valor Atualizado");
    for (n, l) in linhas_de_dados(aba, c_prod) {
        out.linhas_lidas += 1;
        let ticker = texto(l, c_ticker);
        if ticker.is_empty() {
            out.avisos.push(format!(
                "Aba {}, linha {n}: sem código de negociação.",
                aba.nome
            ));
            continue;
        }
        let quantidade = match decimal(l, c_qtd) {
            Ok(Some(q)) if q.fract().is_zero() && q >= Decimal::ZERO => i64::try_from(q).ok(),
            Ok(Some(_)) => {
                out.avisos
                    .push(format!("Quantidade fracionada ou negativa em {ticker}."));
                continue;
            }
            _ => None,
        };
        let Some(quantidade) = quantidade else {
            out.avisos
                .push(format!("Aba {}, linha {n}: quantidade ilegível.", aba.nome));
            continue;
        };
        out.linhas.push(LinhaB3::RendaVariavel {
            aba: tipo,
            ticker,
            produto: texto(l, c_prod),
            instituicao: c_inst.map(|i| texto(l, i)).unwrap_or_default(),
            quantidade,
            preco_fechamento: decimal_opc(l, c_preco),
            valor_atualizado: decimal_opc(l, c_valor),
        });
    }
}

fn ler_renda_fixa(aba: &Aba, out: &mut Posicao) {
    let cols = Colunas::da_aba(aba);
    let Some(ix) = cols.exigir(
        &aba.nome,
        &["Produto", "Código", "Vencimento", "Quantidade"],
        &mut out.avisos,
    ) else {
        return;
    };
    let (c_prod, c_codigo, c_venc, c_qtd) = (ix[0], ix[1], ix[2], ix[3]);
    let c_inst = cols.indice("Instituição");
    let c_emissor = cols.indice("Emissor");
    let c_indexador = cols.indice("Indexador");
    let c_emissao = cols.indice("Data de Emissão");
    let c_curva = cols.indice("Valor Atualizado CURVA");
    for (n, l) in linhas_de_dados(aba, c_prod) {
        out.linhas_lidas += 1;
        let produto = texto(l, c_prod);
        let (Ok(Some(vencimento)), Ok(emissao), Ok(Some(quantidade))) = (
            data(l, c_venc),
            c_emissao.map_or(Ok(None), |i| data(l, i)),
            decimal(l, c_qtd),
        ) else {
            out.avisos.push(format!(
                "Aba {}, linha {n}: vencimento, emissão ou quantidade ilegível.",
                aba.nome
            ));
            continue;
        };
        let (indexador, benchmark) = indexador(texto_opc(l, c_indexador).as_deref());
        let emissor = texto_opc(l, c_emissor)
            .or_else(|| produto.split_once(" - ").map(|(_, e)| e.to_string()))
            .unwrap_or_default();
        out.linhas.push(LinhaB3::RendaFixa {
            tipo: tipo_renda_fixa(&produto),
            produto,
            instituicao: c_inst.map(|i| texto(l, i)).unwrap_or_default(),
            emissor,
            codigo: texto(l, c_codigo),
            indexador,
            benchmark,
            emissao,
            vencimento,
            quantidade,
            valor_curva: decimal_opc(l, c_curva),
        });
    }
}

fn ler_tesouro(aba: &Aba, out: &mut Posicao) {
    let cols = Colunas::da_aba(aba);
    let Some(ix) = cols.exigir(
        &aba.nome,
        &["Produto", "Vencimento", "Valor Aplicado"],
        &mut out.avisos,
    ) else {
        return;
    };
    let (c_prod, c_venc, c_aplicado) = (ix[0], ix[1], ix[2]);
    let c_inst = cols.indice("Instituição");
    let c_isin = cols.indice("Código ISIN");
    let c_valor = cols.indice("Valor Atualizado");
    for (n, l) in linhas_de_dados(aba, c_prod) {
        out.linhas_lidas += 1;
        let produto = texto(l, c_prod);
        let Some(tipo) = tipo_tesouro(&produto) else {
            out.avisos
                .push(format!("Título {produto} ainda não é suportado."));
            continue;
        };
        let (Ok(Some(vencimento)), Ok(valor_aplicado)) = (data(l, c_venc), decimal(l, c_aplicado))
        else {
            out.avisos.push(format!(
                "Aba {}, linha {n}: vencimento ou valor aplicado ilegível.",
                aba.nome
            ));
            continue;
        };
        out.linhas.push(LinhaB3::Tesouro {
            tipo: tipo.to_string(),
            produto,
            isin: texto_opc(l, c_isin),
            instituicao: c_inst.map(|i| texto(l, i)).unwrap_or_default(),
            vencimento,
            valor_aplicado,
            valor_atualizado: decimal_opc(l, c_valor),
        });
    }
}

/// Lê as abas já extraídas. Sem nenhuma linha lida, é `ARQUIVO_INVALIDO` com os avisos na mensagem.
pub fn ler_posicao_abas(abas: &[Aba]) -> Result<Posicao, CoreError> {
    let mut out = Posicao {
        linhas: Vec::new(),
        avisos: Vec::new(),
        linhas_lidas: 0,
    };
    for aba in abas {
        match tipo_da_aba(&aba.nome) {
            Some(TipoAba::Rv(tipo)) => ler_renda_variavel(aba, tipo, &mut out),
            Some(TipoAba::RendaFixa) => ler_renda_fixa(aba, &mut out),
            Some(TipoAba::Tesouro) => ler_tesouro(aba, &mut out),
            None => out
                .avisos
                .push(format!("Aba {} não reconhecida.", aba.nome)),
        }
    }
    if out.linhas.is_empty() {
        let mut msg = MSG_SEM_ABA.to_string();
        for aviso in &out.avisos {
            msg.push(' ');
            msg.push_str(aviso);
        }
        return Err(CoreError::ArquivoInvalido(msg));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BDR_RF: &[u8] = include_bytes!("../../tests/fixtures/b3/posicao-bdr-rf.xlsx");
    const COMPLETA: &[u8] = include_bytes!("../../tests/fixtures/b3/posicao-completa.xlsx");
    const DESCONHECIDA: &[u8] =
        include_bytes!("../../tests/fixtures/b3/posicao-aba-desconhecida.xlsx");

    fn data(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn t(s: &str) -> Celula {
        Celula::Texto(s.to_string())
    }

    fn n(s: &str) -> Celula {
        Celula::Numero(s.to_string())
    }

    fn aba(nome: &str, linhas: Vec<Vec<Celula>>) -> Aba {
        Aba {
            nome: nome.to_string(),
            linhas,
        }
    }

    fn cab_acoes() -> Vec<Celula> {
        [
            "Produto",
            "Instituição",
            "Código de Negociação",
            "Quantidade",
        ]
        .iter()
        .map(|c| t(c))
        .collect()
    }

    fn cab_tesouro() -> Vec<Celula> {
        ["Produto", "Instituição", "Vencimento", "Valor Aplicado"]
            .iter()
            .map(|c| t(c))
            .collect()
    }

    #[test]
    fn le_bdr_e_renda_fixa() {
        let p = ler_posicao(BDR_RF).unwrap();
        assert_eq!(p.avisos, Vec::<String>::new());
        assert_eq!(p.linhas.len(), 3);
        match &p.linhas[0] {
            LinhaB3::RendaVariavel {
                aba,
                ticker,
                quantidade,
                ..
            } => {
                assert_eq!(*aba, AbaRv::Bdr);
                assert_eq!(ticker, "AAPL34");
                assert_eq!(*quantidade, 10);
            }
            outra => panic!("esperava BDR, veio {outra:?}"),
        }
        match &p.linhas[1] {
            LinhaB3::RendaFixa {
                tipo,
                indexador,
                benchmark,
                vencimento,
                codigo,
                ..
            } => {
                assert_eq!(tipo.as_deref(), Some("rf-cdb"));
                assert_eq!(indexador.as_deref(), Some("pos"));
                assert_eq!(benchmark.as_deref(), Some("cdi"));
                assert_eq!(*vencimento, data(2029, 3, 15));
                assert_eq!(codigo, "CDB224ABCDE");
            }
            outra => panic!("esperava CDB, veio {outra:?}"),
        }
        match &p.linhas[2] {
            LinhaB3::RendaFixa {
                tipo,
                indexador,
                benchmark,
                ..
            } => {
                assert_eq!(tipo.as_deref(), Some("rf-cdb"));
                assert_eq!(indexador.as_deref(), Some("pre"));
                assert_eq!(*benchmark, None);
            }
            outra => panic!("esperava CDB, veio {outra:?}"),
        }
    }

    #[test]
    fn le_as_seis_abas_da_amostra_completa() {
        let p = ler_posicao(COMPLETA).unwrap();
        assert_eq!(p.avisos, Vec::<String>::new());
        let rv: Vec<(AbaRv, &str, i64)> = p
            .linhas
            .iter()
            .filter_map(|l| match l {
                LinhaB3::RendaVariavel {
                    aba,
                    ticker,
                    quantidade,
                    ..
                } => Some((*aba, ticker.as_str(), *quantidade)),
                _ => None,
            })
            .collect();
        assert_eq!(
            rv,
            vec![
                (AbaRv::Acao, "ITUB4", 150),
                (AbaRv::Acao, "B3SA3", 200),
                (AbaRv::Bdr, "AAPL34", 10),
                (AbaRv::Etf, "BOVA11", 30),
                (AbaRv::Fii, "HGLG11", 12),
            ]
        );
        let rf = p
            .linhas
            .iter()
            .filter(|l| matches!(l, LinhaB3::RendaFixa { .. }))
            .count();
        assert_eq!(rf, 2);
        let tesouros: Vec<_> = p
            .linhas
            .iter()
            .filter_map(|l| match l {
                LinhaB3::Tesouro {
                    tipo,
                    valor_aplicado,
                    vencimento,
                    ..
                } => Some((tipo.as_str(), *valor_aplicado, *vencimento)),
                _ => None,
            })
            .collect();
        assert_eq!(tesouros.len(), 2);
        assert_eq!(
            tesouros[0],
            ("td-selic", Some(dec("8000.00")), data(2029, 3, 1))
        );
        assert_eq!(tesouros[1].0, "td-ipca");
        assert_eq!(p.linhas_lidas, p.linhas.len());
    }

    #[test]
    fn aba_desconhecida_e_coluna_faltando_viram_avisos_e_arquivo_invalido() {
        let abas = ler_xlsx(DESCONHECIDA).unwrap();
        let e = ler_posicao_abas(&abas).unwrap_err();
        assert_eq!(e.code(), "ARQUIVO_INVALIDO");
        assert_eq!(
            e.to_string(),
            "Não encontrei nenhuma aba da posição da B3 neste arquivo. \
             Aba Aluguel de Ações não reconhecida. \
             Aba Renda Fixa: falta a coluna Vencimento."
        );
        assert!(ler_posicao(DESCONHECIDA).is_err());
    }

    #[test]
    fn aba_acoes_com_e_sem_acento() {
        for nome in ["Ações", "Acoes", "AÇÕES", "acoes"] {
            let a = aba(
                nome,
                vec![
                    cab_acoes(),
                    vec![t("ITUB4 - ITAU"), t("CORRETORA"), t("ITUB4"), n("5")],
                    vec![Celula::Vazia],
                    vec![Celula::Vazia, t("Total")],
                ],
            );
            let p = ler_posicao_abas(&[a]).unwrap();
            assert_eq!(p.linhas.len(), 1, "aba {nome}");
            assert!(p.avisos.is_empty());
        }
    }

    #[test]
    fn titulo_com_juros_semestrais_vira_aviso() {
        let a = aba(
            "Tesouro Direto",
            vec![
                cab_tesouro(),
                vec![
                    t("Tesouro IPCA+ com Juros Semestrais 2040"),
                    t("CORRETORA"),
                    t("15/08/2040"),
                    n("1000"),
                ],
                vec![
                    t("Tesouro Prefixado 2029"),
                    t("CORRETORA"),
                    t("01/01/2029"),
                    n("500"),
                ],
            ],
        );
        let p = ler_posicao_abas(&[a]).unwrap();
        assert_eq!(p.linhas.len(), 1);
        assert!(matches!(&p.linhas[0], LinhaB3::Tesouro { tipo, .. } if tipo == "td-pre"));
        assert_eq!(
            p.avisos,
            vec!["Título Tesouro IPCA+ com Juros Semestrais 2040 ainda não é suportado."]
        );
    }

    #[test]
    fn quantidade_fracionada_em_acoes_vira_aviso() {
        let a = aba(
            "Acoes",
            vec![
                cab_acoes(),
                vec![t("ITUB4 - ITAU"), t("CORRETORA"), t("ITUB4"), n("10.5")],
                vec![t("B3SA3 - B3"), t("CORRETORA"), t("B3SA3"), n("7")],
            ],
        );
        let p = ler_posicao_abas(&[a]).unwrap();
        assert_eq!(p.linhas.len(), 1);
        assert_eq!(p.avisos.len(), 1);
        assert!(p.avisos[0].contains("fracion"), "{:?}", p.avisos);
    }

    #[test]
    fn traco_vale_nulo_e_total_encerra_a_aba() {
        let a = aba(
            "Tesouro Direto",
            vec![
                vec![
                    t("Produto"),
                    t("Instituição"),
                    t("Código ISIN"),
                    t("Vencimento"),
                    t("Valor Aplicado"),
                    t("Valor Atualizado"),
                ],
                vec![
                    t("Tesouro Selic 2029"),
                    t("CORRETORA"),
                    t("-"),
                    t("01/03/2029"),
                    t("-"),
                    n("123.45"),
                ],
                vec![Celula::Vazia],
                vec![
                    Celula::Vazia,
                    Celula::Vazia,
                    Celula::Vazia,
                    Celula::Vazia,
                    t("Total"),
                ],
                vec![
                    t("Tesouro Selic 2030"),
                    t("X"),
                    t("-"),
                    t("01/03/2030"),
                    n("1"),
                    n("1"),
                ],
            ],
        );
        let p = ler_posicao_abas(&[a]).unwrap();
        assert_eq!(p.linhas.len(), 1);
        match &p.linhas[0] {
            LinhaB3::Tesouro {
                isin,
                valor_aplicado,
                valor_atualizado,
                ..
            } => {
                assert_eq!(*isin, None);
                assert_eq!(*valor_aplicado, None);
                assert_eq!(*valor_atualizado, Some(dec("123.45")));
            }
            outra => panic!("{outra:?}"),
        }
    }

    #[test]
    #[ignore = "usa o arquivo real em GURU_POSICAO_REAL; imprime só contagens"]
    fn posicao_real() {
        let caminho = std::env::var("GURU_POSICAO_REAL").expect("defina GURU_POSICAO_REAL");
        let bytes = std::fs::read(caminho).unwrap();
        let p = ler_posicao(&bytes).unwrap();
        let (mut rv, mut rf, mut td) = (0, 0, 0);
        for l in &p.linhas {
            match l {
                LinhaB3::RendaVariavel { .. } => rv += 1,
                LinhaB3::RendaFixa { .. } => rf += 1,
                LinhaB3::Tesouro { .. } => td += 1,
            }
        }
        println!(
            "renda_variavel={rv} renda_fixa={rf} tesouro={td} avisos={} linhas_lidas={}",
            p.avisos.len(),
            p.linhas_lidas
        );
    }
}
