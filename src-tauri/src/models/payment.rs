use serde::{Deserialize, Serialize};
use specta::Type;

/// Método de pago configurable (efectivo, transferencia, tarjeta…).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    pub code: String,
    pub name: String,
    /// EFECTIVO | ELECTRONICO | OTRO
    pub kind: String,
    pub requires_reference: bool,
    pub active: bool,
    pub sort_order: i32,
}

/// Línea de una recepción: un método y su importe.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaymentLine {
    pub id: i32,
    pub method_code: String,
    pub method_name: String,
    /// EFECTIVO | ELECTRONICO | OTRO
    pub method_kind: String,
    pub amount: f64,
    pub reference: Option<String>,
}

/// Aplicación de (parte de) una recepción a una factura.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAllocation {
    pub id: i32,
    pub invoice_id: i32,
    pub invoice_number: String,
    pub amount: f64,
}

/// Recepción de pago completa, con líneas y facturas aplicadas.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: i32,
    /// REC-000001…
    pub code: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub patient_id: Option<i32>,
    pub patient_name: Option<String>,
    pub cash_session_id: Option<i32>,
    /// YYYY-MM-DD HH:MM:SS
    pub received_at: String,
    pub total: f64,
    /// APLICADO | ANULADO
    pub status: String,
    pub notes: Option<String>,
    pub created_by_name: Option<String>,
    pub lines: Vec<PaymentLine>,
    pub allocations: Vec<PaymentAllocation>,
}

/// Saldo derivado de una factura (no se almacena en INVOICES).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceBalance {
    pub invoice_id: i32,
    pub invoice_number: String,
    pub total: f64,
    pub paid_amount: f64,
    pub balance: f64,
    /// PENDIENTE | ABONADA | PAGADA | ANULADA
    pub payment_status: String,
    /// Estado histórico de la factura (EMITIDA | PAGADA | ANULADA).
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentLineInput {
    pub method_code: String,
    pub amount: f64,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentAllocationInput {
    pub invoice_id: i32,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPaymentInput {
    pub owner_id: i32,
    pub patient_id: Option<i32>,
    pub notes: Option<String>,
    pub lines: Vec<CreatePaymentLineInput>,
    pub allocations: Vec<CreatePaymentAllocationInput>,
}
