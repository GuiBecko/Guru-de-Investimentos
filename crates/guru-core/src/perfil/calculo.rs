//! Cálculo puro do perfil a partir das respostas.

use super::{Perfil, questionario::perguntas};
use crate::investimento::{Classe, ErroCampo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoPerfil {
    pub perfil: Perfil,
    pub pontos: u8,
    pub avisos: Vec<String>,
    pub alocacao_sugerida: Vec<(Classe, u8)>,
}

/// Calcula o perfil. `respostas` traz o índice (0 a 3) da opção escolhida em cada pergunta.
pub fn calcular(respostas: &[u8]) -> Result<ResultadoPerfil, ErroCampo> {
    let todas = perguntas();
    if respostas.len() != todas.len() || respostas.iter().any(|&r| r > 3) {
        return Err(ErroCampo {
            campo: "respostas".into(),
            mensagem: "Responda todas as perguntas.".into(),
        });
    }
    let pontos_da = |i: usize| todas[i].opcoes[respostas[i] as usize].pontos;
    let pontos: u8 = (0..todas.len()).map(pontos_da).sum();

    let mut perfil = match pontos {
        0..=12 => Perfil::Conservador,
        13..=24 => Perfil::Moderado,
        _ => Perfil::Arrojado,
    };
    let mut avisos = Vec::new();

    if respostas[1] == 0 && perfil != Perfil::Conservador {
        perfil = Perfil::Conservador;
        avisos.push(
            "Como você deve precisar do dinheiro em menos de 1 ano, o perfil ficou Conservador."
                .to_string(),
        );
    }
    if respostas[5] == 0 && perfil == Perfil::Arrojado {
        perfil = Perfil::Moderado;
        avisos.push(
            "Sem reserva de emergência, o perfil ficou limitado a Moderado. Montar a reserva vem antes."
                .to_string(),
        );
    }
    if pontos_da(2).abs_diff(pontos_da(3)) >= 2 {
        avisos.push(
            "Suas respostas sobre quedas e oscilação não combinam. Vale refazer o teste com calma."
                .to_string(),
        );
    }
    if pontos_da(10) >= 2 && pontos_da(11) == 0 {
        avisos.push(
            "Você declarou mais conhecimento do que a resposta sobre títulos prefixados indica."
                .to_string(),
        );
    }

    Ok(ResultadoPerfil {
        perfil,
        pontos,
        avisos,
        alocacao_sugerida: alocacao_sugerida(perfil),
    })
}

/// Alocação sugerida em pontos percentuais, nas cinco classes, somando 100.
pub fn alocacao_sugerida(perfil: Perfil) -> Vec<(Classe, u8)> {
    let pesos: [u8; 5] = match perfil {
        Perfil::Conservador => [85, 5, 5, 5, 0],
        Perfil::Moderado => [50, 25, 10, 10, 5],
        Perfil::Arrojado => [30, 35, 10, 20, 5],
    };
    Classe::TODOS.iter().copied().zip(pesos).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    const PRAZO: &str =
        "Como você deve precisar do dinheiro em menos de 1 ano, o perfil ficou Conservador.";
    const RESERVA: &str = "Sem reserva de emergência, o perfil ficou limitado a Moderado. Montar a reserva vem antes.";
    const QUEDAS: &str =
        "Suas respostas sobre quedas e oscilação não combinam. Vale refazer o teste com calma.";
    const CONHECIMENTO: &str =
        "Você declarou mais conhecimento do que a resposta sobre títulos prefixados indica.";

    /// Respostas com a soma pedida, sem travas (perguntas 2 e 6 começam em 1) e com a 12 em 0.
    fn com_soma(soma: u8) -> Vec<u8> {
        let mut r = vec![0u8; 12];
        r[1] = 1;
        r[5] = 1;
        let mut falta = soma - 2;
        for v in r.iter_mut().take(11) {
            let cabe = 3 - *v;
            let dar = cabe.min(falta);
            *v += dar;
            falta -= dar;
        }
        assert_eq!(falta, 0);
        r
    }

    #[test]
    fn todas_zero_da_conservador() {
        let r = calcular(&[0; 12]).unwrap();
        assert_eq!(r.perfil, Perfil::Conservador);
        assert_eq!(r.pontos, 0);
    }

    #[test]
    fn todas_tres_da_arrojado() {
        let r = calcular(&[3; 12]).unwrap();
        assert_eq!(r.perfil, Perfil::Arrojado);
        assert_eq!(r.pontos, 36);
        assert!(r.avisos.is_empty());
    }

    #[test]
    fn limites_12_13_24_25() {
        for (soma, perfil) in [
            (12, Perfil::Conservador),
            (13, Perfil::Moderado),
            (24, Perfil::Moderado),
            (25, Perfil::Arrojado),
        ] {
            let r = calcular(&com_soma(soma)).unwrap();
            assert_eq!(r.pontos, soma);
            assert_eq!(r.perfil, perfil, "soma {soma}");
        }
    }

    #[test]
    fn prazo_curto_trava_em_conservador() {
        let mut r = [3u8; 12];
        r[1] = 0;
        let res = calcular(&r).unwrap();
        assert_eq!(res.perfil, Perfil::Conservador);
        assert_eq!(res.avisos, vec![PRAZO.to_string()]);
    }

    #[test]
    fn sem_reserva_limita_arrojado_a_moderado() {
        let mut r = [3u8; 12];
        r[5] = 0;
        let res = calcular(&r).unwrap();
        assert_eq!(res.perfil, Perfil::Moderado);
        assert_eq!(res.avisos, vec![RESERVA.to_string()]);
    }

    #[test]
    fn sem_reserva_nao_mexe_em_moderado() {
        let r = [2, 2, 2, 2, 2, 0, 2, 2, 2, 2, 2, 0];
        let res = calcular(&r).unwrap();
        assert_eq!(res.perfil, Perfil::Moderado);
        assert!(!res.avisos.iter().any(|a| a == RESERVA));
    }

    #[test]
    fn inconsistencia_quedas() {
        let mut r = com_soma(18);
        r[2] = 0;
        r[3] = 2;
        assert!(calcular(&r).unwrap().avisos.contains(&QUEDAS.to_string()));
    }

    #[test]
    fn inconsistencia_conhecimento() {
        let mut r = com_soma(18);
        r[10] = 2;
        r[11] = 0;
        assert!(
            calcular(&r)
                .unwrap()
                .avisos
                .contains(&CONHECIMENTO.to_string())
        );
    }

    #[test]
    fn respostas_coerentes_sem_aviso() {
        let r = [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3];
        let res = calcular(&r).unwrap();
        assert!(res.avisos.is_empty());
        assert_eq!(res.perfil, Perfil::Arrojado);
    }

    #[test]
    fn alocacoes_somam_100() {
        for &p in Perfil::TODOS {
            let a = alocacao_sugerida(p);
            let classes: Vec<Classe> = a.iter().map(|(c, _)| *c).collect();
            assert_eq!(classes, Classe::TODOS);
            assert_eq!(a.iter().map(|(_, v)| *v as u32).sum::<u32>(), 100);
        }
        assert_eq!(
            alocacao_sugerida(Perfil::Moderado)
                .iter()
                .map(|(_, v)| *v)
                .collect::<Vec<_>>(),
            [50, 25, 10, 10, 5]
        );
    }

    #[test]
    fn respostas_incompletas_ou_fora_da_faixa() {
        let esperado = ErroCampo {
            campo: "respostas".into(),
            mensagem: "Responda todas as perguntas.".into(),
        };
        assert_eq!(calcular(&[1; 11]).unwrap_err(), esperado);
        assert_eq!(calcular(&[1; 13]).unwrap_err(), esperado);
        let mut r = [1u8; 12];
        r[4] = 4;
        assert_eq!(calcular(&r).unwrap_err(), esperado);
    }
}
