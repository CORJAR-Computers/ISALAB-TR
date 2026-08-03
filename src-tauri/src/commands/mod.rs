pub mod auth;
pub mod catalog;
pub mod clinical_history;
pub mod dashboard;
pub mod db;
pub mod invoices;
pub mod patients;
pub mod reports;
pub mod samples;
pub mod settings;
pub mod surgeries;
pub mod users;
pub mod vaccines;

use crate::error::AppError;
use crate::models::auth::SessionUser;
use crate::state::AppState;

/// Usuario de la sesión activa. Los registros clínicos (consultas, cirugías,
/// vacunas) y las facturas se atribuyen al operador que inició sesión.
pub fn current_user(state: &AppState) -> Result<SessionUser, AppError> {
    let guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    guard
        .clone()
        .ok_or_else(|| AppError::Forbidden("Inicia sesión para continuar".into()))
}
