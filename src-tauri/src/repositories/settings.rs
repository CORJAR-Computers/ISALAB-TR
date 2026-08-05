use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::settings::ClinicSettings;

/// Mapa de clave → valor de la tabla CLINIC_SETTINGS.
const SETTING_KEYS: [(&str, &str); 13] = [
    ("clinic.name", "clinic_name"),
    ("clinic.nit", "clinic_nit"),
    ("clinic.address", "address"),
    ("clinic.phone", "phone"),
    ("clinic.city", "city"),
    ("clinic.logo_path", "logo_path"),
    ("invoice.tax_rate", "tax_rate"),
    ("invoice.currency", "currency"),
    ("report.signature_mode", "signature_mode"),
    ("report.vet_name", "vet_name"),
    ("report.vet_license", "vet_license"),
    ("ai.groq_api_key", "groq_api_key"),
    ("report.pkcs12_path", "pkcs12_path"),
    // pkcs12_password NUNCA se persiste en base de datos
];

fn value(conn: &mut SimpleConnection, key: &str) -> Result<Option<String>, AppError> {
    let row: Option<(Option<String>,)> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM CLINIC_SETTINGS WHERE KEY_NAME = ?",
            (&key,),
        )
        .map_err(AppError::from)?;
    Ok(row.and_then(|(v,)| v).filter(|x| !x.is_empty()))
}

pub fn get(conn: &mut SimpleConnection) -> Result<ClinicSettings, AppError> {
    let mut s = ClinicSettings::default();

    for (key, field) in SETTING_KEYS {
        let v = value(conn, key)?;
        match field {
            "clinic_name" => {
                if let Some(val) = v {
                    if !val.is_empty() {
                        s.clinic_name = val;
                    }
                }
            }
            "clinic_nit" => {
                if let Some(val) = v {
                    if !val.is_empty() {
                        s.clinic_nit = val;
                    }
                }
            }
            "address" => s.address = v.filter(|x| !x.is_empty()),
            "phone" => s.phone = v.filter(|x| !x.is_empty()),
            "city" => s.city = v.filter(|x| !x.is_empty()),
            "logo_path" => s.logo_path = v.filter(|x| !x.is_empty()),
            "tax_rate" => {
                if let Some(val) = v {
                    s.tax_rate = val.parse().unwrap_or(19.0);
                }
            }
            "currency" => {
                if let Some(val) = v {
                    if !val.is_empty() {
                        s.currency = val;
                    }
                }
            }
            "signature_mode" => {
                if let Some(val) = v {
                    if !val.is_empty() {
                        s.signature_mode = val;
                    }
                }
            }
            "vet_name" => {
                if let Some(val) = v {
                    s.vet_name = val;
                }
            }
            "vet_license" => s.vet_license = v.filter(|x| !x.is_empty()),
            "groq_api_key" => s.groq_api_key = v.filter(|x| !x.is_empty()),
            "pkcs12_path" => s.pkcs12_path = v.filter(|x| !x.is_empty()),
            _ => {}
        }
    }
    Ok(s)
}

