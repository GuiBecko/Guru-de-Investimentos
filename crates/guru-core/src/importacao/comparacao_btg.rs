//! Compara o extrato da BTG com os investimentos já cadastrados, ativo a ativo.

use std::collections::HashSet;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::btg::{AtivoBtg, ExtratoBtg};
use super::comparacao::{
    Acao, Comparacao, Completar, Contexto, DadosArquivo, Estado, Fonte, Item, centavos,
    chave_ativo_rv, chaves_ativo_rf, com_pares, ids, normalizar_emissor, quantidade_texto,
    total_aplicado,
};
use super::instituicao::chave_instituicao;
use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{Investimento, MAX_CENTAVOS, NovoInvestimento, tipo};
use crate::metas::formatar_reais;

/// Instituição de tudo o que vem do extrato (o nome do catálogo).
const INSTITUICAO: &str = "BTG Pactual";

/// Valores do ativo em centavos; `None` no campo quando o extrato não o traz.
struct Valores {
    aplicado: Option<i64>,
    saldo: Option<i64>,
    preco_medio: Option<i64>,
}

/// O que se compara entre o arquivo e os pares cadastrados.
enum Medida {
    /// Renda variável: quantidade.
    Quantidade(i64),
    /// Valor aplicado, com tolerância de 1 centavo.
    Aplicado(i64),
    /// Cripto sem valor aplicado no extrato: um par único é igual.
    SemAplicado,
}

/// Teste de casamento de um investimento cadastrado.
type Casa<'r> = Box<dyn Fn(&Investimento) -> bool + 'r>;

/// Como um ativo do extrato procura o seu par na carteira.
struct Regra<'r> {
    /// Nome exibido no item.
    ativo: String,
    /// Chaves do ativo para `Contexto::ocupadas`.
    chaves: Vec<String>,
    /// Tipo do investimento compatível.
    tipo_ok: Casa<'r>,
    /// Casamento pelo código gravado (ou pelo ticker, na renda variável).
    por_codigo: Casa<'r>,
    /// `por_codigo` compara o código gravado (`codigo_b3`), não o ticker: o candidato de outra
    /// corretora achado assim vale mesmo numa corretora do catálogo.
    codigo_gravado: bool,
    /// Casamento de investimento sem código gravado; `None` quando não há regra.
    sem_codigo: Option<Casa<'r>>,
    /// Nome parecido mas não igual (fundo e cripto): candidato a `Vincular`, nunca par.
    parecido: Option<Casa<'r>>,
    medida: Medida,
    tipo_codigo: &'static str,
    no_arquivo: Option<String>,
    sugestao: NovoInvestimento,
    dados: DadosArquivo,
}

fn nome_do_ativo(ativo: &AtivoBtg) -> String {
    match ativo {
        AtivoBtg::Fundo { nome, .. } | AtivoBtg::Cripto { nome, .. } => nome.clone(),
        AtivoBtg::RendaVariavel { ticker, .. } => ticker.clone(),
        AtivoBtg::RendaFixa {
            tipo: t, emissor, ..
        } => {
            format!("{} {emissor}", tipo(t).map_or(*t, |i| i.rotulo))
        }
    }
}

