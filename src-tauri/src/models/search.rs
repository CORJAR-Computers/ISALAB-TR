use serde::{Deserialize, Serialize};
use specta::Type;

/// Resultado de la búsqueda global (paleta Ctrl+K): una entidad del sistema
/// con su destino de navegación en la UI.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchResult {
    /// patient | sample | invoice | surgery
    pub kind: String,
    pub id: i32,
    /// Etiqueta principal (nombre del paciente, cliente, etc.).
    pub title: String,
    /// Etiqueta secundaria (detalle contextual).
    pub subtitle: String,
    /// Código de trazabilidad cuando existe (PAC-, M-, FAC-…).
    pub code: Option<String>,
}
