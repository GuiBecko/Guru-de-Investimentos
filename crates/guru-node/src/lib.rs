use std::path::Path;
use std::sync::Mutex;

use guru_core::{CORE_VERSION, CoreError, Db};
use napi_derive::napi;

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

    #[napi]
    pub fn info(&self) -> napi::Result<AppInfo, String> {
        let db = self.db.lock().map_err(|_| {
            napi::Error::new(
                "LOCK_POISONED".to_string(),
                "Estado interno do núcleo corrompido.".to_string(),
            )
        })?;
        Ok(AppInfo {
            core_version: CORE_VERSION.to_string(),
            schema_version: db.schema_version().map_err(to_js)?,
            db_path: db.path().to_string_lossy().into_owned(),
        })
    }
}
