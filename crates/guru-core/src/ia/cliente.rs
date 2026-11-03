//! Cliente da Messages API da Anthropic em HTTP puro (Rust não tem SDK oficial), com streaming SSE.
//! A chave nunca entra em mensagem de erro nem em valor devolvido.

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::sse::LeitorSse;

const URL_MENSAGENS: &str = "https://api.anthropic.com/v1/messages";
const URL_MODELOS: &str = "https://api.anthropic.com/v1/models?limit=1";
const VERSAO_API: &str = "2026-10-21";
const BETA_FALLBACK: &str = "server-side-fallback-2026-07-01";
/// Prazo para resolver, conectar e enviar o pedido.
const CONECTAR: Duration = Duration::from_secs(60);
/// Prazo para a resposta inteira, do DNS ao último byte do corpo.
const RESPOSTA: Duration = Duration::from_secs(600);
/// Prazo total para validar a chave (`GET /v1/models?limit=1`), que não é streaming.
const VALIDAR: Duration = Duration::from_secs(30);
const LIMITE_RESPOSTA: u64 = 8 * 1024 * 1024;
const LIMITE_CORPO_ERRO: u64 = 64 * 1024;
const MAX_MENSAGEM_ERRO: usize = 500;
const MAX_TOKENS: u32 = 32_000;
/// Intervalo entre os eventos do `ClienteFixture`, para a tela ver o texto chegando.
const INTERVALO_FIXTURE: Duration = Duration::from_millis(20);

/// Fixo e sem data, para o cache do prompt.
const SISTEMA: &str = "Você é o assistente do Guru de Investimentos, um app brasileiro de carteira pessoal. Responda em português do Brasil, com clareza e sem jargão desnecessário. Apresente opções, com prós e contras, e deixe a decisão com o usuário: não faça recomendação individual de investimento nem diga o que ele deve comprar ou vender. Use o contexto enviado sobre a carteira, o perfil, as metas e o mercado. Para fatos de mercado recentes (decisões do Copom, projeções, notícias), busque na web quando a busca estiver disponível e cite a fonte. Quando algo for opinião sua, sem fonte, diga que é opinião. Se faltar um dado para responder bem, diga qual.";

/// O pedido fixo dos insights, a única mensagem do usuário.
pub(crate) const PEDIDO_INSIGHTS: &str = "Analise o contexto e escreva de 3 a 5 insights curtos sobre a carteira, em português do Brasil, cada um com no máximo 300 caracteres. Use os tipos rebalanceamento (desvio da alocação-alvo), meta (ritmo ou atraso das metas), vencimento (o que vence nos próximos 12 meses), concentracao (peso alto num ativo, setor ou instituição) e mercado (o que os dados de mercado e as notícias do contexto significam para a carteira). Para cada um, escolha a ação que leva ao lugar certo do app: aporte, meta, classe (com o código da classe), conversar ou nenhuma. Apresente fatos e opções, sem recomendar a compra ou a venda de um ativo específico. Use só os dados do contexto.";

/// O pedido fixo das notícias, a única mensagem do usuário.
pub(crate) const PEDIDO_NOTICIAS: &str = "Busque na web notícias dos últimos 7 dias sobre os ativos listados no contexto e sobre os indicadores que os afetam (Selic, IPCA, câmbio). Escolha até 8 notícias relevantes, de veículos jornalísticos ou fontes oficiais. Responda só com uma linha por notícia, no formato `- <ativo ou classe> | <título em português, com até 100 caracteres>`, citando a matéria em cada linha. Não escreva mais nada.";

