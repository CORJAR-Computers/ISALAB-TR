use serde::Serialize;
use specta::Type;
use thiserror::Error;

/// Error tipado de la app. Se serializa como `{ type, data }` para el frontend.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum AppError {
    #[error("{0}")]
    Db(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Internal(String),
}

impl From<rsfbclient::FbError> for AppError {
    fn from(e: rsfbclient::FbError) -> Self {
        AppError::Db(e.to_string())
    }
}
