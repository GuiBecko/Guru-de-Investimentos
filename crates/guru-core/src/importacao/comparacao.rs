//! Compara a posição da B3 com os investimentos já cadastrados, linha a linha.

use std::collections::HashSet;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::instituicao::{chave_instituicao, e_corretora_conhecida};
use super::posicao::{AbaRv, LinhaB3, Posicao};
use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{ErroCampo, Investimento, NovoInvestimento, tipo, validar};
use crate::metas::formatar_reais;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estado {
    Novo,
    Atualizar,
    Divergente,
    Igual,
}

impl Estado {
    pub fn codigo(&self) -> &'static str {
        match self {
            Estado::Novo => "novo",
            Estado::Atualizar => "atualizar",
            Estado::Divergente => "divergente",
            Estado::Igual => "igual",
        }
    }

    pub fn rotulo(&self) -> &'static str {
        match self {
            Estado::Novo => "novo",
            Estado::Atualizar => "atualizar",
            Estado::Divergente => "divergente",
            Estado::Igual => "igual",
        }
    }

    /// Ordem de exibição: novo, atualizar, divergente, igual.
    pub(super) fn ordem(&self) -> u8 {
        match self {
            Estado::Novo => 0,
            Estado::Atualizar => 1,
            Estado::Divergente => 2,
            Estado::Igual => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Adicionar,
    Ignorar,
    UsarArquivo,
    Manter,
    /// Liga a linha do arquivo a este investimento (id) já cadastrado.
    Vincular(i64),
}

impl Acao {
    pub fn codigo(&self) -> &'static str {
        match self {
            Acao::Adicionar => "adicionar",
            Acao::Ignorar => "ignorar",
            Acao::UsarArquivo => "usar_arquivo",
            Acao::Manter => "manter",
            Acao::Vincular(_) => "vincular",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    /// `b3:<posição da linha na planilha>` ou `btg:<n>`; estável dentro de uma comparação.
    pub chave: String,
    pub estado: Estado,
    pub ativo: String,
    pub instituicao: String,
    pub no_app: Option<String>,
    pub no_arquivo: Option<String>,
    pub investimentos: Vec<i64>,
    pub acoes: Vec<Acao>,
    pub acao_padrao: Acao,
    pub sugestao: Option<NovoInvestimento>,
    pub faltando: Vec<ErroCampo>,
    /// O que a aplicação precisa do arquivo; não vai para a interface.
    pub(crate) dados: DadosArquivo,
}

/// Dados do arquivo que a aplicação grava ou usa (o texto do item é só para exibir).
#[derive(Debug, Clone, Default)]
pub(crate) struct DadosArquivo {
    /// Ticker (renda variável), `Código` (renda fixa) ou ISIN / `<tipo>:<vencimento>` (Tesouro).
    pub codigo: Option<String>,
    /// Quantidade total do grupo, só na renda variável.
    pub quantidade: Option<i64>,
    /// `Valor Aplicado` somado do grupo, só no Tesouro (`None` se a planilha não traz).
    pub aplicado_centavos: Option<i64>,
    /// Preço médio em centavos (extrato da BTG).
    pub preco_medio_centavos: Option<i64>,
    /// Saldo bruto e a data a que ele se refere (extrato da BTG).
    pub saldo: Option<(i64, NaiveDate)>,
    /// Campos a completar num investimento já cadastrado (extrato da BTG).
    pub completar: Option<Completar>,
}

/// Campos de um investimento que o arquivo pode completar quando estão vazios no app.
#[derive(Debug, Clone, Default)]
pub(crate) struct Completar {
    pub emissor: Option<String>,
    pub indexador: Option<String>,
    pub taxa: Option<String>,
    pub benchmark: Option<String>,
}

/// De onde vem o arquivo importado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fonte {
    B3,
    Btg,
}

impl Fonte {
    pub fn codigo(&self) -> &'static str {
        match self {
            Fonte::B3 => "b3",
            Fonte::Btg => "btg",
        }
    }

    pub fn rotulo(&self) -> &'static str {
        match self {
            Fonte::B3 => "B3",
            Fonte::Btg => "BTG",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Comparacao {
    pub fonte: Fonte,
    /// Data a que o arquivo se refere (extrato da BTG); `None` na B3.
    pub data_arquivo: Option<NaiveDate>,
    pub itens: Vec<Item>,
    pub avisos: Vec<String>,
    pub linhas_lidas: usize,
}

/// Sem acentos, em maiúsculas, sem pontuação nem sufixo societário (`S/A`, `S.A.`, `SA`, `LTDA`).
pub(super) fn normalizar_emissor(texto: &str) -> String {
    let sem_acento: String = texto
        .chars()
        .flat_map(char::to_uppercase)
        .map(|c| match c {
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'Ç' => 'C',
            outro => outro,
        })
        .collect();
    let mut palavras: Vec<&str> = sem_acento
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect();
    loop {
        match palavras.as_slice() {
            [.., "S", "A"] => palavras.truncate(palavras.len() - 2),
            [.., "SA" | "LTDA"] => palavras.truncate(palavras.len() - 1),
            _ => break,
        }
    }
    palavras.join(" ")
}

pub(super) fn quantidade_texto(tipo_codigo: &str, q: i64) -> String {
    let um = q == 1;
    let nome = match tipo_codigo {
        "rv-acao" => ["ação", "ações"],
        "rv-bdr" => ["BDR", "BDRs"],
        _ => ["cota", "cotas"],
    };
    format!("{q} {}", nome[usize::from(!um)])
}

/// Valor em reais para centavos; `None` se não cabe (a conta nunca estoura: é checada).
pub(super) fn centavos(valor: Decimal) -> Option<i64> {
    let vezes_cem = valor.checked_mul(Decimal::ONE_HUNDRED)?;
    i64::try_from(vezes_cem.round()).ok()
}

/// Produto da linha cujo valor em reais não cabe em centavos; ela vira aviso e não é comparada.
fn fora_do_limite(linha: &LinhaB3) -> Option<&str> {
    let (produto, valor) = match linha {
        LinhaB3::Tesouro {
            produto,
            valor_aplicado,
            ..
        } => (produto, valor_aplicado),
        LinhaB3::RendaFixa {
            produto,
            valor_curva,
            ..
        } => (produto, valor_curva),
        LinhaB3::RendaVariavel { .. } => return None,
    };
    valor
        .is_some_and(|v| centavos(v).is_none())
        .then_some(produto.as_str())
}

pub(super) fn chave_ativo_rv(ticker: &str) -> String {
    format!("rv:{}", ticker.to_uppercase())
}

fn chave_ativo_td(tipo: &str, vencimento: NaiveDate) -> String {
    format!("td:{tipo}:{vencimento}")
}

/// Chaves que identificam uma linha de renda fixa: o código (se há) e vencimento + emissor.
pub(super) fn chaves_ativo_rf(codigo: &str, vencimento: NaiveDate, emissor: &str) -> Vec<String> {
    let mut chaves = vec![format!("rf:v:{vencimento}:{}", normalizar_emissor(emissor))];
    if !codigo.is_empty() {
        chaves.push(format!("rf:c:{codigo}"));
    }
    chaves
}

fn tipo_da_aba(aba: AbaRv) -> &'static str {
    match aba {
        AbaRv::Acao => "rv-acao",
        AbaRv::Bdr => "rv-bdr",
        AbaRv::Etf => "rv-etf",
        AbaRv::Fii => "rv-fii",
    }
}

pub(super) fn faltando(sugestao: &NovoInvestimento, hoje: NaiveDate) -> Vec<ErroCampo> {
    validar(sugestao, hoje).err().unwrap_or_default()
}

/// Estado e ações quando há pares cadastrados e `igual` diz se o total confere.
pub(super) fn com_pares(pares: &[&Investimento], igual: bool) -> (Estado, Vec<Acao>) {
    if igual {
        return (Estado::Igual, vec![Acao::Manter]);
    }
    match pares {
        [unico] if unico.codigo_b3.is_some() => {
            (Estado::Atualizar, vec![Acao::UsarArquivo, Acao::Manter])
        }
        [_] => (Estado::Divergente, vec![Acao::Manter, Acao::UsarArquivo]),
        _ => (Estado::Divergente, vec![Acao::Manter]),
    }
}

pub(super) fn padrao(estado: Estado) -> Acao {
    match estado {
        Estado::Novo => Acao::Adicionar,
        Estado::Atualizar => Acao::UsarArquivo,
        _ => Acao::Manter,
    }
}

pub(super) fn ids(pares: &[&Investimento]) -> Vec<i64> {
    pares.iter().map(|i| i.id).collect()
}

pub(super) fn total_aplicado(pares: &[&Investimento]) -> i64 {
    pares
        .iter()
        .fold(0i64, |a, i| a.saturating_add(i.valor_aplicado_centavos))
}

/// Linhas de renda variável do mesmo ticker, da mesma aba e da mesma corretora.
struct GrupoRv<'a> {
    ticker: String,
    aba: AbaRv,
    chave_instituicao: String,
    primeira_linha: usize,
    quantidade: i64,
    /// Grafia da primeira linha do grupo.
    instituicao: &'a str,
}

