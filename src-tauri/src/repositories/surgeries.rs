use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::surgery::{CreateSurgeryInput, Surgery};
use crate::repositories::next_id;

/// Columnas de una cirugía con paciente y veterinario unidos.
const SURGERY_SELECT: &str = "
    SELECT s.ID, s.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME, o.PHONE,
           s.VETERINARIAN_ID, u.FULL_NAME,
           s.SURGERY_TYPE,
           LEFT(CAST(s.SCHEDULED_AT AS VARCHAR(60)), 19),
           s.ANESTHESIA_TYPE, s.PREOPERATIVE_NOTES, s.POSTOPERATIVE_NOTES,
           s.STATUS
    FROM SURGERIES s
    JOIN PATIENTS p ON p.ID = s.PATIENT_ID
    JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
    JOIN OWNERS o ON o.ID = p.OWNER_ID
    LEFT JOIN USERS u ON u.ID = s.VETERINARIAN_ID";

pub(crate) type SurgeryRow = (
    i32,             // id
    i32,             // patient_id
    String,          // patient_name
    String,          // species_name
    String,          // owner_name
    Option<String>,  // owner_phone
    Option<i32>,     // veterinarian_id
    Option<String>,  // veterinarian_name
    String,          // surgery_type
    String,          // scheduled_at
    Option<String>,  // anesthesia_type
    Option<String>,  // preoperative_notes
    Option<String>,  // postoperative_notes
    String,          // status
);

pub(crate) fn map_surgery(r: SurgeryRow) -> Surgery {
    Surgery {
        id: r.0,
        patient_id: r.1,
        patient_name: r.2,
        species_name: r.3,
        owner_name: r.4,
        owner_phone: r.5,
        veterinarian_id: r.6,
        veterinarian_name: r.7,
        surgery_type: r.8,
        scheduled_at: r.9,
        anesthesia_type: r.10,
        preoperative_notes: r.11,
        postoperative_notes: r.12,
        status: r.13,
    }
}

