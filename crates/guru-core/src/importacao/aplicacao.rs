//! Aplica as decisões do usuário sobre a comparação, tudo numa transação só.
//!
//! Regras:
//! - Nada é apagado por importação.
//! - `UsarArquivo` na renda variável muda só a quantidade e recalcula o aplicado (quantidade × preço
//!   médio, sem tocar no preço médio); no Tesouro muda só o valor aplicado.
//! - Quando o item casa com um único investimento sem `codigo_b3` e o casamento foi confirmado
//!   (estado `Igual`, ou o usuário escolheu `UsarArquivo`), o código da fonte é gravado nele, qualquer que
//!   seja a ação escolhida. Isso não conta como alteração em `aplicadas`. Um item divergente
//!   mantido não ganha código: o usuário não confirmou que é o mesmo ativo.
//! - Item **confirmado** = `Igual` com um par, `UsarArquivo`, `Vincular(id)` ou `Adicionar` (no
//!   investimento recém-inserido). Nele se grava também o saldo do extrato (substitui o anterior:
//!   o extrato mais recente vale) e se completam os campos vazios (emissor; indexador, taxa e
//!   benchmark juntos quando a taxa está vazia; preço médio). Nada disso conta em `aplicadas`, e
//!   campo já preenchido nunca é sobrescrito.
//! - `UsarArquivo` na renda variável sem preço médio gravado usa o preço médio do arquivo.

use std::collections::HashSet;
use std::str::FromStr;

use chrono::NaiveDate;
use rusqlite::{Connection, Transaction, TransactionBehavior, named_params};
use rust_decimal::Decimal;

use super::comparacao::{Acao, Comparacao, Completar, DadosArquivo, Estado, Item};
use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::{
    ErroCampo, InvestimentoValido, MAX_CENTAVOS, NovoInvestimento, inserir_validado,
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
        /// Código, saldo e campos a completar do arquivo, gravados no investimento inserido.
        dados: Box<DadosArquivo>,
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
    Saldo {
        id: i64,
        centavos: i64,
        data: NaiveDate,
    },
    Completar {
        id: i64,
        completar: Option<Completar>,
        preco_medio: Option<i64>,
    },
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
                        dados: Box::new(item.dados.clone()),
                    },
                    true,
                )),
                Err(erros) => return Ok(Err(erros)),
            }
        }
        Some(Acao::UsarArquivo) => {
            let id = so_um(item)?;
            let (quantidade, preco) = quantidade_e_preco(conn, id)?;
            if let Some(q) = item.dados.quantidade {
                if q <= 0 {
                    return Ok(Err(erro("quantidade", "Informe a quantidade.")));
                }
                // Preço médio vazio no app: vale o do arquivo (a operação `Completar` o grava).
                let Some(preco) = preco.or(item.dados.preco_medio_centavos) else {
                    return Ok(Err(erro("precoMedioCentavos", "Informe o preço médio.")));
                };
                if quantidade.is_none() {
                    return Ok(Err(erro("quantidade", "Informe a quantidade.")));
                }
                match q.checked_mul(preco).filter(|a| *a <= MAX_CENTAVOS) {
                    Some(aplicado) => ops.push((
                        Operacao::Quantidade {
                            id,
                            quantidade: q,
                            aplicado,
                        },
                        true,
                    )),
                    None => {
                        return Ok(Err(erro(
                            "quantidade",
                            "Quantidade vezes preço médio passa do limite.",
                        )));
                    }
                }
            } else {
                match item.dados.aplicado_centavos {
                    Some(a) if (1..=MAX_CENTAVOS).contains(&a) => {
                        ops.push((Operacao::Valor { id, aplicado: a }, true));
                    }
                    _ => {
                        return Ok(Err(erro(
                            "valorAplicadoCentavos",
                            "O arquivo não traz um valor aplicado válido.",
                        )));
                    }
                }
            }
        }
        Some(Acao::Vincular(id)) => {
            let codigo = item
                .dados
                .codigo
                .clone()
                .ok_or_else(|| invalida(format!("A linha {} não tem código.", item.chave)))?;
            ops.push((Operacao::Vincular { id, codigo }, true));
        }
        Some(Acao::Manter | Acao::Ignorar) | None => {}
    }
    let confirmado = item.estado == Estado::Igual || acao == Some(Acao::UsarArquivo);
    let mut alvo = None;
    if confirmado && let [id] = item.investimentos.as_slice() {
        alvo = Some(*id);
        if let Some(codigo) = &item.dados.codigo {
            ops.push((
                Operacao::Vincular {
                    id: *id,
                    codigo: codigo.clone(),
                },
                false,
            ));
        }
    }
    if let Some(Acao::Vincular(id)) = acao {
        alvo = Some(id);
    }
    // O `Adicionar` grava saldo e campos no investimento inserido (ver `executar`).
    if let Some(id) = alvo {
        ops.extend(extras(id, &item.dados).into_iter().map(|op| (op, false)));
    }
    Ok(Ok(ops))
}

