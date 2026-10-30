use chrono::NaiveDate;
use guru_core::{
    AbaRv, Acao, Comparacao, Db, Estado, Gravacao, Item, LinhaB3, NovoInvestimento, Posicao,
    SCHEMA_VERSION, comparar, ler_posicao,
};

const COMPLETA: &[u8] = include_bytes!("fixtures/b3/posicao-completa.xlsx");

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
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

fn rv(tipo: &str, ticker: &str, qtd: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: tipo.into(),
        instituicao: "Rico".into(),
        ticker: s(ticker),
        data_aplicacao: s("2025-04-01"),
        quantidade: Some(qtd),
        preco_medio_centavos: Some(3_000),
        ..Default::default()
    }
}

fn cdb(emissor: &str, vencimento: &str) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: "Banco Inter".into(),
        emissor: s(emissor),
        indexador: s("pos"),
        taxa: s("102"),
        benchmark: s("cdi"),
        data_aplicacao: s("2025-03-15"),
        vencimento: s(vencimento),
        valor_aplicado_centavos: Some(2_000_000),
        liquidez: s("vencimento"),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn selic(aplicado: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "td-selic".into(),
        instituicao: "Rico".into(),
        taxa: s("0.09"),
        data_aplicacao: s("2025-01-10"),
        vencimento: s("2029-03-01"),
        valor_aplicado_centavos: Some(aplicado),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn comparar_completa(db: &Db) -> Comparacao {
    let posicao = ler_posicao(COMPLETA).unwrap();
    comparar(db, &posicao, hoje()).unwrap()
}

fn item<'a>(c: &'a Comparacao, ativo: &str) -> &'a Item {
    c.itens
        .iter()
        .find(|i| i.ativo == ativo)
        .unwrap_or_else(|| panic!("sem item {ativo}: {:?}", c.itens))
}

fn item_cdb<'a>(c: &'a Comparacao, emissor: &str) -> &'a Item {
    c.itens
        .iter()
        .find(|i| i.ativo.starts_with("CDB") && i.ativo.contains(emissor))
        .unwrap_or_else(|| panic!("sem CDB {emissor}: {:?}", c.itens))
}

fn marcar_importado(dir: &tempfile::TempDir, id: i64, codigo: &str) {
    let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
    conn.execute(
        "UPDATE investimentos SET codigo_b3 = ?1 WHERE id = ?2",
        rusqlite::params![codigo, id],
    )
    .unwrap();
}

#[test]
fn carteira_vazia_tudo_novo() {
    let (_d, db) = abrir();
    let c = comparar_completa(&db);
    assert_eq!(c.itens.len(), 9);
    assert!(c.itens.iter().all(|i| i.estado == Estado::Novo));
    assert!(
        c.itens
            .iter()
            .all(|i| i.acao_padrao == Acao::Adicionar && i.sugestao.is_some())
    );
    assert_eq!(c.itens[0].chave.split(':').next(), Some("b3"));

    let itub = item(&c, "ITUB4");
    assert_eq!(itub.na_b3.as_deref(), Some("150 ações"));
    assert_eq!(itub.no_app, None);
    let sug = itub.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "rv-acao");
    assert_eq!(sug.quantidade, Some(150));
    let campos: Vec<&str> = itub.faltando.iter().map(|e| e.campo.as_str()).collect();
    assert!(campos.contains(&"precoMedioCentavos"), "{campos:?}");
    assert!(campos.contains(&"dataAplicacao"), "{campos:?}");

    let cdb = item_cdb(&c, "FICTICIO");
    let sug = cdb.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "rf-cdb");
    assert_eq!(sug.indexador.as_deref(), Some("pos"));
    assert_eq!(sug.benchmark.as_deref(), Some("cdi"));
    assert!(sug.data_aplicacao.is_some());
    let campos: Vec<&str> = cdb.faltando.iter().map(|e| e.campo.as_str()).collect();
    assert!(campos.contains(&"valorAplicadoCentavos"), "{campos:?}");
    assert!(campos.contains(&"taxa"), "{campos:?}");

    let td = item(&c, "Tesouro Selic 2029");
    let sug = td.sugestao.as_ref().unwrap();
    assert_eq!(sug.tipo, "td-selic");
    assert_eq!(sug.valor_aplicado_centavos, Some(800_000));
    assert_eq!(td.na_b3.as_deref(), Some("R$ 8.000,00"));

    assert_eq!(item(&c, "BOVA11").na_b3.as_deref(), Some("30 cotas"));
    assert_eq!(item(&c, "AAPL34").na_b3.as_deref(), Some("10 BDRs"));
}

