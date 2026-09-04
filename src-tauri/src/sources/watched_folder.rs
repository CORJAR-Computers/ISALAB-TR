//! Driver de **carpeta vigilada**: el analizador exporta resultados a una
//! carpeta local (USB/software del equipo) y este driver los importa con el
//! mapeo guardado en `ANALYZER_SOURCE_COLUMNS`, reutilizando el pipeline de
//! importación CSV existente (`repositories::import`).
//!
//! Semántica por archivo:
//! * Se procesa la primera vez que aparece en la carpeta.
//! * Si el archivo ya se procesó (fila en `ANALYZER_IMPORT_JOBS`) solo se
//!   reprocesa cuando su fecha de modificación es **posterior** a la última
//!   vez que se procesó (el analizador re-exportó el mismo nombre, p. ej.
//!   `RESULTADOS.CSV` diario). La importación es idempotente (upsert).
//! * Tras importar con éxito, el archivo se mueve a `<carpeta>/importados/`
//!   para mantener la carpeta limpia. Si falla, se queda en su sitio y el
//!   error queda en la cola para reintentar desde la UI.

use std::path::{Path, PathBuf};

use chrono::TimeZone;
use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::analyzer_source::SourceFileOutcome;
use crate::models::import::AnalyzerImportMapping;
use crate::repositories::import as import_repo;
use crate::sources::AnalyzerDriver;

/// Subcarpeta donde se archivan los archivos importados con éxito.
const PROCESSED_SUBDIR: &str = "importados";

fn mtime_secs(path: &Path) -> Option<i64> {
    let m = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(m.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64)
}

/// Convierte "YYYY-MM-DD HH:MM:SS" (Firebird usa hora local del equipo) a
/// epoch seconds, interpretándola en la zona local del proceso.
fn db_ts_to_secs(ts: &str) -> Option<i64> {
    let naive = chrono::NaiveDateTime::parse_from_str(ts.trim(), "%Y-%m-%d %H:%M:%S").ok()?;
    chrono::Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.timestamp())
}

pub struct WatchedFolderCsvSource {
    pub source_id: i32,
    pub folder: String,
    pub mapping: AnalyzerImportMapping,
}

impl AnalyzerDriver for WatchedFolderCsvSource {
    fn source_type(&self) -> &'static str {
        "WATCHED_FOLDER"
    }

    fn poll_once(
        &mut self,
        conn: &mut SimpleConnection,
    ) -> Result<Vec<SourceFileOutcome>, AppError> {
        let dir = PathBuf::from(&self.folder);
        let entries = std::fs::read_dir(&dir).map_err(|e| {
            AppError::Validation(format!(
                "No se pudo leer la carpeta vigilada {}: {e}",
                self.folder
            ))
        })?;

        let mut files: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.to_lowercase().ends_with(".csv") {
                files.push(path);
            }
        }
        files.sort();

        let mut outcomes = Vec::new();
        for path in files {
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let outcome = self.process_file(conn, &path, &file_name)?;
            if let Some(o) = outcome {
                outcomes.push(o);
            }
        }
        Ok(outcomes)
    }
}

impl WatchedFolderCsvSource {
    fn process_file(
        &mut self,
        conn: &mut SimpleConnection,
        path: &Path,
        file_name: &str,
    ) -> Result<Option<SourceFileOutcome>, AppError> {
        // Último procesamiento conocido de este archivo.
        let last: Option<(String, String)> = conn
            .query_first(
                "SELECT STATUS, LEFT(CAST(PROCESSED_AT AS VARCHAR(60)), 19)
                 FROM ANALYZER_IMPORT_JOBS WHERE SOURCE_ID = ? AND FILE_NAME = ?",
                (&self.source_id, file_name),
            )
            .map_err(AppError::from)?;

        // Si ya se procesó y el archivo no cambió desde entonces → saltar.
        if let Some((_, processed_at)) = &last {
            let new_mtime = mtime_secs(path).unwrap_or(i64::MAX); // si no se puede leer, procesar
            let processed = db_ts_to_secs(processed_at).unwrap_or(i64::MIN);
            if new_mtime <= processed {
                return Ok(None);
            }
        }

        // Importa con el mapeo guardado (reutiliza todo el pipeline CSV).
        let result = import_repo::import(conn, &path.to_string_lossy(), &self.mapping);

        match result {
            Ok(summary) => {
                // Si NINGUNA fila pudo importarse (p. ej. las muestras aún no
                // se registraron en el sistema), NO archivamos: el archivo
                // vuelve a intentarse cuando cambie o al reintentar desde la
                // UI, evitando perder resultados que llegaron antes que la
                // recepción de la muestra.
                if summary.results_imported == 0 && !summary.skipped.is_empty() {
                    let first = summary
                        .skipped
                        .first()
                        .map(|s| s.reason.clone())
                        .unwrap_or_default();
                    let msg = if summary.skipped.len() == 1 {
                        format!("ningún resultado importado: {first}")
                    } else {
                        format!(
                            "ningún resultado importado ({} filas): {first}",
                            summary.skipped.len()
                        )
                    };
                    return Ok(Some(SourceFileOutcome {
                        file_name: file_name.to_string(),
                        status: "FALLIDO".into(),
                        samples_updated: 0,
                        results_imported: 0,
                        skipped_rows: summary.skipped.len() as i32,
                        error_msg: Some(msg),
                    }));
                }

                // Archiva el archivo procesado (best effort).
                let archived = self.archive(path);
                let error_msg = if !archived {
                    Some("importado pero no se pudo archivar el archivo".into())
                } else {
                    None
                };
                Ok(Some(SourceFileOutcome {
                    file_name: file_name.to_string(),
                    status: "IMPORTADO".into(),
                    samples_updated: summary.samples_updated,
                    results_imported: summary.results_imported,
                    skipped_rows: summary.skipped.len() as i32,
                    error_msg,
                }))
            }
            Err(e) => {
                // El archivo se queda en la carpeta; el error queda en la cola.
                Ok(Some(SourceFileOutcome {
                    file_name: file_name.to_string(),
                    status: "FALLIDO".into(),
                    samples_updated: 0,
                    results_imported: 0,
                    skipped_rows: 0,
                    error_msg: Some(e.to_string()),
                }))
            }
        }
    }

