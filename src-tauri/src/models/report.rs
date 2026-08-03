use serde::{Deserialize, Serialize};
use specta::Type;

/// Un informe PDF ya generado (listado de la carpeta app_data/reports).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportFile {
    /// Ruta absoluta del archivo (para abrirlo con el visor del SO).
    pub path: String,
    pub file_name: String,
    /// Código de la muestra origen (derivado del nombre del archivo).
    pub sample_code: String,
    /// YYYY-MM-DD HH:MM:SS (hora local de generación).
    pub generated_at: String,
}
