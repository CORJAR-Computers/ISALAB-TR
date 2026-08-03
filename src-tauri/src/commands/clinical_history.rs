use tauri::State;

use crate::error::AppError;
use crate::models::clinical_history::ClinicalHistory;
use crate::models::consultation::{
    Consultation, ConsultationListItem, CreateConsultationInput,
};
use crate::repositories::clinical_history as history_repo;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_clinical_history(
    state: State<'_, AppState>,
    patient_id: i32,
) -> Result<ClinicalHistory, AppError> {
    let mut pooled = state.pool.acquire()?;
    history_repo::get_clinical_history(pooled.conn(), patient_id)
}

#[tauri::command]
#[specta::specta]
pub fn create_consultation(
    state: State<'_, AppState>,
    input: CreateConsultationInput,
) -> Result<Consultation, AppError> {
    let mut pooled = state.pool.acquire()?;
    history_repo::create_consultation(pooled.conn(), &input)
}

/// Agenda: listado global de consultas con filtros por estado y búsqueda.
#[tauri::command]
#[specta::specta]
pub fn list_consultations(
    state: State<'_, AppState>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<ConsultationListItem>, AppError> {
    let mut pooled = state.pool.acquire()?;
    history_repo::list_agenda(pooled.conn(), status.as_deref(), search.as_deref())
}

/// Agenda: completa o cancela una consulta pendiente.
#[tauri::command]
#[specta::specta]
pub fn set_consultation_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<ConsultationListItem, AppError> {
    let mut pooled = state.pool.acquire()?;
    history_repo::set_consultation_status(pooled.conn(), id, &status)
}
