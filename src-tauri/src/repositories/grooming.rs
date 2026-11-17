use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::grooming::{
    FinishGroomingSessionInput, GroomingDayLine, GroomingReminder, GroomingSession,
    UpsertGroomingSessionInput, VisitAttachment,
};
use crate::repositories::next_id;

// ============================== HELPERS =====================================

type GroomingRow = (
    i32,            // g.id
    i32,            // vs.visit_id
    i32,            // g.visit_service_id
    String,         // service name
    i32,            // patient_id
    String,         // patient name
    String,         // owner name
    Option<i32>,    // groomer_id
    Option<String>, // groomer_name
    Option<String>, // style
    Option<String>, // bath_type
    Option<String>, // coat_condition
    i32,            // matting_level
    Option<f64>,    // weight_kg
    Option<String>, // behavior
    Option<String>, // products_used
    Option<String>, // started_at
    Option<String>, // finished_at
    Option<String>, // next_suggested_at
    Option<String>, // notes
    String,         // created_at
    String,         // updated_at
);

fn map_grooming(r: GroomingRow) -> GroomingSession {
    GroomingSession {
        id: r.0,
        visit_id: r.1,
        visit_service_id: r.2,
        service_name: r.3,
        patient_id: r.4,
        patient_name: r.5,
        owner_name: r.6,
        groomer_id: r.7,
        groomer_name: r.8,
        style: r.9,
        bath_type: r.10,
        coat_condition: r.11,
        matting_level: r.12,
        weight_kg: r.13,
        behavior: r.14,
        products_used: r.15,
        started_at: r.16,
        finished_at: r.17,
        next_suggested_at: r.18,
        notes: r.19,
        created_at: r.20,
        updated_at: r.21,
    }
}

const GROOMING_SELECT: &str = "
    SELECT g.ID,
           vs.VISIT_ID, vs.ID,
           s.NAME,
           p.ID, p.NAME, o.FULL_NAME,
           g.GROOMER_ID, u.FULL_NAME,
           g.STYLE, g.BATH_TYPE, g.COAT_CONDITION,
           g.MATTING_LEVEL, g.WEIGHT_KG, g.BEHAVIOR, g.PRODUCTS_USED,
           LEFT(CAST(g.STARTED_AT AS VARCHAR(60)), 19),
           LEFT(CAST(g.FINISHED_AT AS VARCHAR(60)), 19),
           CAST(g.NEXT_SUGGESTED_AT AS VARCHAR(10)),
           g.NOTES,
           LEFT(CAST(g.CREATED_AT AS VARCHAR(60)), 19),
           LEFT(CAST(g.UPDATED_AT AS VARCHAR(60)), 19)
    FROM GROOMING_SESSIONS g
    JOIN VISIT_SERVICES vs ON vs.ID = g.VISIT_SERVICE_ID
    JOIN SERVICES s ON s.ID = vs.SERVICE_ID
    JOIN VISITS v ON v.ID = vs.VISIT_ID
    JOIN PATIENTS p ON p.ID = v.PATIENT_ID
    JOIN OWNERS o ON o.ID = v.OWNER_ID
    LEFT JOIN USERS u ON u.ID = g.GROOMER_ID";

// ============================== SESIONES ====================================

/// Sesión de la línea de servicio, o `None` si aún no tiene ficha.
pub fn get_grooming_session_by_service(
    conn: &mut SimpleConnection,
    visit_service_id: i32,
) -> Result<Option<GroomingSession>, AppError> {
    let row: Option<GroomingRow> = conn
        .query_first(
            &format!("{GROOMING_SELECT} WHERE g.VISIT_SERVICE_ID = ?"),
            (&visit_service_id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_grooming))
}