/// O JSON Schema da saída estruturada dos insights.
fn schema_insights() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["insights"],
        "properties": {"insights": {"type": "array", "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["tipo", "texto", "acao", "classe"],
            "properties": {
                "tipo": {"type": "string", "enum":
                    ["rebalanceamento", "meta", "vencimento", "concentracao", "mercado"]},
                "texto": {"type": "string"},
                "acao": {"type": "string", "enum":
                    ["aporte", "meta", "classe", "conversar", "nenhuma"]},
                "classe": {"anyOf": [
                    {"type": "string", "enum": ["rf", "acoes", "fii", "ext", "outros"]},
                    {"type": "null"}
                ]}
            }
        }}}
    })
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErroIa {
    #[error("Configure a sua chave em Configurar IA.")]
    SemChave,
    #[error("A Anthropic recusou a sua chave. Confira em Configurar IA.")]
    ChaveRecusada,
    #[error("Limite de uso da sua conta Anthropic atingido. Tente de novo em instantes.")]
    Limite,
    #[error("A Anthropic está indisponível agora. Tente de novo em instantes.")]
    Indisponivel,
    #[error("A Anthropic recusou o pedido: {0}")]
    PedidoInvalido(String),
    #[error("Sem conexão com a Anthropic.")]
    SemConexao,
    #[error("Resposta interrompida.")]
    Cancelada,
    /// Recusa sem nenhum bloco de conteúdo: não há o que gravar.
    #[error("A IA preferiu não responder a esta pergunta.")]
    Recusa,
    #[error("A IA devolveu os insights num formato inesperado. Tente de novo.")]
    RespostaInvalida,
    #[error("Ligue “Buscar na web” em Configurar IA para buscar notícias.")]
    BuscaDesligada,
}

impl ErroIa {
    pub fn code(&self) -> &'static str {
        match self {
            ErroIa::SemChave => "IA_SEM_CHAVE",
            ErroIa::ChaveRecusada => "IA_CHAVE_RECUSADA",
            ErroIa::Limite => "IA_LIMITE",
            ErroIa::Indisponivel => "IA_INDISPONIVEL",
            ErroIa::PedidoInvalido(_) => "IA_PEDIDO_INVALIDO",
            ErroIa::SemConexao => "IA_SEM_CONEXAO",
            ErroIa::Cancelada => "IA_CANCELADA",
            ErroIa::Recusa => "IA_RECUSA",
            ErroIa::RespostaInvalida => "IA_RESPOSTA_INVALIDA",
            ErroIa::BuscaDesligada => "IA_BUSCA_DESLIGADA",
        }
    }
}

pub trait ClienteIa: Send + Sync {
    fn validar_chave(&self, chave: &str) -> Result<(), ErroIa>;
    /// Envia o corpo e entrega cada linha `data:` já parseada ao `ao_evento`; para quando `cancelar` vira true.
    fn enviar(
        &self,
        chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa>;
}

fn eh_haiku(modelo: &str) -> bool {
    modelo.contains("haiku")
}

/// O corpo exato do pedido e o cabeçalho `anthropic-beta`, quando houver.
/// Sem `temperature`, `top_p`, `top_k` nem `thinking` (a omissão é o pensamento adaptativo).
pub fn corpo_pedido(
    modelo: &str,
    contexto: &str,
    busca_web: bool,
    mensagens: &[Value],
) -> (Value, Option<&'static str>) {
    let mut corpo = json!({
        "model": modelo,
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "system": [
            {"type": "text", "text": SISTEMA},
            {"type": "text", "text": contexto, "cache_control": {"type": "ephemeral"}}
        ],
        "messages": mensagens,
        "output_config": {"effort": "medium"},
    });
    if busca_web {
        let tipo = if eh_haiku(modelo) {
            "web_search_20250305"
        } else {
            "web_search_20260209"
        };
        corpo["tools"] = json!([{
            "type": tipo,
            "name": "web_search",
            "max_uses": 5,
            "user_location": {
                "type": "approximate",
                "country": "BR",
                "timezone": "America/Sao_Paulo"
            }
        }]);
    }
    if eh_haiku(modelo) {
        (corpo, None)
    } else {
        corpo["fallbacks"] = json!("default");
        (corpo, Some(BETA_FALLBACK))
    }
}

/// O corpo dos insights: o da 6a sem busca, com `PEDIDO_INSIGHTS` como única mensagem e a saída
/// estruturada. Sem `tools` mesmo com "Buscar na web" ligada: a API recusa saída estruturada
/// junto com citações, e a busca sempre cita.
pub fn corpo_insights(modelo: &str, contexto: &str) -> (Value, Option<&'static str>) {
    let mensagens = [json!({"role": "user", "content": PEDIDO_INSIGHTS})];
    let (mut corpo, beta) = corpo_pedido(modelo, contexto, false, &mensagens);
    corpo["output_config"]["format"] = json!({"type": "json_schema", "schema": schema_insights()});
    (corpo, beta)
}

/// Erro HTTP fora de 2xx, a partir do status e do corpo `{"type":"error","error":{...}}`.
fn erro_de_status(status: u16, corpo: &str, chave: &str) -> ErroIa {
    match status {
        401 | 403 => ErroIa::ChaveRecusada,
        429 => ErroIa::Limite,
        400..=499 => ErroIa::PedidoInvalido(mensagem_de_erro(corpo, chave)),
        _ => ErroIa::Indisponivel,
    }
}

/// A `message` do corpo de erro, sem a chave e com tamanho limitado.
fn mensagem_de_erro(corpo: &str, chave: &str) -> String {
    let mensagem = serde_json::from_str::<Value>(corpo)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "sem detalhes.".to_string());
    let mensagem = if chave.is_empty() {
        mensagem
    } else {
        mensagem.replace(chave, "…")
    };
    mensagem
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_MENSAGEM_ERRO)
        .collect()
}

