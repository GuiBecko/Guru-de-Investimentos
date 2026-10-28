//! Catálogo fechado de tipos de investimento e dos valores que os campos aceitam.

macro_rules! enum_catalogo {
    ($nome:ident { $($var:ident => ($codigo:expr, $rotulo:expr)),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $nome {
            $($var),+
        }

        impl $nome {
            pub fn codigo(&self) -> &'static str {
                match self {
                    $(Self::$var => $codigo),+
                }
            }

            pub fn rotulo(&self) -> &'static str {
                match self {
                    $(Self::$var => $rotulo),+
                }
            }

            pub fn de_codigo(codigo: &str) -> Option<Self> {
                match codigo {
                    $($codigo => Some(Self::$var),)+
                    _ => None,
                }
            }
        }
    };
}

enum_catalogo!(Classe {
    Rf => ("rf", "Renda fixa"),
    Acoes => ("acoes", "Ações"),
    Fii => ("fii", "FIIs"),
    Ext => ("ext", "Exterior"),
    Outros => ("outros", "Outros"),
});

enum_catalogo!(Forma {
    RendaFixa => ("renda_fixa", "Renda fixa"),
    RendaVariavel => ("renda_variavel", "Renda variável"),
    Valor => ("valor", "Valor"),
});

enum_catalogo!(Indexador {
    Pos => ("pos", "Pós-fixado"),
    Pre => ("pre", "Prefixado"),
    Ipca => ("ipca", "Híbrido (IPCA + taxa)"),
    Selic => ("selic", "Selic + taxa"),
});

enum_catalogo!(Benchmark {
    Cdi => ("cdi", "CDI"),
    Selic => ("selic", "Selic"),
});

enum_catalogo!(Liquidez {
    Diaria => ("diaria", "Diária"),
    D1 => ("d1", "D+1"),
    D30 => ("d30", "D+30"),
    Vencimento => ("vencimento", "No vencimento"),
    Carencia => ("carencia", "Carência até uma data"),
});

enum_catalogo!(Ir {
    Regressiva => ("regressiva", "Tabela regressiva"),
    Isento => ("isento", "Isento"),
});

enum_catalogo!(Garantia {
    Fgc => ("fgc", "FGC"),
    Tesouro => ("tesouro", "Tesouro Nacional"),
    Nenhuma => ("nenhuma", "Sem garantia"),
});

#[derive(Debug, Clone, Copy)]
pub struct TipoInfo {
    pub codigo: &'static str,
    pub rotulo: &'static str,
    pub grupo: &'static str,
    pub classe: Classe,
    pub forma: Forma,
    pub indexador_forcado: Option<Indexador>,
    pub liquidez_forcada: Option<Liquidez>,
    pub ir_padrao: Option<Ir>,
    pub ir_editavel: bool,
    pub garantia: Option<Garantia>,
}

impl TipoInfo {
    /// Títulos do Tesouro Direto: indexador e liquidez vêm do tipo, sem emissor.
    pub fn e_tesouro(&self) -> bool {
        self.garantia == Some(Garantia::Tesouro)
    }
}

const fn renda_fixa(
    codigo: &'static str,
    rotulo: &'static str,
    ir: Ir,
    ir_editavel: bool,
    garantia: Garantia,
) -> TipoInfo {
    TipoInfo {
        codigo,
        rotulo,
        grupo: "Renda fixa",
        classe: Classe::Rf,
        forma: Forma::RendaFixa,
        indexador_forcado: None,
        liquidez_forcada: None,
        ir_padrao: Some(ir),
        ir_editavel,
        garantia: Some(garantia),
    }
}

const fn tesouro(codigo: &'static str, rotulo: &'static str, indexador: Indexador) -> TipoInfo {
    TipoInfo {
        indexador_forcado: Some(indexador),
        liquidez_forcada: Some(Liquidez::D1),
        ..renda_fixa(codigo, rotulo, Ir::Regressiva, false, Garantia::Tesouro)
    }
}

const fn renda_variavel(codigo: &'static str, rotulo: &'static str, classe: Classe) -> TipoInfo {
    TipoInfo {
        codigo,
        rotulo,
        grupo: "Renda variável",
        classe,
        forma: Forma::RendaVariavel,
        indexador_forcado: None,
        liquidez_forcada: None,
        ir_padrao: None,
        ir_editavel: false,
        garantia: None,
    }
}