/// Linhas do Tesouro do mesmo título, do mesmo vencimento e da mesma corretora.
struct GrupoTesouro<'a> {
    tipo: String,
    vencimento: NaiveDate,
    produto: String,
    isin: Option<String>,
    primeira_linha: usize,
    /// `None` se alguma linha do grupo não traz o valor aplicado.
    aplicado: Option<i64>,
    chave_instituicao: String,
    /// Grafia da primeira linha do grupo.
    instituicao: &'a str,
}

pub(super) struct Contexto<'a> {
    pub(super) investimentos: &'a [Investimento],
    pub(super) hoje: NaiveDate,
    pub(super) usados: HashSet<i64>,
    /// `(chave do ativo, chave da instituição)` de cada linha do arquivo; um investimento cuja
    /// instituição já tem linha do mesmo ativo não é candidato a "outra corretora".
    pub(super) ocupadas: HashSet<(String, String)>,
}

/// Ações de um item divergente que só tem candidatos de outra grafia de corretora.
pub(super) fn acoes_de_outra_corretora(candidatos: &[&Investimento]) -> Vec<Acao> {
    let mut acoes: Vec<Acao> = candidatos.iter().map(|i| Acao::Vincular(i.id)).collect();
    acoes.push(Acao::Adicionar);
    acoes.push(Acao::Ignorar);
    acoes
}

