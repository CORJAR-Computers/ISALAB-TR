use serde::Serialize;
use specta::Type;

/// Conteo de registros agrupados por estado, devuelto por los endpoints de contadores.
#[derive(Debug, Clone, Serialize, Type)]
pub struct StatusCount {
    pub status: String,
    pub count: i32,
}
