use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::NaiveDate;
use guru_core::{
    ClienteFixture, ClienteIa, Cofre, CofreMemoria, CoreError, Db, ErroIa, Fonte, ProgressoIa,
    corpo_pedido, perguntar, remover_chave, salvar_chave,
};
use serde_json::{Value, json};

const CHAVE: &str = "sk-ant-api03-SEGREDO-de-teste-7c1e9b4f2a";

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
}

fn pasta() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ia"))
}

fn fixture() -> ClienteFixture {
    ClienteFixture { pasta: pasta() }
}

fn banco() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn cofre_com_chave() -> CofreMemoria {
    let cofre = CofreMemoria::default();
    cofre.gravar(CHAVE).unwrap();
    cofre
}

fn sql(dir: &tempfile::TempDir) -> rusqlite::Connection {
    rusqlite::Connection::open(dir.path().join("guru.db")).unwrap()
}

fn contar(dir: &tempfile::TempDir, tabela: &str) -> i64 {
    sql(dir)
        .query_row(&format!("SELECT count(*) FROM {tabela}"), [], |r| r.get(0))
        .unwrap()
}

fn conteudos(dir: &tempfile::TempDir, conversa: i64) -> Vec<Value> {
    let c = sql(dir);
    let mut st = c
        .prepare("SELECT conteudo_json FROM mensagem WHERE conversa_id = ?1 ORDER BY id")
        .unwrap();
    st.query_map([conversa], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|s| serde_json::from_str(&s.unwrap()).unwrap())
        .collect()
}

/// Pergunta sem cancelamento, guardando cada progresso.
fn pergunta(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    conversa: Option<i64>,
    texto: &str,
) -> (Result<i64, CoreError>, Vec<ProgressoIa>) {
    let cancelar = AtomicBool::new(false);
    let mut vistos = Vec::new();
    let r = perguntar(
        db,
        cofre,
        cliente,
        hoje(),
        conversa,
        texto,
        &cancelar,
        &mut |p| vistos.push(p),
    );
    (r, vistos)
}

/// Cliente de teste que registra os corpos e repassa ao `ClienteFixture`.
struct Gravador {
    dentro: ClienteFixture,
    corpos: Mutex<Vec<Value>>,
}

impl Gravador {
    fn novo() -> Self {
        Self {
            dentro: fixture(),
            corpos: Mutex::new(Vec::new()),
        }
    }
    fn corpos(&self) -> Vec<Value> {
        self.corpos.lock().unwrap().clone()
    }
}

