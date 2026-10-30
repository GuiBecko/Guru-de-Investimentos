use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use guru_core::{CORE_VERSION, CoreError, Db, Gravacao};
use napi::bindgen_prelude::{AsyncTask, Buffer};
use napi_derive::napi;

mod importacao;
mod investimentos;
mod mercado;
mod metas;
mod painel;
mod perfil;
use importacao::{ComparacaoGuardada, ComparacaoJs, DecisaoJs, ResultadoImportacaoJs};
use investimentos::{
    Catalogo, ErroCampoJs, InvestimentoJs, NovoInvestimentoJs, ResultadoGravacao, ResumoJs,
    entrada, erros,
};
use mercado::{AtualizarMercado, ModoTeste, OpcoesCore, StatusFonteJs};
use metas::{MetaJs, NovaMetaJs};
use painel::CalcularPainel;
use perfil::{AlocacaoClasseJs, PercentualClasseJs, PerfilJs, QuestionarioJs, ResultadoPerfilJs};

#[napi(object)]
pub struct AppInfo {
    pub core_version: String,
    pub schema_version: u32,
    pub db_path: String,
}

#[napi]
pub struct Core {
    db: Mutex<Db>,
    pasta_dados: PathBuf,
    /// Só existe nos testes (ver `OpcoesCore`).
    modo_teste: Option<ModoTeste>,
    /// Última comparação de importação; só ela pode ser aplicada.
    importacao: Mutex<Option<ComparacaoGuardada>>,
}

fn to_js(e: CoreError) -> napi::Error<String> {
    napi::Error::new(e.code().to_string(), e.to_string())
}

fn erro_interno() -> napi::Error<String> {
    napi::Error::new(
        "ERRO_INTERNO".to_string(),
        "Ocorreu um erro interno ao processar a planilha.".to_string(),
    )
}

#[napi]
impl Core {
    #[napi(factory)]
    pub fn open(data_dir: String, opcoes: Option<OpcoesCore>) -> napi::Result<Core, String> {
        let modo_teste = mercado::modo_teste(opcoes)?;
        let db = Db::open(Path::new(&data_dir)).map_err(to_js)?;
        Ok(Core {
            db: Mutex::new(db),
            pasta_dados: PathBuf::from(data_dir),
            modo_teste,
            importacao: Mutex::new(None),
        })
    }

    /// Data de mercado: a fixa recebida em `Core.open` nos testes, senão a real. Vale só para o
    /// atualizador e a valorização (leituras); validações e gravações usam `hoje_real`.
    fn data_mercado(&self) -> chrono::NaiveDate {
        self.modo_teste
            .as_ref()
            .and_then(|m| m.hoje)
            .unwrap_or_else(hoje_real)
    }

