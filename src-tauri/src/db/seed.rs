//! Datos de arranque que dependen de lógica de Rust (no de SQL estático).

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::auth;
use crate::error::AppError;

/// Primer acceso de la app: si el usuario `admin` aún no tiene contraseña
/// (PASSWORD_HASH NULL), se fija `admin123` (Argon2id) con
/// `MUST_CHANGE_PASSWORD = TRUE`, de modo que el primer inicio de sesión
/// exige fijar una contraseña propia antes de usar la app.
///
/// Devuelve `true` si se asignó la contraseña por defecto.
pub fn ensure_default_admin(conn: &mut SimpleConnection) -> Result<bool, AppError> {
    let hash: Option<(Option<String>,)> = conn
        .query_first(
            "SELECT PASSWORD_HASH FROM USERS WHERE UPPER(USERNAME) = 'ADMIN'",
            (),
        )
        .map_err(AppError::from)?;

    match hash.map(|(h,)| h) {
        Some(Some(_)) => Ok(false),
        Some(None) => {
            let new_hash = auth::hash_password("admin123")?;
            conn.execute(
                "UPDATE USERS SET PASSWORD_HASH = ?, MUST_CHANGE_PASSWORD = TRUE WHERE UPPER(USERNAME) = 'ADMIN'",
                (&new_hash,),
            )
            .map_err(AppError::from)?;
            Ok(true)
        }
        None => Ok(false),
    }
}
