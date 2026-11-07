//! Leitor do "Extrato da Conta Investimento" da BTG, a partir das linhas
//! físicas do PDF (ver `pdf.rs`).
//!
//! O extrato é uma sequência de seções (título, cabeçalho de colunas, linhas de
//! dados). Os rótulos vêm com letras perdidas (`\0`), então títulos e cabeçalhos
//! são comparados pelo esqueleto. Uma linha de dado pode estar quebrada em
//! várias linhas físicas: cada uma vai para a âncora (a linha dos números) mais
//! próxima, e cada célula vai para a coluna do cabeçalho à sua esquerda.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::pdf::{LinhaPdf, esqueleto, extrair_linhas};
use crate::error::CoreError;

mod secoes;
mod tabela;
#[cfg(test)]
mod testes;
mod valores;

use secoes::{Secao, Tipo, secoes};
use tabela::{
    Cabecalho, Coluna, Falha, col, data_na_coluna, e_titulo_fundo, ler_tabela, linha_com_valores,
};
use valores::{cnpj, data, liquidez, nome_fundo, taxa, valor, valor_obrigatorio};

const MSG_OUTRO: &str = "Este PDF não é um Extrato da Conta Investimento da BTG.";
const MSG_CRIPTO: &str = "Seção de criptoativos em formato não reconhecido.";

#[derive(Debug, Clone, PartialEq)]
pub struct ExtratoBtg {
    /// Fim do período do extrato.
    pub data: NaiveDate,
    pub ativos: Vec<AtivoBtg>,
    pub avisos: Vec<String>,
    /// `ativos.len()` mais as linhas recusadas com aviso.
    pub linhas_lidas: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AtivoBtg {
    Fundo {
        nome: String,
        cnpj: String,
        aplicado: Decimal,
        primeira_compra: NaiveDate,
        saldo_bruto: Option<Decimal>,
        referencia: Option<NaiveDate>,
    },
    RendaFixa {
        tipo: &'static str,
        emissor: String,
        codigo: String,
        emissao: NaiveDate,
        vencimento: NaiveDate,
        primeira_aquisicao: NaiveDate,
        indexador: Option<&'static str>,
        taxa: Option<Decimal>,
        benchmark: Option<&'static str>,
        liquidez: &'static str,
        aplicado: Decimal,
        saldo_bruto: Decimal,
    },
    RendaVariavel {
        tipo: &'static str,
        ticker: String,
        nome: String,
        quantidade: i64,
        preco_medio: Decimal,
    },
    Cripto {
        nome: String,
        aplicado: Option<Decimal>,
        saldo_bruto: Decimal,
    },
}

/// Lê o PDF do extrato.
pub fn ler_extrato(bytes: &[u8]) -> Result<ExtratoBtg, CoreError> {
    ler_extrato_linhas(&extrair_linhas(bytes)?)
}

/// Lê o extrato a partir das linhas já extraídas. Pura.
pub fn ler_extrato_linhas(linhas: &[LinhaPdf]) -> Result<ExtratoBtg, CoreError> {
    let data = validar(linhas)?;
    let mut leitor = Leitor::default();
    for secao in secoes(linhas, &mut leitor.avisos) {
        leitor.ler(&secao);
    }
    Ok(leitor.concluir(data))
}

// ---------------------------------------------------------------- documento

/// Confere que é o extrato da BTG e devolve o fim do período.
fn validar(linhas: &[LinhaPdf]) -> Result<NaiveDate, CoreError> {
    let outro = || CoreError::ArquivoInvalido(MSG_OUTRO.into());
    let primeira = linhas.first().map(|l| l.pagina).ok_or_else(outro)?;
    let capa = linhas
        .iter()
        .filter(|l| l.pagina == primeira)
        .map(LinhaPdf::texto)
        .collect::<Vec<_>>()
        .join(" ");
    let da_btg = linhas
        .iter()
        .any(|l| l.texto().to_lowercase().contains("btgpactual"));
    if !esqueleto(&capa).contains(&esqueleto("Conta Investimento")) || !da_btg {
        return Err(outro());
    }
    let periodo = esqueleto("Período de");
    linhas
        .iter()
        .map(LinhaPdf::texto)
        .find(|t| esqueleto(t).starts_with(&periodo))
        .and_then(|t| {
            t.split(|c: char| c.is_whitespace() || c == '\0')
                .filter_map(data)
                .nth(1)
        })
        .ok_or_else(outro)
}

// -------------------------------------------------------------------- leitor

struct Lotes {
    nome: String,
    cnpj: String,
    aplicado: Decimal,
    primeira: NaiveDate,
}

struct PosicaoFundo {
    nome: String,
    cnpj: String,
    saldo_bruto: Option<Decimal>,
    referencia: Option<NaiveDate>,
}

#[derive(Default)]
struct Leitor {
    avisos: Vec<String>,
    recusadas: usize,
    lotes: Vec<Lotes>,
    posicoes: Vec<PosicaoFundo>,
    renda_fixa: Vec<AtivoBtg>,
    outros: Vec<AtivoBtg>,
}

impl Leitor {
    fn recusar(&mut self, aviso: String) {
        self.avisos.push(aviso);
        self.recusadas += 1;
    }

