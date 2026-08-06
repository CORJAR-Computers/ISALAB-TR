use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::sample::{LabResult, Sample};
use crate::models::sample_list_item::SampleListItem;

pub(crate) type SampleListItemRow = (
    i32,
    String,
    i32,
    String,
    String,
    String,
    i32,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    i32,
    i32,
);

pub(crate) type LabResultRow = (
    i32,
    i32,
    i32,
    String,
    Option<String>,
    f64,
    String,
    Option<f64>,
    Option<f64>,
    Option<String>,
);

pub(crate) type TrendPointRow = (String, f64, Option<f64>, Option<f64>, String);

/// Columnas de una muestra con el tipo unido.
const SAMPLE_SELECT: &str = "
    SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
           LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
           s.STATUS, s.COLLECTED_BY, s.NOTES
    FROM SAMPLES s
    JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID";

pub(crate) type SampleRow = (
    i32,            // id
    String,         // code
    i32,            // patient_id
    i32,            // sample_type_id
    String,         // sample_type_name
    String,         // received_at
    String,         // status
    Option<String>, // collected_by
    Option<String>, // notes
);

pub(crate) fn map_sample(r: SampleRow) -> Sample {
    Sample {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        sample_type_id: r.3,
        sample_type_name: r.4,
        received_at: r.5,
        status: r.6,
        collected_by: r.7,
        notes: r.8,
        results: Vec::new(),
    }
}

/// Devuelve una muestra completa (con resultados) o `None`.
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Sample>, AppError> {
    let row: Option<SampleRow> = conn
        .query_first(&format!("{SAMPLE_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;

    let Some(row) = row else { return Ok(None) };

    let mut sample = map_sample(row);
    sample.results = list_results(conn, sample.id)?;
    Ok(Some(sample))
}

/// Listado global de muestras (mesa de trabajo del laboratorio) con filtros
/// opcionales por estado y búsqueda (código, paciente o propietario).
pub fn list(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<SampleListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT s.ID, s.CODE, s.PATIENT_ID, p.NAME, o.FULL_NAME, sp.NAME,
                s.SAMPLE_TYPE_ID, st.NAME,
                LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                s.STATUS, s.COLLECTED_BY, s.NOTES,
                (SELECT COUNT(*) FROM LAB_RESULTS lr WHERE lr.SAMPLE_ID = s.ID),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO'))
         FROM SAMPLES s
         JOIN PATIENTS p ON p.ID = s.PATIENT_ID
         JOIN OWNERS o ON o.ID = p.OWNER_ID
         JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
         JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
         WHERE (? IS NULL OR s.STATUS = ?)
           AND (? IS NULL
                OR UPPER(s.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?))
         ORDER BY s.RECEIVED_AT DESC, s.ID DESC";

    let rows: Vec<SampleListItemRow> = conn
        .query(sql, (&status, &status, &like, &like, &like, &like))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| SampleListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            owner_name: r.4,
            species_name: r.5,
            sample_type_id: r.6,
            sample_type_name: r.7,
            received_at: r.8,
            status: r.9,
            collected_by: r.10,
            notes: r.11,
            result_count: r.12,
            abnormal_count: r.13,
        })
        .collect())
}

/// Muestras por lista de ids (para etiquetas/lotes), en el mismo formato de la
/// mesa de trabajo y ordenadas por fecha de recepción descendente.
pub fn list_by_ids(
    conn: &mut SimpleConnection,
    ids: &[i32],
) -> Result<Vec<SampleListItem>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "
        SELECT s.ID, s.CODE, s.PATIENT_ID, p.NAME, o.FULL_NAME, sp.NAME,
                s.SAMPLE_TYPE_ID, st.NAME,
                LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                s.STATUS, s.COLLECTED_BY, s.NOTES,
                (SELECT COUNT(*) FROM LAB_RESULTS lr WHERE lr.SAMPLE_ID = s.ID),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO'))
         FROM SAMPLES s
         JOIN PATIENTS p ON p.ID = s.PATIENT_ID
         JOIN OWNERS o ON o.ID = p.OWNER_ID
         JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
         JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
         WHERE s.ID IN ({placeholders})
         ORDER BY s.RECEIVED_AT DESC, s.ID DESC"
    );
    let params: Vec<rsfbclient::SqlType> = ids
        .iter()
        .map(|&id| rsfbclient::SqlType::Integer(id as i64))
        .collect();
    let rows: Vec<SampleListItemRow> = conn.query(&sql, params).map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| SampleListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            owner_name: r.4,
            species_name: r.5,
            sample_type_id: r.6,
            sample_type_name: r.7,
            received_at: r.8,
            status: r.9,
            collected_by: r.10,
            notes: r.11,
            result_count: r.12,
            abnormal_count: r.13,
        })
        .collect())
}

