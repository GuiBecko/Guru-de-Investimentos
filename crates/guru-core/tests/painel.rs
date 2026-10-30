use chrono::NaiveDate;
use guru_core::{
    Classe, Cotacao, Db, Gravacao, NovoInvestimento, PontoDiario, Referencia, acumulado_benchmark,
    benchmark_padrao, datas_painel, historico, ler_cotahist_txt, ler_sgs, ler_tesouro, painel,
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

fn centesimos(fator: Decimal) -> i64 {
    use rust_decimal::prelude::ToPrimitive;
    ((fator - Decimal::ONE) * Decimal::from(10_000) + Decimal::new(5, 1))
        .floor()
        .to_i64()
        .unwrap()
}

fn cotacao(db: &Db, ticker: &str, d: NaiveDate, centavos: i64) {
    db.gravar_cotacoes(&[Cotacao {
        ticker: ticker.into(),
        data: d,
        fechamento_centavos: centavos,
    }])
    .unwrap();
}

#[test]
fn cdi_acumulado_e_o_produto_dos_fatores_da_fixture() {
    let (_d, db) = banco();
    let mut fator = Decimal::ONE;
    for p in ler_sgs(&fixture("sgs-12.json")).unwrap() {
        if p.data > data(2026, 9, 30) && p.data <= hoje() {
            fator *= Decimal::ONE + p.valor / Decimal::from(100);
        }
    }
    let esperado = centesimos(fator);
    assert!(esperado > 0);
    let r = acumulado_benchmark(&db, Referencia::Cdi, &[data(2026, 9, 30), hoje()]).unwrap();
    assert_eq!(r, Some(vec![Some(0), Some(esperado)]));
}

#[test]
fn cdi_sem_dado_no_primeiro_ponto_fica_ausente() {
    let (_d, db) = banco();
    let datas = datas_painel(hoje(), 12).unwrap();
    assert_eq!(
        acumulado_benchmark(&db, Referencia::Cdi, &datas).unwrap(),
        None
    );
    assert_eq!(
        acumulado_benchmark(&db, Referencia::Selic, &datas).unwrap(),
        None
    );
}

#[test]
fn ipca_entra_so_com_os_meses_fechados_depois_do_primeiro_ponto() {
    let (_d, db) = banco();
    let datas = datas_painel(hoje(), 12).unwrap();
    let ipca: Vec<(String, Decimal)> = ler_sgs(&fixture("sgs-433.json"))
        .unwrap()
        .into_iter()
        .map(|p| (p.data.format("%Y-%m").to_string(), p.valor))
        .collect();
    let ate = |mes: &str| {
        let f = ipca
            .iter()
            .filter(|(m, _)| m.as_str() > "2025-10" && m.as_str() <= mes)
            .fold(Decimal::ONE, |a, (_, v)| {
                a * (Decimal::ONE + v / Decimal::from(100))
            });
        centesimos(f)
    };
    let r = acumulado_benchmark(&db, Referencia::Ipca, &datas)
        .unwrap()
        .unwrap();
    assert_eq!(r.len(), 13);
    assert_eq!(r[0], Some(0));
    assert_eq!(r[1], Some(ate("2025-11")));
    assert_eq!(r[11], Some(ate("2026-09")));
    // 28/10 ainda não fechou outubro: igual a 30/09.
    assert_eq!(r[12], Some(ate("2026-09")));
    assert!(ate("2026-09") > ate("2025-11"));
}

#[test]
fn bova11_sem_cotacao_no_primeiro_ponto_fica_ausente() {
    let (_d, db) = banco();
    let datas = datas_painel(hoje(), 12).unwrap();
    assert_eq!(
        acumulado_benchmark(&db, Referencia::Bova11, &datas).unwrap(),
        None
    );
}

#[test]
fn bova11_com_cotacao_sintetica_no_primeiro_ponto_da_a_razao() {
    let (_d, db) = banco();
    let fim = db
        .cotacao_em("BOVA11", hoje())
        .unwrap()
        .unwrap()
        .fechamento_centavos;
    let inicio = fim * 100 / 110;
    cotacao(&db, "BOVA11", data(2026, 9, 30), inicio);
    let datas = [data(2026, 9, 30), data(2026, 10, 15), hoje()];
    let r = acumulado_benchmark(&db, Referencia::Bova11, &datas)
        .unwrap()
        .unwrap();
    let esperado = centesimos(Decimal::from(fim) / Decimal::from(inicio));
    assert_eq!(r[0], Some(0));
    // 15/10 só enxerga a cotação sintética de 30/09: razão 1, retorno 0.
    assert_eq!(r[1], Some(0));
    assert_eq!(r[2], Some(esperado));
    assert!((990..=1010).contains(&esperado), "{esperado}");
}

#[test]
fn benchmark_padrao_por_classe() {
    assert_eq!(benchmark_padrao(Classe::Acoes), Referencia::Bova11);
    assert_eq!(benchmark_padrao(Classe::Rf), Referencia::Cdi);
}

#[test]
fn rendeu_desconta_o_aporte_no_meio_do_periodo() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-02-01")); // 150 × 35,00
    criar(
        &db,
        NovoInvestimento {
            quantidade: Some(100),
            ..itub4("2026-08-10") // aporte no meio: 100 × 35,00
        },
    );
    let p = painel(&db, hoje(), 6, None).unwrap();
    let (v0, vn) = (
        p.historico.pontos[0].patrimonio_centavos,
        p.historico.pontos[6].patrimonio_centavos,
    );
    assert_eq!(v0, 150 * 3500);
    assert_eq!(vn, 250 * 3978);
    // V_último − V₀ − aporte do período.
    assert_eq!(p.numeros.rendeu_centavos, vn - v0 - 100 * 3500);
    assert_eq!(p.numeros.patrimonio_centavos, vn);
    assert_eq!(p.numeros.aportado_centavos, 250 * 3500);
    assert_eq!(
        p.numeros.periodo_centesimos,
        p.historico.pontos[6].acumulado_centesimos
    );
    assert_eq!(
        p.numeros.mes_centesimos,
        p.historico.pontos[6].retorno_centesimos
    );
    assert!(p.numeros.mes_cdi_centesimos.unwrap() > 0);
    // O CDI só existe na fixture a partir de setembro: sem dado em 30/04, o período some.
    assert_eq!(p.numeros.periodo_cdi_centesimos, None);
}

