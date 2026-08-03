use tauri::State;

use crate::auth;
use crate::error::AppError;
use crate::models::auth::{LoginInput, SessionUser};
use crate::repositories::auth as auth_repo;
use crate::state::AppState;

/// Inicia sesión verificando la contraseña (Argon2id) contra la tabla USERS.
/// La sesión es única: una app de escritorio con un operador a la vez.
#[tauri::command]
#[specta::specta]
pub fn login(
    state: State<'_, AppState>,
    input: LoginInput,
) -> Result<SessionUser, AppError> {
    let mut pooled = state.pool.acquire()?;
    let user = auth_repo::find_by_username(pooled.conn(), &input.username)?
        .ok_or_else(|| AppError::Validation("Usuario o contraseña incorrectos".into()))?;

    if !user.active {
        return Err(AppError::Validation("Usuario inactivo. Contacta al administrador.".into()));
    }
    let hash = user.password_hash.as_deref().ok_or_else(|| {
        AppError::Validation("Usuario sin contraseña configurada".into())
    })?;
    if !auth::verify_password(&input.password, hash)? {
        return Err(AppError::Validation("Usuario o contraseña incorrectos".into()));
    }

    let session = auth_repo::to_session(&user);
    let mut guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    *guard = Some(session.clone());
    Ok(session)
}

#[tauri::command]
#[specta::specta]
pub fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    *guard = None;
    Ok(())
}

/// Sesión activa (restaura la UI al reabrir la ventana).
#[tauri::command]
#[specta::specta]
pub fn get_session(state: State<'_, AppState>) -> Result<Option<SessionUser>, AppError> {
    let guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    Ok(guard.clone())
}