/// Upsert de cada clave (preserva DESCRIPTION y las claves desconocidas).
pub fn save(
    conn: &mut SimpleConnection,
    input: &ClinicSettings,
) -> Result<ClinicSettings, AppError> {
    let tax = format!("{:.2}", input.tax_rate);

    // Reutiliza SETTING_KEYS: el listado de claves vive en un solo lugar.
    for (key, field) in SETTING_KEYS {
        let v: Option<String> = match field {
            "clinic_name" => Some(input.clinic_name.clone()),
            "clinic_nit" => Some(input.clinic_nit.clone()),
            "address" => input.address.clone(),
            "phone" => input.phone.clone(),
            "city" => input.city.clone(),
            "logo_path" => input.logo_path.clone(),
            "tax_rate" => Some(tax.clone()),
            "currency" => Some(input.currency.clone()),
            "signature_mode" => Some(input.signature_mode.clone()),
            "vet_name" => Some(input.vet_name.clone()),
            "vet_license" => input.vet_license.clone(),
            "groq_api_key" => input.groq_api_key.clone(),
            "pkcs12_path" => input.pkcs12_path.clone(),
            _ => None,
        };

        let exists: Option<(i32,)> = conn
            .query_first("SELECT 1 FROM CLINIC_SETTINGS WHERE KEY_NAME = ?", (&key,))
            .map_err(AppError::from)?;

        if exists.is_some() {
            conn.execute(
                "UPDATE CLINIC_SETTINGS SET VALUE_TEXT = ? WHERE KEY_NAME = ?",
                (&v, &key),
            )
            .map_err(AppError::from)?;
        } else {
            conn.execute(
                "INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT, DESCRIPTION)
                 VALUES (?, ?, 'Configurado desde la app')",
                (&key, &v),
            )
            .map_err(AppError::from)?;
        }
    }

    get(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_get_default_settings() {
        let (mut conn, db_path) = setup();
        let settings = get(&mut conn).unwrap();
        // Defaults from ClinicSettings::default()
        assert_eq!(settings.clinic_name, "Mi Clínica Veterinaria");
        assert_eq!(settings.clinic_nit, "900000000-0");
        assert_eq!(settings.tax_rate, 19.0);
        assert_eq!(settings.currency, "COP");
        assert_eq!(settings.signature_mode, "GRAPHIC");
        assert!(settings.groq_api_key.is_none());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_and_get_settings() {
        let (mut conn, db_path) = setup();

        let input = ClinicSettings {
            clinic_name: "Pet Health Clinic".to_string(),
            clinic_nit: "800123456-7".to_string(),
            address: Some("Calle 10 # 5-20".to_string()),
            phone: Some("300 555 1234".to_string()),
            city: Some("Medellín".to_string()),
            logo_path: None,
            tax_rate: 16.0,
            currency: "USD".to_string(),
            signature_mode: "GRAPHIC".to_string(),
            vet_name: "Dr. Carlos López".to_string(),
            vet_license: Some("MVZ-12345".to_string()),
            groq_api_key: Some("gsk_test_key".to_string()),
            pkcs12_path: None,
            pkcs12_password: None,
        };

        let saved = save(&mut conn, &input).unwrap();
        assert_eq!(saved.clinic_name, "Pet Health Clinic");
        assert_eq!(saved.clinic_nit, "800123456-7");
        assert_eq!(saved.address, Some("Calle 10 # 5-20".to_string()));
        assert_eq!(saved.phone, Some("300 555 1234".to_string()));
        assert_eq!(saved.city, Some("Medellín".to_string()));
        assert_eq!(saved.tax_rate, 16.0);
        assert_eq!(saved.currency, "USD");
        assert_eq!(saved.vet_name, "Dr. Carlos López");
        assert_eq!(saved.vet_license, Some("MVZ-12345".to_string()));
        assert_eq!(saved.groq_api_key, Some("gsk_test_key".to_string()));

        // Verificar que get devuelve los mismos valores
        let fetched = get(&mut conn).unwrap();
        assert_eq!(fetched.clinic_name, "Pet Health Clinic");
        assert_eq!(fetched.tax_rate, 16.0);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_updates_existing_settings() {
        let (mut conn, db_path) = setup();

        // Guardar primera vez
        let input1 = ClinicSettings {
            clinic_name: "Primera Clínica".to_string(),
            ..Default::default()
        };
        save(&mut conn, &input1).unwrap();

        // Actualizar solo el nombre
        let input2 = ClinicSettings {
            clinic_name: "Segunda Clínica".to_string(),
            ..Default::default()
        };
        let saved = save(&mut conn, &input2).unwrap();
        assert_eq!(saved.clinic_name, "Segunda Clínica");

        // Verificar que no se duplicaron registros
        let count: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM CLINIC_SETTINGS WHERE KEY_NAME = 'clinic.name'",
                (),
            )
            .unwrap();
        assert_eq!(count.unwrap().0, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_with_null_optionals() {
        let (mut conn, db_path) = setup();

        let input = ClinicSettings {
            clinic_name: "Test Clinic".to_string(),
            clinic_nit: "123".to_string(),
            address: None,
            phone: None,
            city: None,
            logo_path: None,
            tax_rate: 0.0,
            currency: "COP".to_string(),
            signature_mode: "GRAPHIC".to_string(),
            vet_name: "".to_string(),
            vet_license: None,
            groq_api_key: None,
            pkcs12_path: None,
            pkcs12_password: None,
        };

        let saved = save(&mut conn, &input).unwrap();
        assert_eq!(saved.clinic_name, "Test Clinic");
        assert!(saved.address.is_none());
        assert!(saved.phone.is_none());
        assert!(saved.groq_api_key.is_none());
        assert_eq!(saved.tax_rate, 0.0);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_groq_api_key_empty_becomes_none() {
        let (mut conn, db_path) = setup();

        // Guardar con key
        let input1 = ClinicSettings {
            clinic_name: "Test".to_string(),
            clinic_nit: "123".to_string(),
            groq_api_key: Some("gsk_real_key".to_string()),
            ..Default::default()
        };
        save(&mut conn, &input1).unwrap();

        // Actualizar con string vacío → debe guardar None
        let input2 = ClinicSettings {
            clinic_name: "Test".to_string(),
            clinic_nit: "123".to_string(),
            groq_api_key: Some("".to_string()),
            ..Default::default()
        };
        let saved = save(&mut conn, &input2).unwrap();
        assert!(saved.groq_api_key.is_none());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_after_empty_database() {
        let (mut conn, db_path) = setup();
        // Sin haber guardado nada, get debe devolver defaults
        let settings = get(&mut conn).unwrap();
        assert_eq!(settings.clinic_name, "Mi Clínica Veterinaria");
        assert_eq!(settings.tax_rate, 19.0);
        cleanup_test_db(&db_path);
    }
}
