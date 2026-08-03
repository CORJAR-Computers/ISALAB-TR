use serde::{Deserialize, Serialize};
use specta::Type;

/// Vacuna administrada a un paciente (con campos unidos para el listado global).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Vaccine {
    pub id: i32,
    pub patient_id: i32,
    pub vaccine_type_id: Option<i32>,
    pub vaccine_name: String,
    pub dose: Option<String>,
    pub administered_at: String,
    pub next_dose_at: Option<String>,
    pub lot: Option<String>,
    pub manufacturer: Option<String>,
    pub veterinarian_name: Option<String>,
    pub notes: Option<String>,
}

/// Entrada del registro de vacunación (insumo del formulario).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaccineInput {
    pub patient_id: i32,
    pub vaccine_type_id: Option<i32>,
    pub vaccine_name: String,
    pub dose: Option<String>,
    pub administered_at: String,
    pub next_dose_at: Option<String>,
    pub lot: Option<String>,
    pub manufacturer: Option<String>,
    pub notes: Option<String>,
}

/// Fila del listado global de vacunación (vista por paciente/propietario).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VaccineListItem {
    pub id: i32,
    pub patient_id: i32,
    pub patient_name: String,
    pub species_name: String,
    pub owner_name: String,
    pub vaccine_name: String,
    pub administered_at: String,
    pub next_dose_at: Option<String>,
    pub lot: Option<String>,
    pub manufacturer: Option<String>,
    pub veterinarian_name: Option<String>,
}
