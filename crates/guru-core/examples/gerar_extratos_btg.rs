//! Gera os extratos FICTÍCIOS da BTG usados como amostras de teste.
//!
//! Reproduz o layout do extrato real (coordenadas, células quebradas em linhas
//! físicas, cabeçalho e rodapé em toda página de dados) e o defeito dos rótulos:
//! o `a` minúsculo e as ligaduras `fi`/`fl` somem (ver `rotulo`). Nenhum dado
//! pessoal: tudo aqui é inventado.
//!
//! Uso: `cargo run -p guru-core --example gerar_extratos_btg -- <pasta>`

use std::path::{Path, PathBuf};

use lopdf::{Document, Object, Stream, dictionary};

const LARGURA: f32 = 841.92;
const ALTURA: f32 = 594.96;
const PERIODO: &str = "01/10/26 a 31/10/26";

/// Reproduz o defeito dos rótulos do PDF real: some o `a` minúsculo e `fi`/`fl`.
fn rotulo(texto: &str) -> String {
    texto.replace("fi", "").replace("fl", "").replace('a', "")
}

/// Codifica em WinAnsi (o que importa aqui: Latin-1 e o travessão).
fn winansi(texto: &str) -> Vec<u8> {
    let mut saida = Vec::new();
    for c in texto.chars() {
        let byte = match c {
            '\u{2013}' => 0x96,
            c if (c as u32) < 0x100 => c as u32 as u8,
            c => panic!("caractere fora do WinAnsi: {c:?}"),
        };
        if matches!(byte, b'(' | b')' | b'\\') {
            saida.push(b'\\');
        }
        saida.push(byte);
    }
    saida
}

/// Uma página: cada célula é um `BT … Tf … Td (texto) Tj ET` próprio.
struct Pagina {
    conteudo: Vec<u8>,
}

impl Pagina {
    fn nova() -> Self {
        Pagina {
            conteudo: Vec::new(),
        }
    }

    fn cel(&mut self, x: f32, y: f32, tamanho: f32, texto: &str) {
        self.conteudo
            .extend(format!("BT /F1 {tamanho} Tf {x} {y} Td (").into_bytes());
        self.conteudo.extend(winansi(texto));
        self.conteudo.extend(b") Tj ET\n");
    }

    /// Dado (fonte 7,5).
    fn d(&mut self, x: f32, y: f32, texto: &str) {
        self.cel(x, y, 7.5, texto);
    }

    /// Cabeçalho de coluna (fonte 7), sem as letras perdidas.
    fn h(&mut self, x: f32, y: f32, texto: &str) {
        self.cel(x, y, 7.0, &rotulo(texto));
    }

    /// Título de seção (fonte 10), sem as letras perdidas.
    fn t(&mut self, y: f32, texto: &str) {
        self.cel(23.0, y, 10.0, &rotulo(texto));
    }

    /// Número de nota de rodapé solto (fonte 5).
    fn nota(&mut self, x: f32, y: f32, n: &str) {
        self.cel(x, y, 5.0, n);
    }

    /// Cabeçalho e rodapé que a BTG repete em toda página de dados.
    fn moldura(&mut self, numero: u32) {
        self.cel(23.0, 541.0, 10.0, &rotulo("Extrato da Conta Investimento"));
        self.cel(23.0, 526.0, 8.0, &rotulo(&format!("Período de {PERIODO}")));
        self.cel(815.0, 17.0, 7.0, &numero.to_string());
        self.cel(
            572.0,
            10.0,
            6.0,
            &rotulo(
                "Disclaimers: Seção com ativos calculados em data anterior à data da geração do extrato",
            ),
        );
    }
}

fn dados(numero: u32) -> Pagina {
    let mut p = Pagina::nova();
    p.moldura(numero);
    p
}

// ---------------------------------------------------------------- fundos

struct Lote {
    data: &'static str,
    cotas: &'static str,
    cotacao: &'static str,
    compra: &'static str,
    bruto: &'static str,
    ir: &'static str,
    liquido: &'static str,
}

struct Fundo {
    nome: &'static str,
    cnpj: &'static str,
    referencia: &'static str,
    anterior: &'static str,
    cotas: &'static str,
    cotacao: &'static str,
    bruto: &'static str,
    ir: &'static str,
    liquido: &'static str,
    variacao: &'static str,
    lotes: Vec<Lote>,
}