/// Saldo e campos a completar do arquivo para o investimento `id` (item confirmado).
fn extras(id: i64, dados: &DadosArquivo) -> Vec<Operacao> {
    let mut ops = Vec::new();
    if let Some((centavos, data)) = dados.saldo {
        ops.push(Operacao::Saldo { id, centavos, data });
    }
    if dados.completar.is_some() || dados.preco_medio_centavos.is_some() {
        ops.push(Operacao::Completar {
            id,
            completar: dados.completar.clone(),
            preco_medio: dados.preco_medio_centavos,
        });
    }
    ops
}

fn executar(conn: &Connection, op: &Operacao, contar: bool) -> Result<bool, CoreError> {
    const AGORA: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";
    let alterou = match op {
        Operacao::Inserir {
            valido,
            meta_id,
            dados,
        } => {
            let id = inserir_validado(conn, (**valido).clone(), *meta_id)?;
            conn.execute(
                "UPDATE investimentos SET codigo_b3 = :codigo WHERE id = :id",
                named_params! {":codigo": dados.codigo, ":id": id},
            )
            .map_err(classifica)?;
            for op in extras(id, dados) {
                executar(conn, &op, false)?;
            }
            true
        }
        Operacao::Saldo { id, centavos, data } => {
            conn.execute(
                "UPDATE investimentos SET saldo_centavos = :c, saldo_data = :d WHERE id = :id",
                named_params! {":c": centavos, ":d": data.to_string(), ":id": id},
            )
            .map_err(classifica)?;
            true
        }
        Operacao::Completar {
            id,
            completar,
            preco_medio,
        } => {
            // Só o que está vazio. Indexador, taxa e benchmark andam juntos: entram só quando a
            // taxa está vazia e o arquivo traz uma.
            let c = completar.clone().unwrap_or_default();
            // Gravada como o cadastro grava (decimal normalizado); texto inválido não entra.
            let taxa = c
                .taxa
                .as_deref()
                .and_then(|t| Decimal::from_str(t).ok())
                .map(|t| t.normalize().to_string());
            conn.execute(
                "UPDATE investimentos SET \
                 emissor = COALESCE(NULLIF(emissor, ''), :e), \
                 indexador = CASE WHEN (taxa IS NULL OR taxa = '') AND :t IS NOT NULL \
                     THEN :i ELSE indexador END, \
                 benchmark = CASE WHEN (taxa IS NULL OR taxa = '') AND :t IS NOT NULL \
                     THEN :b ELSE benchmark END, \
                 taxa = CASE WHEN (taxa IS NULL OR taxa = '') AND :t IS NOT NULL \
                     THEN :t ELSE taxa END, \
                 preco_medio_centavos = COALESCE(preco_medio_centavos, :p) \
                 WHERE id = :id",
                named_params! {
                    ":e": c.emissor,
                    ":i": c.indexador,
                    ":t": taxa,
                    ":b": c.benchmark,
                    ":p": preco_medio,
                    ":id": id,
                },
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
    };
    Ok(alterou && contar)
}

/// Um investimento só pode ser ligado a uma linha: dois `Vincular` para o mesmo id, ou um
/// `Vincular` para um id que outro item já confirma (igual ou `UsarArquivo`), é
/// `DECISAO_INVALIDA`.
fn conferir_vinculos(comparacao: &Comparacao, decisoes: &[Decisao]) -> Result<(), CoreError> {
    let acao_de = |item: &Item| {
        decisoes
            .iter()
            .find(|d| d.chave == item.chave)
            .map(|d| d.acao)
    };
    let mut ocupados = HashSet::new();
    for item in &comparacao.itens {
        let acao = acao_de(item);
        let ocupa = item.estado == Estado::Igual || acao == Some(Acao::UsarArquivo);
        if let ([id], true) = (item.investimentos.as_slice(), ocupa) {
            ocupados.insert(*id);
        }
    }
    let mut vinculados = HashSet::new();
    for d in decisoes {
        if let Acao::Vincular(id) = d.acao
            && (ocupados.contains(&id) || !vinculados.insert(id))
        {
            return Err(invalida(
                "Cada investimento só pode ser vinculado a uma linha do arquivo.",
            ));
        }
    }
    Ok(())
}

/// Aplica as decisões de uma vez: ou grava tudo, ou nada. `hoje` é a data da importação.
///
/// Decisão para chave desconhecida, ação fora de `item.acoes` ou repetida é `DECISAO_INVALIDA`.
/// Item sem decisão não muda o que já estava gravado: quando o casamento é seguro (igual), só
/// recebe o código do arquivo, o saldo e os campos vazios.
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
    conferir_vinculos(comparacao, decisoes)?;

    // Dropar a transação sem `commit` desfaz tudo, inclusive nos retornos antecipados.
    // `IMMEDIATE` pega a escrita logo no início (esperando o `busy_timeout` se a atualização de
    // mercado estiver gravando): uma transação adiada que já leu falharia ao passar a escrever.
    let tx =
        Transaction::new_unchecked(&db.conn, TransactionBehavior::Immediate).map_err(classifica)?;
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
        "INSERT INTO importacao (data, arquivo, linhas, aplicadas, fonte) \
         VALUES (:d, :a, :l, :n, :f)",
        named_params! {
            ":d": hoje.to_string(),
            ":a": arquivo,
            ":l": i64::try_from(comparacao.linhas_lidas).unwrap_or(i64::MAX),
            ":n": i64::try_from(aplicadas).unwrap_or(i64::MAX),
            ":f": comparacao.fonte.codigo(),
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
