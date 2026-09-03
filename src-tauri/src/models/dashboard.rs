use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::consultation::ConsultationListItem;
use crate::models::sample_list_item::SampleListItem;
use crate::models::surgery::Surgery;
use crate::models::vaccine::VaccineListItem;

/// Conteo de un analito (para el ranking de los más solicitados).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalyteCount {
    pub analyte_name: String,
    pub count: i32,
}

/// Volumen de muestras recibidas en un día (tendencia).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DailySampleVolume {
    /// YYYY-MM-DD
    pub date: String,
    pub count: i32,
}

/// Tiempo promedio de respuesta (recepción → finalización) por tipo de muestra.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SampleTypeTurnaround {
    pub sample_type_id: i32,
    pub sample_type_name: String,
    /// Promedio en minutos.
    pub avg_minutes: f64,
    /// Muestras finalizadas consideradas.
    pub count: i32,
}

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
    /// Tiempo promedio recepción → finalización (en horas).
    pub avg_processing_hours: f64,
    /// Tiempo promedio de respuesta (recepción → finalización) por tipo de
    /// muestra, en minutos (ordenado de mayor a menor).
    pub turnaround_by_sample_type: Vec<SampleTypeTurnaround>,
    /// Porcentaje de muestras finalizadas con al menos un valor fuera de rango (0-100).
    pub abnormal_rate: f64,
    /// Volumen de muestras recibidas en los últimos 7 días (tendencia).
    pub weekly_volume: Vec<DailySampleVolume>,
    /// Analitos más solicitados (máx. 5).
    pub top_analytes: Vec<AnalyteCount>,
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