/// Cambia el estado de una muestra validando la transición.
/// FINALIZADA requiere al menos un resultado cargado (cierre del analista).
pub fn set_status(conn: &mut SimpleConnection, id: i32, status: &str) -> Result<Sample, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SAMPLES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) =
        current.ok_or_else(|| AppError::NotFound(format!("Muestra {id} no encontrada")))?;

    let allowed = match status {
        "EN_PROCESO" => current == "RECIBIDA",
        "FINALIZADA" => {
            if !matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO") {
                false
            } else {
                let has_results: Option<(i32,)> = conn
                    .query_first("SELECT 1 FROM LAB_RESULTS WHERE SAMPLE_ID = ?", (&id,))
                    .map_err(AppError::from)?;
                has_results.is_some()
            }
        }
        "ANULADA" => matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO"),
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status} \
             (RECIBIDA→EN_PROCESO, →FINALIZADA con resultados, →ANULADA)"
        )));
    }

    conn.execute(
        "UPDATE SAMPLES SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP WHERE ID = ?",
        (&status, &id),
    )
    .map_err(AppError::from)?;

    get(conn, id)?
        .ok_or_else(|| AppError::Internal("Muestra actualizada pero no recuperada".into()))
}

/// Resultados de una muestra (para la ficha completa y el informe PDF).
pub fn list_results(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Vec<LabResult>, AppError> {
    let rows: Vec<LabResultRow> = conn
        .query(
            "SELECT r.ID, r.SAMPLE_ID, r.ANALYTE_ID, a.NAME, a.UNIT,
                    r.RESULT_VALUE, r.STATUS,
                    rr.MIN_VALUE, rr.MAX_VALUE,
                    LEFT(CAST(r.ANALYZED_AT AS VARCHAR(60)), 19)
             FROM LAB_RESULTS r
             JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
             LEFT JOIN REFERENCE_RANGES rr ON rr.ID = r.REFERENCE_RANGE_ID
             WHERE r.SAMPLE_ID = ?
             ORDER BY a.NAME",
            (&sample_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows.into_iter().map(map_lab_result).collect())
}

pub fn map_lab_result(r: LabResultRow) -> LabResult {
    LabResult {
        id: r.0,
        sample_id: r.1,
        analyte_id: r.2,
        analyte_name: r.3,
        unit: r.4,
        value: r.5,
        status: r.6,
        ref_min: r.7,
        ref_max: r.8,
        analyzed_at: r.9,
    }
}

/// Resultados de laboratorio con datos de muestra/paciente unidos, para
/// exportación CSV (filtro opcional por estado y búsqueda global).
pub fn list_results_for_export(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<crate::csv::ResultExportRow>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT s.CODE, p.NAME, o.FULL_NAME, sp.NAME, st.NAME,
               LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
               a.NAME, a.UNIT, r.RESULT_VALUE, r.STATUS,
               rr.MIN_VALUE, rr.MAX_VALUE,
               LEFT(CAST(r.ANALYZED_AT AS VARCHAR(60)), 19)
        FROM LAB_RESULTS r
        JOIN SAMPLES s ON s.ID = r.SAMPLE_ID
        JOIN PATIENTS p ON p.ID = s.PATIENT_ID
        JOIN OWNERS o ON o.ID = p.OWNER_ID
        JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
        JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
        JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
        LEFT JOIN REFERENCE_RANGES rr ON rr.ID = r.REFERENCE_RANGE_ID
        WHERE (? IS NULL OR s.STATUS = ?)
          AND (? IS NULL
               OR UPPER(s.CODE) LIKE UPPER(?)
               OR UPPER(p.NAME) LIKE UPPER(?)
               OR UPPER(o.FULL_NAME) LIKE UPPER(?))
        ORDER BY s.RECEIVED_AT DESC, s.ID DESC, a.NAME";

    type Row = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        f64,
        String,
        Option<f64>,
        Option<f64>,
        Option<String>,
    );

    let rows: Vec<Row> = conn
        .query(sql, (&status, &status, &like, &like, &like, &like))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| crate::csv::ResultExportRow {
            code: r.0,
            patient_name: r.1,
            owner_name: r.2,
            species_name: r.3,
            sample_type_name: r.4,
            received_at: r.5,
            analyte_name: r.6,
            unit: r.7,
            value: r.8,
            status: r.9,
            ref_min: r.10,
            ref_max: r.11,
            analyzed_at: r.12,
        })
        .collect())
}

