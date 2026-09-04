use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::notification::NotificationLogEntry;

pub(crate) type NotificationRow = (
    i32,
    Option<i32>,
    i32,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

fn map_row(r: NotificationRow) -> NotificationLogEntry {
    NotificationLogEntry {
        id: r.0,
        result_id: r.1,
        sample_id: r.2,
        channel: r.3,
        recipient_name: r.4,
        recipient_address: r.5,
        status: r.6,
        sent_at: r.7,
        acked_at: r.8,
        acked_by: r.9,
        note: r.10,
        created_at: r.11,
    }
}

/// Datos para insertar una fila en NOTIFICATION_LOG.
pub struct NewNotification<'a> {
    pub result_id: Option<i32>,
    pub sample_id: i32,
    pub channel: &'a str,
    pub recipient_name: Option<&'a str>,
    pub recipient_address: Option<&'a str>,
    pub status: &'a str,
    pub note: Option<&'a str>,
}

/// Inserta una fila en NOTIFICATION_LOG y la devuelve completa.
pub fn log(
    conn: &mut SimpleConnection,
    input: &NewNotification<'_>,
) -> Result<NotificationLogEntry, AppError> {
    conn.execute(
        "INSERT INTO NOTIFICATION_LOG
            (RESULT_ID, SAMPLE_ID, CHANNEL, RECIPIENT_NAME, RECIPIENT_ADDRESS,
             STATUS, SENT_AT, NOTE)
         VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?)",
        (
            &input.result_id,
            &input.sample_id,
            &input.channel,
            &input.recipient_name,
            &input.recipient_address,
            &input.status,
            &input.note,
        ),
    )
    .map_err(AppError::from)?;

    let id: Option<(i32,)> = conn
        .query_first(
            "SELECT FIRST 1 ID FROM NOTIFICATION_LOG
             WHERE SAMPLE_ID = ? ORDER BY ID DESC",
            (&input.sample_id,),
        )
        .map_err(AppError::from)?;
    let (id,) =
        id.ok_or_else(|| AppError::Internal("No se pudo recuperar la notificación".into()))?;
    get(conn, id)?.ok_or_else(|| AppError::Internal("Notificación no recuperada".into()))
}

/// Devuelve una entrada por id.
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<NotificationLogEntry>, AppError> {
    let row: Option<NotificationRow> = conn
        .query_first(
            "SELECT ID, RESULT_ID, SAMPLE_ID, CHANNEL, RECIPIENT_NAME,
                    RECIPIENT_ADDRESS, STATUS,
                    LEFT(CAST(SENT_AT AS VARCHAR(60)), 19),
                    LEFT(CAST(ACKED_AT AS VARCHAR(60)), 19),
                    ACKED_BY, NOTE,
                    LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
             FROM NOTIFICATION_LOG WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_row))
}

/// Historial de notificaciones de una muestra (más reciente primero).
pub fn list_by_sample(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Vec<NotificationLogEntry>, AppError> {
    let rows: Vec<NotificationRow> = conn
        .query(
            "SELECT ID, RESULT_ID, SAMPLE_ID, CHANNEL, RECIPIENT_NAME,
                    RECIPIENT_ADDRESS, STATUS,
                    LEFT(CAST(SENT_AT AS VARCHAR(60)), 19),
                    LEFT(CAST(ACKED_AT AS VARCHAR(60)), 19),
                    ACKED_BY, NOTE,
                    LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
             FROM NOTIFICATION_LOG
             WHERE SAMPLE_ID = ?
             ORDER BY CREATED_AT DESC, ID DESC",
            (&sample_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_row).collect())
}

/// Registra la confirmación (acknowledgment) del analista para una lista de
/// resultados críticos: una fila ACKNOWLEDGED por resultado.
pub fn acknowledge(
    conn: &mut SimpleConnection,
    sample_id: i32,
    result_ids: &[i32],
    acked_by: &str,
) -> Result<Vec<NotificationLogEntry>, AppError> {
    let mut created = Vec::with_capacity(result_ids.len());
    for rid in result_ids {
        conn.execute(
            "INSERT INTO NOTIFICATION_LOG
                (RESULT_ID, SAMPLE_ID, CHANNEL, STATUS, ACKED_AT, ACKED_BY)
             VALUES (?, ?, 'MANUAL', 'ACKNOWLEDGED', CURRENT_TIMESTAMP, ?)",
            (rid, &sample_id, acked_by),
        )
        .map_err(AppError::from)?;
        let id: Option<(i32,)> = conn
            .query_first(
                "SELECT FIRST 1 ID FROM NOTIFICATION_LOG
                 WHERE RESULT_ID = ? AND CHANNEL = 'MANUAL' ORDER BY ID DESC",
                (rid,),
            )
            .map_err(AppError::from)?;
        if let Some((id,)) = id {
            if let Some(e) = get(conn, id)? {
                created.push(e);
            }
        }
    }
    Ok(created)
}

/// Datos del propietario (nombre + email) para una muestra, para el envío
/// de avisos por email. Devuelve None si el propietario no tiene email.
pub fn owner_email_for_sample(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Option<(String, String)>, AppError> {
    let row: Option<(String, String)> = conn
        .query_first(
            "SELECT FIRST 1 o.FULL_NAME, o.EMAIL
             FROM SAMPLES s
             JOIN PATIENTS p ON p.ID = s.PATIENT_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             WHERE s.ID = ? AND o.EMAIL IS NOT NULL",
            (&sample_id,),
        )
        .map_err(AppError::from)?;
    Ok(row)
}

/// Nombre del paciente de una muestra (para personalizar el asunto/cuerpo).
pub fn patient_name_for_sample(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Option<String>, AppError> {
    let row: Option<(String,)> = conn
        .query_first(
            "SELECT FIRST 1 p.NAME
             FROM SAMPLES s
             JOIN PATIENTS p ON p.ID = s.PATIENT_ID
             WHERE s.ID = ?",
            (&sample_id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(|(n,)| n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    #[test]
    fn test_notification_log_ack_list_roundtrip() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO LAB_RESULTS (ID, SAMPLE_ID, ANALYTE_ID, RESULT_VALUE, STATUS)
             VALUES (1, 1, 1, 55.0, 'CRITICO_ALTO')",
            (),
        )
        .unwrap();

        // Envío por email → SENT
        let sent = log(
            &mut conn,
            &NewNotification {
                result_id: Some(1),
                sample_id: 1,
                channel: "EMAIL",
                recipient_name: Some("Propietario"),
                recipient_address: Some("owner@example.com"),
                status: "SENT",
                note: Some("Prueba"),
            },
        )
        .unwrap();
        assert_eq!(sent.status, "SENT");
        assert_eq!(sent.channel, "EMAIL");

        // Confirmación del analista → ACKNOWLEDGED
        let acks = acknowledge(&mut conn, 1, &[1], "vet_ana").unwrap();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, "ACKNOWLEDGED");
        assert_eq!(acks[0].acked_by.as_deref(), Some("vet_ana"));

        // Listado: ACK primero (más reciente)
        let all = list_by_sample(&mut conn, 1).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].status, "ACKNOWLEDGED");

        // Email del propietario
        conn.execute(
            "UPDATE OWNERS SET EMAIL = 'owner@example.com' WHERE ID = ?",
            (&patient_id,),
        )
        .unwrap();
        let owner = owner_email_for_sample(&mut conn, 1).unwrap();
        assert!(owner.is_some());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_owner_email_missing_returns_none() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        assert!(owner_email_for_sample(&mut conn, 1).unwrap().is_none());
        test_helpers::cleanup_test_db(&db_path);
    }
}