#[test]
fn quantidade_diferente_e_divergente_e_importado_e_atualizar() {
    let (d, db) = abrir();
    let id = criar(&db, rv("rv-acao", "ITUB4", 120));
    let c = comparar_completa(&db);
    let i = item(&c, "ITUB4");
    assert_eq!(i.estado, Estado::Divergente);
    assert_eq!(i.no_app.as_deref(), Some("120 ações"));
    assert_eq!(i.na_b3.as_deref(), Some("150 ações"));
    assert_eq!(i.investimentos, vec![id]);
    assert_eq!(i.acao_padrao, Acao::Manter);
    assert!(i.sugestao.is_none());

    marcar_importado(&d, id, "ITUB4");
    let c = comparar_completa(&db);
    let i = item(&c, "ITUB4");
    assert_eq!(i.estado, Estado::Atualizar);
    assert_eq!(i.acao_padrao, Acao::UsarB3);
    assert!(i.acoes.contains(&Acao::UsarB3) && i.acoes.contains(&Acao::Manter));
}

#[test]
fn compras_somadas_sao_igual_ou_divergente_so_com_manter() {
    let (_d, db) = abrir();
    criar(&db, rv("rv-acao", "itub4", 100));
    let id2 = criar(&db, rv("rv-acao", "ITUB4", 50));
    let c = comparar_completa(&db);
    let i = item(&c, "ITUB4");
    assert_eq!(i.estado, Estado::Igual);
    assert_eq!(i.acao_padrao, Acao::Manter);
    assert_eq!(i.investimentos.len(), 2);

    db.atualizar_investimento(id2, &rv("rv-acao", "ITUB4", 40), hoje())
        .unwrap();
    let c = comparar_completa(&db);
    let i = item(&c, "ITUB4");
    assert_eq!(i.estado, Estado::Divergente);
    assert_eq!(i.no_app.as_deref(), Some("140 ações"));
    assert_eq!(i.acoes, vec![Acao::Manter]);
}

#[test]
fn compras_importadas_somadas_nao_escolhem_onde_atualizar() {
    let (d, db) = abrir();
    let a = criar(&db, rv("rv-acao", "ITUB4", 100));
    let b = criar(&db, rv("rv-acao", "ITUB4", 40));
    marcar_importado(&d, a, "ITUB4");
    marcar_importado(&d, b, "ITUB4");
    let c = comparar_completa(&db);
    let i = item(&c, "ITUB4");
    assert_eq!(i.estado, Estado::Divergente);
    assert_eq!(i.acoes, vec![Acao::Manter]);
}

#[test]
fn cdb_casa_pelo_emissor_normalizado() {
    let (_d, db) = abrir();
    let id = criar(&db, cdb("Banco Fictício SA", "2029-03-15"));
    let c = comparar_completa(&db);
    let i = item_cdb(&c, "FICTICIO");
    assert_eq!(i.estado, Estado::Igual);
    assert_eq!(i.investimentos, vec![id]);
    assert!(i.sugestao.is_none());
    // O outro CDB (vencimento 2027) continua novo.
    assert_eq!(item_cdb(&c, "EXEMPLO").estado, Estado::Novo);
}

#[test]
fn cdb_casa_primeiro_pelo_codigo_importado() {
    let (d, db) = abrir();
    let id = criar(&db, cdb("Outro Banco", "2030-01-01"));
    marcar_importado(&d, id, "CDB224ABCDE");
    let c = comparar_completa(&db);
    let i = c
        .itens
        .iter()
        .find(|i| i.investimentos == vec![id])
        .unwrap();
    assert_eq!(i.estado, Estado::Igual);
}

#[test]
fn dois_candidatos_sao_divergentes_com_vincular() {
    let (_d, db) = abrir();
    let a = criar(&db, cdb("Banco Ficticio", "2029-03-15"));
    let b = criar(&db, cdb("BANCO FICTICIO S.A.", "2029-03-15"));
    let c = comparar_completa(&db);
    let i = c
        .itens
        .iter()
        .find(|i| i.estado == Estado::Divergente)
        .unwrap();
    assert_eq!(i.acao_padrao, Acao::Manter);
    assert!(i.acoes.contains(&Acao::Vincular(a)));
    assert!(i.acoes.contains(&Acao::Vincular(b)));
    assert!(i.acoes.contains(&Acao::Adicionar));
    assert!(i.sugestao.is_some());
    assert!(!i.faltando.is_empty());
}

