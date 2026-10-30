use chrono::NaiveDate;
use guru_core::{
    AbaRv, Acao, Comparacao, Db, Decisao, Estado, Gravacao, Item, LinhaB3, NovoInvestimento,
    Posicao, SCHEMA_VERSION, aplicar_importacao, comparar, ler_posicao, ultima_importacao,
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

fn tesouro_b3(instituicao: &str, centavos: i64) -> LinhaB3 {
    LinhaB3::Tesouro {
        produto: "Tesouro Selic 2029".into(),
        tipo: "td-selic".into(),
        isin: None,
        instituicao: instituicao.into(),
        vencimento: NaiveDate::from_ymd_opt(2029, 3, 1).unwrap(),
        valor_aplicado: Some(rust_decimal::Decimal::new(centavos, 2)),
        valor_atualizado: None,
    }
}

#[test]
fn tesouro_em_duas_corretoras_soma_e_vira_um_item() {
    let (_d, db) = abrir();
    criar(&db, selic(800_000));
    let p = posicao(vec![tesouro_b3("A", 500_000), tesouro_b3("B", 300_000)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    assert_eq!(c.itens.len(), 1);
    assert_eq!(c.itens[0].estado, Estado::Igual);
    assert_eq!(c.itens[0].instituicao, "A, B");

    let (_d2, vazio) = abrir();
    let c = comparar(&vazio, &p, hoje()).unwrap();
    assert_eq!(c.itens.len(), 1);
    let sug = c.itens[0].sugestao.as_ref().unwrap();
    assert_eq!(sug.instituicao, "A");
    assert_eq!(sug.valor_aplicado_centavos, Some(800_000));
}

#[test]
fn sugestao_de_renda_variavel_usa_so_a_primeira_corretora() {
    let (_d, db) = abrir();
    let p = posicao(vec![rv_b3("ITUB4", "A", 100), rv_b3("ITUB4", "B", 50)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    assert_eq!(c.itens[0].instituicao, "A, B");
    assert_eq!(c.itens[0].sugestao.as_ref().unwrap().instituicao, "A");
}

#[test]
fn mesmo_ticker_em_abas_diferentes_nao_depende_da_ordem() {
    let (_d, db) = abrir();
    let etf = |q| LinhaB3::RendaVariavel {
        aba: AbaRv::Etf,
        ticker: "BOVA11".into(),
        produto: "BOVA11".into(),
        instituicao: "A".into(),
        quantidade: q,
        preco_fechamento: None,
        valor_atualizado: None,
    };
    let acao = |q| rv_b3("BOVA11", "A", q);
    let c1 = comparar(&db, &posicao(vec![etf(1), acao(2)]), hoje()).unwrap();
    assert_eq!(c1.itens.len(), 2);
    assert!(
        c1.itens
            .iter()
            .any(|i| i.na_b3.as_deref() == Some("1 cota"))
    );
    assert!(
        c1.itens
            .iter()
            .any(|i| i.na_b3.as_deref() == Some("2 ações"))
    );
}

#[test]
fn rf_ligado_por_codigo_nao_serve_a_duas_linhas() {
    let (d, db) = abrir();
    let id = criar(&db, cdb("Banco Fictício SA", "2029-03-15"));
    marcar_importado(&d, id, "X1");
    let venc = NaiveDate::from_ymd_opt(2029, 3, 15).unwrap();
    let p = posicao(vec![cdb_b3("X1", "CDB", venc), cdb_b3("X1", "CDB", venc)]);
    let c = comparar(&db, &p, hoje()).unwrap();
    let primeiro = c.itens.iter().find(|i| i.chave == "b3:0").unwrap();
    let segundo = c.itens.iter().find(|i| i.chave == "b3:1").unwrap();
    assert_eq!(primeiro.estado, Estado::Igual);
    assert_eq!(segundo.estado, Estado::Novo);
}

// ---- Aplicação ----

fn decisao(chave: &str, acao: Acao) -> Decisao {
    Decisao {
        chave: chave.to_string(),
        acao,
        dados: None,
    }
}

/// Completa a sugestão como o usuário faria na tela.
fn completar(item: &Item) -> NovoInvestimento {
    let mut n = item.sugestao.clone().expect("item sem sugestão");
    n.data_aplicacao.get_or_insert_with(|| "2025-04-01".into());
    if n.tipo.starts_with("rv-") {
        n.preco_medio_centavos = Some(3_000);
    } else {
        n.valor_aplicado_centavos.get_or_insert(2_000_000);
        let taxa = match (n.tipo.as_str(), n.indexador.as_deref()) {
            ("td-selic", _) => "0.09",
            ("td-ipca", _) => "6.5",
            (_, Some("pre")) => "12.5",
            _ => "102",
        };
        n.taxa.get_or_insert_with(|| taxa.to_string());
    }
    n
}

fn adicionar_tudo(c: &Comparacao) -> Vec<Decisao> {
    c.itens
        .iter()
        .map(|i| Decisao {
            chave: i.chave.clone(),
            acao: Acao::Adicionar,
            dados: Some(completar(i)),
        })
        .collect()
}

fn aplicar(db: &Db, c: &Comparacao, d: &[Decisao]) -> guru_core::ResultadoImportacao {
    aplicar_importacao(db, c, d, "posicao.xlsx", hoje()).unwrap()
}

fn total(db: &Db) -> usize {
    db.listar_investimentos(hoje()).unwrap().len()
}

#[test]
fn adicionar_tudo_e_comparar_de_novo_da_tudo_igual() {
    let (_d, db) = abrir();
    let c = comparar_completa(&db);
    let r = aplicar(&db, &c, &adicionar_tudo(&c));
    assert!(r.erros.is_empty(), "{:?}", r.erros);
    assert_eq!(r.aplicadas, c.itens.len());
    assert_eq!(total(&db), c.itens.len());

    let c2 = comparar_completa(&db);
    assert_eq!(c2.itens.len(), c.itens.len());
    assert!(
        c2.itens.iter().all(|i| i.estado == Estado::Igual),
        "{:?}",
        c2.itens
            .iter()
            .map(|i| (&i.ativo, i.estado))
            .collect::<Vec<_>>()
    );
    // Importar de novo não duplica.
    let r2 = aplicar(&db, &c2, &[]);
    assert_eq!(r2.aplicadas, 0);
    assert_eq!(total(&db), c.itens.len());
}

#[test]
fn codigo_b3_vem_do_ticker_do_codigo_e_do_isin() {
    let (_d, db) = abrir();
    let c = comparar_completa(&db);
    aplicar(&db, &c, &adicionar_tudo(&c));
    let conn = rusqlite::Connection::open(db.path()).unwrap();
    let codigos: Vec<(String, Option<String>)> = conn
        .prepare("SELECT tipo, codigo_b3 FROM investimentos")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(codigos.iter().all(|(_, c)| c.is_some()), "{codigos:?}");
    let de = |t: &str| -> Vec<String> {
        codigos
            .iter()
            .filter(|(tp, _)| tp.starts_with(t))
            .filter_map(|(_, c)| c.clone())
            .collect()
    };
    assert!(de("rv-").contains(&"ITUB4".to_string()));
    assert!(de("rf-").contains(&"CDB224ABCDE".to_string()));
    assert_eq!(de("td-").len(), 2);
    assert!(
        de("td-").contains(&"BRSTNCLF1RU6".to_string()),
        "{codigos:?}"
    );
}

#[test]
fn adicionar_incompleto_nao_grava_nada() {
    let (_d, db) = abrir();
    let c = comparar_completa(&db);
    let mut decisoes = adicionar_tudo(&c);
    let itub = item(&c, "ITUB4").chave.clone();
    let d = decisoes.iter_mut().find(|d| d.chave == itub).unwrap();
    d.dados.as_mut().unwrap().preco_medio_centavos = None;
    let r = aplicar(&db, &c, &decisoes);
    assert_eq!(r.aplicadas, 0);
    assert_eq!(r.erros.len(), 1);
    assert_eq!(r.erros[0].0, itub);
    assert!(
        r.erros[0].1.iter().any(|e| e.campo == "precoMedioCentavos"),
        "{:?}",
        r.erros
    );
    assert_eq!(total(&db), 0);
    assert_eq!(ultima_importacao(&db).unwrap(), None);
}

#[test]
fn usar_b3_na_renda_variavel_muda_so_a_quantidade() {
    let (d, db) = abrir();
    let mut novo = rv("rv-acao", "ITUB4", 120);
    novo.preco_medio_centavos = Some(3_142);
    let id = criar(&db, novo);
    marcar_importado(&d, id, "ITUB4");
    let c = comparar_completa(&db);
    let chave = item(&c, "ITUB4").chave.clone();
    let r = aplicar(&db, &c, &[decisao(&chave, Acao::UsarB3)]);
    assert_eq!((r.aplicadas, r.erros.len()), (1, 0));
    let inv = db.investimento(id, hoje()).unwrap();
    assert_eq!(inv.quantidade, Some(150));
    assert_eq!(inv.preco_medio_centavos, Some(3_142));
    assert_eq!(inv.valor_aplicado_centavos, 150 * 3_142);
}

#[test]
fn usar_b3_no_tesouro_muda_so_o_valor_aplicado() {
    let (d, db) = abrir();
    let id = criar(&db, selic(500_000));
    marcar_importado(&d, id, "BRSTNCLF1RU6");
    let c = comparar_completa(&db);
    let i = item(&c, "Tesouro Selic 2029");
    assert_eq!(i.estado, Estado::Atualizar);
    let chave = i.chave.clone();
    aplicar(&db, &c, &[decisao(&chave, Acao::UsarB3)]);
    let inv = db.investimento(id, hoje()).unwrap();
    assert_eq!(inv.valor_aplicado_centavos, 800_000);
    assert_eq!(inv.taxa.as_deref(), Some("0.09"));
}

#[test]
fn fora_da_b3_mantem_ou_remove() {
    let (_d, db) = abrir();
    let id = criar(&db, rv("rv-acao", "PETR4", 10));
    let c = comparar_completa(&db);
    let fora = c
        .itens
        .iter()
        .find(|i| i.estado == Estado::ForaDaB3)
        .unwrap();
    assert_eq!(fora.acao_padrao, Acao::Manter);
    let chave = fora.chave.clone();
    let r = aplicar(&db, &c, &[decisao(&chave, Acao::Manter)]);
    assert_eq!(r.aplicadas, 0);
    assert!(db.investimento(id, hoje()).is_ok());
    let r = aplicar(&db, &c, &[decisao(&chave, Acao::Remover)]);
    assert_eq!(r.aplicadas, 1);
    assert!(db.investimento(id, hoje()).is_err());
}

#[test]
fn vincular_grava_codigo_e_a_proxima_comparacao_casa_direto() {
    let (_d, db) = abrir();
    let a = criar(&db, cdb("Banco Ficticio", "2029-03-15"));
    let _b = criar(&db, cdb("BANCO FICTICIO S.A.", "2029-03-15"));
    let c = comparar_completa(&db);
    let chave = c
        .itens
        .iter()
        .find(|i| i.estado == Estado::Divergente)
        .unwrap()
        .chave
        .clone();
    let r = aplicar(&db, &c, &[decisao(&chave, Acao::Vincular(a))]);
    assert_eq!(r.aplicadas, 1);
    assert_eq!(
        db.investimento(a, hoje()).unwrap().codigo_b3.as_deref(),
        Some("CDB224ABCDE")
    );
    let c2 = comparar_completa(&db);
    let i = c2
        .itens
        .iter()
        .find(|i| i.investimentos == vec![a])
        .unwrap();
    assert_eq!(i.estado, Estado::Igual);
}

#[test]
fn igual_na_renda_fixa_grava_o_codigo_sem_contar_como_alteracao() {
    let (_d, db) = abrir();
    let id = criar(&db, cdb("Banco Ficticio", "2029-03-15"));
    let c = comparar_completa(&db);
    let r = aplicar(&db, &c, &[]);
    assert_eq!(r.aplicadas, 0);
    assert_eq!(
        db.investimento(id, hoje()).unwrap().codigo_b3.as_deref(),
        Some("CDB224ABCDE")
    );
}

#[test]
fn divergente_mantido_nao_ganha_codigo() {
    let (_d, db) = abrir();
    let id = criar(&db, rv("rv-acao", "ITUB4", 120));
    let c = comparar_completa(&db);
    let chave = item(&c, "ITUB4").chave.clone();
    aplicar(&db, &c, &[decisao(&chave, Acao::Manter)]);
    assert_eq!(db.investimento(id, hoje()).unwrap().codigo_b3, None);
}

#[test]
fn acao_fora_da_lista_ou_chave_desconhecida_e_decisao_invalida() {
    let (_d, db) = abrir();
    let c = comparar_completa(&db);
    let chave = item(&c, "ITUB4").chave.clone();
    for d in [
        decisao(&chave, Acao::Remover),
        decisao(&chave, Acao::UsarB3),
        decisao("b3:999", Acao::Manter),
    ] {
        let e = aplicar_importacao(&db, &c, &[d], "x.xlsx", hoje()).unwrap_err();
        assert_eq!(e.code(), "DECISAO_INVALIDA");
    }
    let duas = [
        decisao(&chave, Acao::Ignorar),
        decisao(&chave, Acao::Ignorar),
    ];
    let e = aplicar_importacao(&db, &c, &duas, "x.xlsx", hoje()).unwrap_err();
    assert_eq!(e.code(), "DECISAO_INVALIDA");
    assert_eq!(total(&db), 0);
}

#[test]
fn ultima_importacao_devolve_a_data() {
    let (_d, db) = abrir();
    assert_eq!(ultima_importacao(&db).unwrap(), None);
    let c = comparar_completa(&db);
    aplicar(&db, &c, &[]);
    assert_eq!(
        ultima_importacao(&db).unwrap().as_deref(),
        Some("2026-10-30")
    );
}

// ---- Planilhas hostis ----

/// Monta um `.xlsx` mínimo com abas de células `str` (texto) ou numéricas (prefixo `#`).
fn xlsx(abas: &[(&str, &[&[&str]])]) -> Vec<u8> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    let mut wb = String::from(r#"<workbook xmlns:r="x"><sheets>"#);
    let mut rels = String::from("<Relationships>");
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (n, (nome, linhas)) in abas.iter().enumerate() {
        let i = n + 1;
        wb += &format!(r#"<sheet name="{nome}" sheetId="{i}" r:id="rId{i}"/>"#);
        rels += &format!(
            r#"<Relationship Id="rId{i}" Type="x/worksheet" Target="worksheets/s{i}.xml"/>"#
        );
        let mut xml = String::from("<worksheet><sheetData>");
        for linha in *linhas {
            xml += "<row>";
            for c in *linha {
                match c.strip_prefix('#') {
                    Some(num) => xml += &format!("<c><v>{num}</v></c>"),
                    None => xml += &format!(r#"<c t="str"><v>{c}</v></c>"#),
                }
            }
            xml += "</row>";
        }
        xml += "</sheetData></worksheet>";
        z.start_file(format!("xl/worksheets/s{i}.xml"), opcoes)
            .unwrap();
        z.write_all(xml.as_bytes()).unwrap();
    }
    wb += "</sheets></workbook>";
    rels += "</Relationships>";
    z.start_file("xl/workbook.xml", opcoes).unwrap();
    z.write_all(wb.as_bytes()).unwrap();
    z.start_file("xl/_rels/workbook.xml.rels", opcoes).unwrap();
    z.write_all(rels.as_bytes()).unwrap();
    z.finish().unwrap().into_inner()
}

const ENORME: &str = "#79228162514264337593543950335";

#[test]
fn valor_enorme_vira_aviso_e_nao_derruba_a_comparacao() {
    let bytes = xlsx(&[
        (
            "Ações",
            &[
                &[
                    "Produto",
                    "Instituição",
                    "Código de Negociação",
                    "Quantidade",
                ],
                &["ITUB4 - ITAU UNIBANCO", "Rico", "ITUB4", "#150"],
            ],
        ),
        (
            "Tesouro Direto",
            &[
                &["Produto", "Instituição", "Vencimento", "Valor Aplicado"],
                &["Tesouro Selic 2029", "Rico", "01/03/2029", ENORME],
            ],
        ),
        (
            "Renda Fixa",
            &[
                &[
                    "Produto",
                    "Instituição",
                    "Código",
                    "Vencimento",
                    "Quantidade",
                    "Valor Atualizado CURVA",
                ],
                &[
                    "CDB - BANCO ENORME S/A",
                    "Rico",
                    "CDB9",
                    "15/03/2029",
                    "#1",
                    ENORME,
                ],
            ],
        ),
    ]);
    let (_d, db) = abrir();
    criar(&db, selic(800_000));
    let posicao = ler_posicao(&bytes).unwrap();
    let c = comparar(&db, &posicao, hoje()).unwrap();
    assert!(
        c.avisos
            .contains(&"Valor fora do limite em Tesouro Selic 2029.".to_string()),
        "{:?}",
        c.avisos
    );
    assert!(
        c.avisos
            .contains(&"Valor fora do limite em CDB - BANCO ENORME S/A.".to_string()),
        "{:?}",
        c.avisos
    );
    // As linhas fora do limite não entram: o Tesouro cadastrado fica fora da B3.
    let ativos: Vec<(&str, Estado)> = c
        .itens
        .iter()
        .map(|i| (i.ativo.as_str(), i.estado))
        .collect();
    assert_eq!(ativos.len(), 2, "{ativos:?}");
    assert!(ativos.contains(&("ITUB4", Estado::Novo)), "{ativos:?}");
    assert!(
        c.itens
            .iter()
            .any(|i| i.estado == Estado::ForaDaB3 && i.ativo.contains("Selic")),
        "{ativos:?}"
    );
}

// ---- Renda fixa ambígua ----

fn cdb_taxa(taxa: &str) -> NovoInvestimento {
    NovoInvestimento {
        taxa: s(taxa),
        ..cdb("Banco Fictício S/A", "2029-03-15")
    }
}

fn venc_2029() -> NaiveDate {
    NaiveDate::from_ymd_opt(2029, 3, 15).unwrap()
}

#[test]
fn rf_ambigua_oferece_os_dois_candidatos_as_duas_linhas() {
    let (_d, db) = abrir();
    let a = criar(&db, cdb_taxa("102"));
    let b = criar(&db, cdb_taxa("110"));
    let p = posicao(vec![
        cdb_b3("CDB1", "CDB", venc_2029()),
        cdb_b3("CDB2", "CDB", venc_2029()),
    ]);
    let c = comparar(&db, &p, hoje()).unwrap();
    for chave in ["b3:0", "b3:1"] {
        let i = c.itens.iter().find(|i| i.chave == chave).unwrap();
        assert_eq!(i.estado, Estado::Divergente, "{chave}");
        assert!(
            i.acoes.contains(&Acao::Vincular(a)),
            "{chave}: {:?}",
            i.acoes
        );
        assert!(
            i.acoes.contains(&Acao::Vincular(b)),
            "{chave}: {:?}",
            i.acoes
        );
    }
    // Ninguém casou os dois de forma única: continuam em "fora da B3".
    let fora: Vec<i64> = c
        .itens
        .iter()
        .filter(|i| i.estado == Estado::ForaDaB3)
        .flat_map(|i| i.investimentos.clone())
        .collect();
    assert_eq!(fora, vec![a, b]);

    // Vincular uma linha a cada um funciona e a próxima comparação casa pelo código.
    let r = aplicar(
        &db,
        &c,
        &[
            decisao("b3:0", Acao::Vincular(a)),
            decisao("b3:1", Acao::Vincular(b)),
        ],
    );
    assert_eq!((r.aplicadas, r.erros.len()), (2, 0));
    assert_eq!(
        db.investimento(a, hoje()).unwrap().codigo_b3.as_deref(),
        Some("CDB1")
    );
    assert_eq!(
        db.investimento(b, hoje()).unwrap().codigo_b3.as_deref(),
        Some("CDB2")
    );
    let c2 = comparar(&db, &p, hoje()).unwrap();
    assert!(
        c2.itens.iter().all(|i| i.estado == Estado::Igual),
        "{:?}",
        c2.itens
    );
}

#[test]
fn rf_ambigua_vincular_o_mesmo_investimento_duas_vezes_e_decisao_invalida() {
    let (_d, db) = abrir();
    let a = criar(&db, cdb_taxa("102"));
    let _b = criar(&db, cdb_taxa("110"));
    let p = posicao(vec![
        cdb_b3("CDB1", "CDB", venc_2029()),
        cdb_b3("CDB2", "CDB", venc_2029()),
    ]);
    let c = comparar(&db, &p, hoje()).unwrap();
    let duas = [
        decisao("b3:0", Acao::Vincular(a)),
        decisao("b3:1", Acao::Vincular(a)),
    ];
    let e = aplicar_importacao(&db, &c, &duas, "x.xlsx", hoje()).unwrap_err();
    assert_eq!(e.code(), "DECISAO_INVALIDA");

    // Vincular e remover o mesmo investimento também não vale.
    let fora = format!("app:{a}");
    let conflito = [
        decisao("b3:0", Acao::Vincular(a)),
        decisao(&fora, Acao::Remover),
    ];
    let e = aplicar_importacao(&db, &c, &conflito, "x.xlsx", hoje()).unwrap_err();
    assert_eq!(e.code(), "DECISAO_INVALIDA");
    assert_eq!(db.investimento(a, hoje()).unwrap().codigo_b3, None);
}

#[test]
fn vincular_o_que_outra_linha_casou_de_forma_unica_e_decisao_invalida() {
    let (_d, db) = abrir();
    let a = criar(&db, cdb_taxa("102"));
    let mut lci = cdb_taxa("95");
    lci.tipo = "rf-lci".into();
    lci.ir = s("isento");
    let b = criar(&db, lci);
    // A linha de prefixo desconhecido vê os dois; a de CDB, depois, casa só com o CDB.
    let p = posicao(vec![
        cdb_b3("X1", "XYZ", venc_2029()),
        cdb_b3("CDB1", "CDB", venc_2029()),
    ]);
    let c = comparar(&db, &p, hoje()).unwrap();
    let ambigua = c.itens.iter().find(|i| i.chave == "b3:0").unwrap();
    assert!(ambigua.acoes.contains(&Acao::Vincular(a)));
    assert!(ambigua.acoes.contains(&Acao::Vincular(b)));
    let unica = c.itens.iter().find(|i| i.chave == "b3:1").unwrap();
    assert_eq!(
        (unica.estado, unica.investimentos.clone()),
        (Estado::Igual, vec![a])
    );

    let e = aplicar_importacao(
        &db,
        &c,
        &[decisao("b3:0", Acao::Vincular(a))],
        "x.xlsx",
        hoje(),
    )
    .unwrap_err();
    assert_eq!(e.code(), "DECISAO_INVALIDA");
    let r = aplicar(&db, &c, &[decisao("b3:0", Acao::Vincular(b))]);
    assert_eq!((r.aplicadas, r.erros.len()), (1, 0));
    assert_eq!(
        db.investimento(b, hoje()).unwrap().codigo_b3.as_deref(),
        Some("X1")
    );
}

// ---- Itens menores ----

#[test]
fn tesouro_sem_valor_aplicado_nao_oferece_usar_b3() {
    let sem_valor = || match tesouro_b3("Rico", 0) {
        LinhaB3::Tesouro {
            produto,
            tipo,
            isin,
            instituicao,
            vencimento,
            ..
        } => LinhaB3::Tesouro {
            produto,
            tipo,
            isin,
            instituicao,
            vencimento,
            valor_aplicado: None,
            valor_atualizado: None,
        },
        _ => unreachable!(),
    };
    // Sem código: divergente, só Manter.
    let (d, db) = abrir();
    let id = criar(&db, selic(800_000));
    let c = comparar(&db, &posicao(vec![sem_valor()]), hoje()).unwrap();
    let i = &c.itens[0];
    assert_eq!(
        (i.acoes.clone(), i.acao_padrao),
        (vec![Acao::Manter], Acao::Manter)
    );
    // Já importado: também não vira Atualizar com UsarB3 como padrão.
    marcar_importado(&d, id, "td-selic:2029-03-01");
    let c = comparar(&db, &posicao(vec![sem_valor()]), hoje()).unwrap();
    let i = &c.itens[0];
    assert!(!i.acoes.contains(&Acao::UsarB3), "{:?}", i.acoes);
    assert_eq!(i.acao_padrao, Acao::Manter);
}

#[test]
fn usar_b3_sem_preco_medio_ou_sem_quantidade_pede_o_campo() {
    let (d, db) = abrir();
    let id = criar(&db, rv("rv-acao", "ITUB4", 120));
    marcar_importado(&d, id, "ITUB4");
    let conn = rusqlite::Connection::open(d.path().join("guru.db")).unwrap();
    let caso = |sql: &str| {
        conn.execute(sql, rusqlite::params![id]).unwrap();
        let c = comparar_completa(&db);
        let chave = item(&c, "ITUB4").chave.clone();
        let r = aplicar(&db, &c, &[decisao(&chave, Acao::UsarB3)]);
        assert_eq!(r.aplicadas, 0);
        r.erros[0].1.clone()
    };
    let e = caso("UPDATE investimentos SET preco_medio_centavos = NULL WHERE id = ?1");
    assert_eq!(
        (e[0].campo.as_str(), e[0].mensagem.as_str()),
        ("precoMedioCentavos", "Informe o preço médio.")
    );
    let e = caso(
        "UPDATE investimentos SET preco_medio_centavos = 3000, quantidade = NULL WHERE id = ?1",
    );
    assert_eq!(
        (e[0].campo.as_str(), e[0].mensagem.as_str()),
        ("quantidade", "Informe a quantidade.")
    );
}

#[test]
fn aplicar_espera_a_escrita_concorrente_em_vez_de_falhar() {
    use std::sync::mpsc;
    let (d, db) = abrir();
    let id = criar(&db, rv("rv-acao", "ITUB4", 120));
    marcar_importado(&d, id, "ITUB4");
    let c = comparar_completa(&db);
    let chave = item(&c, "ITUB4").chave.clone();

    // Outra conexão (a atualização de mercado) segura a escrita e grava logo depois.
    let caminho = d.path().join("guru.db");
    let (pronto, esperar) = mpsc::channel();
    let outra = std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(caminho).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        pronto.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        conn.execute_batch("UPDATE investimentos SET observacao = 'mercado'; COMMIT")
            .unwrap();
    });
    esperar.recv().unwrap();
    let r = aplicar_importacao(&db, &c, &[decisao(&chave, Acao::UsarB3)], "p.xlsx", hoje());
    outra.join().unwrap();
    let r = r.expect("a aplicação deve esperar a outra escrita terminar");
    assert_eq!((r.aplicadas, r.erros.len()), (1, 0));
    assert_eq!(db.investimento(id, hoje()).unwrap().quantidade, Some(150));
}
