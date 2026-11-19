use serde::{Deserialize, Serialize};
use specta::Type;

/// Sesión de caja (apertura/cierre). Solo puede haber una ABIERTA.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashSession {
    pub id: i32,
    /// CAJ-00000001…
    pub code: String,
    /// YYYY-MM-DD HH:MM:SS
    pub opened_at: String,
    pub opened_by: i32,
    pub opened_by_name: Option<String>,
    pub opening_float: f64,
    /// ABIERTA | CERRADA
    pub status: String,
    pub closed_at: Option<String>,
    pub closed_by_name: Option<String>,
    pub expected_cash: Option<f64>,
    pub counted_cash: Option<f64>,
    pub difference: Option<f64>,
    pub total_billed: Option<f64>,
    pub total_collected: Option<f64>,
    pub notes: Option<String>,
}

/// Movimiento manual o automático de caja.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashMovement {
    pub id: i32,
    pub cash_session_id: i32,
    /// PAGO_EFECTIVO | GASTO | RETIRO | DEVOLUCION | AJUSTE
    pub kind: String,
    /// Firmado: los ingresos son positivos y los egresos negativos.
    pub amount: f64,
    pub payment_id: Option<i32>,
    pub reason: Option<String>,
    pub created_by_name: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub created_at: String,
}

/// Total cobrado por método de pago dentro de una sesión.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashMethodTotal {
    pub method_code: String,
    pub method_name: String,
    pub amount: f64,
}

/// Línea de arqueo por denominación.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashCountLine {
    pub denomination: f64,
    pub quantity: i32,
    pub subtotal: f64,
}

/// Resumen completo de una sesión: cifras, movimientos y arqueo.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashSessionSummary {
    pub session: CashSession,
    pub opening_float: f64,
    pub expected_cash: f64,
    pub counted_cash: Option<f64>,
    pub difference: Option<f64>,
    pub total_billed: f64,
    pub total_collected: f64,
    /// Suma de ingresos de caja (pagos en efectivo y ajustes positivos).
    pub cash_in: f64,
    /// Suma de egresos de caja (gastos, retiros, devoluciones y ajustes negativos).
    pub cash_out: f64,
    pub payments_count: i32,
    pub methods: Vec<CashMethodTotal>,
    pub movements: Vec<CashMovement>,
    pub count_lines: Vec<CashCountLine>,
}

/// Movimiento manual: gasto, retiro o ajuste con motivo obligatorio.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddCashMovementInput {
    /// GASTO | RETIRO | AJUSTE
    pub kind: String,
    /// Siempre positivo; el backend aplica el signo según el tipo.
    pub amount: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CashCountInput {
    pub denomination: f64,
    pub quantity: i32,
}

/// Cierre de caja: arqueo por denominación y notas opcionales.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CloseCashInput {
    pub count_lines: Vec<CashCountInput>,
    pub notes: Option<String>,
}
