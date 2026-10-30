use chrono::NaiveDate;
use guru_core::{
    Aporte, Classe, Db, Gravacao, MAX_CENTAVOS, NovoInvestimento, ResultadoAporte, SituacaoAporte,
    dividir_aporte,
};

const RF: Classe = Classe::Rf;
const ACOES: Classe = Classe::Acoes;
const FII: Classe = Classe::Fii;
const EXT: Classe = Classe::Ext;
const OUTROS: Classe = Classe::Outros;

fn hoje() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
}

fn parte(a: &Aporte, c: Classe) -> &guru_core::ParteAporte {
    a.partes.iter().find(|p| p.classe == c).unwrap()
}

fn soma(a: &Aporte) -> i64 {
    a.partes.iter().map(|p| p.sugerido_centavos).sum()
}

#[test]
fn exemplo_do_plano() {
    let a = dividir_aporte(
        200_000,
        &[(RF, 600_000, 40), (ACOES, 200_000, 40), (FII, 200_000, 20)],
    );
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 200_000);
    assert_eq!(parte(&a, FII).sugerido_centavos, 0);
    assert_eq!(parte(&a, RF).sugerido_centavos, 0);
    assert_eq!(parte(&a, ACOES).percentual_depois_decimos, 333);
    assert_eq!(parte(&a, RF).percentual_depois_decimos, 500);
    assert_eq!(parte(&a, FII).percentual_depois_decimos, 167);
    assert_eq!(a.patrimonio_centavos, 1_000_000);
}

#[test]
fn carteira_vazia_divide_pelo_alvo() {
    let a = dividir_aporte(100_000, &[(RF, 0, 40), (ACOES, 0, 40), (FII, 0, 20)]);
    assert_eq!(parte(&a, RF).sugerido_centavos, 40_000);
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 40_000);
    assert_eq!(parte(&a, FII).sugerido_centavos, 20_000);
    assert!(
        a.partes
            .iter()
            .all(|p| p.situacao == SituacaoAporte::Abaixo)
    );
}

#[test]
fn soma_exata_no_valor_minimo() {
    let cinco: Vec<_> = Classe::TODOS.iter().map(|c| (*c, 0, 20)).collect();
    let a = dividir_aporte(100, &cinco);
    assert_eq!(soma(&a), 100);
    assert!(a.partes.iter().all(|p| p.sugerido_centavos == 20));
}

#[test]
fn soma_exata_com_restos() {
    let a = dividir_aporte(100, &[(RF, 0, 34), (ACOES, 0, 33), (FII, 0, 33)]);
    assert_eq!(soma(&a), 100);
    assert_eq!(parte(&a, RF).sugerido_centavos, 34);
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 33);
    assert_eq!(parte(&a, FII).sugerido_centavos, 33);
}

#[test]
fn restos_desempatam_pela_ordem_das_classes() {
    let a = dividir_aporte(1, &[(RF, 0, 50), (ACOES, 0, 50)]);
    assert_eq!(parte(&a, RF).sugerido_centavos, 1);
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 0);
    // a ordem de entrada não importa
    let b = dividir_aporte(1, &[(ACOES, 0, 50), (RF, 0, 50)]);
    assert_eq!(parte(&b, RF).sugerido_centavos, 1);
}

#[test]
fn classe_acima_do_alvo_nao_recebe() {
    let a = dividir_aporte(10_000, &[(RF, 900_000, 50), (ACOES, 100_000, 50)]);
    assert_eq!(parte(&a, RF).sugerido_centavos, 0);
    assert_eq!(parte(&a, RF).situacao, SituacaoAporte::Acima);
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 10_000);
    assert_eq!(parte(&a, ACOES).situacao, SituacaoAporte::Abaixo);
}

#[test]
fn aporte_grande_preenche_todas_as_classes() {
    let a = dividir_aporte(
        100_000_000,
        &[(RF, 600_000, 40), (ACOES, 200_000, 40), (FII, 200_000, 20)],
    );
    assert_eq!(soma(&a), 100_000_000);
    for p in &a.partes {
        let alvo = i64::from(p.alvo_percentual) * 10;
        assert!((p.percentual_depois_decimos - alvo).abs() <= 1, "{p:?}");
    }
}

#[test]
fn classe_com_alvo_zero_fica_fora() {
    let a = dividir_aporte(
        10_000,
        &[
            (RF, 0, 50),
            (ACOES, 0, 50),
            (OUTROS, 100_000, 0),
            (EXT, 0, 0),
        ],
    );
    let o = parte(&a, OUTROS);
    assert_eq!(o.sugerido_centavos, 0);
    assert_eq!(o.situacao, SituacaoAporte::ForaDoAlvo);
    assert!(o.percentual_depois_decimos < o.percentual_atual_decimos);
    // sem posição e sem alvo, não é listada
    assert!(a.partes.iter().all(|p| p.classe != EXT));
    assert_eq!(soma(&a), 10_000);
}

