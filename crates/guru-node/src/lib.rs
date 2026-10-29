use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use guru_core::{CORE_VERSION, CoreError, Db, Gravacao};
use napi_derive::napi;

mod investimentos;
use investimentos::{
    Catalogo, InvestimentoJs, NovoInvestimentoJs, ResultadoGravacao, ResumoJs, entrada, erros,
};

#[napi(object)]
pub struct AppInfo {
    pub core_version: String,
    pub schema_version: u32,
    pub db_path: String,
}

#[napi]
pub struct Core {
    db: Mutex<Db>,
}

fn to_js(e: CoreError) -> napi::Error<String> {
    napi::Error::new(e.code().to_string(), e.to_string())
}

#[napi]
impl Core {
    #[napi(factory)]
    pub fn open(data_dir: String) -> napi::Result<Core, String> {
        let db = Db::open(Path::new(&data_dir)).map_err(to_js)?;
        Ok(Core { db: Mutex::new(db) })
    }

    fn db(&self) -> napi::Result<MutexGuard<'_, Db>, String> {
        self.db.lock().map_err(|_| {
            napi::Error::new(
                "LOCK_POISONED".to_string(),
                "Estado interno do núcleo corrompido.".to_string(),
            )
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
        let lista = self.db()?.listar_investimentos().map_err(to_js)?;
        Ok(lista.into_iter().map(investimentos::investimento).collect())
    }

    #[napi]
    pub fn investimento(&self, id: i64) -> napi::Result<InvestimentoJs, String> {
        let i = self.db()?.investimento(id).map_err(to_js)?;
        Ok(investimentos::investimento(i))
    }

    #[napi]
    pub fn criar_investimento(
        &self,
        dados: NovoInvestimentoJs,
    ) -> napi::Result<ResultadoGravacao, String> {
        let g = self
            .db()?
            .criar_investimento(&entrada(dados), hoje())
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
            .atualizar_investimento(id, &entrada(dados), hoje())
            .map_err(to_js)?;
        Ok(resultado(g))
    }

    #[napi]
    pub fn apagar_investimento(&self, id: i64) -> napi::Result<(), String> {
        self.db()?.apagar_investimento(id).map_err(to_js)
    }

    #[napi]
    pub fn resumo_carteira(&self) -> napi::Result<ResumoJs, String> {
        let r = self.db()?.resumo_carteira().map_err(to_js)?;
        Ok(investimentos::resumo(r))
    }
}

fn hoje() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
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