/// Sesión completa por id.
pub fn get_grooming_session(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<GroomingSession>, AppError> {
    let row: Option<GroomingRow> = conn
        .query_first(&format!("{GROOMING_SELECT} WHERE g.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_grooming))
}

/// Todas las sesiones de estética de un paciente (historial clínico),
/// de la más reciente a la más antigua.
pub fn list_grooming_sessions_by_patient(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<GroomingSession>, AppError> {
    let rows: Vec<GroomingRow> = conn
        .query(
            &format!(
                "{GROOMING_SELECT} WHERE v.PATIENT_ID = ? ORDER BY g.CREATED_AT DESC, g.ID DESC"
            ),
            (&patient_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_grooming).collect())
}

/// Valida que la línea sea de categoría PELUQUERIA y devuelve (visit_id, service_name).
fn grooming_line(
    conn: &mut SimpleConnection,
    visit_service_id: i32,
) -> Result<(i32, String), AppError> {
    let row: Option<(i32, String)> = conn
        .query_first(
            "SELECT vs.VISIT_ID, s.NAME
             FROM VISIT_SERVICES vs
             JOIN SERVICES s ON s.ID = vs.SERVICE_ID
             JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
             WHERE vs.ID = ? AND c.CODE = 'PELUQUERIA'",
            (&visit_service_id,),
        )
        .map_err(AppError::from)?;
    row.ok_or_else(|| {
        AppError::Validation(format!(
            "La línea {visit_service_id} no es un servicio de peluquería"
        ))
    })
}

/// Crea o actualiza la sesión de peluquería de una línea (1 sesión por línea).
pub fn upsert_grooming_session(
    conn: &mut SimpleConnection,
    input: &UpsertGroomingSessionInput,
) -> Result<GroomingSession, AppError> {
    // Solo acepta líneas de la categoría PELUQUERIA (validación).
    grooming_line(conn, input.visit_service_id)?;
    if !(0..=3).contains(&input.matting_level) {
        return Err(AppError::Validation(
            "El nivel de enredos debe estar entre 0 y 3".into(),
        ));
    }

    // El peluquero (si llega) debe existir.
    if let Some(gid) = input.groomer_id {
        let exists: Option<(i32,)> = conn
            .query_first("SELECT ID FROM USERS WHERE ID = ?", (&gid,))
            .map_err(AppError::from)?;
        if exists.is_none() {
            return Err(AppError::Validation(format!(
                "El peluquero {gid} no existe"
            )));
        }
    }

    let existing: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM GROOMING_SESSIONS WHERE VISIT_SERVICE_ID = ?",
            (&input.visit_service_id,),
        )
        .map_err(AppError::from)?;

    match existing {
        Some((id,)) => {
            conn.execute(
                "UPDATE GROOMING_SESSIONS
                 SET GROOMER_ID = ?, STYLE = ?, BATH_TYPE = ?, COAT_CONDITION = ?,
                     MATTING_LEVEL = ?, WEIGHT_KG = ?, BEHAVIOR = ?, PRODUCTS_USED = ?,
                     STARTED_AT = COALESCE(?, STARTED_AT), NEXT_SUGGESTED_AT = ?,
                     NOTES = ?, UPDATED_AT = CURRENT_TIMESTAMP
                 WHERE ID = ?",
                (
                    &input.groomer_id,
                    &input.style,
                    &input.bath_type,
                    &input.coat_condition,
                    &input.matting_level,
                    &input.weight_kg,
                    &input.behavior,
                    &input.products_used,
                    &input.started_at,
                    &input.next_suggested_at,
                    &input.notes,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
        }
        None => {
            let id = next_id(conn, "GEN_GROOMING_SESSIONS_ID")?;
            conn.execute(
                "INSERT INTO GROOMING_SESSIONS
                    (ID, VISIT_SERVICE_ID, GROOMER_ID, STYLE, BATH_TYPE, COAT_CONDITION,
                     MATTING_LEVEL, WEIGHT_KG, BEHAVIOR, PRODUCTS_USED,
                     STARTED_AT, NEXT_SUGGESTED_AT, NOTES)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE(?, CURRENT_TIMESTAMP), ?, ?)",
                (
                    &id,
                    &input.visit_service_id,
                    &input.groomer_id,
                    &input.style,
                    &input.bath_type,
                    &input.coat_condition,
                    &input.matting_level,
                    &input.weight_kg,
                    &input.behavior,
                    &input.products_used,
                    &input.started_at,
                    &input.next_suggested_at,
                    &input.notes,
                ),
            )
            .map_err(AppError::from)?;
        }
    }

    // Si la sesión fijó peluquero y la línea no tiene, se hereda a la línea.
    if let Some(gid) = input.groomer_id {
        conn.execute(
            "UPDATE VISIT_SERVICES
             SET PROFESSIONAL_ID = ?
             WHERE ID = ? AND PROFESSIONAL_ID IS NULL",
            (&gid, &input.visit_service_id),
        )
        .map_err(AppError::from)?;
    }

    get_grooming_session_by_service(conn, input.visit_service_id)?
        .ok_or_else(|| AppError::Internal("Sesión guardada pero no recuperada".into()))
}

/// Cierra la sesión: FINISHED_AT (por defecto ahora), próximo turno sugerido y
/// notas. Exige peluquero asignado (en la sesión o en la línea) y marca la
/// línea COMPLETADA.
pub fn finish_grooming_session(
    conn: &mut SimpleConnection,
    visit_service_id: i32,
    input: &FinishGroomingSessionInput,
) -> Result<GroomingSession, AppError> {
    grooming_line(conn, visit_service_id)?;

    let session = get_grooming_session_by_service(conn, visit_service_id)?.ok_or_else(|| {
        AppError::NotFound(format!(
            "La línea {visit_service_id} no tiene sesión de peluquería"
        ))
    })?;

    // No se cierra sin profesional asignado (regla del plan, §6.3).
    let has_groomer = session.groomer_id.is_some() || {
        let row: Option<(Option<i32>,)> = conn
            .query_first(
                "SELECT PROFESSIONAL_ID FROM VISIT_SERVICES WHERE ID = ?",
                (&visit_service_id,),
            )
            .map_err(AppError::from)?;
        row.and_then(|(p,)| p).is_some()
    };
    if !has_groomer {
        return Err(AppError::Validation(
            "Asigna un peluquero antes de finalizar la sesión".into(),
        ));
    }

    // La línea cancelada no se puede cerrar como completada.
    let line_status: Option<(String,)> = conn
        .query_first(
            "SELECT STATUS FROM VISIT_SERVICES WHERE ID = ?",
            (&visit_service_id,),
        )
        .map_err(AppError::from)?;
    if let Some((st,)) = line_status {
        if st == "CANCELADA" {
            return Err(AppError::Validation(
                "La línea de servicio está cancelada".into(),
            ));
        }
    }

    // UPDATE con RETURNING no es portable en el driver: update + re-read.
    if input.finished_at.is_some() {
        conn.execute(
            "UPDATE GROOMING_SESSIONS SET FINISHED_AT = ?, NEXT_SUGGESTED_AT = ?,
                NOTES = COALESCE(?, NOTES), UPDATED_AT = CURRENT_TIMESTAMP
             WHERE VISIT_SERVICE_ID = ?",
            (&input.finished_at, &input.next_suggested_at, &input.notes, &visit_service_id),
        )
        .map_err(AppError::from)?;
    } else {
        conn.execute(
            "UPDATE GROOMING_SESSIONS SET FINISHED_AT = CURRENT_TIMESTAMP, NEXT_SUGGESTED_AT = ?,
                NOTES = COALESCE(?, NOTES), UPDATED_AT = CURRENT_TIMESTAMP
             WHERE VISIT_SERVICE_ID = ?",
            (&input.next_suggested_at, &input.notes, &visit_service_id),
        )
        .map_err(AppError::from)?;
    }

    // Cierra también la línea de la visita.
    if input.finished_at.is_some() {
        conn.execute(
            "UPDATE VISIT_SERVICES SET STATUS = 'COMPLETADA', FINISHED_AT = ?
             WHERE ID = ?",
            (&input.finished_at, &visit_service_id),
        )
        .map_err(AppError::from)?;
    } else {
        conn.execute(
            "UPDATE VISIT_SERVICES SET STATUS = 'COMPLETADA', FINISHED_AT = CURRENT_TIMESTAMP
             WHERE ID = ?",
            (&visit_service_id,),
        )
        .map_err(AppError::from)?;
    }

    get_grooming_session_by_service(conn, visit_service_id)?
        .ok_or_else(|| AppError::Internal("Sesión finalizada pero no recuperada".into()))
}

// ============================== TABLERO =====================================

/// Líneas de peluquería de las visitas de una fecha (tablero del día).
pub fn list_grooming_day(
    conn: &mut SimpleConnection,
    date: &str,
) -> Result<Vec<GroomingDayLine>, AppError> {
    let rows: Vec<(
        i32,            // vs.id
        i32,            // visit_id
        String,         // visit_code
        i32,            // patient_id
        String,         // patient_name
        String,         // owner_name
        i32,            // service_id
        String,         // service_name
        Option<i32>,    // professional_id
        Option<String>, // professional_name
        String,         // line_status
        String,         // arrived_at
        String,         // visit_status
        Option<i32>,    // session_id
        Option<String>, // session_started_at
        Option<String>, // session_finished_at
        Option<i32>,    // matting_level
        Option<String>, // next_suggested_at
    )> = conn
        .query(
            "
            SELECT vs.ID, v.ID, v.CODE, p.ID, p.NAME, o.FULL_NAME,
                   s.ID, s.NAME, vs.PROFESSIONAL_ID, u.FULL_NAME,
                   vs.STATUS,
                   LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 19),
                   v.STATUS,
                   g.ID,
                   LEFT(CAST(g.STARTED_AT AS VARCHAR(60)), 19),
                   LEFT(CAST(g.FINISHED_AT AS VARCHAR(60)), 19),
                   g.MATTING_LEVEL,
                   CAST(g.NEXT_SUGGESTED_AT AS VARCHAR(10))
            FROM VISIT_SERVICES vs
            JOIN SERVICES s ON s.ID = vs.SERVICE_ID
            JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
            JOIN VISITS v ON v.ID = vs.VISIT_ID
            JOIN PATIENTS p ON p.ID = v.PATIENT_ID
            JOIN OWNERS o ON o.ID = v.OWNER_ID
            LEFT JOIN USERS u ON u.ID = vs.PROFESSIONAL_ID
            LEFT JOIN GROOMING_SESSIONS g ON g.VISIT_SERVICE_ID = vs.ID
            WHERE c.CODE = 'PELUQUERIA'
              AND LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 10) = ?
              AND vs.STATUS <> 'CANCELADA'
            ORDER BY v.ARRIVED_AT, vs.ID",
            (&date,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let session_state = match (r.13, r.15) {
                (None, _) => "SIN_SESION",
                (_, Some(_)) => "FINALIZADA",
                (Some(_), None) => "EN_PROCESO",
            };
            crate::models::grooming::GroomingDayLine {
                visit_service_id: r.0,
                visit_id: r.1,
                visit_code: r.2,
                patient_id: r.3,
                patient_name: r.4,
                owner_name: r.5,
                service_id: r.6,
                service_name: r.7,
                professional_id: r.8,
                professional_name: r.9,
                line_status: r.10,
                arrived_at: r.11,
                visit_status: r.12,
                session_state: session_state.into(),
                next_suggested_at: r.17,
                matting_level: r.16.unwrap_or(0),
            }
        })
        .collect())
}

/// Próximos turnos sugeridos por sesiones ya cerradas (recordatorios).
pub fn list_grooming_reminders(
    conn: &mut SimpleConnection,
) -> Result<Vec<GroomingReminder>, AppError> {
    let rows: Vec<(
        i32,            // session_id
        i32,            // visit_service_id
        String,         // patient_name
        String,         // owner_name
        String,         // service_name
        String,         // next_suggested_at
        Option<String>, // groomer_name
    )> = conn
        .query(
            "
            SELECT g.ID, g.VISIT_SERVICE_ID, p.NAME, o.FULL_NAME,
                   s.NAME, CAST(g.NEXT_SUGGESTED_AT AS VARCHAR(10)), u.FULL_NAME
            FROM GROOMING_SESSIONS g
            JOIN VISIT_SERVICES vs ON vs.ID = g.VISIT_SERVICE_ID
            JOIN SERVICES s ON s.ID = vs.SERVICE_ID
            JOIN VISITS v ON v.ID = vs.VISIT_ID
            JOIN PATIENTS p ON p.ID = v.PATIENT_ID
            JOIN OWNERS o ON o.ID = v.OWNER_ID
            LEFT JOIN USERS u ON u.ID = g.GROOMER_ID
            WHERE g.NEXT_SUGGESTED_AT IS NOT NULL
              AND g.NEXT_SUGGESTED_AT >= CURRENT_DATE
            ORDER BY g.NEXT_SUGGESTED_AT, g.ID
            ROWS 20",
            (),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(
            |(session_id, visit_service_id, patient_name, owner_name, service_name, next_suggested_at, groomer_name)| {
                crate::models::grooming::GroomingReminder {
                    session_id,
                    visit_service_id,
                    patient_name,
                    owner_name,
                    service_name,
                    next_suggested_at,
                    groomer_name,
                }
            },
        )
        .collect())
}

// ============================== ADJUNTOS ====================================

type AttachmentRow = (
    i32,            // id
    i32,            // visit_id
    String,         // file_name
    String,         // file_path
    Option<String>, // mime_type
    Option<String>, // kind
    String,         // created_at
);

fn map_attachment(r: AttachmentRow) -> VisitAttachment {
    VisitAttachment {
        id: r.0,
        visit_id: r.1,
        file_name: r.2,
        file_path: r.3,
        mime_type: r.4,
        kind: r.5,
        created_at: r.6,
    }
}

const ATTACHMENT_SELECT: &str = "
    SELECT ID, VISIT_ID, FILE_NAME, FILE_PATH, MIME_TYPE, KIND,
           LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
    FROM VISIT_ATTACHMENTS";

/// Adjuntos de una visita (fotos antes/después y documentos), más recientes primero.
pub fn list_visit_attachments(
    conn: &mut SimpleConnection,
    visit_id: i32,
) -> Result<Vec<VisitAttachment>, AppError> {
    let rows: Vec<AttachmentRow> = conn
        .query(
            &format!("{ATTACHMENT_SELECT} WHERE VISIT_ID = ? ORDER BY CREATED_AT DESC, ID DESC"),
            (&visit_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_attachment).collect())
}

pub fn get_visit_attachment(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<VisitAttachment>, AppError> {
    let row: Option<AttachmentRow> = conn
        .query_first(&format!("{ATTACHMENT_SELECT} WHERE ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_attachment))
}

pub fn insert_visit_attachment(
    conn: &mut SimpleConnection,
    visit_id: i32,
    file_name: &str,
    file_path: &str,
    mime_type: Option<String>,
    kind: Option<String>,
) -> Result<VisitAttachment, AppError> {
    let id = next_id(conn, "GEN_VISIT_ATTACHMENTS_ID")?;
    conn.execute(
        "INSERT INTO VISIT_ATTACHMENTS (ID, VISIT_ID, FILE_NAME, FILE_PATH, MIME_TYPE, KIND)
         VALUES (?, ?, ?, ?, ?, ?)",
        (&id, &visit_id, &file_name, &file_path, &mime_type, &kind),
    )
    .map_err(AppError::from)?;

    get_visit_attachment(conn, id)?
        .ok_or_else(|| AppError::Internal("Adjunto no encontrado tras insertar".into()))
}

pub fn delete_visit_attachment(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM VISIT_ATTACHMENTS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visit::AddVisitServiceInput;
    use crate::repositories::visits as visit_repo;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        test_helpers::insert_test_patient(&mut conn);
        (conn, db_path)
    }

    /// Visita de paciente 1 con una línea del servicio de peluquería 11.
    fn grooming_visit(conn: &mut SimpleConnection) -> crate::models::visit::Visit {
        visit_repo::create_visit(
            conn,
            &crate::models::visit::CreateVisitInput {
                patient_id: 1,
                owner_id: None,
                priority: None,
                notes: None,
                services: vec![AddVisitServiceInput {
                    service_id: 11, // Baño del seed 0025 (categoría PELUQUERIA)
                    professional_id: None,
                    quantity: 1,
                    notes: None,
                }],
            },
            None,
            None,
        )
        .unwrap()
    }

    fn upsert_input(visit_service_id: i32) -> UpsertGroomingSessionInput {
        UpsertGroomingSessionInput {
            visit_service_id,
            groomer_id: None,
            style: Some("Corte de raza".into()),
            bath_type: Some("Medicado".into()),
            coat_condition: Some("Pelaje grueso".into()),
            matting_level: 1,
            weight_kg: Some(12.5),
            behavior: Some("Tranquilo".into()),
            products_used: Some("Shampoo medicado".into()),
            started_at: None,
            next_suggested_at: Some("2026-12-01".into()),
            notes: None,
        }
    }

    #[test]
    fn test_upsert_creates_and_updates_session() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);
        let line = &visit.services[0];

        // Crear.
        let session = upsert_grooming_session(&mut conn, &upsert_input(line.id)).unwrap();
        assert_eq!(session.visit_id, visit.id);
        assert_eq!(session.visit_service_id, line.id);
        assert_eq!(session.patient_name, "Luna");
        assert_eq!(session.style.as_deref(), Some("Corte de raza"));
        assert_eq!(session.matting_level, 1);
        assert!(session.started_at.is_some());
        assert_eq!(session.next_suggested_at.as_deref(), Some("2026-12-01"));

        // Actualizar (una sola fila por línea).
        let mut upd = upsert_input(line.id);
        upd.matting_level = 3;
        upd.style = Some("Corte higiénico".into());
        let updated = upsert_grooming_session(&mut conn, &upd).unwrap();
        assert_eq!(updated.id, session.id);
        assert_eq!(updated.matting_level, 3);
        assert_eq!(updated.style.as_deref(), Some("Corte higiénico"));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_upsert_rejects_non_grooming_service() {
        let (mut conn, db_path) = setup();
        // Servicio 1 = Consulta general (categoría CONSULTA).
        let visit = visit_repo::create_visit(
            &mut conn,
            &crate::models::visit::CreateVisitInput {
                patient_id: 1,
                owner_id: None,
                priority: None,
                notes: None,
                services: vec![AddVisitServiceInput {
                    service_id: 1,
                    professional_id: None,
                    quantity: 1,
                    notes: None,
                }],
            },
            None,
            None,
        )
        .unwrap();

        let err = upsert_grooming_session(&mut conn, &upsert_input(visit.services[0].id));
        assert!(matches!(err, Err(AppError::Validation(_))));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_upsert_rejects_bad_matting_and_groomer() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);
        let line_id = visit.services[0].id;

        let mut bad = upsert_input(line_id);
        bad.matting_level = 5;
        assert!(upsert_grooming_session(&mut conn, &bad).is_err());

        let mut bad_groomer = upsert_input(line_id);
        bad_groomer.groomer_id = Some(999);
        assert!(upsert_grooming_session(&mut conn, &bad_groomer).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_finish_requires_groomer_and_closes_line() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);
        let line_id = visit.services[0].id;

        upsert_grooming_session(&mut conn, &upsert_input(line_id)).unwrap();

        // Sin peluquero: no se puede cerrar.
        let err = finish_grooming_session(
            &mut conn,
            line_id,
            &FinishGroomingSessionInput {
                finished_at: None,
                next_suggested_at: None,
                notes: None,
            },
        );
        assert!(
            matches!(err, Err(AppError::Validation(_))),
            "error inesperado: {err:?}"
        );

        // Con peluquero: cierra sesión y línea.
        let mut with_groomer = upsert_input(line_id);
        with_groomer.groomer_id = Some(1); // admin del seed
        upsert_grooming_session(&mut conn, &with_groomer).unwrap();

        let finished = finish_grooming_session(
            &mut conn,
            line_id,
            &FinishGroomingSessionInput {
                finished_at: None,
                next_suggested_at: Some("2026-12-15".into()),
                notes: Some("Salida tranquila".into()),
            },
        )
        .unwrap();
        assert!(finished.finished_at.is_some());
        assert_eq!(finished.next_suggested_at.as_deref(), Some("2026-12-15"));
        assert_eq!(finished.notes.as_deref(), Some("Salida tranquila"));

        // La línea quedó COMPLETADA con FINISHED_AT.
        let updated = visit_repo::get_visit(&mut conn, visit.id).unwrap().unwrap();
        let line = updated.services.iter().find(|s| s.id == line_id).unwrap();
        assert_eq!(line.status, "COMPLETADA");
        assert!(line.finished_at.is_some());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_finish_without_session_not_found() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);
        let err = finish_grooming_session(
            &mut conn,
            visit.services[0].id,
            &FinishGroomingSessionInput {
                finished_at: None,
                next_suggested_at: None,
                notes: None,
            },
        );
        assert!(matches!(err, Err(AppError::NotFound(_))));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_grooming_day_and_reminders() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);
        let line_id = visit.services[0].id;

        // Línea de consulta (otro servicio) NO aparece en el tablero.
        let other_visit = visit_repo::create_visit(
            &mut conn,
            &crate::models::visit::CreateVisitInput {
                patient_id: 1,
                owner_id: None,
                priority: None,
                notes: None,
                services: vec![AddVisitServiceInput {
                    service_id: 1, // Consulta general
                    professional_id: None,
                    quantity: 1,
                    notes: None,
                }],
            },
            None,
            None,
        )
        .unwrap();

        let day = list_grooming_day(&mut conn, &visit.arrived_at[0..10]).unwrap();
        assert_eq!(day.len(), 1);
        assert_eq!(day[0].visit_service_id, line_id);
        assert_eq!(day[0].session_state, "SIN_SESION");
        assert_eq!(day[0].visit_code, visit.code);
        // El tablero no contiene la línea de consulta.
        assert!(!day
            .iter()
            .any(|l| l.visit_service_id == other_visit.services[0].id));

        // Ficha abierta sin iniciar → ABIERTA.
        upsert_grooming_session(&mut conn, &upsert_input(line_id)).unwrap();
        let day = list_grooming_day(&mut conn, &visit.arrived_at[0..10]).unwrap();
        assert_eq!(day[0].session_state, "EN_PROCESO"); // started_at = ahora
        assert_eq!(day[0].matting_level, 1);

        // Cerrar la sesión con próximo turno → FINALIZADA + recordatorio.
        let mut with_groomer = upsert_input(line_id);
        with_groomer.groomer_id = Some(1);
        upsert_grooming_session(&mut conn, &with_groomer).unwrap();
        finish_grooming_session(
            &mut conn,
            line_id,
            &FinishGroomingSessionInput {
                finished_at: None,
                next_suggested_at: Some("2099-01-15".into()),
                notes: None,
            },
        )
        .unwrap();

        let day = list_grooming_day(&mut conn, &visit.arrived_at[0..10]).unwrap();
        assert_eq!(day[0].session_state, "FINALIZADA");
        assert_eq!(day[0].next_suggested_at.as_deref(), Some("2099-01-15"));

        let reminders = list_grooming_reminders(&mut conn).unwrap();
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].visit_service_id, line_id);
        assert_eq!(reminders[0].next_suggested_at, "2099-01-15");
        assert_eq!(reminders[0].patient_name, "Luna");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_visit_attachments_crud() {
        let (mut conn, db_path) = setup();
        let visit = grooming_visit(&mut conn);

        let att = insert_visit_attachment(
            &mut conn,
            visit.id,
            "antes.png",
            "C:\\app_data\\attachments\\a1.png",
            Some("image/png".into()),
            Some("BEFORE".into()),
        )
        .unwrap();
        assert_eq!(att.visit_id, visit.id);
        assert_eq!(att.kind.as_deref(), Some("BEFORE"));

        let all = list_visit_attachments(&mut conn, visit.id).unwrap();
        assert_eq!(all.len(), 1);

        delete_visit_attachment(&mut conn, att.id).unwrap();
        let all = list_visit_attachments(&mut conn, visit.id).unwrap();
        assert!(all.is_empty());
        test_helpers::cleanup_test_db(&db_path);
    }
}