fn fundos() -> Vec<Fundo> {
    vec![
        Fundo {
            nome: "FUNDO EXEMPLO RF LP",
            cnpj: "11.111.111/0001-11",
            referencia: "30/10/26",
            anterior: "1.040,00",
            cotas: "1.000,000000",
            cotacao: "1,050000",
            bruto: "1.050,00",
            ir: "5,00",
            liquido: "1.045,00",
            variacao: "5,00",
            lotes: vec![
                Lote {
                    data: "02/01/26",
                    cotas: "400,000000",
                    cotacao: "1,000000",
                    compra: "400,00",
                    bruto: "420,00",
                    ir: "2,00",
                    liquido: "418,00",
                },
                Lote {
                    data: "05/03/26",
                    cotas: "600,000000",
                    cotacao: "1,000000",
                    compra: "600,00",
                    bruto: "630,00",
                    ir: "3,00",
                    liquido: "627,00",
                },
            ],
        },
        Fundo {
            nome: "FUNDO TESTE MULTIMERCADO",
            cnpj: "22.222.222/0001-22*",
            referencia: "30/10/26",
            anterior: "208,00",
            cotas: "200,000000",
            cotacao: "1,050000",
            bruto: "210,00",
            ir: "1,00",
            liquido: "209,00",
            variacao: "1,00",
            lotes: vec![Lote {
                data: "10/02/26",
                cotas: "200,000000",
                cotacao: "1,000000",
                compra: "200,00",
                bruto: "210,00",
                ir: "1,00",
                liquido: "209,00",
            }],
        },
    ]
}

/// Posição de fundos a partir do título em `t`. Devolve o y da linha de total.
fn fundos_posicao(p: &mut Pagina, t: f32, fundos: &[Fundo]) -> f32 {
    p.t(t, "Fundo de Investimento - Posição - Portfólio de fundos");
    let c = t - 17.0; // cabeçalho, linha 1 (y=470 quando t=487)
    for (x, s) in [
        (130.0, "Saldo"),
        (220.0, "Quantidade"),
        (330.0, "Cotação"),
        (420.0, "Saldo"),
        (500.0, "Provisão"),
        (580.0, "Provisão"),
        (660.0, "Saldo"),
        (740.0, "Variação"),
    ] {
        p.h(x, c, s);
    }
    p.nota(790.0, c, "1");
    for (x, s) in [
        (23.0, "Data Referência"),
        (130.0, "Líquido R$"),
        (220.0, "de Cotas"),
        (330.0, "Atual R$"),
        (420.0, "Bruto R$"),
        (500.0, "de IR R$"),
        (580.0, "de IOF R$"),
        (660.0, "Líquido R$"),
        (740.0, "Nominal R$"),
    ] {
        p.h(x, c - 9.0, s);
    }
    p.d(130.0, c - 18.0, fundos[0].referencia);

    let mut y = t - 51.0; // título do primeiro fundo (y=436)
    for f in fundos {
        p.d(27.0, y, &format!("{} - Classe CNPJ: {}", f.nome, f.cnpj));
        let l = y - 15.0;
        p.d(26.0, l, f.referencia);
        p.d(150.0, l, f.anterior);
        p.d(240.0, l, f.cotas);
        p.d(350.0, l, f.cotacao);
        p.d(440.0, l, f.bruto);
        p.d(520.0, l, f.ir);
        p.d(600.0, l, "-");
        p.d(680.0, l, f.liquido);
        p.d(760.0, l, f.variacao);
        y -= 40.0;
    }
    let total = y + 40.0 - 15.0 - 20.0;
    p.h(27.0, total, "Total em fundos");
    p.d(440.0, total, "1.260,00");
    p.d(520.0, total, "6,00");
    p.d(600.0, total, "-");
    p.d(680.0, total, "1.254,00");
    p.d(760.0, total, "6,00");
    total
}

