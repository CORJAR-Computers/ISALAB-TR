use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::sample::{CreateSampleInput, LabResult, RegisterResultInput, Sample};
use crate::models::sample_list_item::SampleListItem;
use crate::repositories::clinical_history as history_repo;
use crate::repositories::samples as samples_repo;
use crate::state::AppState;

/// Recepción de muestra analítica (trazabilidad: código M-YYYY-NNNN, estado RECIBIDA).
#[tauri::command]
#[specta::specta]
pub fn create_sample(
    state: State<'_, AppState>,
    input: CreateSampleInput,
) -> Result<Sample, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    history_repo::create_sample(pooled.conn(), &input)
}

/// Carga un resultado de laboratorio; valida el rango de referencia por
/// especie/edad/sexo vía stored procedure y finaliza la muestra.
/// Invalida el cache de IA para esta muestra.
#[tauri::command]
#[specta::specta]
pub fn register_lab_result(
    state: State<'_, AppState>,
    input: RegisterResultInput,
) -> Result<LabResult, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let result = history_repo::register_lab_result(pooled.conn(), &input)?;
    
    // Invalidar cache de IA cuando se actualizan resultados
    state.ai_cache.invalidate(input.sample_id);
    
    Ok(result)
}

/// Mesa de trabajo del laboratorio: listado global de muestras con filtros
/// opcionales por estado y búsqueda (código, paciente o propietario).
#[tauri::command]
#[specta::specta]
pub fn list_samples(
    state: State<'_, AppState>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<SampleListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    samples_repo::list(pooled.conn(), status.as_deref(), search.as_deref())
}

/// Ficha completa de una muestra (con resultados) para el detalle del lab.
#[tauri::command]
#[specta::specta]
pub fn get_sample(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<Sample>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    samples_repo::get(pooled.conn(), id)
}

/// Cambia el estado de una muestra (RECIBIDA→EN_PROCESO, →ANULADA).
#[tauri::command]
#[specta::specta]
pub fn set_sample_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<Sample, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let sample = samples_repo::set_status(pooled.conn(), id, &status)?;

    // Auditoría de transición de estado de muestra.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "SAMPLE_STATUS_CHANGE",
            Some(&format!("Muestra {} → estado {}", id, status)),
        )
        .ok();
    }

    Ok(sample)
}
