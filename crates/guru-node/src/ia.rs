//! Ponte do assistente de IA: tipos JS, pedidos em andamento e a tarefa assíncrona da chave.
//!
//! A chave nunca sai daqui: `configIa` devolve só a máscara (`sk-ant-…` + 4 últimos caracteres).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::NaiveDate;
use guru_core::{
    ClienteFixture, ClienteHttp, ClienteIa, Cofre, CofreMemoria, CofreSistema, Conversa,
    ConversaResumo, CoreError, Db, FocoAporte, Fonte, FonteInterna, Gerados, Insight, Noticia,
    PainelIa, ParteContexto, ProgressoIa, rotulo_insight,
};
use napi::JsValue;
use napi::bindgen_prelude::{AsyncTask, Env, Task};
use napi_derive::napi;

use crate::investimentos::{ErroCampoJs, erros};

#[napi(object)]
pub struct ModeloJs {
    pub codigo: String,
    pub rotulo: String,
}

#[napi(object)]
pub struct UsoIaJs {
    pub tokens: i64,
    pub custo_centesimos_dolar: i64,
}

#[napi(object, use_nullable = true)]
pub struct ConfigIaJs {
    pub modelo: String,
    pub modelos: Vec<ModeloJs>,
    pub instrucoes: String,
    pub apenas_percentuais: bool,
    pub nomes_instituicoes: bool,
    pub busca_web: bool,
    /// Mascarada: `sk-ant-…` + 4 últimos caracteres, ou `null` sem chave.
    pub chave: Option<String>,
    pub uso: UsoIaJs,
}

#[napi(object)]
pub struct ConfigIaEntradaJs {
    pub modelo: String,
    pub instrucoes: String,
    pub apenas_percentuais: bool,
    pub nomes_instituicoes: bool,
    pub busca_web: bool,
}

#[napi(object)]
pub struct ConfigSalvaJs {
    pub erros: Vec<ErroCampoJs>,
    /// Carteira e metas editadas voltaram ao automático porque a privacidade ficou mais restrita.
    pub edicoes_restauradas: bool,
}

#[napi(object, use_nullable = true)]
pub struct ParteJs {
    pub parte: String,
    pub rotulo: String,
    pub ligada: bool,
    pub editado: bool,
    pub texto: String,
    pub resumo: String,
    pub tokens: u32,
    pub data_dados: Option<String>,
}

#[napi(object)]
pub struct ContextoIaJs {
    pub partes: Vec<ParteJs>,
    pub tokens: u32,
}

#[napi(object, use_nullable = true)]
pub struct FonteJs {
    pub numero: u32,
    pub url: String,
    pub titulo: String,
    pub dominio: String,
    pub data: Option<String>,
    pub trecho: String,
}

#[napi(object, use_nullable = true)]
pub struct FonteInternaJs {
    pub parte: String,
    pub rotulo: String,
    pub data_dados: Option<String>,
    pub antigo: bool,
}

#[napi(object, use_nullable = true)]
pub struct ConversaResumoJs {
    pub id: i64,
    pub titulo: String,
    pub atualizada_em: String,
    /// Código da classe do chat do Próximo aporte; `null` nas conversas do Assistente.
    pub foco: Option<String>,
    pub foco_rotulo: Option<String>,
}

#[napi(object, use_nullable = true)]
pub struct MensagemJs {
    pub id: i64,
    pub papel: String,
    pub texto: String,
    pub fontes: Vec<FonteJs>,
    pub aviso: Option<String>,
}

#[napi(object, use_nullable = true)]
pub struct ConversaJs {
    pub id: i64,
    pub titulo: String,
    pub modelo: String,
    pub foco: Option<String>,
    pub foco_rotulo: Option<String>,
    pub fontes_internas: Vec<FonteInternaJs>,
    pub mensagens: Vec<MensagemJs>,
}

/// O que a tela do Próximo aporte manda para o chat de uma classe.
#[napi(object, use_nullable = true)]
pub struct FocoAporteJs {
    pub classe: String,
    pub valor_centavos: i64,
    pub direcoes: Vec<String>,
    pub direcao_livre: String,
    pub meta_id: Option<i64>,
}

