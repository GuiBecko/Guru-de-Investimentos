use chrono::NaiveDate;
use guru_core::{Classe, CoreError, Db, ErroCampo, Gravacao, NovaMeta, NovoInvestimento, Situacao};

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

fn banco() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn meta(nome: &str, valor: i64, prazo: &str) -> NovaMeta {
    NovaMeta {
        nome: nome.into(),
        valor_alvo_centavos: Some(valor),
        prazo: s(prazo),
    }
}

fn cdb(valor: i64, meta_id: Option<i64>) -> NovoInvestimento {
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
        meta_id,
        ..Default::default()
    }
}

fn id_inv(g: Gravacao) -> i64 {
    match g {
        Gravacao::Ok(id) => id,
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn criar(db: &Db, m: &NovaMeta) -> i64 {
    db.criar_meta(m, hoje()).unwrap().unwrap()
}

#[test]
fn cria_e_le_meta_sem_vinculos() {
    let (_d, db) = banco();
    let id = criar(&db, &meta("Viagem", 2_000_000, "2028-12"));
    let m = db.meta(id, hoje()).unwrap();
    assert_eq!(m.nome, "Viagem");
    assert_eq!(m.valor_alvo_centavos, 2_000_000);
    assert_eq!(m.prazo, "2028-12");
    assert_eq!(m.acumulado_centavos, 0);
    assert_eq!(m.investimentos, 0);
    assert_eq!(m.situacao, Situacao::SemAportes);
    assert_eq!(m.meses_restantes, 27);
}

#[test]
fn criar_invalida_devolve_erros_e_nao_grava() {
    let (_d, db) = banco();
    let r = db.criar_meta(&meta("", 0, "2026-09"), hoje()).unwrap();
    let erros = r.unwrap_err();
    let campos: Vec<_> = erros.iter().map(|e| e.campo.as_str()).collect();
    assert_eq!(campos, ["nome", "valorAlvoCentavos", "prazo"]);
    assert!(db.listar_metas(hoje()).unwrap().is_empty());
}

#[test]
fn lista_com_acumulado_dos_vinculados_ordenada_por_prazo_e_nome() {
    let (_d, db) = banco();
    let b = criar(&db, &meta("beta", 1_000_000, "2027-12"));
    let a = criar(&db, &meta("Alfa", 1_000_000, "2027-12"));
    let c = criar(&db, &meta("Cedo", 500_000, "2026-12"));
    id_inv(
        db.criar_investimento(&cdb(300_000, Some(b)), hoje())
            .unwrap(),
    );
    id_inv(
        db.criar_investimento(&cdb(200_000, Some(b)), hoje())
            .unwrap(),
    );
    id_inv(db.criar_investimento(&cdb(900_000, None), hoje()).unwrap());
    let lista = db.listar_metas(hoje()).unwrap();
    let nomes: Vec<_> = lista.iter().map(|m| m.nome.as_str()).collect();
    assert_eq!(nomes, ["Cedo", "Alfa", "beta"]);
    assert_eq!(lista[0].id, c);
    assert_eq!(lista[1].id, a);
    assert_eq!(lista[2].acumulado_centavos, 500_000);
    assert_eq!(lista[2].investimentos, 2);
    assert_eq!(lista[2].percentual_decimos, 500);
    assert_eq!(lista[1].investimentos, 0);
}

#[test]
fn edita_meta_e_aceita_prazo_passado() {
    let (_d, db) = banco();
    let id = criar(&db, &meta("Viagem", 2_000_000, "2026-11"));
    // Meses depois, o prazo da meta já passou; editar o nome continua possível.
    let depois = NaiveDate::from_ymd_opt(2027, 3, 1).unwrap();
    db.atualizar_meta(id, &meta("Viagem longa", 2_500_000, "2026-11"), depois)
        .unwrap()
        .unwrap();
    let m = db.meta(id, depois).unwrap();
    assert_eq!(m.nome, "Viagem longa");
    assert_eq!(m.valor_alvo_centavos, 2_500_000);
    assert_eq!(m.situacao, Situacao::Vencida);
    assert_eq!(m.meses_restantes, 0);
}

#[test]
fn edita_invalida_nao_altera() {
    let (_d, db) = banco();
    let id = criar(&db, &meta("Viagem", 2_000_000, "2027-11"));
    let erros = db
        .atualizar_meta(id, &meta("", 2_000_000, "2027-11"), hoje())
        .unwrap()
        .unwrap_err();
    assert_eq!(
        erros,
        vec![ErroCampo {
            campo: "nome".into(),
            mensagem: "Informe o nome da meta.".into()
        }]
    );
    assert_eq!(db.meta(id, hoje()).unwrap().nome, "Viagem");
}

#[test]
fn ids_inexistentes_dao_nao_encontrado() {
    let (_d, db) = banco();
    for e in [
        db.meta(99, hoje()).unwrap_err(),
        db.atualizar_meta(99, &meta("X", 100, "2027-01"), hoje())
            .unwrap_err(),
        db.apagar_meta(99).unwrap_err(),
    ] {
        assert_eq!(e.code(), "NAO_ENCONTRADO");
        assert!(matches!(e, CoreError::MetaNaoEncontrada(99)));
    }
}

#[test]
fn apagar_meta_desvincula_e_mantem_o_investimento() {
    let (_d, db) = banco();
    let m = criar(&db, &meta("Viagem", 2_000_000, "2028-12"));
    let inv = id_inv(
        db.criar_investimento(&cdb(300_000, Some(m)), hoje())
            .unwrap(),
    );
    assert_eq!(db.investimento(inv, hoje()).unwrap().meta_id, Some(m));
    db.apagar_meta(m).unwrap();
    let i = db.investimento(inv, hoje()).unwrap();
    assert_eq!(i.meta_id, None);
    assert_eq!(i.meta_nome, None);
    assert!(db.listar_metas(hoje()).unwrap().is_empty());
}

#[test]
fn investimento_com_meta_inexistente_da_erro_de_campo() {
    let (_d, db) = banco();
    let r = db
        .criar_investimento(&cdb(300_000, Some(42)), hoje())
        .unwrap();
    let Gravacao::Invalido(erros) = r else {
        panic!("deveria ser inválido")
    };
    assert_eq!(
        erros,
        vec![ErroCampo {
            campo: "metaId".into(),
            mensagem: "Escolha uma meta da lista.".into()
        }]
    );
    assert!(db.listar_investimentos(hoje()).unwrap().is_empty());
}

#[test]
fn erro_de_meta_junta_com_os_da_validacao() {
    let (_d, db) = banco();
    let mut novo = cdb(0, Some(42));
    novo.valor_aplicado_centavos = None;
    let Gravacao::Invalido(erros) = db.criar_investimento(&novo, hoje()).unwrap() else {
        panic!("deveria ser inválido")
    };
    let campos: Vec<_> = erros.iter().map(|e| e.campo.as_str()).collect();
    assert_eq!(campos, ["valorAplicadoCentavos", "metaId"]);
}

#[test]
fn investimento_com_meta_valida_traz_o_nome_e_atualizar_troca_ou_remove() {
    let (_d, db) = banco();
    let m1 = criar(&db, &meta("Viagem", 2_000_000, "2028-12"));
    let m2 = criar(&db, &meta("Casa", 9_000_000, "2030-12"));
    let inv = id_inv(
        db.criar_investimento(&cdb(300_000, Some(m1)), hoje())
            .unwrap(),
    );
    let i = db.investimento(inv, hoje()).unwrap();
    assert_eq!(
        (i.meta_id, i.meta_nome.as_deref()),
        (Some(m1), Some("Viagem"))
    );
    assert_eq!(
        db.listar_investimentos(hoje()).unwrap()[0]
            .meta_nome
            .as_deref(),
        Some("Viagem")
    );

    db.atualizar_investimento(inv, &cdb(300_000, Some(m2)), hoje())
        .unwrap();
    assert_eq!(
        db.investimento(inv, hoje()).unwrap().meta_nome.as_deref(),
        Some("Casa")
    );
    let Gravacao::Invalido(e) = db
        .atualizar_investimento(inv, &cdb(300_000, Some(777)), hoje())
        .unwrap()
    else {
        panic!("deveria ser inválido")
    };
    assert_eq!(e[0].campo, "metaId");
    db.atualizar_investimento(inv, &cdb(300_000, None), hoje())
        .unwrap();
    assert_eq!(db.investimento(inv, hoje()).unwrap().meta_id, None);
}

#[test]
fn resumo_traz_percentuais_e_classes_com_alvo_sem_investimento() {
    let (_d, db) = banco();
    // Sem alvo definido: só as classes com investimento, alvo None.
    id_inv(
        db.criar_investimento(&cdb(3_800_000, None), hoje())
            .unwrap(),
    );
    id_inv(
        db.criar_investimento(
            &NovoInvestimento {
                tipo: "rv-acao".into(),
                instituicao: "Rico".into(),
                emissor: s("Itaú"),
                ticker: s("ITUB4"),
                data_aplicacao: s("2026-01-10"),
                quantidade: Some(150),
                preco_medio_centavos: Some(3142),
                ..Default::default()
            },
            hoje(),
        )
        .unwrap(),
    );
    let r = db.resumo_carteira(hoje()).unwrap();
    assert_eq!(r.classes.len(), 2);
    assert!(r.classes.iter().all(|c| c.alvo_percentual.is_none()));
    // 3.800.000 / 4.271.300 = 88,96...%
    assert_eq!(r.classes[0].percentual_decimos, 890);
    assert_eq!(r.classes[1].percentual_decimos, 110);

    let valores: Vec<(String, i64)> = [
        ("rf", 70),
        ("acoes", 20),
        ("fii", 0),
        ("ext", 10),
        ("outros", 0),
    ]
    .iter()
    .map(|(c, v)| (c.to_string(), *v))
    .collect();
    assert!(db.salvar_alocacao_alvo(&valores).unwrap().is_empty());
    let r = db.resumo_carteira(hoje()).unwrap();
    let v: Vec<_> = r
        .classes
        .iter()
        .map(|c| {
            (
                c.classe,
                c.total_aplicado_centavos,
                c.quantidade,
                c.alvo_percentual,
            )
        })
        .collect();
    assert_eq!(
        v,
        [
            (Classe::Rf, 3_800_000, 1, Some(70)),
            (Classe::Acoes, 471_300, 1, Some(20)),
            (Classe::Ext, 0, 0, Some(10)),
        ]
    );
    assert_eq!(r.classes[2].percentual_decimos, 0);
}

#[test]
fn resumo_vazio_com_alvo_lista_as_classes_do_alvo_com_total_zero() {
    let (_d, db) = banco();
    let valores: Vec<(String, i64)> = [
        ("rf", 85),
        ("acoes", 5),
        ("fii", 5),
        ("ext", 5),
        ("outros", 0),
    ]
    .iter()
    .map(|(c, v)| (c.to_string(), *v))
    .collect();
    db.salvar_alocacao_alvo(&valores).unwrap();
    let r = db.resumo_carteira(hoje()).unwrap();
    assert_eq!(r.total_aplicado_centavos, 0);
    assert_eq!(r.classes.len(), 4);
    assert!(
        r.classes
            .iter()
            .all(|c| c.percentual_decimos == 0 && c.quantidade == 0)
    );
}