impl ClienteIa for Gravador {
    fn validar_chave(&self, chave: &str) -> Result<(), ErroIa> {
        self.dentro.validar_chave(chave)
    }
    fn enviar(
        &self,
        chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa> {
        self.corpos.lock().unwrap().push(corpo.clone());
        self.dentro.enviar(chave, corpo, cancelar, ao_evento)
    }
}

const SISTEMA: &str = "Você é o assistente do Guru de Investimentos, um app brasileiro de carteira pessoal. Responda em português do Brasil, com clareza e sem jargão desnecessário. Apresente opções, com prós e contras, e deixe a decisão com o usuário: não faça recomendação individual de investimento nem diga o que ele deve comprar ou vender. Use o contexto enviado sobre a carteira, o perfil, as metas e o mercado. Para fatos de mercado recentes (decisões do Copom, projeções, notícias), busque na web quando a busca estiver disponível e cite a fonte. Quando algo for opinião sua, sem fonte, diga que é opinião. Se faltar um dado para responder bem, diga qual.";

const TEXTO_SIMPLES: &str = "Olá! Sua carteira está concentrada em renda fixa pós-fixada, o que combina com um perfil conservador.\n\nAlgumas opções:\n\n- **Manter como está**: mais previsível.\n- **Diversificar com prefixados**: trava a taxa, mas oscila no caminho.\n\nA decisão é sua.";

fn busca(tipo: &str) -> Value {
    json!([{
        "type": tipo,
        "name": "web_search",
        "max_uses": 5,
        "user_location": {"type": "approximate", "country": "BR", "timezone": "America/Sao_Paulo"}
    }])
}

#[test]
fn corpo_tem_os_campos_exatos() {
    let msgs = vec![json!({"role": "user", "content": "Oi"})];
    let (c, beta) = corpo_pedido("claude-opus-5-5", "CONTEXTO", true, &msgs);
    assert_eq!(c["model"], "claude-opus-5-5");
    assert_eq!(c["max_tokens"], 32000);
    assert_eq!(c["stream"], true);
    for proibido in ["temperature", "top_p", "top_k", "thinking"] {
        assert!(c.get(proibido).is_none(), "não deveria ter {proibido}");
    }
    assert_eq!(
        c["system"],
        json!([
            {"type": "text", "text": SISTEMA},
            {"type": "text", "text": "CONTEXTO", "cache_control": {"type": "ephemeral"}}
        ])
    );
    assert_eq!(c["messages"], json!(msgs));
    assert_eq!(c["output_config"], json!({"effort": "medium"}));
    assert_eq!(c["tools"], busca("web_search_20260209"));
    assert_eq!(c["fallbacks"], "default");
    assert_eq!(beta, Some("server-side-fallback-2026-07-01"));
    assert_eq!(c.as_object().unwrap().len(), 8, "campos a mais: {c}");

    let (c, beta) = corpo_pedido("claude-sonnet-5-5", "CONTEXTO", true, &msgs);
    assert_eq!(c["tools"], busca("web_search_20260209"));
    assert_eq!(c["fallbacks"], "default");
    assert_eq!(beta, Some("server-side-fallback-2026-07-01"));

    let (c, beta) = corpo_pedido("claude-haiku-5-5", "CONTEXTO", true, &msgs);
    assert_eq!(c["tools"], busca("web_search_20250305"));
    assert!(c.get("fallbacks").is_none());
    assert_eq!(beta, None);

    let (c, _) = corpo_pedido("claude-opus-5-5", "CONTEXTO", false, &msgs);
    assert!(c.get("tools").is_none());
    let (c, _) = corpo_pedido("claude-haiku-5-5", "CONTEXTO", false, &msgs);
    assert!(c.get("tools").is_none());
}

#[test]
fn sem_chave_da_erro_e_nao_grava() {
    let (dir, db) = banco();
    let cofre = CofreMemoria::default();
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "Oi");
    let e = r.unwrap_err();
    assert_eq!(e.code(), "IA_SEM_CHAVE");
    assert_eq!(e.to_string(), "Configure a sua chave em Configurar IA.");
    assert!(db.listar_conversas().unwrap().is_empty());
    assert_eq!(contar(&dir, "mensagem"), 0);
    assert_eq!(contar(&dir, "ia_uso"), 0);
}

