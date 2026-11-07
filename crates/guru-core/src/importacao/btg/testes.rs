use std::str::FromStr;

use super::valores::{data, taxa};
use super::*;
use crate::pdf::Celula;

fn l(pagina: u32, y: f64, celulas: &[(f64, &str)]) -> LinhaPdf {
    LinhaPdf {
        pagina,
        y,
        tamanho: 7.35,
        celulas: celulas
            .iter()
            .map(|(x, t)| Celula {
                x: *x,
                texto: (*t).to_string(),
            })
            .collect(),
    }
}

fn d(a: i32, m: u32, dia: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, dia).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Topo da página 1 e o contato da BTG, como no extrato real.
fn extrato(mut corpo: Vec<LinhaPdf>) -> ExtratoBtg {
    let mut linhas = vec![
        l(1, 541.0, &[(23.0, "Extrato d\0 Cont\0 Investimento")]),
        l(1, 526.0, &[(23.0, "Período de 01/10/26 \0 31/10/26")]),
    ];
    linhas.append(&mut corpo);
    linhas.push(l(1, 17.0, &[(815.0, "1")]));
    linhas.push(l(2, 487.0, &[(23.0, "Disclaimers")]));
    linhas.push(l(
        2,
        453.0,
        &[(454.0, "atendimentoinvestimentos@btgpactual.com")],
    ));
    ler_extrato_linhas(&linhas).unwrap()
}

/// Detalhamento de CDB com as posições do extrato real.
fn cdb(taxa_cima: &str, taxa_baixo: &str) -> Vec<LinhaPdf> {
    vec![
        l(
            1,
            487.0,
            &[(
                23.0,
                "Rend\0 \0x\0 - Detalhamento - CDB | BANCO FICTICIO S.A",
            )],
        ),
        l(
            1,
            461.0,
            &[
                (258.0, "Dias de carênci\0 par\0"),
                (341.0, "Dat\0 inicial de"),
                (404.0, "Tax\0"),
                (489.0, "Preço Compr\0"),
                (542.0, "Valor Compr\0"),
                (651.0, "Saldo Bruto"),
                (776.0, "Saldo"),
            ],
        ),
        l(1, 458.0, &[(255.0, "1")]),
        l(
            1,
            456.0,
            &[
                (23.0, "Ativo"),
                (85.0, "Emissão"),
                (128.0, "Vencimento"),
                (173.0, "Aquisição"),
                (215.0, "Liquidez"),
                (445.0, "Quantidade"),
                (590.0, "Preço R$"),
                (695.0, "IR R$"),
                (735.0, "IOF R$"),
            ],
        ),
        l(
            1,
            452.0,
            &[
                (258.0, "liquidez"),
                (341.0, "liquidez"),
                (404.0, "Compr\0"),
                (489.0, "R$"),
                (542.0, "R$"),
                (651.0, "R$"),
                (776.0, "Líquido R$"),
            ],
        ),
        l(1, 437.0, &[(27.0, "CDB-"), (409.0, taxa_cima)]),
        l(
            1,
            434.0,
            &[
                (88.0, "15/01/26"),
                (131.0, "15/01/29"),
                (176.0, "15/01/26"),
                (472.0, "1"),
                (493.0, "5.000,00"),
                (550.0, "5.000,00"),
                (593.0, "5.400,00"),
                (655.0, "5.400,00"),
                (713.0, "60,00"),
                (749.0, "0,00"),
                (779.0, "5.340,00"),
            ],
        ),
        l(1, 433.0, &[(220.0, "Não"), (298.0, "-"), (371.0, "-")]),
        l(1, 428.0, &[(27.0, "CDBA26FICT1"), (409.0, taxa_baixo)]),
        // Totais, sem rótulo, 24 pt abaixo da aquisição.
        l(
            1,
            410.0,
            &[
                (550.0, "5.000,00"),
                (655.0, "5.400,00"),
                (713.0, "60,00"),
                (749.0, "0,00"),
                (779.0, "5.340,00"),
            ],
        ),
    ]
}

#[test]
fn total_nao_entra_na_ultima_linha() {
    let e = extrato(cdb("12,50%", "a.a."));
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    assert_eq!(
        e.ativos,
        [AtivoBtg::RendaFixa {
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
        }]
    );
    assert_eq!(e.data, d(2026, 10, 31));
}

#[test]
fn cabecalho_com_rotulos_em_linhas_diferentes() {
    // Variante vista no extrato real: "Taxa Compra" só na linha do meio e
    // "Valor" / "Compra R$" em linhas separadas.
    let mut linhas = cdb("12,50%", "a.a.");
    linhas[1] = l(
        1,
        461.0,
        &[
            (258.0, "Dias de carênci\0 par\0"),
            (335.0, "Dat\0 inicial de"),
            (490.0, "Preço Compr\0"),
            (543.0, "Valor"),
            (651.0, "Saldo Bruto"),
            (776.0, "Saldo"),
        ],
    );
    linhas[3] = l(
        1,
        456.0,
        &[
            (23.0, "Ativo"),
            (85.0, "Emissão"),
            (128.0, "Vencimento"),
            (173.0, "Aquisição"),
            (215.0, "Liquidez"),
            (396.0, "Tax\0 Compr\0"),
            (446.0, "Quantidade"),
            (590.0, "Preço R$"),
            (695.0, "IR R$"),
            (735.0, "IOF R$"),
        ],
    );
    linhas[4] = l(
        1,
        452.0,
        &[
            (258.0, "liquidez"),
            (335.0, "liquidez"),
            (490.0, "R$"),
            (543.0, "Compr\0 R$"),
            (651.0, "R$"),
            (776.0, "Líquido R$"),
        ],
    );
    let e = extrato(linhas);
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    assert_eq!(e.ativos, extrato(cdb("12,50%", "a.a.")).ativos);
}

