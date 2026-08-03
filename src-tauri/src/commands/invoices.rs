use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::invoice::{CreateInvoiceInput, Invoice, InvoiceListItem};
use crate::repositories::invoices as invoices_repo;
use crate::state::AppState;

/// Emite una factura con items; calcula subtotal, IVA y total en Rust.
#[tauri::command]
#[specta::specta]
pub fn create_invoice(
    state: State<'_, AppState>,
    input: CreateInvoiceInput,
) -> Result<Invoice, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    invoices_repo::create(pooled.conn(), &input)
}

/// Listado de facturas con filtros por estado y búsqueda.
#[tauri::command]
#[specta::specta]
pub fn list_invoices(
    state: State<'_, AppState>,
    status: Option<String>,
    search: Option<String>,
) -> Result<Vec<InvoiceListItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    invoices_repo::list(pooled.conn(), status.as_deref(), search.as_deref())
}

/// Ficha completa de una factura (con items).
#[tauri::command]
#[specta::specta]
pub fn get_invoice(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<Invoice>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    invoices_repo::get(pooled.conn(), id)
}

/// Cambia el estado de una factura (EMITIDA→PAGADA/ANULADA, PAGADA→ANULADA).
#[tauri::command]
#[specta::specta]
pub fn set_invoice_status(
    state: State<'_, AppState>,
    id: i32,
    status: String,
) -> Result<Invoice, AppError> {
    let user = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let invoice = invoices_repo::set_status(pooled.conn(), id, &status)?;

    // Auditoría de transición de estado de factura.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "INVOICE_STATUS_CHANGE",
            Some(&format!("Factura {} → estado {}", id, status)),
        )
        .ok();
    }

    Ok(invoice)
}