#[test]
fn pergunta_simples_grava_pergunta_e_resposta() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, vistos) = pergunta(&db, &cofre, &fixture(), None, "Como está a minha carteira?");
    let id = r.unwrap();

    assert!(vistos.len() > 5, "o texto deveria chegar aos poucos");
    assert!(
        vistos
            .iter()
            .any(|p| !p.texto.is_empty() && p.texto != TEXTO_SIMPLES)
    );
    let ultimo = vistos.last().unwrap();
    assert_eq!(ultimo.texto, TEXTO_SIMPLES);
    assert!(ultimo.terminado);
    assert!(!ultimo.buscando);

    let c = db.conversa(id).unwrap();
    assert_eq!(c.titulo, "Como está a minha carteira?");
    assert_eq!(c.modelo, "claude-opus-5-5");
    assert_eq!(c.mensagens.len(), 2);
    assert_eq!(c.mensagens[0].papel, "user");
    assert_eq!(c.mensagens[0].texto, "Como está a minha carteira?");
    assert_eq!(c.mensagens[1].papel, "assistant");
    assert_eq!(c.mensagens[1].texto, TEXTO_SIMPLES);
    assert!(c.mensagens[1].fontes.is_empty());
    assert_eq!(c.mensagens[1].aviso, None);

    let gravado = conteudos(&dir, id);
    assert_eq!(
        gravado[0],
        json!({"role": "user", "content": "Como está a minha carteira?"})
    );
    assert_eq!(
        gravado[1],
        json!({"role": "assistant", "content": [
            {"type": "thinking",
             "thinking": "O usuário quer uma visão geral da carteira. Vou apresentar opções sem recomendar.",
             "signature": "EqQBCkYIBxgCKkBzaW1wbGVzLWFzc2luYXR1cmEtZGUtdGVzdGU="},
            {"type": "text", "text": TEXTO_SIMPLES}
        ]})
    );

    let lista = db.listar_conversas().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].id, id);

    // Título: os 60 primeiros caracteres; a conversa mais nova vem primeiro.
    let longa = "Quero entender melhor como a decisão do Copom afeta os meus títulos pós-fixados";
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, longa);
    let id2 = r.unwrap();
    let lista = db.listar_conversas().unwrap();
    assert_eq!(lista[0].id, id2);
    assert_eq!(lista[0].titulo, longa.chars().take(60).collect::<String>());
    assert_eq!(lista[0].titulo.chars().count(), 60);

    db.apagar_conversa(id2).unwrap();
    assert_eq!(db.listar_conversas().unwrap().len(), 1);
    assert_eq!(contar(&dir, "mensagem"), 2);
    assert_eq!(db.conversa(id2).unwrap_err().code(), "NAO_ENCONTRADO");
}

#[test]
fn busca_numera_fontes_por_url() {
    let (_dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, vistos) = pergunta(
        &db,
        &cofre,
        &fixture(),
        None,
        "#busca O que o Copom decidiu?",
    );
    let id = r.unwrap();
    assert!(vistos.iter().any(|p| p.buscando), "deveria mostrar a busca");
    assert!(!vistos.last().unwrap().buscando);

    let esperado_texto =
        "O Copom manteve a Selic em 15% ao ano[1][2], e sinalizou cautela com a inflação.[1]";
    let esperadas = vec![
        Fonte {
            numero: 1,
            url: "https://www.bcb.gov.br/detalhenoticia/copom-outubro".into(),
            titulo: "Copom mantém a taxa Selic".into(),
            dominio: "bcb.gov.br".into(),
            data: Some("October 22, 2026".into()),
            trecho: "O Comitê decidiu manter a taxa Selic em 15,00% a.a.".into(),
        },
        Fonte {
            numero: 2,
            url: "https://valor.globo.com/financas/copom-decisao".into(),
            titulo: "O que muda com a decisão do Copom".into(),
            dominio: "valor.globo.com".into(),
            data: None,
            trecho: "A decisão foi unânime e veio em linha com o esperado.".into(),
        },
    ];
    let ultimo = vistos.last().unwrap();
    assert_eq!(ultimo.texto, esperado_texto);
    assert_eq!(ultimo.fontes, esperadas);

    let c = db.conversa(id).unwrap();
    assert_eq!(c.mensagens[1].texto, esperado_texto);
    assert_eq!(c.mensagens[1].fontes, esperadas);
}

