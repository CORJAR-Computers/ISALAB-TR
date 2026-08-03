use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Consultation {
    pub id: i32,
    pub patient_id: i32,
    pub veterinarian_id: Option<i32>,
    /// YYYY-MM-DD HH:MM:SS
    pub consultation_date: String,
    pub reason: String,
    pub anamnesis: Option<String>,
    pub physical_exam: Option<String>,
    pub diagnosis: Option<String>,
    pub treatment_plan: Option<String>,
    pub status: String,
    pub veterinarian_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateConsultationInput {
    pub patient_id: i32,
    pub consultation_date: String,
    pub reason: String,
    pub anamnesis: Option<String>,
    pub physical_exam: Option<String>,
    pub diagnosis: Option<String>,
    pub treatment_plan: Option<String>,
    pub status: String,
}

/// Fila del listado global de consultas (agenda) con datos del paciente unidos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConsultationListItem {
    pub id: i32,
    pub patient_id: i32,
    pub patient_name: String,
    pub species_name: String,
    pub owner_name: String,
    /// YYYY-MM-DD HH:MM:SS
    pub consultation_date: String,
    pub reason: String,
    pub status: String,
    pub veterinarian_name: Option<String>,
}