/// Cliente real. Só fala com as duas URLs exatas da Anthropic.
pub struct ClienteHttp {
    agente: ureq::Agent,
}

impl ClienteHttp {
    pub fn novo() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(RESPOSTA))
            .timeout_resolve(Some(CONECTAR))
            .timeout_connect(Some(CONECTAR))
            .timeout_send_request(Some(CONECTAR))
            .user_agent(format!("GuruDeInvestimentos/{}", env!("CARGO_PKG_VERSION")))
            .http_status_as_error(false)
            .https_only(true)
            // Um redirecionamento poderia levar a chave para fora da lista de destinos.
            .max_redirects(0)
            .max_redirects_will_error(false)
            .build();
        Self {
            agente: ureq::Agent::new_with_config(config),
        }
    }

    fn permitido(url: &str) -> bool {
        url == URL_MENSAGENS || url == URL_MODELOS
    }

    fn ler_erro(resposta: &mut ureq::http::Response<ureq::Body>, chave: &str) -> ErroIa {
        let status = resposta.status().as_u16();
        let mut corpo = Vec::new();
        let _ = resposta
            .body_mut()
            .as_reader()
            .take(LIMITE_CORPO_ERRO)
            .read_to_end(&mut corpo);
        erro_de_status(status, &String::from_utf8_lossy(&corpo), chave)
    }

    fn validar_em(&self, url: &str, chave: &str) -> Result<(), ErroIa> {
        if !Self::permitido(url) {
            return Err(ErroIa::SemConexao);
        }
        let mut resposta = self
            .agente
            .get(url)
            .config()
            .timeout_global(Some(VALIDAR))
            .build()
            .header("x-api-key", chave)
            .header("anthropic-version", VERSAO_API)
            .header("content-type", "application/json")
            .call()
            .map_err(|_| ErroIa::SemConexao)?;
        if resposta.status().is_success() {
            Ok(())
        } else {
            Err(Self::ler_erro(&mut resposta, chave))
        }
    }

    fn enviar_em(
        &self,
        url: &str,
        chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa> {
        if !Self::permitido(url) {
            return Err(ErroIa::SemConexao);
        }
        if cancelar.load(Ordering::SeqCst) {
            return Err(ErroIa::Cancelada);
        }
        let bytes = serde_json::to_vec(corpo).map_err(|_| ErroIa::Indisponivel)?;
        let mut pedido = self
            .agente
            .post(url)
            .header("x-api-key", chave)
            .header("anthropic-version", VERSAO_API)
            .header("content-type", "application/json");
        if corpo.get("fallbacks").is_some() {
            pedido = pedido.header("anthropic-beta", BETA_FALLBACK);
        }
        let mut resposta = pedido.send(&bytes[..]).map_err(|_| ErroIa::SemConexao)?;
        if !resposta.status().is_success() {
            return Err(Self::ler_erro(&mut resposta, chave));
        }
        let mut leitor = resposta.body_mut().as_reader();
        let mut sse = LeitorSse::default();
        let mut eventos = Vec::new();
        let mut pedaco = [0u8; 8192];
        let mut total: u64 = 0;
        loop {
            if cancelar.load(Ordering::SeqCst) {
                return Err(ErroIa::Cancelada);
            }
            let n = leitor.read(&mut pedaco).map_err(|_| ErroIa::SemConexao)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > LIMITE_RESPOSTA {
                return Err(ErroIa::Indisponivel);
            }
            let lido = sse.alimentar(&pedaco[..n], &mut eventos);
            for e in eventos.drain(..) {
                ao_evento(&e);
            }
            lido?;
        }
        let lido = sse.terminar(&mut eventos);
        for e in eventos.drain(..) {
            ao_evento(&e);
        }
        lido
    }
}

