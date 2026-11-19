use serde::{Deserialize, Serialize};
use specta::Type;

/// Ficha de sesión de peluquería: una por línea de servicio de categoría
/// PELUQUERIA en una visita (migración 0029).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GroomingSession {
    pub id: i32,
    pub visit_service_id: i32,
    pub visit_id: i32,
    pub service_name: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_name: String,
    /// Peluquero asignado a la sesión.
    pub groomer_id: Option<i32>,
    pub groomer_name: Option<String>,
    /// Corte de raza, higiénico, cachorro…
    pub style: Option<String>,
    /// Medicado, antipulgas…
    pub bath_type: Option<String>,
    pub coat_condition: Option<String>,
    /// 0 = sin enredos … 3 = enredo severo.
    pub matting_level: i32,
    pub weight_kg: Option<f64>,
    /// Tranquilo, nervioso, agresivo…
    pub behavior: Option<String>,
    pub products_used: Option<String>,
    /// YYYY-MM-DD HH:MM:SS (NULL si no inició).
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    /// YYYY-MM-DD (NULL si no se sugiere próximo turno).
    pub next_suggested_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Línea de peluquería del tablero del día: servicio de categoría PELUQUERIA
/// dentro de una visita de la fecha, con el estado de su sesión.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GroomingDayLine {
    pub visit_service_id: i32,
    pub visit_id: i32,
    pub visit_code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_name: String,
    pub service_id: i32,
    pub service_name: String,
    pub professional_id: Option<i32>,
    pub professional_name: Option<String>,
    /// Estado de la línea: PENDIENTE | EN_PROCESO | COMPLETADA | CANCELADA
    pub line_status: String,
    /// SIN_SESION | ABIERTA (ficha sin iniciar) | EN_PROCESO | FINALIZADA
    pub session_state: String,
    /// YYYY-MM-DD HH:MM:SS llegada del paciente.
    pub arrived_at: String,
    /// Estado de la visita: EN_ESPERA | EN_ATENCION | FINALIZADA | FACTURADA | CANCELADA
    pub visit_status: String,
    /// YYYY-MM-DD próximo turno sugerido (si la sesión lo fijó).
    pub next_suggested_at: Option<String>,
    /// Nivel de enredos de la ficha (0 si no hay sesión).
    pub matting_level: i32,
}

/// Próximo turno sugerido por una sesión finalizada (recordatorio).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GroomingReminder {
    pub session_id: i32,
    pub visit_service_id: i32,
    pub patient_name: String,
    pub owner_name: String,
    pub service_name: String,
    /// YYYY-MM-DD
    pub next_suggested_at: String,
    pub groomer_name: Option<String>,
}

/// Crea o actualiza la sesión de una línea de peluquería (upsert 1:1).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpsertGroomingSessionInput {
    pub visit_service_id: i32,
    pub groomer_id: Option<i32>,
    pub style: Option<String>,
    pub bath_type: Option<String>,
    pub coat_condition: Option<String>,
    pub matting_level: i32,
    pub weight_kg: Option<f64>,
    pub behavior: Option<String>,
    pub products_used: Option<String>,
    /// NULL = iniciar ahora (CURRENT_TIMESTAMP).
    pub started_at: Option<String>,
    /// YYYY-MM-DD o NULL.
    pub next_suggested_at: Option<String>,
    pub notes: Option<String>,
}

/// Cierra la sesión: FINISHED_AT, próximo turno sugerido y notas de cierre.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FinishGroomingSessionInput {
    /// NULL = ahora (CURRENT_TIMESTAMP).
    pub finished_at: Option<String>,
    /// YYYY-MM-DD o NULL.
    pub next_suggested_at: Option<String>,
    pub notes: Option<String>,
}

/// Foto antes/después (o documento) adjunta a una visita (migración 0029).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VisitAttachment {
    pub id: i32,
    pub visit_id: i32,
    pub file_name: String,
    pub mime_type: Option<String>,
    /// BEFORE | AFTER | DOCUMENT
    pub kind: Option<String>,
    /// Tamaño del BLOB en bytes (sin leer el contenido).
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddVisitAttachmentInput {
    pub visit_id: i32,
    /// BEFORE | AFTER | DOCUMENT
    pub kind: Option<String>,
}
