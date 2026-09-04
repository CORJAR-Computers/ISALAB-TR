use tauri::State;

use crate::auth::require_vet_or_admin;
use crate::error::AppError;
use crate::models::analyzer_source::{AnalyzerImportJob, AnalyzerSource, SaveAnalyzerSourceInput};
use crate::repositories::analyzer_sources as sources_repo;
use crate::sources::driver_for;
use crate::state::AppState;

/// Fuentes automáticas configuradas (carpeta vigilada por analizador).
#[tauri::command]
#[specta::specta]
pub fn list_analyzer_sources(state: State<'_, AppState>) -> Result<Vec<AnalyzerSource>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::list(pooled.conn())
}

/// Guarda (crea o reemplaza) la fuente de un analizador. Con `folder_path`
/// None elimina la fuente. El mapeo puede guardarse después por separado.
#[tauri::command]
#[specta::specta]
pub fn save_analyzer_source(
    state: State<'_, AppState>,
    input: SaveAnalyzerSourceInput,
) -> Result<Option<AnalyzerSource>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::save(pooled.conn(), &input)
}

/// Elimina la fuente de un analizador (y su cola de trabajos).
#[tauri::command]
#[specta::specta]
pub fn delete_analyzer_source(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::delete(pooled.conn(), id)
}

/// Sondea una fuente ahora (sin esperar el ciclo del supervisor) y devuelve
/// los trabajos resultantes. Sirve para "probar" tras guardar la carpeta.
#[tauri::command]
#[specta::specta]
pub fn poll_analyzer_source(
    state: State<'_, AppState>,
    source_id: i32,
) -> Result<Vec<AnalyzerImportJob>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    // Carga la fuente y construye el driver.
    let all = sources_repo::list(pooled.conn())?;
    let source = all
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| AppError::NotFound(format!("Fuente {source_id} no encontrada")))?;

    let mut driver = driver_for(&source)
        .ok_or_else(|| AppError::Validation("La fuente no tiene mapeo configurado".into()))?;
    let outcomes = driver.poll_once(pooled.conn())?;

    let mut jobs = Vec::with_capacity(outcomes.len());
    for outcome in &outcomes {
        jobs.push(sources_repo::record_job(pooled.conn(), source_id, outcome)?);
    }
    sources_repo::touch_poll(pooled.conn(), source_id)?;
    Ok(jobs)
}

/// Cola de importación de una fuente (más recientes primero).
#[tauri::command]
#[specta::specta]
pub fn list_analyzer_import_jobs(
    state: State<'_, AppState>,
    source_id: i32,
    limit: i32,
) -> Result<Vec<AnalyzerImportJob>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::list_jobs(pooled.conn(), source_id, limit)
}

/// Vista global de trabajos fallidos de todas las fuentes.
#[tauri::command]
#[specta::specta]
pub fn list_failed_analyzer_imports(
    state: State<'_, AppState>,
    limit: i32,
) -> Result<Vec<AnalyzerImportJob>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::list_failed_jobs(pooled.conn(), limit)
}

/// Elimina un trabajo de la cola: el archivo queda pendiente y el próximo
/// sondeo (o "Probar ahora") volverá a intentar importarlo.
#[tauri::command]
#[specta::specta]
pub fn delete_analyzer_import_job(state: State<'_, AppState>, job_id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    sources_repo::delete_job(pooled.conn(), job_id)
}