impl Default for ClienteHttp {
    fn default() -> Self {
        Self::novo()
    }
}

impl ClienteIa for ClienteHttp {
    fn validar_chave(&self, chave: &str) -> Result<(), ErroIa> {
        self.validar_em(URL_MODELOS, chave)
    }

    fn enviar(
        &self,
        chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa> {
        self.enviar_em(URL_MENSAGENS, chave, corpo, cancelar, ao_evento)
    }
}

/// Modo de teste: respostas gravadas em `.sse`. Os insights (corpo com `output_config.format`) e
/// as notícias (última pergunta = `PEDIDO_NOTICIAS`) escolhem pelas marcas no `system[1]`, que
/// chegam pelas instruções do usuário; as conversas, pela última pergunta do usuário.
pub struct ClienteFixture {
    pub pasta: PathBuf,
}

/// O texto da última mensagem do usuário (as perguntas vão como texto).
fn ultima_pergunta(corpo: &Value) -> &str {
    corpo["messages"]
        .as_array()
        .and_then(|m| {
            m.iter()
                .rev()
                .find(|m| m["role"] == "user")
                .and_then(|m| m["content"].as_str())
        })
        .unwrap_or("")
}

impl ClienteIa for ClienteFixture {
    fn validar_chave(&self, chave: &str) -> Result<(), ErroIa> {
        if chave.contains("recusada") {
            Err(ErroIa::ChaveRecusada)
        } else {
            Ok(())
        }
    }