#[test]
fn tesouro_igual_ou_divergente_pelo_valor_aplicado() {
    let (_d, db) = abrir();
    criar(&db, selic(800_000));
    let c = comparar_completa(&db);
    assert_eq!(item(&c, "Tesouro Selic 2029").estado, Estado::Igual);

    let (d2, db2) = abrir();
    let id = criar(&db2, selic(700_000));
    let c = comparar_completa(&db2);
    let i = item(&c, "Tesouro Selic 2029");
    assert_eq!(i.estado, Estado::Divergente);
    assert_eq!(i.no_app.as_deref(), Some("R$ 7.000,00"));
    assert_eq!(i.na_b3.as_deref(), Some("R$ 8.000,00"));

    marcar_importado(&d2, id, "BRSTNCLF1RC8");
    let c = comparar_completa(&db2);
    assert_eq!(item(&c, "Tesouro Selic 2029").estado, Estado::Atualizar);
}

#[test]
fn fundo_e_cripto_ficam_fora_da_b3() {
    let (_d, db) = abrir();
    let fundo = NovoInvestimento {
        tipo: "fundo".into(),
        instituicao: "XP".into(),
        emissor: s("Fundo X"),
        data_aplicacao: s("2025-01-01"),
        valor_aplicado_centavos: Some(100_000),
        liquidez: s("d30"),
        ..Default::default()
    };
    let cripto = NovoInvestimento {
        tipo: "cripto".into(),
        emissor: s("Bitcoin"),
        liquidez: s("diaria"),
        ..fundo.clone()
    };
    let f = criar(&db, fundo);
    let k = criar(&db, cripto);
    let c = comparar_completa(&db);
    let fora: Vec<&Item> = c
        .itens
        .iter()
        .filter(|i| i.estado == Estado::ForaDaB3)
        .collect();
    assert_eq!(fora.len(), 2);
    assert!(fora.iter().all(|i| i.acao_padrao == Acao::Manter));
    assert!(fora.iter().all(|i| i.acoes.contains(&Acao::Remover)));
    assert!(fora.iter().any(|i| i.chave == format!("app:{f}")));
    assert!(fora.iter().any(|i| i.chave == format!("app:{k}")));
    assert_eq!(fora[0].no_app.as_deref(), Some("R$ 1.000,00"));
}

#[test]
fn ordem_dos_estados_e_codigos() {
    let (_d, db) = abrir();
    criar(&db, rv("rv-acao", "ITUB4", 150));
    criar(&db, rv("rv-acao", "B3SA3", 10));
    criar(&db, rv("rv-acao", "PETR4", 10));
    let c = comparar_completa(&db);
    let estados: Vec<Estado> = c.itens.iter().map(|i| i.estado).collect();
    let mut ordenados = estados.clone();
    let peso = |e: &Estado| match e {
        Estado::Novo => 0,
        Estado::Atualizar => 1,
        Estado::Divergente => 2,
        Estado::ForaDaB3 => 3,
        Estado::Igual => 4,
    };
    ordenados.sort_by_key(peso);
    assert_eq!(estados, ordenados);
    assert_eq!(Estado::ForaDaB3.codigo(), "fora_da_b3");
    assert_eq!(Estado::ForaDaB3.rotulo(), "fora da B3");
    assert_eq!(Acao::UsarB3.codigo(), "usar_b3");
    assert_eq!(Acao::Vincular(3).codigo(), "vincular");
    assert_eq!(c.linhas_lidas, 9);
}

#[test]
fn migra_banco_da_versao_4() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Db::open(dir.path()).unwrap();
        criar(&db, rv("rv-acao", "ITUB4", 10));
    }
    {
        let conn = rusqlite::Connection::open(dir.path().join("guru.db")).unwrap();
        conn.execute_batch(
            "ALTER TABLE investimentos DROP COLUMN codigo_b3;
             DROP TABLE importacao;
             PRAGMA user_version = 4;",
        )
        .unwrap();
    }
    let db = Db::open(dir.path()).unwrap();
    assert_eq!(db.schema_version().unwrap(), 5);
    assert_eq!(SCHEMA_VERSION, 5);
    let lista = db.listar_investimentos(hoje()).unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].codigo_b3, None);
}

fn rv_b3(ticker: &str, instituicao: &str, quantidade: i64) -> LinhaB3 {
    LinhaB3::RendaVariavel {
        aba: AbaRv::Acao,
        ticker: ticker.into(),
        produto: format!("{ticker} - EMPRESA"),
        instituicao: instituicao.into(),
        quantidade,
        preco_fechamento: None,
        valor_atualizado: None,
    }
}

