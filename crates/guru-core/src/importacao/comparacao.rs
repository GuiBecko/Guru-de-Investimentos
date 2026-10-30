//! Compara a posição da B3 com os investimentos já cadastrados, linha a linha.

use std::collections::HashSet;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::posicao::{AbaRv, LinhaB3, Posicao};
use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{ErroCampo, Investimento, NovoInvestimento, tipo, validar};
use crate::metas::formatar_reais;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estado {
    Novo,
    Atualizar,
    Divergente,
    Igual,
    ForaDaB3,
}

impl Estado {
    pub fn codigo(&self) -> &'static str {
        match self {
            Estado::Novo => "novo",
            Estado::Atualizar => "atualizar",
            Estado::Divergente => "divergente",
            Estado::Igual => "igual",
            Estado::ForaDaB3 => "fora_da_b3",
        }
    }

    pub fn rotulo(&self) -> &'static str {
        match self {
            Estado::Novo => "novo",
            Estado::Atualizar => "atualizar",
            Estado::Divergente => "divergente",
            Estado::Igual => "igual",
            Estado::ForaDaB3 => "fora da B3",
        }
    }

    /// Ordem de exibição: novo, atualizar, divergente, fora da B3, igual.
    fn ordem(&self) -> u8 {
        match self {
            Estado::Novo => 0,
            Estado::Atualizar => 1,
            Estado::Divergente => 2,
            Estado::ForaDaB3 => 3,
            Estado::Igual => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Adicionar,
    Ignorar,
    UsarB3,
    Manter,
    /// Liga a linha da planilha a este investimento (id) já cadastrado.
    Vincular(i64),
    Remover,
}

impl Acao {
    pub fn codigo(&self) -> &'static str {
        match self {
            Acao::Adicionar => "adicionar",
            Acao::Ignorar => "ignorar",
            Acao::UsarB3 => "usar_b3",
            Acao::Manter => "manter",
            Acao::Vincular(_) => "vincular",
            Acao::Remover => "remover",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    /// `b3:<posição da linha na planilha>` ou `app:<id>`; estável dentro de uma comparação.
    pub chave: String,
    pub estado: Estado,
    pub ativo: String,
    pub instituicao: String,
    pub no_app: Option<String>,
    pub na_b3: Option<String>,
    pub investimentos: Vec<i64>,
    pub acoes: Vec<Acao>,
    pub acao_padrao: Acao,
    pub sugestao: Option<NovoInvestimento>,
    pub faltando: Vec<ErroCampo>,
}

#[derive(Debug, Clone)]
pub struct Comparacao {
    pub itens: Vec<Item>,
    pub avisos: Vec<String>,
    pub linhas_lidas: usize,
}

/// Sem acentos, em maiúsculas, sem pontuação nem sufixo societário (`S/A`, `S.A.`, `SA`, `LTDA`).
fn normalizar_emissor(texto: &str) -> String {
    let sem_acento: String = texto
        .chars()
        .flat_map(char::to_uppercase)
        .map(|c| match c {
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'Ç' => 'C',
            outro => outro,
        })
        .collect();
    let mut palavras: Vec<&str> = sem_acento
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect();
    loop {
        match palavras.as_slice() {
            [.., "S", "A"] => palavras.truncate(palavras.len() - 2),
            [.., "SA" | "LTDA"] => palavras.truncate(palavras.len() - 1),
            _ => break,
        }
    }
    palavras.join(" ")
}

fn quantidade_texto(tipo_codigo: &str, q: i64) -> String {
    let um = q == 1;
    let nome = match tipo_codigo {
        "rv-acao" => ["ação", "ações"],
        "rv-bdr" => ["BDR", "BDRs"],
        _ => ["cota", "cotas"],
    };
    format!("{q} {}", nome[usize::from(!um)])
}

fn centavos(valor: Decimal) -> Option<i64> {
    i64::try_from((valor * Decimal::from(100)).round()).ok()
}

fn tipo_da_aba(aba: AbaRv) -> &'static str {
    match aba {
        AbaRv::Acao => "rv-acao",
        AbaRv::Bdr => "rv-bdr",
        AbaRv::Etf => "rv-etf",
        AbaRv::Fii => "rv-fii",
    }
}

fn faltando(sugestao: &NovoInvestimento, hoje: NaiveDate) -> Vec<ErroCampo> {
    validar(sugestao, hoje).err().unwrap_or_default()
}

/// Estado e ações quando há pares cadastrados e `igual` diz se o total confere.
fn com_pares(pares: &[&Investimento], igual: bool) -> (Estado, Vec<Acao>) {
    if igual {
        return (Estado::Igual, vec![Acao::Manter]);
    }
    match pares {
        [unico] if unico.codigo_b3.is_some() => {
            (Estado::Atualizar, vec![Acao::UsarB3, Acao::Manter])
        }
        [_] => (Estado::Divergente, vec![Acao::Manter, Acao::UsarB3]),
        _ => (Estado::Divergente, vec![Acao::Manter]),
    }
}

fn padrao(estado: Estado) -> Acao {
    match estado {
        Estado::Novo => Acao::Adicionar,
        Estado::Atualizar => Acao::UsarB3,
        _ => Acao::Manter,
    }
}

fn ids(pares: &[&Investimento]) -> Vec<i64> {
    pares.iter().map(|i| i.id).collect()
}

fn total_aplicado(pares: &[&Investimento]) -> i64 {
    pares
        .iter()
        .fold(0i64, |a, i| a.saturating_add(i.valor_aplicado_centavos))
}

struct Contexto<'a> {
    investimentos: &'a [Investimento],
    hoje: NaiveDate,
    usados: HashSet<i64>,
}