#[test]
fn numeros_do_cdi_quando_ha_dado_no_primeiro_ponto() {
    let (_d, db) = banco();
    criar(&db, cdb("2026-08-10", 1_000_000));
    // 6 meses a partir de 28/10 começam em 30/04: sem CDI; o mês (30/09 a 28/10) existe.
    let p = painel(&db, hoje(), 6, None).unwrap();
    let cdi_mes = acumulado_benchmark(&db, Referencia::Cdi, &[data(2026, 9, 30), hoje()])
        .unwrap()
        .unwrap()[1];
    assert_eq!(p.numeros.mes_cdi_centesimos, cdi_mes);
}

fn em(nome: &str, mut n: NovoInvestimento) -> NovoInvestimento {
    n.instituicao = nome.into();
    n
}

#[test]
fn instituicoes_somam_cem_e_vem_ordenadas() {
    let (_d, db) = banco();
    criar(&db, em("Rico", itub4("2026-10-01")));
    criar(
        &db,
        em(
            "Rico",
            NovoInvestimento {
                tipo: "rv-fii".into(),
                ticker: s("HGLG11"),
                data_aplicacao: s("2026-10-01"),
                quantidade: Some(10),
                preco_medio_centavos: Some(16_000),
                ..Default::default()
            },
        ),
    );
    criar(&db, cdb("2026-09-30", 100_000));
    let p = painel(&db, hoje(), 6, None).unwrap();
    assert_eq!(p.instituicoes.len(), 2);
    assert_eq!(p.instituicoes[0].nome, "Rico");
    assert_eq!(p.instituicoes[1].nome, "Banco Inter");
    assert!(p.instituicoes[0].atual_centavos > p.instituicoes[1].atual_centavos);
    let soma: i64 = p.instituicoes.iter().map(|i| i.percentual_decimos).sum();
    assert_eq!(soma, 1000);
}

#[test]
fn instituicoes_com_classe_so_olham_aquela_classe() {
    let (_d, db) = banco();
    criar(&db, em("Rico", itub4("2026-10-01")));
    criar(&db, cdb("2026-09-30", 100_000));
    let p = painel(&db, hoje(), 6, Some(Classe::Acoes)).unwrap();
    assert_eq!(p.instituicoes.len(), 1);
    assert_eq!(p.instituicoes[0].nome, "Rico");
    assert_eq!(p.instituicoes[0].percentual_decimos, 1000);
}

fn cdb_vence(vencimento: &str, valor: i64) -> NovoInvestimento {
    NovoInvestimento {
        vencimento: s(vencimento),
        ..cdb("2026-01-10", valor)
    }
}

