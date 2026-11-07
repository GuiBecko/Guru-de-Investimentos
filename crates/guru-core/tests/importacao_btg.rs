use std::str::FromStr;

use chrono::NaiveDate;
use guru_core::{
    Acao, AtivoBtg, Comparacao, CoreError, Db, Decisao, Estado, ExtratoBtg, FonteImportacao,
    Gravacao, Investimento, Item, NovoInvestimento, Origem, ResultadoImportacao,
    aplicar_importacao, comparar_btg, ler_extrato, valorizar,
};
use rust_decimal::Decimal;

const COMPLETO: &[u8] = include_bytes!("fixtures/btg/extrato-completo.pdf");
const DESCONHECIDA: &[u8] = include_bytes!("fixtures/btg/extrato-secao-desconhecida.pdf");
const VARIAS_PAGINAS: &[u8] = include_bytes!("fixtures/btg/extrato-varias-paginas.pdf");
const OUTRO_BANCO: &[u8] = include_bytes!("fixtures/btg/outro-banco.pdf");

fn d(a: i32, m: u32, dia: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, dia).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn achar_rf<'a>(e: &'a ExtratoBtg, cod: &str) -> &'a AtivoBtg {
    e.ativos
        .iter()
        .find(|a| matches!(a, AtivoBtg::RendaFixa { codigo, .. } if codigo == cod))
        .unwrap_or_else(|| panic!("renda fixa {cod} não encontrada"))
}

fn achar_rv<'a>(e: &'a ExtratoBtg, t: &str) -> &'a AtivoBtg {
    e.ativos
        .iter()
        .find(|a| matches!(a, AtivoBtg::RendaVariavel { ticker, .. } if ticker == t))
        .unwrap_or_else(|| panic!("renda variável {t} não encontrada"))
}

#[test]
fn extrato_completo() {
    let e = ler_extrato(COMPLETO).unwrap();
    assert_eq!(e.data, d(2026, 10, 31));
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    // Fundo com dois lotes: aplicado somado, primeira compra, saldo e referência da posição
    assert!(e.ativos.contains(&AtivoBtg::Fundo {
        nome: "FUNDO EXEMPLO RF LP".into(),
        cnpj: "11111111000111".into(),
        aplicado: dec("1000.00"),
        primeira_compra: d(2026, 1, 2),
        saldo_bruto: Some(dec("1050.00")),
        referencia: Some(d(2026, 10, 30)),
    }));
    // CDB com código e taxa quebrados em duas linhas
    assert!(e.ativos.contains(&AtivoBtg::RendaFixa {
        tipo: "rf-cdb",
        emissor: "BANCO FICTICIO S.A".into(),
        codigo: "CDBA26FICT1".into(),
        emissao: d(2026, 1, 15),
        vencimento: d(2029, 1, 15),
        primeira_aquisicao: d(2026, 1, 15),
        indexador: Some("pre"),
        taxa: Some(dec("12.50")),
        benchmark: None,
        liquidez: "vencimento",
        aplicado: dec("5000.00"),
        saldo_bruto: dec("5400.00"),
    }));
    // LCA pós-fixada em % do CDI
    assert!(matches!(
        achar_rf(&e, "LCA26EXEMP"),
        AtivoBtg::RendaFixa {
            tipo: "rf-lca",
            indexador: Some("pos"),
            benchmark: Some("cdi"),
            ..
        }
    ));
    // Ticker sem asterisco
    assert!(e.ativos.contains(&AtivoBtg::RendaVariavel {
        tipo: "rv-acao",
        ticker: "ABCD3".into(),
        nome: "EMPRESA EXEMPLO ON".into(),
        quantidade: 10,
        preco_medio: dec("20.00"),
    }));
    assert!(matches!(
        achar_rv(&e, "XYZW34"),
        AtivoBtg::RendaVariavel {
            tipo: "rv-bdr",
            quantidade: 3,
            ..
        }
    ));
    assert!(e.ativos.contains(&AtivoBtg::Cripto {
        nome: "BITCOIN (XBT)".into(),
        aplicado: Some(dec("300.00")),
        saldo_bruto: dec("350.00"),
    }));
    assert_eq!(e.ativos.len(), 7); // conta corrente e valores em trânsito ficam de fora
    assert_eq!(e.linhas_lidas, 7);
}

#[test]
fn secao_desconhecida_vira_aviso() {
    let e = ler_extrato(DESCONHECIDA).unwrap();
    assert_eq!(e.ativos.len(), 1);
    assert!(
        e.avisos
            .iter()
            .any(|a| a.contains("Tesouro Direto") && a.ends_with("não reconhecida."))
    );
    assert!(e.avisos.iter().any(|a| a.contains("Opções")));
}

#[test]
fn tabela_continua_na_pagina_seguinte() {
    let e = ler_extrato(VARIAS_PAGINAS).unwrap();
    assert_eq!(e.ativos.len(), 30);
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
}

