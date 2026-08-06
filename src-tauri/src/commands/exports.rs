use std::fs;

use tauri::State;

use crate::auth::require_vet_or_admin;
use crate::error::AppError;
use crate::repositories::samples as samples_repo;
use crate::state::AppState;

/// Guarda el CSV de muestras (mesa de trabajo con filtros actuales) en
/// `dest_path` (elegido con el diálogo de guardar del SO) y devuelve la ruta.
#[tauri::command]
#[specta::specta]
pub fn export_samples_csv(
    state: State<'_, AppState>,
    dest_path: String,
    status: Option<String>,
    search: Option<String>,
) -> Result<String, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let samples = samples_repo::list(
        pooled.conn(),
        status.as_deref(),
        search.as_deref(),
    )?;
    let csv = crate::csv::samples_to_csv(&samples);
    write_csv(&dest_path, &csv)?;
    Ok(dest_path)
}

/// Guarda el CSV de resultados analíticos (filtro opcional por estado y
/// búsqueda) en `dest_path` y devuelve la ruta.
#[tauri::command]
#[specta::specta]
pub fn export_results_csv(
    state: State<'_, AppState>,
    dest_path: String,
    status: Option<String>,
    search: Option<String>,
) -> Result<String, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows = samples_repo::list_results_for_export(
        pooled.conn(),
        status.as_deref(),
        search.as_deref(),
    )?;
    let csv = crate::csv::results_to_csv(&rows);
    write_csv(&dest_path, &csv)?;
    Ok(dest_path)
}

fn write_csv(dest_path: &str, csv: &str) -> Result<(), AppError> {
    fs::write(dest_path, csv).map_err(|e| {
        AppError::Internal(format!(
            "No se pudo escribir el CSV en {}: {e}",
            dest_path
        ))
    })
}
