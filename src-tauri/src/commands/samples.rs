use tauri::State;

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
    let mut pooled = state.pool.acquire()?;
    history_repo::create_sample(pooled.conn(), &input)
}

/// Carga un resultado de laboratorio; valida el rango de referencia por
/// especie/edad/sexo vía stored procedure y finaliza la muestra.
#[tauri::command]
#[specta::specta]
pub fn register_lab_result(
    state: State<'_, AppState>,
    input: RegisterResultInput,
) -> Result<LabResult, AppError> {
    let mut pooled = state.pool.acquire()?;
    history_repo::register_lab_result(pooled.conn(), &input)
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
    let mut pooled = state.pool.acquire()?;
    samples_repo::set_status(pooled.conn(), id, &status)
}