    fn db(&self) -> napi::Result<MutexGuard<'_, Db>, String> {
        self.db.lock().map_err(|_| {
            napi::Error::new(
                "LOCK_POISONED".to_string(),
                "Estado interno do núcleo corrompido.".to_string(),
            )
        })
    }

    fn importacao(&self) -> napi::Result<MutexGuard<'_, Option<ComparacaoGuardada>>, String> {
        self.importacao.lock().map_err(|_| {
            napi::Error::new(
                "LOCK_POISONED".to_string(),
                "Estado interno do núcleo corrompido.".to_string(),
            )
        })
    }

    /// Roda `f` e transforma um pânico em `ERRO_INTERNO` em vez de abortar o processo do Electron.
    /// As travas voltam a valer: a transação que estivesse aberta foi desfeita ao ser solta, e a
    /// comparação guardada é descartada, já que pode ter ficado pela metade.
    fn blindar<T>(&self, f: impl FnOnce() -> napi::Result<T, String>) -> napi::Result<T, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
            self.db.clear_poison();
            self.importacao.clear_poison();
            if let Ok(mut g) = self.importacao.lock() {
                *g = None;
            }
            Err(erro_interno())
        })
    }

    #[napi]
    pub fn info(&self) -> napi::Result<AppInfo, String> {
        let db = self.db()?;
        Ok(AppInfo {
            core_version: CORE_VERSION.to_string(),
            schema_version: db.schema_version().map_err(to_js)?,
            db_path: db.path().to_string_lossy().into_owned(),
        })
    }

    #[napi]
    pub fn catalogo(&self) -> Catalogo {
        investimentos::catalogo()
    }

    #[napi]
    pub fn listar_investimentos(&self) -> napi::Result<Vec<InvestimentoJs>, String> {
        let lista = self
            .db()?
            .listar_investimentos(self.data_mercado())
            .map_err(to_js)?;
        Ok(lista.into_iter().map(investimentos::investimento).collect())
    }

    #[napi]
    pub fn investimento(&self, id: i64) -> napi::Result<InvestimentoJs, String> {
        let i = self
            .db()?
            .investimento(id, self.data_mercado())
            .map_err(to_js)?;
        Ok(investimentos::investimento(i))
    }

    #[napi]
    pub fn criar_investimento(
        &self,
        dados: NovoInvestimentoJs,
    ) -> napi::Result<ResultadoGravacao, String> {
        let g = self
            .db()?
            .criar_investimento(&entrada(dados), hoje_real())
            .map_err(to_js)?;
        Ok(resultado(g))
    }

    #[napi]
    pub fn atualizar_investimento(
        &self,
        id: i64,
        dados: NovoInvestimentoJs,
    ) -> napi::Result<ResultadoGravacao, String> {
        let g = self
            .db()?
            .atualizar_investimento(id, &entrada(dados), hoje_real())
            .map_err(to_js)?;
        Ok(resultado(g))
    }

    #[napi]
    pub fn apagar_investimento(&self, id: i64) -> napi::Result<(), String> {
        self.db()?.apagar_investimento(id).map_err(to_js)
    }

    #[napi]
    pub fn resumo_carteira(&self) -> napi::Result<ResumoJs, String> {
        let r = self
            .db()?
            .resumo_carteira(self.data_mercado())
            .map_err(to_js)?;
        Ok(investimentos::resumo(r))
    }

    /// Assíncrono (`Promise`): calcula numa thread do libuv, com conexão própria.
    #[napi]
    pub fn painel(&self, meses: u32, classe: Option<String>) -> AsyncTask<CalcularPainel> {
        painel::tarefa(self.pasta_dados.clone(), self.data_mercado(), meses, classe)
    }

    #[napi]
    pub fn atualizar_mercado(&self) -> AsyncTask<AtualizarMercado> {
        mercado::tarefa(
            self.pasta_dados.clone(),
            self.modo_teste.clone(),
            self.data_mercado(),
        )
    }

    #[napi]
    pub fn status_mercado(&self) -> napi::Result<Vec<StatusFonteJs>, String> {
        let s = self.db()?.status_fontes().map_err(to_js)?;
        Ok(mercado::status(s))
    }

    #[napi]
    pub fn questionario(&self) -> QuestionarioJs {
        perfil::questionario()
    }

    #[napi]
    pub fn registrar_perfil(&self, respostas: Vec<u32>) -> napi::Result<ResultadoPerfilJs, String> {
        // Valor fora de 0..=255 vira 255, que o núcleo recusa como resposta inválida.
        let respostas: Vec<u8> = respostas
            .into_iter()
            .map(|r| u8::try_from(r).unwrap_or(u8::MAX))
            .collect();
        let r = self
            .db()?
            .registrar_perfil(&respostas, hoje_real())
            .map_err(to_js)?;
        Ok(match r {
            Ok(g) => ResultadoPerfilJs {
                perfil: Some(perfil::perfil(g)),
                erros: Vec::new(),
            },
            Err(e) => ResultadoPerfilJs {
                perfil: None,
                erros: erros(vec![e]),
            },
        })
    }

    #[napi]
    pub fn perfil_atual(&self) -> napi::Result<Option<PerfilJs>, String> {
        let p = self.db()?.perfil_atual(hoje_real()).map_err(to_js)?;
        Ok(p.map(perfil::perfil))
    }

    #[napi]
    pub fn alocacao_alvo(&self) -> napi::Result<Option<Vec<AlocacaoClasseJs>>, String> {
        let a = self.db()?.alocacao_alvo().map_err(to_js)?;
        Ok(a.map(perfil::alocacao))
    }

    #[napi]
    pub fn salvar_alocacao_alvo(
        &self,
        valores: Vec<PercentualClasseJs>,
    ) -> napi::Result<Vec<ErroCampoJs>, String> {
        let valores: Vec<(String, i64)> = valores
            .into_iter()
            .map(|v| (v.classe, v.percentual))
            .collect();
        let e = self.db()?.salvar_alocacao_alvo(&valores).map_err(to_js)?;
        Ok(erros(e))
    }

    #[napi]
    pub fn objetivo(&self) -> napi::Result<Option<String>, String> {
        self.db()?.objetivo().map_err(to_js)
    }

    #[napi]
    pub fn salvar_objetivo(&self, texto: String) -> napi::Result<Vec<ErroCampoJs>, String> {
        let e = self.db()?.salvar_objetivo(&texto).map_err(to_js)?;
        Ok(erros(e))
    }

    #[napi]
    pub fn listar_metas(&self) -> napi::Result<Vec<MetaJs>, String> {
        let l = self
            .db()?
            .listar_metas(self.data_mercado())
            .map_err(to_js)?;
        Ok(l.into_iter().map(metas::meta).collect())
    }

    #[napi]
    pub fn meta(&self, id: i64) -> napi::Result<MetaJs, String> {
        let m = self.db()?.meta(id, self.data_mercado()).map_err(to_js)?;
        Ok(metas::meta(m))
    }

    #[napi]
    pub fn criar_meta(&self, dados: NovaMetaJs) -> napi::Result<ResultadoGravacao, String> {
        let r = self
            .db()?
            .criar_meta(&metas::entrada(dados), hoje_real())
            .map_err(to_js)?;
        Ok(resultado_meta(r))
    }

    #[napi]
    pub fn atualizar_meta(
        &self,
        id: i64,
        dados: NovaMetaJs,
    ) -> napi::Result<ResultadoGravacao, String> {
        let r = self
            .db()?
            .atualizar_meta(id, &metas::entrada(dados), hoje_real())
            .map_err(to_js)?;
        Ok(resultado_meta(r))
    }

    #[napi]
    pub fn apagar_meta(&self, id: i64) -> napi::Result<(), String> {
        self.db()?.apagar_meta(id).map_err(to_js)
    }

    /// Compara a planilha da B3 com a carteira e guarda o resultado (descartando o anterior).
    #[napi]
    pub fn comparar_importacao(
        &self,
        bytes: Buffer,
        arquivo: String,
    ) -> napi::Result<ComparacaoJs, String> {
        // A anterior deixa de valer já: se esta falhar, o token antigo é IMPORTACAO_EXPIRADA.
        *self.importacao()? = None;
        self.blindar(|| {
            let posicao = guru_core::ler_posicao(&bytes).map_err(to_js)?;
            let c = guru_core::comparar(&*self.db()?, &posicao, hoje_real()).map_err(to_js)?;
            let token = importacao::novo_token();
            let js = importacao::comparacao(&token, &arquivo, &c);
            *self.importacao()? = Some(ComparacaoGuardada {
                token,
                arquivo,
                comparacao: c,
            });
            Ok(js)
        })
    }

    #[napi]
    pub fn aplicar_importacao(
        &self,
        token: String,
        decisoes: Vec<DecisaoJs>,
    ) -> napi::Result<ResultadoImportacaoJs, String> {
        self.blindar(|| {
            let mut guardada = self.importacao()?;
            // O token vale primeiro: um token velho é sempre IMPORTACAO_EXPIRADA.
            let g = guardada
                .as_ref()
                .filter(|g| g.token == token)
                .ok_or_else(importacao::expirada)?;
            let decisoes = decisoes
                .into_iter()
                .map(importacao::decisao)
                .collect::<Result<Vec<_>, _>>()
                .map_err(to_js)?;
            let r = guru_core::aplicar_importacao(
                &*self.db()?,
                &g.comparacao,
                &decisoes,
                &g.arquivo,
                hoje_real(),
            )
            .map_err(to_js)?;
            // Aplicada com sucesso, a comparação é consumida: repetir a chamada não duplica nada.
            // Com erros de campo nada foi gravado e ela fica para o usuário corrigir e tentar de novo.
            if r.erros.is_empty() {
                *guardada = None;
            }
            Ok(importacao::resultado(r.aplicadas, r.erros))
        })
    }

    #[napi]
    pub fn ultima_importacao(&self) -> napi::Result<Option<String>, String> {
        guru_core::ultima_importacao(&*self.db()?).map_err(to_js)
    }
}