#[test]
fn pdf_de_outro_banco() {
    let erro = ler_extrato(OUTRO_BANCO).unwrap_err();
    assert!(
        matches!(erro, CoreError::ArquivoInvalido(ref m) if m == "Este PDF não é um Extrato da Conta Investimento da BTG.")
    );
}

#[test]
#[ignore = "precisa do extrato real: GURU_EXTRATO_BTG=<caminho>"]
fn extrato_real() {
    let caminho = std::env::var("GURU_EXTRATO_BTG").expect("GURU_EXTRATO_BTG");
    let e = ler_extrato(&std::fs::read(caminho).unwrap()).unwrap();
    println!("{e:#?}");
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    assert!(!e.ativos.is_empty());
    // A comparação também não acrescenta avisos (valores fora do limite, saldo sem data).
    let (_d, db) = abrir();
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert_eq!(c.itens.len(), e.ativos.len());
}

// ---------------------------------------------------------------- comparação

fn hoje() -> NaiveDate {
    d(2026, 11, 6)
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

fn abrir() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn criar(db: &Db, novo: NovoInvestimento) -> i64 {
    match db.criar_investimento(&novo, hoje()).unwrap() {
        Gravacao::Ok(id) => id,
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn marcar_codigo(dir: &tempfile::TempDir, id: i64, codigo: &str) {
    let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
    conn.execute(
        "UPDATE investimentos SET codigo_b3 = ?1 WHERE id = ?2",
        rusqlite::params![codigo, id],
    )
    .unwrap();
}

fn acao(instituicao: &str, ticker: &str, qtd: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rv-acao".into(),
        instituicao: instituicao.into(),
        ticker: s(ticker),
        data_aplicacao: s("2025-04-01"),
        quantidade: Some(qtd),
        preco_medio_centavos: Some(2_000),
        ..Default::default()
    }
}

fn cdb_ficticio(instituicao: &str, aplicado: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: instituicao.into(),
        emissor: s("Banco Fictício S.A."),
        indexador: s("pre"),
        taxa: s("12.50"),
        data_aplicacao: s("2026-01-15"),
        vencimento: s("2029-01-15"),
        valor_aplicado_centavos: Some(aplicado),
        liquidez: s("vencimento"),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn fundo(nome: &str, aplicado: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "fundo".into(),
        instituicao: "BTG Pactual".into(),
        emissor: s(nome),
        data_aplicacao: s("2026-01-02"),
        valor_aplicado_centavos: Some(aplicado),
        liquidez: s("d30"),
        ..Default::default()
    }
}

fn comparar_completo(db: &Db) -> Comparacao {
    let e = ler_extrato(COMPLETO).unwrap();
    comparar_btg(db, &e, hoje()).unwrap()
}

fn item<'a>(c: &'a Comparacao, ativo: &str) -> &'a Item {
    c.itens
        .iter()
        .find(|i| i.ativo == ativo)
        .unwrap_or_else(|| panic!("sem item {ativo}: {:?}", c.itens))
}

fn item_que_comeca<'a>(c: &'a Comparacao, inicio: &str) -> &'a Item {
    c.itens
        .iter()
        .find(|i| i.ativo.starts_with(inicio))
        .unwrap_or_else(|| panic!("sem item {inicio}…: {:?}", c.itens))
}