pub fn get_patient_lab_trends(
    conn: &mut SimpleConnection,
    patient_id: i32,
    analyte_id: i32,
) -> Result<Vec<crate::models::sample::TrendPoint>, AppError> {
    let sql = "
        SELECT LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 10),
               r.RESULT_VALUE, rr.MIN_VALUE, rr.MAX_VALUE, r.STATUS
        FROM LAB_RESULTS r
        JOIN SAMPLES s ON s.ID = r.SAMPLE_ID
        LEFT JOIN REFERENCE_RANGES rr ON rr.ID = r.REFERENCE_RANGE_ID
        WHERE s.PATIENT_ID = ? AND r.ANALYTE_ID = ? AND s.STATUS IN ('EN_PROCESO', 'FINALIZADA')
        ORDER BY s.RECEIVED_AT ASC
    ";
    let rows: Vec<TrendPointRow> = conn
        .query(sql, (&patient_id, &analyte_id))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| crate::models::sample::TrendPoint {
            date: r.0,
            value: r.1,
            ref_min: r.2,
            ref_max: r.3,
            status: r.4,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_sample_fields() {
        let row: SampleRow = (
            1,                                 // id
            "M-2026-0001".into(),              // code
            10,                                // patient_id
            1,                                 // sample_type_id
            "Sangre total (EDTA)".into(),      // sample_type_name
            "2026-08-01 10:30:00".into(),      // received_at
            "RECIBIDA".into(),                 // status
            Some("Dr. Ramos".into()),          // collected_by
            Some("Muestra de control".into()), // notes
        );
        let sample = map_sample(row);

        assert_eq!(sample.id, 1);
        assert_eq!(sample.code, "M-2026-0001");
        assert_eq!(sample.patient_id, 10);
        assert_eq!(sample.sample_type_id, 1);
        assert_eq!(sample.sample_type_name, "Sangre total (EDTA)");
        assert_eq!(sample.received_at, "2026-08-01 10:30:00");
        assert_eq!(sample.status, "RECIBIDA");
        assert_eq!(sample.collected_by.as_deref(), Some("Dr. Ramos"));
        assert_eq!(sample.notes.as_deref(), Some("Muestra de control"));
        assert!(sample.results.is_empty()); // map_sample siempre devuelve results vacío
    }

    #[test]
    fn test_map_sample_optional_fields_none() {
        let row: SampleRow = (
            2,
            "M-2026-0002".into(),
            20,
            2,
            "Suero".into(),
            "2026-08-02 14:00:00".into(),
            "EN_PROCESO".into(),
            None,
            None,
        );
        let sample = map_sample(row);

        assert_eq!(sample.id, 2);
        assert_eq!(sample.collected_by, None);
        assert_eq!(sample.notes, None);
    }

    #[test]
    fn test_map_lab_result_with_reference_range() {
        let row: LabResultRow = (
            100,                                // id
            1,                                  // sample_id
            1,                                  // analyte_id
            "Hematocrito".into(),               // analyte_name
            Some("%".into()),                   // unit
            42.5,                               // value
            "NORMAL".into(),                    // status
            Some(37.0),                         // ref_min
            Some(55.0),                         // ref_max
            Some("2026-08-01 11:00:00".into()), // analyzed_at
        );
        let result = map_lab_result(row);

        assert_eq!(result.id, 100);
        assert_eq!(result.sample_id, 1);
        assert_eq!(result.analyte_id, 1);
        assert_eq!(result.analyte_name, "Hematocrito");
        assert_eq!(result.unit.as_deref(), Some("%"));
        assert!((result.value - 42.5).abs() < f64::EPSILON);
        assert_eq!(result.status, "NORMAL");
        assert_eq!(result.ref_min, Some(37.0));
        assert_eq!(result.ref_max, Some(55.0));
        assert_eq!(result.analyzed_at.as_deref(), Some("2026-08-01 11:00:00"));
    }

    #[test]
    fn test_map_lab_result_without_reference_range() {
        let row: LabResultRow = (
            101,
            1,
            10,
            "Glucosa".into(),
            Some("mg/dL".into()),
            95.0,
            "NORMAL".into(),
            None,
            None,
            None,
        );
        let result = map_lab_result(row);

        assert_eq!(result.ref_min, None);
        assert_eq!(result.ref_max, None);
        assert_eq!(result.analyzed_at, None);
    }

    #[test]
    fn test_map_lab_result_abnormal_status() {
        let row: LabResultRow = (
            102,
            1,
            6,
            "ALT".into(),
            Some("U/L".into()),
            150.0,
            "ALTO".into(),
            Some(10.0),
            Some(100.0),
            None,
        );
        let result = map_lab_result(row);
        assert_eq!(result.status, "ALTO");
        assert!((result.value - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sample_list_item_row_mapping() {
        let row: SampleListItemRow = (
            1,
            "M-2026-0001".into(),
            10,
            "Luna".into(),
            "Juan Pérez".into(),
            "Canino".into(),
            1,
            "Sangre".into(),
            "2026-08-01 10:30:00".into(),
            "RECIBIDA".into(),
            Some("Dr. Ramos".into()),
            None,
            3,
            1,
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "M-2026-0001");
        assert_eq!(row.12, 3); // result_count
        assert_eq!(row.13, 1); // abnormal_count
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        test_helpers::insert_test_sample_type(&mut conn);
        (conn, db_path)
    }

    #[test]
    fn test_get_sample_existing() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        // Insertar muestra
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        let sample = get(&mut conn, 1).unwrap();
        assert!(sample.is_some());
        let s = sample.unwrap();
        assert_eq!(s.id, 1);
        assert_eq!(s.code, "M-2026-0001");
        assert_eq!(s.status, "RECIBIDA");
        assert_eq!(s.sample_type_name, "Sangre total (EDTA)");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_sample_not_found() {
        let (mut conn, db_path) = setup();
        let _ = test_helpers::insert_test_patient(&mut conn);

        let sample = get(&mut conn, 999).unwrap();
        assert!(sample.is_none());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_samples_no_filter() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        let samples = list(&mut conn, None, None).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].code, "M-2026-0001");
        assert_eq!(samples[0].patient_name, "Luna");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_samples_with_status_filter() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (2, 'M-2026-0002', ?, 1, '2026-08-02 10:00:00', 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();

        let samples = list(&mut conn, Some("RECIBIDA"), None).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].code, "M-2026-0001");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_sample_status_transition() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        // RECIBIDA -> EN_PROCESO
        let updated = set_status(&mut conn, 1, "EN_PROCESO").unwrap();
        assert_eq!(updated.status, "EN_PROCESO");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_sample_status_invalid_transition() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        // RECIBIDA -> FINALIZADA sin resultados (debe fallar)
        let result = set_status(&mut conn, 1, "FINALIZADA");
        assert!(result.is_err());

        test_helpers::cleanup_test_db(&db_path);
    }
}