#[test]
fn situacao_perto_dentro_de_um_ponto() {
    // 20,5% com alvo 20 → Perto
    let a = dividir_aporte(100, &[(RF, 205, 20), (ACOES, 795, 80)]);
    assert_eq!(parte(&a, RF).percentual_atual_decimos, 205);
    assert_eq!(parte(&a, RF).situacao, SituacaoAporte::Perto);
    // 21,0% → Acima
    let b = dividir_aporte(100, &[(RF, 210, 20), (ACOES, 790, 80)]);
    assert_eq!(parte(&b, RF).situacao, SituacaoAporte::Acima);
}

#[test]
fn ordem_por_sugestao_decrescente() {
    let a = dividir_aporte(100_000, &[(RF, 0, 20), (ACOES, 0, 50), (FII, 0, 30)]);
    let ordem: Vec<Classe> = a.partes.iter().map(|p| p.classe).collect();
    assert_eq!(ordem, [ACOES, FII, RF]);
}

#[test]
fn valor_no_limite_nao_estoura() {
    let a = dividir_aporte(
        MAX_CENTAVOS,
        &[
            (RF, MAX_CENTAVOS / 2, 33),
            (ACOES, 0, 33),
            (FII, 0, 34),
            (EXT, 0, 0),
            (OUTROS, 0, 0),
        ],
    );
    assert_eq!(soma(&a), MAX_CENTAVOS);
    assert!(a.partes.iter().all(|p| p.sugerido_centavos >= 0));
}

fn banco() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path()).unwrap();
    (dir, db)
}

fn salvar_alvo(db: &Db, v: [i64; 5]) {
    let valores: Vec<(String, i64)> = Classe::TODOS
        .iter()
        .zip(v)
        .map(|(c, p)| (c.codigo().to_string(), p))
        .collect();
    assert!(db.salvar_alocacao_alvo(&valores).unwrap().is_empty());
}

fn novo(tipo: &str, valor: i64) -> NovoInvestimento {
    let s = |v: &str| Some(v.to_string());
    if tipo == "rf-cdb" {
        NovoInvestimento {
            tipo: tipo.into(),
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
    } else {
        NovoInvestimento {
            tipo: tipo.into(),
            instituicao: "Banco Inter".into(),
            data_aplicacao: s("2026-04-15"),
            valor_aplicado_centavos: Some(valor),
            ..Default::default()
        }
    }
}

#[test]
fn db_sem_alvo_devolve_sem_alvo() {
    let (_d, db) = banco();
    assert_eq!(
        db.proximo_aporte(hoje(), 100_000).unwrap(),
        ResultadoAporte::SemAlvo
    );
}

#[test]
fn db_valor_invalido_devolve_erros() {
    let (_d, db) = banco();
    for (v, msg) in [
        (0, "Informe um valor de pelo menos R$ 1,00."),
        (99, "Informe um valor de pelo menos R$ 1,00."),
        (MAX_CENTAVOS + 1, "Valor alto demais."),
    ] {
        match db.proximo_aporte(hoje(), v).unwrap() {
            ResultadoAporte::Invalido(e) => {
                assert_eq!(e.len(), 1);
                assert_eq!(e[0].campo, "valorCentavos");
                assert_eq!(e[0].mensagem, msg);
            }
            outro => panic!("esperava Invalido, veio {outro:?}"),
        }
    }
}

#[test]
fn db_usa_valor_atual_do_resumo() {
    let (_d, db) = banco();
    salvar_alvo(&db, [50, 50, 0, 0, 0]);
    let g = db
        .criar_investimento(&novo("rf-cdb", 300_000), hoje())
        .unwrap();
    assert!(matches!(g, Gravacao::Ok(_)), "{g:?}");
    let resumo = db.resumo_carteira(hoje()).unwrap();
    let atual_rf = resumo
        .classes
        .iter()
        .find(|c| c.classe == RF)
        .unwrap()
        .total_atual_centavos;
    let ResultadoAporte::Dividido(a) = db.proximo_aporte(hoje(), 100_000).unwrap() else {
        panic!("esperava Dividido");
    };
    assert_eq!(a.patrimonio_centavos, atual_rf);
    assert_eq!(parte(&a, RF).atual_centavos, atual_rf);
    assert_eq!(parte(&a, RF).sugerido_centavos, 0);
    assert_eq!(parte(&a, ACOES).sugerido_centavos, 100_000);
    assert_eq!(soma(&a), 100_000);
}
