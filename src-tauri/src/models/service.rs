use serde::{Deserialize, Serialize};
use specta::Type;

/// Categoría del catálogo de servicios (CONSULTA, PELUQUERIA…).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCategory {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub sort_order: i32,
    pub is_active: bool,
}

/// Servicio del catálogo: única fuente de precio, duración, IVA y
/// profesional requerido para toda la clínica.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub category_id: i32,
    pub category_name: String,
    pub description: Option<String>,
    pub duration_min: i32,
    pub price: f64,
    /// NULL = usa el IVA global de la clínica.
    pub tax_rate: Option<f64>,
    pub requires_vet: bool,
    pub requires_groomer: bool,
    /// NULL = todas las especies.
    pub species_id: Option<i32>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// Entrada de alta/edición de una categoría.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveServiceCategoryInput {
    /// `None` = crear; `Some` = editar.
    pub id: Option<i32>,
    pub code: String,
    pub name: String,
    pub sort_order: i32,
}

/// Entrada de alta/edición de un servicio.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveServiceInput {
    /// `None` = crear; `Some` = editar.
    pub id: Option<i32>,
    pub code: String,
    pub name: String,
    pub category_id: i32,
    pub description: Option<String>,
    pub duration_min: i32,
    pub price: f64,
    pub tax_rate: Option<f64>,
    pub requires_vet: bool,
    pub requires_groomer: bool,
    pub species_id: Option<i32>,
    pub sort_order: i32,
}
