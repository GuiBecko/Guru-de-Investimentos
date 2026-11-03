use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use chrono::NaiveDate;
use guru_core::{
    Acumulador, ClienteFixture, ClienteIa, Cofre, CofreMemoria, CoreError, Db, ErroIa, Fonte,
    Gravacao, Insight, Noticia, NovoInvestimento, buscar_noticias, corpo_insights, gerar_insights,
    ler_insights, ler_noticias, rotulo_insight,
};
use serde_json::{Value, json};

const CHAVE: &str = "sk-ant-api03-SEGREDO-de-teste-7c1e9b4f2a";

const SISTEMA: &str = "Você é o assistente do Guru de Investimentos, um app brasileiro de carteira pessoal. Responda em português do Brasil, com clareza e sem jargão desnecessário. Apresente opções, com prós e contras, e deixe a decisão com o usuário: não faça recomendação individual de investimento nem diga o que ele deve comprar ou vender. Use o contexto enviado sobre a carteira, o perfil, as metas e o mercado. Para fatos de mercado recentes (decisões do Copom, projeções, notícias), busque na web quando a busca estiver disponível e cite a fonte. Quando algo for opinião sua, sem fonte, diga que é opinião. Se faltar um dado para responder bem, diga qual.";

const PEDIDO_INSIGHTS: &str = "Analise o contexto e escreva de 3 a 5 insights curtos sobre a carteira, em português do Brasil, cada um com no máximo 300 caracteres. Use os tipos rebalanceamento (desvio da alocação-alvo), meta (ritmo ou atraso das metas), vencimento (o que vence nos próximos 12 meses), concentracao (peso alto num ativo, setor ou instituição) e mercado (o que os dados de mercado e as notícias do contexto significam para a carteira). Para cada um, escolha a ação que leva ao lugar certo do app: aporte, meta, classe (com o código da classe), conversar ou nenhuma. Apresente fatos e opções, sem recomendar a compra ou a venda de um ativo específico. Use só os dados do contexto.";

const PEDIDO_NOTICIAS: &str = "Busque na web notícias dos últimos 7 dias sobre os ativos listados no contexto e sobre os indicadores que os afetam (Selic, IPCA, câmbio). Escolha até 8 notícias relevantes, de veículos jornalísticos ou fontes oficiais. Responda só com uma linha por notícia, no formato `- <ativo ou classe> | <título em português, com até 100 caracteres>`, citando a matéria em cada linha. Não escreva mais nada.";

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["insights"],
     "properties":{"insights":{"type":"array","items":{"type":"object","additionalProperties":false,
       "required":["tipo","texto","acao","classe"],
       "properties":{
         "tipo":{"type":"string","enum":["rebalanceamento","meta","vencimento","concentracao","mercado"]},
         "texto":{"type":"string"},
         "acao":{"type":"string","enum":["aporte","meta","classe","conversar","nenhuma"]},
         "classe":{"anyOf":[{"type":"string","enum":["rf","acoes","fii","ext","outros"]},{"type":"null"}]}}}}}})
}

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

fn pasta() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ia"))
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

