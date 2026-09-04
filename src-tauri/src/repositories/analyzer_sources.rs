use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::analyzer_source::{
    build_mapping, AnalyzerImportJob, AnalyzerSource, SaveAnalyzerSourceInput, SourceFileOutcome,
};
use crate::repositories::next_id;

pub const DEFAULT_SOURCE_TYPE: &str = "WATCHED_FOLDER";

/// Tipo de fila de ANALYZER_SOURCES + nombre del analizador.
type SourceRow = (
    i32,            // id
    i32,            // analyzer_id
    String,         // analyzer_name
    String,         // source_type
    Option<String>, // folder_path
    Option<i32>,    // sample_code_column
    bool,           // enabled
    Option<String>, // last_poll_at
);

/// Tipo de fila de ANALYZER_SOURCE_COLUMNS (columna → analito).
type ColumnRow = (i32, i32);

/// Fila de ANALYZER_IMPORT_JOBS con joins a fuente y analizador.
type JobRow = (
    i32,            // id
    i32,            // source_id
    i32,            // analyzer_id
    String,         // analyzer_name
    String,         // file_name
    String,         // status
    i32,            // samples_updated
    i32,            // results_imported
    i32,            // skipped_rows
    Option<String>, // error_msg
    String,         // processed_at
);

/// Cuerpo del SELECT de trabajos (FIRST n va antes, según Firebird).
const JOB_COLUMNS: &str = "
    j.ID, j.SOURCE_ID, s.ANALYZER_ID, a.NAME, j.FILE_NAME,
    j.STATUS, j.SAMPLES_UPDATED, j.RESULTS_IMPORTED, j.SKIPPED_ROWS,
    j.ERROR_MSG, LEFT(CAST(j.PROCESSED_AT AS VARCHAR(60)), 19)
    FROM ANALYZER_IMPORT_JOBS j
    JOIN ANALYZER_SOURCES s ON s.ID = j.SOURCE_ID
    JOIN ANALYZERS a ON a.ID = s.ANALYZER_ID";

fn map_job(r: JobRow) -> AnalyzerImportJob {
    AnalyzerImportJob {
        id: r.0,
        source_id: r.1,
        analyzer_id: r.2,
        analyzer_name: r.3,
        file_name: r.4,
        status: r.5,
        samples_updated: r.6,
        results_imported: r.7,
        skipped_rows: r.8,
        error_msg: r.9,
        processed_at: r.10,
    }
}

const SOURCE_SELECT: &str = "
    SELECT s.ID, s.ANALYZER_ID, a.NAME, s.SOURCE_TYPE, s.FOLDER_PATH,
           s.SAMPLE_CODE_COLUMN, s.ENABLED,
           LEFT(CAST(s.LAST_POLL_AT AS VARCHAR(60)), 19)
    FROM ANALYZER_SOURCES s
    JOIN ANALYZERS a ON a.ID = s.ANALYZER_ID";

fn map_source(r: SourceRow, columns: &[(i32, i32)], mapped_columns: i32) -> AnalyzerSource {
    let mapping = build_mapping(r.5, columns);
    AnalyzerSource {
        id: r.0,
        analyzer_id: r.1,
        analyzer_name: r.2,
        source_type: r.3,
        folder_path: r.4,
        enabled: r.6,
        last_poll_at: r.7,
        mapping,
        mapped_columns,
    }
}

