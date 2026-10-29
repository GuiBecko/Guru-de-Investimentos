//! Questionário de perfil de investidor e cálculo do resultado.

mod calculo;
mod questionario;
mod repositorio;

use crate::investimento::enum_catalogo;

pub use calculo::{ResultadoPerfil, alocacao_sugerida, calcular};
pub use questionario::{AVISO_FIXO, Opcao, Pergunta, perguntas};
pub use repositorio::PerfilGravado;

enum_catalogo!(Perfil {
    Conservador => ("conservador", "Conservador"),
    Moderado => ("moderado", "Moderado"),
    Arrojado => ("arrojado", "Arrojado"),
});

impl Perfil {
    pub fn descricao(&self) -> &'static str {
        match self {
            Self::Conservador => {
                "Prioriza a segurança e a liquidez. Aceita pouca ou nenhuma oscilação."
            }
            Self::Moderado => {
                "Aceita alguma oscilação em parte da carteira para buscar retorno acima da renda fixa no médio prazo."
            }
            Self::Arrojado => {
                "Aceita oscilações fortes no curto prazo em busca de mais retorno no longo prazo."
            }
        }
    }
}