impl From<FocoAporteJs> for FocoAporte {
    fn from(f: FocoAporteJs) -> FocoAporte {
        FocoAporte {
            classe: f.classe,
            valor_centavos: f.valor_centavos,
            direcoes: f.direcoes,
            direcao_livre: f.direcao_livre,
            meta_id: f.meta_id,
        }
    }
}

#[napi(object, use_nullable = true)]
pub struct InsightJs {
    pub tipo: String,
    pub rotulo: String,
    pub texto: String,
    pub acao: String,
    pub classe: Option<String>,
    pub classe_rotulo: Option<String>,
}

#[napi(object, use_nullable = true)]
pub struct NoticiaJs {
    pub titulo: String,
    pub url: String,
    pub dominio: String,
    pub data: Option<String>,
    pub etiqueta: String,
}

#[napi(object)]
pub struct InsightsGeradosJs {
    /// `aaaa-mm-dd`.
    pub gerado_em: String,
    pub modelo: String,
    pub itens: Vec<InsightJs>,
}

#[napi(object)]
pub struct NoticiasGeradasJs {
    pub gerado_em: String,
    pub modelo: String,
    pub itens: Vec<NoticiaJs>,
}

#[napi(object, use_nullable = true)]
pub struct PainelIaJs {
    pub busca_web: bool,
    pub insights: Option<InsightsGeradosJs>,
    pub noticias: Option<NoticiasGeradasJs>,
}

fn insight(i: Insight) -> InsightJs {
    let classe_rotulo = i
        .classe
        .as_deref()
        .and_then(guru_core::Classe::de_codigo)
        .map(|c| c.rotulo().to_string());
    InsightJs {
        rotulo: rotulo_insight(&i.tipo).to_string(),
        tipo: i.tipo,
        texto: i.texto,
        acao: i.acao,
        classe: i.classe,
        classe_rotulo,
    }
}

pub fn painel(busca_web: bool, p: PainelIa) -> PainelIaJs {
    let PainelIa { insights, noticias } = p;
    PainelIaJs {
        busca_web,
        insights: insights.map(|g: Gerados<Insight>| InsightsGeradosJs {
            gerado_em: g.gerado_em.to_string(),
            modelo: g.modelo,
            itens: g.itens.into_iter().map(insight).collect(),
        }),
        noticias: noticias.map(|g: Gerados<Noticia>| NoticiasGeradasJs {
            gerado_em: g.gerado_em.to_string(),
            modelo: g.modelo,
            itens: g
                .itens
                .into_iter()
                .map(|n| NoticiaJs {
                    titulo: n.titulo,
                    url: n.url,
                    dominio: n.dominio,
                    data: n.data,
                    etiqueta: n.etiqueta,
                })
                .collect(),
        }),
    }
}

#[napi(object)]
pub struct PedidoJs {
    pub pedido: u32,
}

#[napi(object)]
pub struct ErroPedidoJs {
    pub code: String,
    pub message: String,
}

#[napi(object, use_nullable = true)]
pub struct ProgressoIaJs {
    pub texto: String,
    pub fontes: Vec<FonteJs>,
    pub buscando: bool,
    pub terminado: bool,
    pub conversa_id: Option<i64>,
    pub erro: Option<ErroPedidoJs>,
}

pub fn mascarar(chave_final: Option<String>) -> Option<String> {
    chave_final.map(|f| format!("sk-ant-…{f}"))
}

pub fn parte(p: ParteContexto) -> ParteJs {
    ParteJs {
        parte: p.parte.to_string(),
        rotulo: p.rotulo.to_string(),
        ligada: p.ligada,
        editado: p.editado,
        texto: p.texto,
        resumo: p.resumo,
        tokens: p.tokens,
        data_dados: p.data_dados.map(|d| d.to_string()),
    }
}

fn fonte(f: Fonte) -> FonteJs {
    FonteJs {
        numero: f.numero,
        url: f.url,
        titulo: f.titulo,
        dominio: f.dominio,
        data: f.data,
        trecho: f.trecho,
    }
}

