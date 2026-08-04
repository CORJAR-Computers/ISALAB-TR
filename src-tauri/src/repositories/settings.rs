use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::settings::ClinicSettings;

/// Mapa de clave → valor de la tabla CLINIC_SETTINGS.
const SETTING_KEYS: [(&str, &str); 12] = [
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
];

fn value(conn: &mut SimpleConnection, key: &str) -> Result<Option<String>, AppError> {
    let row: Option<(Option<String>,)> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM CLINIC_SETTINGS WHERE KEY_NAME = ?",
            (&key,),
        )
        .map_err(AppError::from)?;
    Ok(row.and_then(|(v,)| v))
}

pub fn get(conn: &mut SimpleConnection) -> Result<ClinicSettings, AppError> {
    let mut s = ClinicSettings::default();

    for (key, field) in SETTING_KEYS {
        let v = value(conn, key)?.unwrap_or_default();
        match field {
            "clinic_name" => s.clinic_name = if v.is_empty() { s.clinic_name } else { v },
            "clinic_nit" => s.clinic_nit = if v.is_empty() { s.clinic_nit } else { v },
            "address" => s.address = Some(v),
            "phone" => s.phone = Some(v),
            "city" => s.city = Some(v),
            "logo_path" => s.logo_path = Some(v),
            "tax_rate" => s.tax_rate = v.parse().unwrap_or(19.0),
            "currency" => s.currency = if v.is_empty() { s.currency } else { v },
            "signature_mode" => s.signature_mode = if v.is_empty() { s.signature_mode } else { v },
            "vet_name" => s.vet_name = v,
            "vet_license" => s.vet_license = Some(v),
            "groq_api_key" => s.groq_api_key = Some(v).filter(|x| !x.is_empty()),
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
            _ => None,
        };

        let exists: Option<(i32,)> = conn
            .query_first(
                "SELECT 1 FROM CLINIC_SETTINGS WHERE KEY_NAME = ?",
                (&key,),
            )
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