impl<'a> Contexto<'a> {
    /// Sem par na corretora da linha: os investimentos (que passam em `filtro`) de corretora fora
    /// do catálogo, digitada à mão, com o código gravado igual a `codigo`. O código foi gravado
    /// por um `Vincular` (ou por outra importação), então eles contam como pares da mesma
    /// corretora, sem que a corretora deles mude. Ficam de fora os já usados e os de corretora
    /// que tem, no arquivo, linha de algum dos `ativos`.
    pub(super) fn ligados_pelo_codigo(
        &self,
        filtro: impl Fn(&Investimento) -> bool,
        codigo: Option<&str>,
        ativos: &[String],
    ) -> Vec<&'a Investimento> {
        let Some(codigo) = codigo.filter(|c| !c.is_empty()) else {
            return Vec::new();
        };
        self.investimentos
            .iter()
            .filter(|i| i.codigo_b3.as_deref() == Some(codigo) && filtro(i))
            .filter(|i| !self.usados.contains(&i.id) && !e_corretora_conhecida(&i.instituicao))
            .filter(|i| {
                let chave = chave_instituicao(&i.instituicao);
                !ativos
                    .iter()
                    .any(|a| self.ocupadas.contains(&(a.clone(), chave.clone())))
            })
            .collect()
    }
}

