use serde::{Deserialize, Serialize};
use specta::Type;

/// Archivo adjunto de un resultado (foto de placa, frotis o electroforesis).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResultAttachment {
    pub id: i32,
    pub result_id: i32,
    /// Nombre original del archivo (para mostrarlo en la UI).
    pub file_name: String,
    /// Ruta persistida en la carpeta de datos de la app (app_data/attachments).
    pub file_path: String,
    pub mime_type: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabResult {
    pub id: i32,
    pub sample_id: i32,
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub value: f64,
    /// BAJO | NORMAL | ALTO | SIN_RANGO (calculado por SP_VALIDATE_ANALYTICAL_RESULT)
    pub status: String,
    pub ref_min: Option<f64>,
    pub ref_max: Option<f64>,
    pub analyzed_at: Option<String>,
    /// Evidencias adjuntas (placas, frotis, electroforesis) para soporte del diagnóstico.
    pub attachments: Vec<ResultAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub date: String,
    pub value: f64,
    pub ref_min: Option<f64>,
    pub ref_max: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub id: i32,
    /// Código único de trazabilidad (M-YYYY-NNNN)
    pub code: String,
    pub patient_id: i32,
    pub sample_type_id: i32,
    pub sample_type_name: String,
    pub received_at: String,
    /// RECIBIDA | EN_PROCESO | FINALIZADA | ANULADA
    pub status: String,
    pub collected_by: Option<String>,
    pub notes: Option<String>,
    /// Equipo analizador elegido por el operario (NULL = lectura manual/estándar).
    pub analyzer_id: Option<i32>,
    /// Nombre del equipo (para la UI y el reporte PDF).
    pub analyzer_name: Option<String>,
    pub results: Vec<LabResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateSampleInput {
    pub patient_id: i32,
    pub sample_type_id: i32,
    pub received_at: String,
    pub collected_by: Option<String>,
    pub notes: Option<String>,
    /// Equipo analizador (NULL = perfil GENERAL/estándar).
    pub analyzer_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResultInput {
    pub sample_id: i32,
    pub analyte_id: i32,
    pub value: f64,
}

// ===== Payloads de los eventos Firebird → frontend =====

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SampleChangedEvent {
    pub sample_id: i32,
    pub patient_id: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabResultChangedEvent {
    pub sample_id: i32,
    pub patient_id: i32,
}