/// Detalhamento de um fundo a partir do título em `t`. Devolve o y do total.
fn fundo_detalhamento(p: &mut Pagina, t: f32, f: &Fundo) -> f32 {
    p.t(
        t,
        &format!(
            "Fundo de Investimento - Detalhamento - {} - Classe CNPJ: {}",
            f.nome, f.cnpj
        ),
    );
    let c = t - 26.0; // cabeçalho (y=461 quando t=487)
    p.nota(407.0, c + 1.0, "1");
    p.nota(691.0, c + 1.0, "2");
    p.nota(787.0, c + 1.0, "3");
    for (x, s) in [
        (23.0, "Data Compra"),
        (90.0, "Quantidade de Cotas"),
        (201.0, "Cotação Compra R$"),
        (305.0, "Valor de Compra Ajustado R$"),
        (454.0, "Saldo Bruto R$"),
        (531.0, "Provisão de IR R$"),
        (622.0, "Provisão de IOF R$"),
        (726.0, "Saldo Líquido R$"),
    ] {
        p.h(x, c, s);
    }
    let mut y = c - 15.0; // primeiro lote (y=446)
    for l in &f.lotes {
        p.d(26.0, y, l.data);
        p.d(147.0, y, l.cotas);
        p.d(255.0, y, l.cotacao);
        p.d(423.0, y, l.compra);
        p.d(500.0, y, l.bruto);
        p.d(600.0, y, l.ir);
        p.d(788.0, y, l.liquido);
        p.d(672.0, y - 1.0, "-");
        y -= 15.0;
    }
    let total = y + 15.0 - 21.0;
    p.h(27.0, total, "Total");
    p.d(129.0, total, f.cotas);
    p.d(500.0, total, f.bruto);
    p.d(600.0, total, f.ir);
    p.d(672.0, total, "-");
    p.d(788.0, total, f.liquido);
    total
}

// ------------------------------------------------------------ renda fixa

struct Rf {
    ativo: (&'static str, &'static str),
    emissao: &'static str,
    vencimento: &'static str,
    aquisicao: &'static str,
    liquidez: &'static str,
    /// Taxa em duas linhas físicas (acima e abaixo da linha dos números).
    taxa: (&'static str, &'static str),
    compra: &'static str,
    bruto: &'static str,
    ir: &'static str,
    iof: &'static str,
    liquido: &'static str,
}

/// Seção de renda fixa a partir do título em `t`. Devolve o y dos totais.
fn rf_secao(p: &mut Pagina, t: f32, titulo: &str, r: &Rf) -> f32 {
    p.t(t, titulo);
    let c = t - 26.0;
    p.h(258.0, c, "Dias de carência para");
    p.h(341.0, c, "Data inicial de");
    p.h(404.0, c, "Taxa");
    p.h(489.0, c, "Preço Compra");
    p.h(542.0, c, "Valor Compra");
    p.h(651.0, c, "Saldo Bruto");
    p.h(776.0, c, "Saldo");
    p.nota(255.0, c - 3.0, "1");
    for (x, s) in [
        (23.0, "Ativo"),
        (85.0, "Emissão"),
        (128.0, "Vencimento"),
        (173.0, "Aquisição"),
        (215.0, "Liquidez"),
        (445.0, "Quantidade"),
        (590.0, "Preço R$"),
        (695.0, "IR R$"),
        (735.0, "IOF R$"),
    ] {
        p.h(x, c - 5.0, s);
    }
    for (x, s) in [
        (258.0, "liquidez"),
        (341.0, "liquidez"),
        (404.0, "Compra"),
        (489.0, "R$"),
        (542.0, "R$"),
        (651.0, "R$"),
        (776.0, "Líquido R$"),
    ] {
        p.h(x, c - 9.0, s);
    }
    let y = t - 53.0; // âncora (y=434)
    p.d(27.0, y + 3.0, r.ativo.0);
    p.d(27.0, y - 6.0, r.ativo.1);
    p.d(409.0, y + 3.0, r.taxa.0);
    p.d(409.0, y - 6.0, r.taxa.1);
    p.d(88.0, y, r.emissao);
    p.d(131.0, y, r.vencimento);
    p.d(176.0, y, r.aquisicao);
    p.d(220.0, y - 1.0, r.liquidez);
    p.d(298.0, y - 1.0, "-");
    p.d(371.0, y - 1.0, "-");
    p.d(472.0, y, "1");
    p.d(493.0, y, r.compra);
    p.d(550.0, y, r.compra);
    p.d(593.0, y, r.bruto);
    p.d(655.0, y, r.bruto);
    p.d(713.0, y, r.ir);
    p.d(749.0, y, r.iof);
    p.d(779.0, y, r.liquido);
    let totais = y - 24.0;
    p.d(550.0, totais, r.compra);
    p.d(655.0, totais, r.bruto);
    p.d(713.0, totais, r.ir);
    p.d(749.0, totais, r.iof);
    p.d(779.0, totais, r.liquido);
    totais
}

