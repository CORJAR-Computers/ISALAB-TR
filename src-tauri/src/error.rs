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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_variants() {
        let db_err = AppError::Db("Connection failed".into());
        assert_eq!(db_err.to_string(), "Connection failed");

        let not_found = AppError::NotFound("Patient 42 not found".into());
        assert_eq!(not_found.to_string(), "Patient 42 not found");

        let validation = AppError::Validation("Invalid input".into());
        assert_eq!(validation.to_string(), "Invalid input");

        let forbidden = AppError::Forbidden("Admin required".into());
        assert_eq!(forbidden.to_string(), "Admin required");

        let internal = AppError::Internal("Unexpected".into());
        assert_eq!(internal.to_string(), "Unexpected");
    }

    #[test]
    fn test_error_serialization_tagged() {
        let err = AppError::NotFound("test".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["type"], "NotFound");
        assert_eq!(json["data"], "test");
    }

    #[test]
    fn test_fb_error_conversion() {
        // FbError can be created via From<&str> or other means
        // The key test is that the From<FbError> impl produces AppError::Db
        let fb_err: rsfbclient::FbError = "test error".into();
        let app_err: AppError = fb_err.into();
        match app_err {
            AppError::Db(msg) => assert!(!msg.is_empty()),
            _ => panic!("Expected Db variant"),
        }
    }
}
