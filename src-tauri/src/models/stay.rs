use serde::{Deserialize, Serialize};
use specta::Type;

/// Espacio físico de la clínica donde se alojan los pacientes.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: i32,
    pub name: String,
    /// CANIL | GATO | RECEPERA | QUIRURGICO | AISLAMIENTO | PERRERA
    pub kind: String,
    /// Lugares disponibles en el recurso (casi siempre 1).
    pub capacity: i32,
    pub active: bool,
    pub notes: Option<String>,
}

/// Recurso con su ocupación actual (tablero de estancias).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBoardItem {
    pub resource: Resource,
    /// Estancias abiertas asignadas a este recurso.
    pub occupied: i32,
    /// Estancias abiertas que requieren observación veterinaria.
    pub needs_vet: i32,
}

/// Estancia de un paciente (guardería, hospedaje u hospitalización).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Stay {
    pub id: i32,
    /// Código E-YYYY-NNNN.
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_id: i32,
    pub owner_name: String,
    pub owner_phone: Option<String>,
    pub visit_id: Option<i32>,
    pub resource_id: Option<i32>,
    pub resource_name: Option<String>,
    /// GUARDERIA | HOSPEDAJE | HOSPITALIZACION
    pub kind: String,
    /// ABIERTA | CERRADA | CANCELADA
    pub status: String,
    /// YYYY-MM-DD HH:MM:SS
    pub check_in_at: String,
    pub check_out_at: Option<String>,
    pub daily_rate: f64,
    pub notes: Option<String>,
    /// Días de estancia (mín. 1 mientras esté abierta).
    pub days: i32,
    /// Valor estimado de la estancia hasta la fecha actual.
    pub estimated_total: f64,
}

/// Fila compacta del listado de estancias.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StayListItem {
    pub id: i32,
    pub code: String,
    pub patient_id: i32,
    pub patient_name: String,
    pub owner_name: String,
    pub resource_name: Option<String>,
    pub kind: String,
    pub status: String,
    pub check_in_at: String,
    pub check_out_at: Option<String>,
    pub days: i32,
    pub estimated_total: f64,
}

/// Evento de la bitácora de cuidados de una estancia.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StayEvent {
    pub id: i32,
    pub stay_id: i32,
    /// COMIDA | MEDICACION | PASEO | LIMPIEZA | NOTA | VETERINARIO
    pub kind: String,
    /// YYYY-MM-DD HH:MM:SS
    pub occurred_at: String,
    pub notes: Option<String>,
    pub author_name: Option<String>,
}

/// Alta de estancia (ingreso a guardería, hospedaje u hospitalización).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateStayInput {
    pub patient_id: i32,
    /// Visita que originó el ingreso (opcional).
    pub visit_id: Option<i32>,
    pub resource_id: Option<i32>,
    /// GUARDERIA | HOSPEDAJE | HOSPITALIZACION
    pub kind: String,
    /// YYYY-MM-DD HH:MM:SS (vacío = ahora).
    pub check_in_at: Option<String>,
    pub daily_rate: Option<f64>,
    pub notes: Option<String>,
}

/// Cierre de estancia (check-out).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CheckOutStayInput {
    /// YYYY-MM-DD HH:MM:SS (vacío = ahora).
    pub check_out_at: Option<String>,
    pub notes: Option<String>,
}

/// Evento de cuidados (bitácora de la estancia).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateStayEventInput {
    /// COMIDA | MEDICACION | PASEO | LIMPIEZA | NOTA | VETERINARIO
    pub kind: String,
    /// YYYY-MM-DD HH:MM:SS (vacío = ahora).
    pub occurred_at: Option<String>,
    pub notes: Option<String>,
}

/// Alta o edición de un recurso (canil, gatera, box…).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveResourceInput {
    /// NULL = crear; con ID = editar.
    pub id: Option<i32>,
    pub name: String,
    /// CANIL | GATO | RECEPERA | QUIRURGICO | AISLAMIENTO | PERRERA
    pub kind: String,
    pub capacity: Option<i32>,
    pub active: Option<bool>,
    pub notes: Option<String>,
}
