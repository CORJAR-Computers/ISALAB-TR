use rsfbclient::prelude::*;
use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::species::{Analyte, Breed, SampleType, Species, VaccineType};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn list_species(state: State<'_, AppState>) -> Result<Vec<Species>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<(i32, String, String)> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM SPECIES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Species {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_breeds(
    state: State<'_, AppState>,
    species_id: i32,
) -> Result<Vec<Breed>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<(i32, i32, String)> = pooled
        .conn()
        .query(
            "SELECT ID, SPECIES_ID, NAME FROM BREEDS
             WHERE SPECIES_ID = ? AND IS_ACTIVE = TRUE ORDER BY NAME",
            (&species_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Breed {
            id: r.0,
            species_id: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_sample_types(state: State<'_, AppState>) -> Result<Vec<SampleType>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<(i32, String, String)> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM SAMPLE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| SampleType {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_analytes(state: State<'_, AppState>) -> Result<Vec<Analyte>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<(i32, String, String, Option<String>, Option<String>)> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME, UNIT, METHOD FROM ANALYTES
             WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Analyte {
            id: r.0,
            code: r.1,
            name: r.2,
            unit: r.3,
            method: r.4,
        })
        .collect())
}

/// Catálogo de vacunas del esquema (Rabia, Polivalente, FeLV…).
#[tauri::command]
#[specta::specta]
pub fn list_vaccine_types(state: State<'_, AppState>) -> Result<Vec<VaccineType>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<(i32, String, String)> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM VACCINE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| VaccineType {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}
