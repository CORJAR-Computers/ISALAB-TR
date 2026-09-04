use serde::{Deserialize, Serialize};
use specta::Type;

/// Prueba solicitada en una orden: un panel o un analito suelto.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabOrderItem {
    pub id: i32,
    pub order_id: i32,
    pub panel_id: Option<i32>,
    pub panel_name: Option<String>,
    /// Tipo de muestra que implica el panel (para agrupar tubos al accesionar).
    pub panel_sample_type_id: Option<i32>,
    pub panel_sample_type_name: Option<String>,
    pub analyte_id: Option<i32>,
    pub analyte_name: Option<String>,
    pub unit: Option<String>,
    pub seq: i32,
}

/// Fila del listado de órdenes (sin items) con datos del paciente unidos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabOrderListItem {
    pub id: i32,
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub species_name: String,
    pub owner_name: String,
    pub consultation_id: Option<i32>,
    pub requested_by: Option<String>,
    pub priority: String,
    pub status: String,
    pub item_count: i32,
    /// YYYY-MM-DD HH:MM:SS
    pub requested_at: String,
}

/// Orden de laboratorio completa con sus pruebas y muestras accesionadas.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabOrder {
    pub id: i32,
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub species_name: String,
    pub owner_name: String,
    pub consultation_id: Option<i32>,
    pub requested_by: Option<String>,
    pub priority: String,
    pub status: String,
    pub notes: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub requested_at: String,
    pub items: Vec<LabOrderItem>,
    /// Muestras creadas al accesionar esta orden.
    pub samples: Vec<OrderSampleRef>,
}

/// Referencia a una muestra accesionada desde la orden (para abrirla).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OrderSampleRef {
    pub id: i32,
    pub code: String,
    pub sample_type_name: String,
    pub status: String,
}

/// Crear una orden de laboratorio.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabOrderInput {
    pub patient_id: i32,
    pub consultation_id: Option<i32>,
    pub requested_by: Option<String>,
    pub priority: String,
    pub notes: Option<String>,
    /// YYYY-MM-DD HH:MM:SS (por defecto ahora si se omite).
    pub requested_at: Option<String>,
    /// Pruebas solicitadas (panel y/o analito por ítem).
    pub items: Vec<CreateLabOrderItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateLabOrderItemInput {
    pub panel_id: Option<i32>,
    pub analyte_id: Option<i32>,
}

/// Accesionar una orden: crea una muestra (tubo) de un tipo determinado
/// ligada a la orden. Se puede llamar varias veces (un tubo por tipo).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccessionOrderInput {
    pub order_id: i32,
    pub sample_type_id: i32,
    /// YYYY-MM-DD HH:MM:SS (por defecto ahora si se omite).
    pub received_at: Option<String>,
    pub collected_by: Option<String>,
    pub notes: Option<String>,
}