fn cdb() -> Rf {
    Rf {
        ativo: ("CDB-", "CDBA26FICT1"),
        emissao: "15/01/26",
        vencimento: "15/01/29",
        aquisicao: "15/01/26",
        liquidez: "Não",
        taxa: ("12,50%", "a.a."),
        compra: "5.000,00",
        bruto: "5.400,00",
        ir: "60,00",
        iof: "0,00",
        liquido: "5.340,00",
    }
}

fn lca() -> Rf {
    Rf {
        ativo: ("LCA-", "LCA26EXEMP"),
        emissao: "01/02/26",
        vencimento: "01/02/28",
        aquisicao: "01/02/26",
        liquidez: "Não",
        taxa: ("95,00% do", "CDI"),
        compra: "2.000,00",
        bruto: "2.120,00",
        ir: "0,00",
        iof: "0,00",
        liquido: "2.120,00",
    }
}

// -------------------------------------------------------- renda variável

struct Acao {
    codigo: String,
    nome: String,
    qtde: String,
    fechamento: String,
    medio: String,
    saldo: String,
}

fn acao(codigo: &str, nome: &str, qtde: &str, fech: &str, medio: &str, saldo: &str) -> Acao {
    Acao {
        codigo: codigo.into(),
        nome: nome.into(),
        qtde: qtde.into(),
        fechamento: fech.into(),
        medio: medio.into(),
        saldo: saldo.into(),
    }
}

fn rv_cabecalho(p: &mut Pagina, y_nota: f32, y_cab: f32, col2: &str) {
    p.nota(637.0, y_nota, "1");
    for (x, s) in [
        (23.0, "Código"),
        (176.0, col2),
        (329.0, "Qtde."),
        (412.0, "Preço Fechamento R$"),
        (579.0, "Preço Médio R$"),
        (705.0, "Saldo Bruto R$"),
    ] {
        p.h(x, y_cab, s);
    }
}

fn rv_linha(p: &mut Pagina, y: f32, a: &Acao) {
    p.d(27.0, y, &a.codigo);
    p.d(180.0, y, &a.nome);
    p.d(403.0, y, &a.qtde);
    p.d(552.0, y, &a.fechamento);
    p.d(679.0, y, &a.medio);
    p.d(793.0, y, &a.saldo);
}

fn rv_total(p: &mut Pagina, y: f32, rotulo_total: &str, saldo: &str) {
    p.d(793.0, y, saldo);
    p.h(27.0, y - 1.0, rotulo_total);
}

/// Seção de renda variável completa (título + cabeçalho + linhas + total) em `t`.
fn rv_secao(
    p: &mut Pagina,
    t: f32,
    titulo: &str,
    col2: &str,
    total: (&str, &str),
    linhas: &[Acao],
) -> f32 {
    p.t(t, titulo);
    rv_cabecalho(p, t - 25.0, t - 27.0, col2);
    let mut y = t - 42.0;
    for a in linhas {
        rv_linha(p, y, a);
        y -= 15.0;
    }
    let y_total = y + 15.0 - 20.0;
    rv_total(p, y_total, total.0, total.1);
    y_total
}

// ------------------------------------------------- conta e cripto (simples)

/// Seção simples: título, cabeçalho e uma linha (data x=26, valor x=788).
fn simples(p: &mut Pagina, t: f32, titulo: &str, descricao: &str, valor: &str) -> f32 {
    p.t(t, titulo);
    p.h(23.0, t - 25.0, "Data");
    p.h(120.0, t - 25.0, "Descrição");
    p.h(788.0, t - 25.0, "Saldo R$");
    p.d(26.0, t - 41.0, "30/10/26");
    p.d(120.0, t - 41.0, descricao);
    p.d(788.0, t - 41.0, valor);
    t - 41.0
}

fn cripto(p: &mut Pagina, t: f32) -> f32 {
    p.t(t, "CriptoAtivos - Posição");
    for (x, s) in [
        (23.0, "Ativo"),
        (330.0, "Qtde."),
        (520.0, "Valor Aplicado R$"),
        (705.0, "Saldo Bruto R$"),
    ] {
        p.h(x, t - 26.0, s);
    }
    p.d(27.0, t - 41.0, "BITCOIN (XBT)");
    p.d(403.0, t - 41.0, "0,001");
    p.d(600.0, t - 41.0, "300,00");
    p.d(793.0, t - 41.0, "350,00");
    t - 41.0
}