/// Converte os valores usados do ativo; `None` se o aplicado ou o preço médio não cabe em
/// centavos. Preço médio zero ou negativo fica vazio. Saldo fora de `0..=MAX_CENTAVOS`, ou de fundo
/// sem data de referência, fica vazio e vira aviso.
fn valores(ativo: &AtivoBtg, avisos: &mut Vec<String>) -> Option<Valores> {
    let opcional = |v: Option<Decimal>| match v {
        Some(v) => centavos(v).map(Some),
        None => Some(None),
    };
    let mut saldo = |v: Option<Decimal>| {
        let c = v.map(|v| centavos(v).filter(|c| (0..=MAX_CENTAVOS).contains(c)));
        if c == Some(None) {
            avisos.push(format!(
                "Saldo fora do limite em {}, não importado.",
                nome_do_ativo(ativo)
            ));
        }
        c.flatten()
    };
    Some(match ativo {
        AtivoBtg::Fundo {
            nome,
            aplicado,
            saldo_bruto,
            referencia,
            ..
        } => {
            let aplicado = Some(centavos(*aplicado)?);
            let mut s = saldo(*saldo_bruto);
            if s.is_some() && referencia.is_none() {
                // Sem a data, o saldo não é atribuído à data do extrato: fica de fora.
                avisos.push(format!(
                    "Fundo {nome}: saldo sem data de referência, não importado."
                ));
                s = None;
            }
            Valores {
                aplicado,
                saldo: s,
                preco_medio: None,
            }
        }
        AtivoBtg::RendaFixa { aplicado, .. } => Valores {
            aplicado: Some(centavos(*aplicado)?),
            saldo: None,
            preco_medio: None,
        },
        AtivoBtg::RendaVariavel { preco_medio, .. } => Valores {
            aplicado: None,
            saldo: None,
            preco_medio: Some(centavos(*preco_medio)?).filter(|p| *p > 0),
        },
        AtivoBtg::Cripto {
            aplicado,
            saldo_bruto,
            ..
        } => Valores {
            aplicado: opcional(*aplicado)?,
            saldo: saldo(Some(*saldo_bruto)),
            preco_medio: None,
        },
    })
}

fn chaves_do_ativo(ativo: &AtivoBtg) -> Vec<String> {
    match ativo {
        AtivoBtg::RendaVariavel { ticker, .. } => vec![chave_ativo_rv(ticker)],
        AtivoBtg::RendaFixa {
            codigo,
            vencimento,
            emissor,
            ..
        } => chaves_ativo_rf(codigo, *vencimento, emissor),
        AtivoBtg::Fundo { nome, cnpj, .. } => vec![
            format!("fundo:c:{cnpj}"),
            format!("fundo:n:{}", normalizar_emissor(nome)),
        ],
        AtivoBtg::Cripto { nome, .. } => vec![format!("cripto:{}", normalizar_emissor(nome))],
    }
}

fn liquidez_padrao(tipo_codigo: &str) -> Option<String> {
    tipo(tipo_codigo)
        .and_then(|t| t.liquidez_padrao)
        .map(|l| l.codigo().to_string())
}

fn ir_padrao(tipo_codigo: &str) -> Option<String> {
    tipo(tipo_codigo)
        .and_then(|t| t.ir_padrao)
        .map(|i| i.codigo().to_string())
}

fn emissor_igual(i: &Investimento, normal: &str) -> bool {
    !normal.is_empty() && i.emissor.as_deref().map(normalizar_emissor).as_deref() == Some(normal)
}

/// As palavras de `curto` aparecem em sequência, inteiras, em `longo`.
fn contem_palavras(longo: &str, curto: &str) -> bool {
    let longo: Vec<&str> = longo.split(' ').filter(|p| !p.is_empty()).collect();
    let curto: Vec<&str> = curto.split(' ').filter(|p| !p.is_empty()).collect();
    !curto.is_empty() && longo.windows(curto.len()).any(|w| w == curto.as_slice())
}

/// Nome normalizado parecido mas não igual: um contém o outro em palavras inteiras.
fn emissor_parecido(i: &Investimento, normal: &str) -> bool {
    let Some(outro) = i.emissor.as_deref().map(normalizar_emissor) else {
        return false;
    };
    outro != normal && (contem_palavras(normal, &outro) || contem_palavras(&outro, normal))
}

