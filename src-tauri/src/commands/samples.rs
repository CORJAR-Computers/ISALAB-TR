use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::sample::{
    CreateSampleInput, LabResult, RegisterResultInput, RegisterResultsInput, Sample,
};
use crate::models::sample_list_item::SampleListItem;
use crate::models::worklist::WorklistData;
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

/// Bandeja de trabajo diaria: muestras pendientes (RECIBIDA/EN_PROCESO)
/// agrupadas por tipo de muestra con el tiempo transcurrido desde la
/// recepción, para que el técnico sepa qué procesar primero.
#[tauri::command]
#[specta::specta]
pub fn get_worklist(state: State<'_, AppState>) -> Result<WorklistData, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    samples_repo::get_worklist(pooled.conn())
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
pub fn get_sample(state: State<'_, AppState>, id: i32) -> Result<Option<Sample>, AppError> {
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

/// Conteo de muestras por estado (sin cargar las filas completas).
#[tauri::command]
#[specta::specta]
pub fn count_samples(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::status_count::StatusCount>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    samples_repo::count_by_status(pooled.conn())
}

/// Carga varios resultados de una misma muestra (grilla de panel o
/// importación desde analizador) y devuelve todos los resultados.
#[tauri::command]
#[specta::specta]
pub fn register_lab_results(
    state: State<'_, AppState>,
    input: RegisterResultsInput,
) -> Result<Vec<LabResult>, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let results = history_repo::register_results_batch(pooled.conn(), &input)?;

    // Invalidar cache de IA cuando se actualizan resultados.
    state.ai_cache.invalidate(input.sample_id);

    Ok(results)
}

/// Registra la calidad preanalítica de una muestra (interferencia HIL).
#[tauri::command]
#[specta::specta]
pub fn set_sample_quality(
    state: State<'_, AppState>,
    id: i32,
    quality_index: Option<String>,
    quality_severity: Option<String>,
    quality_note: Option<String>,
) -> Result<Sample, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let sample = samples_repo::set_quality(
        pooled.conn(),
        id,
        quality_index.as_deref(),
        quality_severity.as_deref(),
        quality_note.as_deref(),
    )?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "SAMPLE_QUALITY_CHANGE",
            Some(&format!(
                "Muestra {id} · calidad {} ({})",
                quality_index.unwrap_or_else(|| "NORMAL".into()),
                quality_severity.unwrap_or_else(|| "—".into())
            )),
        )
        .ok();
    }

    Ok(sample)
}

/// Rechaza una muestra (RECIBIDA/EN_PROCESO → RECHAZADA) con motivo.
#[tauri::command]
#[specta::specta]
pub fn reject_sample(
    state: State<'_, AppState>,
    id: i32,
    reason: String,
) -> Result<Sample, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let sample = samples_repo::reject_sample(pooled.conn(), id, &reason, &user.username)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "SAMPLE_REJECTED",
            Some(&format!("Muestra {} rechazada: {}", id, reason)),
        )
        .ok();
    }

    Ok(sample)
}

/// Reabre una muestra rechazada (RECHAZADA → RECIBIDA).
#[tauri::command]
#[specta::specta]
pub fn reopen_sample(state: State<'_, AppState>, id: i32) -> Result<Sample, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let sample = samples_repo::reopen_sample(pooled.conn(), id)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "SAMPLE_REOPENED",
            Some(&format!("Muestra {id} reabierta (RECIBIDA)")),
        )
        .ok();
    }

    Ok(sample)
}
