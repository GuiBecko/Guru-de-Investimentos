//! Tipos que atravessam a ponte e conversões de/para o núcleo.

use guru_core::{
    Benchmark, Classe, ErroCampo, Garantia, INSTITUICOES, Indexador, Investimento, Ir, Liquidez,
    NovoInvestimento, Resumo, TipoInfo, tipos,
};
use napi_derive::napi;

#[napi(object)]
pub struct Opcao {
    pub codigo: String,
    pub rotulo: String,
}

#[napi(object)]
pub struct TipoCatalogo {
    pub codigo: String,
    pub rotulo: String,
    pub grupo: String,
    pub classe: String,
    pub forma: String,
    pub indexador_forcado: Option<String>,
    pub liquidez_forcada: Option<String>,
    pub ir_padrao: Option<String>,
    pub ir_editavel: bool,
    pub garantia: Option<String>,
}

#[napi(object)]
pub struct Catalogo {
    pub tipos: Vec<TipoCatalogo>,
    pub classes: Vec<Opcao>,
    pub indexadores: Vec<Opcao>,
    pub benchmarks: Vec<Opcao>,
    pub liquidezes: Vec<Opcao>,
    pub irs: Vec<Opcao>,
    pub garantias: Vec<Opcao>,
    pub instituicoes: Vec<String>,
}

#[napi(object)]
pub struct NovoInvestimentoJs {
    pub tipo: String,
    pub instituicao: String,
    pub emissor: Option<String>,
    pub ticker: Option<String>,
    pub indexador: Option<String>,
    pub taxa: Option<String>,
    pub benchmark: Option<String>,
    pub data_aplicacao: Option<String>,
    pub vencimento: Option<String>,
    pub valor_aplicado_centavos: Option<i64>,
    pub quantidade: Option<i64>,
    pub preco_medio_centavos: Option<i64>,
    pub liquidez: Option<String>,
    pub data_carencia: Option<String>,
    pub ir: Option<String>,
    pub observacao: Option<String>,
}

#[napi(object)]
pub struct ErroCampoJs {
    pub campo: String,
    pub mensagem: String,
}

#[napi(object)]
pub struct ResultadoGravacao {
    pub id: Option<i64>,
    pub erros: Vec<ErroCampoJs>,
}

#[napi(object)]
pub struct InvestimentoJs {
    pub id: i64,
    pub tipo: String,
    pub tipo_rotulo: String,
    pub classe: String,
    pub classe_rotulo: String,
    pub nome: String,
    pub instituicao: String,
    pub emissor: Option<String>,
    pub ticker: Option<String>,
    pub indexador: Option<String>,
    pub indexador_rotulo: Option<String>,
    pub taxa: Option<String>,
    pub benchmark: Option<String>,
    pub rendimento: Option<String>,
    pub data_aplicacao: String,
    pub vencimento: Option<String>,
    pub valor_aplicado_centavos: i64,
    pub quantidade: Option<i64>,
    pub preco_medio_centavos: Option<i64>,
    pub liquidez: Option<String>,
    pub liquidez_rotulo: Option<String>,
    pub data_carencia: Option<String>,
    pub ir: Option<String>,
    pub ir_rotulo: Option<String>,
    pub garantia: Option<String>,
    pub garantia_rotulo: Option<String>,
    pub observacao: Option<String>,
}

#[napi(object)]
pub struct ResumoClasseJs {
    pub classe: String,
    pub rotulo: String,
    pub total_aplicado_centavos: i64,
    pub quantidade: i64,
}

#[napi(object)]
pub struct ResumoJs {
    pub total_aplicado_centavos: i64,
    pub classes: Vec<ResumoClasseJs>,
}

macro_rules! opcoes {
    ($($valor:expr),+ $(,)?) => {
        vec![$(Opcao {
            codigo: $valor.codigo().to_string(),
            rotulo: $valor.rotulo().to_string(),
        }),+]
    };
}

fn tipo_catalogo(t: &TipoInfo) -> TipoCatalogo {
    TipoCatalogo {
        codigo: t.codigo.to_string(),
        rotulo: t.rotulo.to_string(),
        grupo: t.grupo.to_string(),
        classe: t.classe.codigo().to_string(),
        forma: t.forma.codigo().to_string(),
        indexador_forcado: t.indexador_forcado.map(|v| v.codigo().to_string()),
        liquidez_forcada: t.liquidez_forcada.map(|v| v.codigo().to_string()),
        ir_padrao: t.ir_padrao.map(|v| v.codigo().to_string()),
        ir_editavel: t.ir_editavel,
        garantia: t.garantia.map(|v| v.codigo().to_string()),
    }
}

