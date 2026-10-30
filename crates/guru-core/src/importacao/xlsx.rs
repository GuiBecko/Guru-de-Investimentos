//! Leitor mínimo e seguro de `.xlsx`: só abas, linhas e células (texto ou número).

use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use zip::ZipArchive;

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Celula {
    Vazia,
    Texto(String),
    /// Texto bruto da célula numérica, para virar `Decimal` sem passar por `f64`.
    Numero(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aba {
    pub nome: String,
    pub linhas: Vec<Vec<Celula>>,
}

const MSG_NAO_XLSX: &str = "Este arquivo não é uma planilha do Excel (.xlsx).";
const MSG_GRANDE: &str = "O arquivo é grande demais para uma planilha de posição.";

const MAX_ARQUIVO: usize = 20 * 1024 * 1024;
const MAX_PARTE: u64 = 50 * 1024 * 1024;
const MAX_ABAS: usize = 50;
const MAX_LINHAS: usize = 100_000;
const MAX_COLUNAS: usize = 16_384;

fn nao_xlsx() -> CoreError {
    CoreError::ArquivoInvalido(MSG_NAO_XLSX.to_string())
}

fn grande_demais() -> CoreError {
    CoreError::ArquivoInvalido(MSG_GRANDE.to_string())
}

type Zip<'a> = ZipArchive<Cursor<&'a [u8]>>;

/// Lê as abas do `.xlsx`, na ordem do `workbook.xml`.
pub fn ler_xlsx(bytes: &[u8]) -> Result<Vec<Aba>, CoreError> {
    if bytes.len() > MAX_ARQUIVO {
        return Err(grande_demais());
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| nao_xlsx())?;

    let workbook = ler_parte(&mut zip, "xl/workbook.xml")?.ok_or_else(nao_xlsx)?;
    let folhas = ler_folhas(&workbook)?;
    if folhas.len() > MAX_ABAS {
        return Err(grande_demais());
    }
    let rels = match ler_parte(&mut zip, "xl/_rels/workbook.xml.rels")? {
        Some(b) => ler_rels(&b)?,
        None => Vec::new(),
    };
    let compartilhadas = match ler_parte(&mut zip, "xl/sharedStrings.xml")? {
        Some(b) => ler_compartilhadas(&b)?,
        None => Vec::new(),
    };

    let mut abas = Vec::new();
    for (nome, rid) in folhas {
        let Some(alvo) = rels.iter().find(|(id, _)| *id == rid).map(|(_, a)| a) else {
            continue; // aba sem relação de planilha (ex.: folha de gráfico)
        };
        let xml = ler_parte(&mut zip, alvo)?.ok_or_else(nao_xlsx)?;
        abas.push(Aba {
            nome,
            linhas: ler_aba(&xml, &compartilhadas)?,
        });
    }
    Ok(abas)
}

/// Lê uma parte do ZIP, parando se passar de 50 MB descompactados.
fn ler_parte(zip: &mut Zip<'_>, nome: &str) -> Result<Option<Vec<u8>>, CoreError> {
    let parte = match zip.by_name(nome) {
        Ok(p) => p,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(_) => return Err(nao_xlsx()),
    };
    let mut buf = Vec::new();
    parte
        .take(MAX_PARTE + 1)
        .read_to_end(&mut buf)
        .map_err(|_| nao_xlsx())?;
    if buf.len() as u64 > MAX_PARTE {
        return Err(grande_demais());
    }
    Ok(Some(buf))
}

fn leitor(xml: &[u8]) -> Reader<&[u8]> {
    Reader::from_reader(xml)
}

fn atributo(e: &BytesStart<'_>, nome: &str) -> Result<Option<String>, CoreError> {
    for a in e.attributes() {
        let a = a.map_err(|_| nao_xlsx())?;
        if a.key.local_name().as_ref() == nome {
            let v = a
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| nao_xlsx())?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

/// Acrescenta o texto de um evento (texto, CDATA ou entidade) a `destino`.
fn acumular(ev: &Event<'_>, destino: &mut String) {
    match ev {
        Event::Text(t) => destino.push_str(&t.xml10_content()),
        Event::CData(c) => destino.push_str(c),
        Event::GeneralRef(r) => {
            if let Ok(Some(c)) = r.resolve_char_ref() {
                destino.push(c);
            } else {
                match &**r {
                    "amp" => destino.push('&'),
                    "lt" => destino.push('<'),
                    "gt" => destino.push('>'),
                    "quot" => destino.push('"'),
                    "apos" => destino.push('\''),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Pares (nome, id de relação) das abas, em ordem.
fn ler_folhas(xml: &[u8]) -> Result<Vec<(String, String)>, CoreError> {
    let mut r = leitor(xml);
    let mut folhas = Vec::new();
    loop {
        match r.read_event().map_err(|_| nao_xlsx())? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "sheet" => {
                let nome = atributo(&e, "name")?.ok_or_else(nao_xlsx)?;
                let rid = atributo(&e, "id")?.ok_or_else(nao_xlsx)?;
                folhas.push((nome, rid));
            }
            Event::Eof => return Ok(folhas),
            _ => {}
        }
    }
}

/// Pares (id, caminho da parte no ZIP) das relações do tipo planilha.
fn ler_rels(xml: &[u8]) -> Result<Vec<(String, String)>, CoreError> {
    let mut r = leitor(xml);
    let mut rels = Vec::new();
    loop {
        match r.read_event().map_err(|_| nao_xlsx())? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "Relationship" => {
                let tipo = atributo(&e, "Type")?.unwrap_or_default();
                if !tipo.ends_with("/worksheet") {
                    continue;
                }
                let id = atributo(&e, "Id")?.ok_or_else(nao_xlsx)?;
                let alvo = atributo(&e, "Target")?.ok_or_else(nao_xlsx)?;
                rels.push((id, caminho_da_parte(&alvo)));
            }
            Event::Eof => return Ok(rels),
            _ => {}
        }
    }
}

/// Alvo absoluto (`/xl/...`) ou relativo a `xl/`, sem segmentos `.` e `..`.
fn caminho_da_parte(alvo: &str) -> String {
    let completo = match alvo.strip_prefix('/') {
        Some(abs) => abs.to_string(),
        None => format!("xl/{alvo}"),
    };
    let mut partes: Vec<&str> = Vec::new();
    for seg in completo.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                partes.pop();
            }
            s => partes.push(s),
        }
    }
    partes.join("/")
}

/// Coleta os `<t>` de um `<si>` ou `<is>`, ignorando a fonética (`<rPh>`).
#[derive(Default)]
struct TextoRico {
    texto: String,
    em_t: bool,
    em_fonetica: bool,
}

impl TextoRico {
    fn inicio(&mut self, nome: &str) {
        match nome {
            "t" if !self.em_fonetica => self.em_t = true,
            "rPh" => self.em_fonetica = true,
            _ => {}
        }
    }

    fn fim(&mut self, nome: &str) {
        match nome {
            "t" => self.em_t = false,
            "rPh" => self.em_fonetica = false,
            _ => {}
        }
    }

    fn evento(&mut self, ev: &Event<'_>) {
        if self.em_t && !self.em_fonetica {
            acumular(ev, &mut self.texto);
        }
    }
}

fn ler_compartilhadas(xml: &[u8]) -> Result<Vec<String>, CoreError> {
    let mut r = leitor(xml);
    let mut lista = Vec::new();
    let mut atual: Option<TextoRico> = None;
    loop {
        let ev = r.read_event().map_err(|_| nao_xlsx())?;
        match &ev {
            Event::Start(e) => match (e.local_name().as_ref(), atual.as_mut()) {
                ("si", _) => atual = Some(TextoRico::default()),
                (nome, Some(a)) => a.inicio(nome),
                _ => {}
            },
            Event::Empty(e) if e.local_name().as_ref() == "si" => lista.push(String::new()),
            Event::End(e) => match (e.local_name().as_ref(), atual.as_mut()) {
                ("si", _) => lista.push(atual.take().map(|a| a.texto).unwrap_or_default()),
                (nome, Some(a)) => a.fim(nome),
                _ => {}
            },
            Event::Eof => return Ok(lista),
            _ => {
                if let Some(a) = atual.as_mut() {
                    a.evento(&ev);
                }
            }
        }
    }
}

/// Índice (base 0) da coluna numa referência como `AB12`.
fn coluna_da_referencia(r: &str) -> Option<usize> {
    let mut col = 0usize;
    let mut letras = 0;
    for c in r.chars().take_while(char::is_ascii_alphabetic) {
        col = col * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1);
        letras += 1;
        if letras > 3 {
            return None;
        }
    }
    (letras > 0).then(|| col - 1)
}

/// Número (base 0) da linha numa referência como `AB12`.
fn linha_da_referencia(r: &str) -> Option<usize> {
    let digitos: String = r.chars().skip_while(char::is_ascii_alphabetic).collect();
    digitos.parse::<usize>().ok()?.checked_sub(1)
}

struct CelulaAberta {
    coluna: usize,
    tipo: String,
    valor: String,
    em_v: bool,
    inline: TextoRico,
    em_is: bool,
}

fn ler_aba(xml: &[u8], compartilhadas: &[String]) -> Result<Vec<Vec<Celula>>, CoreError> {
    let mut r = leitor(xml);
    let mut linhas: Vec<Vec<Celula>> = Vec::new();
    let mut linha_atual = 0usize;
    let mut proxima_linha = 0usize;
    let mut proxima_coluna = 0usize;
    let mut aberta: Option<CelulaAberta> = None;

    loop {
        let ev = r.read_event().map_err(|_| nao_xlsx())?;
        match &ev {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "row" => {
                linha_atual = match atributo(e, "r")?.as_deref().and_then(linha_da_referencia) {
                    Some(n) => n,
                    None => proxima_linha,
                };
                if linha_atual >= MAX_LINHAS {
                    return Err(grande_demais());
                }
                if linhas.len() <= linha_atual {
                    linhas.resize(linha_atual + 1, Vec::new());
                }
                proxima_linha = linha_atual + 1;
                proxima_coluna = 0;
            }
            Event::Start(e) if e.local_name().as_ref() == "c" => {
                let coluna = match atributo(e, "r")?.as_deref().and_then(coluna_da_referencia) {
                    Some(c) => c,
                    None => proxima_coluna,
                };
                if coluna >= MAX_COLUNAS {
                    return Err(nao_xlsx());
                }
                proxima_coluna = coluna + 1;
                aberta = Some(CelulaAberta {
                    coluna,
                    tipo: atributo(e, "t")?.unwrap_or_default(),
                    valor: String::new(),
                    em_v: false,
                    inline: TextoRico::default(),
                    em_is: false,
                });
            }
            Event::Empty(e) if e.local_name().as_ref() == "c" => {
                let coluna = match atributo(e, "r")?.as_deref().and_then(coluna_da_referencia) {
                    Some(c) => c,
                    None => proxima_coluna,
                };
                proxima_coluna = coluna + 1;
            }
            Event::Start(e) => {
                if let Some(c) = aberta.as_mut() {
                    match e.local_name().as_ref() {
                        "v" => c.em_v = true,
                        "is" => c.em_is = true,
                        nome if c.em_is => c.inline.inicio(nome),
                        _ => {}
                    }
                }
            }
            Event::End(e) => {
                let nome = e.local_name();
                if nome.as_ref() == "c" {
                    if let Some(c) = aberta.take() {
                        if c.coluna >= MAX_COLUNAS {
                            return Err(nao_xlsx());
                        }
                        let celula = converter(c, compartilhadas)?;
                        let linha = &mut linhas[linha_atual];
                        if linha.len() <= celula.0 {
                            linha.resize(celula.0 + 1, Celula::Vazia);
                        }
                        linha[celula.0] = celula.1;
                    }
                } else if let Some(c) = aberta.as_mut() {
                    match nome.as_ref() {
                        "v" => c.em_v = false,
                        "is" => c.em_is = false,
                        n if c.em_is => c.inline.fim(n),
                        _ => {}
                    }
                }
            }
            Event::Eof => return Ok(linhas),
            _ => {
                if let Some(c) = aberta.as_mut() {
                    if c.em_v {
                        acumular(&ev, &mut c.valor);
                    } else if c.em_is {
                        c.inline.evento(&ev);
                    }
                }
            }
        }
    }
}

fn texto(s: String) -> Celula {
    if s.is_empty() {
        Celula::Vazia
    } else {
        Celula::Texto(s)
    }
}

fn converter(c: CelulaAberta, compartilhadas: &[String]) -> Result<(usize, Celula), CoreError> {
    let valor = c.valor.trim();
    let celula = match c.tipo.as_str() {
        "s" => {
            let i: usize = valor.parse().map_err(|_| nao_xlsx())?;
            texto(compartilhadas.get(i).cloned().ok_or_else(nao_xlsx)?)
        }
        "inlineStr" => texto(c.inline.texto),
        "str" | "d" => texto(c.valor),
        "b" => Celula::Texto(if valor == "1" { "TRUE" } else { "FALSE" }.to_string()),
        "e" => Celula::Vazia,
        _ if valor.is_empty() => Celula::Vazia,
        _ => Celula::Numero(valor.to_string()),
    };
    Ok((c.coluna, celula))
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    const BDR_RF: &[u8] = include_bytes!("../../tests/fixtures/b3/posicao-bdr-rf.xlsx");

    const MSG_NAO_XLSX: &str = "Este arquivo não é uma planilha do Excel (.xlsx).";
    const MSG_GRANDE: &str = "O arquivo é grande demais para uma planilha de posição.";

    fn t(s: &str) -> Celula {
        Celula::Texto(s.to_string())
    }
    fn n(s: &str) -> Celula {
        Celula::Numero(s.to_string())
    }

    fn zip_de(partes: &[(&str, &str)]) -> Vec<u8> {
        let mut z = ZipWriter::new(Cursor::new(Vec::new()));
        for (nome, conteudo) in partes {
            z.start_file(
                *nome,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
            z.write_all(conteudo.as_bytes()).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    fn invalido(r: Result<Vec<Aba>, CoreError>, msg: &str) {
        match r {
            Err(e @ CoreError::ArquivoInvalido(_)) => {
                assert_eq!(e.code(), "ARQUIVO_INVALIDO");
                assert_eq!(e.to_string(), msg);
            }
            outro => panic!("esperava ArquivoInvalido, veio {outro:?}"),
        }
    }

    const WB: &str = r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="A &amp; B" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
    const RELS_ABS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet1.xml"/></Relationships>"#;

    #[test]
    fn le_as_duas_abas_do_arquivo_de_amostra() {
        let abas = ler_xlsx(BDR_RF).unwrap();
        let nomes: Vec<&str> = abas.iter().map(|a| a.nome.as_str()).collect();
        assert_eq!(nomes, ["BDR", "Renda Fixa"]);
        let cab: Vec<Celula> = [
            "Produto",
            "Instituição",
            "Conta",
            "Código de Negociação",
            "Código ISIN / Distribuição",
            "Tipo",
            "Escriturador",
            "Quantidade",
            "Quantidade Disponível",
            "Quantidade Indisponível",
            "Motivo",
            "Preço de Fechamento",
            "Valor Atualizado",
        ]
        .iter()
        .map(|s| t(s))
        .collect();
        let bdr = &abas[0];
        assert_eq!(bdr.linhas[0], cab);
        assert_eq!(bdr.linhas[1][7], n("10"));
        assert_eq!(bdr.linhas[1][10], t("-"));
        assert_eq!(bdr.linhas[1][11], n("71.5"));
    }

    #[test]
    fn linha_vazia_e_totais() {
        let bdr = &ler_xlsx(BDR_RF).unwrap()[0];
        assert_eq!(bdr.linhas[2], vec![Celula::Vazia]);
        assert_eq!(bdr.linhas[3][0], Celula::Vazia);
        assert_eq!(bdr.linhas[3][12], t("Total"));
        assert_eq!(bdr.linhas[4][12], n("715.0"));
    }

    #[test]
    fn bytes_que_nao_sao_zip() {
        invalido(ler_xlsx(b"%PDF-1.7 nao sou zip"), MSG_NAO_XLSX);
        invalido(ler_xlsx(b""), MSG_NAO_XLSX);
    }

    #[test]
    fn zip_sem_workbook() {
        invalido(ler_xlsx(&zip_de(&[("outra.txt", "oi")])), MSG_NAO_XLSX);
    }

    #[test]
    fn parte_acima_de_50_mb_descompactados() {
        let mut z = ZipWriter::new(Cursor::new(Vec::new()));
        z.start_file(
            "xl/workbook.xml",
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .unwrap();
        let bloco = vec![0u8; 1024 * 1024];
        for _ in 0..51 {
            z.write_all(&bloco).unwrap();
        }
        let bytes = z.finish().unwrap().into_inner();
        assert!(bytes.len() < 1024 * 1024);
        invalido(ler_xlsx(&bytes), MSG_GRANDE);
    }

    #[test]
    fn arquivo_robusto_sem_shared_strings_e_com_alvo_absoluto() {
        let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>
<row><c t="inlineStr"><is><r><t>Ri</t></r><r><t xml:space="preserve">ca </t></r><rPh><t>x</t></rPh></is></c><c t="str"><v>f &amp; g</v></c><c t="b"><v>1</v></c><c><f>1+1</f><v>2</v></c></row>
<row r="4"><c r="C4"><v>7.25</v></c><c r="E4" t="e"><v>#DIV/0!</v></c></row>
</sheetData></worksheet>"#;
        let bytes = zip_de(&[
            ("xl/workbook.xml", WB),
            ("xl/_rels/workbook.xml.rels", RELS_ABS),
            ("xl/worksheets/sheet1.xml", sheet),
        ]);
        let abas = ler_xlsx(&bytes).unwrap();
        assert_eq!(abas.len(), 1);
        assert_eq!(abas[0].nome, "A & B");
        let l = &abas[0].linhas;
        assert_eq!(l[0], vec![t("Rica "), t("f & g"), t("TRUE"), n("2")]);
        assert_eq!(l[1], Vec::<Celula>::new());
        assert_eq!(l[2], Vec::<Celula>::new());
        assert_eq!(
            l[3],
            vec![
                Celula::Vazia,
                Celula::Vazia,
                n("7.25"),
                Celula::Vazia,
                Celula::Vazia
            ]
        );
    }

    #[test]
    fn shared_strings_com_rich_text() {
        let ss = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><r><t>Ban</t></r><r><t>co</t></r></si><si><t>Z&#233;</t></si></sst>"#;
        let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="AA1" t="s"><v>0</v></c><c r="AB1" t="s"><v>1</v></c></row></sheetData></worksheet>"#;
        let bytes = zip_de(&[
            ("xl/workbook.xml", WB),
            ("xl/_rels/workbook.xml.rels", RELS_ABS),
            ("xl/sharedStrings.xml", ss),
            ("xl/worksheets/sheet1.xml", sheet),
        ]);
        let l = &ler_xlsx(&bytes).unwrap()[0].linhas;
        assert_eq!(l[0].len(), 28);
        assert_eq!(l[0][26], t("Banco"));
        assert_eq!(l[0][27], t("Zé"));
        assert!(l[0][..26].iter().all(|c| *c == Celula::Vazia));
    }

    #[test]
    fn aba_apontando_para_parte_inexistente() {
        let bytes = zip_de(&[
            ("xl/workbook.xml", WB),
            ("xl/_rels/workbook.xml.rels", RELS_ABS),
        ]);
        invalido(ler_xlsx(&bytes), MSG_NAO_XLSX);
    }

    #[test]
    fn linha_muito_distante_e_grande_demais() {
        let sheet = r#"<worksheet><sheetData><row r="100001"><c r="A100001"><v>1</v></c></row></sheetData></worksheet>"#;
        let bytes = zip_de(&[
            ("xl/workbook.xml", WB),
            ("xl/_rels/workbook.xml.rels", RELS_ABS),
            ("xl/worksheets/sheet1.xml", sheet),
        ]);
        invalido(ler_xlsx(&bytes), MSG_GRANDE);
    }

    #[test]
    fn xml_quebrado_nao_e_planilha() {
        let bytes = zip_de(&[("xl/workbook.xml", "<workbook><sheets></workbook>")]);
        invalido(ler_xlsx(&bytes), MSG_NAO_XLSX);
    }

    #[test]
    fn le_as_outras_amostras() {
        let completa = ler_xlsx(include_bytes!(
            "../../tests/fixtures/b3/posicao-completa.xlsx"
        ))
        .unwrap();
        assert_eq!(completa.len(), 6);
        let desconhecida = ler_xlsx(include_bytes!(
            "../../tests/fixtures/b3/posicao-aba-desconhecida.xlsx"
        ))
        .unwrap();
        assert_eq!(desconhecida.len(), 2);
    }
}