    fn enviar(
        &self,
        _chave: &str,
        corpo: &Value,
        cancelar: &AtomicBool,
        ao_evento: &mut dyn FnMut(&Value),
    ) -> Result<(), ErroIa> {
        let pergunta = ultima_pergunta(corpo);
        let sistema = corpo["system"][1]["text"].as_str().unwrap_or("");
        let gerado = if corpo["output_config"]["format"].is_object() {
            Some(if sistema.contains("#insights-invalido") {
                "insights_invalido.sse"
            } else {
                "insights.sse"
            })
        } else if pergunta == PEDIDO_NOTICIAS {
            Some(if sistema.contains("#noticias-vazias") {
                "noticias_vazias.sse"
            } else if sistema.contains("#noticias-recusa") {
                "noticias_recusa.sse"
            } else if sistema.contains("#noticias-max-tokens") {
                "noticias_max_tokens.sse"
            } else if sistema.contains("#noticias-pausa") {
                "noticias_pausa.sse"
            } else {
                "noticias.sse"
            })
        } else {
            None
        };
        if gerado.is_some() && sistema.contains("#erro401") {
            return Err(ErroIa::ChaveRecusada);
        }
        if gerado.is_some() && sistema.contains("#erro529") {
            return Err(ErroIa::Indisponivel);
        }
        let continuacao = corpo["messages"]
            .as_array()
            .and_then(|m| m.last())
            .is_some_and(|m| m["role"] == "assistant");
        let arquivo = if let Some(arquivo) = gerado {
            arquivo
        } else if pergunta.contains("#erro401") {
            return Err(ErroIa::ChaveRecusada);
        } else if pergunta.contains("#erro529") {
            return Err(ErroIa::Indisponivel);
        } else if pergunta.contains("#recusavazia") {
            "recusa_vazia.sse"
        } else if pergunta.contains("#pausavazia") {
            "pausa_vazia.sse"
        } else if pergunta.contains("#vazia") {
            "vazia.sse"
        } else if pergunta.contains("#busca") {
            "busca.sse"
        } else if pergunta.contains("#pause") && !continuacao {
            "pause.sse"
        } else if pergunta.contains("#recusa") {
            "recusa.sse"
        } else if pergunta.contains("#quebrado") {
            "quebrado.sse"
        } else {
            "simples.sse"
        };
        let bytes = std::fs::read(self.pasta.join(arquivo)).map_err(|_| ErroIa::SemConexao)?;
        let mut sse = LeitorSse::default();
        let mut eventos = Vec::new();
        let lido = sse
            .alimentar(&bytes, &mut eventos)
            .and_then(|()| sse.terminar(&mut eventos));
        for (i, e) in eventos.iter().enumerate() {
            if i > 0 {
                std::thread::sleep(INTERVALO_FIXTURE);
            }
            if cancelar.load(Ordering::SeqCst) {
                return Err(ErroIa::Cancelada);
            }
            ao_evento(e);
        }
        lido
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn mapeia_os_status_da_tabela_de_erros() {
        const K: &str = "sk-ant-api03-chave-de-teste";
        let corpo = r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens: muito grande"}}"#;
        assert_eq!(erro_de_status(401, corpo, K), ErroIa::ChaveRecusada);
        assert_eq!(erro_de_status(403, corpo, K), ErroIa::ChaveRecusada);
        assert_eq!(erro_de_status(429, corpo, K), ErroIa::Limite);
        assert_eq!(erro_de_status(500, corpo, K), ErroIa::Indisponivel);
        assert_eq!(erro_de_status(529, corpo, K), ErroIa::Indisponivel);
        let e = erro_de_status(400, corpo, K);
        assert_eq!(e.code(), "IA_PEDIDO_INVALIDO");
        assert_eq!(
            e.to_string(),
            "A Anthropic recusou o pedido: max_tokens: muito grande"
        );
        assert_eq!(
            erro_de_status(413, "<html>", K).to_string(),
            "A Anthropic recusou o pedido: sem detalhes."
        );
    }

    #[test]
    fn mensagem_de_erro_nunca_repete_a_chave() {
        let chave = "sk-ant-api03-segredo-1234";
        let corpo = format!(r#"{{"type":"error","error":{{"message":"chave {chave} inválida"}}}}"#);
        let e = erro_de_status(400, &corpo, chave).to_string();
        assert!(!e.contains(chave));
        assert_eq!(e, "A Anthropic recusou o pedido: chave … inválida");
    }

    #[test]
    fn so_as_duas_urls_exatas_sao_permitidas() {
        assert!(ClienteHttp::permitido(
            "https://api.anthropic.com/v1/messages"
        ));
        assert!(ClienteHttp::permitido(
            "https://api.anthropic.com/v1/models?limit=1"
        ));
        for ruim in [
            "http://api.anthropic.com/v1/messages",
            "https://api.anthropic.com/v1/messages?beta=true",
            "https://api.anthropic.com/v1/models",
            "https://api.anthropic.com.evil.com/v1/messages",
            "https://exemplo.com/v1/messages",
            "",
        ] {
            assert!(!ClienteHttp::permitido(ruim), "deveria recusar: {ruim}");
        }
    }

    #[test]
    fn cliente_http_recusa_destino_fora_da_lista_sem_rede() {
        let c = ClienteHttp::novo();
        assert_eq!(
            c.validar_em("https://exemplo.com/", "sk-ant-x"),
            Err(ErroIa::SemConexao)
        );
        let r = c.enviar_em(
            "https://exemplo.com/",
            "sk-ant-x",
            &json!({}),
            &AtomicBool::new(false),
            &mut |_| panic!("não deveria receber evento"),
        );
        assert_eq!(r, Err(ErroIa::SemConexao));
    }

    #[test]
    fn prazos_do_cliente_http() {
        let c = ClienteHttp::novo();
        let t = c.agente.config().timeouts();
        assert_eq!(t.global, Some(Duration::from_secs(600)));
        assert_eq!(t.connect, Some(Duration::from_secs(60)));
        assert_eq!(t.resolve, Some(Duration::from_secs(60)));
        assert_eq!(c.agente.config().max_redirects(), 0);
        assert!(c.agente.config().https_only());
    }
}
