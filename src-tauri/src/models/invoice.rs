use serde::{Deserialize, Serialize};
use specta::Type;

/// Línea de detalle de una factura.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceItem {
    pub id: i32,
    pub description: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
}

/// Factura completa con items y datos del propietario/paciente unidos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub id: i32,
    /// Código único (FAC-000001…)
    pub invoice_number: String,
    pub patient_id: Option<i32>,
    pub patient_name: Option<String>,
    pub owner_id: i32,
    pub owner_name: String,
    pub consultation_id: Option<i32>,
    /// YYYY-MM-DD HH:MM:SS
    pub issue_date: String,
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
    /// EMITIDA | PAGADA | ANULADA
    pub status: String,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<InvoiceItem>,
}

/// Fila del listado de facturas (sin items).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceListItem {
    pub id: i32,
    pub invoice_number: String,
    pub owner_name: String,
    pub patient_name: Option<String>,
    pub issue_date: String,
    pub total: f64,
    /// EMITIDA | PAGADA | ANULADA
    pub status: String,
    pub payment_method: Option<String>,
    pub item_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceItemInput {
    pub description: String,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceInput {
    pub patient_id: Option<i32>,
    pub owner_id: i32,
    pub consultation_id: Option<i32>,
    /// IVA por defecto (%), tomado de la configuración si no se envía.
    pub tax_rate: Option<f64>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateInvoiceItemInput>,
}
