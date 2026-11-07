//! Extração de linhas e células de texto de um PDF, sem saber de que banco ele é.
//!
//! O `pdf-extract` entrega cada caractere com a sua posição; aqui eles são
//! agrupados em linhas (por `y`) e em células (por distância entre inícios).

use std::panic::{AssertUnwindSafe, catch_unwind};

use pdf_extract::{Document, MediaBox, OutputDev, OutputError, Transform, output_doc};

use crate::error::CoreError;

/// Máximo de páginas aceitas.
pub const LIMITE_PAGINAS: usize = 200;

const MSG_INVALIDO: &str = "Este arquivo não é um PDF válido.";
const MSG_GRANDE: &str = "O arquivo é grande demais para um extrato da BTG.";

/// Um caractere posicionado. `y` cresce para cima, como no PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct Caractere {
    pub pagina: u32,
    pub x: f64,
    pub y: f64,
    /// Tamanho efetivo da fonte, em pontos.
    pub tamanho: f64,
    /// Largura informada pela fonte, em pontos; `0` quando a fonte não informa
    /// (a fonte mono dos números do extrato da BTG).
    pub largura: f64,
    pub texto: String,
}

/// Trecho de texto contíguo dentro de uma linha.
#[derive(Debug, Clone, PartialEq)]
pub struct Celula {
    /// `x` do primeiro caractere.
    pub x: f64,
    pub texto: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinhaPdf {
    pub pagina: u32,
    pub y: f64,
    /// Maior tamanho de fonte dos caracteres da linha, em pontos.
    pub tamanho: f64,
    pub celulas: Vec<Celula>,
}

impl LinhaPdf {
    /// Texto da linha, com as células unidas por um espaço.
    pub fn texto(&self) -> String {
        self.celulas
            .iter()
            .map(|c| c.texto.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Forma comparável de um rótulo: ignora `\0`, o `a`, as ligaduras `fi`/`fl`,
/// maiúsculas, espaços e pontuação. Os dois lados da comparação passam por aqui,
/// então funciona tanto com o PDF de glifos perdidos quanto com um corrigido.
pub fn esqueleto(texto: &str) -> String {
    texto
        .to_lowercase()
        .replace('\0', "")
        .replace("fi", "")
        .replace("fl", "")
        .replace('a', "")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Começa célula nova quando o início do caractere está a mais de 1,2 × o
/// tamanho da fonte do início do anterior (vale também quando a largura não é
/// informada: na fonte mono os inícios ficam a ~0,6 × e as colunas a ~1,5 ×),
/// ou, quando a fonte informa a largura do anterior, quando sobra um vão de mais
/// de 0,3 × o tamanho depois do fim dele (rótulos de colunas vizinhas a 3 pt).
/// O vão só vale se os inícios também estão a mais de 0,9 × o tamanho: dentro de
/// uma palavra o passo chega a ~0,83 ×, entre rótulos vizinhos fica em ~1,0 ×, e
/// assim uma largura errada (pequena, mas não zero) não parte cada caractere.
fn nova_celula(anterior: &Caractere, c: &Caractere) -> bool {
    let passo = c.x - anterior.x;
    let inicio = passo > 1.2 * c.tamanho;
    let vao = anterior.largura > 0.0
        && passo > 0.9 * c.tamanho
        && c.x - (anterior.x + anterior.largura) > 0.3 * c.tamanho;
    inicio || vao
}

/// Agrupa caracteres em linhas e células. Pura.
pub fn agrupar(mut caracteres: Vec<Caractere>) -> Vec<LinhaPdf> {
    caracteres.sort_by(|a, b| {
        a.pagina
            .cmp(&b.pagina)
            .then(b.y.total_cmp(&a.y))
            .then(a.x.total_cmp(&b.x))
    });

    // Cada grupo: (página, y de referência, caracteres)
    let mut grupos: Vec<(u32, f64, Vec<Caractere>)> = Vec::new();
    for c in caracteres {
        match grupos.last_mut() {
            Some((pagina, y, lista))
                if *pagina == c.pagina && (*y - c.y).abs() <= 0.3 * c.tamanho =>
            {
                lista.push(c);
            }
            _ => grupos.push((c.pagina, c.y, vec![c])),
        }
    }

    grupos
        .into_iter()
        .filter_map(|(pagina, y, mut lista)| {
            lista.sort_by(|a, b| a.x.total_cmp(&b.x));
            let tamanho = lista.iter().map(|c| c.tamanho).fold(0.0, f64::max);
            let mut celulas: Vec<Celula> = Vec::new();
            let mut anterior: Option<&Caractere> = None;
            for c in &lista {
                let nova = anterior.is_none_or(|a| nova_celula(a, c));
                if nova {
                    celulas.push(Celula {
                        x: c.x,
                        texto: String::new(),
                    });
                }
                if let Some(ultima) = celulas.last_mut() {
                    ultima.texto.push_str(&c.texto);
                }
                anterior = Some(c);
            }
            for c in &mut celulas {
                c.texto = c.texto.trim().to_string();
            }
            celulas.retain(|c| !c.texto.is_empty());
            if celulas.is_empty() {
                None
            } else {
                Some(LinhaPdf {
                    pagina,
                    y,
                    tamanho,
                    celulas,
                })
            }
        })
        .collect()
}

struct Coletor {
    pagina: u32,
    caracteres: Vec<Caractere>,
}

impl OutputDev for Coletor {
    fn begin_page(
        &mut self,
        page_num: u32,
        _media_box: &MediaBox,
        _art_box: Option<(f64, f64, f64, f64)>,
    ) -> Result<(), OutputError> {
        self.pagina = page_num;
        Ok(())
    }

    fn end_page(&mut self) -> Result<(), OutputError> {
        Ok(())
    }

    fn output_character(
        &mut self,
        trm: &Transform,
        width: f64,
        _spacing: f64,
        font_size: f64,
        char: &str,
    ) -> Result<(), OutputError> {
        let tamanho = font_size * trm.m22.abs();
        self.caracteres.push(Caractere {
            pagina: self.pagina,
            x: trm.m31,
            y: trm.m32,
            tamanho,
            // `width` vem em fração do corpo (largura do glifo / 1000).
            largura: width.max(0.0) * tamanho,
            texto: char.to_string(),
        });
        Ok(())
    }

    fn begin_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }

    fn end_word(&mut self) -> Result<(), OutputError> {
        Ok(())
    }

    fn end_line(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}

/// Lê o PDF e devolve as linhas, de cima para baixo, página a página.
/// Erro ou pânico dentro do `pdf-extract` viram `ARQUIVO_INVALIDO`.
pub fn extrair_linhas(bytes: &[u8]) -> Result<Vec<LinhaPdf>, CoreError> {
    let invalido = || CoreError::ArquivoInvalido(MSG_INVALIDO.into());
    let resultado = catch_unwind(AssertUnwindSafe(|| {
        let doc = Document::load_mem(bytes).map_err(|_| invalido())?;
        if doc.get_pages().len() > LIMITE_PAGINAS {
            return Err(CoreError::ArquivoInvalido(MSG_GRANDE.into()));
        }
        let mut coletor = Coletor {
            pagina: 0,
            caracteres: Vec::new(),
        };
        output_doc(&doc, &mut coletor).map_err(|_| invalido())?;
        Ok(agrupar(coletor.caracteres))
    }));
    resultado.unwrap_or_else(|_| Err(invalido()))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn chars(pagina: u32, y: f64, tamanho: f64, itens: &[(f64, &str)]) -> Vec<Caractere> {
        itens
            .iter()
            .map(|(x, t)| Caractere {
                pagina,
                x: *x,
                y,
                tamanho,
                largura: 0.0,
                texto: (*t).to_string(),
            })
            .collect()
    }

    fn textos(l: &LinhaPdf) -> Vec<&str> {
        l.celulas.iter().map(|c| c.texto.as_str()).collect()
    }

    #[test]
    fn esqueleto_iguala_rotulo_quebrado_e_inteiro() {
        assert_eq!(
            esqueleto("Rend\0 \0x\0 - Posição - CDB"),
            esqueleto("Renda fixa - Posição - CDB")
        );
        assert_eq!(esqueleto("Dat\0 Compr\0"), esqueleto("Data Compra"));
        assert_eq!(
            esqueleto("Período de 01/10/26 \0 31/10/26"),
            esqueleto("Período de 01/10/26 a 31/10/26")
        );
        assert_ne!(
            esqueleto("Renda fixa - Posição - CDB"),
            esqueleto("Renda variável - Posição - BDR")
        );
    }

    #[test]
    fn agrupar_junta_caracteres_proximos_mesmo_com_largura_errada() {
        let l = agrupar(chars(
            1,
            434.0,
            7.0,
            &[
                (472.0, "7"),
                (477.0, ","),
                (480.0, "0"),
                (493.0, "1"),
                (498.0, "."),
                (500.0, "0"),
            ],
        ));
        assert_eq!(textos(&l[0]), ["7,0", "1.0"]);
    }

    /// Caracteres em passo fixo, com largura informada (0 = não informada).
    fn corrido(
        y: f64,
        tamanho: f64,
        x: f64,
        passo: f64,
        largura: f64,
        texto: &str,
    ) -> Vec<Caractere> {
        texto
            .chars()
            .enumerate()
            .map(|(i, ch)| Caractere {
                pagina: 1,
                x: x + passo * i as f64,
                y,
                tamanho,
                largura,
                texto: ch.to_string(),
            })
            .collect()
    }

    #[test]
    fn agrupar_separa_colunas_mono_proximas() {
        // Datas de emissão e vencimento a ~1,5 × o corpo: fonte mono sem largura.
        let mut c = corrido(434.0, 7.35, 88.0, 4.5, 0.0, "15/01/26");
        c.extend(corrido(434.0, 7.35, 130.7, 4.5, 0.0, "15/01/29"));
        let l = agrupar(c);
        assert_eq!(textos(&l[0]), ["15/01/26", "15/01/29"]);
        assert_eq!(l[0].tamanho, 7.35);
    }

    #[test]
    fn agrupar_separa_rotulos_colados_pela_largura() {
        // "Compra" termina 3,6 pt antes de "Valor": a fonte informa a largura.
        let mut c = corrido(460.0, 7.35, 489.0, 3.9, 3.9, "Compra");
        c.extend(corrido(
            460.0,
            7.35,
            489.0 + 3.9 * 5.0 + 7.5,
            3.9,
            3.9,
            "Valor",
        ));
        let l = agrupar(c);
        assert_eq!(textos(&l[0]), ["Compra", "Valor"]);
    }

    #[test]
    fn largura_errada_nao_parte_cada_caractere() {
        // Fonte que informa largura pequena (0,1 × corpo), mas não zero, com os
        // caracteres em passo mono de 0,61 × corpo.
        let c = corrido(434.0, 7.35, 88.0, 4.5, 0.735, "15/01/26");
        assert_eq!(textos(&agrupar(c)[0]), ["15/01/26"]);
    }

    #[test]
    fn agrupar_guarda_o_maior_tamanho_da_linha() {
        let mut c = chars(1, 100.0, 5.0, &[(10.0, "1")]);
        c.extend(chars(1, 100.0, 10.0, &[(40.0, "T")]));
        assert_eq!(agrupar(c)[0].tamanho, 10.0);
    }

    #[test]
    fn agrupar_separa_linhas_por_y_e_ordena_de_cima_para_baixo() {
        let mut c = chars(1, 428.0, 7.0, &[(27.0, "C")]);
        c.extend(chars(
            1,
            437.0,
            7.0,
            &[(27.0, "C"), (31.5, "D"), (36.0, "B"), (40.0, "-")],
        ));
        c.extend(chars(1, 434.0, 7.0, &[(88.0, "2")]));
        let l = agrupar(c);
        assert_eq!(
            l.iter().map(|x| x.y).collect::<Vec<_>>(),
            [437.0, 434.0, 428.0]
        );
    }

    #[test]
    fn agrupar_separa_paginas() {
        let mut c = chars(2, 100.0, 7.0, &[(10.0, "B")]);
        c.extend(chars(1, 100.0, 7.0, &[(10.0, "A")]));
        let l = agrupar(c);
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].pagina, l[0].texto()), (1, "A".to_string()));
        assert_eq!((l[1].pagina, l[1].texto()), (2, "B".to_string()));
    }

    #[test]
    fn agrupar_preserva_nulo_apara_pontas_e_descarta_linha_vazia() {
        let mut c = chars(
            1,
            50.0,
            7.0,
            &[(10.0, " "), (14.0, "\0"), (18.0, "x"), (22.0, " ")],
        );
        c.extend(chars(1, 80.0, 7.0, &[(10.0, " "), (14.0, " ")]));
        let l = agrupar(c);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].texto(), "\0x");
    }

    #[test]
    fn pdf_corrompido_e_arquivo_invalido() {
        let e = extrair_linhas(b"%PDF-1.4\n isto nao e um pdf").unwrap_err();
        assert!(
            matches!(e, CoreError::ArquivoInvalido(ref m) if m == "Este arquivo não é um PDF válido.")
        );
        assert!(extrair_linhas(b"PK\x03\x04 planilha").is_err());
    }
}
