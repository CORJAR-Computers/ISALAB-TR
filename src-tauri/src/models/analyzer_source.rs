use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::import::{AnalyzerImportMapping, ImportColumnMapping};

/// Fuente automática de resultados configurada para un analizador.
/// Hoy el único driver es la carpeta vigilada (CSV); el campo `source_type`
/// queda listo para drivers futuros (ASTM serial, HL7 por red…).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerSource {
    pub id: i32,
    pub analyzer_id: i32,
    pub analyzer_name: String,
    pub source_type: String,
    pub folder_path: Option<String>,
    pub enabled: bool,
    /// Última vez que el supervisor sondeó esta fuente (YYYY-MM-DD HH:MM:SS).
    pub last_poll_at: Option<String>,
    /// Mapeo CSV guardado (columna del código de muestra + columnas analito).
    pub mapping: Option<AnalyzerImportMapping>,
    /// Nº de analitos mapeados (atajo para la lista).
    pub mapped_columns: i32,
}

/// Guarda (crea o reemplaza) la fuente automática de un analizador.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveAnalyzerSourceInput {
    pub analyzer_id: i32,
    pub source_type: Option<String>,
    pub folder_path: Option<String>,
    pub enabled: bool,
    pub mapping: Option<AnalyzerImportMapping>,
}

/// Entrada de la cola de importación automática (un archivo detectado).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerImportJob {
    pub id: i32,
    pub source_id: i32,
    pub analyzer_id: i32,
    pub analyzer_name: String,
    pub file_name: String,
    /// IMPORTADO | FALLIDO
    pub status: String,
    pub samples_updated: i32,
    pub results_imported: i32,
    pub skipped_rows: i32,
    pub error_msg: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub processed_at: String,
}

/// Resultado de procesar un archivo CSV de la carpeta vigilada.
#[derive(Debug, Clone)]
pub struct SourceFileOutcome {
    pub file_name: String,
    pub status: String,
    pub samples_updated: i32,
    pub results_imported: i32,
    pub skipped_rows: i32,
    pub error_msg: Option<String>,
}

// Helpers de conversión usados por el repositorio y el driver.

/// Construye el mapeo a partir de la fila del código de muestra y las
/// columnas mapeadas (índice de columna → analito).
pub fn build_mapping(
    sample_code_column: Option<i32>,
    columns: &[(i32, i32)],
) -> Option<AnalyzerImportMapping> {
    let code = sample_code_column?;
    Some(AnalyzerImportMapping {
        sample_code_column: code,
        columns: columns
            .iter()
            .map(|(ci, ai)| ImportColumnMapping {
                column_index: *ci,
                analyte_id: *ai,
            })
            .collect(),
    })
}
