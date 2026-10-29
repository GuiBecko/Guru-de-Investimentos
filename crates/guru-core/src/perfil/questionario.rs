//! Conteúdo do questionário: doze perguntas em três blocos.

pub struct Opcao {
    pub texto: &'static str,
    pub pontos: u8,
}

pub struct Pergunta {
    pub numero: u8,
    pub bloco: &'static str,
    pub texto: &'static str,
    pub porque: &'static str,
    pub opcoes: [Opcao; 4],
}

pub const AVISO_FIXO: [&str; 2] = [
    "Responda pensando no que você faria de verdade, não no que acha que deveria fazer. Não existe resposta certa.",
    "Seu perfil e a alocação sugerida aparecem depois da última pergunta.",
];

const fn o(texto: &'static str, pontos: u8) -> Opcao {
    Opcao { texto, pontos }
}

const fn opcoes(a: &'static str, b: &'static str, c: &'static str, d: &'static str) -> [Opcao; 4] {
    [o(a, 0), o(b, 1), o(c, 2), o(d, 3)]
}

const OBJETIVO: &str = "Objetivo e prazo";
const SITUACAO: &str = "Situação financeira";
const CONHECIMENTO: &str = "Conhecimento e experiência";

static PERGUNTAS: [Pergunta; 12] = [
    Pergunta {
        numero: 1,
        bloco: OBJETIVO,
        texto: "Qual é o seu principal objetivo ao investir?",
        porque: "O objetivo define quanto risco faz sentido correr.",
        opcoes: opcoes(
            "Preservar o que tenho, sem perdas",
            "Ganhar um pouco mais que a poupança, com pouca oscilação",
            "Fazer o patrimônio crescer, aceitando oscilações",
            "Buscar o maior retorno possível, mesmo com perdas temporárias grandes",
        ),
    },
    Pergunta {
        numero: 2,
        bloco: OBJETIVO,
        texto: "Daqui a quanto tempo você deve precisar da maior parte deste dinheiro?",
        porque: "Quanto mais tempo até o resgate, mais tempo a carteira tem para se recuperar de uma queda.",
        opcoes: opcoes(
            "Menos de 1 ano",
            "De 1 a 3 anos",
            "De 3 a 10 anos",
            "Mais de 10 anos",
        ),
    },
    Pergunta {
        numero: 3,
        bloco: OBJETIVO,
        texto: "Se seus investimentos caíssem 15% em um mês, o que você faria?",
        porque: "Quedas fazem parte de qualquer investimento que rende mais que a poupança. Saber como você reage ajuda a não sugerir algo que vai tirar o seu sono.",
        opcoes: opcoes(
            "Resgataria tudo para não perder mais",
            "Resgataria uma parte",
            "Manteria e esperaria recuperar",
            "Aproveitaria para aplicar mais",
        ),
    },
    Pergunta {
        numero: 4,
        bloco: OBJETIVO,
        texto: "Para o próximo ano, qual destas carteiras você escolheria?",
        porque: "Ver os números lado a lado ajuda a confirmar quanto de oscilação você aceita.",
        opcoes: opcoes(
            "Ganho certo de 6%",
            "Resultado entre −3% e +12%",
            "Resultado entre −10% e +20%",
            "Resultado entre −25% e +40%",
        ),
    },
    Pergunta {
        numero: 5,
        bloco: SITUACAO,
        texto: "Quanto da sua renda mensal vai para despesas fixas?",
        porque: "Quem tem folga no orçamento aguenta melhor um período ruim sem precisar resgatar.",
        opcoes: opcoes(
            "Mais de 80%",
            "De 60% a 80%",
            "De 40% a 60%",
            "Menos de 40%",
        ),
    },
    Pergunta {
        numero: 6,
        bloco: SITUACAO,
        texto: "Você tem uma reserva de emergência?",
        porque: "A reserva evita vender um investimento no pior momento por causa de um imprevisto.",
        opcoes: opcoes(
            "Não tenho",
            "Menos de 3 meses de custo de vida",
            "De 3 a 6 meses",
            "Mais de 6 meses",
        ),
    },
    Pergunta {
        numero: 7,
        bloco: SITUACAO,
        texto: "Quanto do seu patrimônio investido você pode precisar resgatar nos próximos 2 anos?",
        porque: "Dinheiro que vai ser usado logo precisa ficar em investimentos com pouca oscilação.",
        opcoes: opcoes(
            "Mais da metade",
            "Entre um quarto e a metade",
            "Menos de um quarto",
            "Nada",
        ),
    },
    Pergunta {
        numero: 8,
        bloco: SITUACAO,
        texto: "Quanto você tem investido hoje, sem contar o imóvel em que mora?",
        porque: "O tamanho do patrimônio muda quanto uma oscilação pesa na sua vida.",
        opcoes: opcoes(
            "Até R$ 10 mil",
            "De R$ 10 mil a R$ 100 mil",
            "De R$ 100 mil a R$ 1 milhão",
            "Mais de R$ 1 milhão",
        ),
    },
    Pergunta {
        numero: 9,
        bloco: CONHECIMENTO,
        texto: "Qual destas frases descreve melhor onde você já investiu?",
        porque: "Experiência com um tipo de investimento ajuda a entender os riscos dele.",
        opcoes: opcoes(
            "Só na poupança",
            "Em renda fixa, como CDB e Tesouro Direto",
            "Em fundos, FIIs ou ações",
            "Em opções, derivativos ou criptoativos",
        ),
    },
    Pergunta {
        numero: 10,
        bloco: CONHECIMENTO,
        texto: "Há quanto tempo você investe fora da poupança?",
        porque: "O tempo investindo costuma incluir já ter passado por alguma queda.",
        opcoes: opcoes(
            "Nunca investi",
            "Menos de 2 anos",
            "De 2 a 5 anos",
            "Mais de 5 anos",
        ),
    },
    Pergunta {
        numero: 11,
        bloco: CONHECIMENTO,
        texto: "Como você descreveria o seu conhecimento sobre investimentos?",
        porque: "Responda pensando no que você sabe hoje, não no que gostaria de saber.",
        opcoes: opcoes("Nenhum", "Básico", "Bom", "Avançado ou profissional"),
    },
    Pergunta {
        numero: 12,
        bloco: CONHECIMENTO,
        texto: "Quando a taxa de juros da economia sobe, o que acontece com o preço de um título prefixado que você já tem?",
        porque: "Esta pergunta ajuda a conferir a resposta anterior. Não existe problema em não saber.",
        opcoes: [o("Não sei", 0), o("Sobe", 0), o("Não muda", 0), o("Cai", 3)],
    },
];

/// As doze perguntas, na ordem em que são feitas.
pub fn perguntas() -> &'static [Pergunta] {
    &PERGUNTAS
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn doze_perguntas_em_tres_blocos() {
        let todas = perguntas();
        assert_eq!(todas.len(), 12);
        for (i, p) in todas.iter().enumerate() {
            assert_eq!(p.numero as usize, i + 1);
        }
        for (blocos, nome) in [
            (0..4, "Objetivo e prazo"),
            (4..8, "Situação financeira"),
            (8..12, "Conhecimento e experiência"),
        ] {
            for i in blocos {
                assert_eq!(todas[i].bloco, nome);
            }
        }
    }

    #[test]
    fn pergunta_12_pontua_so_a_certa() {
        let pontos: Vec<u8> = perguntas()[11].opcoes.iter().map(|o| o.pontos).collect();
        assert_eq!(pontos, [0, 0, 0, 3]);
    }

    #[test]
    fn demais_perguntas_pontuam_de_0_a_3_em_ordem() {
        for p in &perguntas()[..11] {
            let pontos: Vec<u8> = p.opcoes.iter().map(|o| o.pontos).collect();
            assert_eq!(pontos, [0, 1, 2, 3], "pergunta {}", p.numero);
        }
    }
}
