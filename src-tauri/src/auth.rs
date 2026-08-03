//! Autenticación: hashing y verificación de contraseñas (Argon2id).
//!
//! En una app de escritorio la verificación se hace contra la tabla USERS
//! local; el hash nunca sale del dispositivo.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;

use crate::error::AppError;

/// Genera un hash Argon2id con salt aleatorio.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))
}

/// Verifica una contraseña contra un hash PHC (p. ej. `$argon2id$v=19$…`).
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Hash inválido: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