/// Carga las columnas mapeadas (columna_csv, analito) de una fuente.
fn load_columns(conn: &mut SimpleConnection, source_id: i32) -> Result<Vec<(i32, i32)>, AppError> {
    let rows: Vec<ColumnRow> = conn
        .query(
            "SELECT COLUMN_INDEX, ANALYTE_ID FROM ANALYZER_SOURCE_COLUMNS
             WHERE SOURCE_ID = ? ORDER BY COLUMN_INDEX",
            (&source_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows)
}

fn load_source(conn: &mut SimpleConnection, row: SourceRow) -> Result<AnalyzerSource, AppError> {
    let columns = load_columns(conn, row.0)?;
    Ok(map_source(row, &columns, columns.len() as i32))
}

/// Fuente configurada de un analizador (o None si no tiene).
pub fn get_for_analyzer(
    conn: &mut SimpleConnection,
    analyzer_id: i32,
) -> Result<Option<AnalyzerSource>, AppError> {
    let row: Option<SourceRow> = conn
        .query_first(
            &format!("{SOURCE_SELECT} WHERE s.ANALYZER_ID = ?"),
            (&analyzer_id,),
        )
        .map_err(AppError::from)?;
    match row {
        Some(r) => Ok(Some(load_source(conn, r)?)),
        None => Ok(None),
    }
}

/// Todas las fuentes configuradas (para la UI de gestión).
pub fn list(conn: &mut SimpleConnection) -> Result<Vec<AnalyzerSource>, AppError> {
    let rows: Vec<SourceRow> = conn
        .query(&format!("{SOURCE_SELECT} ORDER BY a.NAME"), ())
        .map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(load_source(conn, r)?);
    }
    Ok(out)
}

/// Fuentes habilitadas con su mapeo completo (para el supervisor que sondea
/// carpetas en segundo plano).
pub fn list_enabled(conn: &mut SimpleConnection) -> Result<Vec<AnalyzerSource>, AppError> {
    let rows: Vec<SourceRow> = conn
        .query(
            &format!("{SOURCE_SELECT} WHERE s.ENABLED = TRUE AND s.FOLDER_PATH IS NOT NULL"),
            (),
        )
        .map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(load_source(conn, r)?);
    }
    Ok(out)
}

