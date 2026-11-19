use tauri::State;

use crate::auth::{require_bill, require_session};
use crate::error::AppError;
use crate::models::payment::{InvoiceBalance, Payment, PaymentMethod, RegisterPaymentInput};
use crate::repositories::payments as payments_repo;
use crate::state::AppState;

/// Catálogo de métodos de pago (para el diálogo de cobro).
#[tauri::command]
#[specta::specta]
pub fn list_payment_methods(
    state: State<'_, AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<PaymentMethod>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    payments_repo::list_methods(pooled.conn(), include_inactive.unwrap_or(false))
}

/// Registra una recepción (abono o pago total) con formas de pago mixtas.
#[tauri::command]
#[specta::specta]
pub fn register_payment(
    state: State<'_, AppState>,
    input: RegisterPaymentInput,
) -> Result<Payment, AppError> {
    let user = require_bill(&state)?;
    let mut pooled = state.pool.acquire()?;
    let payment = payments_repo::register(pooled.conn(), &input, Some(user.id))?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "PAYMENT_REGISTERED",
            Some(&format!(
                "Pago {} por {:.2} aplicado a {} factura(s)",
                payment.code,
                payment.total,
                payment.allocations.len()
            )),
        )
        .ok();
    }

    Ok(payment)
}

/// Saldo derivado de una factura (total, abonado, saldo y estado de cobro).
#[tauri::command]
#[specta::specta]
pub fn get_invoice_balance(
    state: State<'_, AppState>,
    invoice_id: i32,
) -> Result<InvoiceBalance, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    payments_repo::invoice_balance(pooled.conn(), invoice_id)
}

/// Historial de pagos aplicados a una factura.
#[tauri::command]
#[specta::specta]
pub fn list_invoice_payments(
    state: State<'_, AppState>,
    invoice_id: i32,
) -> Result<Vec<Payment>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    payments_repo::list_by_invoice(pooled.conn(), invoice_id)
}