#[test]
fn carteira_vazia_da_so_novos_com_sugestao_completa() {
    let (_d, db) = abrir();
    let c = comparar_completo(&db);
    assert_eq!(c.fonte, FonteImportacao::Btg);
    assert_eq!(c.data_arquivo, Some(d(2026, 10, 31)));
    assert_eq!(c.linhas_lidas, 7);
    assert_eq!(c.itens.len(), 7);
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    for i in &c.itens {
        assert_eq!(i.estado, Estado::Novo, "{}", i.ativo);
        assert_eq!(i.acoes, vec![Acao::Adicionar, Acao::Ignorar]);
        assert_eq!(i.acao_padrao, Acao::Adicionar);
        assert_eq!(i.instituicao, "BTG Pactual");
        assert!(i.chave.starts_with("btg:"), "{}", i.chave);
        assert!(i.faltando.is_empty(), "{}: {:?}", i.ativo, i.faltando);
    }

    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.no_arquivo.as_deref(), Some("10 ações"));
    let sug = abcd.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "rv-acao");
    assert_eq!(sug.quantidade, Some(10));
    assert_eq!(sug.preco_medio_centavos, Some(2_000));
    assert_eq!(sug.data_aplicacao.as_deref(), Some("2026-10-31"));

    let cdb = item_que_comeca(&c, "CDB");
    assert_eq!(cdb.no_arquivo.as_deref(), Some("R$ 5.000,00"));
    let sug = cdb.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "rf-cdb");
    assert_eq!(sug.emissor.as_deref(), Some("BANCO FICTICIO S.A"));
    assert_eq!(sug.indexador.as_deref(), Some("pre"));
    assert_eq!(sug.taxa.as_deref(), Some("12.50"));
    assert_eq!(sug.benchmark, None);
    assert_eq!(sug.data_aplicacao.as_deref(), Some("2026-01-15"));
    assert_eq!(sug.vencimento.as_deref(), Some("2029-01-15"));
    assert_eq!(sug.valor_aplicado_centavos, Some(500_000));
    assert_eq!(sug.liquidez.as_deref(), Some("vencimento"));
    assert_eq!(sug.ir.as_deref(), Some("regressiva"));

    let lca = item_que_comeca(&c, "LCA");
    let sug = lca.sugestao.as_ref().unwrap();
    assert_eq!(sug.benchmark.as_deref(), Some("cdi"));
    assert_eq!(sug.ir.as_deref(), Some("isento"));

    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.no_arquivo.as_deref(), Some("R$ 1.000,00"));
    let sug = f.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "fundo");
    assert_eq!(sug.emissor.as_deref(), Some("FUNDO EXEMPLO RF LP"));
    assert_eq!(sug.data_aplicacao.as_deref(), Some("2026-01-02"));
    assert_eq!(sug.valor_aplicado_centavos, Some(100_000));
    assert_eq!(sug.liquidez.as_deref(), Some("d30"));

    let btc = item(&c, "BITCOIN (XBT)");
    assert_eq!(btc.no_arquivo.as_deref(), Some("R$ 300,00"));
    let sug = btc.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "cripto");
    assert_eq!(sug.emissor.as_deref(), Some("BITCOIN (XBT)"));
    assert_eq!(sug.valor_aplicado_centavos, Some(30_000));
    assert_eq!(sug.data_aplicacao.as_deref(), Some("2026-10-31"));
}

#[test]
fn cripto_sem_aplicado_mostra_o_saldo() {
    let (_d, db) = abrir();
    let e = ExtratoBtg {
        data: d(2026, 10, 31),
        ativos: vec![AtivoBtg::Cripto {
            nome: "ETHEREUM (ETH)".into(),
            aplicado: None,
            saldo_bruto: dec("80.00"),
        }],
        avisos: vec![],
        linhas_lidas: 1,
    };
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    let i = &c.itens[0];
    assert_eq!(i.estado, Estado::Novo);
    assert_eq!(i.no_arquivo.as_deref(), Some("R$ 80,00 (saldo)"));
    assert_eq!(i.sugestao.as_ref().unwrap().valor_aplicado_centavos, None);

    // Com um par só e sem valor aplicado no extrato: igual
    criar(
        &db,
        NovoInvestimento {
            tipo: "cripto".into(),
            instituicao: "BTG Pactual".into(),
            emissor: s("Ethereum (ETH)"),
            data_aplicacao: s("2026-01-02"),
            valor_aplicado_centavos: Some(5_000),
            liquidez: s("diaria"),
            ..Default::default()
        },
    );
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    assert_eq!(c.itens[0].estado, Estado::Igual);
}

#[test]
fn valor_fora_do_limite_vira_aviso() {
    let (_d, db) = abrir();
    let e = ExtratoBtg {
        data: d(2026, 10, 31),
        ativos: vec![AtivoBtg::Cripto {
            nome: "MOEDA GIGANTE".into(),
            aplicado: Some(Decimal::MAX),
            saldo_bruto: dec("1.00"),
        }],
        avisos: vec!["Aviso do leitor.".into()],
        linhas_lidas: 1,
    };
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    assert!(c.itens.is_empty());
    assert_eq!(
        c.avisos,
        vec![
            "Aviso do leitor.".to_string(),
            "Valor fora do limite em MOEDA GIGANTE.".to_string()
        ]
    );
}

#[test]
fn btg_casa_ativo_vindo_da_b3() {
    let (dir, db) = abrir();
    let id = criar(&db, acao("BTG PACTUAL CTVM S/A", "ABCD3", 10));
    marcar_codigo(&dir, id, "ABCD3");
    let id_cdb = criar(&db, cdb_ficticio("BTG PACTUAL CTVM S/A", 500_000));
    marcar_codigo(&dir, id_cdb, "CDBA26FICT1");
    let c = comparar_completo(&db);
    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.estado, Estado::Igual);
    assert_eq!(abcd.investimentos, vec![id]);
    assert_eq!(abcd.no_app.as_deref(), Some("10 ações"));
    let cdb = item_que_comeca(&c, "CDB");
    assert_eq!(cdb.estado, Estado::Igual);
    assert_eq!(cdb.investimentos, vec![id_cdb]);
    assert_eq!(cdb.no_app.as_deref(), Some("R$ 5.000,00"));
}