fn cdb_b3(codigo: &str, prefixo: &str, vencimento: NaiveDate) -> LinhaB3 {
    LinhaB3::RendaFixa {
        produto: format!("{prefixo} - BANCO FICTICIO S/A"),
        tipo: match prefixo {
            "CDB" => Some("rf-cdb".to_string()),
            _ => None,
        },
        instituicao: "Corretora".into(),
        emissor: "BANCO FICTICIO S/A".into(),
        codigo: codigo.into(),
        indexador: None,
        benchmark: None,
        emissao: None,
        vencimento,
        quantidade: 1.into(),
        valor_curva: None,
    }
}

fn posicao(linhas: Vec<LinhaB3>) -> Posicao {
    Posicao {
        linhas_lidas: linhas.len(),
        linhas,
        avisos: Vec::new(),
    }
}

#[test]
fn mesmo_ticker_em_duas_corretoras_soma_e_vira_um_item() {
    let (_d, db) = abrir();
    criar(&db, rv("rv-acao", "ITUB4", 150));
    let p = posicao(vec![rv_b3("ITUB4", "A", 100), rv_b3("itub4", "B", 50)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    assert_eq!(c.itens.len(), 1);
    assert_eq!(c.itens[0].estado, Estado::Igual);
    assert_eq!(c.itens[0].instituicao, "A, B");
    assert_eq!(c.itens[0].na_b3.as_deref(), Some("150 ações"));
}

#[test]
fn mesmo_ticker_em_duas_corretoras_divergente_pela_soma() {
    let (_d, db) = abrir();
    criar(&db, rv("rv-acao", "ITUB4", 100));
    let p = posicao(vec![rv_b3("ITUB4", "A", 100), rv_b3("ITUB4", "B", 50)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    assert_eq!(c.itens.len(), 1);
    assert_eq!(c.itens[0].estado, Estado::Divergente);
    assert_eq!(c.itens[0].no_app.as_deref(), Some("100 ações"));
    assert_eq!(c.itens[0].na_b3.as_deref(), Some("150 ações"));
}

#[test]
fn investimento_casado_nao_serve_a_uma_segunda_linha() {
    let (_d, db) = abrir();
    let id = criar(&db, cdb("Banco Fictício SA", "2029-03-15"));
    let venc = NaiveDate::from_ymd_opt(2029, 3, 15).unwrap();
    let p = posicao(vec![cdb_b3("X1", "CDB", venc), cdb_b3("X2", "CDB", venc)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    let primeiro = c.itens.iter().find(|i| i.chave == "b3:0").unwrap();
    let segundo = c.itens.iter().find(|i| i.chave == "b3:1").unwrap();
    assert_eq!(primeiro.estado, Estado::Igual);
    assert_eq!(primeiro.investimentos, vec![id]);
    assert_eq!(segundo.estado, Estado::Novo);
    assert!(c.itens.iter().all(|i| i.estado != Estado::ForaDaB3));
}

#[test]
fn prefixo_desconhecido_deixa_o_tipo_em_branco() {
    let (_d, db) = abrir();
    let venc = NaiveDate::from_ymd_opt(2029, 3, 15).unwrap();
    let p = posicao(vec![cdb_b3("X1", "XYZ", venc)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    let i = &c.itens[0];
    assert_eq!(i.estado, Estado::Novo);
    assert_eq!(i.sugestao.as_ref().unwrap().tipo, "");
    assert!(i.faltando.iter().any(|e| e.campo == "tipo"));
}

#[test]
fn tesouro_tolera_um_centavo() {
    let tesouro_b3 = |centavos: i64| LinhaB3::Tesouro {
        produto: "Tesouro Selic 2029".into(),
        tipo: "td-selic".into(),
        isin: None,
        instituicao: "Rico".into(),
        vencimento: NaiveDate::from_ymd_opt(2029, 3, 1).unwrap(),
        valor_aplicado: Some(rust_decimal::Decimal::new(centavos, 2)),
        valor_atualizado: None,
    };
    let (_d, db) = abrir();
    criar(&db, selic(800_000));
    let estado = |c: i64| {
        comparar(&db, &posicao(vec![tesouro_b3(c)]), hoje())
            .unwrap()
            .itens[0]
            .estado
    };
    assert_eq!(estado(800_001), Estado::Igual);
    assert_eq!(estado(799_999), Estado::Igual);
    assert_eq!(estado(800_002), Estado::Divergente);
}