fn regra<'r>(ativo: &'r AtivoBtg, v: &Valores, data: NaiveDate) -> Regra<'r> {
    let nome = nome_do_ativo(ativo);
    let chaves = chaves_do_ativo(ativo);
    match ativo {
        AtivoBtg::RendaVariavel {
            tipo: t,
            ticker,
            quantidade,
            ..
        } => Regra {
            ativo: nome,
            chaves,
            tipo_ok: Box::new(|i| i.tipo.starts_with("rv-")),
            por_codigo: Box::new(move |i| {
                i.ticker
                    .as_deref()
                    .is_some_and(|x| x.eq_ignore_ascii_case(ticker))
            }),
            codigo_gravado: false,
            sem_codigo: None,
            parecido: None,
            medida: Medida::Quantidade(*quantidade),
            tipo_codigo: t,
            no_arquivo: Some(quantidade_texto(t, *quantidade)),
            sugestao: NovoInvestimento {
                tipo: t.to_string(),
                instituicao: INSTITUICAO.into(),
                ticker: Some(ticker.to_uppercase()),
                quantidade: Some(*quantidade),
                preco_medio_centavos: v.preco_medio,
                data_aplicacao: Some(data.to_string()),
                ..Default::default()
            },
            dados: DadosArquivo {
                codigo: Some(ticker.to_uppercase()),
                quantidade: Some(*quantidade),
                preco_medio_centavos: v.preco_medio,
                ..Default::default()
            },
        },
        AtivoBtg::RendaFixa {
            tipo: t,
            emissor,
            codigo,
            vencimento,
            primeira_aquisicao,
            indexador,
            taxa,
            benchmark,
            liquidez,
            ..
        } => {
            let aplicado = v.aplicado.unwrap_or_default();
            let emissor_norm = normalizar_emissor(emissor);
            let venc = vencimento.to_string();
            let taxa_texto = taxa.map(|x| x.to_string());
            Regra {
                ativo: nome,
                chaves,
                tipo_ok: Box::new(|i| i.tipo.starts_with("rf-")),
                por_codigo: Box::new(move |i| {
                    !codigo.is_empty() && i.codigo_b3.as_deref() == Some(codigo.as_str())
                }),
                codigo_gravado: true,
                sem_codigo: Some(Box::new(move |i| {
                    i.tipo == *t
                        && i.vencimento.as_deref() == Some(venc.as_str())
                        && emissor_igual(i, &emissor_norm)
                })),
                parecido: None,
                medida: Medida::Aplicado(aplicado),
                tipo_codigo: t,
                no_arquivo: Some(formatar_reais(aplicado)),
                sugestao: NovoInvestimento {
                    tipo: t.to_string(),
                    instituicao: INSTITUICAO.into(),
                    emissor: Some(emissor.clone()),
                    indexador: indexador.map(str::to_string),
                    taxa: taxa_texto.clone(),
                    benchmark: benchmark.map(str::to_string),
                    data_aplicacao: Some(primeira_aquisicao.to_string()),
                    vencimento: Some(vencimento.to_string()),
                    valor_aplicado_centavos: Some(aplicado),
                    liquidez: Some(liquidez.to_string()),
                    ir: ir_padrao(t),
                    ..Default::default()
                },
                dados: DadosArquivo {
                    codigo: (!codigo.is_empty()).then(|| codigo.clone()),
                    aplicado_centavos: Some(aplicado),
                    completar: Some(Completar {
                        emissor: Some(emissor.clone()),
                        indexador: indexador.map(str::to_string),
                        taxa: taxa_texto,
                        benchmark: benchmark.map(str::to_string),
                    }),
                    ..Default::default()
                },
            }
        }
        AtivoBtg::Fundo {
            nome: nome_fundo,
            cnpj,
            primeira_compra,
            referencia,
            ..
        } => {
            let aplicado = v.aplicado.unwrap_or_default();
            let normal = normalizar_emissor(nome_fundo);
            Regra {
                ativo: nome,
                chaves,
                tipo_ok: Box::new(|i| i.tipo == "fundo"),
                por_codigo: Box::new(move |i| i.codigo_b3.as_deref() == Some(cnpj.as_str())),
                codigo_gravado: true,
                sem_codigo: Some(Box::new({
                    let normal = normal.clone();
                    move |i| emissor_igual(i, &normal)
                })),
                parecido: Some(Box::new(move |i| emissor_parecido(i, &normal))),
                medida: Medida::Aplicado(aplicado),
                tipo_codigo: "fundo",
                no_arquivo: Some(formatar_reais(aplicado)),
                sugestao: NovoInvestimento {
                    tipo: "fundo".into(),
                    instituicao: INSTITUICAO.into(),
                    emissor: Some(nome_fundo.clone()),
                    data_aplicacao: Some(primeira_compra.to_string()),
                    valor_aplicado_centavos: Some(aplicado),
                    liquidez: liquidez_padrao("fundo"),
                    ir: ir_padrao("fundo"),
                    ..Default::default()
                },
                dados: DadosArquivo {
                    codigo: Some(cnpj.clone()),
                    aplicado_centavos: Some(aplicado),
                    // `valores` já descartou o saldo sem referência (com aviso).
                    saldo: v.saldo.zip(*referencia),
                    ..Default::default()
                },
            }
        }
        AtivoBtg::Cripto {
            nome: nome_cripto, ..
        } => {
            let normal = normalizar_emissor(nome_cripto);
            let codigo = normal.clone();
            let no_arquivo = match (v.aplicado, v.saldo) {
                (Some(a), _) => Some(formatar_reais(a)),
                (None, Some(s)) => Some(format!("{} (saldo)", formatar_reais(s))),
                (None, None) => None,
            };
            Regra {
                ativo: nome,
                chaves,
                tipo_ok: Box::new(|i| i.tipo == "cripto"),
                por_codigo: Box::new(move |i| i.codigo_b3.as_deref() == Some(codigo.as_str())),
                codigo_gravado: true,
                sem_codigo: Some(Box::new({
                    let normal = normal.clone();
                    move |i| emissor_igual(i, &normal)
                })),
                parecido: Some(Box::new({
                    let normal = normal.clone();
                    move |i| emissor_parecido(i, &normal)
                })),
                medida: v.aplicado.map_or(Medida::SemAplicado, Medida::Aplicado),
                tipo_codigo: "cripto",
                no_arquivo,
                sugestao: NovoInvestimento {
                    tipo: "cripto".into(),
                    instituicao: INSTITUICAO.into(),
                    emissor: Some(nome_cripto.clone()),
                    data_aplicacao: Some(data.to_string()),
                    valor_aplicado_centavos: v.aplicado,
                    liquidez: liquidez_padrao("cripto"),
                    ir: ir_padrao("cripto"),
                    ..Default::default()
                },
                dados: DadosArquivo {
                    codigo: Some(normal),
                    aplicado_centavos: v.aplicado,
                    saldo: v.saldo.map(|s| (s, data)),
                    ..Default::default()
                },
            }
        }
    }
}

