use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::consultation::Consultation;
use crate::models::grooming::GroomingSession;
use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::sample::Sample;
use crate::models::vaccine::Vaccine;
use crate::models::visit::VisitListItem;

/// Resumen compacto de un cobro (factura) del paciente para la línea de
/// tiempo del historial clínico.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PatientChargeSummary {
    pub invoice_id: i32,
    /// Número de factura (FAC-…).
    pub invoice_number: String,
    /// YYYY-MM-DD HH:MM:SS
    pub issue_date: String,
    pub total: f64,
    /// EMITIDA | PAGADA | ANULADA
    pub status: String,
}

/// Agregado del historial clínico completo de un paciente.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicalHistory {
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub consultations: Vec<Consultation>,
    pub vaccines: Vec<Vaccine>,
    pub samples: Vec<Sample>,
    /// Visitas de recepción del paciente (fase 5).
    pub visits: Vec<VisitListItem>,
    /// Sesiones de estética del paciente (fase 5).
    pub grooming_sessions: Vec<GroomingSession>,
    /// Cobros (facturas) del paciente, de la más reciente (fase 5).
    pub charges: Vec<PatientChargeSummary>,
}