impl Contexto<'_> {
    fn marcar(&mut self, pares: &[&Investimento]) {
        self.usados.extend(pares.iter().map(|i| i.id));
    }

    #[allow(clippy::too_many_arguments)]
    fn item(
        &self,
        chave: String,
        estado: Estado,
        ativo: String,
        instituicao: String,
        textos: (Option<String>, Option<String>),
        investimentos: Vec<i64>,
        acoes: Vec<Acao>,
        sugestao: Option<NovoInvestimento>,
    ) -> Item {
        let faltando = sugestao
            .as_ref()
            .map(|s| faltando(s, self.hoje))
            .unwrap_or_default();
        Item {
            chave,
            estado,
            ativo,
            instituicao,
            no_app: textos.0,
            na_b3: textos.1,
            investimentos,
            acao_padrao: padrao(estado),
            acoes,
            sugestao,
            faltando,
        }
    }

    fn renda_variavel(
        &mut self,
        chave: String,
        aba: AbaRv,
        ticker: &str,
        instituicao: &str,
        quantidade: i64,
    ) -> Item {
        let tipo_codigo = tipo_da_aba(aba);
        let pares: Vec<&Investimento> = self
            .investimentos
            .iter()
            .filter(|i| {
                i.tipo.starts_with("rv-")
                    && i.ticker
                        .as_deref()
                        .is_some_and(|t| t.eq_ignore_ascii_case(ticker))
            })
            .collect();
        let na_b3 = Some(quantidade_texto(tipo_codigo, quantidade));
        self.marcar(&pares);
        if pares.is_empty() {
            let sugestao = NovoInvestimento {
                tipo: tipo_codigo.to_string(),
                instituicao: instituicao.to_string(),
                ticker: Some(ticker.to_uppercase()),
                quantidade: Some(quantidade),
                ..Default::default()
            };
            return self.item(
                chave,
                Estado::Novo,
                ticker.to_string(),
                instituicao.to_string(),
                (None, na_b3),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao),
            );
        }
        let soma = pares
            .iter()
            .filter_map(|i| i.quantidade)
            .fold(0i64, i64::saturating_add);
        let (estado, acoes) = com_pares(&pares, soma == quantidade);
        self.item(
            chave,
            estado,
            ticker.to_string(),
            instituicao.to_string(),
            (Some(quantidade_texto(tipo_codigo, soma)), na_b3),
            ids(&pares),
            acoes,
            None,
        )
    }

    fn renda_fixa(&mut self, chave: String, linha: &LinhaB3) -> Option<Item> {
        let LinhaB3::RendaFixa {
            produto,
            tipo: tipo_rf,
            instituicao,
            emissor,
            codigo,
            indexador,
            benchmark,
            emissao,
            vencimento,
            valor_curva,
            ..
        } = linha
        else {
            return None;
        };
        let rf = |i: &&Investimento| i.tipo.starts_with("rf-");
        let por_codigo: Vec<&Investimento> = self
            .investimentos
            .iter()
            .filter(rf)
            .filter(|i| !codigo.is_empty() && i.codigo_b3.as_deref() == Some(codigo))
            .collect();
        let pares = if por_codigo.is_empty() {
            let emissor_norm = normalizar_emissor(emissor);
            let venc = vencimento.to_string();
            self.investimentos
                .iter()
                .filter(rf)
                .filter(|i| i.codigo_b3.is_none() && !self.usados.contains(&i.id))
                .filter(|i| i.vencimento.as_deref() == Some(venc.as_str()))
                .filter(|i| {
                    !emissor_norm.is_empty()
                        && i.emissor.as_deref().map(normalizar_emissor).as_deref()
                            == Some(emissor_norm.as_str())
                })
                .filter(|i| tipo_rf.as_deref().is_none_or(|t| i.tipo == t))
                .collect()
        } else {
            por_codigo
        };
        self.marcar(&pares);
        let ativo = produto.replacen(" - ", " ", 1);
        let na_b3 = valor_curva
            .and_then(centavos)
            .map(|c| format!("{} na curva", formatar_reais(c)));
        let no_app = (!pares.is_empty()).then(|| formatar_reais(total_aplicado(&pares)));
        let sugestao = || {
            let info = tipo_rf.as_deref().and_then(tipo);
            NovoInvestimento {
                tipo: tipo_rf.clone().unwrap_or_default(),
                instituicao: instituicao.clone(),
                emissor: (!emissor.is_empty()).then(|| emissor.clone()),
                indexador: indexador.clone(),
                benchmark: benchmark.clone(),
                data_aplicacao: emissao.map(|d| d.to_string()),
                vencimento: Some(vencimento.to_string()),
                liquidez: info
                    .and_then(|t| t.liquidez_padrao)
                    .map(|l| l.codigo().to_string()),
                ir: info
                    .and_then(|t| t.ir_padrao)
                    .map(|i| i.codigo().to_string()),
                ..Default::default()
            }
        };
        Some(match pares.as_slice() {
            [] => self.item(
                chave,
                Estado::Novo,
                ativo,
                instituicao.clone(),
                (None, na_b3),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao()),
            ),
            [_] => self.item(
                chave,
                Estado::Igual,
                ativo,
                instituicao.clone(),
                (no_app, na_b3),
                ids(&pares),
                vec![Acao::Manter],
                None,
            ),
            _ => {
                let mut acoes = vec![Acao::Manter];
                acoes.extend(pares.iter().map(|i| Acao::Vincular(i.id)));
                acoes.push(Acao::Adicionar);
                self.item(
                    chave,
                    Estado::Divergente,
                    ativo,
                    instituicao.clone(),
                    (no_app, na_b3),
                    ids(&pares),
                    acoes,
                    Some(sugestao()),
                )
            }
        })
    }

    fn tesouro(&mut self, chave: String, linha: &LinhaB3) -> Option<Item> {
        let LinhaB3::Tesouro {
            produto,
            tipo: tipo_td,
            instituicao,
            vencimento,
            valor_aplicado,
            ..
        } = linha
        else {
            return None;
        };
        let venc = vencimento.to_string();
        let pares: Vec<&Investimento> = self
            .investimentos
            .iter()
            .filter(|i| i.tipo == *tipo_td && i.vencimento.as_deref() == Some(venc.as_str()))
            .collect();
        self.marcar(&pares);
        let aplicado_b3 = valor_aplicado.and_then(centavos);
        let na_b3 = aplicado_b3.map(formatar_reais);
        if pares.is_empty() {
            let info = tipo(tipo_td);
            let sugestao = NovoInvestimento {
                tipo: tipo_td.clone(),
                instituicao: instituicao.clone(),
                vencimento: Some(venc),
                valor_aplicado_centavos: aplicado_b3,
                liquidez: info
                    .and_then(|t| t.liquidez_padrao)
                    .map(|l| l.codigo().to_string()),
                ir: info
                    .and_then(|t| t.ir_padrao)
                    .map(|i| i.codigo().to_string()),
                ..Default::default()
            };
            return Some(self.item(
                chave,
                Estado::Novo,
                produto.clone(),
                instituicao.clone(),
                (None, na_b3),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao),
            ));
        }
        let soma = total_aplicado(&pares);
        let igual = aplicado_b3.is_some_and(|b| b.abs_diff(soma) <= 1);
        let (estado, acoes) = com_pares(&pares, igual);
        Some(self.item(
            chave,
            estado,
            produto.clone(),
            instituicao.clone(),
            (Some(formatar_reais(soma)), na_b3),
            ids(&pares),
            acoes,
            None,
        ))
    }

    fn fora_da_b3(&self, inv: &Investimento) -> Item {
        let no_app = match (inv.tipo.starts_with("rv-"), inv.quantidade) {
            (true, Some(q)) => quantidade_texto(&inv.tipo, q),
            _ => formatar_reais(inv.valor_aplicado_centavos),
        };
        self.item(
            format!("app:{}", inv.id),
            Estado::ForaDaB3,
            inv.nome.clone(),
            inv.instituicao.clone(),
            (Some(no_app), None),
            vec![inv.id],
            vec![Acao::Manter, Acao::Remover],
            None,
        )
    }
}