// ---------------------------------------------------------------- arquivos

fn capa() -> Pagina {
    let mut p = Pagina::nova();
    p.cel(60.0, 450.0, 28.0, &rotulo("Extrato da"));
    p.cel(60.0, 415.0, 28.0, &rotulo("Conta Investimento"));
    p.cel(60.0, 360.0, 12.0, "FULANO DE TAL");
    p.cel(60.0, 340.0, 10.0, &rotulo("CPF: 000.000.000-00"));
    p.cel(60.0, 325.0, 10.0, &rotulo("Conta: 000000000"));
    p.cel(60.0, 290.0, 10.0, &rotulo(&format!("Período de {PERIODO}")));
    p.cel(60.0, 275.0, 10.0, &rotulo("Emitido em 06/11/26 10:00"));
    p.cel(815.0, 17.0, 7.0, "1");
    p
}

fn disclaimers(numero: u32) -> Pagina {
    let mut p = dados(numero);
    p.t(487.0, "Disclaimers");
    p.cel(
        23.0,
        465.0,
        7.5,
        &rotulo("Este documento é meramente informativo e não substitui os registros oficiais."),
    );
    p.cel(
        23.0,
        450.0,
        7.5,
        &rotulo("Dúvidas: fale com a sua assessoria ou escreva para"),
    );
    p.cel(23.0, 435.0, 7.5, "atendimentoinvestimentos@btgpactual.com");
    p
}

fn extrato_completo() -> Vec<Pagina> {
    let fs = fundos();
    let mut fundos_p = dados(2);
    let total = fundos_posicao(&mut fundos_p, 487.0, &fs);
    let mut t = total - 41.0;
    for f in &fs {
        let fim = fundo_detalhamento(&mut fundos_p, t, f);
        t = fim - 38.0;
    }

    let mut rf = dados(3);
    let fim = rf_secao(
        &mut rf,
        487.0,
        "Renda fixa - Detalhamento - CDB | BANCO FICTICIO S.A",
        &cdb(),
    );
    rf_secao(
        &mut rf,
        fim - 37.0,
        "Renda fixa - Detalhamento - LCA | BANCO EXEMPLO S/A",
        &lca(),
    );

    let mut rv = dados(4);
    let fim = rv_secao(
        &mut rv,
        487.0,
        "Renda variável - Posição - Ações",
        "Ação",
        ("Total em Ações R$", "220,00"),
        &[acao(
            "ABCD3*",
            "EMPRESA EXEMPLO ON",
            "10",
            "22,00",
            "20,00",
            "220,00",
        )],
    );
    let fim = rv_secao(
        &mut rv,
        fim - 37.0,
        "Renda variável - Posição - BDR",
        "Ativo",
        ("Total em BDR's R$", "156,00"),
        &[acao(
            "XYZW34*",
            "EXEMPLO CORP DRN",
            "3",
            "52,00",
            "50,00",
            "156,00",
        )],
    );
    let fim = simples(
        &mut rv,
        fim - 37.0,
        "Conta corrente - Posição",
        "SALDO EM CONTA CORRENTE",
        "150,00",
    );
    simples(
        &mut rv,
        fim - 37.0,
        "Valores em trânsito",
        "TED RECEBIDA",
        "75,00",
    );

    let mut cr = dados(5);
    cripto(&mut cr, 487.0);

    vec![capa(), fundos_p, rf, rv, cr, disclaimers(6)]
}

fn extrato_secao_desconhecida() -> Vec<Pagina> {
    let mut p = dados(1);
    let fim = rf_secao(
        &mut p,
        487.0,
        "Renda fixa - Detalhamento - Tesouro Direto | TESOURO NACIONAL",
        &Rf {
            ativo: ("TESOURO-", "LFT010329"),
            emissao: "01/03/26",
            vencimento: "01/03/29",
            aquisicao: "01/03/26",
            liquidez: "Sim",
            taxa: ("0,10%", "a.a."),
            compra: "1.000,00",
            bruto: "1.030,00",
            ir: "5,00",
            iof: "0,00",
            liquido: "1.025,00",
        },
    );
    let fim = rv_secao(
        &mut p,
        fim - 37.0,
        "Renda variável - Posição - Opções",
        "Ativo",
        ("Total em Opções R$", "50,00"),
        &[acao(
            "ABCDK30",
            "OPCAO EXEMPLO",
            "100",
            "0,50",
            "0,40",
            "50,00",
        )],
    );
    rv_secao(
        &mut p,
        fim - 37.0,
        "Renda variável - Posição - Ações",
        "Ação",
        ("Total em Ações R$", "220,00"),
        &[acao(
            "ABCD3",
            "EMPRESA EXEMPLO ON",
            "10",
            "22,00",
            "20,00",
            "220,00",
        )],
    );
    vec![p, disclaimers(2)]
}

