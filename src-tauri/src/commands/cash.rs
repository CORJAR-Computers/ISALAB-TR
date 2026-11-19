use tauri::State;

use crate::auth::{require_bill, require_session};
use crate::error::AppError;
use crate::models::cash::{
    AddCashMovementInput, CashMovement, CashSession, CashSessionSummary, CloseCashInput,
};
use crate::repositories::cash as cash_repo;
use crate::state::AppState;

/// Sesión de caja abierta actual, o `None` si no hay ninguna.
#[tauri::command]
#[specta::specta]
pub fn get_open_cash_session(state: State<'_, AppState>) -> Result<Option<CashSession>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    cash_repo::get_open_session(pooled.conn())
}

/// Abre la caja con su base inicial. Falla si ya hay una abierta.
#[tauri::command]
#[specta::specta]
pub fn open_cash_session(
    state: State<'_, AppState>,
    opening_float: f64,
    notes: Option<String>,
) -> Result<CashSession, AppError> {
    let user = require_bill(&state)?;
    let mut pooled = state.pool.acquire()?;
    let session = cash_repo::open_session(pooled.conn(), opening_float, notes, user.id)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "CASH_SESSION_OPENED",
            Some(&format!(
                "Caja {} abierta con base {:.2}",
                session.code, session.opening_float
            )),
        )
        .ok();
    }

    Ok(session)
}

/// Registra un gasto, retiro o ajuste en la caja abierta.
#[tauri::command]
#[specta::specta]
pub fn add_cash_movement(
    state: State<'_, AppState>,
    input: AddCashMovementInput,
) -> Result<CashMovement, AppError> {
    let user = require_bill(&state)?;
    let mut pooled = state.pool.acquire()?;
    let movement = cash_repo::add_movement(
        pooled.conn(),
        &input.kind,
        input.amount,
        &input.reason,
        Some(user.id),
    )?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "CASH_MOVEMENT_ADDED",
            Some(&format!(
                "{} de {:.2} en caja: {}",
                movement.kind,
                movement.amount.abs(),
                input.reason.trim()
            )),
        )
        .ok();
    }

    Ok(movement)
}

/// Movimientos de una sesión (por defecto, la caja abierta).
#[tauri::command]
#[specta::specta]
pub fn list_cash_movements(
    state: State<'_, AppState>,
    session_id: Option<i32>,
) -> Result<Vec<CashMovement>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let id = match session_id {
        Some(id) => id,
        None => match cash_repo::get_open_session(pooled.conn())? {
            Some(session) => session.id,
            None => return Ok(Vec::new()),
        },
    };
    cash_repo::list_movements(pooled.conn(), id)
}

/// Resumen de una sesión: cifras, métodos, movimientos y arqueo.
#[tauri::command]
#[specta::specta]
pub fn get_cash_session_summary(
    state: State<'_, AppState>,
    session_id: Option<i32>,
) -> Result<CashSessionSummary, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let id = match session_id {
        Some(id) => id,
        None => {
            cash_repo::get_open_session(pooled.conn())?
                .map(|s| s.id)
                .ok_or_else(|| {
                    AppError::Validation("No hay una caja abierta. Abre la caja primero.".into())
                })?
        }
    };
    cash_repo::summary(pooled.conn(), id)
}

/// Cierra la caja abierta con el arqueo por denominación.
#[tauri::command]
#[specta::specta]
pub fn close_cash_session(
    state: State<'_, AppState>,
    input: CloseCashInput,
) -> Result<CashSessionSummary, AppError> {
    let user = require_bill(&state)?;
    let mut pooled = state.pool.acquire()?;
    let summary = cash_repo::close_session(pooled.conn(), &input, Some(user.id))?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "CASH_SESSION_CLOSED",
            Some(&format!(
                "Caja {} cerrada: esperado {:.2}, contado {:.2}, diferencia {:.2}",
                summary.session.code,
                summary.expected_cash,
                summary.counted_cash.unwrap_or(0.0),
                summary.difference.unwrap_or(0.0)
            )),
        )
        .ok();
    }

    Ok(summary)
}

/// Historial de sesiones de caja, más reciente primero.
#[tauri::command]
#[specta::specta]
pub fn list_cash_sessions(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> Result<Vec<CashSession>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    cash_repo::list_sessions(pooled.conn(), limit.unwrap_or(30))
}
