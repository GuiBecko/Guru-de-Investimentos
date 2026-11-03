use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use chrono::NaiveDate;
use guru_core::{
    Classe, ClienteFixture, ClienteIa, Cofre, CofreMemoria, CoreError, DIRECOES, Db, ErroIa,
    FocoAporte, Gravacao, NovaMeta, NovoInvestimento, perguntar, perguntar_aporte,
};
use serde_json::Value;

const CHAVE: &str = "sk-ant-api03-SEGREDO-de-teste-7c1e9b4f2a";

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
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

/// Alvo 45/30/15/10/0.
fn salvar_alvo(db: &Db) {
    let valores: Vec<(String, i64)> = Classe::TODOS
        .iter()
        .zip([45, 30, 15, 10, 0])
        .map(|(c, p)| (c.codigo().to_string(), p))
        .collect();
    assert!(db.salvar_alocacao_alvo(&valores).unwrap().is_empty());
}

fn criar(db: &Db, n: &NovoInvestimento) {
    match db.criar_investimento(n, hoje()).unwrap() {
        Gravacao::Ok(_) => {}
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn cdb(valor: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: "Banco Inter".into(),
        emissor: s("Banco Inter"),
        indexador: s("pos"),
        taxa: s("102"),
        benchmark: s("cdi"),
        data_aplicacao: s("2026-04-15"),
        vencimento: s("2028-03-15"),
        valor_aplicado_centavos: Some(valor),
        liquidez: s("vencimento"),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn rv(
    tipo: &str,
    ticker: &str,
    quantidade: i64,
    preco: i64,
    meta: Option<i64>,
) -> NovoInvestimento {
    NovoInvestimento {
        tipo: tipo.into(),
        instituicao: "Rico".into(),
        ticker: s(ticker),
        data_aplicacao: s("2026-01-10"),
        quantidade: Some(quantidade),
        preco_medio_centavos: Some(preco),
        meta_id: meta,
        ..Default::default()
    }
}

/// R$ 10.000,00: RF 5.950 (59,5%), Ações 2.200 (22,0%), FIIs 1.350, Exterior 500. Um aporte de
/// R$ 2.000,00 dá Ações 1.100, FIIs 300, Exterior 600 e RF 0. A meta (alvo R$ 8.000,00) tem
/// as ações, 27,5% atingido. Devolve o id da meta.
fn carteira_exemplo(db: &Db) -> i64 {
    salvar_alvo(db);
    let meta = db
        .criar_meta(
            &NovaMeta {
                nome: "Entrada do apartamento".into(),
                valor_alvo_centavos: Some(800_000),
                prazo: s("2030-12"),
            },
            hoje(),
        )
        .unwrap()
        .unwrap();
    criar(db, &cdb(595_000));
    criar(db, &rv("rv-acao", "ITUB4", 100, 2_200, Some(meta)));
    criar(db, &rv("rv-fii", "HGLG11", 100, 1_350, None));
    criar(db, &rv("rv-bdr", "AAPL34", 10, 5_000, None));
    meta
}

fn foco(classe: &str, direcoes: &[&str], livre: &str, meta: Option<i64>) -> FocoAporte {
    FocoAporte {
        classe: classe.into(),
        valor_centavos: 200_000,
        direcoes: direcoes.iter().map(|d| d.to_string()).collect(),
        direcao_livre: livre.into(),
        meta_id: meta,
    }
}

/// O bloco "## Próximo aporte" até a linha em branco antes de "Hoje é".
fn bloco(texto: &str) -> &str {
    let inicio = texto
        .find("## Próximo aporte")
        .expect("sem o bloco do aporte");
    let fim = texto.find("\n\nHoje é").expect("sem a data");
    assert!(inicio < fim, "o bloco deveria vir antes de \"Hoje é\"");
    &texto[inicio..fim]
}

#[test]
fn direcoes_na_ordem_das_global_constraints() {
    assert_eq!(
        DIRECOES,
        [
            ("renda_fixa", "Renda fixa"),
            ("acoes", "Ações"),
            ("fiis", "FIIs"),
            ("exterior", "Exterior"),
            ("novo", "Experimentar algo novo"),
            ("seguranca", "Mais segurança"),
        ]
    );
}

#[test]
fn contexto_do_aporte_tem_valor_classe_direcao_e_meta() {
    let (_d, db) = banco();
    let meta = carteira_exemplo(&db);
    // Direções fora de ordem e repetidas: vão uma vez, na ordem de DIRECOES.
    let f = foco(
        "acoes",
        &["novo", "acoes", "novo"],
        "  quero algo com liquidez ",
        Some(meta),
    );
    let c = db.contexto_aporte(hoje(), &f).unwrap();
    let esperado = [
        "## Próximo aporte",
        "Valor deste aporte: R$ 2.000,00.",
        "Classe em foco: Ações, com R$ 1.100,00 deste aporte pela divisão de rebalanceamento (alvo 30%, hoje 8,0 p.p. abaixo do alvo).",
        "Divisão do aporte: Renda fixa 0,0%; Ações 55,0%; FIIs 15,0%; Exterior 30,0%.",
        "Direção que o usuário quer dar a este dinheiro: Ações; Experimentar algo novo. Nas palavras dele: \"quero algo com liquidez\".",
        "Meta ligada a este aporte: Entrada do apartamento · prazo 12/2030 · 27,5% atingido.",
        "O usuário quer ajuda para escolher em que investir dentro de Ações com este valor: apresente opções, com prós e contras, e tire as dúvidas dele.",
    ];
    let linhas: Vec<&str> = bloco(&c.texto).lines().collect();
    assert_eq!(linhas, esperado);
    assert!(c.texto.ends_with("Hoje é 30/10/2026."));
    // O resto é o contexto da 6a, sem mudança.
    let base = db.contexto_montado(hoje()).unwrap().texto;
    assert_eq!(
        c.texto.replace(&format!("{}\n\n", esperado.join("\n")), ""),
        base
    );

    let ultima = c.fontes_internas.last().unwrap();
    assert_eq!(ultima.parte, "aporte");
    assert_eq!(ultima.rotulo, "Próximo aporte");
    assert_eq!(ultima.data_dados, None);
    assert!(!ultima.antigo);
    assert_eq!(
        c.fontes_internas.len(),
        db.contexto_montado(hoje()).unwrap().fontes_internas.len() + 1
    );
    assert_eq!(c.tokens, guru_core::estimar_tokens(&c.texto));
}

#[test]
fn direcao_vazia_e_sem_meta() {
    let (_d, db) = banco();
    carteira_exemplo(&db);
    let c = db
        .contexto_aporte(hoje(), &foco("fii", &[], "   ", None))
        .unwrap();
    let b = bloco(&c.texto);
    assert!(b.contains("\nDireção que o usuário quer dar a este dinheiro: não indicada.\n"));
    assert!(b.contains("\nMeta ligada a este aporte: nenhuma.\n"));
    assert!(!b.contains("Nas palavras dele"));
    assert!(b.contains(
        "Classe em foco: FIIs, com R$ 300,00 deste aporte pela divisão de rebalanceamento (alvo 15%, hoje 1,5 p.p. abaixo do alvo)."
    ));
    // A classe que recebe R$ 0 também tem chat.
    let rf = db
        .contexto_aporte(hoje(), &foco("rf", &["seguranca"], "", None))
        .unwrap();
    assert!(bloco(&rf.texto).contains(
        "Classe em foco: Renda fixa, com R$ 0,00 deste aporte pela divisão de rebalanceamento (alvo 45%, hoje 14,5 p.p. acima do alvo)."
    ));
    assert!(
        bloco(&rf.texto)
            .contains("Direção que o usuário quer dar a este dinheiro: Mais segurança.\n")
    );
}

#[test]
fn carteira_desligada_tira_a_situacao_de_hoje() {
    let (_d, db) = banco();
    carteira_exemplo(&db);
    let f = foco("acoes", &["acoes"], "", None);
    let linhas = |db: &Db| {
        let c = db.contexto_aporte(hoje(), &f).unwrap();
        bloco(&c.texto)
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let ligada = linhas(&db);
    assert_eq!(
        ligada[2],
        "Classe em foco: Ações, com R$ 1.100,00 deste aporte pela divisão de rebalanceamento (alvo 30%, hoje 8,0 p.p. abaixo do alvo)."
    );

    db.salvar_parte_contexto("carteira", false, None).unwrap();
    let desligada = linhas(&db);
    assert_eq!(desligada[1], "Valor deste aporte: R$ 2.000,00.");
    assert_eq!(
        desligada[2],
        "Classe em foco: Ações, com R$ 1.100,00 deste aporte pela divisão de rebalanceamento (alvo 30%)."
    );
    assert_eq!(
        desligada[3],
        "Divisão do aporte: Renda fixa 0,0%; Ações 55,0%; FIIs 15,0%; Exterior 30,0%."
    );
    let texto = desligada.join("\n");
    assert!(!texto.contains("p.p."), "{texto}");
    assert!(!texto.contains("hoje "), "{texto}");
    // O resto do bloco não muda.
    assert_eq!(ligada[3..], desligada[3..]);
}

#[test]
fn aporte_nao_leva_valores_da_carteira() {
    let (_d, db) = banco();
    salvar_alvo(&db);
    criar(&db, &cdb(900_000));
    assert!(db.config_ia().unwrap().apenas_percentuais);
    let total = db.resumo_carteira(hoje()).unwrap().total_atual_centavos;
    let c = db
        .contexto_aporte(hoje(), &foco("acoes", &["acoes"], "", None))
        .unwrap();
    let b = bloco(&c.texto);
    assert!(b.contains("R$ 2.000,00"), "{b}");
    for proibido in [
        "9.000".to_string(),
        guru_core::formatar_reais(total),
        guru_core::formatar_reais(total + 200_000),
    ] {
        assert!(!b.contains(&proibido), "vazou {proibido}: {b}");
    }
    assert!(!c.texto.contains("9.000"), "{}", c.texto);
}

/// Conta as chamadas e repassa ao fixture.
struct Contador {
    dentro: ClienteFixture,
    corpos: Mutex<Vec<Value>>,
}

impl Contador {
    fn novo() -> Self {
        Self {
            dentro: ClienteFixture {
                pasta: PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ia")),
            },
            corpos: Mutex::new(Vec::new()),
        }
    }
    fn corpos(&self) -> Vec<Value> {
        self.corpos.lock().unwrap().clone()
    }
}

impl ClienteIa for Contador {
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

fn pergunta_aporte(
    db: &Db,
    cliente: &dyn ClienteIa,
    f: &FocoAporte,
    texto: &str,
) -> Result<i64, CoreError> {
    let cancelar = AtomicBool::new(false);
    perguntar_aporte(
        db,
        &cofre_com_chave(),
        cliente,
        hoje(),
        f,
        texto,
        &cancelar,
        &mut |_| {},
    )
}

#[test]
fn foco_invalido() {
    let cliente = Contador::novo();
    let confere = |db: &Db, f: FocoAporte, msg: &str| {
        let e = pergunta_aporte(db, &cliente, &f, "Oi").unwrap_err();
        assert_eq!(e.code(), "IA_FOCO_INVALIDO", "{msg}");
        assert_eq!(e.to_string(), msg);
        // A conta direta também recusa.
        let e = db.contexto_aporte(hoje(), &f).unwrap_err();
        assert_eq!(e.code(), "IA_FOCO_INVALIDO", "{msg}");
        assert_eq!(e.to_string(), msg);
    };

    let (_d, sem_alvo) = banco();
    confere(
        &sem_alvo,
        foco("acoes", &[], "", None),
        "Defina a sua alocação-alvo antes de conversar sobre o aporte.",
    );

    let (_d, db) = banco();
    carteira_exemplo(&db);
    let mut f = foco("acoes", &[], "", None);
    f.valor_centavos = 99;
    confere(&db, f, "Informe um valor de pelo menos R$ 1,00.");
    // Outros: alvo 0 e sem posição, fora da divisão; um código desconhecido também.
    for classe in ["outros", "cripto"] {
        confere(
            &db,
            foco(classe, &[], "", None),
            "Esta classe não está na divisão do aporte.",
        );
    }
    confere(
        &db,
        foco("acoes", &["acoes", "cripto"], "", None),
        "Direção desconhecida.",
    );
    confere(
        &db,
        foco("acoes", &[], &"a".repeat(301), None),
        "Use no máximo 300 caracteres.",
    );
    confere(
        &db,
        foco("acoes", &[], "", Some(9999)),
        "Esta meta não existe mais. Atualize a lista.",
    );
    assert_eq!(cliente.corpos().len(), 0, "nenhuma chamada à API");
    assert!(db.listar_conversas().unwrap().is_empty());

    // 300 caracteres ainda cabem.
    assert!(
        db.contexto_aporte(hoje(), &foco("acoes", &[], &"á".repeat(300), None))
            .is_ok()
    );
}

#[test]
fn conversa_do_aporte_tem_foco_e_aparece_na_lista() {
    let (_d, db) = banco();
    let meta = carteira_exemplo(&db);
    let cliente = Contador::novo();
    let assistente = perguntar(
        &db,
        &cofre_com_chave(),
        &cliente,
        hoje(),
        None,
        "Oi",
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    let pergunta = "Quais investimentos de Ações combinam com a direção que escolhi?";
    let id = pergunta_aporte(
        &db,
        &cliente,
        &foco("acoes", &["acoes"], "", Some(meta)),
        pergunta,
    )
    .unwrap();

    let lista = db.listar_conversas().unwrap();
    assert_eq!(lista.len(), 2);
    assert_eq!(lista[0].id, id);
    assert_eq!(lista[0].foco, Some(Classe::Acoes));
    assert_eq!(
        lista[0].titulo,
        pergunta.chars().take(60).collect::<String>()
    );
    assert_eq!(lista[1].id, assistente);
    assert_eq!(lista[1].foco, None);

    let c = db.conversa(id).unwrap();
    assert_eq!(c.foco, Some(Classe::Acoes));
    assert_eq!(c.mensagens.len(), 2);
    assert_eq!(c.fontes_internas.last().unwrap().parte, "aporte");
    assert_eq!(db.conversa(assistente).unwrap().foco, None);

    // O contexto enviado é o do aporte, e o uso da resposta boa entra no mês.
    let corpos = cliente.corpos();
    let system = corpos[1]["system"][1]["text"].as_str().unwrap();
    assert!(system.contains("## Próximo aporte"));
    assert_eq!(db.uso_do_mes(hoje()).unwrap().entrada, 2 * 1200);

    // Continuar a conversa (pelo Assistente) não muda o foco.
    perguntar(
        &db,
        &cofre_com_chave(),
        &cliente,
        hoje(),
        Some(id),
        "E os custos?",
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(db.conversa(id).unwrap().foco, Some(Classe::Acoes));
    assert_eq!(db.conversa(id).unwrap().mensagens.len(), 4);
}

#[test]
fn segunda_pergunta_reusa_o_contexto_do_aporte() {
    let (_d, db) = banco();
    let meta = carteira_exemplo(&db);
    let cliente = Contador::novo();
    let id = pergunta_aporte(
        &db,
        &cliente,
        &foco("acoes", &["novo"], "", Some(meta)),
        "Compare duas opções para esse valor, com prós e contras.",
    )
    .unwrap();

    // A meta muda depois da primeira pergunta: o contexto congelado não pode mudar.
    db.atualizar_meta(
        meta,
        &NovaMeta {
            nome: "Casa na praia".into(),
            valor_alvo_centavos: Some(400_000),
            prazo: s("2031-06"),
        },
        hoje(),
    )
    .unwrap()
    .unwrap();

    let r = perguntar(
        &db,
        &cofre_com_chave(),
        &cliente,
        hoje(),
        Some(id),
        "E os impostos?",
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(r, id);
    let corpos = cliente.corpos();
    assert_eq!(corpos.len(), 2);
    assert_eq!(
        serde_json::to_string(&corpos[1]["system"]).unwrap(),
        serde_json::to_string(&corpos[0]["system"]).unwrap()
    );
    assert!(
        corpos[1]["system"][1]["text"]
            .as_str()
            .unwrap()
            .contains("Entrada do apartamento")
    );
}
