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
    ConversaResumo, CoreError, Db, Fonte, FonteInterna, ParteContexto, ProgressoIa,
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

#[napi(object)]
pub struct ConversaResumoJs {
    pub id: i64,
    pub titulo: String,
    pub atualizada_em: String,
}

#[napi(object, use_nullable = true)]
pub struct MensagemJs {
    pub id: i64,
    pub papel: String,
    pub texto: String,
    pub fontes: Vec<FonteJs>,
    pub aviso: Option<String>,
}

#[napi(object)]
pub struct ConversaJs {
    pub id: i64,
    pub titulo: String,
    pub modelo: String,
    pub fontes_internas: Vec<FonteInternaJs>,
    pub mensagens: Vec<MensagemJs>,
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
    }
}

pub fn conversa(c: Conversa) -> ConversaJs {
    ConversaJs {
        id: c.id,
        titulo: c.titulo,
        modelo: c.modelo,
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

    /// Registra o pedido e roda `perguntar` numa thread própria, com conexão própria: o `Mutex`
    /// do `Core` não fica preso nos até 10 minutos de streaming. Uma thread (e não `AsyncTask`)
    /// porque a `Promise` do `AsyncTask` só resolve no fim, e aqui o retorno é imediato.
    pub fn perguntar(
        &self,
        pasta_dados: PathBuf,
        hoje: NaiveDate,
        conversa_id: Option<i64>,
        pergunta: String,
    ) -> napi::Result<u32, String> {
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
                    guru_core::perguntar(
                        &db,
                        cofre.as_ref(),
                        cliente.as_ref(),
                        hoje,
                        conversa_id,
                        &pergunta,
                        &cancelar,
                        &mut |p| {
                            if let Some(e) = travar(&pedidos).get_mut(&pedido) {
                                e.progresso = p;
                            }
                        },
                    )
                }));
                let resultado: Result<i64, (String, String)> = match r {
                    Ok(Ok(id)) => Ok(id),
                    Ok(Err(e)) => Err((e.code().to_string(), e.to_string())),
                    Err(_) => Err((
                        "ERRO_INTERNO".to_string(),
                        "Ocorreu um erro interno ao falar com a IA.".to_string(),
                    )),
                };
                if let Some(e) = travar(&pedidos).get_mut(&pedido) {
                    match resultado {
                        Ok(id) => e.conversa_id = Some(id),
                        Err(erro) => e.erro = Some(erro),
                    }
                    e.terminado = true;
                }
            });
        if lanc.is_err() {
            travar(&self.pedidos).remove(&pedido);
            return Err(napi::Error::new(
                "ERRO_INTERNO".to_string(),
                "Não foi possível iniciar a pergunta.".to_string(),
            ));
        }
        Ok(pedido)
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

pub fn tarefa_chave(pasta_dados: PathBuf, ia: &Ia, chave: String) -> AsyncTask<SalvarChave> {
    AsyncTask::new(SalvarChave {
        pasta_dados,
        cofre: ia.cofre.clone(),
        cliente: ia.cliente.clone(),
        chave: Some(chave),
    })
}
