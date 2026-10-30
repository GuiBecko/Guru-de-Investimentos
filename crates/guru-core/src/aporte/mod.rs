//! Próximo aporte: divide um valor entre as classes, rebalanceando rumo à alocação-alvo.
//! Só aporte, nunca venda: cada sugestão vale 0 ou mais e a soma é exatamente o valor.

use std::cmp::Ordering;

use chrono::NaiveDate;

use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{Classe, ErroCampo, MAX_CENTAVOS, decimos};

/// Menor aporte aceito: R$ 1,00.
const MIN_CENTAVOS: i64 = 100;

/// Onde a classe está em relação ao alvo, antes do aporte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situacao {
    Abaixo,
    Perto,
    Acima,
    ForaDoAlvo,
}

impl Situacao {
    pub fn codigo(self) -> &'static str {
        match self {
            Situacao::Abaixo => "abaixo",
            Situacao::Perto => "perto",
            Situacao::Acima => "acima",
            Situacao::ForaDoAlvo => "fora_do_alvo",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParteAporte {
    pub classe: Classe,
    pub atual_centavos: i64,
    pub alvo_percentual: u8,
    pub sugerido_centavos: i64,
    pub percentual_atual_decimos: i64,
    pub percentual_depois_decimos: i64,
    pub situacao: Situacao,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aporte {
    pub valor_centavos: i64,
    pub patrimonio_centavos: i64,
    pub partes: Vec<ParteAporte>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoAporte {
    SemAlvo,
    Invalido(Vec<ErroCampo>),
    Dividido(Aporte),
}

fn ordem_classe(classe: Classe) -> usize {
    Classe::TODOS
        .iter()
        .position(|c| *c == classe)
        .unwrap_or(Classe::TODOS.len())
}

fn para_i64(v: i128) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// Divide `valor_centavos` (já validado, ≥ 1) entre as `classes` `(classe, atual, alvo)`.
/// Só as classes com alvo > 0 recebem; listam-se as com alvo > 0 ou posição > 0.
pub fn dividir_aporte(valor_centavos: i64, classes: &[(Classe, i64, u8)]) -> Aporte {
    let a = i128::from(valor_centavos);
    // (classe, V ≥ 0, t)
    let todas: Vec<(Classe, i128, i128)> = classes
        .iter()
        .map(|(c, v, t)| (*c, i128::from((*v).max(0)), i128::from(*t)))
        .collect();
    let patrimonio: i128 = todas.iter().map(|(_, v, _)| *v).sum();

    // Candidatas: alvo > 0, por V/t crescente (V_a·t_b < V_b·t_a); empate pela ordem de TODOS.
    let mut candidatas: Vec<(Classe, i128, i128)> =
        todas.iter().copied().filter(|(_, _, t)| *t > 0).collect();
    candidatas.sort_by(|x, y| {
        (x.1 * y.2)
            .cmp(&(y.1 * x.2))
            .then_with(|| ordem_classe(x.0).cmp(&ordem_classe(y.0)))
    });

    // Maior k com (A + ΣV)·t_k ≥ V_k·Σt.
    let (mut soma_v, mut soma_t) = (0i128, 0i128);
    let mut melhor: Option<(usize, i128, i128)> = None;
    for (i, (_, v, t)) in candidatas.iter().enumerate() {
        soma_v += v;
        soma_t += t;
        if (a + soma_v) * t >= v * soma_t {
            melhor = Some((i + 1, soma_v, soma_t));
        }
    }

    // Partes: piso da parte exata e resto, em unidades de 1/Σt.
    let mut sugestoes: Vec<(Classe, i128, i128)> = Vec::new(); // (classe, piso, resto)
    if let Some((k, sv, st)) = melhor {
        for (c, v, t) in &candidatas[..k] {
            let numerador = ((a + sv) * t - v * st).max(0);
            sugestoes.push((*c, numerador / st, numerador % st));
        }
        let distribuido: i128 = sugestoes.iter().map(|(_, p, _)| *p).sum();
        let sobra = (a - distribuido).max(0) as usize;
        let mut ordem: Vec<usize> = (0..sugestoes.len()).collect();
        ordem.sort_by(|&i, &j| {
            sugestoes[j]
                .2
                .cmp(&sugestoes[i].2)
                .then_with(|| ordem_classe(sugestoes[i].0).cmp(&ordem_classe(sugestoes[j].0)))
        });
        for &i in ordem.iter().take(sobra) {
            sugestoes[i].1 += 1;
        }
    }

    let total_depois = para_i64(patrimonio + a);
    let total_antes = para_i64(patrimonio);
    let mut partes: Vec<ParteAporte> = todas
        .iter()
        .filter(|(_, v, t)| *t > 0 || *v > 0)
        .map(|(c, v, t)| {
            let x = sugestoes
                .iter()
                .find(|(s, _, _)| s == c)
                .map_or(0, |(_, p, _)| *p);
            let atual = decimos(para_i64(*v), total_antes);
            let d = atual - 10 * (*t as i64);
            let situacao = if *t == 0 {
                Situacao::ForaDoAlvo
            } else if d <= -10 {
                Situacao::Abaixo
            } else if d >= 10 {
                Situacao::Acima
            } else {
                Situacao::Perto
            };
            ParteAporte {
                classe: *c,
                atual_centavos: para_i64(*v),
                alvo_percentual: *t as u8,
                sugerido_centavos: para_i64(x),
                percentual_atual_decimos: atual,
                percentual_depois_decimos: decimos(para_i64(*v + x), total_depois),
                situacao,
            }
        })
        .collect();
    partes.sort_by(|x, y| match y.sugerido_centavos.cmp(&x.sugerido_centavos) {
        Ordering::Equal => ordem_classe(x.classe).cmp(&ordem_classe(y.classe)),
        o => o,
    });

    Aporte {
        valor_centavos,
        patrimonio_centavos: total_antes,
        partes,
    }
}

fn validar_valor(valor_centavos: i64) -> Vec<ErroCampo> {
    let mensagem = if valor_centavos < MIN_CENTAVOS {
        "Informe um valor de pelo menos R$ 1,00."
    } else if valor_centavos > MAX_CENTAVOS {
        "Valor alto demais."
    } else {
        return Vec::new();
    };
    vec![ErroCampo {
        campo: "valorCentavos".to_string(),
        mensagem: mensagem.to_string(),
    }]
}

impl Db {
    /// Divide o aporte pela alocação-alvo salva, com o valor atual de cada classe em `hoje`.
    /// Nada é gravado.
    pub fn proximo_aporte(
        &self,
        hoje: NaiveDate,
        valor_centavos: i64,
    ) -> Result<ResultadoAporte, CoreError> {
        let erros = validar_valor(valor_centavos);
        if !erros.is_empty() {
            return Ok(ResultadoAporte::Invalido(erros));
        }
        let Some(alvo) = self.alocacao_alvo()? else {
            return Ok(ResultadoAporte::SemAlvo);
        };
        let resumo = self.resumo_carteira(hoje)?;
        let classes: Vec<(Classe, i64, u8)> = Classe::TODOS
            .iter()
            .map(|c| {
                let atual = resumo
                    .classes
                    .iter()
                    .find(|r| r.classe == *c)
                    .map_or(0, |r| r.total_atual_centavos);
                let t = alvo.iter().find(|(a, _)| a == c).map_or(0, |(_, t)| *t);
                (*c, atual, t)
            })
            .collect();
        Ok(ResultadoAporte::Dividido(dividir_aporte(
            valor_centavos,
            &classes,
        )))
    }
}
