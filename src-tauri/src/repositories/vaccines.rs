use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::vaccine::{CreateVaccineInput, Vaccine, VaccineListItem};
use crate::repositories::next_id;

pub(crate) type VaccineListItemRow = (
    i32, i32, String, String, String, String, String, Option<String>,
    Option<String>, Option<String>, Option<String>,
);

/// Columnas de una vacuna con el tipo y el veterinario unidos (ficha completa).
const VACCINE_SELECT: &str = "
    SELECT v.ID, v.PATIENT_ID, v.VACCINE_TYPE_ID, v.VACCINE_NAME, v.DOSE,
           LEFT(CAST(v.ADMINISTERED_AT AS VARCHAR(60)), 19),
           CAST(v.NEXT_DOSE_AT AS VARCHAR(10)),
           v.LOT, v.MANUFACTURER, u.FULL_NAME, v.NOTES
    FROM VACCINES v
    LEFT JOIN USERS u ON u.ID = v.VETERINARIAN_ID";

pub(crate) type VaccineRow = (
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

pub(crate) fn map_vaccine(r: VaccineRow) -> Vaccine {
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

    let rows: Vec<VaccineListItemRow> = conn
        .query(sql, (&like, &like, &like, &like))
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

    let rows: Vec<VaccineListItemRow> = conn.query(&sql, ()).map_err(AppError::from)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_vaccine_all_fields() {
        let row: VaccineRow = (
            1,                          // id
            10,                         // patient_id
            Some(1),                    // vaccine_type_id (Rabia)
            "Rabia".into(),            // vaccine_name
            Some("1 ml".into()),       // dose
            "2026-08-01 10:00:00".into(), // administered_at
            Some("2027-08-01".into()), // next_dose_at
            Some("LOT-12345".into()),  // lot
            Some("Zoetis".into()),     // manufacturer
            Some("Dr. Ramos".into()),  // veterinarian_name
            Some("Sin reacciones".into()), // notes
        );
        let vaccine = map_vaccine(row);

        assert_eq!(vaccine.id, 1);
        assert_eq!(vaccine.patient_id, 10);
        assert_eq!(vaccine.vaccine_type_id, Some(1));
        assert_eq!(vaccine.vaccine_name, "Rabia");
        assert_eq!(vaccine.dose.as_deref(), Some("1 ml"));
        assert_eq!(vaccine.administered_at, "2026-08-01 10:00:00");
        assert_eq!(vaccine.next_dose_at.as_deref(), Some("2027-08-01"));
        assert_eq!(vaccine.lot.as_deref(), Some("LOT-12345"));
        assert_eq!(vaccine.manufacturer.as_deref(), Some("Zoetis"));
        assert_eq!(vaccine.veterinarian_name.as_deref(), Some("Dr. Ramos"));
        assert_eq!(vaccine.notes.as_deref(), Some("Sin reacciones"));
    }

    #[test]
    fn test_map_vaccine_optional_fields_none() {
        let row: VaccineRow = (
            2, 20, None, "Desparasitación".into(), None,
            "2026-08-15 14:00:00".into(), None, None, None, None, None,
        );
        let vaccine = map_vaccine(row);

        assert_eq!(vaccine.id, 2);
        assert_eq!(vaccine.vaccine_type_id, None);
        assert_eq!(vaccine.dose, None);
        assert_eq!(vaccine.next_dose_at, None);
        assert_eq!(vaccine.lot, None);
        assert_eq!(vaccine.manufacturer, None);
        assert_eq!(vaccine.veterinarian_name, None);
        assert_eq!(vaccine.notes, None);
    }

    #[test]
    fn test_vaccine_list_item_row_mapping() {
        let row: VaccineListItemRow = (
            1, 10, "Luna".into(), "Canino".into(), "Juan Pérez".into(),
            "Rabia".into(), "2026-08-01 10:00:00".into(),
            Some("2027-08-01".into()), Some("LOT-123".into()),
            Some("Zoetis".into()), Some("Dr. Ramos".into()),
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, 10);
        assert_eq!(row.2, "Luna");
        assert_eq!(row.5, "Rabia");
        assert_eq!(row.7.as_deref(), Some("2027-08-01"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    #[test]
    fn test_create_vaccine() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        let input = CreateVaccineInput {
            patient_id,
            vaccine_type_id: None,
            vaccine_name: "Rabia".into(),
            dose: Some("1 ml".into()),
            administered_at: "2026-08-01 10:00:00".into(),
            next_dose_at: Some("2027-08-01".into()),
            lot: Some("LOT-12345".into()),
            manufacturer: Some("Zoetis".into()),
            notes: Some("Sin reacciones".into()),
        };

        let vaccine = create(&mut conn, &input, Some(1)).unwrap();
        assert_eq!(vaccine.vaccine_name, "Rabia");
        assert_eq!(vaccine.patient_id, patient_id);
        assert_eq!(vaccine.dose.as_deref(), Some("1 ml"));
        assert!(vaccine.id > 0);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_by_patient() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        // Insertar dos vacunas
        conn.execute(
            "INSERT INTO VACCINES (ID, PATIENT_ID, VACCINE_NAME, DOSE, ADMINISTERED_AT)
             VALUES (1, ?, 'Rabia', '1 ml', '2026-08-01 10:00:00')",
            (&patient_id,),
        ).unwrap();
        conn.execute(
            "INSERT INTO VACCINES (ID, PATIENT_ID, VACCINE_NAME, DOSE, ADMINISTERED_AT)
             VALUES (2, ?, 'Polivalente', '2 ml', '2026-07-01 10:00:00')",
            (&patient_id,),
        ).unwrap();

        let vaccines = by_patient(&mut conn, patient_id).unwrap();
        assert_eq!(vaccines.len(), 2);
        // Ordenadas por fecha descendente
        assert_eq!(vaccines[0].vaccine_name, "Rabia");
        assert_eq!(vaccines[1].vaccine_name, "Polivalente");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_vaccines() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO VACCINES (ID, PATIENT_ID, VACCINE_NAME, ADMINISTERED_AT)
             VALUES (1, ?, 'Rabia', '2026-08-01 10:00:00')",
            (&patient_id,),
        ).unwrap();

        let vaccines = list(&mut conn, None).unwrap();
        assert_eq!(vaccines.len(), 1);
        assert_eq!(vaccines[0].vaccine_name, "Rabia");
        assert_eq!(vaccines[0].patient_name, "Luna");

        test_helpers::cleanup_test_db(&db_path);
    }
}