fn hoje_real() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

fn resultado_meta(r: Result<i64, Vec<guru_core::ErroCampo>>) -> ResultadoGravacao {
    match r {
        Ok(id) => ResultadoGravacao {
            id: Some(id),
            erros: Vec::new(),
        },
        Err(lista) => ResultadoGravacao {
            id: None,
            erros: erros(lista),
        },
    }
}

fn resultado(g: Gravacao) -> ResultadoGravacao {
    match g {
        Gravacao::Ok(id) => ResultadoGravacao {
            id: Some(id),
            erros: Vec::new(),
        },
        Gravacao::Invalido(lista) => ResultadoGravacao {
            id: None,
            erros: erros(lista),
        },
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn core() -> Core {
        let pasta = std::env::temp_dir().join(format!("guru-node-{}", importacao::novo_token()));
        std::fs::create_dir_all(&pasta).unwrap();
        Core::open(pasta.to_string_lossy().into_owned(), None).unwrap()
    }

    #[test]
    fn panico_vira_erro_interno_e_as_travas_continuam_valendo() {
        let c = core();
        let r: napi::Result<(), String> = c.blindar(|| {
            let _db = c.db()?;
            let _imp = c.importacao()?;
            panic!("simulado");
        });
        let e = r.unwrap_err();
        assert_eq!(e.status, "ERRO_INTERNO");
        assert!(c.db().is_ok());
        assert!(c.importacao().unwrap().is_none());
        assert_eq!(c.blindar(|| Ok(7)).unwrap(), 7);
    }
}
