//! Diagnóstico exportable y reenvío de errores de la UI al log.
//!
//! Un incidente en la clínica se reporta por teléfono: sin las versiones, el
//! estado de Firebird y los logs de las últimas horas no hay forma de
//! reconstruirlo. `export_diagnostics` escribe un ZIP con `diagnostico.txt`
//! (versión, esquema, estado de Firebird y resumen de la conexión) y los
//! archivos de log rotados en `logs/`, para adjuntarlo tal cual.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use tauri::{AppHandle, State};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::commands::db::{db_health, DbHealth};
use crate::db::migrations;
use crate::error::AppError;
use crate::logging;
use crate::state::AppState;

/// Tope de log incluido (512 KB): suficiente para un incidente reciente y sigue
/// siendo un archivo que se puede enviar por correo.
const LOG_BUDGET_BYTES: usize = 512 * 1024;
/// Nombre del reporte dentro del ZIP.
const REPORT_NAME: &str = "diagnostico.txt";

/// Reenvía al log un error o aviso capturado en la UI.
///
/// Nunca falla la operación del usuario por no poder escribir el log: si el
/// archivo no está disponible el mensaje se pierde, pero la acción sigue.
#[tauri::command]
#[specta::specta]
pub fn log_frontend(level: String, message: String, context: Option<String>) {
    logging::frontend(&level, &message, context.as_deref());
}

/// Escribe el diagnóstico en `dest_path` y devuelve la ruta.
///
/// `dest_path` lo elige el usuario con el diálogo de guardar; la extensión la
/// pone la UI (`.zip`).
#[tauri::command]
#[specta::specta]
pub fn export_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
    dest_path: String,
) -> Result<String, AppError> {
    let health = db_health(state);
    let logs = logging::read_logs(LOG_BUDGET_BYTES);
    let report = report_body(
        &health,
        &app.package_info().version.to_string(),
        &format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        &logs,
    );
    write_diagnostics(&dest_path, &report, &logs)?;

    logging::info(
        "diagnostics",
        &format!("diagnóstico exportado a {dest_path}"),
    );
    Ok(dest_path)
}

/// Escribe el ZIP del diagnóstico, creando la carpeta destino si hace falta.
///
/// Dentro: `diagnostico.txt` con el entorno y `logs/<archivo>` con cada log
/// rotado. Sin comprimir serían cientos de KB de texto; con deflate ocupa una
/// fracción y el soporte lo abre con cualquier descompresor.
fn write_diagnostics(
    dest_path: &str,
    report: &str,
    logs: &[(String, String)],
) -> Result<(), AppError> {
    let dest = PathBuf::from(dest_path);
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::Internal(format!(
                    "No se pudo preparar la carpeta del diagnóstico: {e}"
                ))
            })?;
        }
    }

    let file = fs::File::create(&dest).map_err(|e| {
        AppError::Internal(format!(
            "No se pudo crear el diagnóstico en {dest_path}: {e}"
        ))
    })?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let write_entry =
        |archive: &mut ZipWriter<fs::File>, name: &str, contents: &str| -> Result<(), AppError> {
            archive.start_file(name, options).map_err(|e| {
                AppError::Internal(format!("No se pudo añadir {name} al diagnóstico: {e}"))
            })?;
            archive.write_all(contents.as_bytes()).map_err(|e| {
                AppError::Internal(format!("No se pudo escribir {name} en el diagnóstico: {e}"))
            })
        };

    write_entry(&mut archive, REPORT_NAME, report)?;
    for (name, text) in logs {
        write_entry(&mut archive, &format!("logs/{name}"), text)?;
    }

    archive.finish().map_err(|e| {
        AppError::Internal(format!(
            "No se pudo cerrar el diagnóstico en {dest_path}: {e}"
        ))
    })?;
    Ok(())
}