#[test]
fn segunda_pergunta_reenvia_blocos_identicos_e_mesmo_system() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let cliente = Gravador::novo();
    let (r, _) = pergunta(&db, &cofre, &cliente, None, "#busca O que o Copom decidiu?");
    let id = r.unwrap();

    // A configuração muda no meio da conversa: o contexto congelado não pode mudar.
    db.salvar_config_ia(
        "claude-haiku-5-5",
        "Fale como um pirata.",
        false,
        true,
        false,
    )
    .unwrap();

    let (r, _) = pergunta(
        &db,
        &cofre,
        &cliente,
        Some(id),
        "E o que isso muda para mim?",
    );
    assert_eq!(r.unwrap(), id);

    let corpos = cliente.corpos();
    assert_eq!(corpos.len(), 2);
    assert_eq!(
        serde_json::to_string(&corpos[1]["system"]).unwrap(),
        serde_json::to_string(&corpos[0]["system"]).unwrap()
    );
    assert_eq!(corpos[1]["model"], corpos[0]["model"]);
    assert_eq!(corpos[1]["tools"], corpos[0]["tools"]);

    let gravado = conteudos(&dir, id);
    assert_eq!(gravado.len(), 4);
    let msgs = corpos[1]["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0], gravado[0]);
    assert_eq!(msgs[1], gravado[1]);
    assert_eq!(
        msgs[2],
        json!({"role": "user", "content": "E o que isso muda para mim?"})
    );
    assert_eq!(corpos[0]["messages"][0], msgs[0]);

    // Os blocos completos voltam sem tirar nada.
    let blocos = msgs[1]["content"].as_array().unwrap();
    let tipos: Vec<&str> = blocos.iter().map(|b| b["type"].as_str().unwrap()).collect();
    assert_eq!(
        tipos,
        [
            "thinking",
            "server_tool_use",
            "web_search_tool_result",
            "text",
            "text"
        ]
    );
    assert_eq!(
        blocos[0]["signature"],
        "EqQBCkYIBxgCKkBidXNjYS1hc3NpbmF0dXJhLWRlLXRlc3Rl"
    );
    assert_eq!(
        blocos[1]["input"],
        json!({"query": "decisão do Copom outubro 2026"})
    );
    assert_eq!(
        blocos[2]["content"][0]["encrypted_content"],
        "EqgfCioIARgBIiQ3YmNiLWNvbnRldWRvLWNpZnJhZG8tdW0="
    );
    assert_eq!(blocos[2]["content"][0]["page_age"], "October 22, 2026");
    assert_eq!(blocos[3]["text"], "O Copom manteve a Selic em 15% ao ano");
    assert_eq!(blocos[3]["citations"].as_array().unwrap().len(), 2);
    assert_eq!(
        blocos[3]["citations"][1]["encrypted_index"],
        "EpcBCioIARgBIiR2YWxvci1pbmRpY2UtdW0="
    );
    assert_eq!(blocos[4]["citations"].as_array().unwrap().len(), 1);
    assert!(
        !blocos[4]["text"].as_str().unwrap().contains('['),
        "o marcador é só do texto exibido"
    );
}

#[test]
fn pause_turn_reenvia_e_junta() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let cliente = Gravador::novo();
    let (r, vistos) = pergunta(&db, &cofre, &cliente, None, "#pause Como ficou o IPCA?");
    let id = r.unwrap();

    let corpos = cliente.corpos();
    assert_eq!(corpos.len(), 2);
    let msgs = corpos[1]["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2, "sem mensagem extra do usuário");
    assert_eq!(msgs[0], corpos[0]["messages"][0]);
    assert_eq!(msgs[1]["role"], "assistant");
    assert_eq!(msgs[1]["content"].as_array().unwrap().len(), 3);
    assert_eq!(
        msgs[1]["content"][2]["content"]["error_code"],
        "too_many_requests"
    );
    assert_eq!(corpos[1]["system"], corpos[0]["system"]);

    let gravado = conteudos(&dir, id);
    assert_eq!(gravado.len(), 2, "uma pergunta e uma resposta só");
    let blocos = gravado[1]["content"].as_array().unwrap();
    assert_eq!(blocos.len(), 5);
    assert_eq!(&blocos[..3], &msgs[1]["content"].as_array().unwrap()[..]);

    let esperado = format!("Vou buscar os dados mais recentes. {TEXTO_SIMPLES}");
    assert_eq!(db.conversa(id).unwrap().mensagens[1].texto, esperado);
    assert!(!vistos.iter().take(5).any(|p| p.terminado));
    assert!(vistos.last().unwrap().terminado);
    assert_eq!(vistos.last().unwrap().texto, esperado);

    let uso = db.uso_do_mes(hoje()).unwrap();
    assert_eq!(uso.entrada, 900 + 1200);
    assert_eq!(uso.saida, 40 + 85);
}

