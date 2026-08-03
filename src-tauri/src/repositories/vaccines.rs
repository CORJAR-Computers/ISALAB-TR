use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::vaccine::{CreateVaccineInput, Vaccine, VaccineListItem};
use crate::repositories::next_id;

/// Columnas de una vacuna con el tipo y el veterinario unidos (ficha completa).
const VACCINE_SELECT: &str = "
    SELECT v.ID, v.PATIENT_ID, v.VACCINE_TYPE_ID, v.VACCINE_NAME, v.DOSE,
           LEFT(CAST(v.ADMINISTERED_AT AS VARCHAR(60)), 19),
           CAST(v.NEXT_DOSE_AT AS VARCHAR(10)),
           v.LOT, v.MANUFACTURER, u.FULL_NAME, v.NOTES
    FROM VACCINES v
    LEFT JOIN USERS u ON u.ID = v.VETERINARIAN_ID";

type VaccineRow = (
    i32,            // id
    i32,            // patient_id
    Option<i32>,    // vaccine_type_id
    String,         // vaccine_name
    Option<String>, // dose
    String,         // administered_at
    Option<String>, // next_dose_at
    Option<String>, // lot
    Option<String>, // manufacturer
    Option<String>, // veterinarian_name
    Option<String>, // notes
);

fn map_vaccine(r: VaccineRow) -> Vaccine {
    Vaccine {
        id: r.0,
        patient_id: r.1,
        vaccine_type_id: r.2,
        vaccine_name: r.3,
        dose: r.4,
        administered_at: r.5,
        next_dose_at: r.6,
        lot: r.7,
        manufacturer: r.8,
        veterinarian_name: r.9,
        notes: r.10,
    }
}

/// Registra una vacuna/desparasitación. El veterinario se atribuye desde la
/// sesión activa (autor del registro).
pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateVaccineInput,
    veterinarian_id: Option<i32>,
) -> Result<Vaccine, AppError> {
    let id = next_id(conn, "GEN_VACCINES_ID")?;
    conn.execute(
        "INSERT INTO VACCINES
            (ID, PATIENT_ID, VACCINE_TYPE_ID, VACCINE_NAME, DOSE, ADMINISTERED_AT,
             NEXT_DOSE_AT, LOT, MANUFACTURER, VETERINARIAN_ID, NOTES)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.vaccine_type_id,
            &input.vaccine_name,
            &input.dose,
            &input.administered_at,
            &input.next_dose_at,
            &input.lot,
            &input.manufacturer,
            &veterinarian_id,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    let row: Option<VaccineRow> = conn
        .query_first(&format!("{VACCINE_SELECT} WHERE v.ID = ?"), (&id,))
        .map_err(AppError::from)?;

    row.map(map_vaccine)
        .ok_or_else(|| AppError::Internal("Vacuna creada pero no recuperada".into()))
}

/// Vacunas de un paciente (para el historial clínico).
pub fn by_patient(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<Vaccine>, AppError> {
    let rows: Vec<VaccineRow> = conn
        .query(
            &format!(
                "{VACCINE_SELECT} WHERE v.PATIENT_ID = ? ORDER BY v.ADMINISTERED_AT DESC"
            ),
            (&patient_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_vaccine).collect())
}

/// Listado global de vacunación con filtro opcional por paciente o vacuna.
pub fn list(
    conn: &mut SimpleConnection,
    search: Option<&str>,
) -> Result<Vec<VaccineListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT v.ID, v.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME,
               v.VACCINE_NAME,
               LEFT(CAST(v.ADMINISTERED_AT AS VARCHAR(60)), 19),
               CAST(v.NEXT_DOSE_AT AS VARCHAR(10)),
               v.LOT, v.MANUFACTURER, u.FULL_NAME
        FROM VACCINES v
        JOIN PATIENTS p ON p.ID = v.PATIENT_ID
        JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
        JOIN OWNERS o ON o.ID = p.OWNER_ID
        LEFT JOIN USERS u ON u.ID = v.VETERINARIAN_ID
        WHERE (? IS NULL
               OR UPPER(p.NAME) LIKE UPPER(?)
               OR UPPER(o.FULL_NAME) LIKE UPPER(?)
               OR UPPER(v.VACCINE_NAME) LIKE UPPER(?))
        ORDER BY v.ADMINISTERED_AT DESC";

    let rows: Vec<(
        i32, i32, String, String, String, String, String, Option<String>,
        Option<String>, Option<String>, Option<String>,
    )> = conn
        .query(&sql, (&like, &like, &like, &like))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| VaccineListItem {
            id: r.0,
            patient_id: r.1,
            patient_name: r.2,
            species_name: r.3,
            owner_name: r.4,
            vaccine_name: r.5,
            administered_at: r.6,
            next_dose_at: r.7,
            lot: r.8,
            manufacturer: r.9,
            veterinarian_name: r.10,
        })
        .collect())
}

/// Próximos refuerzos (NEXT_DOSE_AT >= hoy) para la agenda del dashboard.
pub fn list_upcoming(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<VaccineListItem>, AppError> {
    let sql = format!(
        "SELECT FIRST {limit} v.ID, v.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME,
                v.VACCINE_NAME,
                LEFT(CAST(v.ADMINISTERED_AT AS VARCHAR(60)), 19),
                CAST(v.NEXT_DOSE_AT AS VARCHAR(10)),
                v.LOT, v.MANUFACTURER, u.FULL_NAME
         FROM VACCINES v
         JOIN PATIENTS p ON p.ID = v.PATIENT_ID
         JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
         JOIN OWNERS o ON o.ID = p.OWNER_ID
         LEFT JOIN USERS u ON u.ID = v.VETERINARIAN_ID
         WHERE v.NEXT_DOSE_AT IS NOT NULL AND v.NEXT_DOSE_AT >= CURRENT_DATE
         ORDER BY v.NEXT_DOSE_AT ASC"
    );

    let rows: Vec<(
        i32, i32, String, String, String, String, String, Option<String>,
        Option<String>, Option<String>, Option<String>,
    )> = conn.query(&sql, ()).map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| VaccineListItem {
            id: r.0,
            patient_id: r.1,
            patient_name: r.2,
            species_name: r.3,
            owner_name: r.4,
            vaccine_name: r.5,
            administered_at: r.6,
            next_dose_at: r.7,
            lot: r.8,
            manufacturer: r.9,
            veterinarian_name: r.10,
        })
        .collect())
}