    /// Mueve el archivo a `<carpeta>/importados/` (lo crea si falta).
    fn archive(&self, path: &Path) -> bool {
        let target_dir = PathBuf::from(&self.folder).join(PROCESSED_SUBDIR);
        if std::fs::create_dir_all(&target_dir).is_err() {
            return false;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let target = target_dir.join(&name);
        // Si ya existe un archivo con ese nombre, se sobrescribe (último export).
        let _ = std::fs::remove_file(&target);
        std::fs::rename(path, &target).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::import::ImportColumnMapping;
    use crate::repositories::analyzer_sources as sources_repo;
    use crate::repositories::clinical_history;
    use crate::repositories::samples;
    use crate::test_helpers;
    use std::fs;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        test_helpers::setup_test_db()
    }

    /// Crea una carpeta temporal única para el test y devuelve su ruta.
    fn temp_folder(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "isalab_watch_{tag}_{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_csv(folder: &Path, name: &str, content: &str) -> PathBuf {
        let p = folder.join(name);
        fs::write(&p, content).unwrap();
        p
    }

    /// Configura DB con paciente + tipo de muestra + analito, crea una muestra
    /// RECIBIDA y devuelve (conn, db_path, sample_code).
    fn seed_sample_with_code() -> (SimpleConnection, PathBuf, String) {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);
        test_helpers::insert_test_reference_range(&mut conn);

        let sample = clinical_history::create_sample(
            &mut conn,
            &crate::models::sample::CreateSampleInput {
                patient_id,
                sample_type_id: 1,
                received_at: "2026-08-15 09:00:00".into(),
                collected_by: None,
                notes: None,
                analyzer_id: Some(2),
                quality_index: None,
                quality_severity: None,
                quality_note: None,
            },
        )
        .unwrap();
        (conn, db_path, sample.code)
    }

    #[test]
    fn test_watched_folder_imports_and_archives() {
        let (mut conn, db_path, code) = seed_sample_with_code();

        let folder = temp_folder("ok");
        // Guarda la fuente configurada para el MINDRAY (ID 2).
        let src = sources_repo::save(
            &mut conn,
            &crate::models::analyzer_source::SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some(folder.to_string_lossy().to_string()),
                enabled: true,
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

        // CSV: código de muestra + valor del analito HCT.
        write_csv(&folder, "M-001.csv", &format!("{code};HCT\n{code};48\n"));

        let mut driver = WatchedFolderCsvSource {
            source_id: src.id,
            folder: folder.to_string_lossy().to_string(),
            mapping: src.mapping.clone().unwrap(),
        };
        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, "IMPORTADO");
        assert_eq!(outcomes[0].samples_updated, 1);
        assert_eq!(outcomes[0].results_imported, 1);
        // El supervisor registra cada outcome en la cola.
        for o in &outcomes {
            sources_repo::record_job(&mut conn, src.id, o).unwrap();
        }

        // La cola registra el trabajo y el archivo se archivó.
        let jobs = sources_repo::list_jobs(&mut conn, src.id, 10).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, "IMPORTADO");
        assert!(folder.join("importados/M-001.csv").exists());

