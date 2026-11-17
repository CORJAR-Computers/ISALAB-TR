use tauri::State;

use crate::auth::{require_bill, require_receive, require_session};
use crate::error::AppError;
use crate::models::invoice::Invoice;
use crate::models::visit::{
    AddVisitServiceInput, CreateVisitInput, UpdateVisitServiceStatusInput, Visit, VisitListItem,
    VisitService,
};
use crate::repositories::visits as visits_repo;
use crate::state::AppState;

/// Registra en la auditoría (best-effort: no bloquea la operación).
fn log_visit_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut pooled) = state.pool.acquire() {
        crate::repositories::auth::log_audit(pooled.conn(), user_id, username, action, Some(detail))
            .ok();
    }
}

/// Lista la cola de recepción con filtros: fecha (YYYY-MM-DD), estado y
/// búsqueda libre. Urgentes primero.
#[tauri::command]
#[specta::specta]
pub fn list_visits(
    state: State<'_, AppState>,
    date: Option<String>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<VisitListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    visits_repo::list_visits(
        pooled.conn(),
        date.as_deref(),
        status.as_deref(),
        search.as_deref(),
    )
}

/// Ficha completa de una visita (con sus líneas de servicio).
#[tauri::command]
#[specta::specta]
pub fn get_visit(state: State<'_, AppState>, id: i32) -> Result<Option<Visit>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    visits_repo::get_visit(pooled.conn(), id)
}

/// Check-in de un paciente: crea la visita con sus servicios iniciales
/// (precios congelados del catálogo en este instante).
#[tauri::command]
#[specta::specta]
pub fn create_visit(
    state: State<'_, AppState>,
    input: CreateVisitInput,
) -> Result<Visit, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let visit = visits_repo::create_visit(pooled.conn(), &input, Some(user.id), None)?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_CREATE",
        &format!(
            "Visita {} · paciente {} · {} servicio(s)",
            visit.code,
            visit.patient_name,
            visit.services.len()
        ),
    );
    Ok(visit)
}

/// Cambia el estado de la visita (EN_ESPERA → EN_ATENCION → FINALIZADA;
/// CANCELADA posible desde estados no facturados).
#[tauri::command]
#[specta::specta]
pub fn set_visit_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<Visit, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let visit = visits_repo::set_visit_status(pooled.conn(), id, &status)?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_STATUS",
        &format!("Visita {} → {}", visit.code, status),
    );
    Ok(visit)
}

/// Agrega un servicio a la visita (snapshot de precio del catálogo).
#[tauri::command]
#[specta::specta]
pub fn add_visit_service(
    state: State<'_, AppState>,
    visit_id: i32,
    input: AddVisitServiceInput,
) -> Result<VisitService, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let line = visits_repo::add_visit_service(pooled.conn(), visit_id, &input)?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_SERVICE_ADD",
        &format!(
            "Visita {visit_id} · servicio {} x{} · precio congelado {}",
            line.service_code, line.quantity, line.unit_price
        ),
    );
    Ok(line)
}

/// Quita una línea PENDIENTE de la visita.
#[tauri::command]
#[specta::specta]
pub fn remove_visit_service(
    state: State<'_, AppState>,
    visit_id: i32,
    line_id: i32,
) -> Result<(), AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    visits_repo::remove_visit_service(pooled.conn(), visit_id, line_id)?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_SERVICE_REMOVE",
        &format!("Visita {visit_id} · línea {line_id} eliminada"),
    );
    Ok(())
}

/// Cambia el estado de una línea (PENDIENTE → EN_PROCESO → COMPLETADA/CANCELADA).
/// El peluquero/veterinario usa esto desde su bandeja.
#[tauri::command]
#[specta::specta]
pub fn update_visit_service_status(
    state: State<'_, AppState>,
    visit_id: i32,
    line_id: i32,
    input: UpdateVisitServiceStatusInput,
) -> Result<VisitService, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let line = visits_repo::update_visit_service_status(
        pooled.conn(),
        visit_id,
        line_id,
        &input,
    )?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_SERVICE_STATUS",
        &format!(
            "Visita {visit_id} · línea {line_id} ({}) → {}",
            line.service_code, input.status
        ),
    );
    Ok(line)
}

/// Cobra la visita: genera la factura desde las líneas con precio congelado
/// y marca la visita FACTURADA. Una sola factura por visita.
#[tauri::command]
#[specta::specta]
pub fn checkout_visit(
    state: State<'_, AppState>,
    visit_id: i32,
    payment_method: Option<String>,
    notes: Option<String>,
) -> Result<Invoice, AppError> {
    let user = require_bill(&state)?;
    let mut pooled = state.pool.acquire()?;
    let invoice = visits_repo::checkout_visit(
        pooled.conn(),
        visit_id,
        payment_method.as_deref(),
        notes.as_deref(),
    )?;

    log_visit_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_CHECKOUT",
        &format!(
            "Visita {visit_id} facturada como {} · total {}",
            invoice.invoice_number, invoice.total
        ),
    );
    Ok(invoice)
}