/// Compara cada linha da posição com a carteira. `hoje` só serve para validar as sugestões.
pub fn comparar(db: &Db, posicao: &Posicao, hoje: NaiveDate) -> Result<Comparacao, CoreError> {
    let investimentos = db.listar_investimentos_brutos()?;
    let mut ctx = Contexto {
        investimentos: &investimentos,
        hoje,
        usados: HashSet::new(),
    };
    let mut itens = Vec::new();
    // Linhas de renda variável com o mesmo ticker (corretoras diferentes) valem como um item só.
    let mut grupos: Vec<(String, usize, AbaRv, i64, Vec<&str>)> = Vec::new();
    for (n, linha) in posicao.linhas.iter().enumerate() {
        if let LinhaB3::RendaVariavel {
            aba,
            ticker,
            instituicao,
            quantidade,
            ..
        } = linha
        {
            let chave = ticker.to_uppercase();
            match grupos.iter_mut().find(|g| g.0 == chave) {
                Some(g) => {
                    g.3 = g.3.saturating_add(*quantidade);
                    if !g.4.contains(&instituicao.as_str()) {
                        g.4.push(instituicao);
                    }
                }
                None => grupos.push((chave, n, *aba, *quantidade, vec![instituicao])),
            }
        }
    }
    for (n, linha) in posicao.linhas.iter().enumerate() {
        let chave = format!("b3:{n}");
        let item = match linha {
            LinhaB3::RendaVariavel { ticker, .. } => {
                let ticker_maiusculo = ticker.to_uppercase();
                grupos
                    .iter()
                    .find(|g| g.0 == ticker_maiusculo && g.1 == n)
                    .map(|(_, _, aba, quantidade, instituicoes)| {
                        ctx.renda_variavel(
                            chave,
                            *aba,
                            ticker,
                            &instituicoes.join(", "),
                            *quantidade,
                        )
                    })
            }
            LinhaB3::RendaFixa { .. } => ctx.renda_fixa(chave, linha),
            LinhaB3::Tesouro { .. } => ctx.tesouro(chave, linha),
        };
        itens.extend(item);
    }
    for inv in &investimentos {
        if !ctx.usados.contains(&inv.id) {
            itens.push(ctx.fora_da_b3(inv));
        }
    }
    itens.sort_by_key(|i| i.estado.ordem());
    Ok(Comparacao {
        itens,
        avisos: posicao.avisos.clone(),
        linhas_lidas: posicao.linhas_lidas,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn emissor_sem_sufixos_nem_acentos() {
        assert_eq!(normalizar_emissor("Banco Fictício S/A"), "BANCO FICTICIO");
        assert_eq!(normalizar_emissor("BANCO FICTICIO S.A."), "BANCO FICTICIO");
        assert_eq!(normalizar_emissor("Banco Fictício SA"), "BANCO FICTICIO");
        assert_eq!(
            normalizar_emissor("Financeira  Exemplo Ltda."),
            "FINANCEIRA EXEMPLO"
        );
        assert_eq!(normalizar_emissor("Sá Crédito"), "SA CREDITO");
    }
}
