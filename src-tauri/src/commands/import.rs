use tauri::State;

use crate::auth::require_vet_or_admin;
use crate::error::AppError;
use crate::models::import::{AnalyzerImportMapping, ImportPreview, ImportSummary};
use crate::repositories::import as import_repo;
use crate::state::AppState;

/// Vista previa de un archivo CSV de analizador: encabezados, primeras filas
/// y sugerencia automática del mapeo columna → analito.
#[tauri::command]
#[specta::specta]
pub fn preview_analyzer_import(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportPreview, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    import_repo::preview(pooled.conn(), &path)
}

/// Importa resultados desde el CSV del analizador con el mapeo confirmado.
#[tauri::command]
#[specta::specta]
pub fn import_analyzer_results(
    state: State<'_, AppState>,
    path: String,
    mapping: AnalyzerImportMapping,
) -> Result<ImportSummary, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let summary = import_repo::import(pooled.conn(), &path, &mapping)?;

    // Auditoría de la importación.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "IMPORT_ANALYZER_RESULTS",
            Some(&format!(
                "Importación {}: {} muestras, {} resultados",
                std::path::Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone()),
                summary.samples_updated,
                summary.results_imported
            )),
        )
        .ok();
    }

    Ok(summary)
}
