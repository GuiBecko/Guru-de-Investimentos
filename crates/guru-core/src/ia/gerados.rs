//! O que a IA gera para o painel da Carteira: os insights (saída estruturada, sem busca) e as
//! notícias (busca na web, montadas só a partir das fontes citadas). Cada geração substitui a
//! anterior por inteiro; um erro deixa a anterior como estava.

use std::sync::atomic::AtomicBool;

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::Classe;

use super::cliente::{ClienteIa, ErroIa, PEDIDO_NOTICIAS, corpo_insights, corpo_pedido};
use super::cofre::Cofre;
use super::conversa::{enviar_com_pausas, mes, somar_uso};
use super::sse::{Acumulador, Fonte, Progresso};

const MAX_INSIGHTS: usize = 5;
const MAX_TEXTO_INSIGHT: usize = 500;
const MAX_NOTICIAS: usize = 8;
const MAX_TITULO: usize = 100;
const ETIQUETA_MERCADO: &str = "Mercado";

const TIPOS: [(&str, &str); 5] = [
    ("rebalanceamento", "Rebalanceamento"),
    ("meta", "Meta"),
    ("vencimento", "Vencimento"),
    ("concentracao", "Concentração"),
    ("mercado", "Mercado"),
];
const ACOES: [&str; 5] = ["aporte", "meta", "classe", "conversar", "nenhuma"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Insight {
    pub tipo: String,
    pub texto: String,
    pub acao: String,
    /// Código da classe; obrigatório quando `acao` é `classe`.
    pub classe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noticia {
    pub titulo: String,
    /// Sempre `https://` e sempre a URL de uma fonte que a resposta citou.
    pub url: String,
    pub dominio: String,
    /// O `page_age` da fonte, como veio.
    pub data: Option<String>,
    /// Um ticker da carteira, o rótulo de uma classe ou "Mercado".
    pub etiqueta: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gerados<T> {
    pub gerado_em: NaiveDate,
    pub modelo: String,
    pub itens: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PainelIa {
    pub insights: Option<Gerados<Insight>>,
    pub noticias: Option<Gerados<Noticia>>,
}

/// O rótulo do tipo na tela. Um tipo desconhecido (que a validação já recusa) vira "Mercado".
pub fn rotulo_insight(tipo: &str) -> &'static str {
    TIPOS
        .iter()
        .find(|(c, _)| *c == tipo)
        .map_or(ETIQUETA_MERCADO, |(_, r)| r)
}

/// Valida o JSON da resposta contra o esquema e as regras extras; fica com os 5 primeiros.
pub fn ler_insights(json: &str, stop_reason: Option<&str>) -> Result<Vec<Insight>, ErroIa> {
    let invalido = ErroIa::RespostaInvalida;
    if stop_reason == Some("max_tokens") {
        return Err(invalido);
    }
    let v: Value = serde_json::from_str(json).map_err(|_| invalido.clone())?;
    let lista = v["insights"].as_array().ok_or(invalido.clone())?;
    if lista.is_empty() {
        return Err(invalido);
    }
    let mut out = Vec::new();
    for item in lista {
        let o = item.as_object().ok_or(invalido.clone())?;
        let texto_de = |campo: &str| o.get(campo).and_then(Value::as_str);
        let (Some(tipo), Some(texto), Some(acao), Some(classe)) = (
            texto_de("tipo"),
            texto_de("texto"),
            texto_de("acao"),
            o.get("classe"),
        ) else {
            return Err(invalido);
        };
        if !TIPOS.iter().any(|(c, _)| *c == tipo) || !ACOES.contains(&acao) {
            return Err(invalido);
        }
        let classe = match classe {
            Value::Null => None,
            Value::String(c) if Classe::de_codigo(c).is_some() => Some(c.clone()),
            _ => return Err(invalido),
        };
        let texto = texto.trim();
        if texto.is_empty() || texto.chars().count() > MAX_TEXTO_INSIGHT {
            return Err(invalido);
        }
        if acao == "classe" && classe.is_none() {
            return Err(invalido);
        }
        out.push(Insight {
            tipo: tipo.to_string(),
            texto: texto.to_string(),
            acao: acao.to_string(),
            classe,
        });
    }
    out.truncate(MAX_INSIGHTS);
    Ok(out)
}

/// A URL da primeira citação da web de um bloco de texto.
fn primeira_url(bloco: &Value) -> Option<&str> {
    bloco["citations"].as_array()?.iter().find_map(|c| {
        (c["type"] == "web_search_result_location")
            .then(|| c["url"].as_str())
            .flatten()
    })
}

/// `- <ativo> | <título>` (o `^-\s*(.+?)\s*\|\s*(.+)$` das Global Constraints) em (ativo, título).
fn partes_da_linha(linha: &str) -> Option<(&str, &str)> {
    let resto = linha.trim().strip_prefix('-')?;
    let (ativo, titulo) = resto.split_once('|')?;
    let (ativo, titulo) = (ativo.trim(), titulo.trim());
    (!titulo.is_empty()).then_some((ativo, titulo))
}

/// Tira os marcadores `[n]` que o modelo tenha escrito.
fn sem_marcadores(texto: &str) -> String {
    let mut out = String::new();
    let mut resto = texto;
    while let Some(i) = resto.find('[') {
        let depois = &resto[i + 1..];
        let digitos = depois.chars().take_while(char::is_ascii_digit).count();
        if digitos > 0 && depois[digitos..].starts_with(']') {
            out.push_str(&resto[..i]);
            resto = &depois[digitos + 1..];
        } else {
            out.push_str(&resto[..=i]);
            resto = depois;
        }
    }
    out.push_str(resto);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Monta as notícias das linhas da resposta. Só vira notícia a linha no formato que citou uma
/// fonte `https://`; link, domínio e data vêm da `Fonte`, nunca do texto do modelo.
pub fn ler_noticias(blocos: &[Value], fontes: &[Fonte], etiquetas: &[String]) -> Vec<Noticia> {
    // Cada linha: o texto e a URL da primeira citação dos blocos que terminam nela.
    let mut linhas: Vec<(String, Option<String>)> = vec![(String::new(), None)];
    for b in blocos.iter().filter(|b| b["type"] == "text") {
        let texto = b["text"].as_str().unwrap_or("");
        let primeira = linhas.len() - 1;
        for (i, pedaco) in texto.split('\n').enumerate() {
            if i > 0 {
                linhas.push((String::new(), None));
            }
            if let Some(l) = linhas.last_mut() {
                l.0.push_str(pedaco);
            }
        }
        // Quebras no fim do bloco não mudam a linha em que ele termina.
        let fim = primeira + texto.trim_end_matches('\n').matches('\n').count();
        if let Some(url) = primeira_url(b)
            && linhas[fim].1.is_none()
        {
            linhas[fim].1 = Some(url.to_string());
        }
    }
    let mut out: Vec<Noticia> = Vec::new();
    for (linha, url) in linhas {
        if out.len() == MAX_NOTICIAS {
            break;
        }
        let Some(url) = url.filter(|u| u.starts_with("https://")) else {
            continue;
        };
        let Some((ativo, titulo)) = partes_da_linha(&linha) else {
            continue;
        };
        if out.iter().any(|n| n.url == url) {
            continue;
        }
        let Some(fonte) = fontes.iter().find(|f| f.url == url) else {
            continue;
        };
        let titulo: String = sem_marcadores(titulo).chars().take(MAX_TITULO).collect();
        let titulo = titulo.trim_end().to_string();
        if titulo.is_empty() {
            continue;
        }
        let etiqueta = etiquetas
            .iter()
            .find(|e| e.to_lowercase() == ativo.to_lowercase())
            .cloned()
            .unwrap_or_else(|| ETIQUETA_MERCADO.to_string());
        out.push(Noticia {
            titulo,
            url: fonte.url.clone(),
            dominio: fonte.dominio.clone(),
            data: fonte.data.clone(),
            etiqueta,
        });
    }
    out
}

impl Db {
    fn gerado<T: DeserializeOwned>(&self, tipo: &str) -> Result<Option<Gerados<T>>, CoreError> {
        let linha: Option<(String, String, String)> = self
            .conn
            .query_row(
                "SELECT gerado_em, modelo, itens_json FROM ia_gerado WHERE tipo = ?1",
                [tipo],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(classifica)?;
        let Some((gerado_em, modelo, itens)) = linha else {
            return Ok(None);
        };
        let gerado_em = NaiveDate::parse_from_str(&gerado_em, "%Y-%m-%d")
            .map_err(|_| CoreError::DbInvalid("data de geração ilegível".into()))?;
        let itens = serde_json::from_str(&itens)
            .map_err(|_| CoreError::DbInvalid("itens gerados ilegíveis".into()))?;
        Ok(Some(Gerados {
            gerado_em,
            modelo,
            itens,
        }))
    }

    /// Os últimos insights e notícias gravados, cada um com a data e o modelo.
    pub fn painel_ia(&self) -> Result<PainelIa, CoreError> {
        Ok(PainelIa {
            insights: self.gerado("insights")?,
            noticias: self.gerado("noticias")?,
        })
    }

    pub(crate) fn noticias_guardadas(&self) -> Result<Option<Gerados<Noticia>>, CoreError> {
        self.gerado("noticias")
    }

    /// Substitui a geração do tipo e soma o uso, numa transação só.
    fn gravar_gerado<T: Serialize>(
        &self,
        tipo: &str,
        hoje: NaiveDate,
        modelo: &str,
        itens: &[T],
        acumulador: &Acumulador,
    ) -> Result<(), CoreError> {
        let itens = serde_json::to_string(itens)
            .map_err(|_| CoreError::DbInvalid("itens gerados ilegíveis".into()))?;
        let tx = self.conn.unchecked_transaction().map_err(classifica)?;
        tx.execute(
            "INSERT INTO ia_gerado (tipo, gerado_em, modelo, itens_json) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (tipo) DO UPDATE SET gerado_em = excluded.gerado_em, \
             modelo = excluded.modelo, itens_json = excluded.itens_json",
            params![tipo, hoje.format("%Y-%m-%d").to_string(), modelo, itens],
        )
        .map_err(classifica)?;
        somar_uso(&tx, &mes(hoje), modelo, acumulador.uso())?;
        tx.commit().map_err(classifica)?;
        Ok(())
    }

    /// Os tickers da carteira e os rótulos das cinco classes.
    fn etiquetas_noticias(&self, hoje: NaiveDate) -> Result<Vec<String>, CoreError> {
        let mut etiquetas: Vec<String> = Vec::new();
        for i in self.listar_investimentos(hoje)? {
            if let Some(t) = i.ticker
                && !etiquetas.contains(&t)
            {
                etiquetas.push(t);
            }
        }
        etiquetas.extend(Classe::TODOS.iter().map(|c| c.rotulo().to_string()));
        Ok(etiquetas)
    }
}

fn ler_chave(cofre: &dyn Cofre) -> Result<String, CoreError> {
    Ok(cofre
        .ler()?
        .filter(|c| !c.is_empty())
        .ok_or(ErroIa::SemChave)?)
}

/// O corpo de cada parte: o base com as mensagens da vez (a continuação de um `pause_turn`).
fn com_mensagens(base: &Value, pedido: &[Value]) -> Value {
    let mut corpo = base.clone();
    corpo["messages"] = json!(pedido);
    corpo
}

/// Envia, valida e grava (ia_gerado + uso numa transação). Erro: nada muda no ia_gerado. Uma
/// resposta fora do esquema já foi paga: os tokens dela entram no uso.
pub fn gerar_insights(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    hoje: NaiveDate,
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<(), CoreError> {
    let chave = ler_chave(cofre)?;
    let modelo = db.config_ia()?.modelo;
    let contexto = db.contexto_montado(hoje)?.texto;
    let (base, _) = corpo_insights(&modelo, &contexto);
    let mensagens = base["messages"].as_array().cloned().unwrap_or_default();
    let corpo = |pedido: &[Value]| com_mensagens(&base, pedido);
    let acumulador = enviar_com_pausas(
        db, cliente, &chave, hoje, &modelo, &corpo, &mensagens, cancelar, progresso,
    )?;
    let json: String = acumulador
        .blocos()
        .iter()
        .filter(|b| b["type"] == "text")
        .filter_map(|b| b["text"].as_str())
        .collect();
    let itens = match ler_insights(&json, acumulador.stop_reason()) {
        Ok(itens) => itens,
        Err(erro) => {
            // Melhor esforço: o erro que importa é o da resposta.
            let _ = somar_uso(&db.conn, &mes(hoje), &modelo, acumulador.uso());
            return Err(erro.into());
        }
    };
    db.gravar_gerado("insights", hoje, &modelo, &itens, &acumulador)
}

/// Busca as notícias com a busca na web e grava a lista (vazia também) com a data. Com
/// "Buscar na web" desligada, nada é enviado. Erro: nada muda no ia_gerado; uma resposta
/// parada por `refusal` (IA_RECUSA), `max_tokens` ou ainda em `pause_turn` depois das
/// continuações (IA_INDISPONIVEL) também é erro, com os tokens somados no uso.
pub fn buscar_noticias(
    db: &Db,
    cofre: &dyn Cofre,
    cliente: &dyn ClienteIa,
    hoje: NaiveDate,
    cancelar: &AtomicBool,
    progresso: &mut dyn FnMut(Progresso),
) -> Result<(), CoreError> {
    let chave = ler_chave(cofre)?;
    let config = db.config_ia()?;
    if !config.busca_web {
        return Err(ErroIa::BuscaDesligada.into());
    }
    let contexto = db.contexto_noticias(hoje)?;
    let mensagens = [json!({"role": "user", "content": PEDIDO_NOTICIAS})];
    let corpo = |pedido: &[Value]| corpo_pedido(&config.modelo, &contexto, true, pedido).0;
    let acumulador = enviar_com_pausas(
        db,
        cliente,
        &chave,
        hoje,
        &config.modelo,
        &corpo,
        &mensagens,
        cancelar,
        progresso,
    )?;
    // Uma resposta que não terminou bem daria uma lista parcial: as notícias guardadas ficam.
    let interrompida = match acumulador.stop_reason() {
        Some("refusal") => Some(ErroIa::Recusa),
        Some("max_tokens" | "pause_turn") => Some(ErroIa::Indisponivel),
        _ => None,
    };
    if let Some(erro) = interrompida {
        // Melhor esforço: o erro que importa é o da resposta.
        let _ = somar_uso(&db.conn, &mes(hoje), &config.modelo, acumulador.uso());
        return Err(erro.into());
    }
    let etiquetas = db.etiquetas_noticias(hoje)?;
    let itens = ler_noticias(
        acumulador.blocos(),
        &acumulador.progresso().fontes,
        &etiquetas,
    );
    db.gravar_gerado("noticias", hoje, &config.modelo, &itens, &acumulador)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn marcadores_saem_do_titulo() {
        assert_eq!(sem_marcadores("Selic sobe [1] [23]"), "Selic sobe");
        assert_eq!(sem_marcadores("[1]Copom"), "Copom");
        assert_eq!(sem_marcadores("Índice [a] e [ ]"), "Índice [a] e [ ]");
        assert_eq!(sem_marcadores("fim ["), "fim [");
    }

    #[test]
    fn linha_no_formato() {
        assert_eq!(
            partes_da_linha("- ITUB4 | Título"),
            Some(("ITUB4", "Título"))
        );
        assert_eq!(
            partes_da_linha("-ITUB4|Título | com barra"),
            Some(("ITUB4", "Título | com barra"))
        );
        assert_eq!(partes_da_linha("ITUB4 | Título"), None);
        assert_eq!(partes_da_linha("- ITUB4 |   "), None);
        assert_eq!(partes_da_linha("- | Título"), Some(("", "Título")));
    }
}