/// Sempre devolve `pause.sse`: tira a resposta parcial antes de repassar ao fixture.
struct SemprePausa {
    dentro: ClienteFixture,
    chamadas: Mutex<u32>,
}

impl ClienteIa for SemprePausa {
    fn validar_chave(&self, _: &str) -> Result<(), ErroIa> {
        Ok(())
    }
    fn enviar(
        &self,
        chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa> {
        *self.chamadas.lock().unwrap() += 1;
        let mut corpo = corpo.clone();
        let msgs = corpo["messages"].as_array_mut().unwrap();
        if msgs.last().unwrap()["role"] == "assistant" {
            msgs.pop();
        }
        self.dentro.enviar(chave, &corpo, cancelar, ao_evento)
    }
}

#[test]
fn pause_turn_reenvia_no_maximo_tres_vezes() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let cliente = SemprePausa {
        dentro: fixture(),
        chamadas: Mutex::new(0),
    };
    let (r, _) = pergunta(&db, &cofre, &cliente, None, "#pause IPCA?");
    let id = r.unwrap();
    assert_eq!(*cliente.chamadas.lock().unwrap(), 4);
    let gravado = conteudos(&dir, id);
    assert_eq!(gravado.len(), 2);
    assert_eq!(gravado[1]["content"].as_array().unwrap().len(), 12);
}

#[test]
fn recusa_grava_com_aviso() {
    let (_dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "#recusa Algo proibido");
    let c = db.conversa(r.unwrap()).unwrap();
    assert_eq!(c.mensagens.len(), 2);
    assert_eq!(c.mensagens[1].texto, "Não posso ajudar com isso.");
    assert_eq!(
        c.mensagens[1].aviso.as_deref(),
        Some("A IA preferiu não responder a esta pergunta.")
    );
    assert_eq!(c.mensagens[0].aviso, None);
}

#[test]
fn erro_no_meio_nao_grava_nada() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let msg = "A Anthropic está indisponível agora. Tente de novo em instantes.";
    for p in ["#erro529 Oi", "#quebrado Oi"] {
        let (r, _) = pergunta(&db, &cofre, &fixture(), None, p);
        let e = r.unwrap_err();
        assert_eq!(e.code(), "IA_INDISPONIVEL", "{p}");
        assert_eq!(e.to_string(), msg);
    }
    assert!(db.listar_conversas().unwrap().is_empty());
    assert_eq!(contar(&dir, "conversa"), 0);
    assert_eq!(contar(&dir, "mensagem"), 0);
    // Só o #quebrado chegou ao `message_start`: os tokens dele contam, os do #erro529 não existem.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().entrada, 500);

    // Numa conversa existente: o erro não deixa pergunta órfã, e a seguinte funciona.
    let id = pergunta(&db, &cofre, &fixture(), None, "Oi").0.unwrap();
    let (r, _) = pergunta(&db, &cofre, &fixture(), Some(id), "#erro401 E agora?");
    let e = r.unwrap_err();
    assert_eq!(e.code(), "IA_CHAVE_RECUSADA");
    assert_eq!(
        e.to_string(),
        "A Anthropic recusou a sua chave. Confira em Configurar IA."
    );
    let (r, _) = pergunta(&db, &cofre, &fixture(), Some(id), "#quebrado E agora?");
    assert_eq!(r.unwrap_err().code(), "IA_INDISPONIVEL");
    assert_eq!(db.conversa(id).unwrap().mensagens.len(), 2);
    pergunta(&db, &cofre, &fixture(), Some(id), "E agora?")
        .0
        .unwrap();
    assert_eq!(db.conversa(id).unwrap().mensagens.len(), 4);

    let (r, _) = pergunta(&db, &cofre, &fixture(), Some(9999), "Oi");
    assert_eq!(r.unwrap_err().code(), "NAO_ENCONTRADO");
}

