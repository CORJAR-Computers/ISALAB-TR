use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::consultation::ConsultationListItem;
use crate::models::sample_list_item::SampleListItem;
use crate::models::surgery::Surgery;
use crate::models::vaccine::VaccineListItem;

/// Métricas del panel de control (dashboard) con las próximas citas,
/// cirugías y refuerzos de vacunación de la agenda.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    // ---- Pacientes ----
    pub patients_total: i32,
    pub patients_active: i32,
    // ---- Laboratorio ----
    pub samples_total: i32,
    pub samples_in_progress: i32,
    pub samples_finished: i32,
    pub samples_cancelled: i32,
    pub abnormal_results: i32,
    // ---- Agenda ----
    pub consultations_pending: i32,
    pub surgeries_programmed: i32,
    // ---- Vacunación ----
    /// Refuerzos cuya fecha ya venció.
    pub vaccines_due: i32,
    // ---- Facturación ----
    /// Facturas emitidas sin pagar.
    pub invoices_unpaid: i32,
    /// Ingresos totales (facturas PAGADA).
    pub revenue_total: f64,
    // ---- Listas para la vista ----
    /// Próximas consultas PENDIENTE (máx. 5).
    pub upcoming_consultations: Vec<ConsultationListItem>,
    /// Próximas cirugías PROGRAMADA/EN_CURSO (máx. 5).
    pub upcoming_surgeries: Vec<Surgery>,
    /// Próximos refuerzos de vacunación (máx. 5).
    pub upcoming_vaccines: Vec<VaccineListItem>,
    /// Últimas muestras recibidas (máx. 5).
    pub recent_samples: Vec<SampleListItem>,
}