#[test]
fn mesmo_ticker_em_outra_corretora_nao_casa() {
    let (_d, db) = abrir();
    let id = criar(&db, acao("XP Investimentos", "ABCD3", 10));
    let c = comparar_completo(&db);
    // A XP é outra corretora do catálogo: o item da BTG é novo e o da XP nem aparece.
    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.estado, Estado::Novo);
    assert!(abcd.investimentos.is_empty());
    assert!(c.itens.iter().all(|i| !i.investimentos.contains(&id)));
    assert_eq!(c.itens.len(), 7);
}

#[test]
fn mesmo_ticker_em_corretora_digitada_a_mao_oferece_vincular() {
    let (_d, db) = abrir();
    let id = criar(&db, acao("Genial", "ABCD3", 10));
    let c = comparar_completo(&db);
    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.estado, Estado::Divergente);
    assert_eq!(
        abcd.acoes,
        vec![Acao::Vincular(id), Acao::Adicionar, Acao::Ignorar]
    );
    assert_eq!(abcd.acao_padrao, Acao::Ignorar);
    assert!(abcd.sugestao.is_some());
}

#[test]
fn mesmo_ticker_na_btg_e_em_outra_corretora_casa_so_o_da_btg() {
    let (_d, db) = abrir();
    let xp = criar(&db, acao("XP Investimentos", "ABCD3", 5));
    let btg = criar(&db, acao("BTG Pactual", "ABCD3", 10));
    let c = comparar_completo(&db);
    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.estado, Estado::Igual);
    assert_eq!(abcd.investimentos, vec![btg]);
    assert!(c.itens.iter().all(|i| !i.investimentos.contains(&xp)));
}

#[test]
fn mesmo_codigo_em_outra_corretora_vira_divergente() {
    // Código gravado igual vale em qualquer corretora, mesmo numa do catálogo: candidato de
    // outra corretora, o padrão é ignorar.
    let (dir, db) = abrir();
    let id = criar(&db, cdb_ficticio("XP Investimentos", 500_000));
    marcar_codigo(&dir, id, "CDBA26FICT1");
    let c = comparar_completo(&db);
    let cdb = item_que_comeca(&c, "CDB");
    assert_eq!(cdb.estado, Estado::Divergente);
    assert_eq!(
        cdb.acoes,
        vec![Acao::Vincular(id), Acao::Adicionar, Acao::Ignorar]
    );
    assert_eq!(cdb.acao_padrao, Acao::Ignorar);
    assert!(cdb.sugestao.is_some());
}

#[test]
fn fundo_lancado_a_mao_casa_pelo_nome() {
    let (_d, db) = abrir();
    let id = criar(&db, fundo("Fundo Exemplo RF LP", 100_000));
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Igual);
    assert_eq!(f.investimentos, vec![id]);

    let (_d2, db2) = abrir();
    let id2 = criar(&db2, fundo("Fundo Exemplo RF LP", 90_000));
    let c = comparar_completo(&db2);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Divergente);
    assert_eq!(f.acoes, vec![Acao::Manter, Acao::UsarArquivo]);
    assert_eq!(f.acao_padrao, Acao::Manter);
    assert_eq!(f.investimentos, vec![id2]);
    assert_eq!(f.no_app.as_deref(), Some("R$ 900,00"));
}

#[test]
fn dois_fundos_com_o_mesmo_nome_viram_divergente_com_vincular() {
    let (_d, db) = abrir();
    let a = criar(&db, fundo("Fundo Exemplo RF LP", 50_000));
    let b = criar(&db, fundo("FUNDO EXEMPLO RF LP", 50_000));
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Divergente);
    assert_eq!(
        f.acoes,
        vec![
            Acao::Manter,
            Acao::Vincular(a),
            Acao::Vincular(b),
            Acao::Adicionar
        ]
    );
    assert_eq!(f.acao_padrao, Acao::Manter);
    assert!(f.sugestao.is_some());
}

#[test]
fn aplicado_diferente_com_codigo_vira_atualizar() {
    let (dir, db) = abrir();
    let id = criar(&db, cdb_ficticio("BTG Pactual", 400_000));
    marcar_codigo(&dir, id, "CDBA26FICT1");
    let c = comparar_completo(&db);
    let cdb = item_que_comeca(&c, "CDB");
    assert_eq!(cdb.estado, Estado::Atualizar);
    assert_eq!(cdb.acoes, vec![Acao::UsarArquivo, Acao::Manter]);
    assert_eq!(cdb.acao_padrao, Acao::UsarArquivo);
    assert_eq!(cdb.no_app.as_deref(), Some("R$ 4.000,00"));
    assert_eq!(cdb.no_arquivo.as_deref(), Some("R$ 5.000,00"));
    // ordenado: novo, atualizar, divergente, igual
    let ordem: Vec<Estado> = c.itens.iter().map(|i| i.estado).collect();
    let mut ordenada = ordem.clone();
    ordenada.sort_by_key(|e| match e {
        Estado::Novo => 0,
        Estado::Atualizar => 1,
        Estado::Divergente => 2,
        Estado::Igual => 3,
    });
    assert_eq!(ordem, ordenada);
}

