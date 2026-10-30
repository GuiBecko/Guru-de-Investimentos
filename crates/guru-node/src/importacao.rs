//! Tipos da ponte para a importação da planilha de posição da B3.

use guru_core::{Acao, Comparacao, CoreError, Decisao, Item, NovoInvestimento};
use napi_derive::napi;

use crate::investimentos::{ErroCampoJs, NovoInvestimentoJs, entrada, erros};

#[napi(object, use_nullable = true)]
pub struct ItemImportacaoJs {
    pub chave: String,
    pub estado: String,
    pub estado_rotulo: String,
    pub ativo: String,
    pub instituicao: String,
    pub no_app: Option<String>,
    pub na_b3: Option<String>,
    pub investimentos: Vec<i64>,
    pub acoes: Vec<String>,
    pub acao_padrao: String,
    pub sugestao: Option<NovoInvestimentoJs>,
    pub faltando: Vec<ErroCampoJs>,
}

#[napi(object, use_nullable = true)]
pub struct ComparacaoJs {
    pub token: String,
    pub arquivo: String,
    pub linhas_lidas: u32,
    pub avisos: Vec<String>,
    pub itens: Vec<ItemImportacaoJs>,
}

/// Entrada: `idVinculo` e `dados` podem faltar.
#[napi(object)]
pub struct DecisaoJs {
    pub chave: String,
    pub acao: String,
    pub id_vinculo: Option<i64>,
    pub dados: Option<NovoInvestimentoJs>,
}

#[napi(object)]
pub struct ErroItemJs {
    pub chave: String,
    pub campos: Vec<ErroCampoJs>,
}

#[napi(object)]
pub struct ResultadoImportacaoJs {
    pub aplicadas: u32,
    pub erros: Vec<ErroItemJs>,
}

/// A última comparação guardada no `Core`, com o token que a identifica e o nome do arquivo.
pub struct ComparacaoGuardada {
    pub token: String,
    pub arquivo: String,
    pub comparacao: Comparacao,
}

pub fn expirada() -> napi::Error<String> {
    napi::Error::new(
        "IMPORTACAO_EXPIRADA".to_string(),
        "Esta comparação não é mais a atual. Escolha o arquivo de novo.".to_string(),
    )
}

fn saida(n: NovoInvestimento) -> NovoInvestimentoJs {
    NovoInvestimentoJs {
        tipo: n.tipo,
        instituicao: n.instituicao,
        emissor: n.emissor,
        ticker: n.ticker,
        indexador: n.indexador,
        taxa: n.taxa,
        benchmark: n.benchmark,
        data_aplicacao: n.data_aplicacao,
        vencimento: n.vencimento,
        valor_aplicado_centavos: n.valor_aplicado_centavos,
        quantidade: n.quantidade,
        preco_medio_centavos: n.preco_medio_centavos,
        liquidez: n.liquidez,
        data_carencia: n.data_carencia,
        ir: n.ir,
        observacao: n.observacao,
        meta_id: n.meta_id,
    }
}

fn item(i: &Item) -> ItemImportacaoJs {
    ItemImportacaoJs {
        chave: i.chave.clone(),
        estado: i.estado.codigo().to_string(),
        estado_rotulo: i.estado.rotulo().to_string(),
        ativo: i.ativo.clone(),
        instituicao: i.instituicao.clone(),
        no_app: i.no_app.clone(),
        na_b3: i.na_b3.clone(),
        investimentos: i.investimentos.clone(),
        acoes: i.acoes.iter().map(|a| a.codigo().to_string()).collect(),
        acao_padrao: i.acao_padrao.codigo().to_string(),
        sugestao: i.sugestao.clone().map(saida),
        faltando: erros(i.faltando.clone()),
    }
}

pub fn comparacao(token: &str, arquivo: &str, c: &Comparacao) -> ComparacaoJs {
    ComparacaoJs {
        token: token.to_string(),
        arquivo: arquivo.to_string(),
        linhas_lidas: u32::try_from(c.linhas_lidas).unwrap_or(u32::MAX),
        avisos: c.avisos.clone(),
        itens: c.itens.iter().map(item).collect(),
    }
}

pub fn decisao(d: DecisaoJs) -> Result<Decisao, CoreError> {
    let invalida = |m: String| CoreError::DecisaoInvalida(m);
    let acao = match d.acao.as_str() {
        "adicionar" => Acao::Adicionar,
        "ignorar" => Acao::Ignorar,
        "usar_b3" => Acao::UsarB3,
        "manter" => Acao::Manter,
        "remover" => Acao::Remover,
        "vincular" => Acao::Vincular(d.id_vinculo.ok_or_else(|| {
            invalida(format!(
                "Vincular exige o investimento (idVinculo) em {}.",
                d.chave
            ))
        })?),
        outra => return Err(invalida(format!("Ação desconhecida: {outra}."))),
    };
    Ok(Decisao {
        chave: d.chave,
        acao,
        dados: d.dados.map(entrada),
    })
}

pub fn resultado(
    aplicadas: usize,
    lista: Vec<(String, Vec<guru_core::ErroCampo>)>,
) -> ResultadoImportacaoJs {
    ResultadoImportacaoJs {
        aplicadas: u32::try_from(aplicadas).unwrap_or(u32::MAX),
        erros: lista
            .into_iter()
            .map(|(chave, campos)| ErroItemJs {
                chave,
                campos: erros(campos),
            })
            .collect(),
    }
}

/// Token novo a cada comparação: contador + relógio, em hexadecimal.
pub fn novo_token() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CONTADOR: AtomicU64 = AtomicU64::new(0);
    let n = CONTADOR.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    format!("{t:x}-{n:x}")
}
