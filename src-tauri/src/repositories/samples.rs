use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::sample::{LabResult, Sample};
use crate::models::sample_list_item::SampleListItem;

/// Columnas de una muestra con el tipo unido.
const SAMPLE_SELECT: &str = "
    SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
           LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
           s.STATUS, s.COLLECTED_BY, s.NOTES
    FROM SAMPLES s
    JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID";

type SampleRow = (
    i32,          // id
    String,       // code
    i32,          // patient_id
    i32,          // sample_type_id
    String,       // sample_type_name
    String,       // received_at
    String,       // status
    Option<String>, // collected_by
    Option<String>, // notes
);

fn map_sample(r: SampleRow) -> Sample {
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
pub fn get(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<Sample>, AppError> {
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

    let sql = format!(
        "SELECT s.ID, s.CODE, s.PATIENT_ID, p.NAME, o.FULL_NAME, sp.NAME,
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
         ORDER BY s.RECEIVED_AT DESC, s.ID DESC"
    );

    let rows: Vec<(
        i32, String, i32, String, String, String, i32, String, String, String,
        Option<String>, Option<String>, i32, i32,
    )> = conn
        .query(&sql, (&status, &status, &like, &like, &like, &like))
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

/// Cambia el estado de una muestra validando la transición.
/// FINALIZADA requiere al menos un resultado cargado (cierre del analista).
pub fn set_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<Sample, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM SAMPLES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) = current.ok_or_else(|| {
        AppError::NotFound(format!("Muestra {id} no encontrada"))
    })?;

    let allowed = match status {
        "EN_PROCESO" => current == "RECIBIDA",
        "FINALIZADA" => {
            if !matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO") {
                false
            } else {
                let has_results: Option<(i32,)> = conn
                    .query_first(
                        "SELECT 1 FROM LAB_RESULTS WHERE SAMPLE_ID = ?",
                        (&id,),
                    )
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

    get(conn, id)?.ok_or_else(|| {
        AppError::Internal("Muestra actualizada pero no recuperada".into())
    })
}

/// Resultados de una muestra (para la ficha completa y el informe PDF).
pub fn list_results(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Vec<LabResult>, AppError> {
    let rows: Vec<(
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
    )> = conn
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

pub fn map_lab_result(
    r: (
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
    ),
) -> LabResult {
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