// ---------------------------------------------------------------- aplicação

fn sql(dir: &tempfile::TempDir, comando: &str, id: i64) {
    let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
    conn.execute(comando, [id]).unwrap();
}

fn decisao(chave: &str, acao: Acao) -> Decisao {
    Decisao {
        chave: chave.to_string(),
        acao,
        dados: None,
    }
}

fn aplicar(db: &Db, c: &Comparacao, decisoes: &[Decisao]) -> ResultadoImportacao {
    let r = aplicar_importacao(db, c, decisoes, "extrato.pdf", hoje()).unwrap();
    assert!(r.erros.is_empty(), "{:?}", r.erros);
    r
}

fn adicionar_tudo(db: &Db, c: &Comparacao) -> ResultadoImportacao {
    let decisoes: Vec<Decisao> = c
        .itens
        .iter()
        .map(|i| decisao(&i.chave, Acao::Adicionar))
        .collect();
    aplicar(db, c, &decisoes)
}

fn por_tipo(db: &Db, tipo: &str) -> Vec<Investimento> {
    db.listar_investimentos(hoje())
        .unwrap()
        .into_iter()
        .filter(|i| i.tipo == tipo)
        .collect()
}

#[test]
fn adicionar_fundo_grava_codigo_e_saldo() {
    let (_d, db) = abrir();
    let c = comparar_completo(&db);
    let r = adicionar_tudo(&db, &c);
    assert_eq!(r.aplicadas, 7);

    let fundos = por_tipo(&db, "fundo");
    let f = fundos
        .iter()
        .find(|i| i.emissor.as_deref() == Some("FUNDO EXEMPLO RF LP"))
        .unwrap();
    assert_eq!(f.codigo_b3.as_deref(), Some("11111111000111"));
    assert_eq!(f.valor_aplicado_centavos, 100_000);
    assert_eq!(f.saldo_centavos, Some(105_000));
    assert_eq!(f.saldo_data.as_deref(), Some("2026-10-30"));
    let v = valorizar(&db, f, hoje()).unwrap();
    assert_eq!(v.origem, Origem::Extrato);
    assert_eq!(v.atual_centavos, 105_000);

    let cripto = por_tipo(&db, "cripto");
    assert_eq!(cripto.len(), 1);
    let b = &cripto[0];
    assert_eq!(b.codigo_b3.as_deref(), Some("BITCOIN XBT"));
    assert_eq!(b.valor_aplicado_centavos, 30_000);
    assert_eq!(b.saldo_centavos, Some(35_000));
    assert_eq!(b.saldo_data.as_deref(), Some("2026-10-31"));

    let acoes = por_tipo(&db, "rv-acao");
    let abcd = acoes
        .iter()
        .find(|i| i.ticker.as_deref() == Some("ABCD3"))
        .unwrap();
    assert_eq!(abcd.codigo_b3.as_deref(), Some("ABCD3"));
    assert_eq!(abcd.preco_medio_centavos, Some(2_000));
    assert_eq!(abcd.saldo_centavos, None);

    let cdb = &por_tipo(&db, "rf-cdb")[0];
    assert_eq!(cdb.codigo_b3.as_deref(), Some("CDBA26FICT1"));
    assert_eq!(cdb.taxa.as_deref(), Some("12.5")); // 12,50% a.a., normalizada como no cadastro
    assert_eq!(cdb.saldo_centavos, None);
}

#[test]
fn fundo_sem_referencia_nao_ganha_saldo() {
    let (_d, db) = abrir();
    let e = ExtratoBtg {
        data: d(2026, 10, 31),
        ativos: vec![AtivoBtg::Fundo {
            nome: "FUNDO SEM DATA".into(),
            cnpj: "22222222000122".into(),
            aplicado: dec("500.00"),
            primeira_compra: d(2026, 2, 1),
            saldo_bruto: Some(dec("520.00")),
            referencia: None,
        }],
        avisos: vec![],
        linhas_lidas: 1,
    };
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    // Sem a data de referência o saldo não entra (nem com a data do extrato) e vira aviso.
    assert_eq!(
        c.avisos,
        vec!["Fundo FUNDO SEM DATA: saldo sem data de referência, não importado.".to_string()]
    );
    adicionar_tudo(&db, &c);
    let f = &por_tipo(&db, "fundo")[0];
    assert_eq!(f.codigo_b3.as_deref(), Some("22222222000122"));
    assert_eq!(f.saldo_centavos, None);
    assert_eq!(f.saldo_data, None);
}

