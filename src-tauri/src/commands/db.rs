use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State};

use crate::db::config::{self, DbConfig, DbMode, DbStatus, DEFAULT_PORT};
use crate::db::{health, migrations};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DbHealth {
    /// `true` solo cuando la conexión y el esquema son correctos.
    pub ok: bool,
    pub status: DbStatus,
    pub message: String,
    /// `embedded` | `remote`.
    pub mode: String,
    /// Destino legible: `192.168.1.10/3050 · ISALAB`.
    pub target: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub client_found: bool,
    pub client_path: String,
    pub schema_version: i32,
    pub expected_schema_version: i32,
    pub latency_ms: u32,
}

/// Estado real de la conexión Firebird (ping con timeout).
///
/// No se limita a leer el arranque: si el servidor se cayó o volvió, este
/// comando lo refleja y limpia el pool para que la app se recupere sola.
#[tauri::command]
#[specta::specta]
pub fn db_health(state: State<'_, AppState>) -> DbHealth {
    let cfg = &state.db;
    let ping = health::ping(cfg);

    // Un fallo deja el pool marcado: las conexiones ociosas se descartan y la
    // próxima operación abre una nueva contra el servidor ya recuperado.
    if !ping.status.is_ok() {
        state.pool.invalidate();
    }

    let schema_version = if ping.schema_version > 0 {
        ping.schema_version
    } else {
        state.schema_version
    };

    DbHealth {
        ok: ping.status.is_ok(),
        status: ping.status,
        message: ping.message,
        mode: cfg.mode.as_str().to_string(),
        target: cfg.target(),
        host: cfg.host.clone(),
        port: cfg.port,
        database: cfg.database.clone(),
        user: cfg.user.clone(),
        client_found: cfg.fbclient.exists(),
        client_path: cfg.fbclient.display().to_string(),
        schema_version,
        expected_schema_version: migrations::expected_version(),
        latency_ms: ping.latency_ms,
    }
}

/// Datos de conexión que se muestran en el diálogo «Configurar servidor».
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DbConnectionInfo {
    pub mode: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub has_password: bool,
    pub target: String,
    /// `environment` | `file` | `default`.
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveDbConnectionInput {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// Vacío = conservar la contraseña ya guardada.
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DbTestResult {
    pub status: DbStatus,
    pub message: String,
    pub latency_ms: u32,
    pub schema_version: i32,
}

#[tauri::command]
#[specta::specta]
pub fn get_db_connection(state: State<'_, AppState>) -> DbConnectionInfo {
    let cfg = &state.db;
    let env = config::EnvOverrides::from_env();
    let has_env = env.host.is_some() || env.mode.is_some();

    let source = if has_env {
        "environment"
    } else if cfg.is_remote() {
        "file"
    } else {
        "default"
    };

    DbConnectionInfo {
        mode: cfg.mode.as_str().to_string(),
        host: cfg.host.clone(),
        port: cfg.port,
        database: cfg.database.clone(),
        user: cfg.user.clone(),
        has_password: !cfg.password.is_empty(),
        target: cfg.target(),
        source: source.to_string(),
    }
}

/// Prueba una conexión sin guardarla.
///
/// Si la contraseña viene vacía se usa la actual: así «Probar» funciona al
/// abrir el diálogo sin volver a escribirla.
#[tauri::command]
#[specta::specta]
pub fn test_db_connection(
    state: State<'_, AppState>,
    input: SaveDbConnectionInput,
) -> DbTestResult {
    let cfg = connection_from_input(&input, &state.db);
    let ping = health::ping(&cfg);
    DbTestResult {
        status: ping.status,
        message: ping.message,
        latency_ms: ping.latency_ms,
        schema_version: ping.schema_version,
    }
}

/// Guarda la conexión al servidor en `db-connection.json`.
///
/// No exige sesión a propósito: si la base no está configurada o no responde
/// no se puede iniciar sesión, y sin esto el equipo quedaría bloqueado. Solo
/// escribe un archivo local del propio PC (la contraseña va cifrada con DPAPI).
/// El cambio se aplica al reiniciar la aplicación.
#[tauri::command]
#[specta::specta]
pub fn save_db_connection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: SaveDbConnectionInput,
) -> Result<String, AppError> {
    let host = input.host.trim();
    if host.is_empty() {
        return Err(AppError::Validation(
            "Indique la IP o el nombre del servidor Firebird".into(),
        ));
    }
    if input.user.trim().is_empty() {
        return Err(AppError::Validation(
            "Indique el usuario de la base de datos".into(),
        ));
    }

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let cfg = connection_from_input(&input, &state.db);
    config::save_file(&app_data, &cfg)?;
    Ok(cfg.target())
}

/// Construye una configuración remota a partir de los datos del diálogo.
fn connection_from_input(input: &SaveDbConnectionInput, current: &DbConfig) -> DbConfig {
    let password = if input.password.is_empty() {
        current.password.clone()
    } else {
        input.password.clone()
    };

    DbConfig {
        mode: DbMode::Remote,
        host: input.host.trim().to_string(),
        port: if input.port == 0 {
            DEFAULT_PORT
        } else {
            input.port
        },
        database: if input.database.trim().is_empty() {
            config::DEFAULT_DATABASE.to_string()
        } else {
            input.database.trim().to_string()
        },
        user: if input.user.trim().is_empty() {
            config::DEFAULT_USER.to_string()
        } else {
            input.user.trim().to_string()
        },
        password,
        ..current.clone()
    }
}

#[tauri::command]
#[specta::specta]
pub fn create_local_backup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    dest_path: String,
) -> Result<String, AppError> {
    // Exige sesión de ADMIN: el backup recorre toda la carpeta de datos de la
    // app (configuración local, clave de IA cifrada con DPAPI, certificado
    // PKCS#12) y no debe ser invocable sin autenticación.
    crate::auth::require_admin(&state)?;
    use std::fs::File;
    use std::io::{Read, Write};
    use walkdir::WalkDir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let file = File::create(&dest_path)
        .map_err(|e| AppError::Internal(format!("Error creando archivo zip: {}", e)))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let it = WalkDir::new(&app_data_dir)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(&app_data_dir).map_err(|e| {
            AppError::Internal(format!("Error procesando ruta {}: {}", path.display(), e))
        })?;
        let name_str = name.to_string_lossy().replace("\\", "/");

        if path.is_file() {
            if name_str.ends_with(".lock") {
                continue; // Saltar locks temporales
            }
            zip.start_file(name_str, options)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let mut f = File::open(path).map_err(|e| AppError::Internal(e.to_string()))?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            zip.write_all(&buffer)
                .map_err(|e| AppError::Internal(e.to_string()))?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name_str, options)
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    zip.finish()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(dest_path)
}
