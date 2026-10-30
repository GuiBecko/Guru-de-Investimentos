use chrono::{Datelike, NaiveDate};

use super::modelo::Situacao;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progresso {
    pub acumulado_centavos: i64,
    pub percentual_decimos: i64,
    pub meses_restantes: i64,
    pub aporte_necessario_centavos: Option<i64>,
    pub ritmo_centavos: Option<i64>,
    pub situacao: Situacao,
    pub mensagem: String,
}

/// `R$ 1.234,56`; negativo com `-` na frente. Só inteiros.
pub fn formatar_reais(centavos: i64) -> String {
    let abs = centavos.unsigned_abs();
    let inteiro = (abs / 100).to_string();
    let mut grupos = String::new();
    for (i, c) in inteiro.chars().enumerate() {
        if i > 0 && (inteiro.len() - i).is_multiple_of(3) {
            grupos.push('.');
        }
        grupos.push(c);
    }
    let sinal = if centavos < 0 { "-" } else { "" };
    format!("{sinal}R$ {grupos},{:02}", abs % 100)
}

fn divide_para_cima(a: i128, b: i128) -> i128 {
    (a + b - 1) / b
}

/// Meses do mês de `de` até o mês de `ate`, contando os dois (mesmo mês → 1).
fn meses_contados(de: (i32, u32), ate: (i32, u32)) -> i64 {
    i64::from(ate.0 - de.0) * 12 + i64::from(ate.1) - i64::from(de.1) + 1
}