/// Investimentos que casam com a regra; `mesma` exige a mesma corretora (a BTG). Primeiro pelo
/// código; sem par pelo código, pela regra sem código (só investimentos sem código gravado).
/// Devolve também se os candidatos vieram pelo código gravado.
fn buscar<'i>(ctx: &Contexto<'i>, r: &Regra, mesma: bool) -> (Vec<&'i Investimento>, bool) {
    let chave_btg = chave_instituicao(INSTITUICAO);
    let investimentos: &'i [Investimento] = ctx.investimentos;
    let livre = |i: &&Investimento| {
        (r.tipo_ok)(i)
            && !ctx.usados.contains(&i.id)
            && (!mesma || chave_instituicao(&i.instituicao) == chave_btg)
    };
    let por_codigo: Vec<&Investimento> = investimentos
        .iter()
        .filter(livre)
        .filter(|i| (r.por_codigo)(i))
        .collect();
    if !por_codigo.is_empty() {
        return (por_codigo, r.codigo_gravado);
    }
    let Some(sem_codigo) = &r.sem_codigo else {
        return (Vec::new(), false);
    };
    let candidatos = investimentos
        .iter()
        .filter(livre)
        .filter(|i| i.codigo_b3.is_none() && sem_codigo(i))
        .collect();
    (candidatos, false)
}

/// Investimentos da BTG, sem código gravado, de nome parecido mas não igual ao do ativo.
fn parecidos<'i>(ctx: &Contexto<'i>, r: &Regra) -> Vec<&'i Investimento> {
    let Some(parecido) = &r.parecido else {
        return Vec::new();
    };
    let chave_btg = chave_instituicao(INSTITUICAO);
    ctx.investimentos
        .iter()
        .filter(|i| (r.tipo_ok)(i) && !ctx.usados.contains(&i.id))
        .filter(|i| i.codigo_b3.as_deref().is_none_or(str::is_empty))
        .filter(|i| chave_instituicao(&i.instituicao) == chave_btg && parecido(i))
        .collect()
}