    fn linha_invalida(&mut self, secao: &Secao) {
        self.recusar(format!(
            "Seção \"{}\": linha com valores não reconhecidos.",
            secao.titulo
        ));
    }

    fn falha(&mut self, secao: &Secao, falha: Falha) {
        self.avisos.push(match falha {
            Falha::Coluna(coluna) => {
                format!("Seção \"{}\": falta a coluna {coluna}.", secao.titulo)
            }
            Falha::SemLinhas => {
                format!("Seção \"{}\": nenhuma linha reconhecida.", secao.titulo)
            }
        });
    }

    fn ler(&mut self, secao: &Secao) {
        match secao.tipo {
            Tipo::FundoPosicao => self.fundo_posicao(secao),
            Tipo::FundoDetalhe => self.fundo_detalhe(secao),
            Tipo::RendaFixa(tipo) => self.renda_fixa(secao, tipo),
            Tipo::RendaVariavel(tipo) => self.renda_variavel(secao, tipo),
            Tipo::Cripto => self.cripto(secao),
            Tipo::Ignorada => {}
        }
    }

    fn fundo_posicao(&mut self, secao: &Secao) {
        const COLUNAS: &[Coluna] = &[
            col("Data Referência", &["Data Referência"]),
            col("Saldo Bruto R$", &["Saldo Bruto R$"]),
        ];
        let abaixo_do_titulo =
            |linhas: &[&LinhaPdf], i: usize, _: &Cabecalho| i > 0 && e_titulo_fundo(linhas[i - 1]);
        let registros = match ler_tabela(secao, COLUNAS, abaixo_do_titulo) {
            Ok(r) => r,
            Err(f) => return self.falha(secao, f),
        };
        for r in registros {
            let titulo = secao.linhas[r.indice - 1].texto();
            let saldo = valor(&r.valores[1]);
            match (cnpj(&titulo), saldo) {
                (Some(cnpj), Ok(saldo_bruto)) => self.posicoes.push(PosicaoFundo {
                    nome: nome_fundo(&titulo, 0),
                    cnpj,
                    saldo_bruto,
                    referencia: data(&r.valores[0]),
                }),
                _ => self.linha_invalida(secao),
            }
        }
    }

    fn fundo_detalhe(&mut self, secao: &Secao) {
        const COLUNAS: &[Coluna] = &[
            col("Data Compra", &["Data Compra"]),
            col(
                "Valor de Compra Ajustado R$",
                &["Valor de Compra Ajustado R$"],
            ),
        ];
        let registros = match ler_tabela(secao, COLUNAS, data_na_coluna(0)) {
            Ok(r) => r,
            Err(f) => return self.falha(secao, f),
        };
        let Some(cnpj) = cnpj(&secao.bruto) else {
            return self.recusar(format!("Seção \"{}\": CNPJ não reconhecido.", secao.titulo));
        };
        for r in registros {
            let (Some(compra), Some(aplicado)) =
                (data(&r.valores[0]), valor_obrigatorio(&r.valores[1]))
            else {
                self.linha_invalida(secao);
                continue;
            };
            match self.lotes.iter_mut().find(|l| l.cnpj == cnpj) {
                Some(l) => {
                    l.aplicado += aplicado;
                    l.primeira = l.primeira.min(compra);
                }
                None => self.lotes.push(Lotes {
                    nome: nome_fundo(&secao.bruto, 2),
                    cnpj: cnpj.clone(),
                    aplicado,
                    primeira: compra,
                }),
            }
        }
    }

    fn renda_fixa(&mut self, secao: &Secao, tipo: &'static str) {
        const COLUNAS: &[Coluna] = &[
            col("Ativo", &["Ativo"]),
            col("Emissão", &["Emissão"]),
            col("Vencimento", &["Vencimento"]),
            col("Aquisição", &["Aquisição"]),
            col("Liquidez", &["Liquidez"]),
            col("Taxa Compra", &["Taxa Compra"]),
            col("Valor Compra R$", &["Valor Compra R$"]),
            col("Saldo Bruto R$", &["Saldo Bruto R$"]),
        ];
        let registros = match ler_tabela(secao, COLUNAS, data_na_coluna(1)) {
            Ok(r) => r,
            Err(f) => return self.falha(secao, f),
        };
        let emissor = secao
            .bruto
            .split_once('|')
            .map(|(_, e)| e.trim().replace('\0', "a"))
            .unwrap_or_default();
        for r in registros {
            let v = &r.valores;
            let ativo = v[0].trim_end_matches('*');
            let codigo = ativo
                .split_once('-')
                .map_or(ativo, |(_, c)| c)
                .trim()
                .to_string();
            let campos = (
                data(&v[1]),
                data(&v[2]),
                data(&v[3]),
                liquidez(&v[4]),
                valor_obrigatorio(&v[6]),
                valor_obrigatorio(&v[7]),
            );
            let (
                Some(emissao),
                Some(vencimento),
                Some(aquisicao),
                Some(liq),
                Some(aplicado),
                Some(saldo),
            ) = campos
            else {
                self.linha_invalida(secao);
                continue;
            };
            if codigo.is_empty() {
                self.linha_invalida(secao);
                continue;
            }
            let existente = self.renda_fixa.iter_mut().find(|a| {
                matches!(a, AtivoBtg::RendaFixa { tipo: t, codigo: c, .. } if *t == tipo && *c == codigo)
            });
            if let Some(AtivoBtg::RendaFixa {
                primeira_aquisicao,
                aplicado: total,
                saldo_bruto,
                ..
            }) = existente
            {
                *primeira_aquisicao = (*primeira_aquisicao).min(aquisicao);
                *total += aplicado;
                *saldo_bruto += saldo;
                continue;
            }
            let (indexador, taxa, benchmark) = taxa(&v[5]);
            self.renda_fixa.push(AtivoBtg::RendaFixa {
                tipo,
                emissor: emissor.clone(),
                codigo,
                emissao,
                vencimento,
                primeira_aquisicao: aquisicao,
                indexador,
                taxa,
                benchmark,
                liquidez: liq,
                aplicado,
                saldo_bruto: saldo,
            });
        }
    }

