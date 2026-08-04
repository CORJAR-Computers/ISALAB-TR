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

type SurgeryRow = (
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

fn map_surgery(r: SurgeryRow) -> Surgery {
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