pub fn catalogo() -> Catalogo {
    Catalogo {
        tipos: tipos().iter().map(tipo_catalogo).collect(),
        classes: opcoes![
            Classe::Rf,
            Classe::Acoes,
            Classe::Fii,
            Classe::Ext,
            Classe::Outros
        ],
        indexadores: opcoes![
            Indexador::Pos,
            Indexador::Pre,
            Indexador::Ipca,
            Indexador::Selic
        ],
        benchmarks: opcoes![Benchmark::Cdi, Benchmark::Selic],
        liquidezes: opcoes![
            Liquidez::Diaria,
            Liquidez::D1,
            Liquidez::D30,
            Liquidez::Vencimento,
            Liquidez::Carencia
        ],
        irs: opcoes![Ir::Regressiva, Ir::Isento],
        garantias: opcoes![Garantia::Fgc, Garantia::Tesouro, Garantia::Nenhuma],
        instituicoes: INSTITUICOES.iter().map(|s| s.to_string()).collect(),
    }
}

pub fn entrada(j: NovoInvestimentoJs) -> NovoInvestimento {
    NovoInvestimento {
        tipo: j.tipo,
        instituicao: j.instituicao,
        emissor: j.emissor,
        ticker: j.ticker,
        indexador: j.indexador,
        taxa: j.taxa,
        benchmark: j.benchmark,
        data_aplicacao: j.data_aplicacao,
        vencimento: j.vencimento,
        valor_aplicado_centavos: j.valor_aplicado_centavos,
        quantidade: j.quantidade,
        preco_medio_centavos: j.preco_medio_centavos,
        liquidez: j.liquidez,
        data_carencia: j.data_carencia,
        ir: j.ir,
        observacao: j.observacao,
    }
}

pub fn erros(lista: Vec<ErroCampo>) -> Vec<ErroCampoJs> {
    lista
        .into_iter()
        .map(|e| ErroCampoJs {
            campo: e.campo,
            mensagem: e.mensagem,
        })
        .collect()
}

pub fn investimento(i: Investimento) -> InvestimentoJs {
    InvestimentoJs {
        id: i.id,
        tipo: i.tipo,
        tipo_rotulo: i.tipo_rotulo,
        classe: i.classe.codigo().to_string(),
        classe_rotulo: i.classe.rotulo().to_string(),
        nome: i.nome,
        instituicao: i.instituicao,
        emissor: i.emissor,
        ticker: i.ticker,
        indexador: i.indexador.map(|v| v.codigo().to_string()),
        indexador_rotulo: i.indexador.map(|v| v.rotulo().to_string()),
        taxa: i.taxa,
        benchmark: i.benchmark.map(|v| v.codigo().to_string()),
        rendimento: i.rendimento,
        data_aplicacao: i.data_aplicacao,
        vencimento: i.vencimento,
        valor_aplicado_centavos: i.valor_aplicado_centavos,
        quantidade: i.quantidade,
        preco_medio_centavos: i.preco_medio_centavos,
        liquidez: i.liquidez.map(|v| v.codigo().to_string()),
        liquidez_rotulo: i.liquidez.map(|v| v.rotulo().to_string()),
        data_carencia: i.data_carencia,
        ir: i.ir.map(|v| v.codigo().to_string()),
        ir_rotulo: i.ir.map(|v| v.rotulo().to_string()),
        garantia: i.garantia.map(|v| v.codigo().to_string()),
        garantia_rotulo: i.garantia.map(|v| v.rotulo().to_string()),
        observacao: i.observacao,
    }
}

pub fn resumo(r: Resumo) -> ResumoJs {
    ResumoJs {
        total_aplicado_centavos: r.total_aplicado_centavos,
        classes: r
            .classes
            .into_iter()
            .map(|c| ResumoClasseJs {
                classe: c.classe.codigo().to_string(),
                rotulo: c.classe.rotulo().to_string(),
                total_aplicado_centavos: c.total_aplicado_centavos,
                quantidade: c.quantidade,
            })
            .collect(),
    }
}