#[test]
fn cancelar_nao_grava_nada() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let cancelar = AtomicBool::new(false);
    let mut n = 0;
    let r = perguntar(
        &db,
        &cofre,
        &fixture(),
        hoje(),
        None,
        "Oi",
        &cancelar,
        &mut |_| {
            n += 1;
            if n == 3 {
                cancelar.store(true, Ordering::SeqCst);
            }
        },
    );
    let e = r.unwrap_err();
    assert_eq!(e.code(), "IA_CANCELADA");
    assert_eq!(e.to_string(), "Resposta interrompida.");
    assert!(n < 10, "deveria parar logo depois do pedido");
    assert_eq!(contar(&dir, "conversa"), 0);
    assert_eq!(contar(&dir, "mensagem"), 0);
    // Os tokens do `message_start` contam mesmo com a resposta parada.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().entrada, 1200);

    pergunta(&db, &cofre, &fixture(), None, "Oi").0.unwrap();
    assert_eq!(contar(&dir, "mensagem"), 2);
}

#[test]
fn chave_nunca_aparece_no_banco() {
    let (dir, db) = banco();
    let cofre = CofreMemoria::default();
    assert!(
        salvar_chave(&db, &cofre, &fixture(), CHAVE)
            .unwrap()
            .is_empty()
    );
    let id = pergunta(&db, &cofre, &fixture(), None, "#busca Copom?")
        .0
        .unwrap();
    pergunta(&db, &cofre, &fixture(), Some(id), "#pause E o IPCA?")
        .0
        .unwrap();
    let erro = pergunta(&db, &cofre, &fixture(), Some(id), "#erro401 Oi")
        .0
        .unwrap_err();
    assert!(!erro.to_string().contains(CHAVE));
    let conversa = db.conversa(id).unwrap();
    assert!(!format!("{conversa:?}").contains(CHAVE));
    assert!(!format!("{:?}", db.config_ia().unwrap()).contains(CHAVE));

    let sem_final = &CHAVE[..CHAVE.len() - 4];
    let c = sql(&dir);
    let tabelas: Vec<String> = c
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut textos = 0;
    for t in &tabelas {
        let mut st = c.prepare(&format!("SELECT * FROM \"{t}\"")).unwrap();
        let colunas = st.column_count();
        let mut linhas = st.query([]).unwrap();
        while let Some(l) = linhas.next().unwrap() {
            for i in 0..colunas {
                if let rusqlite::types::ValueRef::Text(b) = l.get_ref(i).unwrap() {
                    textos += 1;
                    let s = String::from_utf8_lossy(b);
                    assert!(!s.contains(sem_final), "chave em {t}.{i}");
                }
            }
        }
    }
    assert!(textos > 10);
    drop(c);
    for arquivo in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(arquivo.unwrap().path()).unwrap();
        assert!(
            !bytes
                .windows(sem_final.len())
                .any(|w| w == sem_final.as_bytes()),
            "chave num arquivo do banco"
        );
    }
}

struct SemRede;

impl ClienteIa for SemRede {
    fn validar_chave(&self, _: &str) -> Result<(), ErroIa> {
        Err(ErroIa::SemConexao)
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
fn salvar_chave_valida_formato_e_api() {
    let (_dir, db) = banco();
    let cofre = CofreMemoria::default();

    let e = salvar_chave(&db, &cofre, &fixture(), "abc").unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].campo, "chave");
    assert_eq!(
        e[0].mensagem,
        "Isso não parece uma chave da Anthropic (começa com sk-ant-)."
    );
    assert_eq!(cofre.ler().unwrap(), None);