#[test]
fn taxa_quebrada_pos_fixada() {
    let e = extrato(cdb("103,00% do", "CDI"));
    assert!(matches!(
        e.ativos[0],
        AtivoBtg::RendaFixa {
            indexador: Some("pos"),
            benchmark: Some("cdi"),
            ..
        }
    ));
}

#[test]
fn taxas() {
    assert_eq!(taxa("12,75% a.a."), (Some("pre"), Some(dec("12.75")), None));
    assert_eq!(
        taxa("103,00% do CDI"),
        (Some("pos"), Some(dec("103.00")), Some("cdi"))
    );
    assert_eq!(
        taxa("IPCA + 6,50% a.a."),
        (Some("ipca"), Some(dec("6.50")), None)
    );
    assert_eq!(
        taxa("IPCA + 6,50%"),
        (Some("ipca"), Some(dec("6.50")), None)
    );
    assert_eq!(taxa("CDI + 1,00%"), (None, None, None));
    assert_eq!(taxa(""), (None, None, None));
    assert_eq!(taxa("IPCA +"), (None, None, None));
}

#[test]
fn ano_com_dois_digitos() {
    assert_eq!(data("07/10/30"), Some(d(2030, 10, 7)));
    assert_eq!(data("31/02/30"), None);
    assert_eq!(data("07/10/2030"), None);
}

fn acoes(qtde: &str) -> Vec<LinhaPdf> {
    vec![
        l(1, 259.0, &[(23.0, "Rend\0 variável - Posição - Ações")]),
        l(
            1,
            232.0,
            &[
                (23.0, "Código"),
                (176.0, "Ação"),
                (329.0, "Qtde."),
                (412.0, "Preço Fechamento R$"),
                (579.0, "Preço Médio R$"),
                (637.0, "1"),
                (705.0, "Saldo Bruto R$"),
            ],
        ),
        l(
            1,
            217.0,
            &[
                (27.0, "ABCD3*"),
                (180.0, "EMPRESA EXEMPLO ON"),
                (403.0, qtde),
                (552.0, "22,00"),
                (679.0, "20,00"),
                (793.0, "33,00"),
            ],
        ),
        l(1, 197.0, &[(793.0, "33,00")]),
        l(1, 196.0, &[(27.0, "Total em Ações R$")]),
    ]
}

#[test]
fn quantidade_fracionaria_de_acao_vira_aviso() {
    let e = extrato(acoes("1,5"));
    assert!(e.ativos.is_empty());
    assert_eq!(
        e.avisos,
        ["Ação ABCD3: quantidade fracionária não importada."]
    );
    assert_eq!(e.linhas_lidas, 1);
    assert_eq!(extrato(acoes("1.000")).ativos.len(), 1);
}

#[test]
fn cripto_sem_coluna_de_saldo_vira_aviso() {
    let e = extrato(vec![
        l(1, 487.0, &[(23.0, "CriptoAtivos - Posição")]),
        l(1, 461.0, &[(23.0, "Ativo"), (330.0, "Qtde.")]),
        l(1, 446.0, &[(27.0, "BITCOIN (XBT)"), (403.0, "0,001")]),
    ]);
    assert!(e.ativos.is_empty());
    assert_eq!(
        e.avisos,
        ["Seção de criptoativos em formato não reconhecido."]
    );
}

fn fundo(rotulo: impl Fn(&str) -> String) -> Vec<LinhaPdf> {
    let titulo = rotulo(
        "Fundo de Investimento - Detalhamento - FUNDO X - RL - Classe CNPJ: 11.111.111/0001-11*",
    );
    let cab = [
        (23.0, "Data Compra"),
        (90.0, "Quantidade de Cotas"),
        (201.0, "Cotação Compra R$"),
        (305.0, "Valor de Compra Ajustado R$"),
        (454.0, "Saldo Bruto R$"),
        (531.0, "Provisão de IR R$"),
        (622.0, "Provisão de IOF R$"),
        (726.0, "Saldo Líquido R$"),
    ]
    .map(|(x, t)| (x, rotulo(t)));
    let cab: Vec<(f64, &str)> = cab.iter().map(|(x, t)| (*x, t.as_str())).collect();
    let lote = |y: f64, data: &str, valor: &str| {
        l(
            1,
            y,
            &[
                (26.0, data),
                (147.0, "400,000000"),
                (255.0, "1,000000"),
                (423.0, valor),
                (500.0, "420,00"),
                (600.0, "2,00"),
                (788.0, "418,00"),
            ],
        )
    };
    vec![
        l(1, 487.0, &[(23.0, titulo.as_str())]),
        l(1, 462.0, &[(407.0, "1"), (691.0, "2"), (787.0, "3")]),
        l(1, 461.0, &cab),
        lote(446.0, "05/03/26", "600,00"),
        l(1, 445.0, &[(672.0, "-")]),
        lote(431.0, "02/01/26", "400,00"),
        l(1, 430.0, &[(672.0, "-")]),
        l(
            1,
            410.0,
            &[(27.0, "Total"), (129.0, "800,000000"), (500.0, "840,00")],
        ),
    ]
}

