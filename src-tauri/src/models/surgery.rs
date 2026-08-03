use serde::{Deserialize, Serialize};
use specta::Type;

/// Cirugía programada con datos del paciente unidos (agenda quirúrgica).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Surgery {
    pub id: i32,
    pub patient_id: i32,
    pub patient_name: String,
    pub species_name: String,
    pub owner_name: String,
    pub veterinarian_id: Option<i32>,
    pub veterinarian_name: Option<String>,
    pub surgery_type: String,
    /// YYYY-MM-DD HH:MM:SS
    pub scheduled_at: String,
    pub anesthesia_type: Option<String>,
    pub preoperative_notes: Option<String>,
    pub postoperative_notes: Option<String>,
    /// PROGRAMADA | EN_CURSO | COMPLETADA | CANCELADA
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateSurgeryInput {
    pub patient_id: i32,
    pub surgery_type: String,
    pub scheduled_at: String,
    pub anesthesia_type: Option<String>,
    pub preoperative_notes: Option<String>,
    pub postoperative_notes: Option<String>,
}
