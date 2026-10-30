//! Baixa só o que falta das fontes oficiais e guarda pelo repositório.

use std::collections::HashSet;
use std::time::Duration;

use chrono::{Datelike, Days, Months, NaiveDate, Utc};
use rust_decimal::Decimal;

use super::rede::{Baixador, url_b3, url_sgs, url_tesouro};
use super::{B3Alvo, ErroFonte, PontoDiario, StatusFonte, ler_cotahist_zip, ler_sgs};
use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::Investimento;

const ESPERA_NOVA_TENTATIVA: Duration = Duration::from_secs(5);
const ANOS_POR_CONSULTA: u32 = 10;
const FOLGA_TESOURO_DIAS: u64 = 10;
const TENTATIVAS_B3: u32 = 7;
const MESES_DE_HISTORICO: u32 = 12;
/// Referências usadas pela fase 3b, gravadas junto com os tickers da carteira.
const TICKERS_REFERENCIA: [&str; 2] = ["BOVA11", "XFIX11"];

const MSG_BCB: &str = "O Banco Central não respondeu. Tente de novo mais tarde.";
const MSG_TESOURO: &str = "O Tesouro Transparente não respondeu. Tente de novo mais tarde.";
const MSG_B3: &str = "A B3 não respondeu. Tente de novo mais tarde.";
const MSG_B3_SEM_PREGAO: &str = "A B3 não publicou cotações nos últimos 7 dias.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoAtualizacao {
    /// As cinco fontes, na ordem fixa, como ficaram depois da atualização.
    pub fontes: Vec<StatusFonte>,
    /// Quantas linhas novas foram gravadas (todas as fontes somadas).
    pub novos: usize,
}

/// Falha de uma fonte (vira `erro` no status) ou do banco (vira `Err`).
enum Falha {
    Fonte,
    Banco(CoreError),
}

impl From<CoreError> for Falha {
    fn from(e: CoreError) -> Self {
        Falha::Banco(e)
    }
}

impl From<ErroFonte> for Falha {
    fn from(_: ErroFonte) -> Self {
        Falha::Fonte
    }
}

/// Título do Tesouro Direto no CSV (`Tipo Titulo`, vencimento) de um investimento `td-*`.
/// O tipo do CSV é o rótulo do catálogo ("Tesouro Selic", "Tesouro Prefixado", "Tesouro IPCA+").
pub fn titulo_tesouro(inv: &Investimento) -> Option<(String, NaiveDate)> {
    if !inv.tipo.starts_with("td-") {
        return None;
    }
    let vencimento = NaiveDate::parse_from_str(inv.vencimento.as_deref()?, "%Y-%m-%d").ok()?;
    Some((inv.tipo_rotulo.clone(), vencimento))
}

fn chave_tesouro(tipo: &str, vencimento: NaiveDate) -> String {
    format!("tesouro:{tipo}:{}", iso(vencimento))
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Tenta; em caso de qualquer falha, espera 5 s e tenta uma segunda e última vez.
fn com_nova_tentativa<T>(
    espera: &dyn Fn(Duration),
    mut f: impl FnMut() -> Result<T, ErroFonte>,
) -> Result<T, ErroFonte> {
    match f() {
        Ok(v) => Ok(v),
        Err(_) => {
            espera(ESPERA_NOVA_TENTATIVA);
            f()
        }
    }
}

/// Como `com_nova_tentativa`, mas o 404 do SGS é resposta definitiva (sem dados), sem repetir.
fn com_nova_tentativa_sgs<T>(
    espera: &dyn Fn(Duration),
    mut f: impl FnMut() -> Result<T, ErroFonte>,
) -> Result<T, ErroFonte> {
    match f() {
        Err(ErroFonte::Status(404)) => Err(ErroFonte::Status(404)),
        Err(_) => {
            espera(ESPERA_NOVA_TENTATIVA);
            f()
        }
        Ok(v) => Ok(v),
    }
}

/// Janelas `[de, ate]` de no máximo 10 anos cobrindo `de..=ate`.
fn janelas(de: NaiveDate, ate: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
    let mut out = Vec::new();
    let mut ini = de;
    while ini <= ate {
        let fim = ini
            .checked_add_months(Months::new(12 * ANOS_POR_CONSULTA))
            .and_then(|d| d.pred_opt())
            .map_or(ate, |d| d.min(ate));
        out.push((ini, fim));
        match fim.succ_opt() {
            Some(prox) => ini = prox,
            None => break,
        }
    }
    out
}

enum Procura {
    Achou(NaiveDate),
    JaCoberto,
    Esgotou,
}

struct Contexto<'a> {
    db: &'a Db,
    baixador: &'a dyn Baixador,
    espera: &'a dyn Fn(Duration),
    hoje: NaiveDate,
    inicio: NaiveDate,
}

