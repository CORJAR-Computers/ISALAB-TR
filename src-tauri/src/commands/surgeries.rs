use tauri::State;

use crate::commands::current_user;
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
    let user = current_user(&state)?;
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
    let mut pooled = state.pool.acquire()?;
    surgeries_repo::set_status(pooled.conn(), id, &status)
}
