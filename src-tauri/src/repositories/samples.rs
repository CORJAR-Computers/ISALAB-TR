use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::sample::{LabResult, Sample, SampleEvent};
use crate::models::sample_list_item::SampleListItem;

pub(crate) type SampleListItemRow = (
    i32,
    String,
    i32,
    String,
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
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
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

pub(crate) type SampleEventRow = (i32, i32, String, String, Option<String>, String);

/// Columnas de una muestra con el tipo unido, el equipo analizador y la
/// calidad preanalítica / rechazo.
const SAMPLE_SELECT: &str = "
    SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
           LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
           s.STATUS, s.COLLECTED_BY, s.NOTES,
           s.ANALYZER_ID, az.NAME,
           s.QUALITY_INDEX, s.QUALITY_SEVERITY, s.QUALITY_NOTE,
           LEFT(CAST(s.REJECTED_AT AS VARCHAR(60)), 19),
           s.REJECTED_BY, s.REJECTION_REASON
    FROM SAMPLES s
    JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
    LEFT JOIN ANALYZERS az ON az.ID = s.ANALYZER_ID";

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
    Option<i32>,    // analyzer_id
    Option<String>, // analyzer_name
    Option<String>, // quality_index
    Option<String>, // quality_severity
    Option<String>, // quality_note
    Option<String>, // rejected_at
    Option<String>, // rejected_by
    Option<String>, // rejection_reason
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
        analyzer_id: r.9,
        analyzer_name: r.10,
        results: Vec::new(),
        quality_index: r.11,
        quality_severity: r.12,
        quality_note: r.13,
        rejected_at: r.14,
        rejected_by: r.15,
        rejection_reason: r.16,
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
        SELECT s.ID, s.CODE, s.PATIENT_ID, p.CODE, p.NAME, o.FULL_NAME, sp.NAME,
                s.SAMPLE_TYPE_ID, st.NAME,
                LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                s.STATUS, s.COLLECTED_BY, s.NOTES,
                (SELECT COUNT(*) FROM LAB_RESULTS lr WHERE lr.SAMPLE_ID = s.ID),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO')),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('CRITICO_ALTO', 'CRITICO_BAJO')),
                s.QUALITY_INDEX, s.QUALITY_SEVERITY, s.REJECTION_REASON
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
            patient_code: r.3,
            patient_name: r.4,
            owner_name: r.5,
            species_name: r.6,
            sample_type_id: r.7,
            sample_type_name: r.8,
            received_at: r.9,
            status: r.10,
            collected_by: r.11,
            notes: r.12,
            result_count: r.13,
            abnormal_count: r.14,
            critical_count: r.15,
            quality_index: r.16,
            quality_severity: r.17,
            rejection_reason: r.18,
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
        SELECT s.ID, s.CODE, s.PATIENT_ID, p.CODE, p.NAME, o.FULL_NAME, sp.NAME,
                s.SAMPLE_TYPE_ID, st.NAME,
                LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                s.STATUS, s.COLLECTED_BY, s.NOTES,
                (SELECT COUNT(*) FROM LAB_RESULTS lr WHERE lr.SAMPLE_ID = s.ID),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO')),
                (SELECT COUNT(*) FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('CRITICO_ALTO', 'CRITICO_BAJO')),
                s.QUALITY_INDEX, s.QUALITY_SEVERITY, s.REJECTION_REASON
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
            patient_code: r.3,
            patient_name: r.4,
            owner_name: r.5,
            species_name: r.6,
            sample_type_id: r.7,
            sample_type_name: r.8,
            received_at: r.9,
            status: r.10,
            collected_by: r.11,
            notes: r.12,
            result_count: r.13,
            abnormal_count: r.14,
            critical_count: r.15,
            quality_index: r.16,
            quality_severity: r.17,
            rejection_reason: r.18,
        })
        .collect())
}

/// Cambia el estado de una muestra validando la transición.
/// FINALIZADA requiere al menos un resultado cargado (cierre del analista).
/// RECHAZADA solo puede venir de RECIBIDA/EN_PROCESO; la reapertura vuelve a
/// RECIBIDA (los campos de rechazo se limpian en `reopen_sample`).
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
        "RECHAZADA" => matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO"),
        "RECIBIDA" => current == "RECHAZADA",
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status} \
             (RECIBIDA→EN_PROCESO, →FINALIZADA con resultados, →ANULADA, →RECHAZADA; \
              RECHAZADA→RECIBIDA)"
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

