use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::lab_order::{
    AccessionOrderInput, CreateLabOrderInput, LabOrder, LabOrderListItem,
};
use crate::models::sample::Sample;
use crate::models::status_count::StatusCount;
use crate::repositories::auth as auth_repo;
use crate::repositories::lab_orders as orders_repo;
use crate::state::AppState;

/// Crea una orden de laboratorio (pruebas solicitadas por el veterinario).
#[tauri::command]
#[specta::specta]
pub fn create_lab_order(
    state: State<'_, AppState>,
    input: CreateLabOrderInput,
) -> Result<LabOrder, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let requested_by = input
        .requested_by
        .clone()
        .unwrap_or_else(|| user.username.clone());
    let mut input = input;
    input.requested_by = Some(requested_by);
    let order = orders_repo::create(pooled.conn(), &input)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "LAB_ORDER_CREATED",
            Some(&format!("Orden {} creada", order.code)),
        )
        .ok();
    }
    Ok(order)
}

/// Listado de órdenes de laboratorio con filtros opcionales.
#[tauri::command]
#[specta::specta]
pub fn list_lab_orders(
    state: State<'_, AppState>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<LabOrderListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    orders_repo::list(pooled.conn(), status.as_deref(), search.as_deref())
}

/// Órdenes de un paciente (para el historial clínico).
#[tauri::command]
#[specta::specta]
pub fn list_patient_lab_orders(
    state: State<'_, AppState>,
    patient_id: i32,
) -> Result<Vec<LabOrderListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    orders_repo::list_for_patient(pooled.conn(), patient_id)
}

/// Detalle de una orden (pruebas + muestras accesionadas).
#[tauri::command]
#[specta::specta]
pub fn get_lab_order(state: State<'_, AppState>, id: i32) -> Result<Option<LabOrder>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    orders_repo::get(pooled.conn(), id)
}

/// Conteo de órdenes por estado (pestañas del listado).
#[tauri::command]
#[specta::specta]
pub fn count_lab_orders(state: State<'_, AppState>) -> Result<Vec<StatusCount>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    Ok(orders_repo::count_by_status(pooled.conn())?
        .into_iter()
        .map(|(status, count)| StatusCount { status, count })
        .collect())
}

/// Cambia el estado de la orden (RECIBIDA/EN_PROCESO/COMPLETADA/ANULADA).
#[tauri::command]
#[specta::specta]
pub fn set_lab_order_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<LabOrder, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let order = orders_repo::set_status(pooled.conn(), id, &status)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "LAB_ORDER_STATUS",
            Some(&format!("Orden {} → {}", order.code, status)),
        )
        .ok();
    }
    Ok(order)
}

/// Accesiona la orden: crea una muestra ligada (tubo del tipo indicado).
#[tauri::command]
#[specta::specta]
pub fn accession_lab_order(
    state: State<'_, AppState>,
    input: AccessionOrderInput,
) -> Result<Sample, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let sample = orders_repo::accession(pooled.conn(), &input)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "LAB_ORDER_ACCESSIONED",
            Some(&format!(
                "Orden {} → muestra {}",
                input.order_id, sample.code
            )),
        )
        .ok();
    }
    Ok(sample)
}

/// Orden de la que proviene una muestra (para precargar paneles en la ficha).
#[tauri::command]
#[specta::specta]
pub fn get_order_for_sample(
    state: State<'_, AppState>,
    sample_id: i32,
) -> Result<Option<LabOrder>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    orders_repo::get_for_sample(pooled.conn(), sample_id)
}