#[test]
fn igual_atualiza_o_saldo_sem_contar_alteracao() {
    let (dir, db) = abrir();
    let id = criar(&db, fundo("Fundo Exemplo RF LP", 100_000));
    sql(
        &dir,
        "UPDATE investimentos SET saldo_centavos = 100000, saldo_data = '2026-09-30' WHERE id = ?1",
        id,
    );
    let c = comparar_completo(&db);
    assert_eq!(item(&c, "FUNDO EXEMPLO RF LP").estado, Estado::Igual);
    let r = aplicar(&db, &c, &[]);
    assert_eq!(r.aplicadas, 0);
    let f = db.investimento(id, hoje()).unwrap();
    assert_eq!(f.saldo_centavos, Some(105_000));
    assert_eq!(f.saldo_data.as_deref(), Some("2026-10-30"));
    assert_eq!(f.codigo_b3.as_deref(), Some("11111111000111"));
    assert_eq!(f.valor_aplicado_centavos, 100_000);
}

#[test]
fn divergente_mantido_nao_ganha_saldo_nem_codigo() {
    let (_d, db) = abrir();
    let id = criar(&db, fundo("Fundo Exemplo RF LP", 90_000));
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Divergente);
    let r = aplicar(&db, &c, &[decisao(&f.chave, Acao::Manter)]);
    assert_eq!(r.aplicadas, 0);
    let i = db.investimento(id, hoje()).unwrap();
    assert_eq!(i.codigo_b3, None);
    assert_eq!(i.saldo_centavos, None);
    assert_eq!(i.saldo_data, None);
    assert_eq!(i.valor_aplicado_centavos, 90_000);
}

#[test]
fn vincular_grava_codigo_e_saldo_no_escolhido() {
    let (_d, db) = abrir();
    let a = criar(&db, fundo("Fundo Exemplo RF LP", 50_000));
    let b = criar(&db, fundo("FUNDO EXEMPLO RF LP", 50_000));
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    aplicar(&db, &c, &[decisao(&f.chave, Acao::Vincular(a))]);
    let ia = db.investimento(a, hoje()).unwrap();
    assert_eq!(ia.codigo_b3.as_deref(), Some("11111111000111"));
    assert_eq!(ia.saldo_centavos, Some(105_000));
    let ib = db.investimento(b, hoje()).unwrap();
    assert_eq!(ib.codigo_b3, None);
    assert_eq!(ib.saldo_centavos, None);
}

#[test]
fn cdb_vindo_da_b3_ganha_taxa_sem_perder_o_que_tinha() {
    let (dir, db) = abrir();
    let id = criar(&db, cdb_ficticio("BTG PACTUAL CTVM S/A", 500_000));
    marcar_codigo(&dir, id, "CDBA26FICT1");
    sql(
        &dir,
        "UPDATE investimentos SET emissor = 'Banco Fictício', taxa = NULL, indexador = NULL, \
         benchmark = NULL WHERE id = ?1",
        id,
    );
    let c = comparar_completo(&db);
    assert_eq!(item_que_comeca(&c, "CDB").estado, Estado::Igual);
    let r = aplicar(&db, &c, &[]);
    assert_eq!(r.aplicadas, 0);
    let i = db.investimento(id, hoje()).unwrap();
    // 12,50% a.a., gravada normalizada como o cadastro grava
    assert_eq!(i.taxa.as_deref(), Some("12.5"));
    assert_eq!(i.indexador.map(|x| x.codigo()), Some("pre"));
    assert_eq!(i.benchmark, None);
    let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
    let bruta: String = conn
        .query_row("SELECT taxa FROM investimentos WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(bruta, "12.5");
    assert_eq!(i.emissor.as_deref(), Some("Banco Fictício"));

    // Taxa já preenchida: nada muda
    let (dir2, db2) = abrir();
    let id2 = criar(&db2, cdb_ficticio("BTG PACTUAL CTVM S/A", 500_000));
    marcar_codigo(&dir2, id2, "CDBA26FICT1");
    sql(
        &dir2,
        "UPDATE investimentos SET taxa = '13' WHERE id = ?1",
        id2,
    );
    let c = comparar_completo(&db2);
    aplicar(&db2, &c, &[]);
    let i = db2.investimento(id2, hoje()).unwrap();
    assert_eq!(i.taxa.as_deref(), Some("13"));
    assert_eq!(i.indexador.map(|x| x.codigo()), Some("pre"));
    assert_eq!(i.emissor.as_deref(), Some("Banco Fictício S.A."));
}

#[test]
fn usar_arquivo_preenche_preco_medio_vazio() {
    let (dir, db) = abrir();
    let id = criar(&db, acao("BTG Pactual", "ABCD3", 5));
    sql(
        &dir,
        "UPDATE investimentos SET preco_medio_centavos = NULL WHERE id = ?1",
        id,
    );
    let c = comparar_completo(&db);
    let abcd = item(&c, "ABCD3");
    // Sem código gravado: divergente, mas o usuário pode confirmar com `UsarArquivo`
    assert!(abcd.acoes.contains(&Acao::UsarArquivo));
    let r = aplicar(&db, &c, &[decisao(&abcd.chave, Acao::UsarArquivo)]);
    assert_eq!(r.aplicadas, 1);
    let i = db.investimento(id, hoje()).unwrap();
    assert_eq!(i.quantidade, Some(10));
    assert_eq!(i.preco_medio_centavos, Some(2_000));
    assert_eq!(i.valor_aplicado_centavos, 20_000);
    assert_eq!(i.codigo_b3.as_deref(), Some("ABCD3"));
}

#[test]
fn preco_medio_preenchido_nao_e_sobrescrito() {
    let (_d, db) = abrir();
    let id = criar(
        &db,
        NovoInvestimento {
            preco_medio_centavos: Some(1_500),
            ..acao("BTG Pactual", "ABCD3", 10)
        },
    );
    let c = comparar_completo(&db);
    assert_eq!(item(&c, "ABCD3").estado, Estado::Igual);
    aplicar(&db, &c, &[]);
    let i = db.investimento(id, hoje()).unwrap();
    assert_eq!(i.preco_medio_centavos, Some(1_500));
    assert_eq!(i.codigo_b3.as_deref(), Some("ABCD3"));
}

#[test]
fn importacao_btg_grava_a_fonte() {
    let (dir, db) = abrir();
    let c = comparar_completo(&db);
    aplicar(&db, &c, &[]);
    let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
    let fonte: String = conn
        .query_row("SELECT fonte FROM importacao", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fonte, "btg");
}

// ---------------------------------------------------------------- revisão final

fn cripto(nome: &str, instituicao: &str) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "cripto".into(),
        instituicao: instituicao.into(),
        emissor: s(nome),
        data_aplicacao: s("2026-01-02"),
        valor_aplicado_centavos: Some(30_000),
        liquidez: s("diaria"),
        ..Default::default()
    }
}