impl Contexto<'_> {
    pub(super) fn marcar(&mut self, pares: &[&Investimento]) {
        self.usados.extend(pares.iter().map(|i| i.id));
    }

    /// Dos `candidatos` (regra sem instituição), os ainda livres e cuja corretora não tem, no
    /// arquivo, linha de nenhum dos `ativos`: provavelmente o mesmo ativo com outra grafia da
    /// corretora. Uma corretora do catálogo é outra corretora de fato e não entra, salvo
    /// quando o candidato veio `por_codigo` (código gravado igual: a identidade mais forte).
    /// Não os marca como usados.
    pub(super) fn de_outra_corretora<'i>(
        &self,
        candidatos: Vec<&'i Investimento>,
        ativos: &[String],
        por_codigo: bool,
    ) -> Vec<&'i Investimento> {
        candidatos
            .into_iter()
            .filter(|i| !self.usados.contains(&i.id))
            .filter(|i| por_codigo || !e_corretora_conhecida(&i.instituicao))
            .filter(|i| {
                let chave = chave_instituicao(&i.instituicao);
                !ativos
                    .iter()
                    .any(|a| self.ocupadas.contains(&(a.clone(), chave.clone())))
            })
            .collect()
    }

    /// Item divergente: só há candidatos de outra grafia de corretora; o padrão é ignorar.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn outra_corretora(
        &self,
        chave: String,
        ativo: String,
        instituicao: String,
        textos: (Option<String>, Option<String>),
        candidatos: &[&Investimento],
        sugestao: NovoInvestimento,
    ) -> Item {
        let mut item = self.item(
            chave,
            Estado::Divergente,
            ativo,
            instituicao,
            textos,
            ids(candidatos),
            acoes_de_outra_corretora(candidatos),
            Some(sugestao),
        );
        item.acao_padrao = Acao::Ignorar;
        item
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn item(
        &self,
        chave: String,
        estado: Estado,
        ativo: String,
        instituicao: String,
        textos: (Option<String>, Option<String>),
        investimentos: Vec<i64>,
        acoes: Vec<Acao>,
        sugestao: Option<NovoInvestimento>,
    ) -> Item {
        let faltando = sugestao
            .as_ref()
            .map(|s| faltando(s, self.hoje))
            .unwrap_or_default();
        Item {
            chave,
            estado,
            ativo,
            instituicao,
            no_app: textos.0,
            no_arquivo: textos.1,
            investimentos,
            acao_padrao: padrao(estado),
            acoes,
            sugestao,
            faltando,
            dados: DadosArquivo::default(),
        }
    }

    fn renda_variavel(
        &mut self,
        chave: String,
        aba: AbaRv,
        ticker: &str,
        instituicao: &str,
        quantidade: i64,
    ) -> Item {
        let tipo_codigo = tipo_da_aba(aba);
        let chave_inst = chave_instituicao(instituicao);
        let mut pares: Vec<&Investimento> = self
            .investimentos
            .iter()
            .filter(|i| {
                i.tipo.starts_with("rv-")
                    && chave_instituicao(&i.instituicao) == chave_inst
                    && i.ticker
                        .as_deref()
                        .is_some_and(|t| t.eq_ignore_ascii_case(ticker))
            })
            .collect();
        if pares.is_empty() {
            pares = self.ligados_pelo_codigo(
                |i| i.tipo.starts_with("rv-"),
                Some(&ticker.to_uppercase()),
                &[chave_ativo_rv(ticker)],
            );
        }
        let no_arquivo = Some(quantidade_texto(tipo_codigo, quantidade));
        let dados_rv = DadosArquivo {
            codigo: Some(ticker.to_uppercase()),
            quantidade: Some(quantidade),
            aplicado_centavos: None,
            ..Default::default()
        };
        self.marcar(&pares);
        if pares.is_empty() {
            let sugestao = NovoInvestimento {
                tipo: tipo_codigo.to_string(),
                instituicao: instituicao.to_string(),
                ticker: Some(ticker.to_uppercase()),
                quantidade: Some(quantidade),
                ..Default::default()
            };
            let outros: Vec<&Investimento> = self
                .investimentos
                .iter()
                .filter(|i| {
                    i.tipo.starts_with("rv-")
                        && i.ticker
                            .as_deref()
                            .is_some_and(|t| t.eq_ignore_ascii_case(ticker))
                })
                .collect();
            let outros = self.de_outra_corretora(outros, &[chave_ativo_rv(ticker)], false);
            if !outros.is_empty() {
                let mut item = self.outra_corretora(
                    chave,
                    ticker.to_string(),
                    instituicao.to_string(),
                    (None, no_arquivo),
                    &outros,
                    sugestao,
                );
                item.dados = dados_rv;
                return item;
            }
            let mut item = self.item(
                chave,
                Estado::Novo,
                ticker.to_string(),
                instituicao.to_string(),
                (None, no_arquivo),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao),
            );
            item.dados = dados_rv;
            return item;
        }
        let soma = pares
            .iter()
            .filter_map(|i| i.quantidade)
            .fold(0i64, i64::saturating_add);
        let (estado, acoes) = com_pares(&pares, soma == quantidade);
        let mut item = self.item(
            chave,
            estado,
            ticker.to_string(),
            instituicao.to_string(),
            (Some(quantidade_texto(tipo_codigo, soma)), no_arquivo),
            ids(&pares),
            acoes,
            None,
        );
        item.dados = dados_rv;
        item
    }

    fn renda_fixa(&mut self, chave: String, linha: &LinhaB3) -> Option<Item> {
        let LinhaB3::RendaFixa {
            produto,
            tipo: tipo_rf,
            instituicao,
            emissor,
            codigo,
            indexador,
            benchmark,
            emissao,
            vencimento,
            valor_curva,
            ..
        } = linha
        else {
            return None;
        };
        let chave_inst = chave_instituicao(instituicao);
        let emissor_norm = normalizar_emissor(emissor);
        let venc = vencimento.to_string();
        // Regra de casamento; `mesma` exige a mesma corretora.
        // Devolve também se os candidatos vieram pelo código gravado.
        let buscar = |mesma: bool| -> (Vec<&Investimento>, bool) {
            let rf = |i: &&Investimento| {
                i.tipo.starts_with("rf-")
                    && (!mesma || chave_instituicao(&i.instituicao) == chave_inst)
            };
            let por_codigo: Vec<&Investimento> = self
                .investimentos
                .iter()
                .filter(rf)
                .filter(|i| {
                    !codigo.is_empty()
                        && i.codigo_b3.as_deref() == Some(codigo)
                        && !self.usados.contains(&i.id)
                })
                .collect();
            if !por_codigo.is_empty() {
                return (por_codigo, true);
            }
            let sem_codigo = self
                .investimentos
                .iter()
                .filter(rf)
                .filter(|i| i.codigo_b3.is_none() && !self.usados.contains(&i.id))
                .filter(|i| i.vencimento.as_deref() == Some(venc.as_str()))
                .filter(|i| {
                    !emissor_norm.is_empty()
                        && i.emissor.as_deref().map(normalizar_emissor).as_deref()
                            == Some(emissor_norm.as_str())
                })
                .filter(|i| tipo_rf.as_deref().is_none_or(|t| i.tipo == t))
                .collect();
            (sem_codigo, false)
        };
        let ativos = chaves_ativo_rf(codigo, *vencimento, emissor);
        let (mut pares, _) = buscar(true);
        if pares.is_empty() {
            pares = self.ligados_pelo_codigo(|i| i.tipo.starts_with("rf-"), Some(codigo), &ativos);
        }
        let outros = if pares.is_empty() {
            let (candidatos, por_codigo) = buscar(false);
            self.de_outra_corretora(candidatos, &ativos, por_codigo)
        } else {
            Vec::new()
        };
        // Com dois ou mais candidatos nenhum fica casado: as outras linhas com a mesma chave
        // também os oferecem para vincular.
        if pares.len() < 2 {
            self.marcar(&pares);
        }
        let ativo = produto.replacen(" - ", " ", 1);
        let no_arquivo = valor_curva
            .and_then(centavos)
            .map(|c| format!("{} na curva", formatar_reais(c)));
        let no_app = (!pares.is_empty()).then(|| formatar_reais(total_aplicado(&pares)));
        let sugestao = || {
            let info = tipo_rf.as_deref().and_then(tipo);
            NovoInvestimento {
                tipo: tipo_rf.clone().unwrap_or_default(),
                instituicao: instituicao.clone(),
                emissor: (!emissor.is_empty()).then(|| emissor.clone()),
                indexador: indexador.clone(),
                benchmark: benchmark.clone(),
                data_aplicacao: emissao.map(|d| d.to_string()),
                vencimento: Some(vencimento.to_string()),
                liquidez: info
                    .and_then(|t| t.liquidez_padrao)
                    .map(|l| l.codigo().to_string()),
                ir: info
                    .and_then(|t| t.ir_padrao)
                    .map(|i| i.codigo().to_string()),
                ..Default::default()
            }
        };
        let mut item = match pares.as_slice() {
            [] if !outros.is_empty() => self.outra_corretora(
                chave,
                ativo,
                instituicao.clone(),
                (None, no_arquivo),
                &outros,
                sugestao(),
            ),
            [] => self.item(
                chave,
                Estado::Novo,
                ativo,
                instituicao.clone(),
                (None, no_arquivo),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao()),
            ),
            [_] => self.item(
                chave,
                Estado::Igual,
                ativo,
                instituicao.clone(),
                (no_app, no_arquivo),
                ids(&pares),
                vec![Acao::Manter],
                None,
            ),
            _ => {
                let mut acoes = vec![Acao::Manter];
                acoes.extend(pares.iter().map(|i| Acao::Vincular(i.id)));
                acoes.push(Acao::Adicionar);
                self.item(
                    chave,
                    Estado::Divergente,
                    ativo,
                    instituicao.clone(),
                    (no_app, no_arquivo),
                    ids(&pares),
                    acoes,
                    Some(sugestao()),
                )
            }
        };
        item.dados.codigo = (!codigo.is_empty()).then(|| codigo.clone());
        Some(item)
    }

    fn tesouro(&mut self, chave: String, grupo: &GrupoTesouro) -> Item {
        let venc = grupo.vencimento.to_string();
        let grupo_venc = venc.clone();
        let instituicao = grupo.instituicao.to_string();
        let codigo_td = grupo
            .isin
            .clone()
            .unwrap_or_else(|| format!("{}:{}", grupo.tipo, venc));
        let mut pares: Vec<&Investimento> = self
            .investimentos
            .iter()
            .filter(|i| {
                i.tipo == grupo.tipo
                    && chave_instituicao(&i.instituicao) == grupo.chave_instituicao
                    && i.vencimento.as_deref() == Some(venc.as_str())
                    && !self.usados.contains(&i.id)
            })
            .collect();
        if pares.is_empty() {
            pares = self.ligados_pelo_codigo(
                |i| i.tipo == grupo.tipo,
                Some(&codigo_td),
                &[chave_ativo_td(&grupo.tipo, grupo.vencimento)],
            );
        }
        self.marcar(&pares);
        let aplicado_b3 = grupo.aplicado;
        let dados_td = DadosArquivo {
            codigo: Some(codigo_td),
            quantidade: None,
            aplicado_centavos: aplicado_b3,
            ..Default::default()
        };
        let no_arquivo = aplicado_b3.map(formatar_reais);
        if pares.is_empty() {
            let info = tipo(&grupo.tipo);
            let sugestao = NovoInvestimento {
                tipo: grupo.tipo.clone(),
                instituicao: instituicao.clone(),
                vencimento: Some(venc),
                valor_aplicado_centavos: aplicado_b3,
                liquidez: info
                    .and_then(|t| t.liquidez_padrao)
                    .map(|l| l.codigo().to_string()),
                ir: info
                    .and_then(|t| t.ir_padrao)
                    .map(|i| i.codigo().to_string()),
                ..Default::default()
            };
            let outros: Vec<&Investimento> = self
                .investimentos
                .iter()
                .filter(|i| {
                    i.tipo == grupo.tipo && i.vencimento.as_deref() == Some(grupo_venc.as_str())
                })
                .collect();
            let outros = self.de_outra_corretora(
                outros,
                &[chave_ativo_td(&grupo.tipo, grupo.vencimento)],
                false,
            );
            if !outros.is_empty() {
                let mut item = self.outra_corretora(
                    chave,
                    grupo.produto.clone(),
                    instituicao,
                    (None, no_arquivo),
                    &outros,
                    sugestao,
                );
                item.dados = dados_td;
                return item;
            }
            let mut item = self.item(
                chave,
                Estado::Novo,
                grupo.produto.clone(),
                instituicao,
                (None, no_arquivo),
                Vec::new(),
                vec![Acao::Adicionar, Acao::Ignorar],
                Some(sugestao),
            );
            item.dados = dados_td;
            return item;
        }
        let soma = total_aplicado(&pares);
        let (estado, acoes) = match aplicado_b3 {
            // Sem o valor aplicado na planilha não há o que usar da B3: só manter.
            None => (Estado::Divergente, vec![Acao::Manter]),
            Some(b) => com_pares(&pares, b.abs_diff(soma) <= 1),
        };
        let mut item = self.item(
            chave,
            estado,
            grupo.produto.clone(),
            instituicao,
            (Some(formatar_reais(soma)), no_arquivo),
            ids(&pares),
            acoes,
            None,
        );
        item.dados = dados_td;
        item
    }
}

