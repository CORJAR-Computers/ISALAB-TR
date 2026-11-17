use serde::{Deserialize, Serialize};
use specta::Type;

/// Cita de la agenda generalizada (cualquier servicio del catálogo).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Appointment {
    pub id: i32,
    /// Código único (C-YYYY-NNNN).
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub service_id: i32,
    pub service_name: String,
    pub service_code: String,
    pub category_id: i32,
    pub category_name: String,
    pub professional_id: Option<i32>,
    pub professional_name: Option<String>,
    /// YYYY-MM-DD HH:MM:SS
    pub scheduled_at: String,
    pub duration_min: i32,
    /// PROGRAMADA | CONFIRMADA | EN_ATENCION | ATENDIDA | NO_ASISTIO | CANCELADA
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Fila del listado de citas (sin notas ni timestamps de creación).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppointmentListItem {
    pub id: i32,
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub service_id: i32,
    pub service_name: String,
    pub scheduled_at: String,
    pub duration_min: i32,
    pub status: String,
    pub professional_name: Option<String>,
}

/// Entrada para crear una cita.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppointmentInput {
    pub patient_id: i32,
    pub service_id: i32,
    /// Profesional asignado (veterinario, peluquero…). NULL = sin asignar.
    pub professional_id: Option<i32>,
    /// YYYY-MM-DD HH:MM:SS
    pub scheduled_at: String,
    /// Duración en minutos (si no llega, se toma del catálogo del servicio).
    pub duration_min: Option<i32>,
    pub notes: Option<String>,
}

/// Cambio de estado de una cita.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppointmentStatusInput {
    /// Estado: PROGRAMADA | CONFIRMADA | EN_ATENCION | ATENDIDA | NO_ASISTIO | CANCELADA
    pub status: String,
    pub notes: Option<String>,
}
