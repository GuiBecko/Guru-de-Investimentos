//! Aplica as decisões do usuário sobre a comparação, tudo numa transação só.
//!
//! Regras:
//! - Nada é apagado a não ser por um `Remover` explícito.
//! - `UsarB3` na renda variável muda só a quantidade e recalcula o aplicado (quantidade × preço
//!   médio, sem tocar no preço médio); no Tesouro muda só o valor aplicado.
//! - Quando o item casa com um único investimento sem `codigo_b3` e o casamento foi confirmado
//!   (estado `Igual`, ou o usuário escolheu `UsarB3`), o código da B3 é gravado nele, qualquer que
//!   seja a ação escolhida. Isso não conta como alteração em `aplicadas`. Um item divergente
//!   mantido não ganha código: o usuário não confirmou que é o mesmo ativo.

use std::collections::HashSet;

use chrono::NaiveDate;
use rusqlite::{Connection, named_params};

use super::comparacao::{Acao, Comparacao, Estado, Item};
use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::{
    ErroCampo, InvestimentoValido, MAX_CENTAVOS, NovoInvestimento, apagar_em, inserir_validado,
    validar_com_meta,
};

#[derive(Debug, Clone)]
pub struct Decisao {
    pub chave: String,
    pub acao: Acao,
    /// Para `Adicionar`: a sugestão completada pelo usuário (sem ela vale a sugestão do item).
    pub dados: Option<NovoInvestimento>,
}

#[derive(Debug, Clone)]
pub struct ResultadoImportacao {
    pub aplicadas: usize,
    /// Quando não vazio, nada foi gravado.
    pub erros: Vec<(String, Vec<ErroCampo>)>,
}

enum Operacao {
    Inserir {
        valido: Box<InvestimentoValido>,
        meta_id: Option<i64>,
        codigo: Option<String>,
    },
    Quantidade {
        id: i64,
        quantidade: i64,
        aplicado: i64,
    },
    Valor {
        id: i64,
        aplicado: i64,
    },
    Vincular {
        id: i64,
        codigo: String,
    },
    Apagar(i64),
}

fn invalida(msg: impl Into<String>) -> CoreError {
    CoreError::DecisaoInvalida(msg.into())
}

fn erro(campo: &str, mensagem: &str) -> Vec<ErroCampo> {
    vec![ErroCampo {
        campo: campo.to_string(),
        mensagem: mensagem.to_string(),
    }]
}

/// Quantidade e preço médio gravados do investimento.
fn quantidade_e_preco(conn: &Connection, id: i64) -> Result<(Option<i64>, Option<i64>), CoreError> {
    conn.query_row(
        "SELECT quantidade, preco_medio_centavos FROM investimentos WHERE id = :id",
        named_params! {":id": id},
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => CoreError::NaoEncontrado(id),
        outro => classifica(outro),
    })
}

fn so_um(item: &Item) -> Result<i64, CoreError> {
    match item.investimentos.as_slice() {
        [id] => Ok(*id),
        _ => Err(invalida(format!(
            "A ação precisa de um único investimento ({}).",
            item.chave
        ))),
    }
}

/// Operações do item (o `bool` diz se contam como alteração) ou os erros de campo.
type Plano = Result<Vec<(Operacao, bool)>, Vec<ErroCampo>>;

/// Decide o que gravar para o item; devolve erros de campo sem escrever nada.
fn planejar(
    conn: &Connection,
    item: &Item,
    decisao: Option<&Decisao>,
    hoje: NaiveDate,
) -> Result<Plano, CoreError> {
    let acao = decisao.map(|d| d.acao);
    let mut ops: Vec<(Operacao, bool)> = Vec::new();
    match acao {
        Some(Acao::Adicionar) => {
            let dados = decisao
                .and_then(|d| d.dados.clone())
                .or_else(|| item.sugestao.clone())
                .ok_or_else(|| invalida(format!("Faltam os dados de {}.", item.chave)))?;
            match validar_com_meta(conn, &dados, hoje)? {
                Ok(valido) => ops.push((
                    Operacao::Inserir {
                        valido: Box::new(valido),
                        meta_id: dados.meta_id,
                        codigo: item.b3.codigo.clone(),
                    },
                    true,
                )),
                Err(erros) => return Ok(Err(erros)),
            }
        }
        Some(Acao::UsarB3) => {
            let id = so_um(item)?;
            let (quantidade, preco) = quantidade_e_preco(conn, id)?;
            if let Some(q) = item.b3.quantidade {
                if q <= 0 {
                    return Ok(Err(erro("quantidade", "Informe a quantidade.")));
                }
                let aplicado = preco
                    .and_then(|p| q.checked_mul(p))
                    .filter(|a| *a <= MAX_CENTAVOS);
                match (quantidade, aplicado) {
                    (Some(_), Some(aplicado)) => ops.push((
                        Operacao::Quantidade {
                            id,
                            quantidade: q,
                            aplicado,
                        },
                        true,
                    )),
                    _ => {
                        return Ok(Err(erro(
                            "quantidade",
                            "Quantidade vezes preço médio passa do limite.",
                        )));
                    }
                }
            } else {
                match item.b3.aplicado_centavos {
                    Some(a) if (1..=MAX_CENTAVOS).contains(&a) => {
                        ops.push((Operacao::Valor { id, aplicado: a }, true));
                    }
                    _ => {
                        return Ok(Err(erro(
                            "valorAplicadoCentavos",
                            "A planilha não traz um valor aplicado válido.",
                        )));
                    }
                }
            }
        }
        Some(Acao::Vincular(id)) => {
            let codigo = item
                .b3
                .codigo
                .clone()
                .ok_or_else(|| invalida(format!("A linha {} não tem código.", item.chave)))?;
            ops.push((Operacao::Vincular { id, codigo }, true));
        }
        Some(Acao::Remover) => ops.push((Operacao::Apagar(so_um(item)?), true)),
        Some(Acao::Manter | Acao::Ignorar) | None => {}
    }
    let confirmado = item.estado == Estado::Igual || acao == Some(Acao::UsarB3);
    if confirmado && let ([id], Some(codigo)) = (item.investimentos.as_slice(), &item.b3.codigo) {
        ops.push((
            Operacao::Vincular {
                id: *id,
                codigo: codigo.clone(),
            },
            false,
        ));
    }
    Ok(Ok(ops))
}