fn comparar_ativo(ctx: &mut Contexto, chave: String, r: Regra) -> Item {
    let (mut pares, _) = buscar(ctx, &r, true);
    if pares.is_empty() {
        pares = ctx.ligados_pelo_codigo(&r.tipo_ok, r.dados.codigo.as_deref(), &r.chaves);
    }
    let instituicao = INSTITUICAO.to_string();
    if pares.is_empty() {
        let (candidatos, por_codigo) = buscar(ctx, &r, false);
        let mut outros = ctx.de_outra_corretora(candidatos, &r.chaves, por_codigo);
        if outros.is_empty() {
            outros = parecidos(ctx, &r);
        }
        let mut item = if outros.is_empty() {
            ctx.item(
                chave,
                Estado::Novo,
                r.ativo,
                instituicao,
                (None, r.no_arquivo),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(r.sugestao),
            )
        } else {
            ctx.outra_corretora(
                chave,
                r.ativo,
                instituicao,
                (None, r.no_arquivo),
                &outros,
                r.sugestao,
            )
        };
        item.dados = r.dados;
        return item;
    }
    let (no_app, igual) = match r.medida {
        Medida::Quantidade(q) => {
            let soma = pares
                .iter()
                .filter_map(|i| i.quantidade)
                .fold(0i64, i64::saturating_add);
            (quantidade_texto(r.tipo_codigo, soma), soma == q)
        }
        Medida::Aplicado(a) => {
            let soma = total_aplicado(&pares);
            (formatar_reais(soma), a.abs_diff(soma) <= 1)
        }
        Medida::SemAplicado => (formatar_reais(total_aplicado(&pares)), pares.len() == 1),
    };
    let mut item = if pares.len() == 1 {
        ctx.marcar(&pares);
        let (estado, acoes) = com_pares(&pares, igual);
        ctx.item(
            chave,
            estado,
            r.ativo,
            instituicao,
            (Some(no_app), r.no_arquivo),
            ids(&pares),
            acoes,
            None,
        )
    } else {
        // Com dois ou mais candidatos nenhum fica casado: o usuário escolhe qual vincular.
        let mut acoes = vec![Acao::Manter];
        acoes.extend(pares.iter().map(|i| Acao::Vincular(i.id)));
        acoes.push(Acao::Adicionar);
        ctx.item(
            chave,
            Estado::Divergente,
            r.ativo,
            instituicao,
            (Some(no_app), r.no_arquivo),
            ids(&pares),
            acoes,
            Some(r.sugestao),
        )
    };
    item.dados = r.dados;
    item
}

/// Compara cada ativo do extrato com a carteira. `hoje` só serve para validar as sugestões.
pub fn comparar_btg(
    db: &Db,
    extrato: &ExtratoBtg,
    hoje: NaiveDate,
) -> Result<Comparacao, CoreError> {
    let investimentos = db.listar_investimentos_brutos()?;
    let mut ctx = Contexto {
        investimentos: &investimentos,
        hoje,
        usados: HashSet::new(),
        ocupadas: HashSet::new(),
    };
    let mut avisos = extrato.avisos.clone();
    let mut ativos: Vec<(usize, &AtivoBtg, Valores)> = Vec::new();
    for (n, ativo) in extrato.ativos.iter().enumerate() {
        match valores(ativo, &mut avisos) {
            Some(v) => ativos.push((n, ativo, v)),
            None => avisos.push(format!("Valor fora do limite em {}.", nome_do_ativo(ativo))),
        }
    }
    let chave_btg = chave_instituicao(INSTITUICAO);
    for (_, ativo, _) in &ativos {
        for chave in chaves_do_ativo(ativo) {
            ctx.ocupadas.insert((chave, chave_btg.clone()));
        }
    }
    let mut itens: Vec<Item> = ativos
        .iter()
        .map(|(n, ativo, v)| {
            comparar_ativo(&mut ctx, format!("btg:{n}"), regra(ativo, v, extrato.data))
        })
        .collect();
    itens.sort_by_key(|i| i.estado.ordem());
    Ok(Comparacao {
        fonte: Fonte::Btg,
        data_arquivo: Some(extrato.data),
        itens,
        avisos,
        linhas_lidas: extrato.linhas_lidas,
    })
}
