use serde::{Deserialize, Serialize};
use specta::Type;

/// Vista previa de un archivo CSV de analizador: encabezados, primeras filas
/// y la sugerencia automática de mapeo (columna → analito).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub file_name: String,
    pub delimiter: String,
    pub headers: Vec<String>,
    /// Primeras filas de datos (máx. 5) para que la UI muestre una previsualización.
    pub sample_rows: Vec<Vec<String>>,
    /// Índice de la columna que parece contener el código de muestra, o null.
    pub suggested_sample_code_column: Option<i32>,
    /// Sugerencia por columna: analito coincidente por nombre, o null.
    pub suggested_analytes: Vec<Option<i32>>,
    pub total_rows: i32,
}

/// Mapeo confirmado por el usuario para la importación.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerImportMapping {
    /// Índice (0-based) de la columna con el código de la muestra.
    pub sample_code_column: i32,
    /// Mapeos columna → analito.
    pub columns: Vec<ImportColumnMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportColumnMapping {
    pub column_index: i32,
    pub analyte_id: i32,
}

/// Resultado de la importación: filas procesadas y omisiones con motivo.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    /// Nº de muestras a las que se les cargó al menos un resultado.
    pub samples_updated: i32,
    /// Nº total de resultados insertados/actualizados.
    pub results_imported: i32,
    pub skipped: Vec<ImportSkip>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSkip {
    /// Nº de fila (1-based, sin contar encabezados) que se omitió.
    pub row: i32,
    pub reason: String,
}