/// Registra la calidad preanalítica de una muestra (interferencia HIL,
/// severidad y nota). Solo en estados abiertos (no FINALIZADA/ANULADA).
pub fn set_quality(
    conn: &mut SimpleConnection,
    id: i32,
    quality_index: Option<&str>,
    quality_severity: Option<&str>,
    quality_note: Option<&str>,
) -> Result<Sample, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SAMPLES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) =
        current.ok_or_else(|| AppError::NotFound(format!("Muestra {id} no encontrada")))?;
    if matches!(current.as_str(), "FINALIZADA" | "ANULADA") {
        return Err(AppError::Validation(format!(
            "No se puede modificar la calidad de una muestra {current}"
        )));
    }

    conn.execute(
        "UPDATE SAMPLES
            SET QUALITY_INDEX = ?, QUALITY_SEVERITY = ?, QUALITY_NOTE = ?,
                UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&quality_index, &quality_severity, &quality_note, &id),
    )
    .map_err(AppError::from)?;

    get(conn, id)?
        .ok_or_else(|| AppError::Internal("Muestra actualizada pero no recuperada".into()))
}

/// Rechaza una muestra (RECIBIDA/EN_PROCESO → RECHAZADA) con motivo obligatorio.
pub fn reject_sample(
    conn: &mut SimpleConnection,
    id: i32,
    reason: &str,
    rejected_by: &str,
) -> Result<Sample, AppError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::Validation(
            "Debes indicar el motivo del rechazo".into(),
        ));
    }

    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SAMPLES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) =
        current.ok_or_else(|| AppError::NotFound(format!("Muestra {id} no encontrada")))?;
    if !matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO") {
        return Err(AppError::Validation(format!(
            "No se puede rechazar una muestra {current}"
        )));
    }

    conn.execute(
        "UPDATE SAMPLES
            SET STATUS = 'RECHAZADA', REJECTED_AT = CURRENT_TIMESTAMP,
                REJECTED_BY = ?, REJECTION_REASON = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&rejected_by, &reason, &id),
    )
    .map_err(AppError::from)?;

    log_sample_event(conn, id, "REJECTED", rejected_by, Some(reason))?;

    get(conn, id)?
        .ok_or_else(|| AppError::Internal("Muestra actualizada pero no recuperada".into()))
}

/// Reabre una muestra rechazada (RECHAZADA → RECIBIDA), limpiando los
/// campos de rechazo para un nuevo ciclo de recepción. Registra la
/// reapertura en el historial de eventos de la muestra.
pub fn reopen_sample(
    conn: &mut SimpleConnection,
    id: i32,
    reopened_by: &str,
) -> Result<Sample, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SAMPLES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) =
        current.ok_or_else(|| AppError::NotFound(format!("Muestra {id} no encontrada")))?;
    if current != "RECHAZADA" {
        return Err(AppError::Validation(format!(
            "Solo se puede reabrir una muestra RECHAZADA (estado actual: {current})"
        )));
    }

    conn.execute(
        "UPDATE SAMPLES
            SET STATUS = 'RECIBIDA', REJECTED_AT = NULL, REJECTED_BY = NULL,
                REJECTION_REASON = NULL, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&id,),
    )
    .map_err(AppError::from)?;

    log_sample_event(conn, id, "REOPENED", reopened_by, None)?;

    get(conn, id)?
        .ok_or_else(|| AppError::Internal("Muestra actualizada pero no recuperada".into()))
}

