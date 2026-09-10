use std::collections::HashMap;

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::sample::ResultAttachment;
use crate::repositories::next_id;

pub(crate) type ResultAttachmentRow = (
    i32,            // id
    i32,            // result_id
    String,         // file_name
    String,         // file_path
    Option<String>, // mime_type
    String,         // created_at
);

pub(crate) fn map_attachment(r: ResultAttachmentRow) -> ResultAttachment {
    ResultAttachment {
        id: r.0,
        result_id: r.1,
        file_name: r.2,
        file_path: r.3,
        mime_type: r.4,
        created_at: r.5,
    }
}

const SELECT: &str = "SELECT ID, RESULT_ID, FILE_NAME, FILE_PATH, MIME_TYPE, \
                      CAST(CREATED_AT AS VARCHAR(30)) FROM RESULT_ATTACHMENTS";

/// Adjuntos de un resultado (ordenados por fecha de carga).
pub fn list_for_result(
    conn: &mut SimpleConnection,
    result_id: i32,
) -> Result<Vec<ResultAttachment>, AppError> {
    let rows: Vec<ResultAttachmentRow> = conn
        .query(
            &format!("{SELECT} WHERE RESULT_ID = ? ORDER BY CREATED_AT, ID"),
            (&result_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_attachment).collect())
}