/// Cuerpo del diagnóstico: datos del entorno (inyectables en tests) + estado
/// real de Firebird + resumen de los logs (que viajan aparte, en el ZIP).
fn report_body(
    health: &DbHealth,
    version: &str,
    platform: &str,
    logs: &[(String, String)],
) -> String {
    let log_bytes: usize = logs.iter().map(|(_, text)| text.len()).sum();
    let log_dir = logging::dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|| "(no disponible)".to_string());

    let mut out = String::new();
    out.push_str("ISALAB — diagnóstico\n");
    out.push_str("=====================\n\n");
    out.push_str(&format!(
        "Generado:       {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    out.push_str(&format!("Versión:        {version}\n"));
    out.push_str(&format!("Plataforma:     {platform}\n"));
    out.push_str(&format!(
        "Esquema:        {} (esperado {})\n",
        health.schema_version,
        migrations::expected_version()
    ));
    out.push_str(&format!(
        "Firebird:       {} · {} · {} ms\n",
        if health.ok { "OK" } else { "CON ERROR" },
        health.message,
        health.latency_ms
    ));
    out.push_str(&format!(
        "Conexión:       modo {} · destino {} (cliente {}{})\n",
        health.mode,
        health.target,
        if health.client_found {
            "encontrado"
        } else {
            "NO encontrado"
        },
        if health.client_path.is_empty() {
            String::new()
        } else {
            format!(": {}", health.client_path)
        }
    ));
    out.push_str(&format!(
        "Logs:           {} archivo(s), {} KB · {}\n",
        logs.len(),
        log_bytes.div_ceil(1024),
        log_dir
    ));
    out.push('\n');

    if !logs.is_empty() {
        out.push_str("Archivos incluidos en logs/:\n");
        for (name, text) in logs {
            out.push_str(&format!("  - {name} ({} KB)\n", text.len().div_ceil(1024)));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    use crate::db::config::DbStatus;

    fn health() -> DbHealth {
        DbHealth {
            ok: false,
            status: DbStatus::ServerUnreachable,
            message: "No se pudo conectar a Firebird".into(),
            mode: "embedded".into(),
            target: "127.0.0.1/3050 · ISALAB".into(),
            host: "127.0.0.1".into(),
            port: 3050,
            database: "isalab".into(),
            user: "sysdba".into(),
            client_found: true,
            client_path: "fbclient.dll".into(),
            schema_version: 0,
            expected_schema_version: 36,
            latency_ms: 0,
        }
    }

    /// Extrae el contenido de una entrada del ZIP.
    fn entry(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> String {
        let mut text = String::new();
        archive
            .by_name(name)
            .unwrap_or_else(|e| panic!("entrada {name}: {e}"))
            .read_to_string(&mut text)
            .expect("leer entrada");
        text
    }

    #[test]
    fn escribe_un_zip_con_el_reporte_y_los_logs() {
        let dir = std::env::temp_dir().join(format!("isalab-diag-{}", uuid::Uuid::new_v4()));
        let dest = dir.join("sub").join("diagnostico.zip");
        let logs = vec![
            ("isalab.log".to_string(), "linea nueva\n".to_string()),
            ("isalab.1.log".to_string(), "linea vieja\n".to_string()),
        ];

        write_diagnostics(&dest.to_string_lossy(), "reporte de prueba", &logs)
            .expect("escribir diagnóstico");

        let mut archive =
            zip::ZipArchive::new(fs::File::open(&dest).expect("abrir ZIP")).expect("leer ZIP");
        assert_eq!(archive.len(), 3, "reporte + dos logs");
        assert_eq!(entry(&mut archive, "diagnostico.txt"), "reporte de prueba");
        assert_eq!(entry(&mut archive, "logs/isalab.log"), "linea nueva\n");
        assert_eq!(entry(&mut archive, "logs/isalab.1.log"), "linea vieja\n");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn el_reporte_incluye_esquema_y_estado_de_firebird() {
        let logs = vec![("isalab.log".to_string(), "x".to_string())];
        let report = report_body(&health(), "1.1.0", "windows x86_64", &logs);
        assert!(
            report.contains("Esquema:        0 (esperado"),
            "reporte: {report}"
        );
        assert!(
            report.contains("Firebird:       CON ERROR"),
            "reporte: {report}"
        );
        assert!(
            report.contains("No se pudo conectar a Firebird"),
            "reporte: {report}"
        );
        assert!(report.contains("modo embedded"), "reporte: {report}");
        assert!(report.contains("- isalab.log"), "reporte: {report}");
    }
}
