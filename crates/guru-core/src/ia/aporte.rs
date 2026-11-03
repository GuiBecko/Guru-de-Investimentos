//! Foco do Próximo aporte: o bloco a mais do contexto de cada chat por classe e a pergunta que
//! cria a conversa com `foco` = código da classe.

use std::sync::atomic::AtomicBool;

use chrono::NaiveDate;

use crate::aporte::{Aporte, ResultadoAporte, Situacao};
use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{Classe, decimos};
use crate::metas::formatar_reais;

use super::cliente::ClienteIa;
use super::cofre::Cofre;
use super::contexto::{ContextoMontado, FonteInterna, linha_meta, pct_decimos};
use super::conversa::{Destino, perguntar_com};
use super::sse::Progresso;

/// Direções que o usuário pode dar ao dinheiro do aporte: (código, rótulo), nesta ordem.
pub const DIRECOES: [(&str, &str); 6] = [
    ("renda_fixa", "Renda fixa"),
    ("acoes", "Ações"),
    ("fiis", "FIIs"),
    ("exterior", "Exterior"),
    ("novo", "Experimentar algo novo"),
    ("seguranca", "Mais segurança"),
];

const MAX_DIRECAO_LIVRE: usize = 300;

/// O que a tela do Próximo aporte manda para o chat de uma classe. A divisão não vem da tela:
/// é recalculada por `Db::proximo_aporte` com o valor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FocoAporte {
    pub classe: String,
    pub valor_centavos: i64,
    pub direcoes: Vec<String>,
    pub direcao_livre: String,
    pub meta_id: Option<i64>,
}

fn invalido(mensagem: &str) -> CoreError {
    CoreError::FocoInvalido(mensagem.to_string())
}

/// Décimos sem sinal e sem "%": `80` vira `8,0`.
fn pontos(d: i64) -> String {
    let a = d.unsigned_abs();
    format!("{},{}", a / 10, a % 10)
}

/// Espaços e quebras de linha viram um espaço só: o texto do usuário não abre uma seção nova.
fn numa_linha(texto: &str) -> String {
    texto.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Db {
    /// Valida o foco e devolve o bloco "## Próximo aporte". Nada é gravado.
    fn bloco_aporte(&self, hoje: NaiveDate, foco: &FocoAporte) -> Result<String, CoreError> {
        let aporte: Aporte = match self.proximo_aporte(hoje, foco.valor_centavos)? {
            ResultadoAporte::Dividido(a) => a,
            ResultadoAporte::Invalido(erros) => {
                return Err(invalido(
                    erros
                        .first()
                        .map_or("Valor inválido.", |e| e.mensagem.as_str()),
                ));
            }
            ResultadoAporte::SemAlvo => {
                return Err(invalido(
                    "Defina a sua alocação-alvo antes de conversar sobre o aporte.",
                ));
            }
        };
        let parte = Classe::de_codigo(&foco.classe)
            .and_then(|c| aporte.partes.iter().find(|p| p.classe == c))
            .ok_or_else(|| invalido("Esta classe não está na divisão do aporte."))?;
        if foco
            .direcoes
            .iter()
            .any(|d| !DIRECOES.iter().any(|(c, _)| c == d))
        {
            return Err(invalido("Direção desconhecida."));
        }
        let livre = numa_linha(&foco.direcao_livre);
        if livre.chars().count() > MAX_DIRECAO_LIVRE {
            return Err(invalido("Use no máximo 300 caracteres."));
        }
        let meta = match foco.meta_id {
            Some(id) => Some(self.meta(id, hoje).map_err(|e| match e {
                CoreError::MetaNaoEncontrada(_) => {
                    invalido("Esta meta não existe mais. Atualize a lista.")
                }
                outro => outro,
            })?),
            None => None,
        };
        let cfg = self.config_ia()?;

        let rotulo = parte.classe.rotulo();
        let desvio = parte.percentual_atual_decimos - 10 * i64::from(parte.alvo_percentual);
        let situacao = match parte.situacao {
            Situacao::Abaixo => format!("hoje {} p.p. abaixo do alvo", pontos(desvio)),
            Situacao::Perto => "hoje perto do alvo".to_string(),
            Situacao::Acima => format!("hoje {} p.p. acima do alvo", pontos(desvio)),
            Situacao::ForaDoAlvo => "fora da alocação-alvo".to_string(),
        };
        // A divisão completa, em percentual do aporte, na ordem das classes.
        let divisao: Vec<String> = Classe::TODOS
            .iter()
            .filter_map(|c| aporte.partes.iter().find(|p| p.classe == *c))
            .map(|p| {
                format!(
                    "{} {}",
                    p.classe.rotulo(),
                    pct_decimos(decimos(p.sugerido_centavos, aporte.valor_centavos))
                )
            })
            .collect();
        let direcoes: Vec<&str> = DIRECOES
            .iter()
            .filter(|(c, _)| foco.direcoes.iter().any(|d| d == c))
            .map(|(_, r)| *r)
            .collect();
        let mut direcao = format!(
            "Direção que o usuário quer dar a este dinheiro: {}.",
            if direcoes.is_empty() {
                "não indicada".to_string()
            } else {
                direcoes.join("; ")
            }
        );
        if !livre.is_empty() {
            direcao.push_str(&format!(" Nas palavras dele: \"{livre}\"."));
        }
        let meta = meta.map_or_else(|| "nenhuma".to_string(), |m| linha_meta(&m, &cfg));

        Ok([
            "## Próximo aporte".to_string(),
            format!(
                "Valor deste aporte: {}.",
                formatar_reais(aporte.valor_centavos)
            ),
            format!(
                "Classe em foco: {rotulo}, com {} deste aporte pela divisão de rebalanceamento \
                 (alvo {}%, {situacao}).",
                formatar_reais(parte.sugerido_centavos),
                parte.alvo_percentual
            ),
            format!("Divisão do aporte: {}.", divisao.join("; ")),
            direcao,
            format!("Meta ligada a este aporte: {meta}."),
            format!(
                "O usuário quer ajuda para escolher em que investir dentro de {rotulo} com este \
                 valor: apresente opções, com prós e contras, e tire as dúvidas dele."
            ),
        ]
        .join("\n"))
    }

    /// `contexto_montado` com o bloco "## Próximo aporte" antes de "Hoje é", e a fonte interna
    /// do aporte no fim. Um foco inválido dá `FocoInvalido`.
    pub fn contexto_aporte(
        &self,
        hoje: NaiveDate,
        foco: &FocoAporte,
    ) -> Result<ContextoMontado, CoreError> {
        let bloco = self.bloco_aporte(hoje, foco)?;
        self.contexto_com(
            hoje,
            Some((
                bloco,
                FonteInterna {
                    parte: "aporte".into(),
                    rotulo: "Próximo aporte".into(),
                    data_dados: None,
                    antigo: false,
                },
            )),
        )
    }
}

/// Sempre cria a conversa (foco = classe); as perguntas seguintes usam `perguntar` com o id.
/// Um foco inválido não chama a API.
#[allow(clippy::too_many_arguments)]
pub fn perguntar_aporte(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    hoje: NaiveDate,
    foco: &FocoAporte,
    pergunta: &str,
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<i64, CoreError> {
    perguntar_com(
        db,
        cofre,
        cliente,
        hoje,
        Destino::Aporte(foco),
        pergunta,
        cancelar,
        progresso,
    )
}
