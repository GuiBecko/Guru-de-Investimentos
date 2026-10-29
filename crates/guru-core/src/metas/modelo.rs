use crate::investimento::enum_catalogo;

/// Entrada como chega da interface, ainda não validada.
#[derive(Debug, Clone, Default)]
pub struct NovaMeta {
    pub nome: String,
    pub valor_alvo_centavos: Option<i64>,
    /// Mês no formato `YYYY-MM`.
    pub prazo: Option<String>,
}

enum_catalogo!(Situacao {
    Concluida => ("concluida", "Concluída"),
    Vencida => ("vencida", "Vencida"),
    SemAportes => ("sem_aportes", "Sem aportes"),
    Atrasada => ("atrasada", "Atrasada"),
    NoCaminho => ("no_caminho", "No caminho"),
});

#[derive(Debug, Clone)]
pub struct Meta {
    pub id: i64,
    pub nome: String,
    pub valor_alvo_centavos: i64,
    pub prazo: String,
    pub acumulado_centavos: i64,
    /// Décimos de ponto percentual, de 0 a 1000 (teto).
    pub percentual_decimos: i64,
    pub meses_restantes: i64,
    pub aporte_necessario_centavos: Option<i64>,
    pub ritmo_centavos: Option<i64>,
    pub situacao: Situacao,
    pub mensagem: String,
    pub investimentos: i64,
}