/// Programa una cirugía. El veterinario responsable se toma de la sesión.
pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateSurgeryInput,
    veterinarian_id: Option<i32>,
) -> Result<Surgery, AppError> {
    let id = next_id(conn, "GEN_SURGERIES_ID")?;
    conn.execute(
        "INSERT INTO SURGERIES
            (ID, PATIENT_ID, VETERINARIAN_ID, SURGERY_TYPE, SCHEDULED_AT,
             ANESTHESIA_TYPE, PREOPERATIVE_NOTES, POSTOPERATIVE_NOTES, STATUS)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'PROGRAMADA')",
        (
            &id,
            &input.patient_id,
            &veterinarian_id,
            &input.surgery_type,
            &input.scheduled_at,
            &input.anesthesia_type,
            &input.preoperative_notes,
            &input.postoperative_notes,
        ),
    )
    .map_err(AppError::from)?;

    let row: Option<SurgeryRow> = conn
        .query_first(&format!("{SURGERY_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;

    row.map(map_surgery)
        .ok_or_else(|| AppError::Internal("Cirugía creada pero no recuperada".into()))
}

/// Ficha completa de una cirugía (para reportes PDF).
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Surgery>, AppError> {
    let row: Option<SurgeryRow> = conn
        .query_first(&format!("{SURGERY_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_surgery))
}

/// Agenda quirúrgica: listado con filtros opcionales por estado y búsqueda
/// (paciente, propietario o tipo de cirugía).
pub fn list(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<Surgery>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = format!(
        "{SURGERY_SELECT}
         WHERE (? IS NULL OR s.STATUS = ?)
           AND (? IS NULL
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
                OR UPPER(s.SURGERY_TYPE) LIKE UPPER(?))
         ORDER BY s.SCHEDULED_AT DESC, s.ID DESC"
    );

    let rows: Vec<SurgeryRow> = conn
        .query(&sql, (&status, &status, &like, &like, &like, &like))
        .map_err(AppError::from)?;

    Ok(rows.into_iter().map(map_surgery).collect())
}

/// Próximas cirugías de la agenda (dashboard): PROGRAMADA/EN_CURSO desde hoy.
pub fn list_upcoming(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<Surgery>, AppError> {
    let sql = format!(
        "{SURGERY_SELECT}
         WHERE s.STATUS IN ('PROGRAMADA', 'EN_CURSO')
           AND s.SCHEDULED_AT >= CURRENT_TIMESTAMP
         ORDER BY s.SCHEDULED_AT ASC
         ROWS {limit}"
    );
    let rows: Vec<SurgeryRow> = conn.query(&sql, ()).map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_surgery).collect())
}

/// Cambia el estado de una cirugía validando la transición:
/// PROGRAMADA → EN_CURSO/COMPLETADA/CANCELADA, EN_CURSO → COMPLETADA/CANCELADA.
pub fn set_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<Surgery, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SURGERIES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) = current.ok_or_else(|| {
        AppError::NotFound(format!("Cirugía {id} no encontrada"))
    })?;

    let allowed = match status {
        "EN_CURSO" => current == "PROGRAMADA",
        "COMPLETADA" => matches!(current.as_str(), "PROGRAMADA" | "EN_CURSO"),
        "CANCELADA" => matches!(current.as_str(), "PROGRAMADA" | "EN_CURSO"),
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status} \
             (PROGRAMADA→EN_CURSO, →COMPLETADA, →CANCELADA)"
        )));
    }

    conn.execute(
        "UPDATE SURGERIES
            SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&status, &id),
    )
    .map_err(AppError::from)?;

    let row: Option<SurgeryRow> = conn
        .query_first(&format!("{SURGERY_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    row.map(map_surgery)
        .ok_or_else(|| AppError::Internal("Cirugía actualizada pero no recuperada".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_surgery_all_fields() {
        let row: SurgeryRow = (
            1,                          // id
            10,                         // patient_id
            "Luna".into(),              // patient_name
            "Canino".into(),            // species_name
            "Juan Pérez".into(),        // owner_name
            Some("+57 300 1234567".into()), // owner_phone
            Some(5),                     // veterinarian_id
            Some("Dr. Ramos".into()),   // veterinarian_name
            "Cesárea".into(),           // surgery_type
            "2026-08-15 09:00:00".into(), // scheduled_at
            Some("General".into()),     // anesthesia_type
            Some("Paciente en ayunas".into()), // preoperative_notes
            Some("Cirugía exitosa".into()), // postoperative_notes
            "PROGRAMADA".into(),        // status
        );
        let surgery = map_surgery(row);

        assert_eq!(surgery.id, 1);assert_eq!(surgery.patient_id, 10);
        assert_eq!(surgery.patient_name, "Luna");
        assert_eq!(surgery.species_name, "Canino");
        assert_eq!(surgery.owner_name, "Juan Pérez");
        assert_eq!(surgery.owner_phone.as_deref(), Some("+57 300 1234567"));
        assert_eq!(surgery.veterinarian_id, Some(5));
        assert_eq!(surgery.veterinarian_name.as_deref(), Some("Dr. Ramos"));
        assert_eq!(surgery.surgery_type, "Cesárea");
        assert_eq!(surgery.scheduled_at, "2026-08-15 09:00:00");
        assert_eq!(surgery.anesthesia_type.as_deref(), Some("General"));
        assert_eq!(surgery.preoperative_notes.as_deref(), Some("Paciente en ayunas"));
        assert_eq!(surgery.postoperative_notes.as_deref(), Some("Cirugía exitosa"));
        assert_eq!(surgery.status, "PROGRAMADA");
    }

    #[test]
    fn test_map_surgery_optional_fields_none() {
        let row: SurgeryRow = (
            2, 20, "Michi".into(), "Felino".into(), "María López".into(),
            None, None, None, "Esterilización".into(),
            "2026-08-20 14:00:00".into(), None, None, None, "PROGRAMADA".into(),
        );
        let surgery = map_surgery(row);

        assert_eq!(surgery.id, 2);
        assert_eq!(surgery.owner_phone, None);
        assert_eq!(surgery.veterinarian_id, None);
        assert_eq!(surgery.veterinarian_name, None);
        assert_eq!(surgery.anesthesia_type, None);
        assert_eq!(surgery.preoperative_notes, None);
        assert_eq!(surgery.postoperative_notes, None);
    }

    #[test]
    fn test_surgery_status_values() {
        // Verificar que los estados válidos son los esperados
        let valid_statuses = ["PROGRAMADA", "EN_CURSO", "COMPLETADA", "CANCELADA"];
        for status in valid_statuses {
            let row: SurgeryRow = (
                1, 10, "Test".into(), "Canino".into(), "Owner".into(),
                None, None, None, "Test".into(),
                "2026-08-15 09:00:00".into(), None, None, None, status.into(),
            );
            let surgery = map_surgery(row);
            assert_eq!(surgery.status, status);
        }
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
    fn test_create_surgery() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        let input = CreateSurgeryInput {
            patient_id,
            surgery_type: "Cesárea".into(),
            scheduled_at: "2026-08-15 09:00:00".into(),
            anesthesia_type: Some("General".into()),
            preoperative_notes: Some("Ayunas 8 horas".into()),
            postoperative_notes: None,
        };

        let surgery = create(&mut conn, &input, Some(1)).unwrap();
        assert_eq!(surgery.surgery_type, "Cesárea");
        assert_eq!(surgery.status, "PROGRAMADA");
        assert_eq!(surgery.patient_id, patient_id);
        assert!(surgery.id > 0);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_surgery() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Esterilización', '2026-08-20 14:00:00', 'PROGRAMADA')",
            (&patient_id,),
        ).unwrap();

        let surgery = get(&mut conn, 1).unwrap();
        assert!(surgery.is_some());
        let s = surgery.unwrap();
        assert_eq!(s.surgery_type, "Esterilización");
        assert_eq!(s.patient_name, "Luna");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_surgeries() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Cesárea', '2026-08-15 09:00:00', 'PROGRAMADA')",
            (&patient_id,),
        ).unwrap();
        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (2, ?, 'Esterilización', '2026-08-20 14:00:00', 'COMPLETADA')",
            (&patient_id,),
        ).unwrap();

        // Filtrar por estado
        let surgeries = list(&mut conn, Some("PROGRAMADA"), None).unwrap();
        assert_eq!(surgeries.len(), 1);
        assert_eq!(surgeries[0].surgery_type, "Cesárea");

        // Sin filtro
        let surgeries = list(&mut conn, None, None).unwrap();
        assert_eq!(surgeries.len(), 2);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_surgery_status_transition() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Cesárea', '2026-08-15 09:00:00', 'PROGRAMADA')",
            (&patient_id,),
        ).unwrap();

        // PROGRAMADA -> EN_CURSO
        let updated = set_status(&mut conn, 1, "EN_CURSO").unwrap();
        assert_eq!(updated.status, "EN_CURSO");

        // EN_CURSO -> COMPLETADA
        let updated = set_status(&mut conn, 1, "COMPLETADA").unwrap();
        assert_eq!(updated.status, "COMPLETADA");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_surgery_status_invalid_transition() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Cesárea', '2026-08-15 09:00:00', 'PROGRAMADA')",
            (&patient_id,),
        ).unwrap();

        // PROGRAMADA -> ESTADO_INVALIDO (no permitido)
        let result = set_status(&mut conn, 1, "ESTADO_INVALIDO");
        assert!(result.is_err());

        test_helpers::cleanup_test_db(&db_path);
    }
}
