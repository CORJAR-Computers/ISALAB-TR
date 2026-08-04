use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::owner::Owner;
use crate::models::patient::{CreatePatientInput, Patient};
use crate::repositories::patient as patient_repo;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_patients(
    state: State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<Patient>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    patient_repo::list(pooled.conn(), search.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn get_patient(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<Patient>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    patient_repo::get(pooled.conn(), id)
}

#[tauri::command]
#[specta::specta]
pub fn create_patient(
    state: State<'_, AppState>,
    input: CreatePatientInput,
) -> Result<Patient, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    patient_repo::create(pooled.conn(), &input)
}

/// Listado de propietarios (para facturación y búsquedas).
#[tauri::command]
#[specta::specta]
pub fn list_owners(
    state: State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<Owner>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    patient_repo::list_owners(pooled.conn(), search.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn get_patient_lab_trends(
    state: State<'_, AppState>,
    patient_id: i32,
    analyte_id: i32,
) -> Result<Vec<crate::models::sample::TrendPoint>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    crate::repositories::samples::get_patient_lab_trends(pooled.conn(), patient_id, analyte_id)
}