fn extrato_varias_paginas() -> Vec<Pagina> {
    let acoes: Vec<Acao> = (0..30u8)
        .map(|i| {
            let codigo = format!("AA{}{}3", (b'A' + i / 26) as char, (b'A' + i % 26) as char);
            Acao {
                nome: format!("EMPRESA {} ON", &codigo[..4]),
                codigo,
                qtde: (10 + u32::from(i)).to_string(),
                fechamento: "10,00".into(),
                medio: "9,00".into(),
                saldo: format!("{},00", (10 + u32::from(i)) * 10),
            }
        })
        .collect();

    let mut paginas = Vec::new();
    let mut p = dados(1);
    p.t(487.0, "Renda variável - Posição - Ações");
    rv_cabecalho(&mut p, 462.0, 460.0, "Ação");
    let mut y = 445.0;
    for a in &acoes {
        if y < 40.0 {
            paginas.push(p);
            p = dados(paginas.len() as u32 + 1);
            // Continuação: só o cabeçalho de colunas se repete.
            rv_cabecalho(&mut p, 489.0, 487.0, "Ação");
            y = 470.0;
        }
        rv_linha(&mut p, y, a);
        y -= 15.0;
    }
    rv_total(&mut p, y + 15.0 - 20.0, "Total em Ações R$", "7.350,00");
    paginas.push(p);
    paginas.push(disclaimers(paginas.len() as u32 + 1));
    paginas
}

fn outro_banco() -> Vec<Pagina> {
    let mut p = Pagina::nova();
    p.cel(
        23.0,
        541.0,
        10.0,
        "Extrato de Investimentos \u{2013} Banco Qualquer",
    );
    p.cel(23.0, 526.0, 8.0, "Período de 01/10/26 a 31/10/26");
    for (x, s) in [
        (23.0, "Código"),
        (176.0, "Ação"),
        (329.0, "Qtde."),
        (705.0, "Saldo"),
    ] {
        p.cel(x, 487.0, 7.0, s);
    }
    for (i, (c, n)) in [
        ("ABCD3", "EMPRESA EXEMPLO ON"),
        ("EFGH4", "OUTRA EMPRESA PN"),
    ]
    .iter()
    .enumerate()
    {
        let y = 470.0 - 15.0 * i as f32;
        p.cel(27.0, y, 7.5, c);
        p.cel(180.0, y, 7.5, n);
        p.cel(329.0, y, 7.5, "10");
        p.cel(705.0, y, 7.5, "200,00");
    }
    vec![p]
}

/// Monta o PDF: Helvetica padrão com WinAnsiEncoding (acentos corretos).
fn gravar(caminho: &Path, paginas: Vec<Pagina>) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let fonte = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let recursos = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => fonte },
    });
    let mut filhos = Vec::new();
    for p in paginas {
        let conteudo = doc.add_object(Stream::new(dictionary! {}, p.conteudo));
        let pagina = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => conteudo,
        });
        filhos.push(Object::Reference(pagina));
    }
    let n = filhos.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => filhos,
            "Count" => n,
            "Resources" => recursos,
            "MediaBox" => vec![0.into(), 0.into(), LARGURA.into(), ALTURA.into()],
        }),
    );
    let raiz = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", raiz);
    doc.save(caminho).expect("gravar PDF");
}

fn main() {
    let pasta: PathBuf = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/guru-core/tests/fixtures/btg"));
    std::fs::create_dir_all(&pasta).expect("criar pasta");
    for (nome, paginas) in [
        ("extrato-completo.pdf", extrato_completo()),
        (
            "extrato-secao-desconhecida.pdf",
            extrato_secao_desconhecida(),
        ),
        ("extrato-varias-paginas.pdf", extrato_varias_paginas()),
        ("outro-banco.pdf", outro_banco()),
    ] {
        gravar(&pasta.join(nome), paginas);
        println!("gerado {}", pasta.join(nome).display());
    }
}