fn executar(conn: &Connection, op: &Operacao, contar: bool) -> Result<bool, CoreError> {
    const AGORA: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";
    let alterou = match op {
        Operacao::Inserir {
            valido,
            meta_id,
            codigo,
        } => {
            let id = inserir_validado(conn, (**valido).clone(), *meta_id)?;
            conn.execute(
                "UPDATE investimentos SET codigo_b3 = :codigo WHERE id = :id",
                named_params! {":codigo": codigo, ":id": id},
            )
            .map_err(classifica)?;
            true
        }
        Operacao::Quantidade {
            id,
            quantidade,
            aplicado,
        } => {
            let n = conn
                .execute(
                    &format!(
                        "UPDATE investimentos SET quantidade = :q, valor_aplicado_centavos = :a, \
                         atualizado_em = {AGORA} WHERE id = :id"
                    ),
                    named_params! {":q": quantidade, ":a": aplicado, ":id": id},
                )
                .map_err(classifica)?;
            n > 0
        }
        Operacao::Valor { id, aplicado } => {
            let n = conn
                .execute(
                    &format!(
                        "UPDATE investimentos SET valor_aplicado_centavos = :a, \
                         atualizado_em = {AGORA} WHERE id = :id"
                    ),
                    named_params! {":a": aplicado, ":id": id},
                )
                .map_err(classifica)?;
            n > 0
        }
        Operacao::Vincular { id, codigo } => {
            // A ligação implícita só preenche código vazio; o `Vincular` do usuário vale sempre.
            let sql = if contar {
                "UPDATE investimentos SET codigo_b3 = :c WHERE id = :id"
            } else {
                "UPDATE investimentos SET codigo_b3 = :c WHERE id = :id \
                 AND (codigo_b3 IS NULL OR codigo_b3 = '')"
            };
            let n = conn
                .execute(sql, named_params! {":c": codigo, ":id": id})
                .map_err(classifica)?;
            if n == 0 && contar {
                return Err(CoreError::NaoEncontrado(*id));
            }
            true
        }
        Operacao::Apagar(id) => {
            apagar_em(conn, *id)?;
            true
        }
    };
    Ok(alterou && contar)
}

/// Aplica as decisões de uma vez: ou grava tudo, ou nada. `hoje` é a data da importação.
///
/// Decisão para chave desconhecida, ação fora de `item.acoes` ou repetida é `DECISAO_INVALIDA`.
/// Item sem decisão fica como está (só recebe o código da B3 quando o casamento é seguro).
pub fn aplicar_importacao(
    db: &Db,
    comparacao: &Comparacao,
    decisoes: &[Decisao],
    arquivo: &str,
    hoje: NaiveDate,
) -> Result<ResultadoImportacao, CoreError> {
    let mut vistas = HashSet::new();
    for d in decisoes {
        let item = comparacao
            .itens
            .iter()
            .find(|i| i.chave == d.chave)
            .ok_or_else(|| invalida(format!("Linha desconhecida: {}.", d.chave)))?;
        if !item.acoes.contains(&d.acao) {
            return Err(invalida(format!(
                "A ação {} não vale para {}.",
                d.acao.codigo(),
                item.ativo
            )));
        }
        if !vistas.insert(d.chave.as_str()) {
            return Err(invalida(format!("Mais de uma decisão para {}.", d.chave)));
        }
    }

    // Dropar a transação sem `commit` desfaz tudo, inclusive nos retornos antecipados.
    let tx = db.conn.unchecked_transaction().map_err(classifica)?;
    let mut plano = Vec::new();
    let mut erros = Vec::new();
    for item in &comparacao.itens {
        let decisao = decisoes.iter().find(|d| d.chave == item.chave);
        match planejar(&tx, item, decisao, hoje)? {
            Ok(ops) => plano.extend(ops),
            Err(e) => erros.push((item.chave.clone(), e)),
        }
    }
    if !erros.is_empty() {
        return Ok(ResultadoImportacao {
            aplicadas: 0,
            erros,
        });
    }

    let mut aplicadas = 0;
    for (op, contar) in &plano {
        if executar(&tx, op, *contar)? {
            aplicadas += 1;
        }
    }
    tx.execute(
        "INSERT INTO importacao (data, arquivo, linhas, aplicadas) VALUES (:d, :a, :l, :n)",
        named_params! {
            ":d": hoje.to_string(),
            ":a": arquivo,
            ":l": i64::try_from(comparacao.linhas_lidas).unwrap_or(i64::MAX),
            ":n": i64::try_from(aplicadas).unwrap_or(i64::MAX),
        },
    )
    .map_err(classifica)?;
    tx.commit().map_err(classifica)?;
    Ok(ResultadoImportacao {
        aplicadas,
        erros: Vec::new(),
    })
}

/// Data (`AAAA-MM-DD`) da última importação aplicada.
pub fn ultima_importacao(db: &Db) -> Result<Option<String>, CoreError> {
    db.conn
        .query_row("SELECT MAX(data) FROM importacao", [], |r| r.get(0))
        .map_err(classifica)
}