const fn outro(codigo: &'static str, rotulo: &'static str) -> TipoInfo {
    TipoInfo {
        grupo: "Outros",
        classe: Classe::Outros,
        forma: Forma::Valor,
        ..renda_variavel(codigo, rotulo, Classe::Outros)
    }
}

static TIPOS: [TipoInfo; 17] = [
    renda_fixa("rf-cdb", "CDB", Ir::Regressiva, false, Garantia::Fgc),
    renda_fixa("rf-lci", "LCI", Ir::Isento, false, Garantia::Fgc),
    renda_fixa("rf-lca", "LCA", Ir::Isento, false, Garantia::Fgc),
    renda_fixa("rf-lc", "LC", Ir::Regressiva, false, Garantia::Fgc),
    renda_fixa(
        "rf-deb",
        "Debênture",
        Ir::Regressiva,
        true,
        Garantia::Nenhuma,
    ),
    renda_fixa("rf-cri", "CRI", Ir::Isento, false, Garantia::Nenhuma),
    renda_fixa("rf-cra", "CRA", Ir::Isento, false, Garantia::Nenhuma),
    tesouro("td-selic", "Tesouro Selic", Indexador::Selic),
    tesouro("td-pre", "Tesouro Prefixado", Indexador::Pre),
    tesouro("td-ipca", "Tesouro IPCA+", Indexador::Ipca),
    renda_variavel("rv-acao", "Ação", Classe::Acoes),
    renda_variavel("rv-fii", "Fundo imobiliário (FII)", Classe::Fii),
    renda_variavel("rv-etf", "ETF", Classe::Acoes),
    renda_variavel("rv-bdr", "BDR", Classe::Ext),
    outro("fundo", "Fundo de investimento"),
    outro("prev", "Previdência (PGBL/VGBL)"),
    outro("cripto", "Criptoativo"),
];

pub const INSTITUICOES: &[&str] = &[
    "Banco Inter",
    "XP Investimentos",
    "BTG Pactual",
    "Rico",
    "Nubank",
    "Banco do Brasil",
    "Itaú",
    "Bradesco",
    "Santander",
    "Caixa Econômica Federal",
    "C6 Bank",
];

pub fn tipos() -> &'static [TipoInfo] {
    &TIPOS
}

pub fn tipo(codigo: &str) -> Option<&'static TipoInfo> {
    TIPOS.iter().find(|t| t.codigo == codigo)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn catalogo_tem_os_17_tipos_na_ordem() {
        assert_eq!(tipos().len(), 17);
        assert_eq!(tipos()[0].codigo, "rf-cdb");
        assert_eq!(tipos()[16].codigo, "cripto");
    }

    #[test]
    fn lci_e_isenta_com_fgc_e_ir_nao_editavel() {
        let t = tipo("rf-lci").unwrap();
        assert_eq!(t.ir_padrao, Some(Ir::Isento));
        assert_eq!(t.garantia, Some(Garantia::Fgc));
        assert!(!t.ir_editavel);
    }

    #[test]
    fn debenture_tem_ir_editavel_e_sem_garantia() {
        let t = tipo("rf-deb").unwrap();
        assert!(t.ir_editavel);
        assert_eq!(t.garantia, Some(Garantia::Nenhuma));
    }

    #[test]
    fn tesouro_selic_forca_indexador_selic_e_liquidez_d1() {
        let t = tipo("td-selic").unwrap();
        assert_eq!(t.indexador_forcado, Some(Indexador::Selic));
        assert_eq!(t.liquidez_forcada, Some(Liquidez::D1));
    }

    #[test]
    fn bdr_e_da_classe_exterior() {
        assert_eq!(tipo("rv-bdr").unwrap().classe, Classe::Ext);
    }

    #[test]
    fn codigos_e_rotulos_ida_e_volta() {
        assert_eq!(Indexador::Ipca.rotulo(), "Híbrido (IPCA + taxa)");
        assert_eq!(Liquidez::de_codigo("d30"), Some(Liquidez::D30));
        assert_eq!(Ir::de_codigo("xx"), None);
        assert_eq!(Classe::Fii.codigo(), "fii");
        assert_eq!(INSTITUICOES.len(), 11);
        assert!(tipo("nada").is_none());
    }
}
