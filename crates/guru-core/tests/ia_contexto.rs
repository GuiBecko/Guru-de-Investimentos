use chrono::NaiveDate;
use guru_core::{
    Cofre, CofreMemoria, Db, ErroCampo, Gravacao, MODELOS, NovaMeta, NovoInvestimento, StatusFonte,
    estimar_tokens, validar_formato_chave,
};

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

fn cdb(valor: i64, instituicao: &str, emissor: &str, meta_id: Option<i64>) -> NovoInvestimento {
    NovoInvestimento {
        tipo: "rf-cdb".into(),
        instituicao: instituicao.into(),
        emissor: s(emissor),
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

fn criar(db: &Db, n: &NovoInvestimento) {
    match db.criar_investimento(n, hoje()).unwrap() {
        Gravacao::Ok(_) => {}
        Gravacao::Invalido(e) => panic!("inválido: {e:?}"),
    }
}

fn carteira_com_meta(db: &Db, instituicao: &str, emissor: &str) {
    let meta = db
        .criar_meta(
            &NovaMeta {
                nome: "Reserva de emergência".into(),
                valor_alvo_centavos: Some(2_500_000),
                prazo: s("2028-12"),
            },
            hoje(),
        )
        .unwrap()
        .unwrap();
    criar(db, &cdb(900_000, instituicao, emissor, Some(meta)));
}

#[test]
fn config_padrao() {
    let (_d, db) = banco();
    let c = db.config_ia().unwrap();
    assert_eq!(c.modelo, "claude-opus-5-5");
    assert_eq!(c.instrucoes, "");
    assert!(c.apenas_percentuais);
    assert!(!c.nomes_instituicoes);
    assert!(c.busca_web);
    assert_eq!(c.chave_final, None);
    assert_eq!(MODELOS.len(), 3);
    assert_eq!(MODELOS[0], ("claude-opus-5-5", "Opus 5.5 · mais capaz"));
    assert_eq!(MODELOS[1], ("claude-sonnet-5-5", "Sonnet 5.5 · equilíbrio"));
    assert_eq!(MODELOS[2], ("claude-haiku-5-5", "Haiku 5.5 · mais barato"));
}

#[test]
fn salvar_config_valida_modelo_e_instrucoes() {
    let (_d, db) = banco();
    let erros = db
        .salvar_config_ia("gpt-9", &"a".repeat(4001), false, true, false)
        .unwrap()
        .erros;
    assert_eq!(
        erros,
        vec![
            ErroCampo {
                campo: "modelo".into(),
                mensagem: "Escolha um dos modelos da lista.".into()
            },
            ErroCampo {
                campo: "instrucoes".into(),
                mensagem: "Use no máximo 4.000 caracteres.".into()
            },
        ]
    );
    // nada foi gravado
    assert_eq!(db.config_ia().unwrap().modelo, "claude-opus-5-5");
    assert!(db.config_ia().unwrap().apenas_percentuais);

    let ok = db
        .salvar_config_ia("claude-haiku-5-5", "Seja breve.", false, true, false)
        .unwrap();
    assert!(ok.erros.is_empty());
    let c = db.config_ia().unwrap();
    assert_eq!(c.modelo, "claude-haiku-5-5");
    assert_eq!(c.instrucoes, "Seja breve.");
    assert!(!c.apenas_percentuais && c.nomes_instituicoes && !c.busca_web);
}

#[test]
fn contexto_com_apenas_percentuais_nao_tem_reais() {
    let (_d, db) = banco();
    carteira_com_meta(&db, "Banco Inter", "Banco Inter");
    let texto = db.contexto_montado(hoje()).unwrap().texto;
    assert!(texto.contains("Carteira"));
    assert!(texto.contains("Reserva de emergência"));
    for proibido in ["R$", "9000", "9.000", "900000", "25.000", "2500000"] {
        assert!(!texto.contains(proibido), "vazou {proibido}: {texto}");
    }
    // desligando a chave, os valores aparecem
    db.salvar_config_ia("claude-opus-5-5", "", false, false, true)
        .unwrap();
    let texto = db.contexto_montado(hoje()).unwrap().texto;
    assert!(texto.contains("R$"), "{texto}");
}

#[test]
fn contexto_sem_instituicoes_omite_nomes() {
    let (_d, db) = banco();
    carteira_com_meta(&db, "Banco Fictício", "Emissor Exemplo");
    let partes = db.partes_contexto(hoje()).unwrap();
    let texto = db.contexto_montado(hoje()).unwrap().texto;
    for nome in ["Banco Fictício", "Emissor Exemplo"] {
        assert!(!texto.contains(nome), "{texto}");
        assert!(partes.iter().all(|p| !p.texto.contains(nome)));
    }
    db.salvar_config_ia("claude-opus-5-5", "", true, true, true)
        .unwrap();
    let texto = db.contexto_montado(hoje()).unwrap().texto;
    assert!(texto.contains("Banco Fictício"), "{texto}");
    assert!(texto.contains("Emissor Exemplo"), "{texto}");
}

#[test]
fn partes_na_ordem_e_com_resumo() {
    let (_d, db) = banco();
    carteira_com_meta(&db, "Banco Inter", "Banco Inter");
    let partes = db.partes_contexto(hoje()).unwrap();
    let codigos: Vec<_> = partes.iter().map(|p| p.parte).collect();
    assert_eq!(
        codigos,
        ["instrucoes", "perfil", "carteira", "mercado", "metas"]
    );
    let rotulos: Vec<_> = partes.iter().map(|p| p.rotulo).collect();
    assert_eq!(
        rotulos,
        [
            "Suas instruções",
            "Perfil e objetivo",
            "Carteira",
            "Mercado",
            "Metas"
        ]
    );
    let carteira = &partes[2];
    assert_eq!(carteira.resumo, "automático · 1 investimento");
    assert!(carteira.ligada && !carteira.editado);
    assert_eq!(carteira.tokens, estimar_tokens(&carteira.texto));
}

#[test]
fn parte_desligada_fica_fora_do_montado() {
    let (_d, db) = banco();
    carteira_com_meta(&db, "Banco Inter", "Banco Inter");
    assert!(
        db.salvar_parte_contexto("metas", false, None)
            .unwrap()
            .is_empty()
    );
    let partes = db.partes_contexto(hoje()).unwrap();
    let metas = partes.iter().find(|p| p.parte == "metas").unwrap();
    assert!(!metas.ligada);
    assert!(metas.texto.contains("Reserva de emergência")); // a tela ainda mostra
    let m = db.contexto_montado(hoje()).unwrap();
    assert!(!m.texto.contains("Reserva de emergência"));
    assert!(!m.texto.contains("## Metas"));
    assert!(m.fontes_internas.iter().all(|f| f.parte != "metas"));
    assert!(m.fontes_internas.iter().any(|f| f.parte == "carteira"));
}

#[test]
fn parte_editada_vai_literal_e_volta_ao_automatico() {
    let (_d, db) = banco();
    carteira_com_meta(&db, "Banco Inter", "Banco Inter");
    let literal = "  Minha carteira é só   CDB.\nNada mais. ";
    assert!(
        db.salvar_parte_contexto("carteira", true, Some(literal))
            .unwrap()
            .is_empty()
    );
    let partes = db.partes_contexto(hoje()).unwrap();
    let c = partes.iter().find(|p| p.parte == "carteira").unwrap();
    assert!(c.editado);
    assert_eq!(c.texto, literal);
    assert_eq!(c.resumo, "editado por você");
    let montado = db.contexto_montado(hoje()).unwrap().texto;
    assert!(montado.contains(&format!("## Carteira\n{literal}")));

    db.salvar_parte_contexto("carteira", true, None).unwrap();
    let c = db
        .partes_contexto(hoje())
        .unwrap()
        .into_iter()
        .find(|p| p.parte == "carteira")
        .unwrap();
    assert!(!c.editado);
    assert_ne!(c.texto, literal);
    assert!(c.resumo.starts_with("automático"));
}

#[test]
fn salvar_parte_recusa_desconhecida_longa_e_instrucoes_nao_guarda_edicao() {
    let (_d, db) = banco();
    let e = db.salvar_parte_contexto("segredos", true, None).unwrap();
    assert_eq!(e[0].campo, "parte");
    let e = db
        .salvar_parte_contexto("carteira", true, Some(&"a".repeat(20_001)))
        .unwrap();
    assert_eq!(e[0].campo, "textoEditado");
    assert_eq!(e[0].mensagem, "Use no máximo 20.000 caracteres.");
    assert!(
        db.salvar_parte_contexto("instrucoes", true, Some("outro"))
            .unwrap()
            .is_empty()
    );
    db.salvar_config_ia("claude-opus-5-5", "Seja direto.", true, false, true)
        .unwrap();
    let i = &db.partes_contexto(hoje()).unwrap()[0];
    assert!(!i.editado);
    assert_eq!(i.texto, "Seja direto.");
    assert!(
        db.contexto_montado(hoje())
            .unwrap()
            .texto
            .contains("Seja direto.")
    );
}

#[test]
fn montado_termina_com_a_data() {
    let (_d, db) = banco();
    let m = db.contexto_montado(hoje()).unwrap();
    assert!(m.texto.ends_with("Hoje é 29/10/2026."), "{}", m.texto);
    assert_eq!(m.tokens, estimar_tokens(&m.texto));
}

#[test]
fn mercado_antigo_depois_de_7_dias() {
    let (_d, db) = banco();
    let status = |dias: i64| StatusFonte {
        fonte: "cdi".into(),
        ultima_data: Some((hoje() - chrono::Days::new(dias as u64)).to_string()),
        atualizado_em: None,
        erro: None,
    };
    let antigo = |db: &Db| {
        db.contexto_montado(hoje())
            .unwrap()
            .fontes_internas
            .into_iter()
            .find(|f| f.parte == "mercado")
            .unwrap()
    };
    db.registrar_status(&status(10)).unwrap();
    let f = antigo(&db);
    assert!(f.antigo);
    assert_eq!(f.data_dados, Some(hoje() - chrono::Days::new(10)));
    db.registrar_status(&status(7)).unwrap();
    assert!(!antigo(&db).antigo);
}

#[test]
fn estimativa_de_tokens() {
    assert_eq!(estimar_tokens(""), 0);
    assert_eq!(estimar_tokens("abcd"), 1);
    assert_eq!(estimar_tokens("abcde"), 2);
    assert_eq!(estimar_tokens("ççççç"), 2);
}

#[test]
fn formato_da_chave() {
    let msg = "Isso não parece uma chave da Anthropic (começa com sk-ant-).";
    for ruim in [
        "abc-123456789012345678901234",
        "sk-ant-curta",
        "sk-ant-com espaco 123456789012345",
        &format!("sk-ant-{}", "a".repeat(300)),
        "sk-ant-api03-acentuação-1234",
        "sk-ant-api03-controle\u{7}-1234",
        "sk-ant-api03-tab\tno-meio-1234",
        "sk-ant-api03-del\u{7f}-123456",
    ] {
        let e = validar_formato_chave(ruim).expect(ruim);
        assert_eq!(e.campo, "chave");
        assert_eq!(e.mensagem, msg);
        assert!(!e.mensagem.contains(ruim));
    }
    assert!(validar_formato_chave("sk-ant-api03-abcdefghijklmnop").is_none());
    // Espaços e quebra de linha nas pontas (colar a chave) não contam.
    assert!(validar_formato_chave("  sk-ant-api03-abcdefghijklmnop\n").is_none());
}

#[test]
fn cofre_memoria_le_grava_apaga() {
    let c = CofreMemoria::default();
    assert_eq!(c.ler().unwrap(), None);
    c.gravar("sk-ant-teste-1234567890").unwrap();
    assert_eq!(c.ler().unwrap().as_deref(), Some("sk-ant-teste-1234567890"));
    c.apagar().unwrap();
    assert_eq!(c.ler().unwrap(), None);
    c.apagar().unwrap();
}

#[test]
fn erro_do_cofre_tem_codigo_proprio() {
    let e = guru_core::CoreError::CofreIndisponivel;
    assert_eq!(e.code(), "IA_COFRE_INDISPONIVEL");
    assert_eq!(
        e.to_string(),
        "Não foi possível usar o cofre de senhas do sistema."
    );
}

fn carteira(db: &Db) -> guru_core::ParteContexto {
    db.partes_contexto(hoje())
        .unwrap()
        .into_iter()
        .find(|p| p.parte == "carteira")
        .unwrap()
}

#[test]
fn ligar_apenas_percentuais_volta_carteira_e_metas_editadas_ao_automatico() {
    let (_d, db) = banco();
    criar(
        &db,
        &cdb(1_000_000, "Banco Fictício", "Banco Fictício", None),
    );
    let r = db
        .salvar_config_ia("claude-opus-5-5", "", false, true, true)
        .unwrap();
    assert!(r.erros.is_empty() && !r.edicoes_restauradas);
    db.salvar_parte_contexto("carteira", true, Some("Tenho R$ 10.000,00 no CDB."))
        .unwrap();
    db.salvar_parte_contexto("metas", true, Some("Meta de R$ 50.000,00."))
        .unwrap();
    db.salvar_parte_contexto("perfil", true, Some("Meu perfil."))
        .unwrap();
    assert!(db.contexto_montado(hoje()).unwrap().texto.contains("R$"));

    let r = db
        .salvar_config_ia("claude-opus-5-5", "", true, true, true)
        .unwrap();
    assert!(r.erros.is_empty() && r.edicoes_restauradas);
    let texto = db.contexto_montado(hoje()).unwrap().texto;
    assert!(!texto.contains("R$"), "{texto}");
    let partes = db.partes_contexto(hoje()).unwrap();
    for p in &partes {
        // Perfil não depende das chaves: a edição fica.
        assert_eq!(p.editado, p.parte == "perfil", "{}", p.parte);
    }
    // Salvar de novo, sem mudar a privacidade, não restaura nada.
    let r = db
        .salvar_config_ia("claude-opus-5-5", "", true, true, true)
        .unwrap();
    assert!(!r.edicoes_restauradas);
}

#[test]
fn desligar_nomes_das_instituicoes_volta_so_a_carteira() {
    let (_d, db) = banco();
    db.salvar_config_ia("claude-opus-5-5", "", true, true, true)
        .unwrap();
    db.salvar_parte_contexto("carteira", true, Some("Tudo no Banco Fictício."))
        .unwrap();
    db.salvar_parte_contexto("metas", true, Some("Minhas metas."))
        .unwrap();
    let r = db
        .salvar_config_ia("claude-opus-5-5", "", true, false, true)
        .unwrap();
    assert!(r.edicoes_restauradas);
    assert!(!carteira(&db).editado);
    assert!(
        db.partes_contexto(hoje())
            .unwrap()
            .iter()
            .any(|p| p.parte == "metas" && p.editado)
    );
}

#[test]
fn afrouxar_a_privacidade_mantem_as_edicoes() {
    let (_d, db) = banco();
    db.salvar_config_ia("claude-opus-5-5", "", true, false, true)
        .unwrap();
    db.salvar_parte_contexto("carteira", true, Some("Minha carteira."))
        .unwrap();
    // Desligar "Apenas percentuais" e ligar "Nome das instituições" só afrouxam.
    let r = db
        .salvar_config_ia("claude-opus-5-5", "", false, true, true)
        .unwrap();
    assert!(!r.edicoes_restauradas);
    let c = carteira(&db);
    assert!(c.editado);
    assert_eq!(c.texto, "Minha carteira.");
}

#[test]
fn config_com_erro_nao_restaura_edicoes() {
    let (_d, db) = banco();
    db.salvar_config_ia("claude-opus-5-5", "", false, true, true)
        .unwrap();
    db.salvar_parte_contexto("carteira", true, Some("R$ 1,00"))
        .unwrap();
    let r = db.salvar_config_ia("gpt-9", "", true, true, true).unwrap();
    assert!(!r.erros.is_empty() && !r.edicoes_restauradas);
    assert!(carteira(&db).editado);
}