        // El resultado quedó registrado con validación (48 normal: 37-55).
        let sample = samples::get(&mut conn, 1).unwrap().unwrap();
        let res = &sample.results[0];
        assert_eq!(res.analyte_id, 1);
        assert_eq!(res.value, 48.0);
        assert_eq!(res.status, "NORMAL");

        // Segundo sondeo: sin archivos nuevos → sin trabajos nuevos.
        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert!(outcomes.is_empty());

        fs::remove_dir_all(&folder).ok();
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_watched_folder_reimports_when_file_changes() {
        let (mut conn, db_path, code) = seed_sample_with_code();
        let folder = temp_folder("reimport");

        let src = sources_repo::save(
            &mut conn,
            &crate::models::analyzer_source::SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some(folder.to_string_lossy().to_string()),
                enabled: true,
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

        let mut driver = WatchedFolderCsvSource {
            source_id: src.id,
            folder: folder.to_string_lossy().to_string(),
            mapping: src.mapping.clone().unwrap(),
        };

        // Primera exportación diaria del equipo.
        let file = write_csv(&folder, "DIARIO.csv", &format!("{code};HCT\n{code};48\n"));
        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert_eq!(outcomes.len(), 1);
        for o in &outcomes {
            sources_repo::record_job(&mut conn, src.id, o).unwrap();
        }
        // El archivo se archivó al importarse: la segunda exportación del
        // mismo nombre es un archivo nuevo.
        std::thread::sleep(std::time::Duration::from_millis(1200));
        fs::write(&file, format!("{code};HCT\n{code};60\n")).unwrap();

        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, "IMPORTADO");

        // El valor se actualizó (upsert) y quedó ALTO (60 > 55).
        let sample = samples::get(&mut conn, 1).unwrap().unwrap();
        let res = sample.results.iter().find(|r| r.analyte_id == 1).unwrap();
        assert_eq!(res.value, 60.0);
        assert_eq!(res.status, "ALTO");

        fs::remove_dir_all(&folder).ok();
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_watched_folder_keeps_file_when_sample_missing() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_analyte(&mut conn);
        let folder = temp_folder("missing");

        let src = sources_repo::save(
            &mut conn,
            &crate::models::analyzer_source::SaveAnalyzerSourceInput {
                analyzer_id: 2,
                source_type: None,
                folder_path: Some(folder.to_string_lossy().to_string()),
                enabled: true,
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

        // CSV con una muestra que aún no se registró en el sistema: el driver
        // NO debe archivar el archivo (para no perder los resultados cuando la
        // muestra se registre) y debe marcarlo FALLIDO en la cola.
        let file = write_csv(&folder, "M-9999-X.csv", "M-9999-X;HCT\nM-9999-X;50\n");
        let mut driver = WatchedFolderCsvSource {
            source_id: src.id,
            folder: folder.to_string_lossy().to_string(),
            mapping: src.mapping.clone().unwrap(),
        };
        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, "FALLIDO");
        assert_eq!(outcomes[0].results_imported, 0);
        assert_eq!(outcomes[0].skipped_rows, 1);
        assert!(outcomes[0]
            .error_msg
            .as_deref()
            .unwrap()
            .contains("no encontrada"));
        for o in &outcomes {
            sources_repo::record_job(&mut conn, src.id, o).unwrap();
        }
        // El archivo permanece en la carpeta (sin archivar).
        assert!(file.exists());
        assert!(!folder.join("importados/M-9999-X.csv").exists());

        // Segunda pasada sin cambios → el FALLIDO registrado evita reintentos.
        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert!(outcomes.is_empty());

        // La muestra se registra y el usuario reintenta (borra el trabajo): el
        // siguiente sondeo lo importa y archiva.
        let jobs = sources_repo::list_jobs(&mut conn, src.id, 10).unwrap();
        sources_repo::delete_job(&mut conn, jobs[0].id).unwrap();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_reference_range(&mut conn);
        let sample = clinical_history::create_sample(
            &mut conn,
            &crate::models::sample::CreateSampleInput {
                patient_id,
                sample_type_id: 1,
                received_at: "2026-08-15 09:00:00".into(),
                collected_by: None,
                notes: None,
                analyzer_id: Some(2),
                quality_index: None,
                quality_severity: None,
                quality_note: None,
            },
        )
        .unwrap();
        // Reescribe el CSV con el código real de la muestra.
        fs::write(&file, format!("{};HCT\n{};50\n", sample.code, sample.code)).unwrap();

        let outcomes = driver.poll_once(&mut conn).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, "IMPORTADO");
        assert_eq!(outcomes[0].results_imported, 1);
        assert!(folder.join("importados/M-9999-X.csv").exists());

        fs::remove_dir_all(&folder).ok();
        test_helpers::cleanup_test_db(&db_path);
    }
}