/// Adjuntos de TODOS los resultados de una muestra, agrupados por result_id
/// en una sola query (evita el N+1 de `list_for_result` por resultado).
/// Los resultados sin adjuntos no aparecen en el mapa: el llamante usa
/// `remove(&result_id).unwrap_or_default()`.
pub(crate) fn list_for_results_of_sample(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<HashMap<i32, Vec<ResultAttachment>>, AppError> {
    let rows: Vec<ResultAttachmentRow> = conn
        .query(
            &format!(
                "{SELECT} ra WHERE ra.RESULT_ID IN
                  (SELECT r.ID FROM LAB_RESULTS r WHERE r.SAMPLE_ID = ?)
                 ORDER BY ra.CREATED_AT, ra.ID"
            ),
            (&sample_id,),
        )
        .map_err(AppError::from)?;

    let mut grouped: HashMap<i32, Vec<ResultAttachment>> = HashMap::new();
    for row in rows {
        grouped.entry(row.1).or_default().push(map_attachment(row));
    }
    Ok(grouped)
}

pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<ResultAttachment>, AppError> {
    let row: Option<ResultAttachmentRow> = conn
        .query_first(&format!("{SELECT} WHERE ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_attachment))
}

pub fn insert(
    conn: &mut SimpleConnection,
    result_id: i32,
    file_name: &str,
    file_path: &str,
    mime_type: Option<String>,
) -> Result<ResultAttachment, AppError> {
    let id = next_id(conn, "GEN_RESULT_ATTACHMENTS_SEQ")?;
    conn.execute(
        "INSERT INTO RESULT_ATTACHMENTS (ID, RESULT_ID, FILE_NAME, FILE_PATH, MIME_TYPE)
         VALUES (?, ?, ?, ?, ?)",
        (&id, &result_id, file_name, file_path, &mime_type),
    )
    .map_err(AppError::from)?;

    get(conn, id)?.ok_or_else(|| AppError::Internal("Adjunto no encontrado tras insertar".into()))
}

pub fn delete(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM RESULT_ATTACHMENTS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_attachment_fields() {
        let row: ResultAttachmentRow = (
            1,
            5,
            "placa_hemograma.png".into(),
            "C:\\app_data\\attachments\\abc.png".into(),
            Some("image/png".into()),
            "2026-08-06 10:00:00".into(),
        );
        let att = map_attachment(row);
        assert_eq!(att.id, 1);
        assert_eq!(att.result_id, 5);
        assert_eq!(att.file_name, "placa_hemograma.png");
        assert_eq!(att.mime_type.as_deref(), Some("image/png"));
        assert_eq!(att.created_at, "2026-08-06 10:00:00");
    }

    #[test]
    fn test_map_attachment_without_mime() {
        let row: ResultAttachmentRow = (
            2,
            5,
            "foto.jpg".into(),
            "/tmp/att/x.jpg".into(),
            None,
            "2026-08-06 11:00:00".into(),
        );
        let att = map_attachment(row);
        assert_eq!(att.mime_type, None);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::models::sample::RegisterResultInput;
    use crate::repositories::clinical_history as history_repo;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf, i32) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);
        test_helpers::insert_test_reference_range(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        let result = history_repo::register_lab_result(
            &mut conn,
            &RegisterResultInput {
                sample_id: 1,
                analyte_id: 1,
                value: 45.0,
                custom_ref_min: None,
                custom_ref_max: None,
            },
        )
        .unwrap();
        (conn, db_path, result.id)
    }

    #[test]
    fn test_insert_and_list_attachments() {
        let (mut conn, db_path, result_id) = setup();
        assert!(list_for_result(&mut conn, result_id).unwrap().is_empty());

        insert(
            &mut conn,
            result_id,
            "placa.png",
            "/data/att/x.png",
            Some("image/png".into()),
        )
        .unwrap();
        insert(&mut conn, result_id, "frotis.jpg", "/data/att/y.jpg", None).unwrap();

        let list = list_for_result(&mut conn, result_id).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].file_name, "placa.png");
        assert_eq!(list[1].mime_type, None);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_and_delete_attachment() {
        let (mut conn, db_path, result_id) = setup();
        let att = insert(
            &mut conn,
            result_id,
            "electro.png",
            "/data/att/e.png",
            Some("image/png".into()),
        )
        .unwrap();

        let fetched = get(&mut conn, att.id).unwrap().unwrap();
        assert_eq!(fetched.file_name, "electro.png");
        assert_eq!(fetched.result_id, result_id);

        delete(&mut conn, att.id).unwrap();
        assert!(get(&mut conn, att.id).unwrap().is_none());
        assert!(list_for_result(&mut conn, result_id).unwrap().is_empty());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_delete_cascades_with_result() {
        let (mut conn, db_path, result_id) = setup();
        insert(&mut conn, result_id, "placa.png", "/data/att/x.png", None).unwrap();

        conn.execute("DELETE FROM LAB_RESULTS WHERE ID = ?", (&result_id,))
            .unwrap();
        assert!(list_for_result(&mut conn, result_id).unwrap().is_empty());

        test_helpers::cleanup_test_db(&db_path);
    }

    /// Carga agrupada de adjuntos de una muestra completa en UNA query
    /// (reemplaza el N+1 de list_for_result por resultado en list_results).
    #[test]
    fn test_list_for_results_of_sample_groups_by_result() {
        let (mut conn, db_path, result_a) = setup();

        // Segundo resultado de la misma muestra (analito 2 = seeded).
        let result_b = history_repo::register_lab_result(
            &mut conn,
            &RegisterResultInput {
                sample_id: 1,
                analyte_id: 2,
                value: 95.0,
                custom_ref_min: None,
                custom_ref_max: None,
            },
        )
        .unwrap()
        .id;

        insert(
            &mut conn,
            result_a,
            "placa_a1.png",
            "/data/att/a1.png",
            Some("image/png".into()),
        )
        .unwrap();
        insert(&mut conn, result_a, "placa_a2.png", "/data/att/a2.png", None).unwrap();
        insert(
            &mut conn,
            result_b,
            "frotis_b.png",
            "/data/att/b.png",
            Some("image/jpeg".into()),
        )
        .unwrap();

        let grouped = list_for_results_of_sample(&mut conn, 1).unwrap();

        // Dos resultados con adjuntos, agrupados correctamente.
        assert_eq!(grouped.len(), 2);
        let atts_a = grouped.get(&result_a).unwrap();
        assert_eq!(atts_a.len(), 2);
        assert_eq!(atts_a[0].file_name, "placa_a1.png");
        assert_eq!(atts_a[1].file_name, "placa_a2.png");
        assert_eq!(grouped.get(&result_b).unwrap()[0].file_name, "frotis_b.png");

        // Todos pertenecen a resultados de la muestra 1.
        assert!(grouped
            .values()
            .flatten()
            .all(|att| att.result_id == result_a || att.result_id == result_b));

        test_helpers::cleanup_test_db(&db_path);
    }

    /// Una muestra sin adjuntos devuelve mapa vacío (sin filas leídas).
    #[test]
    fn test_list_for_results_of_sample_empty() {
        let (mut conn, db_path, _result_id) = setup();
        let grouped = list_for_results_of_sample(&mut conn, 1).unwrap();
        assert!(grouped.is_empty());
        test_helpers::cleanup_test_db(&db_path);
    }
}
