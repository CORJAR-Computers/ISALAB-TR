use serde::{Deserialize, Serialize};
use specta::Type;

/// Panel de analitos que se cargan juntos en una corrida (p. ej. Hemograma
/// completo). SAMPLE_TYPE_ID NULL = panel genérico para cualquier muestra.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: i32,
    pub name: String,
    pub sample_type_id: Option<i32>,
    pub sample_type_name: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub notes: Option<String>,
    pub analyte_count: i32,
}

/// Analito que compone un panel, con el orden de carga.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelAnalyte {
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub seq: i32,
}

/// Crear o actualizar un panel (si `id` es Some) reemplazando sus analitos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelInput {
    pub id: Option<i32>,
    pub name: String,
    pub sample_type_id: Option<i32>,
    pub sort_order: i32,
    pub notes: Option<String>,
    pub analyte_ids: Vec<i32>,
}