fn fonte_interna(f: FonteInterna) -> FonteInternaJs {
    FonteInternaJs {
        parte: f.parte,
        rotulo: f.rotulo,
        data_dados: f.data_dados.map(|d| d.to_string()),
        antigo: f.antigo,
    }
}

pub fn resumo(c: ConversaResumo) -> ConversaResumoJs {
    ConversaResumoJs {
        id: c.id,
        titulo: c.titulo,
        atualizada_em: c.atualizada_em,
        foco: c.foco.map(|f| f.codigo().to_string()),
        foco_rotulo: c.foco.map(|f| f.rotulo().to_string()),
    }
}

pub fn conversa(c: Conversa) -> ConversaJs {
    ConversaJs {
        id: c.id,
        titulo: c.titulo,
        modelo: c.modelo,
        foco: c.foco.map(|f| f.codigo().to_string()),
        foco_rotulo: c.foco.map(|f| f.rotulo().to_string()),
        fontes_internas: c.fontes_internas.into_iter().map(fonte_interna).collect(),
        mensagens: c
            .mensagens
            .into_iter()
            .map(|m| MensagemJs {
                id: m.id,
                papel: m.papel,
                texto: m.texto,
                fontes: m.fontes.into_iter().map(fonte).collect(),
                aviso: m.aviso,
            })
            .collect(),
    }
}

/// Estado de um pedido em andamento (ou recém-terminado e ainda não lido).
struct EstadoPedido {
    progresso: ProgressoIa,
    cancelar: Arc<AtomicBool>,
    terminado: bool,
    conversa_id: Option<i64>,
    erro: Option<(String, String)>,
}

type Tarefa = Box<
    dyn FnOnce(
            &Db,
            &dyn Cofre,
            &dyn ClienteIa,
            &AtomicBool,
            &mut dyn FnMut(ProgressoIa),
        ) -> Result<Option<i64>, CoreError>
        + Send,
>;

type Pedidos = Arc<Mutex<HashMap<u32, EstadoPedido>>>;

fn travar(p: &Pedidos) -> MutexGuard<'_, HashMap<u32, EstadoPedido>> {
    p.lock().unwrap_or_else(|e| e.into_inner())
}

/// Cofre, cliente e pedidos da IA, guardados por `Core`.
pub struct Ia {
    pub cofre: Arc<dyn Cofre>,
    pub cliente: Arc<dyn ClienteIa>,
    pedidos: Pedidos,
    proximo: AtomicU32,
}

impl Ia {
    /// Só os testes (`pasta_ia`, `cofre_memoria`) trocam o cliente e o cofre; o main os repassa
    /// apenas fora do app empacotado.
    pub fn novo(pasta_ia: Option<String>, cofre_memoria: bool) -> Ia {
        let cofre: Arc<dyn Cofre> = if cofre_memoria {
            Arc::new(CofreMemoria::default())
        } else {
            Arc::new(CofreSistema)
        };
        let cliente: Arc<dyn ClienteIa> = match pasta_ia {
            Some(pasta) => Arc::new(ClienteFixture {
                pasta: PathBuf::from(pasta),
            }),
            None => Arc::new(ClienteHttp::novo()),
        };
        Ia {
            cofre,
            cliente,
            pedidos: Arc::new(Mutex::new(HashMap::new())),
            proximo: AtomicU32::new(1),
        }
    }