fn criar(db: &Db, n: &NovoInvestimento) {
    match db.criar_investimento(n, hoje()).unwrap() {
        Gravacao::Ok(_) => {}
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn cdb() -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: "Banco Fictício".into(),
        emissor: s("Emissor Exemplo"),
        indexador: s("pos"),
        taxa: s("102"),
        benchmark: s("cdi"),
        data_aplicacao: s("2026-04-15"),
        vencimento: s("2028-03-15"),
        valor_aplicado_centavos: Some(900_000),
        liquidez: s("vencimento"),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn rv(tipo: &str, ticker: &str) -> NovoInvestimento {
    NovoInvestimento {
        tipo: tipo.into(),
        instituicao: "Corretora Teste".into(),
        ticker: s(ticker),
        data_aplicacao: s("2026-01-10"),
        quantidade: Some(100),
        preco_medio_centavos: Some(2_000),
        ..Default::default()
    }
}

/// CDB do Banco Fictício (R$ 9.000,00), ITUB4, WEGE3 e HGLG11.
fn carteira(db: &Db) {
    criar(db, &cdb());
    criar(db, &rv("rv-acao", "ITUB4"));
    criar(db, &rv("rv-acao", "WEGE3"));
    criar(db, &rv("rv-fii", "HGLG11"));
}

fn configurar(db: &Db, instrucoes: &str, nomes: bool, busca: bool) {
    let r = db
        .salvar_config_ia("claude-opus-5-5", instrucoes, true, nomes, busca)
        .unwrap();
    assert!(r.erros.is_empty(), "{:?}", r.erros);
}

/// Cliente de teste que registra os corpos e repassa ao `ClienteFixture`.
struct Gravador {
    dentro: ClienteFixture,
    corpos: Mutex<Vec<Value>>,
}

impl Gravador {
    fn novo() -> Self {
        Self {
            dentro: ClienteFixture { pasta: pasta() },
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

fn insights(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa) -> Result<(), CoreError> {
    gerar_insights(
        db,
        cofre,
        cliente,
        hoje(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
}

fn noticias(db: &Db, cofre: &dyn Cofre, cliente: &dyn ClienteIa) -> Result<(), CoreError> {
    buscar_noticias(
        db,
        cofre,
        cliente,
        hoje(),
        &AtomicBool::new(false),
        &mut |_| {},
    )
}

/// As fontes que o acumulador tira do fixture, como `buscar_noticias` recebe.
fn fontes_do_fixture(nome: &str) -> (Vec<Value>, Vec<Fonte>) {
    let texto = std::fs::read_to_string(pasta().join(nome)).unwrap();
    let mut a = Acumulador::novo();
    for linha in texto.lines() {
        if let Some(d) = linha.strip_prefix("data: ") {
            a.evento(&serde_json::from_str(d).unwrap()).unwrap();
        }
    }
    (a.blocos().to_vec(), a.progresso().fontes)
}

fn json_insights(itens: Value) -> String {
    json!({ "insights": itens }).to_string()
}

fn item(tipo: &str, texto: &str, acao: &str, classe: Option<&str>) -> Value {
    json!({"tipo": tipo, "texto": texto, "acao": acao, "classe": classe})
}

#[test]
fn corpo_insights_tem_json_schema_e_nenhuma_ferramenta() {
    let (c, beta) = corpo_insights("claude-opus-5-5", "CONTEXTO");
    assert_eq!(
        c["output_config"],
        json!({"effort": "medium", "format": {"type": "json_schema", "schema": schema()}})
    );
    assert!(c.get("tools").is_none());
    assert_eq!(
        c["messages"],
        json!([{"role": "user", "content": PEDIDO_INSIGHTS}])
    );
    assert_eq!(
        c["system"],
        json!([
            {"type": "text", "text": SISTEMA},
            {"type": "text", "text": "CONTEXTO", "cache_control": {"type": "ephemeral"}}
        ])
    );
    for proibido in ["temperature", "top_p", "top_k", "thinking"] {
        assert!(c.get(proibido).is_none(), "não deveria ter {proibido}");
    }
    assert_eq!(c["model"], "claude-opus-5-5");
    assert_eq!(c["max_tokens"], 32000);
    assert_eq!(c["stream"], true);
    assert_eq!(c["fallbacks"], "default");
    assert_eq!(beta, Some("server-side-fallback-2026-07-01"));

    let (c, beta) = corpo_insights("claude-sonnet-5-5", "CONTEXTO");
    assert_eq!(c["fallbacks"], "default");
    assert_eq!(beta, Some("server-side-fallback-2026-07-01"));
    let (c, beta) = corpo_insights("claude-haiku-5-5", "CONTEXTO");
    assert!(c.get("fallbacks").is_none());
    assert!(c.get("tools").is_none());
    assert_eq!(beta, None);

    // Pela geração, com a busca ligada (padrão): continua sem `tools`, e o contexto é o da 6a.
    let (_d, db) = banco();
    carteira(&db);
    assert!(db.config_ia().unwrap().busca_web);
    let g = Gravador::novo();
    insights(&db, &cofre_com_chave(), &g).unwrap();
    let corpos = g.corpos();
    assert_eq!(corpos.len(), 1);
    assert!(corpos[0].get("tools").is_none());
    assert_eq!(
        corpos[0],
        corpo_insights(
            "claude-opus-5-5",
            &db.contexto_montado(hoje()).unwrap().texto
        )
        .0
    );
}

#[test]
fn corpo_noticias_tem_busca_e_nao_tem_format() {
    let (_d, db) = banco();
    carteira(&db);
    let g = Gravador::novo();
    noticias(&db, &cofre_com_chave(), &g).unwrap();
    let corpos = g.corpos();
    assert_eq!(corpos.len(), 1);
    let c = &corpos[0];
    let contexto = db.contexto_noticias(hoje()).unwrap();
    let msgs = vec![json!({"role": "user", "content": PEDIDO_NOTICIAS})];
    assert_eq!(
        *c,
        guru_core::corpo_pedido("claude-opus-5-5", &contexto, true, &msgs).0
    );
    assert_eq!(c["output_config"], json!({"effort": "medium"}));
    assert_eq!(c["tools"][0]["type"], "web_search_20260209");
    assert_eq!(c["messages"], json!(msgs));
}

#[test]
fn gerar_insights_grava_cinco_e_o_uso() {
    let (_d, db) = banco();
    carteira(&db);
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 0);
    assert!(db.painel_ia().unwrap().insights.is_none());
    let mut vistos = 0;
    gerar_insights(
        &db,
        &cofre_com_chave(),
        &ClienteFixture { pasta: pasta() },
        hoje(),
        &AtomicBool::new(false),
        &mut |_| vistos += 1,
    )
    .unwrap();
    assert!(vistos > 3, "o progresso deveria chegar aos poucos");

    let g = db.painel_ia().unwrap().insights.expect("sem insights");
    assert_eq!(g.gerado_em, hoje());
    assert_eq!(g.modelo, "claude-opus-5-5");
    assert_eq!(g.itens.len(), 5);
    let rotulos: Vec<&str> = g.itens.iter().map(|i| rotulo_insight(&i.tipo)).collect();
    assert_eq!(
        rotulos,
        [
            "Rebalanceamento",
            "Meta",
            "Vencimento",
            "Concentração",
            "Mercado"
        ]
    );
    let acoes: Vec<(&str, Option<&str>)> = g
        .itens
        .iter()
        .map(|i| (i.acao.as_str(), i.classe.as_deref()))
        .collect();
    assert_eq!(
        acoes,
        [
            ("aporte", None),
            ("meta", None),
            ("conversar", None),
            ("classe", Some("acoes")),
            ("nenhuma", None)
        ]
    );
    assert!(g.itens.iter().all(|i| !i.texto.is_empty()));
    // input 3000 + cache 1000 + saída 400.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 4400);
    // As notícias não mudam.
    assert!(db.painel_ia().unwrap().noticias.is_none());
}

#[test]
fn insights_invalidos_mantem_os_anteriores() {
    let (_d, db) = banco();
    carteira(&db);
    let cofre = cofre_com_chave();
    let cliente = ClienteFixture { pasta: pasta() };
    insights(&db, &cofre, &cliente).unwrap();
    let antes = db.painel_ia().unwrap().insights.unwrap();

    configurar(&db, "#insights-invalido", false, true);
    let e = insights(&db, &cofre, &cliente).unwrap_err();
    assert_eq!(e.code(), "IA_RESPOSTA_INVALIDA");
    assert_eq!(
        e.to_string(),
        "A IA devolveu os insights num formato inesperado. Tente de novo."
    );
    let depois = db.painel_ia().unwrap().insights.unwrap();
    assert_eq!(depois, antes);
    assert_eq!(depois.itens.len(), 5);
    // A resposta inválida já foi paga: os tokens dela também contam.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 8800);
}

#[test]
fn ler_insights_recusa_fora_do_esquema() {
    let bom = || {
        vec![
            item("rebalanceamento", "Texto um.", "aporte", None),
            item("meta", "Texto dois.", "meta", None),
            item("concentracao", "Texto três.", "classe", Some("acoes")),
        ]
    };
    let ok = ler_insights(&json_insights(json!(bom())), Some("end_turn")).unwrap();
    assert_eq!(
        ok[2],
        Insight {
            tipo: "concentracao".into(),
            texto: "Texto três.".into(),
            acao: "classe".into(),
            classe: Some("acoes".into()),
        }
    );

    let invalido = |json: String, stop: Option<&str>, caso: &str| {
        let r = ler_insights(&json, stop);
        assert_eq!(
            r.as_ref().err(),
            Some(&ErroIa::RespostaInvalida),
            "caso {caso}: {r:?}"
        );
    };
    invalido("não é JSON".into(), None, "não é JSON");
    invalido("{\"insights\": [".into(), None, "JSON cortado");
    invalido("{}".into(), None, "sem a lista");
    invalido(
        json!({"insights": "x"}).to_string(),
        None,
        "lista não é lista",
    );
    for campo in ["tipo", "texto", "acao", "classe"] {
        let mut l = bom();
        l[1].as_object_mut().unwrap().remove(campo);
        invalido(json_insights(json!(l)), None, &format!("falta {campo}"));
    }
    let mut l = bom();
    l[0]["tipo"] = json!("outro");
    invalido(json_insights(json!(l)), None, "tipo desconhecido");
    let mut l = bom();
    l[0]["acao"] = json!("comprar");
    invalido(json_insights(json!(l)), None, "ação desconhecida");
    let mut l = bom();
    l[2]["classe"] = json!("cripto");
    invalido(json_insights(json!(l)), None, "classe desconhecida");
    let mut l = bom();
    l[0]["texto"] = json!(7);
    invalido(json_insights(json!(l)), None, "texto não é texto");
    invalido(json_insights(json!([])), None, "lista vazia");
    let mut l = bom();
    l[0]["texto"] = json!("   \n ");
    invalido(json_insights(json!(l)), None, "texto vazio");
    let mut l = bom();
    l[0]["texto"] = json!("á".repeat(501));
    invalido(json_insights(json!(l)), None, "texto com 501");
    let mut l = bom();
    l[2]["classe"] = Value::Null;
    invalido(json_insights(json!(l)), None, "ação classe sem classe");
    invalido(
        json_insights(json!(bom())),
        Some("max_tokens"),
        "max_tokens",
    );

    // 500 caracteres ainda passa, e o texto vai sem os espaços das pontas.
    let mut l = bom();
    l[0]["texto"] = json!(format!("  {}  ", "á".repeat(500)));
    let r = ler_insights(&json_insights(json!(l)), None).unwrap();
    assert_eq!(r[0].texto, "á".repeat(500));

    // 6 viram 5: ficam os 5 primeiros.
    let seis: Vec<Value> = (1..=6)
        .map(|n| item("mercado", &format!("Insight {n}."), "nenhuma", None))
        .collect();
    let r = ler_insights(&json_insights(json!(seis)), Some("end_turn")).unwrap();
    assert_eq!(r.len(), 5);
    assert_eq!(r[4].texto, "Insight 5.");
}

#[test]
fn rotulos_dos_tipos() {
    for (tipo, rotulo) in [
        ("rebalanceamento", "Rebalanceamento"),
        ("meta", "Meta"),
        ("vencimento", "Vencimento"),
        ("concentracao", "Concentração"),
        ("mercado", "Mercado"),
    ] {
        assert_eq!(rotulo_insight(tipo), rotulo);
    }
}

#[test]
fn noticias_so_com_link_https_citado() {
    let (_d, db) = banco();
    carteira(&db);
    noticias(&db, &cofre_com_chave(), &ClienteFixture { pasta: pasta() }).unwrap();
    let g = db.painel_ia().unwrap().noticias.expect("sem notícias");
    assert_eq!(g.gerado_em, hoje());
    assert_eq!(g.modelo, "claude-opus-5-5");
    assert_eq!(g.itens.len(), 6, "{:#?}", g.itens);

    let (_, fontes) = fontes_do_fixture("noticias.sse");
    assert!(fontes.iter().any(|f| f.url.starts_with("http://")));
    for n in &g.itens {
        assert!(n.url.starts_with("https://"), "{}", n.url);
        let f = fontes
            .iter()
            .find(|f| f.url == n.url)
            .expect("a URL deveria ser de uma fonte citada");
        assert_eq!(n.dominio, f.dominio);
        assert_eq!(n.data, f.data);
    }
    let etiquetas: Vec<&str> = g.itens.iter().map(|n| n.etiqueta.as_str()).collect();
    assert_eq!(
        etiquetas,
        ["ITUB4", "Renda fixa", "HGLG11", "WEGE3", "Mercado", "Ações"]
    );
    assert_eq!(
        g.itens[0],
        Noticia {
            titulo: "Itaú Unibanco tem lucro recorde no terceiro trimestre".into(),
            url: "https://www.infomoney.com.br/mercados/itau-lucro-3t26".into(),
            dominio: "infomoney.com.br".into(),
            data: Some("October 29, 2026".into()),
            etiqueta: "ITUB4".into(),
        }
    );
    // A fonte sem `page_age` fica sem data.
    assert_eq!(g.itens[3].data, None);
    assert!(g.itens.iter().all(|n| !n.titulo.contains("Selic")));
    assert!(g.itens.iter().all(|n| !n.titulo.contains("Inflação")));
    // A busca conta no uso: 2500 + 600.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 3100);
}

fn bloco_texto(texto: &str, url: Option<&str>) -> Value {
    let mut b = json!({"type": "text", "text": texto});
    if let Some(u) = url {
        b["citations"] = json!([
            {"type": "char_location", "cited_text": "x", "document_index": 0},
            {"type": "web_search_result_location", "cited_text": "x", "url": u, "title": "T"}
        ]);
    }
    b
}

fn fonte(numero: u32, url: &str, data: Option<&str>) -> Fonte {
    Fonte {
        numero,
        url: url.into(),
        titulo: "T".into(),
        dominio: "exemplo.com.br".into(),
        data: data.map(str::to_string),
        trecho: "x".into(),
    }
}

#[test]
fn etiqueta_desconhecida_vira_mercado() {
    let (_d, db) = banco();
    carteira(&db);
    noticias(&db, &cofre_com_chave(), &ClienteFixture { pasta: pasta() }).unwrap();
    let g = db.painel_ia().unwrap().noticias.unwrap();
    let xpto = g
        .itens
        .iter()
        .find(|n| n.titulo == "XPTO conclui oferta de ações")
        .unwrap();
    assert_eq!(xpto.etiqueta, "Mercado");

    // Sem diferença de maiúsculas, com a etiqueta como está na lista; `[n]` sai do título;
    // URL repetida conta uma vez; título cortado em 100.
    let etiquetas = vec!["ITUB4".to_string(), "Renda fixa".to_string()];
    let longo = "a".repeat(130);
    let blocos = vec![
        bloco_texto("- itub4 | ", None),
        bloco_texto("Itaú divulga balanço [1]", Some("https://exemplo.com.br/1")),
        bloco_texto(
            "\n-RENDA FIXA|Outra matéria",
            Some("https://exemplo.com.br/1"),
        ),
        bloco_texto(
            &format!("\n- Ouro | {longo}"),
            Some("https://exemplo.com.br/2"),
        ),
        bloco_texto("\nTexto solto", Some("https://exemplo.com.br/3")),
        bloco_texto(
            "\n- Selic | Sem fonte conhecida",
            Some("https://outro.com/x"),
        ),
    ];
    let fontes = vec![
        fonte(1, "https://exemplo.com.br/1", Some("1 day ago")),
        fonte(2, "https://exemplo.com.br/2", None),
        fonte(3, "https://exemplo.com.br/3", None),
    ];
    let r = ler_noticias(&blocos, &fontes, &etiquetas);
    assert_eq!(r.len(), 2, "{r:#?}");
    assert_eq!(r[0].etiqueta, "ITUB4");
    assert_eq!(r[0].titulo, "Itaú divulga balanço");
    assert_eq!(r[0].data.as_deref(), Some("1 day ago"));
    assert_eq!(r[1].etiqueta, "Mercado");
    assert_eq!(r[1].titulo.chars().count(), 100);

    // No máximo 8.
    let muitos: Vec<Value> = (0..10)
        .map(|n| {
            bloco_texto(
                &format!("- ITUB4 | Notícia {n}\n"),
                Some(&format!("https://exemplo.com.br/n{n}")),
            )
        })
        .collect();
    let fontes: Vec<Fonte> = (0..10)
        .map(|n| fonte(n + 1, &format!("https://exemplo.com.br/n{n}"), None))
        .collect();
    let r = ler_noticias(&muitos, &fontes, &etiquetas);
    assert_eq!(r.len(), 8);
    assert_eq!(r[7].titulo, "Notícia 7");
}

#[test]
fn noticias_vazias_gravam_lista_vazia() {
    let (_d, db) = banco();
    carteira(&db);
    let cofre = cofre_com_chave();
    let cliente = ClienteFixture { pasta: pasta() };
    noticias(&db, &cofre, &cliente).unwrap();
    configurar(&db, "#noticias-vazias", false, true);
    noticias(&db, &cofre, &cliente).unwrap();
    let g = db.painel_ia().unwrap().noticias.unwrap();
    assert!(g.itens.is_empty());
    assert_eq!(g.gerado_em, hoje());
    // Sem notícias guardadas, "Mercado" volta ao resumo de antes.
    let mercado = db
        .partes_contexto(hoje())
        .unwrap()
        .into_iter()
        .find(|p| p.parte == "mercado")
        .unwrap();
    assert_eq!(mercado.resumo, "automático");
    assert!(!mercado.texto.contains("Notícias da carteira"));
}

#[test]
fn contexto_noticias_respeita_privacidade() {
    let (_d, db) = banco();
    criar(&db, &cdb());
    criar(&db, &rv("rv-acao", "ITUB4"));
    let nome = db.listar_investimentos(hoje()).unwrap()[0].nome.clone();
    configurar(&db, "Gosto de dividendos.", false, true);

    let corpo = |db: &Db| {
        let g = Gravador::novo();
        noticias(db, &cofre_com_chave(), &g).unwrap();
        g.corpos()[0].to_string()
    };

    let c = db.contexto_noticias(hoje()).unwrap();
    assert_eq!(
        c,
        "## Ativos da carteira\nRenda fixa: CDB 102% do CDI.\nAções: ITUB4.\n\n\
         ## Suas instruções\nGosto de dividendos.\n\nHoje é 30/10/2026."
    );
    let enviado = corpo(&db);
    for proibido in [
        "R$",
        "9.000",
        "Banco Fictício",
        "Emissor Exemplo",
        "Corretora Teste",
        nome.as_str(),
    ] {
        assert!(!enviado.contains(proibido), "vazou {proibido}: {enviado}");
    }
    assert!(enviado.contains("ITUB4"));

    // Instruções desligadas ficam de fora.
    db.salvar_parte_contexto("instrucoes", false, None).unwrap();
    assert!(
        !db.contexto_noticias(hoje())
            .unwrap()
            .contains("Suas instruções")
    );

    // Carteira desligada: só as classes.
    db.salvar_parte_contexto("carteira", false, None).unwrap();
    let c = db.contexto_noticias(hoje()).unwrap();
    assert_eq!(
        c,
        "## Ativos da carteira\nClasses na carteira: Renda fixa, Ações.\n\nHoje é 30/10/2026."
    );
    assert!(!corpo(&db).contains("ITUB4"));

    // Com "Nome das instituições", a instituição e o emissor aparecem.
    db.salvar_parte_contexto("carteira", true, None).unwrap();
    configurar(&db, "", true, true);
    let c = db.contexto_noticias(hoje()).unwrap();
    assert!(c.contains("Banco Fictício"), "{c}");
    assert!(c.contains("Emissor Exemplo"), "{c}");
    assert!(!c.contains("R$"));
    assert!(
        !c.contains("Suas instruções"),
        "instruções vazias ficam de fora"
    );
}

#[test]
fn busca_desligada_recusa_noticias() {
    let (_d, db) = banco();
    carteira(&db);
    configurar(&db, "", false, false);
    let g = Gravador::novo();
    let e = noticias(&db, &cofre_com_chave(), &g).unwrap_err();
    assert_eq!(e.code(), "IA_BUSCA_DESLIGADA");
    assert_eq!(
        e.to_string(),
        "Ligue “Buscar na web” em Configurar IA para buscar notícias."
    );
    assert!(g.corpos().is_empty());
    assert!(db.painel_ia().unwrap().noticias.is_none());
}

#[test]
fn sem_chave_nao_chama_a_api() {
    let (_d, db) = banco();
    carteira(&db);
    let g = Gravador::novo();
    let cofre = CofreMemoria::default();
    assert_eq!(
        insights(&db, &cofre, &g).unwrap_err().code(),
        "IA_SEM_CHAVE"
    );
    assert_eq!(
        noticias(&db, &cofre, &g).unwrap_err().code(),
        "IA_SEM_CHAVE"
    );
    assert!(g.corpos().is_empty());
    let p = db.painel_ia().unwrap();
    assert!(p.insights.is_none() && p.noticias.is_none());
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, 0);
}

#[test]
fn erro_mantem_o_cache() {
    let (_d, db) = banco();
    carteira(&db);
    let cofre = cofre_com_chave();
    let cliente = ClienteFixture { pasta: pasta() };
    noticias(&db, &cofre, &cliente).unwrap();
    insights(&db, &cofre, &cliente).unwrap();
    let antes = db.painel_ia().unwrap();
    let uso = db.uso_do_mes(hoje()).unwrap().tokens;

    configurar(&db, "#erro529", false, true);
    assert_eq!(
        noticias(&db, &cofre, &cliente).unwrap_err().code(),
        "IA_INDISPONIVEL"
    );
    assert_eq!(
        insights(&db, &cofre, &cliente).unwrap_err().code(),
        "IA_INDISPONIVEL"
    );
    configurar(&db, "#erro401", false, true);
    assert_eq!(
        noticias(&db, &cofre, &cliente).unwrap_err().code(),
        "IA_CHAVE_RECUSADA"
    );
    assert_eq!(db.painel_ia().unwrap(), antes);
    // Erro antes do `message_start` não soma nada.
    assert_eq!(db.uso_do_mes(hoje()).unwrap().tokens, uso);
}

#[test]
fn mercado_leva_as_3_noticias_mais_recentes() {
    let (_d, db) = banco();
    carteira(&db);
    let mercado = |db: &Db| {
        db.partes_contexto(hoje())
            .unwrap()
            .into_iter()
            .find(|p| p.parte == "mercado")
            .unwrap()
    };
    let antes = mercado(&db);
    assert_eq!(antes.resumo, "automático");

    noticias(&db, &cofre_com_chave(), &ClienteFixture { pasta: pasta() }).unwrap();
    let m = mercado(&db);
    let fim = "\nNotícias da carteira (buscadas em 30/10/2026):\n\
        - ITUB4: Itaú Unibanco tem lucro recorde no terceiro trimestre (infomoney.com.br, October 29, 2026)\n\
        - Renda fixa: Tesouro Direto bate recorde de investidores em outubro (valor.globo.com, October 28, 2026)\n\
        - HGLG11: HGLG11 anuncia dividendo de novembro (fiis.com.br, October 27, 2026)";
    assert!(m.texto.ends_with(fim), "{}", m.texto);
    assert_eq!(m.texto, format!("{}{fim}", antes.texto));
    assert_eq!(m.resumo, "automático · 3 notícias");
    // E vai no contexto dos insights.
    assert!(
        db.contexto_montado(hoje())
            .unwrap()
            .texto
            .contains("Notícias da carteira (buscadas em 30/10/2026):")
    );
}

#[test]
fn carteira_desligada_tira_as_noticias_de_ticker_do_mercado() {
    let (_d, db) = banco();
    carteira(&db);
    noticias(&db, &cofre_com_chave(), &ClienteFixture { pasta: pasta() }).unwrap();
    let mercado = |db: &Db| {
        db.partes_contexto(hoje())
            .unwrap()
            .into_iter()
            .find(|p| p.parte == "mercado")
            .unwrap()
    };
    // Carteira ligada: as 3 primeiras, com ticker ou não.
    let m = mercado(&db);
    assert!(m.texto.contains("- ITUB4: Itaú Unibanco"), "{}", m.texto);
    assert!(
        m.texto.contains("- Renda fixa: Tesouro Direto"),
        "{}",
        m.texto
    );

    // Carteira desligada: as de ticker saem, as de classe ou "Mercado" ficam.
    db.salvar_parte_contexto("carteira", false, None).unwrap();
    let m = mercado(&db);
    for ticker in ["ITUB4", "WEGE3", "HGLG11"] {
        assert!(!m.texto.contains(ticker), "vazou {ticker}: {}", m.texto);
    }
    let fim = "\nNotícias da carteira (buscadas em 30/10/2026):\n\
        - Renda fixa: Tesouro Direto bate recorde de investidores em outubro (valor.globo.com, October 28, 2026)\n\
        - Mercado: XPTO conclui oferta de ações";
    assert!(m.texto.contains(fim), "{}", m.texto);
    assert!(m.texto.contains("\n- Ações: "), "{}", m.texto);
    assert_eq!(m.resumo, "automático · 3 notícias");
    let enviado = db.contexto_montado(hoje()).unwrap().texto;
    assert!(!enviado.contains("ITUB4"), "{enviado}");
    assert!(
        enviado.contains("- Renda fixa: Tesouro Direto"),
        "{enviado}"
    );

    // Religada, tudo volta.
    db.salvar_parte_contexto("carteira", true, None).unwrap();
    let m = mercado(&db);
    assert!(m.texto.contains("- ITUB4: Itaú Unibanco"), "{}", m.texto);
    assert!(
        m.texto.contains("- Renda fixa: Tesouro Direto"),
        "{}",
        m.texto
    );
}

/// Uma busca que não terminou bem (`refusal`, `max_tokens` ou `pause_turn` depois das 3
/// continuações) não troca as notícias guardadas, mas os tokens dela contam.
fn interrompida_mantem_as_anteriores(marca: &str, codigo: &str, chamadas: usize, saida: i64) {
    let (_d, db) = banco();
    carteira(&db);
    let cofre = cofre_com_chave();
    noticias(&db, &cofre, &ClienteFixture { pasta: pasta() }).unwrap();
    let antes = db.painel_ia().unwrap();
    assert_eq!(antes.noticias.as_ref().unwrap().itens.len(), 6);
    let uso = db.uso_do_mes(hoje()).unwrap().tokens;

    configurar(&db, marca, false, true);
    let g = Gravador::novo();
    let e = noticias(&db, &cofre, &g).unwrap_err();
    assert_eq!(e.code(), codigo, "{marca}");
    assert_eq!(g.corpos().len(), chamadas, "{marca}");
    assert_eq!(db.painel_ia().unwrap(), antes, "{marca}");
    assert_eq!(
        db.uso_do_mes(hoje()).unwrap().tokens,
        uso + saida,
        "{marca}"
    );
}

#[test]
fn noticias_recusadas_mantem_as_anteriores() {
    interrompida_mantem_as_anteriores("#noticias-recusa", "IA_RECUSA", 1, 2800);
}

#[test]
fn noticias_cortadas_em_max_tokens_mantem_as_anteriores() {
    interrompida_mantem_as_anteriores("#noticias-max-tokens", "IA_INDISPONIVEL", 1, 2800);
}

#[test]
fn noticias_ainda_pausadas_mantem_as_anteriores() {
    // A primeira parte e as 3 continuações.
    interrompida_mantem_as_anteriores("#noticias-pausa", "IA_INDISPONIVEL", 4, 4 * 2800);
}