    let e = salvar_chave(&db, &cofre, &fixture(), "sk-ant-teste-recusada-0000").unwrap();
    assert_eq!(e[0].mensagem, "A Anthropic recusou esta chave.");
    assert_eq!(cofre.ler().unwrap(), None);
    assert_eq!(db.config_ia().unwrap().chave_final, None);

    let e = salvar_chave(&db, &cofre, &SemRede, CHAVE).unwrap();
    assert_eq!(
        e[0].mensagem,
        "Sem conexão com a Anthropic. A chave não foi salva."
    );
    assert_eq!(cofre.ler().unwrap(), None);

    assert!(
        salvar_chave(&db, &cofre, &fixture(), CHAVE)
            .unwrap()
            .is_empty()
    );
    assert_eq!(cofre.ler().unwrap().as_deref(), Some(CHAVE));
    assert_eq!(db.config_ia().unwrap().chave_final.as_deref(), Some("4f2a"));

    remover_chave(&db, &cofre).unwrap();
    assert_eq!(cofre.ler().unwrap(), None);
    assert_eq!(db.config_ia().unwrap().chave_final, None);

    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "Oi");
    assert_eq!(r.unwrap_err().code(), "IA_SEM_CHAVE");
}

#[test]
fn uso_do_mes_soma_e_estima_custo() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let zero = db.uso_do_mes(hoje()).unwrap();
    assert_eq!((zero.tokens, zero.custo_centesimos_dolar), (0, 0));

    // Opus: duas simples (1200 entrada, 1000 escrita, 85 saída) e uma busca
    // (2000 entrada, 1500 leitura, 240 saída).
    pergunta(&db, &cofre, &fixture(), None, "Oi").0.unwrap();
    pergunta(&db, &cofre, &fixture(), None, "Oi de novo")
        .0
        .unwrap();
    pergunta(&db, &cofre, &fixture(), None, "#busca Copom?")
        .0
        .unwrap();
    let u = db.uso_do_mes(hoje()).unwrap();
    assert_eq!(u.entrada, 4400);
    assert_eq!(u.saida, 410);
    assert_eq!(u.cache_leitura, 1500);
    assert_eq!(u.cache_escrita, 2000);
    assert_eq!(u.tokens, 4400 + 410 + 1500 + 2000);
    // 4400×4 + 410×20 + 1500×0,20 + 2000×5 = 36,1 milionésimos de dólar por milhão → US$ 0,0361.
    assert_eq!(u.custo_centesimos_dolar, 4);

    // Os preços de cada modelo, com 1 milhão de tokens de cada tipo num mês sem perguntas:
    // Opus 4 + 20 + 0,20 + 5 = 29,20; Sonnet 2 + 10 + 0,20 + 2,50 = 14,70;
    // Haiku 0,10 + 0,50 + 0,01 + 0,125 = 0,735. Total US$ 44,635.
    let c = sql(&dir);
    for modelo in ["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"] {
        c.execute(
            "INSERT INTO ia_uso VALUES ('2026-09', ?1, 1000000, 1000000, 1000000, 1000000)",
            [modelo],
        )
        .unwrap();
    }
    let setembro = db
        .uso_do_mes(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap())
        .unwrap();
    assert_eq!(setembro.tokens, 12_000_000);
    assert_eq!(setembro.custo_centesimos_dolar, 4464);

    let outro_mes = db
        .uso_do_mes(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap())
        .unwrap();
    assert_eq!(outro_mes.tokens, 0);
}

/// Nenhuma conversa nem mensagem. O uso conta: estas respostas passaram do `message_start`.
fn nada_gravado(dir: &tempfile::TempDir) {
    assert_eq!(contar(dir, "conversa"), 0);
    assert_eq!(contar(dir, "mensagem"), 0);
    assert_eq!(contar(dir, "ia_uso"), 1);
}

