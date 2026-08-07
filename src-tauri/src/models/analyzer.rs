use serde::{Deserialize, Serialize};
use specta::Type;

/// Equipo analizador de laboratorio (marca/modelo). El perfil GENERAL (ID 1)
/// agrupa los rangos estándar por especie/sexo/edad sin equipo automatizado.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Analyzer {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub is_active: bool,
    pub notes: Option<String>,
    /// Nº de rangos de referencia configurados para este equipo.
    pub range_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnalyzerInput {
    pub code: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnalyzerInput {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub notes: Option<String>,
}

/// Rango de referencia de un analito para un equipo y especie, con sexo y
/// franja de edad opcionales (SEX NULL = ambos sexos).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRange {
    pub id: i32,
    pub analyzer_id: i32,
    pub analyzer_name: String,
    pub analyte_id: i32,
    pub analyte_name: String,
    pub unit: Option<String>,
    pub species_id: i32,
    pub species_name: String,
    pub sex: Option<String>,
    pub age_min_months: i32,
    pub age_max_months: i32,
    pub min_value: f64,
    pub max_value: f64,
    pub critical_min: Option<f64>,
    pub critical_max: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRangeInput {
    pub analyzer_id: i32,
    pub analyte_id: i32,
    pub species_id: i32,
    pub sex: Option<String>,
    pub age_min_months: i32,
    pub age_max_months: i32,
    pub min_value: f64,
    pub max_value: f64,
    pub critical_min: Option<f64>,
    pub critical_max: Option<f64>,
    pub notes: Option<String>,
}