pub fn atualizar(
    db: &Db,
    baixador: &dyn Baixador,
    hoje: NaiveDate,
    espera: &dyn Fn(Duration),
) -> Result<ResultadoAtualizacao, CoreError> {
    let carteira = db.listar_investimentos(hoje)?;
    let padrao = hoje
        .checked_sub_months(Months::new(13))
        .unwrap_or(NaiveDate::MIN);
    let inicio = carteira
        .iter()
        .filter_map(|i| NaiveDate::parse_from_str(&i.data_aplicacao, "%Y-%m-%d").ok())
        .fold(padrao, NaiveDate::min);
    let ctx = Contexto {
        db,
        baixador,
        espera,
        hoje,
        inicio,
    };

    let mut novos = 0;
    for serie in ["cdi", "selic"] {
        let r = ctx.indice_diario(serie);
        novos += ctx.fechar(serie, r, MSG_BCB, |c| {
            Ok(c.db.ultima_data_indice(serie)?.map(iso))
        })?;
    }
    let r = ctx.ipca();
    novos += ctx.fechar("ipca", r, MSG_BCB, |c| {
        let meses = c.db.ipca_meses("0000-00", "9999-99")?;
        Ok(meses.last().map(|(m, _)| format!("{m}-01")))
    })?;

    let titulos: HashSet<(String, NaiveDate)> =
        carteira.iter().filter_map(titulo_tesouro).collect();
    if !titulos.is_empty()
        && (ctx.tesouro_incompleto(&titulos)? || !ctx.atualizado_hoje("tesouro")?)
    {
        let mut ultima = None;
        let r = ctx.tesouro(&titulos, &mut ultima);
        novos += ctx.fechar("tesouro", r, MSG_TESOURO, |_| Ok(ultima.map(iso)))?;
    }

    if carteira.iter().any(|i| i.tipo.starts_with("rv-")) {
        let mut tickers: HashSet<String> = carteira
            .iter()
            .filter(|i| i.tipo.starts_with("rv-"))
            .filter_map(|i| i.ticker.as_deref())
            .map(|t| t.trim().to_uppercase())
            .collect();
        tickers.extend(TICKERS_REFERENCIA.iter().map(|t| t.to_string()));
        let mut aviso = None;
        let r = ctx.b3(&tickers, &mut novos, &mut aviso);
        ctx.fechar_b3(r, aviso)?;
    }

    Ok(ResultadoAtualizacao {
        fontes: db.status_fontes()?,
        novos,
    })
}