#[test]
fn recusa_vazia_da_erro_e_nao_grava() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "#recusavazia Algo");
    let e = r.unwrap_err();
    assert_eq!(e.code(), "IA_RECUSA");
    assert_eq!(
        e.to_string(),
        "A IA preferiu não responder a esta pergunta."
    );
    nada_gravado(&dir);
}

#[test]
fn resposta_vazia_da_indisponivel_e_nao_grava() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "#vazia Oi");
    assert_eq!(r.unwrap_err().code(), "IA_INDISPONIVEL");
    nada_gravado(&dir);

    // Todas as partes de um pause_turn vazias: também nada gravado.
    let cliente = Gravador::novo();
    let (r, _) = pergunta(&db, &cofre, &cliente, None, "#pausavazia Oi");
    assert_eq!(r.unwrap_err().code(), "IA_INDISPONIVEL");
    assert_eq!(cliente.corpos().len(), 4);
    nada_gravado(&dir);

    // Numa conversa existente, a resposta vazia não deixa pergunta órfã.
    let id = pergunta(&db, &cofre, &fixture(), None, "Oi").0.unwrap();
    let (r, _) = pergunta(&db, &cofre, &fixture(), Some(id), "#recusavazia E agora?");
    assert_eq!(r.unwrap_err().code(), "IA_RECUSA");
    assert_eq!(db.conversa(id).unwrap().mensagens.len(), 2);
    assert_eq!(contar(&dir, "mensagem"), 2);
}

#[test]
fn uso_conta_resposta_que_falha_depois_do_message_start() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let (r, _) = pergunta(&db, &cofre, &fixture(), None, "#quebrado Oi");
    assert_eq!(r.unwrap_err().code(), "IA_INDISPONIVEL");
    assert_eq!(contar(&dir, "conversa"), 0);
    assert_eq!(contar(&dir, "mensagem"), 0);
    // O `message_start` de quebrado.sse: 500 de entrada e 1 de saída.
    let u = db.uso_do_mes(hoje()).unwrap();
    assert_eq!(
        (u.entrada, u.saida, u.cache_leitura, u.cache_escrita),
        (500, 1, 0, 0)
    );

    // Numa conversa existente: o uso soma, a conversa fica com as duas mensagens.
    let id = pergunta(&db, &cofre, &fixture(), None, "Oi").0.unwrap();
    let (r, _) = pergunta(&db, &cofre, &fixture(), Some(id), "#quebrado E agora?");
    assert_eq!(r.unwrap_err().code(), "IA_INDISPONIVEL");
    assert_eq!(db.conversa(id).unwrap().mensagens.len(), 2);
    assert_eq!(db.uso_do_mes(hoje()).unwrap().entrada, 500 + 1200 + 500);
}

#[test]
fn uso_conta_resposta_cancelada() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    let cancelar = AtomicBool::new(false);
    let mut n = 0;
    let r = perguntar(
        &db,
        &cofre,
        &fixture(),
        hoje(),
        None,
        "Oi",
        &cancelar,
        &mut |_| {
            n += 1;
            if n == 3 {
                cancelar.store(true, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(r.unwrap_err().code(), "IA_CANCELADA");
    assert_eq!(contar(&dir, "conversa"), 0);
    assert_eq!(contar(&dir, "mensagem"), 0);
    let u = db.uso_do_mes(hoje()).unwrap();
    assert_eq!(u.entrada, 1200);
    assert_eq!(u.cache_escrita, 1000);
    assert_eq!(u.cache_leitura, 0);
}

#[test]
fn erro_antes_do_message_start_nao_conta() {
    let (dir, db) = banco();
    let cofre = cofre_com_chave();
    for p in ["#erro529 Oi", "#erro401 Oi"] {
        let (r, _) = pergunta(&db, &cofre, &fixture(), None, p);
        assert!(r.is_err(), "{p}");
    }
    assert_eq!(contar(&dir, "ia_uso"), 0);
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 0);
}
