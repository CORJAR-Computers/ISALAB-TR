use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::surgery::{CreateSurgeryInput, Surgery};
use crate::repositories::surgeries as surgeries_repo;
use crate::state::AppState;

/// Programa una cirugía atribuyéndola al veterinario de la sesión activa.
#[tauri::command]
#[specta::specta]
pub fn create_surgery(
    state: State<'_, AppState>,
    input: CreateSurgeryInput,
) -> Result<Surgery, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    surgeries_repo::create(pooled.conn(), &input, Some(user.id))
}

/// Agenda quirúrgica: listado con filtros por estado y búsqueda.
#[tauri::command]
#[specta::specta]
pub fn list_surgeries(
    state: State<'_, AppState>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<Surgery>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    surgeries_repo::list(pooled.conn(), status.as_deref(), search.as_deref())
}

/// Cambia el estado de una cirugía (PROGRAMADA→EN_CURSO/COMPLETADA/CANCELADA).
#[tauri::command]
#[specta::specta]
pub fn set_surgery_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<Surgery, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let surgery = surgeries_repo::set_status(pooled.conn(), id, &status)?;

    // Auditoría de transición de estado de cirugía.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "SURGERY_STATUS_CHANGE",
            Some(&format!("Cirugía {} → estado {}", id, status)),
        )
        .ok();
    }

    Ok(surgery)
}

/// Conteo de cirugías por estado (sin cargar las filas completas).
#[tauri::command]
#[specta::specta]
pub fn count_surgeries(state: State<'_, AppState>) -> Result<Vec<crate::models::status_count::StatusCount>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    surgeries_repo::count_by_status(pooled.conn())
}