/// Compara cada linha da posição com a carteira. `hoje` só serve para validar as sugestões.
pub fn comparar(db: &Db, posicao: &Posicao, hoje: NaiveDate) -> Result<Comparacao, CoreError> {
    let investimentos = db.listar_investimentos_brutos()?;
    let mut ctx = Contexto {
        investimentos: &investimentos,
        hoje,
        usados: HashSet::new(),
        ocupadas: HashSet::new(),
    };
    let mut itens = Vec::new();
    let mut avisos = posicao.avisos.clone();
    let mut linhas: Vec<(usize, &LinhaB3)> = Vec::new();
    for (n, linha) in posicao.linhas.iter().enumerate() {
        match fora_do_limite(linha) {
            Some(produto) => avisos.push(format!("Valor fora do limite em {produto}.")),
            None => linhas.push((n, linha)),
        }
    }
    for &(_, linha) in &linhas {
        let (ativos, instituicao) = match linha {
            LinhaB3::RendaVariavel {
                ticker,
                instituicao,
                ..
            } => (vec![chave_ativo_rv(ticker)], instituicao),
            LinhaB3::Tesouro {
                tipo,
                vencimento,
                instituicao,
                ..
            } => (vec![chave_ativo_td(tipo, *vencimento)], instituicao),
            LinhaB3::RendaFixa {
                codigo,
                vencimento,
                emissor,
                instituicao,
                ..
            } => (chaves_ativo_rf(codigo, *vencimento, emissor), instituicao),
        };
        let chave_inst = chave_instituicao(instituicao);
        for ativo in ativos {
            ctx.ocupadas.insert((ativo, chave_inst.clone()));
        }
    }
    let mut rv: Vec<GrupoRv> = Vec::new();
    let mut td: Vec<GrupoTesouro> = Vec::new();
    for &(n, linha) in &linhas {
        match linha {
            LinhaB3::RendaVariavel {
                aba,
                ticker,
                instituicao,
                quantidade,
                ..
            } => {
                let chave = ticker.to_uppercase();
                let chave_inst = chave_instituicao(instituicao);
                match rv.iter_mut().find(|g| {
                    g.ticker == chave && g.aba == *aba && g.chave_instituicao == chave_inst
                }) {
                    Some(g) => g.quantidade = g.quantidade.saturating_add(*quantidade),
                    None => rv.push(GrupoRv {
                        ticker: chave,
                        aba: *aba,
                        chave_instituicao: chave_inst,
                        primeira_linha: n,
                        quantidade: *quantidade,
                        instituicao,
                    }),
                }
            }
            LinhaB3::Tesouro {
                produto,
                tipo,
                isin,
                instituicao,
                vencimento,
                valor_aplicado,
                ..
            } => {
                let valor = valor_aplicado.and_then(centavos);
                let chave_inst = chave_instituicao(instituicao);
                match td.iter_mut().find(|g| {
                    g.tipo == *tipo
                        && g.vencimento == *vencimento
                        && g.chave_instituicao == chave_inst
                }) {
                    Some(g) => {
                        g.aplicado = g.aplicado.zip(valor).map(|(a, b)| a.saturating_add(b));
                    }
                    None => td.push(GrupoTesouro {
                        tipo: tipo.clone(),
                        vencimento: *vencimento,
                        produto: produto.clone(),
                        isin: isin.clone(),
                        primeira_linha: n,
                        aplicado: valor,
                        chave_instituicao: chave_inst,
                        instituicao,
                    }),
                }
            }
            LinhaB3::RendaFixa { .. } => {}
        }
    }
    for &(n, linha) in &linhas {
        let chave = format!("b3:{n}");
        let item = match linha {
            LinhaB3::RendaVariavel { ticker, aba, .. } => {
                let ticker_maiusculo = ticker.to_uppercase();
                rv.iter()
                    .find(|g| {
                        g.ticker == ticker_maiusculo && g.aba == *aba && g.primeira_linha == n
                    })
                    .map(|g| ctx.renda_variavel(chave, g.aba, ticker, g.instituicao, g.quantidade))
            }
            LinhaB3::RendaFixa { .. } => ctx.renda_fixa(chave, linha),
            LinhaB3::Tesouro { .. } => td
                .iter()
                .find(|g| g.primeira_linha == n)
                .map(|g| ctx.tesouro(chave, g)),
        };
        itens.extend(item);
    }
    itens.sort_by_key(|i| i.estado.ordem());
    Ok(Comparacao {
        fonte: Fonte::B3,
        data_arquivo: None,
        itens,
        avisos,
        linhas_lidas: posicao.linhas_lidas,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn emissor_sem_sufixos_nem_acentos() {
        assert_eq!(normalizar_emissor("Banco Fictício S/A"), "BANCO FICTICIO");
        assert_eq!(normalizar_emissor("BANCO FICTICIO S.A."), "BANCO FICTICIO");
        assert_eq!(normalizar_emissor("Banco Fictício SA"), "BANCO FICTICIO");
        assert_eq!(
            normalizar_emissor("Financeira  Exemplo Ltda."),
            "FINANCEIRA EXEMPLO"
        );
        assert_eq!(normalizar_emissor("Sá Crédito"), "SA CREDITO");
    }
}