    fn renda_variavel(&mut self, secao: &Secao, tipo: &'static str) {
        const COLUNAS: &[Coluna] = &[
            col("Código", &["Código"]),
            col("Ação/Ativo", &["Ação", "Ativo"]),
            col("Qtde.", &["Qtde."]),
            col("Preço Médio R$", &["Preço Médio R$"]),
        ];
        let registros = match ler_tabela(secao, COLUNAS, linha_com_valores) {
            Ok(r) => r,
            Err(f) => return self.falha(secao, f),
        };
        let rotulo = match tipo {
            "rv-acao" => "Ação",
            "rv-bdr" => "BDR",
            "rv-fii" => "FII",
            _ => "ETF",
        };
        for r in registros {
            let v = &r.valores;
            let ticker = v[0].trim_end_matches('*').trim().to_string();
            let (Some(qtde), Some(preco_medio)) =
                (valor_obrigatorio(&v[2]), valor_obrigatorio(&v[3]))
            else {
                self.linha_invalida(secao);
                continue;
            };
            if !qtde.fract().is_zero() {
                self.recusar(format!(
                    "{rotulo} {ticker}: quantidade fracionária não importada."
                ));
                continue;
            }
            let (Some(quantidade), false) = (qtde.to_i64(), ticker.is_empty()) else {
                self.linha_invalida(secao);
                continue;
            };
            self.outros.push(AtivoBtg::RendaVariavel {
                tipo,
                ticker,
                nome: v[1].trim().to_string(),
                quantidade,
                preco_medio,
            });
        }
    }

    fn cripto(&mut self, secao: &Secao) {
        const COLUNAS: &[Coluna] = &[
            col("Ativo", &["Ativo"]),
            col("Saldo Bruto R$", &["Saldo Bruto R$", "Valor Bruto R$"]),
            Coluna {
                nome: "Valor Aplicado R$",
                rotulos: &["Valor Aplicado R$"],
                obrigatoria: false,
            },
        ];
        let registros = match ler_tabela(secao, COLUNAS, linha_com_valores) {
            Ok(r) => r,
            Err(Falha::Coluna(_)) => return self.avisos.push(MSG_CRIPTO.into()),
            Err(f) => return self.falha(secao, f),
        };
        for r in registros {
            let v = &r.valores;
            let nome = v[0].trim().to_string();
            match (valor_obrigatorio(&v[1]), valor(&v[2])) {
                (Some(saldo_bruto), Ok(aplicado)) if !nome.is_empty() => {
                    self.outros.push(AtivoBtg::Cripto {
                        nome,
                        aplicado,
                        saldo_bruto,
                    })
                }
                _ => self.linha_invalida(secao),
            }
        }
    }

    fn concluir(mut self, data: NaiveDate) -> ExtratoBtg {
        let mut ativos: Vec<AtivoBtg> = Vec::new();
        for l in &self.lotes {
            let posicao = self.posicoes.iter().find(|p| p.cnpj == l.cnpj);
            ativos.push(AtivoBtg::Fundo {
                nome: posicao.map_or_else(|| l.nome.clone(), |p| p.nome.clone()),
                cnpj: l.cnpj.clone(),
                aplicado: l.aplicado,
                primeira_compra: l.primeira,
                saldo_bruto: posicao.and_then(|p| p.saldo_bruto),
                referencia: posicao.and_then(|p| p.referencia),
            });
        }
        let sem_lotes: Vec<String> = self
            .posicoes
            .iter()
            .filter(|p| !self.lotes.iter().any(|l| l.cnpj == p.cnpj))
            .map(|p| format!("Fundo {}: sem detalhamento das aplicações.", p.nome))
            .collect();
        for aviso in sem_lotes {
            self.recusar(aviso);
        }
        ativos.append(&mut self.renda_fixa);
        ativos.append(&mut self.outros);
        ExtratoBtg {
            data,
            linhas_lidas: ativos.len() + self.recusadas,
            ativos,
            avisos: self.avisos,
        }
    }
}
