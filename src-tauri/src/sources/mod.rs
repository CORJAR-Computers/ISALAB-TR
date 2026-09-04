//! # Fuentes de resultados de analizadores (AnalyzerSource)
//!
//! Abstracción sobre *dónde* llegan los resultados de un analizador: hoy la
//! única fuente es una **carpeta vigilada** donde el equipo exporta CSV
//! (MINDRAY, IDEXX…), pero el trait deja el molde listo para drivers futuros
//! (ASTM E1381 por puerto serie, HL7/ORU por red…).
//!
//! Cada fuente se configura por analizador (`ANALYZER_SOURCES`) con su mapeo
//! columna → analito guardado (`ANALYZER_SOURCE_COLUMNS`). El supervisor
//! arranca en segundo plano al iniciar la app y sondea las fuentes habilitadas
//! cada pocos segundos, reutilizando el pipeline de importación CSV existente
//! (`repositories::import`). Cada archivo procesado queda registrado en
//! `ANALYZER_IMPORT_JOBS` (cola con estado por archivo).

use std::time::Duration;

use rsfbclient::SimpleConnection;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::analyzer_source::{AnalyzerSource, SourceFileOutcome};
use crate::models::import::AnalyzerImportMapping;
use crate::repositories::analyzer_sources as sources_repo;
use crate::sources::watched_folder::WatchedFolderCsvSource;

pub mod watched_folder;

/// Intervalo entre sondeos de las carpetas vigiladas.
pub const POLL_INTERVAL_SECS: u64 = 3;

/// Contrato de un driver de fuente de resultados. Cada driver conoce cómo
/// listar los archivos/payloads pendientes y cómo importarlos reutilizando
/// el mapeo configurado en `ANALYZER_SOURCE_COLUMNS`.
pub trait AnalyzerDriver: Send {
    /// Identificador corto del tipo de fuente (coincide con SOURCE_TYPE).
    fn source_type(&self) -> &'static str;

    /// Una pasada de sondeo: procesa lo que haya pendiente en la fuente y
    /// devuelve el resultado por archivo (para registrar en la cola). El
    /// driver consulta `ANALYZER_IMPORT_JOBS` para no reprocesar archivos ya
    /// importados y para reintentar los fallidos solo si cambiaron.
    fn poll_once(
        &mut self,
        conn: &mut SimpleConnection,
    ) -> Result<Vec<SourceFileOutcome>, AppError>;
}

/// Construye el driver adecuado para una fuente configurada. Devuelve None
/// si falta el mapeo o el tipo aún no tiene implementación.
pub fn driver_for(source: &AnalyzerSource) -> Option<Box<dyn AnalyzerDriver>> {
    let folder = source.folder_path.as_deref()?;
    let mapping: AnalyzerImportMapping = source.mapping.clone()?;
    match source.source_type.as_str() {
        "WATCHED_FOLDER" => Some(Box::new(WatchedFolderCsvSource {
            source_id: source.id,
            folder: folder.to_string(),
            mapping,
        })),
        other => {
            eprintln!("[analyzer_sources] tipo de fuente sin driver: {other}");
            None
        }
    }
}

/// Un ciclo del supervisor: sondea todas las fuentes habilitadas y registra
/// los resultados en la cola. Los errores por fuente se aíslan para que una
/// fuente rota no tumbe el resto del ciclo.
pub fn run_supervisor_cycle(pool: &DbPool) {
    let Ok(mut pooled) = pool.acquire() else {
        return; // El pool aún no está listo (setup en curso)
    };
    let sources = match sources_repo::list_enabled(pooled.conn()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[analyzer_sources] no se pudo listar fuentes: {e}");
            return;
        }
    };

    for source in sources {
        let Some(mut driver) = driver_for(&source) else {
            continue;
        };
        let outcomes = match driver.poll_once(pooled.conn()) {
            Ok(o) => o,
            Err(e) => {
                eprintln!(
                    "[analyzer_sources] sondeo de {} ({}) falló: {e}",
                    source.analyzer_name,
                    source.folder_path.as_deref().unwrap_or("?")
                );
                continue;
            }
        };
        for outcome in &outcomes {
            if let Err(e) = sources_repo::record_job(pooled.conn(), source.id, outcome) {
                eprintln!("[analyzer_sources] no se pudo registrar trabajo: {e}");
            }
        }
        // Actualiza LAST_POLL_AT aunque no haya habido archivos.
        if sources_repo::touch_poll(pooled.conn(), source.id).is_err() {
            // No crítico: se reintentará en el próximo ciclo.
        }
    }
}

/// Arranca el hilo de supervisión en segundo plano. Vive mientras la app
/// corra; el proceso se encarga de terminarlo al salir.
pub fn start_supervisor(pool: DbPool) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
        run_supervisor_cycle(&pool);
    });
}
