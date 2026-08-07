use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryLogo {
    pub id: i32,
    pub name: String,
    pub logo_path: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regresión: el frontend lee `logo.logoPath` / `logo.createdAt`. Si alguien
    /// quita el rename_all = "camelCase" vuelve a serializar snake_case y la
    /// UI de logos secundarios se rompe en runtime.
    #[test]
    fn secondary_logo_serializes_camel_case() {
        let logo = SecondaryLogo {
            id: 1,
            name: "Logo Falso".into(),
            logo_path: "C:/logos/falso.png".into(),
            created_at: "2026-08-01 10:00:00".into(),
        };
        let json = serde_json::to_value(&logo).unwrap();
        assert!(
            json.get("logoPath").is_some(),
            "esperaba logoPath, recibió: {json}"
        );
        assert!(
            json.get("createdAt").is_some(),
            "esperaba createdAt, recibió: {json}"
        );
        assert!(json.get("logo_path").is_none());
        assert!(json.get("created_at").is_none());
    }
}
