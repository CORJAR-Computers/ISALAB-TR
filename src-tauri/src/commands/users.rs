use tauri::State;

use crate::auth::{require_admin, require_session};
use crate::error::AppError;
use crate::models::auth::{
    ChangePasswordInput, CreateUserInput, SessionUser, UserListItem,
};
use crate::repositories::auth as auth_repo;
use crate::repositories::users;
use crate::state::AppState;

/// Listado de usuarios (sin hashes) — solo ADMIN.
#[tauri::command]
#[specta::specta]
pub fn list_users(state: State<'_, AppState>) -> Result<Vec<UserListItem>, AppError> {
    require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    users::list(pooled.conn())
}

/// Crea un usuario con rol y contraseña inicial — solo ADMIN. El usuario
/// deberá cambiar la contraseña en su primer acceso (MUST_CHANGE_PASSWORD).
#[tauri::command]
#[specta::specta]
pub fn create_user(
    state: State<'_, AppState>,
    input: CreateUserInput,
) -> Result<UserListItem, AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let new_user = users::create(pooled.conn(), &input)?;

    // Auditoría de creación de usuario.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "USER_CREATED",
            Some(&format!("Usuario '{}' creado con rol {}", input.username, input.role)),
        ).ok();
    }

    Ok(new_user)
}

/// Cambia la contraseña del usuario con la sesión activa. Verifica la
/// contraseña actual y limpia MUST_CHANGE_PASSWORD. Devuelve la sesión
/// actualizada.
#[tauri::command]
#[specta::specta]
pub fn change_password(
    state: State<'_, AppState>,
    input: ChangePasswordInput,
) -> Result<SessionUser, AppError> {
    let user = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let session = users::change_password(
        pooled.conn(),
        user.id,
        &input.current_password,
        &input.new_password,
    )?;

    // Auditoría de cambio de contraseña.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "PASSWORD_CHANGED",
            Some("Contraseña actualizada"),
        ).ok();
    }

    let mut guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    *guard = Some(session.clone());
    Ok(session)
}
