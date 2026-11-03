//! Cofre da chave da Anthropic. A chave nunca vai para o SQLite nem para o log.

use std::sync::Mutex;

use crate::error::CoreError;
use crate::investimento::ErroCampo;

const SERVICO: &str = "guru-de-investimentos";
const USUARIO: &str = "anthropic";

pub trait Cofre: Send + Sync {
    fn ler(&self) -> Result<Option<String>, CoreError>;
    fn gravar(&self, chave: &str) -> Result<(), CoreError>;
    fn apagar(&self) -> Result<(), CoreError>;
}

/// Cofre de senhas do sistema operacional (via `keyring`).
pub struct CofreSistema;

fn entrada() -> Result<keyring::Entry, CoreError> {
    keyring::Entry::new(SERVICO, USUARIO).map_err(|_| CoreError::CofreIndisponivel)
}

impl Cofre for CofreSistema {
    fn ler(&self) -> Result<Option<String>, CoreError> {
        match entrada()?.get_password() {
            Ok(chave) => Ok(Some(chave)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CoreError::CofreIndisponivel),
        }
    }

    fn gravar(&self, chave: &str) -> Result<(), CoreError> {
        entrada()?
            .set_password(chave)
            .map_err(|_| CoreError::CofreIndisponivel)
    }

    fn apagar(&self) -> Result<(), CoreError> {
        match entrada()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CoreError::CofreIndisponivel),
        }
    }
}

/// Cofre em memória, só para testes e para o e2e.
#[derive(Default)]
pub struct CofreMemoria(Mutex<Option<String>>);

impl CofreMemoria {
    fn guardado(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Cofre for CofreMemoria {
    fn ler(&self) -> Result<Option<String>, CoreError> {
        Ok(self.guardado().clone())
    }

    fn gravar(&self, chave: &str) -> Result<(), CoreError> {
        *self.guardado() = Some(chave.to_string());
        Ok(())
    }

    fn apagar(&self) -> Result<(), CoreError> {
        *self.guardado() = None;
        Ok(())
    }
}

/// Sem os espaços das pontas: começa com `sk-ant-`, de 20 a 300 caracteres, só ASCII visível
/// (sem espaço, acento nem caractere de controle, que o cabeçalho HTTP recusaria).
/// A mensagem nunca repete a chave.
pub fn validar_formato_chave(chave: &str) -> Option<ErroCampo> {
    let chave = chave.trim();
    let valida = chave.starts_with("sk-ant-")
        && (20..=300).contains(&chave.len())
        && chave.bytes().all(|b| (0x21..=0x7E).contains(&b));
    (!valida).then(|| ErroCampo {
        campo: "chave".into(),
        mensagem: "Isso não parece uma chave da Anthropic (começa com sk-ant-).".into(),
    })
}