#[test]
fn rotulos_com_glifos_nulos() {
    let inteiro = extrato(fundo(str::to_string));
    let nulos = extrato(fundo(|s| s.replace('a', "\0").replace("fi", "\0")));
    assert!(inteiro.avisos.is_empty(), "{:?}", inteiro.avisos);
    assert_eq!(inteiro.ativos, nulos.ativos);
    assert_eq!(
        inteiro.ativos,
        [AtivoBtg::Fundo {
            nome: "FUNDO X - RL".into(),
            cnpj: "11111111000111".into(),
            aplicado: dec("1000.00"),
            primeira_compra: d(2026, 1, 2),
            saldo_bruto: None,
            referencia: None,
        }]
    );
}

#[test]
fn posicao_de_fundo_sem_detalhamento_vira_aviso() {
    let e = extrato(vec![
        l(
            1,
            487.0,
            &[(
                23.0,
                "Fundo de Investimento - Posição - Portfólio de fundos",
            )],
        ),
        l(1, 470.0, &[(130.0, "Saldo"), (420.0, "Saldo")]),
        l(
            1,
            461.0,
            &[
                (23.0, "Dat\0 Referênci\0"),
                (130.0, "Líquido R$"),
                (420.0, "Bruto R$"),
            ],
        ),
        l(1, 452.0, &[(130.0, "30/10/26")]),
        l(
            1,
            436.0,
            &[(27.0, "FUNDO Y - Classe CNPJ: 22.222.222/0001-22*")],
        ),
        l(
            1,
            421.0,
            &[(26.0, "30/10/26"), (150.0, "100,00"), (440.0, "105,00")],
        ),
    ]);
    assert!(e.ativos.is_empty());
    assert_eq!(
        e.avisos,
        ["Fundo FUNDO Y: sem detalhamento das aplicações."]
    );
    assert_eq!(e.linhas_lidas, 1);
}

#[test]
fn coluna_obrigatoria_ausente_vira_aviso() {
    let mut linhas = cdb("12,50%", "a.a.");
    linhas[3].celulas.retain(|c| c.texto != "Vencimento");
    let e = extrato(linhas);
    assert!(e.ativos.is_empty());
    assert_eq!(
        e.avisos,
        [
            "Seção \"Renda fixa - Detalhamento - CDB | BANCO FICTICIO S.A\": falta a coluna Vencimento."
        ]
    );
}

#[test]
fn indice_e_notas_depois_dos_disclaimers_nao_viram_secao() {
    let mut corpo = vec![l(
        1,
        477.0,
        &[(32.0, "-\u{a0} Fundo de Investimento"), (200.0, "página 2")],
    )];
    corpo.extend(acoes("10"));
    let mut e = extrato(corpo);
    assert!(e.avisos.is_empty(), "{:?}", e.avisos);
    // Notas da página de disclaimers repetem títulos de seção.
    let mut linhas = vec![
        l(1, 541.0, &[(23.0, "Extrato d\0 Cont\0 Investimento")]),
        l(1, 526.0, &[(23.0, "Período de 01/10/26 \0 31/10/26")]),
        l(1, 537.0, &[(23.0, "Disclaimers")]),
        l(1, 432.0, &[(28.0, "Rend\0 Variável - Posição - Ações")]),
        l(1, 405.0, &[(35.0, "\0. Texto da nota"), (300.0, "1,00")]),
        l(1, 300.0, &[(454.0, "sac@btgpactual.com")]),
    ];
    e = ler_extrato_linhas(&linhas).unwrap();
    assert!(e.ativos.is_empty() && e.avisos.is_empty(), "{e:?}");
    linhas.retain(|l| !l.texto().contains("btgpactual"));
    assert!(ler_extrato_linhas(&linhas).is_err());
}

#[test]
fn secao_sem_ancora_vira_aviso() {
    // Datas num formato que o leitor não conhece: nenhuma âncora.
    let mut linhas = cdb("12,50%", "a.a.");
    for c in &mut linhas[6].celulas {
        c.texto = c.texto.replace("/26", "/2026").replace("/29", "/2029");
    }
    let e = extrato(linhas);
    assert!(e.ativos.is_empty());
    assert_eq!(
        e.avisos,
        [
            "Seção \"Renda fixa - Detalhamento - CDB | BANCO FICTICIO S.A\": nenhuma linha reconhecida."
        ]
    );
}