#[test]
fn fundo_com_nome_parecido_oferece_vincular() {
    let (_d, db) = abrir();
    let id = criar(&db, fundo("Fundo Exemplo RF", 100_000));
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Divergente);
    assert_eq!(
        f.acoes,
        vec![Acao::Vincular(id), Acao::Adicionar, Acao::Ignorar]
    );
    assert_eq!(f.acao_padrao, Acao::Ignorar);
    assert_eq!(f.investimentos, vec![id]);
    assert!(f.sugestao.is_some());
    // Vincular grava o CNPJ e a próxima comparação casa pelo código.
    aplicar(&db, &c, &[decisao(&f.chave, Acao::Vincular(id))]);
    let c = comparar_completo(&db);
    assert_eq!(item(&c, "FUNDO EXEMPLO RF LP").estado, Estado::Igual);
}

#[test]
fn cripto_com_nome_parecido_oferece_vincular() {
    let (_d, db) = abrir();
    let id = criar(&db, cripto("Bitcoin", "BTG Pactual"));
    let c = comparar_completo(&db);
    let b = item(&c, "BITCOIN (XBT)");
    assert_eq!(b.estado, Estado::Divergente);
    assert_eq!(
        b.acoes,
        vec![Acao::Vincular(id), Acao::Adicionar, Acao::Ignorar]
    );
    assert_eq!(b.acao_padrao, Acao::Ignorar);
}

#[test]
fn nome_sem_relacao_ou_de_outra_corretora_continua_novo() {
    let (_d, db) = abrir();
    let outro = criar(&db, fundo("Fundo Diferente Multimercado", 100_000));
    // Pedaço de palavra não conta: "EXEMP" não é "EXEMPLO".
    let pedaco = criar(&db, fundo("Fundo Exemp", 100_000));
    let xp = criar(
        &db,
        NovoInvestimento {
            instituicao: "XP Investimentos".into(),
            ..fundo("Fundo Exemplo RF", 100_000)
        },
    );
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Novo);
    assert!(f.investimentos.is_empty());
    assert!(c.itens.iter().all(|i| {
        ![outro, pedaco, xp]
            .iter()
            .any(|id| i.investimentos.contains(id))
    }));
}

