use serde::{Deserialize, Serialize};
use specta::Type;

/// Línea de servicio dentro de una visita (precio congelado del catálogo).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VisitService {
    pub id: i32,
    pub visit_id: i32,
    pub service_id: i32,
    pub service_name: String,
    pub service_code: String,
    pub category_id: i32,
    pub category_name: String,
    /// Profesional asignado (veterinario, peluquero…).
    pub professional_id: Option<i32>,
    pub professional_name: Option<String>,
    pub quantity: i32,
    pub unit_price: f64,
    /// NULL = IVA global de la clínica.
    pub tax_rate: Option<f64>,
    /// PENDIENTE | EN_PROCESO | COMPLETADA | CANCELADA
    pub status: String,
    /// YYYY-MM-DD HH:MM:SS (NULL si no inició).
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub notes: Option<String>,
    pub seq: i32,
}

/// Visita completa (con sus líneas de servicio).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Visit {
    pub id: i32,
    /// Código único (V-YYYY-NNNN).
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub appointment_id: Option<i32>,
    /// YYYY-MM-DD HH:MM:SS
    pub arrived_at: String,
    /// NORMAL | URGENTE
    pub priority: String,
    /// EN_ESPERA | EN_ATENCION | FINALIZADA | FACTURADA | CANCELADA
    pub status: String,
    pub notes: Option<String>,
    pub created_by_name: Option<String>,
    pub services: Vec<VisitService>,
}

/// Fila del listado de visitas de la cola de recepción (sin líneas).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VisitListItem {
    pub id: i32,
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub arrived_at: String,
    pub priority: String,
    pub status: String,
    pub notes: Option<String>,
    /// Resumen "Baño + Consulta general".
    pub services_summary: String,
    /// Total estimado de las líneas no canceladas (precio congelado).
    pub estimated_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddVisitServiceInput {
    pub service_id: i32,
    /// Profesional asignado a esta línea.
    pub professional_id: Option<i32>,
    pub quantity: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateVisitInput {
    pub patient_id: i32,
    /// Si no llega, se toma del paciente.
    pub owner_id: Option<i32>,
    pub priority: Option<String>,
    pub notes: Option<String>,
    pub services: Vec<AddVisitServiceInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVisitServiceStatusInput {
    /// Estado de la línea: EN_PROCESO | COMPLETADA | CANCELADA.
    pub status: String,
    /// Reasignación opcional del profesional.
    pub professional_id: Option<i32>,
}