    /// Registra o pedido e roda a tarefa numa thread própria, com conexão própria: o `Mutex` do
    /// `Core` não fica preso nos até 10 minutos de streaming. Uma thread (e não `AsyncTask`)
    /// porque a `Promise` do `AsyncTask` só resolve no fim, e aqui o retorno é imediato. A tarefa
    /// devolve o id da conversa, ou `None` quando não há (insights e notícias).
    pub fn lancar(&self, pasta_dados: PathBuf, tarefa: Tarefa) -> napi::Result<u32, String> {
        let pedido = self.proximo.fetch_add(1, Ordering::SeqCst);
        let cancelar = Arc::new(AtomicBool::new(false));
        travar(&self.pedidos).insert(
            pedido,
            EstadoPedido {
                progresso: ProgressoIa::default(),
                cancelar: cancelar.clone(),
                terminado: false,
                conversa_id: None,
                erro: None,
            },
        );
        let pedidos = self.pedidos.clone();
        let cofre = self.cofre.clone();
        let cliente = self.cliente.clone();
        let lanc = std::thread::Builder::new()
            .name("guru-ia".into())
            .spawn(move || {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let db = Db::open(&pasta_dados)?;
                    tarefa(&db, cofre.as_ref(), cliente.as_ref(), &cancelar, &mut |p| {
                        if let Some(e) = travar(&pedidos).get_mut(&pedido) {
                            e.progresso = p;
                        }
                    })
                }));
                let resultado: Result<Option<i64>, (String, String)> = match r {
                    Ok(Ok(id)) => Ok(id),
                    Ok(Err(e)) => Err((e.code().to_string(), e.to_string())),
                    Err(_) => Err((
                        "ERRO_INTERNO".to_string(),
                        "Ocorreu um erro interno ao falar com a IA.".to_string(),
                    )),
                };
                if let Some(e) = travar(&pedidos).get_mut(&pedido) {
                    match resultado {
                        Ok(id) => e.conversa_id = id,
                        Err(erro) => e.erro = Some(erro),
                    }
                    e.terminado = true;
                }
            });
        if lanc.is_err() {
            travar(&self.pedidos).remove(&pedido);
            return Err(napi::Error::new(
                "ERRO_INTERNO".to_string(),
                "Não foi possível iniciar o pedido à IA.".to_string(),
            ));
        }
        Ok(pedido)
    }

    pub fn perguntar(
        &self,
        pasta_dados: PathBuf,
        hoje: NaiveDate,
        conversa_id: Option<i64>,
        pergunta: String,
        foco: Option<FocoAporteJs>,
    ) -> napi::Result<u32, String> {
        self.lancar(
            pasta_dados,
            Box::new(move |db, cofre, cliente, cancelar, progresso| {
                // O foco só vale para a primeira pergunta; com conversa, é ignorado.
                match (conversa_id, foco) {
                    (None, Some(f)) => guru_core::perguntar_aporte(
                        db,
                        cofre,
                        cliente,
                        hoje,
                        &f.into(),
                        &pergunta,
                        cancelar,
                        progresso,
                    ),
                    _ => guru_core::perguntar(
                        db,
                        cofre,
                        cliente,
                        hoje,
                        conversa_id,
                        &pergunta,
                        cancelar,
                        progresso,
                    ),
                }
                .map(Some)
            }),
        )
    }

    pub fn gerar_insights(
        &self,
        pasta_dados: PathBuf,
        hoje: NaiveDate,
    ) -> napi::Result<u32, String> {
        self.lancar(
            pasta_dados,
            Box::new(move |db, cofre, cliente, cancelar, progresso| {
                guru_core::gerar_insights(db, cofre, cliente, hoje, cancelar, progresso)
                    .map(|()| None)
            }),
        )
    }

    pub fn buscar_noticias(
        &self,
        pasta_dados: PathBuf,
        hoje: NaiveDate,
    ) -> napi::Result<u32, String> {
        self.lancar(
            pasta_dados,
            Box::new(move |db, cofre, cliente, cancelar, progresso| {
                guru_core::buscar_noticias(db, cofre, cliente, hoje, cancelar, progresso)
                    .map(|()| None)
            }),
        )
    }

    /// O retrato do pedido; um terminado sai do mapa na primeira leitura que o mostra assim.
    pub fn progresso(&self, pedido: u32) -> napi::Result<ProgressoIaJs, String> {
        let mut mapa = travar(&self.pedidos);
        let e = mapa.get(&pedido).ok_or_else(|| {
            napi::Error::new(
                "NAO_ENCONTRADO".to_string(),
                "Pedido não encontrado.".to_string(),
            )
        })?;
        let js = ProgressoIaJs {
            texto: e.progresso.texto.clone(),
            fontes: e.progresso.fontes.iter().cloned().map(fonte).collect(),
            buscando: e.progresso.buscando && !e.terminado,
            terminado: e.terminado,
            conversa_id: e.conversa_id,
            erro: e.erro.as_ref().map(|(code, message)| ErroPedidoJs {
                code: code.clone(),
                message: message.clone(),
            }),
        };
        if e.terminado {
            mapa.remove(&pedido);
        }
        Ok(js)
    }

    /// Pedido desconhecido ou já terminado: nada a fazer.
    pub fn cancelar(&self, pedido: u32) {
        if let Some(e) = travar(&self.pedidos).get(&pedido) {
            e.cancelar.store(true, Ordering::SeqCst);
        }
    }
}