/// Registra un evento en el historial de la muestra (REJECTED | REOPENED).
pub fn log_sample_event(
    conn: &mut SimpleConnection,
    sample_id: i32,
    event_type: &str,
    username: &str,
    reason: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO SAMPLE_EVENTS (SAMPLE_ID, EVENT_TYPE, USERNAME, REASON)
         VALUES (?, ?, ?, ?)",
        (&sample_id, &event_type, &username, &reason),
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// Historial completo de rechazos y reaperturas de una muestra, del más
/// reciente al más antiguo (quién, cuándo y motivo de cada evento).
pub fn list_sample_events(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Vec<SampleEvent>, AppError> {
    let rows: Vec<SampleEventRow> = conn
        .query(
            "SELECT ID, SAMPLE_ID, EVENT_TYPE, USERNAME, REASON,
                    LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
             FROM SAMPLE_EVENTS
             WHERE SAMPLE_ID = ?
             ORDER BY CREATED_AT DESC, ID DESC",
            (&sample_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| SampleEvent {
            id: r.0,
            sample_id: r.1,
            event_type: r.2,
            username: r.3,
            reason: r.4,
            created_at: r.5,
        })
        .collect())
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

    let mut results: Vec<LabResult> = rows.into_iter().map(map_lab_result).collect();

    // Evidencias adjuntas (placas, frotis, electroforesis) por resultado.
    for r in &mut results {
        r.attachments = crate::repositories::attachments::list_for_result(conn, r.id)?;
    }

    // Delta check: variación contra el resultado previo del paciente.
    for r in &mut results {
        r.delta_variation = delta_variation(conn, sample_id, r.analyte_id, r.value)?;
    }

    Ok(results)
}

pub fn map_lab_result(r: LabResultRow) -> LabResult {
    let status = r.6;
    let is_critical = matches!(status.as_str(), "CRITICO_ALTO" | "CRITICO_BAJO");
    LabResult {
        id: r.0,
        sample_id: r.1,
        analyte_id: r.2,
        analyte_name: r.3,
        unit: r.4,
        value: r.5,
        status,
        ref_min: r.7,
        ref_max: r.8,
        analyzed_at: r.9,
        delta_variation: None,
        is_critical,
        attachments: Vec::new(),
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

pub(crate) type WorklistRow = (
    i32,    // id
    String, // code
    i32,    // patient_id
    String, // patient_name
    String, // owner_name
    String, // species_name
    i32,    // sample_type_id
    String, // sample_type_name
    String, // status
    String, // received_at
    i32,    // elapsed_minutes (i32: Specta prohíbe i64 en bindings TS)
    i32,    // result_count
    i32,    // abnormal_count
);

/// Bandeja de trabajo del laboratorio: muestras pendientes (RECIBIDA /
/// EN_PROCESO) agrupadas por tipo de muestra, separando las recibidas hoy
/// de las de días anteriores, con el tiempo transcurrido desde la recepción.
pub fn get_worklist(
    conn: &mut SimpleConnection,
) -> Result<crate::models::worklist::WorklistData, AppError> {
    use crate::models::worklist::WorklistSample;

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let sql = "
        SELECT s.ID, s.CODE, s.PATIENT_ID, p.NAME, o.FULL_NAME, sp.NAME,
               s.SAMPLE_TYPE_ID, st.NAME, s.STATUS,
               LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
               CAST(DATEDIFF(MINUTE FROM s.RECEIVED_AT TO CURRENT_TIMESTAMP) AS INTEGER),
               (SELECT COUNT(*) FROM LAB_RESULTS lr WHERE lr.SAMPLE_ID = s.ID),
               (SELECT COUNT(*) FROM LAB_RESULTS lr
                 WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO'))
        FROM SAMPLES s
        JOIN PATIENTS p ON p.ID = s.PATIENT_ID
        JOIN OWNERS o ON o.ID = p.OWNER_ID
        JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
        JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
        WHERE s.STATUS IN ('RECIBIDA', 'EN_PROCESO')
        ORDER BY s.RECEIVED_AT ASC, s.ID ASC";

    let rows: Vec<WorklistRow> = conn.query(sql, ()).map_err(AppError::from)?;
    let samples = rows
        .into_iter()
        .map(|r| WorklistSample {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            owner_name: r.4,
            species_name: r.5,
            sample_type_id: r.6,
            sample_type_name: r.7,
            status: r.8,
            received_at: r.9,
            elapsed_minutes: r.10,
            result_count: r.11,
            abnormal_count: r.12,
        })
        .collect();

    Ok(group_worklist(samples, &today))
}

/// Agrupa las muestras pendientes por tipo y las separa entre recibidas hoy
/// y de días anteriores. Función pura (testeable sin base de datos); los
/// grupos y las muestras llegan ordenados por antigüedad ascendente.
pub fn group_worklist(
    samples: Vec<crate::models::worklist::WorklistSample>,
    today: &str,
) -> crate::models::worklist::WorklistData {
    use crate::models::worklist::{WorklistData, WorklistGroup};

    let mut today_groups: Vec<WorklistGroup> = Vec::new();
    let mut overdue_groups: Vec<WorklistGroup> = Vec::new();

    for s in samples {
        // RECEIVED_AT llega como "YYYY-MM-DD HH:MM:SS"; la fecha es el primer token.
        let received_date = s.received_at.split(' ').next().unwrap_or(&s.received_at);
        let is_today = received_date == today;
        let target = if is_today {
            &mut today_groups
        } else {
            &mut overdue_groups
        };

        match target
            .iter_mut()
            .find(|g| g.sample_type_id == s.sample_type_id)
        {
            Some(g) => {
                g.count += 1;
                g.max_elapsed_minutes = g.max_elapsed_minutes.max(s.elapsed_minutes);
                g.samples.push(s);
            }
            None => {
                let sample_type_name = s.sample_type_name.clone();
                target.push(WorklistGroup {
                    sample_type_id: s.sample_type_id,
                    sample_type_name,
                    count: 1,
                    max_elapsed_minutes: s.elapsed_minutes,
                    samples: vec![s],
                });
            }
        }
    }

    // Los grupos con la muestra más antigua primero (mayor urgencia).
    today_groups.sort_by_key(|g| std::cmp::Reverse(g.max_elapsed_minutes));
    overdue_groups.sort_by_key(|g| std::cmp::Reverse(g.max_elapsed_minutes));

    let total_pending = today_groups.iter().map(|g| g.count).sum::<i32>()
        + overdue_groups.iter().map(|g| g.count).sum::<i32>();

    WorklistData {
        date: today.to_string(),
        total_pending,
        today: today_groups,
        overdue: overdue_groups,
    }
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

/// Valor previo del mismo analito para el paciente (delta check). Excluye la
/// muestra actual. Devuelve la variación porcentual respecto al anterior.
pub fn delta_variation(
    conn: &mut SimpleConnection,
    sample_id: i32,
    analyte_id: i32,
    new_value: f64,
) -> Result<Option<f64>, AppError> {
    let prev: Option<(f64,)> = conn
        .query_first(
            "SELECT FIRST 1 r.RESULT_VALUE
             FROM LAB_RESULTS r
             JOIN SAMPLES s ON s.ID = r.SAMPLE_ID
             WHERE s.PATIENT_ID = (SELECT PATIENT_ID FROM SAMPLES WHERE ID = ?)
               AND r.ANALYTE_ID = ?
               AND r.SAMPLE_ID <> ?
               AND s.STATUS IN ('EN_PROCESO', 'FINALIZADA')
             ORDER BY s.RECEIVED_AT DESC, r.UPDATED_AT DESC",
            (&sample_id, &analyte_id, &sample_id),
        )
        .map_err(AppError::from)?;

    Ok(match prev {
        Some((prev_value,)) if prev_value > 0.0 => {
            Some(((new_value - prev_value) / prev_value) * 100.0)
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_sample_fields() {
        let row: SampleRow = (
            1,                                  // id
            "M-2026-0001".into(),               // code
            10,                                 // patient_id
            1,                                  // sample_type_id
            "Sangre total (EDTA)".into(),       // sample_type_name
            "2026-08-01 10:30:00".into(),       // received_at
            "RECIBIDA".into(),                  // status
            Some("Dr. Ramos".into()),           // collected_by
            Some("Muestra de control".into()),  // notes
            Some(2),                            // analyzer_id
            Some("MINDRAY B2800".into()),       // analyzer_name
            Some("LIPEMIA".into()),             // quality_index
            Some("MODERADA".into()),            // quality_severity
            Some("Muestra lipémica".into()),    // quality_note
            Some("2026-08-01 10:35:00".into()), // rejected_at
            Some("Dr. Ramos".into()),           // rejected_by
            None,                               // rejection_reason
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
        assert_eq!(sample.analyzer_id, Some(2));
        assert_eq!(sample.analyzer_name.as_deref(), Some("MINDRAY B2800"));
        assert_eq!(sample.quality_index.as_deref(), Some("LIPEMIA"));
        assert_eq!(sample.quality_severity.as_deref(), Some("MODERADA"));
        assert_eq!(sample.rejected_by.as_deref(), Some("Dr. Ramos"));
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
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let sample = map_sample(row);

        assert_eq!(sample.id, 2);
        assert_eq!(sample.collected_by, None);
        assert_eq!(sample.notes, None);
        assert_eq!(sample.analyzer_id, None);
        assert_eq!(sample.analyzer_name, None);
        assert_eq!(sample.quality_index, None);
        assert_eq!(sample.rejection_reason, None);
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
        assert!(!result.is_critical);
        assert_eq!(result.delta_variation, None);
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
        assert!(!result.is_critical);
    }

    #[test]
    fn test_map_lab_result_critical_status() {
        let row: LabResultRow = (
            103,
            1,
            6,
            "Glucosa".into(),
            Some("mg/dL".into()),
            25.0,
            "CRITICO_BAJO".into(),
            Some(70.0),
            Some(110.0),
            None,
        );
        let result = map_lab_result(row);
        assert_eq!(result.status, "CRITICO_BAJO");
        assert!(result.is_critical);
    }

    #[test]
    fn test_group_worklist_splits_today_and_overdue() {
        use crate::models::worklist::WorklistSample;
        let mk = |id: i32, type_id: i32, type_name: &str, date: &str, mins: i32| WorklistSample {
            id,
            code: format!("M-2026-{id:04}"),
            patient_id: 1,
            patient_name: "Luna".into(),
            owner_name: "Juan Pérez".into(),
            species_name: "Canino".into(),
            sample_type_id: type_id,
            sample_type_name: type_name.to_string(),
            status: "RECIBIDA".into(),
            received_at: format!("{date} 10:00:00"),
            elapsed_minutes: mins,
            result_count: 0,
            abnormal_count: 0,
        };
        let rows = vec![
            mk(1, 1, "Sangre total (EDTA)", "2026-08-06", 45),
            mk(2, 2, "Suero", "2026-08-06", 120),
            mk(3, 1, "Sangre total (EDTA)", "2026-08-05", 1500),
            mk(4, 2, "Suero", "2026-08-05", 2000),
        ];
        let wl = group_worklist(rows, "2026-08-06");

        assert_eq!(wl.date, "2026-08-06");
        assert_eq!(wl.total_pending, 4);
        assert_eq!(wl.today.len(), 2);
        assert_eq!(wl.overdue.len(), 2);
        // Grupos de hoy ordenados por urgencia desc (el más antiguo primero).
        assert_eq!(wl.today[0].sample_type_name, "Suero"); // 120 min
        assert_eq!(wl.today[0].max_elapsed_minutes, 120);
        assert_eq!(wl.today[0].samples[0].id, 2);
        assert_eq!(wl.today[1].sample_type_name, "Sangre total (EDTA)"); // 45 min
        assert_eq!(wl.today[1].max_elapsed_minutes, 45);
        // Pendientes de días anteriores, también por urgencia.
        assert_eq!(wl.overdue[0].sample_type_name, "Suero"); // 2000 min
        assert_eq!(wl.overdue[0].max_elapsed_minutes, 2000);
        assert_eq!(wl.overdue[1].sample_type_name, "Sangre total (EDTA)"); // 1500 min
    }

    #[test]
    fn test_group_worklist_groups_by_type_with_counts() {
        use crate::models::worklist::WorklistSample;
        let mk = |id: i32, type_id: i32, type_name: &str, mins: i32| WorklistSample {
            id,
            code: format!("M-2026-{id:04}"),
            patient_id: 1,
            patient_name: "Luna".into(),
            owner_name: "Juan Pérez".into(),
            species_name: "Canino".into(),
            sample_type_id: type_id,
            sample_type_name: type_name.to_string(),
            status: "EN_PROCESO".into(),
            received_at: format!("2026-08-06 09:{id:02}:00"),
            elapsed_minutes: mins,
            result_count: 0,
            abnormal_count: 0,
        };
        let rows = vec![
            mk(1, 1, "Sangre", 30),
            mk(2, 1, "Sangre", 60),
            mk(3, 2, "Orina", 90),
        ];
        let wl = group_worklist(rows, "2026-08-06");

        assert_eq!(wl.today.len(), 2);
        let sangre = wl
            .today
            .iter()
            .find(|g| g.sample_type_name == "Sangre")
            .unwrap();
        assert_eq!(sangre.count, 2);
        assert_eq!(sangre.max_elapsed_minutes, 60);
        assert_eq!(sangre.samples.len(), 2);
        assert!(wl.overdue.is_empty());
    }

    #[test]
    fn test_group_worklist_empty() {
        let wl = group_worklist(Vec::new(), "2026-08-06");
        assert_eq!(wl.total_pending, 0);
        assert!(wl.today.is_empty());
        assert!(wl.overdue.is_empty());
    }

    #[test]
    fn test_sample_list_item_row_mapping() {
        let row: SampleListItemRow = (
            1,
            "M-2026-0001".into(),
            10,
            "P-2026-0010".into(),
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
            0,
            Some("HEMOLISIS".into()),
            Some("LEVE".into()),
            None,
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "M-2026-0001");
        assert_eq!(row.13, 3); // result_count
        assert_eq!(row.14, 1); // abnormal_count
        assert_eq!(row.15, 0); // critical_count
        assert_eq!(row.16.as_deref(), Some("HEMOLISIS"));
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
    fn test_get_worklist_today_and_overdue() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);

        // Pendiente recibida hoy.
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, CURRENT_TIMESTAMP, 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        // Pendiente de ayer (requiere atención).
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (2, 'M-2026-0002', ?, 1, DATEADD(-1 DAY TO CURRENT_TIMESTAMP), 'EN_PROCESO')",
            (&patient_id,),
        )
        .unwrap();
        // Finalizada: NO debe aparecer en la bandeja.
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (3, 'M-2026-0003', ?, 1, CURRENT_TIMESTAMP, 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();

        let wl = get_worklist(&mut conn).unwrap();
        assert_eq!(wl.total_pending, 2);
        assert_eq!(wl.today.len(), 1);
        assert_eq!(wl.today[0].count, 1);
        assert_eq!(wl.today[0].samples[0].code, "M-2026-0001");
        assert!(wl.today[0].samples[0].elapsed_minutes >= 0);
        assert_eq!(wl.overdue.len(), 1);
        assert_eq!(wl.overdue[0].samples[0].code, "M-2026-0002");
        assert!(wl.overdue[0].samples[0].elapsed_minutes >= 1000);

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
        .unwrap(); // RECIBIDA -> FINALIZADA sin resultados (debe fallar)
        let result = set_status(&mut conn, 1, "FINALIZADA");
        assert!(result.is_err());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_reject_reopen_records_event_trail() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);

        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        // Rechazo con motivo → evento REJECTED registrado.
        let rejected = reject_sample(&mut conn, 1, "Hemólisis marcada", "vet_ana").unwrap();
        assert_eq!(rejected.status, "RECHAZADA");
        assert_eq!(
            rejected.rejection_reason.as_deref(),
            Some("Hemólisis marcada")
        );

        let events = list_sample_events(&mut conn, 1).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "REJECTED");
        assert_eq!(events[0].username, "vet_ana");
        assert_eq!(events[0].reason.as_deref(), Some("Hemólisis marcada"));

        // Reapertura → evento REOPENED registrado (los campos se limpian).
        let reopened = reopen_sample(&mut conn, 1, "vet_ana").unwrap();
        assert_eq!(reopened.status, "RECIBIDA");
        assert!(reopened.rejection_reason.is_none());

        let events = list_sample_events(&mut conn, 1).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "REOPENED");
        assert!(events[0].reason.is_none());
        assert_eq!(events[1].event_type, "REJECTED");

        // Sin eventos para otra muestra.
        assert!(list_sample_events(&mut conn, 999).unwrap().is_empty());

        test_helpers::cleanup_test_db(&db_path);
    }
}

/// Conteo de muestras agrupado por estado (para las pestañas de la mesa de
/// trabajo) más los contadores ABNORMAL y CRITICAL (muestras con al menos un
/// resultado fuera de rango / crítico).
pub fn count_by_status(
    conn: &mut SimpleConnection,
) -> Result<Vec<crate::models::status_count::StatusCount>, AppError> {
    let rows: Vec<(String, i32)> = conn
        .query(
            "SELECT STATUS, COUNT(*) FROM SAMPLES GROUP BY STATUS
                UNION ALL
                SELECT 'ABNORMAL' AS STATUS, COUNT(*) FROM SAMPLES s
                WHERE EXISTS (SELECT 1 FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('ALTO', 'BAJO'))
                UNION ALL
                SELECT 'CRITICAL' AS STATUS, COUNT(*) FROM SAMPLES s
                WHERE EXISTS (SELECT 1 FROM LAB_RESULTS lr
                  WHERE lr.SAMPLE_ID = s.ID AND lr.STATUS IN ('CRITICO_ALTO', 'CRITICO_BAJO'))
                ORDER BY 1",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(status, count)| crate::models::status_count::StatusCount { status, count })
        .collect())
}