impl Contexto<'_> {
    fn status(&self, fonte: &str) -> Result<StatusFonte, CoreError> {
        Ok(self
            .db
            .status_fontes()?
            .into_iter()
            .find(|f| f.fonte == fonte)
            .unwrap_or(StatusFonte {
                fonte: fonte.to_string(),
                ultima_data: None,
                atualizado_em: None,
                erro: None,
            }))
    }

    /// A fonte terminou sem erro hoje (ou depois: o relógio é UTC e `hoje` é a data local).
    fn atualizado_hoje(&self, fonte: &str) -> Result<bool, CoreError> {
        let s = self.status(fonte)?;
        Ok(s.erro.is_none()
            && s.atualizado_em
                .and_then(|a| a.get(..10).map(|d| d >= iso(self.hoje).as_str()))
                .unwrap_or(false))
    }

    /// Os preços entram desde 10 dias antes da aplicação mais antiga, para a compra num fim de
    /// semana ou feriado achar o último PU anterior.
    fn inicio_tesouro(&self) -> NaiveDate {
        self.inicio
            .checked_sub_days(Days::new(FOLGA_TESOURO_DIAS))
            .unwrap_or(self.inicio)
    }

    /// Algum título da carteira não tem nenhum preço guardado (até hoje) e ainda não foi
    /// procurado no CSV de hoje. Um título que o CSV não conhece (vencimento digitado errado)
    /// fica `sem_dados`; a marca `tesouro:<tipo>:<vencimento>` diz que ele já foi procurado hoje.
    fn tesouro_incompleto(
        &self,
        titulos: &HashSet<(String, NaiveDate)>,
    ) -> Result<bool, CoreError> {
        for (tipo, vencimento) in titulos {
            if self
                .db
                .preco_tesouro_em(tipo, *vencimento, self.hoje)?
                .is_none()
                && !self.procurado_hoje(tipo, *vencimento)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn procurado_hoje(&self, tipo: &str, vencimento: NaiveDate) -> Result<bool, CoreError> {
        Ok(self
            .db
            .cobertura_desde(&chave_tesouro(tipo, vencimento))?
            .is_some_and(|d| d >= self.hoje))
    }

    /// Registra o status da fonte. Sucesso: `ultima_data` guardada e `atualizado_em` agora.
    /// Falha: mantém `ultima_data` e `atualizado_em` anteriores e grava só a mensagem.
    fn fechar(
        &self,
        fonte: &str,
        resultado: Result<usize, Falha>,
        mensagem: &str,
        ultima_data: impl FnOnce(&Self) -> Result<Option<String>, CoreError>,
    ) -> Result<usize, CoreError> {
        match resultado {
            Ok(novos) => {
                let anterior = self.status(fonte)?;
                self.db.registrar_status(&StatusFonte {
                    fonte: fonte.to_string(),
                    ultima_data: ultima_data(self)?.or(anterior.ultima_data),
                    atualizado_em: Some(Utc::now().to_rfc3339()),
                    erro: None,
                })?;
                Ok(novos)
            }
            Err(Falha::Fonte) => {
                self.falhou(fonte, mensagem)?;
                Ok(0)
            }
            Err(Falha::Banco(e)) => Err(e),
        }
    }

    fn falhou(&self, fonte: &str, mensagem: &str) -> Result<(), CoreError> {
        let anterior = self.status(fonte)?;
        self.db.registrar_status(&StatusFonte {
            erro: Some(mensagem.to_string()),
            ..anterior
        })
    }

    fn fechar_b3(
        &self,
        resultado: Result<(), Falha>,
        aviso: Option<&'static str>,
    ) -> Result<(), CoreError> {
        match (resultado, aviso) {
            (Ok(()), Some(msg)) => self.falhou("b3", msg),
            (resultado, _) => self
                .fechar("b3", resultado.map(|()| 0), MSG_B3, |c| {
                    Ok(c.db.datas_cotacao()?.last().copied().map(iso))
                })
                .map(|_| ()),
        }
    }

    /// Pontos do SGS entre `de` e `ate`, em janelas de no máximo 10 anos.
    fn sgs_em_janelas(
        &self,
        codigo: &str,
        de: NaiveDate,
        ate: NaiveDate,
        mut grava: impl FnMut(&[PontoDiario]) -> Result<usize, CoreError>,
    ) -> Result<usize, Falha> {
        let mut novos = 0;
        for (ini, fim) in janelas(de, ate) {
            let url = url_sgs(codigo, ini, fim);
            let pontos =
                match com_nova_tentativa_sgs(self.espera, || ler_sgs(&self.baixador.baixar(&url)?))
                {
                    Ok(p) => p,
                    // 404 do SGS: o intervalo não tem dados novos (fim de semana, mês futuro).
                    Err(ErroFonte::Status(404)) => Vec::new(),
                    Err(e) => return Err(e.into()),
                };
            novos += grava(&pontos)?;
        }
        Ok(novos)
    }

    /// CDI ou Selic: de `ultima + 1` até hoje e, se a carteira ganhou uma aplicação mais antiga
    /// que a cobertura, de `inicio` até o dia antes da primeira data guardada.
    fn indice_diario(&self, serie: &str) -> Result<usize, Falha> {
        let codigo = if serie == "cdi" { "12" } else { "11" };
        let ultima = self.db.ultima_data_indice(serie)?;
        let primeira = self.db.primeira_data_indice(serie)?;
        let desde = self.db.cobertura_desde(serie)?.or(primeira);
        let de = match ultima {
            Some(u) => u.succ_opt().unwrap_or(u),
            None => self.inicio,
        };
        let mut grava = |p: &[PontoDiario]| self.db.gravar_indice(serie, p);
        let mut novos = self.sgs_em_janelas(codigo, de, self.hoje, &mut grava)?;
        if let (Some(primeira), Some(desde)) = (primeira, desde)
            && self.inicio < desde
            && let Some(fim) = primeira.pred_opt()
        {
            novos += self.sgs_em_janelas(codigo, self.inicio, fim, &mut grava)?;
        }
        self.db.gravar_cobertura(serie, self.inicio)?;
        Ok(novos)
    }

    fn ipca(&self) -> Result<usize, Falha> {
        let primeiro_do_mes = |d: NaiveDate| d.with_day0(0).unwrap_or(d);
        let guardados = self.db.ipca_meses("0000-00", "9999-99")?;
        let mes_data = |m: &str| NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d").ok();
        let primeira = guardados.first().and_then(|(m, _)| mes_data(m));
        let desde = self.db.cobertura_desde("ipca")?.or(primeira);
        let de = match guardados.last().and_then(|(m, _)| mes_data(m)) {
            Some(d) => d.checked_add_months(Months::new(1)).unwrap_or(self.inicio),
            None => primeiro_do_mes(self.inicio),
        };
        let inicio = primeiro_do_mes(self.inicio);
        let mut grava = |p: &[PontoDiario]| {
            let meses: Vec<(String, Decimal)> = p
                .iter()
                .map(|p| (p.data.format("%Y-%m").to_string(), p.valor))
                .collect();
            self.db.gravar_ipca(&meses)
        };
        let mut novos = self.sgs_em_janelas("433", de, self.hoje, &mut grava)?;
        if let (Some(primeira), Some(desde)) = (primeira, desde)
            && inicio < desde
            && let Some(fim) = primeira.pred_opt()
        {
            novos += self.sgs_em_janelas("433", inicio, fim, &mut grava)?;
        }
        self.db.gravar_cobertura("ipca", inicio)?;
        Ok(novos)
    }

    fn tesouro(
        &self,
        titulos: &HashSet<(String, NaiveDate)>,
        ultima: &mut Option<NaiveDate>,
    ) -> Result<usize, Falha> {
        let url = url_tesouro();
        let todos = com_nova_tentativa(self.espera, || {
            super::ler_tesouro(&self.baixador.baixar(&url)?)
        })?;
        let da_carteira: Vec<_> = todos
            .into_iter()
            .filter(|p| {
                p.data >= self.inicio_tesouro()
                    && p.data <= self.hoje
                    && titulos.contains(&(p.tipo.clone(), p.vencimento))
            })
            .collect();
        *ultima = da_carteira.iter().map(|p| p.data).max();
        let novos = self.db.gravar_tesouro(&da_carteira)?;
        for (tipo, vencimento) in titulos {
            self.db
                .gravar_marca_recente(&chave_tesouro(tipo, *vencimento), self.hoje)?;
        }
        Ok(novos)
    }

    /// Último dia de cada um dos 12 meses anteriores a hoje.
    fn alvos_fim_de_mes(&self) -> Vec<NaiveDate> {
        let primeiro = self.hoje.with_day0(0).unwrap_or(self.hoje);
        (0..MESES_DE_HISTORICO)
            .filter_map(|k| {
                primeiro
                    .checked_sub_months(Months::new(k))
                    .and_then(|d| d.pred_opt())
            })
            .collect()
    }

    /// Anda para trás a partir de `inicio` (até 7 dias) e grava o primeiro pregão válido.
    /// `parar_em`: data a partir da qual já há cobertura (para sem baixar). Devolve o pregão.
    fn procurar_pregao(
        &self,
        inicio: NaiveDate,
        parar_em: Option<NaiveDate>,
        tickers: &HashSet<String>,
        novos: &mut usize,
    ) -> Result<Procura, Falha> {
        let mut dia = inicio;
        for _ in 0..TENTATIVAS_B3 {
            if parar_em.is_some_and(|p| dia <= p) {
                return Ok(Procura::JaCoberto);
            }
            if let Some(cotacoes) = self.pregao(dia)? {
                let escolhidas: Vec<_> = cotacoes
                    .into_iter()
                    .filter(|c| tickers.contains(&c.ticker))
                    .collect();
                *novos += self.db.gravar_cotacoes(&escolhidas)?;
                return Ok(Procura::Achou(dia));
            }
            match dia.pred_opt() {
                Some(anterior) => dia = anterior,
                None => break,
            }
        }
        Ok(Procura::Esgotou)
    }

    fn b3(
        &self,
        tickers: &HashSet<String>,
        novos: &mut usize,
        aviso: &mut Option<&'static str>,
    ) -> Result<(), Falha> {
        let mut desejados: Vec<String> = tickers.iter().cloned().collect();
        desejados.sort();
        let cobre = |a: &B3Alvo| desejados.iter().all(|t| a.tickers.contains(t));
        let guardados = self.db.b3_alvos()?;

        // Hoje: volta dia a dia até um arquivo válido ou até um pregão já coberto.
        let coberto_ate = guardados
            .iter()
            .filter(|a| cobre(a))
            .filter_map(|a| a.pregao)
            .max();
        match self.procurar_pregao(self.hoje, coberto_ate, tickers, novos)? {
            Procura::Achou(p) => self.db.gravar_b3_alvo(&B3Alvo {
                alvo: self.hoje,
                pregao: Some(p),
                tickers: desejados.clone(),
            })?,
            Procura::JaCoberto => {}
            Procura::Esgotou => *aviso = Some(MSG_B3_SEM_PREGAO),
        }

        // Fins de mês: só refaz o que falta ticker; alvo esgotado não é tentado de novo.
        for alvo in self.alvos_fim_de_mes() {
            let anterior = guardados.iter().find(|a| a.alvo == alvo);
            if anterior.is_some_and(cobre) {
                continue;
            }
            let ponto_de_partida = anterior.and_then(|a| a.pregao).unwrap_or(alvo);
            let pregao = match self.procurar_pregao(ponto_de_partida, None, tickers, novos)? {
                Procura::Achou(p) => Some(p),
                _ => None,
            };
            self.db.gravar_b3_alvo(&B3Alvo {
                alvo,
                pregao,
                tickers: desejados.clone(),
            })?;
        }
        Ok(())
    }

    /// `Some` com as cotações do dia; `None` se não houve pregão (status ≠ 200 ou ZIP inválido).
    /// Só falha de rede ganha nova tentativa; persistindo, a fonte falha.
    fn pregao(&self, dia: NaiveDate) -> Result<Option<Vec<super::Cotacao>>, Falha> {
        let url = url_b3(dia);
        let mut r = self.baixador.baixar(&url);
        if matches!(r, Err(ErroFonte::Rede(_))) {
            (self.espera)(ESPERA_NOVA_TENTATIVA);
            r = self.baixador.baixar(&url);
        }
        match r {
            Err(ErroFonte::Rede(_)) => Err(Falha::Fonte),
            Err(_) => Ok(None),
            Ok(corpo) => Ok(ler_cotahist_zip(&corpo).ok()),
        }
    }
}