/// Valida e grava a chave (chamada de rede) na threadpool, com conexão própria.
pub struct SalvarChave {
    pub pasta_dados: PathBuf,
    pub cofre: Arc<dyn Cofre>,
    pub cliente: Arc<dyn ClienteIa>,
    pub chave: Option<String>,
}

impl Task for SalvarChave {
    type Output = Result<Vec<guru_core::ErroCampo>, (String, String)>;
    type JsValue = Vec<ErroCampoJs>;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let chave = self.chave.take().unwrap_or_default();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let db = Db::open(&self.pasta_dados)?;
            guru_core::salvar_chave(&db, self.cofre.as_ref(), self.cliente.as_ref(), &chave)
        }));
        Ok(match r {
            Ok(r) => r.map_err(|e: CoreError| (e.code().to_string(), e.to_string())),
            Err(_) => Err((
                "ERRO_INTERNO".to_string(),
                "Ocorreu um erro interno ao salvar a chave.".to_string(),
            )),
        })
    }

    // Mesma rejeição com `code` de `AtualizarMercado::resolve`.
    fn resolve(&mut self, env: Env, saida: Self::Output) -> napi::Result<Self::JsValue> {
        match saida {
            Ok(lista) => Ok(erros(lista)),
            Err((code, mensagem)) => {
                let mut erro =
                    env.create_error(napi::Error::new(napi::Status::GenericFailure, mensagem))?;
                erro.set("code", code)?;
                Err(napi::Error::from(erro.to_unknown()))
            }
        }
    }
}

/// Apaga a chave do cofre do sistema (que pode demorar) na threadpool, com conexão própria.
pub struct RemoverChave {
    pub pasta_dados: PathBuf,
    pub cofre: Arc<dyn Cofre>,
}

impl Task for RemoverChave {
    type Output = Result<(), (String, String)>;
    type JsValue = ();

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let db = Db::open(&self.pasta_dados)?;
            guru_core::remover_chave(&db, self.cofre.as_ref())
        }));
        Ok(match r {
            Ok(r) => r.map_err(|e: CoreError| (e.code().to_string(), e.to_string())),
            Err(_) => Err((
                "ERRO_INTERNO".to_string(),
                "Ocorreu um erro interno ao remover a chave.".to_string(),
            )),
        })
    }

    // Mesma rejeição com `code` de `SalvarChave::resolve`.
    fn resolve(&mut self, env: Env, saida: Self::Output) -> napi::Result<Self::JsValue> {
        match saida {
            Ok(()) => Ok(()),
            Err((code, mensagem)) => {
                let mut erro =
                    env.create_error(napi::Error::new(napi::Status::GenericFailure, mensagem))?;
                erro.set("code", code)?;
                Err(napi::Error::from(erro.to_unknown()))
            }
        }
    }
}

pub fn tarefa_remover_chave(pasta_dados: PathBuf, ia: &Ia) -> AsyncTask<RemoverChave> {
    AsyncTask::new(RemoverChave {
        pasta_dados,
        cofre: ia.cofre.clone(),
    })
}

pub fn tarefa_chave(pasta_dados: PathBuf, ia: &Ia, chave: String) -> AsyncTask<SalvarChave> {
    AsyncTask::new(SalvarChave {
        pasta_dados,
        cofre: ia.cofre.clone(),
        cliente: ia.cliente.clone(),
        chave: Some(chave),
    })
}