#[test]
fn corretora_digitada_a_mao_com_o_codigo_casa_como_igual() {
    let (dir, db) = abrir();
    let fundo_id = criar(
        &db,
        NovoInvestimento {
            instituicao: "Genial".into(),
            ..fundo("Fundo Qualquer", 100_000)
        },
    );
    marcar_codigo(&dir, fundo_id, "11111111000111");
    let acao_id = criar(&db, acao("Genial", "ABCD3", 10));
    marcar_codigo(&dir, acao_id, "ABCD3");
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Igual);
    assert_eq!(f.investimentos, vec![fundo_id]);
    let abcd = item(&c, "ABCD3");
    assert_eq!(abcd.estado, Estado::Igual);
    assert_eq!(abcd.investimentos, vec![acao_id]);
    aplicar(&db, &c, &[]);
    let i = db.investimento(fundo_id, hoje()).unwrap();
    assert_eq!(i.instituicao, "Genial");
    assert_eq!(i.saldo_centavos, Some(105_000));

    // Valor diferente com código gravado: atualizar, ainda sem mudar a corretora.
    sql(
        &dir,
        "UPDATE investimentos SET valor_aplicado_centavos = 90000 WHERE id = ?1",
        fundo_id,
    );
    let c = comparar_completo(&db);
    let f = item(&c, "FUNDO EXEMPLO RF LP");
    assert_eq!(f.estado, Estado::Atualizar);
    aplicar(&db, &c, &[decisao(&f.chave, Acao::UsarArquivo)]);
    let i = db.investimento(fundo_id, hoje()).unwrap();
    assert_eq!(i.instituicao, "Genial");
    assert_eq!(i.valor_aplicado_centavos, 100_000);
}

fn so_fundo(saldo: Option<Decimal>, referencia: Option<NaiveDate>) -> ExtratoBtg {
    ExtratoBtg {
        data: d(2026, 10, 31),
        ativos: vec![AtivoBtg::Fundo {
            nome: "FUNDO SEM DATA".into(),
            cnpj: "22222222000122".into(),
            aplicado: dec("500.00"),
            primeira_compra: d(2026, 2, 1),
            saldo_bruto: saldo,
            referencia,
        }],
        avisos: vec![],
        linhas_lidas: 1,
    }
}

#[test]
fn saldo_fora_do_limite_vira_aviso_e_fica_de_fora() {
    for saldo in ["-1.00", "100000000000000", "100000000000000000000"] {
        let (_d, db) = abrir();
        let e = so_fundo(Some(dec(saldo)), Some(d(2026, 10, 30)));
        let c = comparar_btg(&db, &e, hoje()).unwrap();
        assert_eq!(
            c.avisos,
            vec!["Saldo fora do limite em FUNDO SEM DATA, não importado.".to_string()],
            "{saldo}"
        );
        assert_eq!(c.itens.len(), 1, "{saldo}");
        adicionar_tudo(&db, &c);
        let f = &por_tipo(&db, "fundo")[0];
        assert_eq!(f.saldo_centavos, None, "{saldo}");
        assert_eq!(f.valor_aplicado_centavos, 50_000);
    }
    // Cripto: o saldo inválido some, o aplicado continua.
    let (_d, db) = abrir();
    let e = ExtratoBtg {
        data: d(2026, 10, 31),
        ativos: vec![AtivoBtg::Cripto {
            nome: "MOEDA EXEMPLO".into(),
            aplicado: Some(dec("10.00")),
            saldo_bruto: dec("-5.00"),
        }],
        avisos: vec![],
        linhas_lidas: 1,
    };
    let c = comparar_btg(&db, &e, hoje()).unwrap();
    assert_eq!(
        c.avisos,
        vec!["Saldo fora do limite em MOEDA EXEMPLO, não importado.".to_string()]
    );
    adicionar_tudo(&db, &c);
    let m = &por_tipo(&db, "cripto")[0];
    assert_eq!(m.saldo_centavos, None);
    assert_eq!(m.valor_aplicado_centavos, 1_000);
}

#[test]
fn preco_medio_zero_ou_negativo_fica_vazio() {
    for preco in ["0.00", "-3.00"] {
        let (dir, db) = abrir();
        let e = ExtratoBtg {
            data: d(2026, 10, 31),
            ativos: vec![AtivoBtg::RendaVariavel {
                tipo: "rv-acao",
                ticker: "ABCD3".into(),
                nome: "EMPRESA EXEMPLO ON".into(),
                quantidade: 10,
                preco_medio: dec(preco),
            }],
            avisos: vec![],
            linhas_lidas: 1,
        };
        let c = comparar_btg(&db, &e, hoje()).unwrap();
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
        let i = &c.itens[0];
        assert_eq!(i.sugestao.as_ref().unwrap().preco_medio_centavos, None);

        // Num investimento sem preço médio, nada é completado com o valor inválido.
        let id = criar(&db, acao("BTG Pactual", "ABCD3", 10));
        sql(
            &dir,
            "UPDATE investimentos SET preco_medio_centavos = NULL WHERE id = ?1",
            id,
        );
        let c = comparar_btg(&db, &e, hoje()).unwrap();
        aplicar(&db, &c, &[]);
        assert_eq!(
            db.investimento(id, hoje()).unwrap().preco_medio_centavos,
            None,
            "{preco}"
        );
    }
}
