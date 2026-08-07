use tauri::State;

use crate::auth::require_vet_or_admin;
use crate::error::AppError;
use crate::models::analyzer::{
    Analyzer, CreateAnalyzerInput, ReferenceRange, ReferenceRangeInput, UpdateAnalyzerInput,
};
use crate::repositories::analyzers as analyzers_repo;
use crate::state::AppState;

/// Lista los equipos analizadores configurados (con su nº de rangos).
#[tauri::command]
#[specta::specta]
pub fn list_analyzers(state: State<'_, AppState>) -> Result<Vec<Analyzer>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::list(pooled.conn())
}

/// Crea un equipo analizador (marca/modelo).
#[tauri::command]
#[specta::specta]
pub fn create_analyzer(
    state: State<'_, AppState>,
    input: CreateAnalyzerInput,
) -> Result<Analyzer, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::create(pooled.conn(), &input)
}

/// Actualiza los datos de un equipo.
#[tauri::command]
#[specta::specta]
pub fn update_analyzer(
    state: State<'_, AppState>,
    input: UpdateAnalyzerInput,
) -> Result<Analyzer, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::update(pooled.conn(), &input)
}

/// Activa/desactiva un equipo (los inactivos no aparecen en el selector).
#[tauri::command]
#[specta::specta]
pub fn set_analyzer_active(
    state: State<'_, AppState>,
    id: i32,
    active: bool,
) -> Result<Analyzer, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::set_active(pooled.conn(), id, active)
}

/// Elimina un equipo que no tenga muestras asociadas (borra sus rangos).
#[tauri::command]
#[specta::specta]
pub fn delete_analyzer(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::delete(pooled.conn(), id)
}

/// Rangos de referencia de un equipo (o de todos si `analyzer_id` es NULL).
#[tauri::command]
#[specta::specta]
pub fn list_reference_ranges(
    state: State<'_, AppState>,
    analyzer_id: Option<i32>,
) -> Result<Vec<ReferenceRange>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::list_ranges(pooled.conn(), analyzer_id)
}

/// Crea un rango de referencia para un equipo.
#[tauri::command]
#[specta::specta]
pub fn create_reference_range(
    state: State<'_, AppState>,
    input: ReferenceRangeInput,
) -> Result<ReferenceRange, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::create_range(pooled.conn(), &input)
}

/// Actualiza un rango de referencia existente.
#[tauri::command]
#[specta::specta]
pub fn update_reference_range(
    state: State<'_, AppState>,
    id: i32,
    input: ReferenceRangeInput,
) -> Result<ReferenceRange, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::update_range(pooled.conn(), id, &input)
}

/// Elimina un rango de referencia.
#[tauri::command]
#[specta::specta]
pub fn delete_reference_range(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    analyzers_repo::delete_range(pooled.conn(), id)
}
