use tauri::State;

use crate::auth::{require_receive, require_session};
use crate::error::AppError;
use crate::models::appointment::{
    Appointment, AppointmentListItem, CreateAppointmentInput, UpdateAppointmentStatusInput,
};
use crate::repositories::appointments as appt_repo;
use crate::state::AppState;

/// Registra en la auditoría (best-effort).
fn log_appt_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut pooled) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            pooled.conn(),
            user_id,
            username,
            action,
            Some(detail),
        )
        .ok();
    }
}

/// Lista la agenda de una ventana de fechas (`from`/`to` inclusive,
/// YYYY-MM-DD) con filtros opcionales de estado y profesional.
#[tauri::command]
#[specta::specta]
pub fn list_appointments(
    state: State<'_, AppState>,
    from: String,
    to: String,
    status: Option<String>,
    professional_id: Option<i32>,
) -> Result<Vec<AppointmentListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    appt_repo::list_appointments(
        pooled.conn(),
        &from,
        &to,
        status.as_deref(),
        professional_id,
    )
}

/// Ficha completa de una cita.
#[tauri::command]
#[specta::specta]
pub fn get_appointment(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<Appointment>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    appt_repo::get_appointment(pooled.conn(), id)
}

/// Crea una cita. Valida solapamiento de horarios.
#[tauri::command]
#[specta::specta]
pub fn create_appointment(
    state: State<'_, AppState>,
    input: CreateAppointmentInput,
) -> Result<Appointment, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let appt = appt_repo::create_appointment(pooled.conn(), &input)?;

    log_appt_audit(
        &state,
        Some(user.id),
        &user.username,
        "APPOINTMENT_CREATE",
        &format!(
            "Cita {} · paciente {} · servicio {} · {}",
            appt.code, appt.patient_name, appt.service_name, appt.scheduled_at
        ),
    );
    Ok(appt)
}

/// Cambia el estado de una cita (máquina de estados).
#[tauri::command]
#[specta::specta]
pub fn set_appointment_status(
    state: State<'_, AppState>,
    id: i32,
    input: UpdateAppointmentStatusInput,
) -> Result<Appointment, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let appt = appt_repo::set_appointment_status(
        pooled.conn(),
        id,
        &input.status,
        input.notes.as_deref(),
    )?;

    log_appt_audit(
        &state,
        Some(user.id),
        &user.username,
        "APPOINTMENT_STATUS",
        &format!("Cita {} → {}", appt.code, input.status),
    );
    Ok(appt)
}

/// Anula una cita (CANCELADA).
#[tauri::command]
#[specta::specta]
pub fn cancel_appointment(state: State<'_, AppState>, id: i32) -> Result<Appointment, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let appt = appt_repo::cancel_appointment(pooled.conn(), id)?;

    log_appt_audit(
        &state,
        Some(user.id),
        &user.username,
        "APPOINTMENT_CANCEL",
        &format!("Cita {} anulada", appt.code),
    );
    Ok(appt)
}

/// Check-in: convierte la cita en visita (crea VISITA con APPOINTMENT_ID y
/// la cita pasa a ATENDIDA).
#[tauri::command]
#[specta::specta]
pub fn check_in_appointment(
    state: State<'_, AppState>,
    id: i32,
) -> Result<crate::models::visit::Visit, AppError> {
    let user = require_receive(&state)?;
    let mut pooled = state.pool.acquire()?;
    let visit = appt_repo::check_in_appointment(pooled.conn(), id, Some(user.id))?;

    log_appt_audit(
        &state,
        Some(user.id),
        &user.username,
        "APPOINTMENT_CHECK_IN",
        &format!(
            "Cita → visita {} · paciente {}",
            visit.code, visit.patient_name
        ),
    );
    Ok(visit)
}