/// Situação de uma meta contra o alvo. `vinculados` traz (valor atual, data da aplicação).
pub fn progresso(
    alvo: i64,
    prazo: (i32, u32),
    vinculados: &[(i64, NaiveDate)],
    hoje: NaiveDate,
) -> Progresso {
    let mes_atual = (hoje.year(), hoje.month());
    let acumulado = vinculados
        .iter()
        .fold(0_i64, |acc, (v, _)| acc.saturating_add(*v));
    let percentual_decimos = if alvo > 0 {
        let (a, t) = (i128::from(acumulado), i128::from(alvo));
        // arredonda meio para cima, em inteiros
        ((a * 2000 + t) / (2 * t)).min(1000) as i64
    } else {
        0
    };
    let meses_restantes = meses_contados(mes_atual, prazo).max(0);
    let falta = i128::from(alvo) - i128::from(acumulado);

    let aporte_necessario = (meses_restantes > 0 && falta > 0)
        .then(|| divide_para_cima(falta, i128::from(meses_restantes)) as i64);
    let ritmo = vinculados
        .iter()
        .map(|(_, d)| (d.year(), d.month()))
        .min()
        .map(|primeira| {
            let meses = meses_contados(primeira, mes_atual).max(1);
            acumulado / meses
        });

    let (situacao, mensagem) = if falta <= 0 {
        (Situacao::Concluida, "Meta alcançada.".to_string())
    } else if meses_restantes == 0 {
        (
            Situacao::Vencida,
            format!("O prazo passou. Faltam {}.", formatar_reais(falta as i64)),
        )
    } else if ritmo.is_none() {
        (
            Situacao::SemAportes,
            "Ainda sem investimentos vinculados.".to_string(),
        )
    } else {
        let (r, nec) = (ritmo.unwrap_or(0), aporte_necessario.unwrap_or(0));
        if r < nec {
            let mensagem = if r == 0 {
                "No ritmo atual, a meta não é alcançada.".to_string()
            } else {
                let n = divide_para_cima(falta, i128::from(r)) - i128::from(meses_restantes);
                let meses = if n == 1 { "mês" } else { "meses" };
                format!(
                    "No ritmo atual ({}/mês), chega {n} {meses} depois.",
                    formatar_reais(r)
                )
            };
            (Situacao::Atrasada, mensagem)
        } else {
            (Situacao::NoCaminho, "No caminho certo.".to_string())
        }
    };

    Progresso {
        acumulado_centavos: acumulado,
        percentual_decimos,
        meses_restantes,
        aporte_necessario_centavos: aporte_necessario,
        ritmo_centavos: ritmo,
        situacao,
        mensagem,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn d(a: i32, m: u32, dia: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, dia).unwrap()
    }

    fn hoje() -> NaiveDate {
        d(2026, 10, 29)
    }

    #[test]
    fn formata_reais() {
        assert_eq!(formatar_reais(0), "R$ 0,00");
        assert_eq!(formatar_reais(5), "R$ 0,05");
        assert_eq!(formatar_reais(164_800), "R$ 1.648,00");
        assert_eq!(formatar_reais(123_456_789), "R$ 1.234.567,89");
        assert_eq!(formatar_reais(100_000_000), "R$ 1.000.000,00");
        assert_eq!(formatar_reais(-150), "-R$ 1,50");
        assert_eq!(formatar_reais(i64::MIN), "-R$ 92.233.720.368.547.758,08");
    }

    #[test]
    fn prazo_no_mes_atual_tem_um_mes_e_aporte_igual_a_falta() {
        let p = progresso(1_000_000, (2026, 10), &[(300_000, d(2026, 10, 1))], hoje());
        assert_eq!(p.meses_restantes, 1);
        assert_eq!(p.aporte_necessario_centavos, Some(700_000));
    }

    #[test]
    fn prazo_passado_e_vencida_sem_aporte() {
        let p = progresso(1_000_000, (2026, 9), &[(500_000, d(2025, 1, 1))], hoje());
        assert_eq!(p.meses_restantes, 0);
        assert_eq!(p.aporte_necessario_centavos, None);
        assert_eq!(p.situacao, Situacao::Vencida);
        assert_eq!(p.mensagem, "O prazo passou. Faltam R$ 5.000,00.");
        let antigo = progresso(1_000_000, (2020, 1), &[], hoje());
        assert_eq!(antigo.meses_restantes, 0);
        assert_eq!(antigo.situacao, Situacao::Vencida);
    }

    #[test]
    fn prazo_em_cinco_anos_sem_vinculos_e_sem_aportes() {
        let p = progresso(6_100_000, (2031, 10), &[], hoje());
        assert_eq!(p.meses_restantes, 61);
        assert_eq!(p.aporte_necessario_centavos, Some(100_000));
        assert_eq!(p.ritmo_centavos, None);
        assert_eq!(p.acumulado_centavos, 0);
        assert_eq!(p.percentual_decimos, 0);
        assert_eq!(p.situacao, Situacao::SemAportes);
        assert_eq!(p.mensagem, "Ainda sem investimentos vinculados.");
    }

    #[test]
    fn acumulado_maior_ou_igual_ao_alvo_e_concluida() {
        let p = progresso(1_000_000, (2027, 10), &[(1_200_000, d(2026, 1, 1))], hoje());
        assert_eq!(p.situacao, Situacao::Concluida);
        assert_eq!(p.percentual_decimos, 1000);
        assert_eq!(p.aporte_necessario_centavos, None);
        assert_eq!(p.mensagem, "Meta alcançada.");
        // concluída vale mesmo com o prazo vencido
        let vencida = progresso(1_000_000, (2020, 1), &[(1_000_000, d(2019, 1, 1))], hoje());
        assert_eq!(vencida.situacao, Situacao::Concluida);
    }

    #[test]
    fn atrasada_com_o_n_exato() {
        // 24 meses antes de out/2026 = out/2024; contando o mês atual são 25 meses.
        let p = progresso(
            15_000_000,
            (2030, 11),
            &[(4_120_000, d(2024, 10, 15))],
            hoje(),
        );
        assert_eq!(p.meses_restantes, 50);
        assert_eq!(p.ritmo_centavos, Some(164_800));
        assert_eq!(p.aporte_necessario_centavos, Some(217_600));
        assert_eq!(p.situacao, Situacao::Atrasada);
        assert_eq!(
            p.mensagem,
            "No ritmo atual (R$ 1.648,00/mês), chega 17 meses depois."
        );
        assert_eq!(p.percentual_decimos, 275);
    }

    #[test]
    fn atrasada_com_ritmo_zero_nao_alcanca() {
        let p = progresso(1_000_000, (2030, 1), &[(3, d(2024, 10, 15))], hoje());
        assert_eq!(p.ritmo_centavos, Some(0));
        assert_eq!(p.situacao, Situacao::Atrasada);
        assert_eq!(p.mensagem, "No ritmo atual, a meta não é alcançada.");
    }

    #[test]
    fn no_caminho_quando_o_ritmo_alcanca_o_necessario() {
        let p = progresso(1_000_000, (2027, 10), &[(600_000, d(2026, 4, 10))], hoje());
        assert_eq!(p.meses_restantes, 13);
        assert_eq!(p.ritmo_centavos, Some(85_714));
        assert_eq!(p.aporte_necessario_centavos, Some(30_770));
        assert_eq!(p.situacao, Situacao::NoCaminho);
        assert_eq!(p.mensagem, "No caminho certo.");
    }

    #[test]
    fn primeira_aplicacao_no_mes_atual_conta_um_mes_e_soma_varios_vinculos() {
        let p = progresso(
            1_000_000,
            (2027, 10),
            &[(100_000, d(2026, 10, 2)), (50_000, d(2026, 10, 20))],
            hoje(),
        );
        assert_eq!(p.acumulado_centavos, 150_000);
        assert_eq!(p.ritmo_centavos, Some(150_000));
    }
}
