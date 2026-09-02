use serde::{Deserialize, Serialize};
use specta::Type;

/// Fila de la "mesa de trabajo" del laboratorio: una muestra con los datos del
/// paciente/propietario unidos (vista global, no por paciente).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SampleListItem {
    pub id: i32,
    /// Código único de trazabilidad (M-YYYY-NNNN)
    pub code: String,
    pub patient_id: i32,
    pub patient_code: String,
    pub patient_name: String,
    pub owner_name: String,
    pub species_name: String,
    pub sample_type_id: i32,
    pub sample_type_name: String,
    pub received_at: String,
    /// RECIBIDA | EN_PROCESO | FINALIZADA | ANULADA
    pub status: String,
    pub collected_by: Option<String>,
    pub notes: Option<String>,
    /// Nº de resultados cargados (para el badge de progreso).
    pub result_count: i32,
    /// Nº de resultados fuera de rango (ALTO/BAJO) — alerta visual.
    pub abnormal_count: i32,
}
