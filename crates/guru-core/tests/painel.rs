use chrono::NaiveDate;
use guru_core::{
    Classe, Db, Gravacao, NovoInvestimento, PontoDiario, historico, ler_cotahist_txt, ler_sgs,
    ler_tesouro,
};
use rust_decimal::Decimal;

fn data(a: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, d).unwrap()
}

fn hoje() -> NaiveDate {
    data(2026, 10, 28)
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

fn fixture(nome: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{nome}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn banco() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    let cdi: Vec<PontoDiario> = ler_sgs(&fixture("sgs-12.json")).unwrap();
    db.gravar_indice("cdi", &cdi).unwrap();
    db.gravar_indice("selic", &ler_sgs(&fixture("sgs-11.json")).unwrap())
        .unwrap();
    let ipca: Vec<(String, Decimal)> = ler_sgs(&fixture("sgs-433.json"))
        .unwrap()
        .into_iter()
        .map(|p| (p.data.format("%Y-%m").to_string(), p.valor))
        .collect();
    db.gravar_ipca(&ipca).unwrap();
    db.gravar_tesouro(&ler_tesouro(&fixture("tesouro.csv")).unwrap())
        .unwrap();
    db.gravar_cotacoes(&ler_cotahist_txt(&fixture("COTAHIST_D28102026.TXT")).unwrap())
        .unwrap();
    (dir, db)
}

fn criar(db: &Db, n: NovoInvestimento) {
    match db.criar_investimento(&n, hoje()).unwrap() {
        Gravacao::Ok(_) => {}
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn cdb(aplicacao: &str, valor: i64) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: "Banco Inter".into(),
        emissor: s("Banco Inter"),
        indexador: s("pos"),
        taxa: s("100"),
        benchmark: s("cdi"),
        data_aplicacao: s(aplicacao),
        vencimento: s("2030-03-15"),
        valor_aplicado_centavos: Some(valor),
        liquidez: s("vencimento"),
        ir: s("regressiva"),
        ..Default::default()
    }
}

fn itub4(aplicacao: &str) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rv-acao".into(),
        instituicao: "Rico".into(),
        ticker: s("ITUB4"),
        data_aplicacao: s(aplicacao),
        quantidade: Some(150),
        preco_medio_centavos: Some(3500),
        ..Default::default()
    }
}

#[test]
fn periodo_invalido_tem_codigo() {
    let (_d, db) = banco();
    let e = historico(&db, hoje(), 7, None).unwrap_err();
    assert_eq!(e.code(), "PERIODO_INVALIDO");
}

#[test]
fn acao_aplicada_em_outubro_so_aparece_hoje() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-10-01"));
    let h = historico(&db, hoje(), 12, None).unwrap();
    assert_eq!(h.pontos.len(), 13);
    for p in &h.pontos[..12] {
        assert_eq!(p.patrimonio_centavos, 0);
        assert_eq!(p.aportado_centavos, 0);
    }
    let ultimo = h.pontos.last().unwrap();
    assert_eq!(ultimo.patrimonio_centavos, 150 * 3978);
    assert_eq!(ultimo.aportado_centavos, 150 * 3500);
}

#[test]
fn carteira_nova_nao_tem_retorno_nem_acumulado() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-10-01"));
    let h = historico(&db, hoje(), 6, None).unwrap();
    for p in &h.pontos {
        assert_eq!(p.retorno_centesimos, None);
        assert_eq!(p.acumulado_centesimos, 0);
    }
}

#[test]
fn carteira_vazia_nao_quebra() {
    let (_d, db) = banco();
    let h = historico(&db, hoje(), 6, None).unwrap();
    assert_eq!(h.pontos.len(), 7);
    assert!(h.por_classe.is_empty());
    assert!(h.pontos.iter().all(|p| p.patrimonio_centavos == 0));
}

#[test]
fn cdb_aplicado_no_ultimo_dia_do_mes_entra_naquele_ponto() {
    let (_d, db) = banco();
    criar(&db, cdb("2026-09-30", 1_000_000));
    let h = historico(&db, hoje(), 6, None).unwrap();
    let set = &h.pontos[5];
    assert_eq!(set.data, data(2026, 9, 30));
    assert_eq!(set.patrimonio_centavos, 1_000_000);
    assert_eq!(set.aportado_centavos, 1_000_000);
    assert_eq!(h.pontos[4].aportado_centavos, 0);
    // Em 28/10 o CDB rendeu: primeiro retorno válido, sobre o patrimônio de 30/09.
    let ultimo = h.pontos.last().unwrap();
    assert!(ultimo.patrimonio_centavos > 1_000_000);
    assert!(ultimo.retorno_centesimos.unwrap() > 0);
    assert_eq!(set.retorno_centesimos, None);
    assert_eq!(
        ultimo.acumulado_centesimos,
        ultimo.retorno_centesimos.unwrap()
    );
}

#[test]
fn aporte_no_intervalo_sai_do_retorno() {
    let (_d, db) = banco();
    criar(&db, cdb("2026-08-15", 1_000_000));
    criar(&db, cdb("2026-10-05", 500_000));
    let h = historico(&db, hoje(), 6, None).unwrap();
    let ultimo = h.pontos.last().unwrap();
    let set = &h.pontos[5];
    // O aporte de outubro não pode inflar o retorno: ele ficaria perto de 50% se entrasse.
    let r = ultimo.retorno_centesimos.unwrap();
    assert!(r > 0 && r < 200, "retorno {r}");
    assert_eq!(ultimo.aportado_centavos, 1_500_000);
    assert_eq!(set.aportado_centavos, 1_000_000);
}

#[test]
fn filtro_de_classe_ignora_o_cdb() {
    let (_d, db) = banco();
    criar(&db, cdb("2026-09-30", 1_000_000));
    criar(&db, itub4("2026-10-01"));
    let h = historico(&db, hoje(), 6, Some(Classe::Acoes)).unwrap();
    assert!(h.pontos[..6].iter().all(|p| p.patrimonio_centavos == 0));
    assert_eq!(h.pontos[6].patrimonio_centavos, 150 * 3978);
    assert_eq!(h.por_classe.len(), 1);
    assert_eq!(h.por_classe[0].0, Classe::Acoes);
}

#[test]
fn por_classe_na_ordem_do_catalogo_e_soma_o_patrimonio() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-10-01"));
    criar(&db, cdb("2026-09-30", 1_000_000));
    let h = historico(&db, hoje(), 6, None).unwrap();
    let classes: Vec<Classe> = h.por_classe.iter().map(|(c, _)| *c).collect();
    assert_eq!(classes, vec![Classe::Rf, Classe::Acoes]);
    for (i, p) in h.pontos.iter().enumerate() {
        let soma: i64 = h.por_classe.iter().map(|(_, v)| v[i]).sum();
        assert_eq!(soma, p.patrimonio_centavos);
    }
}
