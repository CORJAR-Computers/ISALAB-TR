use tauri::State;

use crate::auth::{require_admin, require_session, require_stay};
use crate::error::AppError;
use crate::models::stay::{
    CheckOutStayInput, CreateStayEventInput, CreateStayInput, Resource, ResourceBoardItem, Stay,
    StayEvent, StayListItem, SaveResourceInput,
};
use crate::repositories::auth as auth_repo;
use crate::repositories::stays as stays_repo;
use crate::state::AppState;

fn log_stay_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut pooled) = state.pool.acquire() {
        auth_repo::log_audit(pooled.conn(), user_id, username, action, Some(detail)).ok();
    }
}

/// Espacios físicos de la clínica (caniles, gateras, boxes…).
#[tauri::command]
#[specta::specta]
pub fn list_resources(
    state: State<'_, AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<Resource>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    stays_repo::list_resources(pooled.conn(), include_inactive.unwrap_or(false))
}

/// Tablero de estancias: recursos con su ocupación actual.
#[tauri::command]
#[specta::specta]
pub fn list_resource_board(
    state: State<'_, AppState>,
) -> Result<Vec<ResourceBoardItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    stays_repo::list_resource_board(pooled.conn())
}

/// Crea o edita un recurso (solo ADMIN).
#[tauri::command]
#[specta::specta]
pub fn save_resource(
    state: State<'_, AppState>,
    input: SaveResourceInput,
) -> Result<Resource, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let resource = stays_repo::save_resource(pooled.conn(), &input)?;

    log_stay_audit(
        &state,
        Some(user.id),
        &user.username,
        "RESOURCE_SAVE",
        &format!("Recurso {} · {}", resource.id, resource.name),
    );
    Ok(resource)
}

/// Lista estancias con filtros opcionales (estado, tipo, paciente).
#[tauri::command]
#[specta::specta]
pub fn list_stays(
    state: State<'_, AppState>,
    status: Option<String>,
    kind: Option<String>,
    patient_id: Option<i32>,
) -> Result<Vec<StayListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    stays_repo::list_stays(
        pooled.conn(),
        status.as_deref(),
        kind.as_deref(),
        patient_id,
    )
}

/// Estancia completa por id.
#[tauri::command]
#[specta::specta]
pub fn get_stay(state: State<'_, AppState>, id: i32) -> Result<Option<Stay>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    stays_repo::get_stay(pooled.conn(), id)
}

/// Ingreso de una estancia (guardería, hospedaje u hospitalización).
#[tauri::command]
#[specta::specta]
pub fn create_stay(
    state: State<'_, AppState>,
    input: CreateStayInput,
) -> Result<Stay, AppError> {
    let user = require_stay(&state)?;
    let mut pooled = state.pool.acquire()?;
    let stay = stays_repo::create_stay(pooled.conn(), &input, Some(user.id))?;

    log_stay_audit(
        &state,
        Some(user.id),
        &user.username,
        "STAY_CHECK_IN",
        &format!(
            "Estancia {} · paciente {} · {}",
            stay.code, stay.patient_name, stay.kind
        ),
    );
    Ok(stay)
}

/// Check-out de una estancia.
#[tauri::command]
#[specta::specta]
pub fn check_out_stay(
    state: State<'_, AppState>,
    id: i32,
    input: CheckOutStayInput,
) -> Result<Stay, AppError> {
    let user = require_stay(&state)?;
    let mut pooled = state.pool.acquire()?;
    let stay = stays_repo::check_out_stay(pooled.conn(), id, &input)?;

    log_stay_audit(
        &state,
        Some(user.id),
        &user.username,
        "STAY_CHECK_OUT",
        &format!(
            "Estancia {} · paciente {} · {} día(s)",
            stay.code, stay.patient_name, stay.days
        ),
    );
    Ok(stay)
}

/// Cancela una estancia abierta.
#[tauri::command]
#[specta::specta]
pub fn cancel_stay(state: State<'_, AppState>, id: i32) -> Result<Stay, AppError> {
    let user = require_stay(&state)?;
    let mut pooled = state.pool.acquire()?;
    let stay = stays_repo::cancel_stay(pooled.conn(), id)?;

    log_stay_audit(
        &state,
        Some(user.id),
        &user.username,
        "STAY_CANCELLED",
        &format!("Estancia {} · paciente {}", stay.code, stay.patient_name),
    );
    Ok(stay)
}

/// Añade un evento a la bitácora de cuidados de la estancia.
#[tauri::command]
#[specta::specta]
pub fn add_stay_event(
    state: State<'_, AppState>,
    stay_id: i32,
    input: CreateStayEventInput,
) -> Result<StayEvent, AppError> {
    let user = require_stay(&state)?;
    let mut pooled = state.pool.acquire()?;
    let event = stays_repo::add_stay_event(pooled.conn(), stay_id, &input, Some(user.id))?;

    log_stay_audit(
        &state,
        Some(user.id),
        &user.username,
        "STAY_EVENT_ADDED",
        &format!("Estancia {stay_id} · {}", event.kind),
    );
    Ok(event)
}

/// Bitácora de cuidados de una estancia.
#[tauri::command]
#[specta::specta]
pub fn list_stay_events(
    state: State<'_, AppState>,
    stay_id: i32,
) -> Result<Vec<StayEvent>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    stays_repo::list_stay_events(pooled.conn(), stay_id)
}
