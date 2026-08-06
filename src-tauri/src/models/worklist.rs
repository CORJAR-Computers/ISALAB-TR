use serde::{Deserialize, Serialize};
use specta::Type;

/// Muestra pendiente en la bandeja de trabajo con el tiempo transcurrido
/// desde su recepción (para priorizar qué procesar primero).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorklistSample {
    pub id: i32,
    /// Código único de trazabilidad (M-YYYY-NNNN)
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_name: String,
    pub species_name: String,
    pub sample_type_id: i32,
    pub sample_type_name: String,
    /// RECIBIDA | EN_PROCESO
    pub status: String,
    pub received_at: String,
    /// Minutos transcurridos desde la recepción (para ordenar por urgencia).
    /// `i32` (no i64): Specta prohíbe exportar tipos BigInt a TypeScript.
    pub elapsed_minutes: i32,
    /// Nº de resultados cargados (progreso del procesamiento).
    pub result_count: i32,
    /// Nº de resultados fuera de rango (ALTO/BAJO) — alerta visual.
    pub abnormal_count: i32,
}

/// Grupo de la bandeja: un tipo de muestra con sus pendientes ordenados por
/// antigüedad (los más antiguos primero).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorklistGroup {
    pub sample_type_id: i32,
    pub sample_type_name: String,
    /// Total de muestras pendientes del grupo.
    pub count: i32,
    /// Máximo tiempo transcurrido del grupo (para ordenar los grupos por urgencia).
    /// `i32` (no i64): Specta prohíbe exportar tipos BigInt a TypeScript.
    pub max_elapsed_minutes: i32,
    pub samples: Vec<WorklistSample>,
}

/// Bandeja de trabajo diaria del laboratorio: muestras pendientes agrupadas
/// por tipo, separando las recibidas hoy de las de días anteriores.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorklistData {
    /// Fecha de la bandeja (YYYY-MM-DD, hora local del equipo).
    pub date: String,
    /// Total de muestras pendientes (hoy + días anteriores).
    pub total_pending: i32,
    /// Pendientes recibidos hoy, agrupados por tipo de muestra.
    pub today: Vec<WorklistGroup>,
    /// Pendientes recibidos antes de hoy (requieren atención), agrupados igual.
    pub overdue: Vec<WorklistGroup>,
}
