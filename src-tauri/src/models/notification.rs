use serde::{Deserialize, Serialize};
use specta::Type;

/// Fila del registro de notificaciones de valores críticos (NOTIFICATION_LOG).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationLogEntry {
    pub id: i32,
    pub result_id: Option<i32>,
    pub sample_id: i32,
    /// WHATSAPP | EMAIL | MANUAL (confirmación del analista)
    pub channel: String,
    pub recipient_name: Option<String>,
    pub recipient_address: Option<String>,
    /// SENT | FAILED | ACKNOWLEDGED
    pub status: String,
    pub sent_at: Option<String>,
    pub acked_at: Option<String>,
    pub acked_by: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
