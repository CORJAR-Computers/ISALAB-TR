use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::panel::{Panel, PanelAnalyte, PanelInput};
use crate::repositories::panels as panels_repo;
use crate::state::AppState;

/// Lista los paneles de analitos configurados (para la carga por lotes).
#[tauri::command]
#[specta::specta]
pub fn list_panels(state: State<'_, AppState>) -> Result<Vec<Panel>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    panels_repo::list(pooled.conn())
}

/// Analitos de un panel.
#[tauri::command]
#[specta::specta]
pub fn list_panel_analytes(
    state: State<'_, AppState>,
    panel_id: i32,
) -> Result<Vec<PanelAnalyte>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    panels_repo::list_analytes(pooled.conn(), panel_id)
}

/// Crea o actualiza un panel (solo veterinarios/administradores).
#[tauri::command]
#[specta::specta]
pub fn save_panel(state: State<'_, AppState>, input: PanelInput) -> Result<Panel, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    panels_repo::save(pooled.conn(), &input)
}

/// Elimina un panel.
#[tauri::command]
#[specta::specta]
pub fn delete_panel(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    panels_repo::delete(pooled.conn(), id)
}