/// Crea o reemplaza la fuente de un analizador. Si `folder_path` es None se
/// elimina la fuente (y su cola). El mapeo es opcional: permite guardar solo
/// la carpeta primero y configurar columnas después desde un CSV de ejemplo.
pub fn save(
    conn: &mut SimpleConnection,
    input: &SaveAnalyzerSourceInput,
) -> Result<Option<AnalyzerSource>, AppError> {
    let source_type = input.source_type.as_deref().unwrap_or(DEFAULT_SOURCE_TYPE);

    // Valida que el analizador exista y esté activo.
    let analyzer: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM ANALYZERS WHERE ID = ? AND IS_ACTIVE = TRUE",
            (&input.analyzer_id,),
        )
        .map_err(AppError::from)?;
    if analyzer.is_none() {
        return Err(AppError::Validation(format!(
            "El analizador {} no existe o está inactivo",
            input.analyzer_id
        )));
    }

    // Sin carpeta ⇒ eliminar la fuente.
    let folder = input
        .folder_path
        .as_deref()
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty());
    if folder.is_none() {
        conn.execute(
            "DELETE FROM ANALYZER_SOURCES WHERE ANALYZER_ID = ? AND SOURCE_TYPE = ?",
            (&input.analyzer_id, source_type),
        )
        .map_err(AppError::from)?;
        return Ok(None);
    }

    // El mapeo, si viene, debe tener columna de código y analitos.
    if let Some(m) = &input.mapping {
        if m.columns.is_empty() {
            return Err(AppError::Validation(
                "El mapeo debe incluir al menos un analito".into(),
            ));
        }
        for col in
            std::iter::once(&m.sample_code_column).chain(m.columns.iter().map(|c| &c.column_index))
        {
            if *col < 0 {
                return Err(AppError::Validation(
                    "Los índices de columna no pueden ser negativos".into(),
                ));
            }
        }
        // Valida que los analitos existan (la FK también protege el insert).
        for c in &m.columns {
            let hit: Option<(i32,)> = conn
                .query_first(
                    "SELECT 1 FROM ANALYTES WHERE ID = ? AND IS_ACTIVE = TRUE",
                    (&c.analyte_id,),
                )
                .map_err(AppError::from)?;
            if hit.is_none() {
                return Err(AppError::Validation(format!(
                    "El analito {} no existe o está inactivo",
                    c.analyte_id
                )));
            }
        }
    }

    // Upsert de la fuente.
    let existing: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM ANALYZER_SOURCES WHERE ANALYZER_ID = ? AND SOURCE_TYPE = ?",
            (&input.analyzer_id, source_type),
        )
        .map_err(AppError::from)?;

    let (source_id, is_new) = match existing {
        Some((id,)) => (id, false),
        None => (next_id(conn, "GEN_ANALYZER_SOURCES_ID")?, true),
    };

    if is_new {
        conn.execute(
            "INSERT INTO ANALYZER_SOURCES
                (ID, ANALYZER_ID, SOURCE_TYPE, FOLDER_PATH, SAMPLE_CODE_COLUMN, ENABLED)
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                &source_id,
                &input.analyzer_id,
                source_type,
                &folder,
                &input.mapping.as_ref().map(|m| m.sample_code_column),
                &input.enabled,
            ),
        )
        .map_err(AppError::from)?;
    } else {
        conn.execute(
            "UPDATE ANALYZER_SOURCES
                SET FOLDER_PATH = ?, SAMPLE_CODE_COLUMN = ?, ENABLED = ?
              WHERE ID = ?",
            (
                &folder,
                &input.mapping.as_ref().map(|m| m.sample_code_column),
                &input.enabled,
                &source_id,
            ),
        )
        .map_err(AppError::from)?;
    }

    // Reemplaza las columnas mapeadas (DELETE + INSERT como hace save_panel).
    if let Some(m) = &input.mapping {
        conn.execute(
            "DELETE FROM ANALYZER_SOURCE_COLUMNS WHERE SOURCE_ID = ?",
            (&source_id,),
        )
        .map_err(AppError::from)?;
        for c in &m.columns {
            conn.execute(
                "INSERT INTO ANALYZER_SOURCE_COLUMNS (SOURCE_ID, COLUMN_INDEX, ANALYTE_ID)
                 VALUES (?, ?, ?)",
                (&source_id, &c.column_index, &c.analyte_id),
            )
            .map_err(AppError::from)?;
        }
    }

    let row: Option<SourceRow> = conn
        .query_first(&format!("{SOURCE_SELECT} WHERE s.ID = ?"), (&source_id,))
        .map_err(AppError::from)?;
    match row {
        Some(r) => Ok(Some(load_source(conn, r)?)),
        None => Err(AppError::Internal(
            "Fuente guardada pero no recuperada".into(),
        )),
    }
}

