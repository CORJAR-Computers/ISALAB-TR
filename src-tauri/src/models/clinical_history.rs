use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::consultation::Consultation;
use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::sample::Sample;
use crate::models::vaccine::Vaccine;

/// Agregado del historial clínico completo de un paciente.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicalHistory {
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub consultations: Vec<Consultation>,
    pub vaccines: Vec<Vaccine>,
    pub samples: Vec<Sample>,
}
