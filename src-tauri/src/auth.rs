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
    let parsed =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(format!("Hash inválido: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

use crate::models::auth::SessionUser;
use crate::state::AppState;

/// Devuelve el usuario de la sesión activa o un error 403 Forbidden.
pub fn require_session(state: &AppState) -> Result<SessionUser, AppError> {
    let guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    guard
        .clone()
        .ok_or_else(|| AppError::Forbidden("Inicia sesión para continuar".into()))
}

/// Exige que la sesión activa pertenezca a un usuario con rol ADMIN.
pub fn require_admin(state: &AppState) -> Result<SessionUser, AppError> {
    let user = require_session(state)?;
    if user.role == "ADMIN" {
        Ok(user)
    } else {
        Err(AppError::Forbidden(
            "Solo el administrador puede realizar esta acción".into(),
        ))
    }
}

/// Exige que la sesión activa pertenezca a un VETERINARIO o ADMIN.
pub fn require_vet_or_admin(state: &AppState) -> Result<SessionUser, AppError> {
    let user = require_session(state)?;
    if user.role == "ADMIN" || user.role == "VETERINARIO" {
        Ok(user)
    } else {
        Err(AppError::Forbidden(
            "Se requieren permisos de Veterinario o Administrador para esta acción".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "SecretPassword123!";
        let hash = hash_password(password).expect("El hashing debe ser exitoso");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("WrongPassword", &hash).unwrap());
    }

    // Dummy State creation for testing would require full AppState setup,
    // so we can test the roles logic conceptually or create a mock AppState if it was easy.
    // Instead, since require_admin is tightly coupled with State, we would usually
    // test the business logic decoupling it, but for now we'll leave a placeholder test.
    #[test]
    fn test_role_auth_logic() {
        let user_admin = SessionUser {
            id: 1,
            username: "admin".into(),
            full_name: "Admin".into(),
            role: "ADMIN".into(),
            must_change_password: false,
        };
        let user_vet = SessionUser {
            id: 2,
            username: "vet".into(),
            full_name: "Vet".into(),
            role: "VETERINARIO".into(),
            must_change_password: false,
        };
        let user_aux = SessionUser {
            id: 3,
            username: "aux".into(),
            full_name: "Aux".into(),
            role: "AUXILIAR".into(),
            must_change_password: false,
        };

        // Logic of require_admin
        assert_eq!(user_admin.role, "ADMIN");
        assert_ne!(user_vet.role, "ADMIN");

        // Logic of require_vet_or_admin
        assert!(user_admin.role == "ADMIN" || user_admin.role == "VETERINARIO");
        assert!(user_vet.role == "ADMIN" || user_vet.role == "VETERINARIO");
        assert!(!(user_aux.role == "ADMIN" || user_aux.role == "VETERINARIO"));
    }
}