/// Elimina una fuente (y su cola por CASCADE).
pub fn delete(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM ANALYZER_SOURCES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

/// Actualiza LAST_POLL_AT tras sondear una fuente.
pub fn touch_poll(conn: &mut SimpleConnection, source_id: i32) -> Result<(), AppError> {
    conn.execute(
        "UPDATE ANALYZER_SOURCES SET LAST_POLL_AT = CURRENT_TIMESTAMP WHERE ID = ?",
        (&source_id,),
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// Últimos trabajos de una fuente (cola de importación), más recientes primero.
pub fn list_jobs(
    conn: &mut SimpleConnection,
    source_id: i32,
    limit: i32,
) -> Result<Vec<AnalyzerImportJob>, AppError> {
    let rows: Vec<JobRow> = conn
        .query(
            &format!(
                "SELECT FIRST {limit} {JOB_COLUMNS} WHERE j.SOURCE_ID = ? ORDER BY j.PROCESSED_AT DESC, j.ID DESC"
            ),
            (&source_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_job).collect())
}

/// Cola completa de todos los trabajos fallidos (para una vista global).
pub fn list_failed_jobs(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<AnalyzerImportJob>, AppError> {
    let rows: Vec<JobRow> = conn
        .query(
            &format!(
                "SELECT FIRST {limit} {JOB_COLUMNS} WHERE j.STATUS = 'FALLIDO' ORDER BY j.PROCESSED_AT DESC, j.ID DESC"
            ),
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_job).collect())
}

/// Registra (upsert por archivo) el resultado de procesar un CSV.
/// Un archivo que falla se vuelve a intentar en el próximo sondeo si el
/// contenido cambió, o al pulsar "Reintentar" en la UI (que borra la fila).
pub fn record_job(
    conn: &mut SimpleConnection,
    source_id: i32,
    outcome: &SourceFileOutcome,
) -> Result<AnalyzerImportJob, AppError> {
    let existing: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM ANALYZER_IMPORT_JOBS WHERE SOURCE_ID = ? AND FILE_NAME = ?",
            (&source_id, &outcome.file_name),
        )
        .map_err(AppError::from)?;

    let id = match existing {
        Some((id,)) => {
            conn.execute(
                "UPDATE ANALYZER_IMPORT_JOBS
                    SET STATUS = ?, SAMPLES_UPDATED = ?, RESULTS_IMPORTED = ?,
                        SKIPPED_ROWS = ?, ERROR_MSG = ?, PROCESSED_AT = CURRENT_TIMESTAMP
                  WHERE ID = ?",
                (
                    &outcome.status,
                    &outcome.samples_updated,
                    &outcome.results_imported,
                    &outcome.skipped_rows,
                    &outcome.error_msg,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            id
        }
        None => {
            let nid = next_id(conn, "GEN_ANALYZER_IMPORT_JOBS_ID")?;
            conn.execute(
                "INSERT INTO ANALYZER_IMPORT_JOBS
                    (ID, SOURCE_ID, FILE_NAME, STATUS, SAMPLES_UPDATED,
                     RESULTS_IMPORTED, SKIPPED_ROWS, ERROR_MSG)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &nid,
                    &source_id,
                    &outcome.file_name,
                    &outcome.status,
                    &outcome.samples_updated,
                    &outcome.results_imported,
                    &outcome.skipped_rows,
                    &outcome.error_msg,
                ),
            )
            .map_err(AppError::from)?;
            nid
        }
    };

    let row: Option<JobRow> = conn
        .query_first(&format!("SELECT {JOB_COLUMNS} WHERE j.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    row.map(map_job)
        .ok_or_else(|| AppError::Internal("Trabajo guardado pero no recuperado".into()))
}

/// Borra un trabajo (para reintentar un archivo fallido desde la UI).
pub fn delete_job(conn: &mut SimpleConnection, job_id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM ANALYZER_IMPORT_JOBS WHERE ID = ?", (&job_id,))
        .map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::import::{AnalyzerImportMapping, ImportColumnMapping};
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    fn mapping_fixture() -> AnalyzerImportMapping {
        AnalyzerImportMapping {
            sample_code_column: 0,
            columns: vec![
                ImportColumnMapping {
                    column_index: 1,
                    analyte_id: 1,
                },
                ImportColumnMapping {
                    column_index: 2,
                    analyte_id: 2,
                },
            ],
        }
    }

    #[test]
    fn test_save_and_get_source() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);

        let saved = save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2, // MINDRAY B2800 sembrado
                source_type: None,
                folder_path: Some("C:/exports/mindray".into()),
                enabled: true,
                mapping: Some(mapping_fixture()),
            },
        )
        .unwrap()
        .expect("fuente guardada");
        assert_eq!(saved.analyzer_id, 2);
        assert_eq!(saved.analyzer_name, "MINDRAY B2800");
        assert_eq!(saved.source_type, "WATCHED_FOLDER");
        assert!(saved.enabled);
        let m = saved.mapping.expect("mapeo guardado");
        assert_eq!(m.sample_code_column, 0);
        assert_eq!(m.columns.len(), 2);
        assert_eq!(saved.mapped_columns, 2);

        // Se recupera por analizador.
        let got = get_for_analyzer(&mut conn, 2).unwrap().unwrap();
        assert_eq!(got.id, saved.id);
        assert_eq!(got.folder_path.as_deref(), Some("C:/exports/mindray"));

        // Aparece en list_enabled (está habilitada).
        let enabled = list_enabled(&mut conn).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, saved.id);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_replaces_mapping_and_disables() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);

        save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some("C:/exp".into()),
                enabled: true,
                mapping: Some(mapping_fixture()),
            },
        )
        .unwrap();

        // Guardar de nuevo con otro mapeo y deshabilitada.
        let updated = save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some("C:/exp2".into()),
                enabled: false,
                mapping: Some(AnalyzerImportMapping {
                    sample_code_column: 0,
                    columns: vec![ImportColumnMapping {
                        column_index: 1,
                        analyte_id: 1,
                    }],
                }),
            },
        )
        .unwrap()
        .unwrap();
        assert!(!updated.enabled);
        assert_eq!(updated.folder_path.as_deref(), Some("C:/exp2"));
        assert_eq!(updated.mapping.unwrap().columns.len(), 1);
        assert!(list_enabled(&mut conn).unwrap().is_empty());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_delete_source_when_folder_cleared() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);

        save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some("C:/exp".into()),
                enabled: true,
                mapping: Some(mapping_fixture()),
            },
        )
        .unwrap();

        let removed = save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: None,
                enabled: false,
                mapping: None,
            },
        )
        .unwrap();
        assert!(removed.is_none());
        assert!(get_for_analyzer(&mut conn, 2).unwrap().is_none());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_invalid_analyte_rejected() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);

        let err = save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some("C:/exp".into()),
                enabled: true,
                mapping: Some(AnalyzerImportMapping {
                    sample_code_column: 0,
                    columns: vec![ImportColumnMapping {
                        column_index: 1,
                        analyte_id: 9999,
                    }],
                }),
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("analito 9999"));

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_job_log_upsert_and_list() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);
        let src = save(
            &mut conn,
            &SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some("C:/exp".into()),
                enabled: true,
                mapping: Some(mapping_fixture()),
            },
        )
        .unwrap()
        .unwrap();

        // Primer procesamiento: éxito.
        let j1 = record_job(
            &mut conn,
            src.id,
            &SourceFileOutcome {
                file_name: "M-001.csv".into(),
                status: "IMPORTADO".into(),
                samples_updated: 3,
                results_imported: 6,
                skipped_rows: 0,
                error_msg: None,
            },
        )
        .unwrap();
        assert_eq!(j1.status, "IMPORTADO");

        // Reprocesamiento del mismo archivo: upsert, no duplica.
        let j2 = record_job(
            &mut conn,
            src.id,
            &SourceFileOutcome {
                file_name: "M-001.csv".into(),
                status: "IMPORTADO".into(),
                samples_updated: 3,
                results_imported: 6,
                skipped_rows: 1,
                error_msg: Some("muestra no encontrada".into()),
            },
        )
        .unwrap();
        assert_eq!(j1.id, j2.id);
        assert_eq!(j2.skipped_rows, 1);

        // Un fallido.
        record_job(
            &mut conn,
            src.id,
            &SourceFileOutcome {
                file_name: "M-002.csv".into(),
                status: "FALLIDO".into(),
                samples_updated: 0,
                results_imported: 0,
                skipped_rows: 0,
                error_msg: Some("CSV corrupto".into()),
            },
        )
        .unwrap();

        let jobs = list_jobs(&mut conn, src.id, 10).unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].file_name, "M-002.csv"); // más reciente primero
        assert_eq!(jobs[0].status, "FALLIDO");
        assert_eq!(jobs[1].results_imported, 6);

        // Borrar el fallido = reintentar.
        delete_job(&mut conn, jobs[0].id).unwrap();
        assert_eq!(list_jobs(&mut conn, src.id, 10).unwrap().len(), 1);

        test_helpers::cleanup_test_db(&db_path);
    }
}