#[test]
fn vencimentos_sem_o_vencido_em_ordem_e_no_maximo_cinco() {
    let (_d, db) = banco();
    criar(&db, cdb_vence("2026-10-01", 100_000)); // já venceu
    for (v, valor) in [
        ("2027-03-01", 100_000),
        ("2026-12-15", 110_000),
        ("2026-10-28", 120_000), // vence hoje: entra
        ("2028-01-01", 130_000),
        ("2027-09-09", 140_000),
        ("2026-11-20", 150_000),
        ("2029-05-05", 160_000),
    ] {
        criar(&db, cdb_vence(v, valor));
    }
    let p = painel(&db, hoje(), 6, None).unwrap();
    let datas: Vec<NaiveDate> = p.vencimentos.iter().map(|v| v.vencimento).collect();
    assert_eq!(
        datas,
        vec![
            data(2026, 10, 28),
            data(2026, 11, 20),
            data(2026, 12, 15),
            data(2027, 3, 1),
            data(2027, 9, 9)
        ]
    );
    assert!(p.vencimentos.iter().all(|v| v.atual_centavos > 0));
}

#[test]
fn variacoes_pedem_cotacao_hoje_e_no_ultimo_fim_de_mes() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-08-01")); // só tem cotação em 28/10
    criar(
        &db,
        NovoInvestimento {
            tipo: "rv-fii".into(),
            instituicao: "Rico".into(),
            ticker: s("HGLG11"),
            data_aplicacao: s("2026-08-01"),
            quantidade: Some(10),
            preco_medio_centavos: Some(16_000),
            ..Default::default()
        },
    );
    criar(&db, cdb("2026-08-01", 100_000));
    let hglg = db
        .cotacao_em("HGLG11", hoje())
        .unwrap()
        .unwrap()
        .fechamento_centavos;
    cotacao(&db, "HGLG11", data(2026, 9, 30), hglg * 100 / 110);
    let p = painel(&db, hoje(), 6, None).unwrap();
    assert_eq!(p.variacoes.len(), 1);
    assert_eq!(p.variacoes[0].ticker, "HGLG11");
    assert!((99..=101).contains(&p.variacoes[0].variacao_decimos));
}

#[test]
fn variacoes_ordenadas_do_maior_para_o_menor() {
    let (_d, db) = banco();
    criar(&db, itub4("2026-08-01"));
    criar(
        &db,
        NovoInvestimento {
            tipo: "rv-fii".into(),
            instituicao: "Rico".into(),
            ticker: s("HGLG11"),
            data_aplicacao: s("2026-08-01"),
            quantidade: Some(10),
            preco_medio_centavos: Some(16_000),
            ..Default::default()
        },
    );
    let itub = db
        .cotacao_em("ITUB4", hoje())
        .unwrap()
        .unwrap()
        .fechamento_centavos;
    let hglg = db
        .cotacao_em("HGLG11", hoje())
        .unwrap()
        .unwrap()
        .fechamento_centavos;
    cotacao(&db, "ITUB4", data(2026, 9, 30), itub * 2); // caiu 50%
    cotacao(&db, "HGLG11", data(2026, 9, 30), hglg); // igual, mas em outra data
    let p = painel(&db, hoje(), 6, None).unwrap();
    let tickers: Vec<&str> = p.variacoes.iter().map(|v| v.ticker.as_str()).collect();
    assert_eq!(tickers, ["HGLG11", "ITUB4"]);
    assert_eq!(p.variacoes[0].variacao_decimos, 0);
    assert_eq!(p.variacoes[1].variacao_decimos, -500);
}

#[test]
fn painel_escolhe_os_benchmarks_pelo_filtro() {
    let (_d, db) = banco();
    criar(&db, cdb("2026-09-30", 1_000_000));
    let geral = painel(&db, hoje(), 6, None).unwrap();
    let cods: Vec<&str> = geral.benchmarks.iter().map(|(r, _)| r.codigo()).collect();
    assert_eq!(cods, ["cdi", "ipca", "bova11"]);
    assert_eq!(geral.datas.len(), 7);
    let rf = painel(&db, hoje(), 6, Some(Classe::Rf)).unwrap();
    let cods: Vec<&str> = rf.benchmarks.iter().map(|(r, _)| r.codigo()).collect();
    assert_eq!(cods, ["cdi", "selic", "ipca", "bova11", "xfix11"]);
}

#[test]
fn painel_invalido_e_vazio() {
    let (_d, db) = banco();
    assert_eq!(
        painel(&db, hoje(), 9, None).unwrap_err().code(),
        "PERIODO_INVALIDO"
    );
    let p = painel(&db, hoje(), 12, None).unwrap();
    assert_eq!(p.numeros.patrimonio_centavos, 0);
    assert_eq!(p.numeros.rendeu_centavos, 0);
    assert!(p.instituicoes.is_empty() && p.vencimentos.is_empty() && p.variacoes.is_empty());
}
