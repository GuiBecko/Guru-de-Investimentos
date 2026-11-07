//! Leitura de uma tabela: cabeçalho, âncoras e linhas quebradas.

use super::super::pdf::{LinhaPdf, esqueleto};
use super::secoes::Secao;
use super::valores::data;

/// Coluna esperada: nome para o aviso e rótulos aceitos.
pub(super) struct Coluna {
    pub(super) nome: &'static str,
    pub(super) rotulos: &'static [&'static str],
    pub(super) obrigatoria: bool,
}

pub(super) const fn col(nome: &'static str, rotulos: &'static [&'static str]) -> Coluna {
    Coluna {
        nome,
        rotulos,
        obrigatoria: true,
    }
}

/// Colunas do cabeçalho: `x` inicial e a coluna esperada a que correspondem.
pub(super) struct Cabecalho {
    colunas: Vec<(f64, Option<usize>)>,
}

impl Cabecalho {
    /// Índice da coluna esperada da célula em `x`: a de maior `x` ≤ `x + 3`.
    fn coluna(&self, x: f64) -> Option<usize> {
        self.colunas
            .iter()
            .filter(|(cx, _)| *cx <= x + 3.0)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .and_then(|(_, i)| *i)
    }
}

/// Uma linha de dado montada: índice da âncora na seção e o texto por coluna
/// esperada (vazio quando não há).
pub(super) struct Registro {
    pub(super) indice: usize,
    pub(super) valores: Vec<String>,
}

pub(super) fn e_nota(texto: &str) -> bool {
    !texto.is_empty() && texto.chars().all(|c| c.is_ascii_digit())
}

/// Linha só de rótulos: nenhuma célula (fora as notas) tem dígito.
pub(super) fn so_rotulos(l: &LinhaPdf) -> bool {
    l.celulas
        .iter()
        .all(|c| e_nota(&c.texto) || !c.texto.chars().any(|ch| ch.is_ascii_digit()))
}

pub(super) fn e_total(l: &LinhaPdf) -> bool {
    l.celulas
        .first()
        .is_some_and(|c| esqueleto(&c.texto).starts_with(&esqueleto("Total")))
}

pub(super) fn e_titulo_fundo(l: &LinhaPdf) -> bool {
    esqueleto(&l.texto()).contains(&esqueleto("Classe CNPJ"))
}

pub(super) fn rotulo_casa(rotulo: &str, esperado: &str) -> bool {
    esqueleto(rotulo).trim_end_matches(|c: char| c.is_ascii_digit()) == esqueleto(esperado)
}

/// Monta o cabeçalho a partir da faixa de rótulos; `Err` com o nome da
/// primeira coluna obrigatória ausente.
pub(super) fn cabecalho(
    faixa: &[&LinhaPdf],
    esperadas: &[Coluna],
) -> Result<Cabecalho, &'static str> {
    let mut grupos: Vec<(f64, String)> = Vec::new();
    for l in faixa {
        for c in l.celulas.iter().filter(|c| !e_nota(&c.texto)) {
            match grupos.iter_mut().find(|(x, _)| (x - c.x).abs() <= 4.0) {
                Some((_, rotulo)) => {
                    rotulo.push(' ');
                    rotulo.push_str(&c.texto);
                }
                None => grupos.push((c.x, c.texto.clone())),
            }
        }
    }
    let mut colunas: Vec<(f64, Option<usize>)> = grupos.iter().map(|(x, _)| (*x, None)).collect();
    for (i, e) in esperadas.iter().enumerate() {
        let achada = grupos
            .iter()
            .position(|(_, r)| e.rotulos.iter().any(|esp| rotulo_casa(r, esp)));
        match achada {
            Some(j) => colunas[j].1 = Some(i),
            None if e.obrigatoria => return Err(e.nome),
            None => {}
        }
    }
    Ok(Cabecalho { colunas })
}

/// Por que uma tabela não pôde ser lida.
#[derive(Debug, PartialEq)]
pub(super) enum Falha {
    /// Falta a coluna obrigatória (nome).
    Coluna(&'static str),
    /// Há linhas de dado, mas nenhuma âncora.
    SemLinhas,
}

/// Lê a tabela de uma seção. `e_ancora(linhas, i, cabecalho)` diz se a linha
/// `i` é a linha dos números de um registro.
pub(super) fn ler_tabela(
    secao: &Secao,
    esperadas: &[Coluna],
    e_ancora: impl Fn(&[&LinhaPdf], usize, &Cabecalho) -> bool,
) -> Result<Vec<Registro>, Falha> {
    let linhas = &secao.linhas;
    let n = linhas.iter().take_while(|l| so_rotulos(l)).count();
    let cab = cabecalho(&linhas[..n], esperadas).map_err(Falha::Coluna)?;
    let repetidas: Vec<String> = linhas[..n].iter().map(|l| esqueleto(&l.texto())).collect();

    let mut ancoras: Vec<usize> = Vec::new();
    let mut soltas: Vec<usize> = Vec::new();
    for (i, l) in linhas.iter().enumerate().skip(n) {
        if e_total(l) || e_titulo_fundo(l) || repetidas.contains(&esqueleto(&l.texto())) {
            continue;
        }
        if e_ancora(linhas, i, &cab) {
            ancoras.push(i);
        } else {
            soltas.push(i);
        }
    }

    if ancoras.is_empty() && !soltas.is_empty() {
        return Err(Falha::SemLinhas);
    }

    let mut grupos: Vec<Vec<usize>> = ancoras.iter().map(|&a| vec![a]).collect();
    for i in soltas {
        let l = linhas[i];
        let perto = ancoras
            .iter()
            .enumerate()
            .filter(|(_, a)| linhas[**a].pagina == l.pagina)
            .map(|(k, a)| (k, (linhas[*a].y - l.y).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((k, distancia)) = perto
            && distancia <= 1.2 * linhas[ancoras[k]].tamanho
        {
            grupos[k].push(i);
        }
    }

    Ok(grupos
        .into_iter()
        .zip(&ancoras)
        .map(|(mut grupo, &indice)| {
            grupo.sort_by(|a, b| linhas[*b].y.total_cmp(&linhas[*a].y));
            let mut valores = vec![String::new(); esperadas.len()];
            for c in grupo.iter().flat_map(|&i| &linhas[i].celulas) {
                if let Some(j) = cab.coluna(c.x) {
                    let v = &mut valores[j];
                    if !v.is_empty() && !v.ends_with('-') {
                        v.push(' ');
                    }
                    v.push_str(&c.texto);
                }
            }
            Registro { indice, valores }
        })
        .collect())
}

/// Âncora de fundo e renda fixa: data na coluna `coluna`.
pub(super) fn data_na_coluna(coluna: usize) -> impl Fn(&[&LinhaPdf], usize, &Cabecalho) -> bool {
    move |linhas, i, cab| {
        linhas[i]
            .celulas
            .iter()
            .any(|c| cab.coluna(c.x) == Some(coluna) && data(&c.texto).is_some())
    }
}

/// Âncora de renda variável e cripto: mais de uma célula (não é título nem
/// pedaço quebrado); totais e cabeçalhos já foram tirados.
pub(super) fn linha_com_valores(linhas: &[&LinhaPdf], i: usize, _: &Cabecalho) -> bool {
    linhas[i].celulas.len() > 1
}
