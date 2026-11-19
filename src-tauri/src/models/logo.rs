use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryLogo {
    pub id: i32,
    pub name: String,
    pub mime_type: Option<String>,
    /// Tamaño del BLOB en bytes (sin leer el contenido).
    pub size_bytes: i64,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regresión: el frontend lee `logo.createdAt`. Si alguien quita el
    /// rename_all = "camelCase" vuelve a serializar snake_case y la UI de
    /// logos secundarios se rompe en runtime.
    #[test]
    fn secondary_logo_serializes_camel_case() {
        let logo = SecondaryLogo {
            id: 1,
            name: "Logo Falso".into(),
            mime_type: Some("image/png".into()),
            size_bytes: 1024,
            created_at: "2026-08-01 10:00:00".into(),
        };
        let json = serde_json::to_value(&logo).unwrap();
        assert!(
            json.get("createdAt").is_some(),
            "esperaba createdAt, recibió: {json}"
        );
        assert!(json.get("sizeBytes").is_some());
        assert!(json.get("created_at").is_none());
    }
}
