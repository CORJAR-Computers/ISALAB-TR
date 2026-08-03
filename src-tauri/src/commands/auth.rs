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
    let user = match auth_repo::find_by_username(pooled.conn(), &input.username) {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Auditoría de login fallido (usuario no existe).
            if let Ok(mut audit_conn) = state.pool.acquire() {
                auth_repo::log_audit(
                    audit_conn.conn(),
                    None,
                    &input.username,
                    "LOGIN_FAILED",
                    Some("Usuario no encontrado"),
                ).ok();
            }
            return Err(AppError::Validation("Usuario o contraseña incorrectos".into()));
        }
        Err(e) => return Err(e),
    };

    if !user.active {
        if let Ok(mut audit_conn) = state.pool.acquire() {
            auth_repo::log_audit(
                audit_conn.conn(),
                Some(user.id),
                &user.username,
                "LOGIN_FAILED",
                Some("Usuario inactivo"),
            ).ok();
        }
        return Err(AppError::Validation("Usuario inactivo. Contacta al administrador.".into()));
    }

    let hash = user.password_hash.as_deref().ok_or_else(|| {
        AppError::Validation("Usuario sin contraseña configurada".into())
    })?;

    if !auth::verify_password(&input.password, hash)? {
        // Auditoría de login fallido (contraseña incorrecta).
        if let Ok(mut audit_conn) = state.pool.acquire() {
            auth_repo::log_audit(
                audit_conn.conn(),
                Some(user.id),
                &user.username,
                "LOGIN_FAILED",
                Some("Contraseña incorrecta"),
            ).ok();
        }
        return Err(AppError::Validation("Usuario o contraseña incorrectos".into()));
    }

    let session = auth_repo::to_session(&user);

    // Auditoría de login exitoso.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(session.id),
            &session.username,
            "LOGIN",
            Some("Inicio de sesión exitoso"),
        ).ok();
    }

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
    if let Some(user) = guard.as_ref() {
        if let Ok(mut pooled) = state.pool.acquire() {
            auth_repo::log_audit(
                pooled.conn(),
                Some(user.id),
                &user.username,
                "LOGOUT",
                Some("Cierre de sesión"),
            ).ok();
        }
    }
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
