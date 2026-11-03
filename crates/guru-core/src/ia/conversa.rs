//! Conversas gravadas, uma pergunta inteira de ponta a ponta e a chave.

use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{NaiveDate, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::{Classe, ErroCampo};

use super::aporte::FocoAporte;
use super::cliente::{ClienteIa, ErroIa, corpo_pedido};
use super::cofre::{Cofre, validar_formato_chave};
use super::contexto::FonteInterna;
use super::sse::{Acumulador, Fonte, Progresso, Uso};

const AVISO_RECUSA: &str = "A IA preferiu não responder a esta pergunta.";
const MAX_PAUSAS: u32 = 3;
const TAM_TITULO: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversaResumo {
    pub id: i64,
    pub titulo: String,
    pub atualizada_em: String,
    /// `None`: conversa do Assistente; a classe: chat do Próximo aporte.
    pub foco: Option<Classe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MensagemGravada {
    pub id: i64,
    pub papel: String,
    pub texto: String,
    pub fontes: Vec<Fonte>,
    pub aviso: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversa {
    pub id: i64,
    pub titulo: String,
    pub modelo: String,
    pub foco: Option<Classe>,
    pub fontes_internas: Vec<FonteInterna>,
    pub mensagens: Vec<MensagemGravada>,
}

/// Tokens do mês somados entre os modelos e o custo estimado, sem contar buscas na web.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsoMes {
    pub entrada: i64,
    pub saida: i64,
    pub cache_leitura: i64,
    pub cache_escrita: i64,
    pub tokens: i64,
    pub custo_centesimos_dolar: i64,
}

/// Preço por milhão de tokens em décimos de centavo de dólar:
/// (entrada, saída, leitura de cache, escrita de cache = 1,25 × entrada).
fn precos(modelo: &str) -> (i128, i128, i128, i128) {
    match modelo {
        "claude-sonnet-5-5" => (2000, 10_000, 200, 2500),
        "claude-haiku-5-5" => (100, 500, 10, 125),
        // Opus e qualquer modelo desconhecido: o mais caro, para não subestimar.
        _ => (4000, 20_000, 200, 5000),
    }
}

fn agora() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn mes(hoje: NaiveDate) -> String {
    hoje.format("%Y-%m").to_string()
}

fn fontes_internas_json(fontes: &[FonteInterna]) -> String {
    let lista: Vec<Value> = fontes
        .iter()
        .map(|f| {
            json!({
                "parte": f.parte,
                "rotulo": f.rotulo,
                "data_dados": f.data_dados.map(|d| d.format("%Y-%m-%d").to_string()),
                "antigo": f.antigo,
            })
        })
        .collect();
    Value::Array(lista).to_string()
}

fn ler_fontes_internas(texto: &str) -> Vec<FonteInterna> {
    let v: Value = serde_json::from_str(texto).unwrap_or(Value::Null);
    v.as_array()
        .map(|l| {
            l.iter()
                .map(|f| FonteInterna {
                    parte: f["parte"].as_str().unwrap_or("").to_string(),
                    rotulo: f["rotulo"].as_str().unwrap_or("").to_string(),
                    data_dados: f["data_dados"]
                        .as_str()
                        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
                    antigo: f["antigo"].as_bool().unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn ler_foco(codigo: Option<String>) -> Option<Classe> {
    codigo.as_deref().and_then(Classe::de_codigo)
}

/// Soma os tokens de uma resposta no mês e no modelo. Serve à transação de `gravar_troca` e às
/// respostas perdidas depois do `message_start`.
pub(crate) fn somar_uso(
    conn: &rusqlite::Connection,
    mes: &str,
    modelo: &str,
    uso: Uso,
) -> Result<(), CoreError> {
    conn.execute(
        "INSERT INTO ia_uso (mes, modelo, entrada, saida, cache_leitura, cache_escrita) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (mes, modelo) DO UPDATE SET \
         entrada = entrada + excluded.entrada, saida = saida + excluded.saida, \
         cache_leitura = cache_leitura + excluded.cache_leitura, \
         cache_escrita = cache_escrita + excluded.cache_escrita",
        params![
            mes,
            modelo,
            uso.entrada,
            uso.saida,
            uso.cache_leitura,
            uso.cache_escrita
        ],
    )
    .map_err(classifica)?;
    Ok(())
}

/// Os 60 primeiros caracteres da pergunta, numa linha só.
fn titulo(pergunta: &str) -> String {
    pergunta
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(TAM_TITULO)
        .collect()
}

/// O que a pergunta precisa de uma conversa: o contexto congelado, o modelo, a busca e o histórico.
struct DadosConversa {
    contexto: String,
    modelo: String,
    busca_web: bool,
    historico: Vec<Value>,
}

/// A linha da conversa que nasce com a primeira pergunta.
struct Nova {
    titulo: String,
    fontes_internas: String,
    foco: Option<&'static str>,
}

/// Para onde vai a pergunta.
pub(crate) enum Destino<'a> {
    Existente(i64),
    Assistente,
    Aporte(&'a FocoAporte),
}

/// O que vai ser gravado no fim, de uma vez.
struct Troca<'a> {
    nova: Option<Nova>,
    conversa_id: Option<i64>,
    dados: &'a DadosConversa,
    pergunta: &'a Value,
    pergunta_texto: &'a str,
    resposta: &'a Value,
    resposta_texto: &'a str,
    fontes: &'a [Fonte],
    aviso: Option<&'a str>,
    uso: Uso,
    mes: String,
}

impl Db {
    /// Mais recente primeiro.
    pub fn listar_conversas(&self) -> Result<Vec<ConversaResumo>, CoreError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT id, titulo, atualizada_em, foco FROM conversa \
                 ORDER BY atualizada_em DESC, id DESC",
            )
            .map_err(classifica)?;
        st.query_map([], |r| {
            Ok(ConversaResumo {
                id: r.get(0)?,
                titulo: r.get(1)?,
                atualizada_em: r.get(2)?,
                foco: ler_foco(r.get(3)?),
            })
        })
        .map_err(classifica)?
        .collect::<Result<_, _>>()
        .map_err(classifica)
    }

    pub fn conversa(&self, id: i64) -> Result<Conversa, CoreError> {
        let (titulo, modelo, fontes, foco): (String, String, String, Option<String>) = self
            .conn
            .query_row(
                "SELECT titulo, modelo, fontes_internas, foco FROM conversa WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()
            .map_err(classifica)?
            .ok_or(CoreError::ConversaNaoEncontrada(id))?;
        let mut st = self
            .conn
            .prepare(
                "SELECT id, papel, texto, fontes_json, aviso FROM mensagem \
                 WHERE conversa_id = ?1 ORDER BY id",
            )
            .map_err(classifica)?;
        let mensagens = st
            .query_map([id], |r| {
                let fontes: String = r.get(3)?;
                Ok(MensagemGravada {
                    id: r.get(0)?,
                    papel: r.get(1)?,
                    texto: r.get(2)?,
                    fontes: serde_json::from_str(&fontes).unwrap_or_default(),
                    aviso: r.get(4)?,
                })
            })
            .map_err(classifica)?
            .collect::<Result<_, _>>()
            .map_err(classifica)?;
        Ok(Conversa {
            id,
            titulo,
            modelo,
            foco: ler_foco(foco),
            fontes_internas: ler_fontes_internas(&fontes),
            mensagens,
        })
    }

    /// As mensagens vão junto (`ON DELETE CASCADE`). Apagar o que já não existe não é erro.
    pub fn apagar_conversa(&self, id: i64) -> Result<(), CoreError> {
        self.conn
            .execute("DELETE FROM conversa WHERE id = ?1", [id])
            .map_err(classifica)?;
        Ok(())
    }

    /// Tokens somados no mês de `hoje` e o custo em centésimos de dólar, arredondado.
    pub fn uso_do_mes(&self, hoje: NaiveDate) -> Result<UsoMes, CoreError> {
        let mut st = self
            .conn
            .prepare(
                "SELECT modelo, entrada, saida, cache_leitura, cache_escrita \
                 FROM ia_uso WHERE mes = ?1",
            )
            .map_err(classifica)?;
        let linhas = st
            .query_map([mes(hoje)], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .map_err(classifica)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(classifica)?;
        let mut uso = UsoMes::default();
        let mut custo: i128 = 0;
        for (modelo, entrada, saida, leitura, escrita) in linhas {
            uso.entrada += entrada;
            uso.saida += saida;
            uso.cache_leitura += leitura;
            uso.cache_escrita += escrita;
            let (pe, ps, pl, pw) = precos(&modelo);
            custo += i128::from(entrada) * pe
                + i128::from(saida) * ps
                + i128::from(leitura) * pl
                + i128::from(escrita) * pw;
        }
        uso.tokens = uso.entrada + uso.saida + uso.cache_leitura + uso.cache_escrita;
        // Décimos de centavo por milhão → centavos: divide por 10 milhões, arredondando.
        uso.custo_centesimos_dolar = ((custo + 5_000_000) / 10_000_000) as i64;
        Ok(uso)
    }

    fn dados_conversa(&self, id: i64) -> Result<DadosConversa, CoreError> {
        let (contexto, modelo, busca_web): (String, String, bool) = self
            .conn
            .query_row(
                "SELECT contexto, modelo, busca_web FROM conversa WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(classifica)?
            .ok_or(CoreError::ConversaNaoEncontrada(id))?;
        let mut st = self
            .conn
            .prepare("SELECT conteudo_json FROM mensagem WHERE conversa_id = ?1 ORDER BY id")
            .map_err(classifica)?;
        let historico = st
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(classifica)?
            .map(|s| {
                let s = s.map_err(classifica)?;
                serde_json::from_str(&s)
                    .map_err(|_| CoreError::DbInvalid("mensagem gravada ilegível".into()))
            })
            .collect::<Result<_, _>>()?;
        Ok(DadosConversa {
            contexto,
            modelo,
            busca_web,
            historico,
        })
    }

    /// Grava a conversa (se nova), a pergunta, a resposta e o uso numa transação só.
    fn gravar_troca(&self, t: &Troca) -> Result<i64, CoreError> {
        let tx = self.conn.unchecked_transaction().map_err(classifica)?;
        let quando = agora();
        let id = match (&t.nova, t.conversa_id) {
            (Some(nova), _) => {
                tx.execute(
                    "INSERT INTO conversa (titulo, modelo, contexto, fontes_internas, busca_web, \
                     criada_em, atualizada_em, foco) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
                    params![
                        nova.titulo,
                        t.dados.modelo,
                        t.dados.contexto,
                        nova.fontes_internas,
                        t.dados.busca_web,
                        quando,
                        nova.foco
                    ],
                )
                .map_err(classifica)?;
                tx.last_insert_rowid()
            }
            (None, Some(id)) => {
                let n = tx
                    .execute(
                        "UPDATE conversa SET atualizada_em = ?1 WHERE id = ?2",
                        params![quando, id],
                    )
                    .map_err(classifica)?;
                if n == 0 {
                    return Err(CoreError::ConversaNaoEncontrada(id));
                }
                id
            }
            (None, None) => unreachable!("toda troca tem conversa nova ou existente"),
        };
        let sem_fontes: &[Fonte] = &[];
        for (papel, conteudo, texto, fontes, aviso) in [
            ("user", t.pergunta, t.pergunta_texto, sem_fontes, None),
            ("assistant", t.resposta, t.resposta_texto, t.fontes, t.aviso),
        ] {
            let fontes = serde_json::to_string(fontes)
                .map_err(|_| CoreError::DbInvalid("fontes ilegíveis".into()))?;
            tx.execute(
                "INSERT INTO mensagem (conversa_id, papel, conteudo_json, texto, fontes_json, \
                 aviso, criada_em) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    papel,
                    conteudo.to_string(),
                    texto,
                    fontes,
                    aviso,
                    quando
                ],
            )
            .map_err(classifica)?;
        }
        somar_uso(&tx, &t.mes, &t.dados.modelo, t.uso)?;
        tx.commit().map_err(classifica)?;
        Ok(id)
    }
}

/// Envia, trata pause_turn (até 3) e devolve o acumulador com blocos. Recusa sem blocos vira
/// IA_RECUSA e resposta sem blocos vira IA_INDISPONIVEL. Em qualquer Err depois de um
/// `message_start`, soma o uso acumulado (melhor esforço: uma falha ao somar não troca o erro).
/// Nenhuma mensagem é gravada aqui.
#[allow(clippy::too_many_arguments)]
pub(crate) fn enviar_com_pausas(
    db: &Db,
    cliente: &dyn ClienteIa,
    chave: &str,
    hoje: NaiveDate,
    modelo: &str,
    corpo: &dyn Fn(&[Value]) -> Value,
    mensagens: &[Value],
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<Acumulador, CoreError> {
    let mut acumulador = Acumulador::novo();
    match enviar_partes(
        &mut acumulador,
        cliente,
        chave,
        corpo,
        mensagens,
        cancelar,
        progresso,
    ) {
        Ok(()) => Ok(acumulador),
        Err(erro) => {
            if acumulador.comecou() {
                let _ = somar_uso(&db.conn, &mes(hoje), modelo, acumulador.uso());
            }
            Err(erro)
        }
    }
}

fn enviar_partes(
    acumulador: &mut Acumulador,
    cliente: &dyn ClienteIa,
    chave: &str,
    corpo: &dyn Fn(&[Value]) -> Value,
    mensagens: &[Value],
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<(), CoreError> {
    let mut pausas = 0;
    loop {
        let mut pedido = mensagens.to_vec();
        if !acumulador.blocos().is_empty() {
            // Continuação de um `pause_turn`: a resposta parcial vai como mensagem da IA.
            pedido.push(json!({"role": "assistant", "content": acumulador.blocos()}));
        }
        let mut falha = None;
        cliente.enviar(chave, &corpo(&pedido), cancelar, &mut |e| {
            if falha.is_some() {
                return;
            }
            match acumulador.evento(e) {
                Ok(()) => progresso(acumulador.progresso()),
                Err(erro) => falha = Some(erro),
            }
        })?;
        if let Some(erro) = falha {
            return Err(erro.into());
        }
        if cancelar.load(Ordering::SeqCst) {
            return Err(ErroIa::Cancelada.into());
        }
        if !acumulador.parou() {
            // O corpo acabou antes do `message_stop`: a conexão caiu no meio.
            return Err(ErroIa::SemConexao.into());
        }
        if acumulador.stop_reason() == Some("pause_turn") && pausas < MAX_PAUSAS {
            pausas += 1;
            continue;
        }
        break;
    }
    if acumulador.blocos().is_empty() {
        // Nenhum bloco em nenhuma parte: uma mensagem da IA vazia quebraria o histórico
        // (a API recusa), então nada é gravado.
        let erro = if acumulador.stop_reason() == Some("refusal") {
            ErroIa::Recusa
        } else {
            ErroIa::Indisponivel
        };
        return Err(erro.into());
    }
    Ok(())
}

/// Uma pergunta inteira: cria a conversa se `conversa_id` é None (título = 60 primeiros caracteres
/// da pergunta, contexto congelado), envia, trata pause_turn (até 3) e grava a pergunta e a resposta
/// numa transação só no fim. Qualquer erro devolve Err sem gravar mensagem; os tokens de uma
/// resposta perdida depois do `message_start` entram no uso. `progresso` recebe cada atualização.
#[allow(clippy::too_many_arguments)]
pub fn perguntar(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    hoje: NaiveDate,
    conversa_id: Option<i64>,
    pergunta: &str,
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<i64, CoreError> {
    let destino = conversa_id.map_or(Destino::Assistente, Destino::Existente);
    perguntar_com(
        db, cofre, cliente, hoje, destino, pergunta, cancelar, progresso,
    )
}

/// `perguntar` para qualquer destino: conversa existente, nova do Assistente ou nova do aporte.
#[allow(clippy::too_many_arguments)]
pub(crate) fn perguntar_com(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    hoje: NaiveDate,
    destino: Destino,
    pergunta: &str,
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<i64, CoreError> {
    let chave = cofre
        .ler()?
        .filter(|c| !c.is_empty())
        .ok_or(ErroIa::SemChave)?;
    let (dados, nova, conversa_id) = match destino {
        Destino::Existente(id) => (db.dados_conversa(id)?, None, Some(id)),
        Destino::Assistente | Destino::Aporte(_) => {
            let (contexto, foco) = match destino {
                Destino::Aporte(f) => {
                    // `contexto_aporte` já recusou um código de classe desconhecido.
                    let contexto = db.contexto_aporte(hoje, f)?;
                    (contexto, Classe::de_codigo(&f.classe).map(|c| c.codigo()))
                }
                _ => (db.contexto_montado(hoje)?, None),
            };
            let config = db.config_ia()?;
            let nova = Nova {
                titulo: titulo(pergunta),
                fontes_internas: fontes_internas_json(&contexto.fontes_internas),
                foco,
            };
            let dados = DadosConversa {
                contexto: contexto.texto,
                modelo: config.modelo,
                busca_web: config.busca_web,
                historico: Vec::new(),
            };
            (dados, Some(nova), None)
        }
    };
    let msg_pergunta = json!({"role": "user", "content": pergunta});
    let mut mensagens = dados.historico.clone();
    mensagens.push(msg_pergunta.clone());

    let corpo =
        |pedido: &[Value]| corpo_pedido(&dados.modelo, &dados.contexto, dados.busca_web, pedido).0;
    let acumulador = enviar_com_pausas(
        db,
        cliente,
        &chave,
        hoje,
        &dados.modelo,
        &corpo,
        &mensagens,
        cancelar,
        progresso,
    )?;

    let mut fim = acumulador.progresso();
    if !fim.terminado {
        // Parou em `pause_turn` depois das 3 continuações: grava o que veio.
        fim.terminado = true;
        progresso(fim.clone());
    }
    let resposta = json!({"role": "assistant", "content": acumulador.blocos()});
    let aviso = (acumulador.stop_reason() == Some("refusal")).then_some(AVISO_RECUSA);
    db.gravar_troca(&Troca {
        nova,
        conversa_id,
        dados: &dados,
        pergunta: &msg_pergunta,
        pergunta_texto: pergunta,
        resposta: &resposta,
        resposta_texto: &fim.texto,
        fontes: &fim.fontes,
        aviso,
        uso: acumulador.uso(),
        mes: mes(hoje),
    })
}

/// Confere o formato e a chave na Anthropic; só então grava no cofre e os 4 últimos caracteres.
/// Se o banco falhar, o cofre volta ao que tinha antes: nunca fica uma chave que a tela não mostra.
pub fn salvar_chave(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    chave: &str,
) -> Result<Vec<ErroCampo>, CoreError> {
    if let Some(erro) = validar_formato_chave(chave) {
        return Ok(vec![erro]);
    }
    let chave = chave.trim();
    let erro_campo = |mensagem: &str| {
        Ok(vec![ErroCampo {
            campo: "chave".into(),
            mensagem: mensagem.into(),
        }])
    };
    match cliente.validar_chave(chave) {
        Ok(()) => {}
        Err(ErroIa::ChaveRecusada) => return erro_campo("A Anthropic recusou esta chave."),
        Err(ErroIa::SemConexao) => {
            return erro_campo("Sem conexão com a Anthropic. A chave não foi salva.");
        }
        Err(outro) => return Err(outro.into()),
    }
    let anterior = cofre.ler()?;
    cofre.gravar(chave)?;
    if let Err(e) = db.gravar_chave_final(Some(chave)) {
        // Melhor esforço: o erro que importa é o do banco.
        let _ = match anterior.as_deref() {
            Some(c) => cofre.gravar(c),
            None => cofre.apagar(),
        };
        return Err(e);
    }
    Ok(Vec::new())
}

/// Apaga a chave do cofre e limpa `chave_final`.
pub fn remover_chave(db: &Db, cofre: &dyn Cofre) -> Result<(), CoreError> {
    cofre.apagar()?;
    db.gravar_chave_final(None)
}

#[cfg(test)]
mod testes {
    use super::*;

    struct AceitaTudo;

    impl ClienteIa for AceitaTudo {
        fn validar_chave(&self, _: &str) -> Result<(), ErroIa> {
            Ok(())
        }
        fn enviar(
            &self,
            _: &str,
            _: &Value,
            _: &AtomicBool,
            _: &mut dyn FnMut(&Value),
        ) -> Result<(), ErroIa> {
            Err(ErroIa::SemConexao)
        }
    }

    #[test]
    fn banco_falhando_nao_deixa_chave_nova_no_cofre() {
        use crate::ia::cofre::CofreMemoria;
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let cofre = CofreMemoria::default();
        // Sem nada antes: o cofre volta a ficar vazio.
        db.conn
            .execute_batch(
                "CREATE TRIGGER falha BEFORE UPDATE ON ia_config \
                 BEGIN SELECT RAISE(ABORT, 'falha'); END;",
            )
            .unwrap();
        let nova = "sk-ant-api03-chave-nova-0000";
        assert!(salvar_chave(&db, &cofre, &AceitaTudo, nova).is_err());
        assert_eq!(cofre.ler().unwrap(), None);
        // Com uma chave antes: ela volta.
        cofre.gravar("sk-ant-api03-chave-antiga-1111").unwrap();
        assert!(salvar_chave(&db, &cofre, &AceitaTudo, nova).is_err());
        assert_eq!(
            cofre.ler().unwrap().as_deref(),
            Some("sk-ant-api03-chave-antiga-1111")
        );
        // Sem a falha, grava a chave sem os espaços das pontas.
        db.conn.execute_batch("DROP TRIGGER falha;").unwrap();
        let r = salvar_chave(&db, &cofre, &AceitaTudo, &format!(" {nova}\n")).unwrap();
        assert!(r.is_empty());
        assert_eq!(cofre.ler().unwrap().as_deref(), Some(nova));
        assert_eq!(db.config_ia().unwrap().chave_final.as_deref(), Some("0000"));
    }

    #[test]
    fn titulo_numa_linha_com_60_caracteres() {
        assert_eq!(titulo("  Oi,\n\ntudo  bem? "), "Oi, tudo bem?");
        assert_eq!(titulo(&"á".repeat(80)).chars().count(), 60);
    }

    #[test]
    fn fontes_internas_vao_e_voltam() {
        let f = vec![FonteInterna {
            parte: "mercado".into(),
            rotulo: "Mercado".into(),
            data_dados: NaiveDate::from_ymd_opt(2026, 10, 20),
            antigo: true,
        }];
        assert_eq!(ler_fontes_internas(&fontes_internas_json(&f)), f);
    }
}
