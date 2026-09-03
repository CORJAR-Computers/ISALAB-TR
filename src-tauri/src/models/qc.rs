use serde::{Deserialize, Serialize};
use specta::Type;

/// Material de control (nivel/lote) evaluado en un equipo.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcControlMaterial {
    pub id: i32,
    pub name: String,
    pub analyzer_id: i32,
    pub analyzer_name: String,
    pub lot: Option<String>,
    pub expires_at: Option<String>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub target_count: i32,
}

/// Valor objetivo (media/desviación) de un analito para un material de control.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcTarget {
    pub id: i32,
    pub control_material_id: i32,
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub mean: f64,
    pub sd: f64,
}

/// Entrada para crear/actualizar un material de control con sus objetivos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcMaterialInput {
    pub id: Option<i32>,
    pub name: String,
    pub analyzer_id: i32,
    pub lot: Option<String>,
    pub expires_at: Option<String>,
    pub notes: Option<String>,
    pub targets: Vec<QcTargetInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcTargetInput {
    pub analyte_id: i32,
    pub mean: f64,
    pub sd: f64,
}

/// Corrida de control completa (una medición por analito objetivo).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcRun {
    pub id: i32,
    pub control_material_id: i32,
    pub control_name: String,
    pub analyzer_id: i32,
    pub analyzer_name: String,
    pub run_date: String,
    /// ACEPTADO | RECHAZADO (según las reglas multirregla de Westgard)
    pub status: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub measurements: Vec<QcRunMeasurement>,
}

/// Medición de un analito dentro de una corrida de control.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcRunMeasurement {
    pub id: i32,
    pub qc_run_id: i32,
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub value: f64,
    pub z_score: Option<f64>,
    /// Reglas Westgard violadas (ej. "1_3s", "2_2s"), separadas por coma.
    pub violation: Option<String>,
}

/// Entrada para registrar una corrida de control.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcRunInput {
    pub control_material_id: i32,
    pub notes: Option<String>,
    pub measurements: Vec<QcMeasurementInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcMeasurementInput {
    pub analyte_id: i32,
    pub value: f64,
}

/// Punto del gráfico Levey-Jennings de un analito.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcChartPoint {
    pub run_id: i32,
    pub run_date: String,
    pub value: f64,
    pub z_score: f64,
    pub violation: Option<String>,
}

/// Datos para el gráfico Levey-Jennings: objetivo, bandas ±1/2/3 SD y puntos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcChartData {
    pub control_material_id: i32,
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub mean: f64,
    pub sd: f64,
    pub points: Vec<QcChartPoint>,
}

/// Estado QC del último corrida por analizador (badge de alerta en la UI).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct QcAnalyzerStatus {
    pub analyzer_id: i32,
    /// "ACEPTADO" | "RECHAZADO" | null si no hay corridas registradas.
    pub latest_status: Option<String>,
}
