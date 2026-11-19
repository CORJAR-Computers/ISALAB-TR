use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State};

use rsfbclient::prelude::*;
use crate::db::config::{self, DbConfig, DbMode, DbStatus, DEFAULT_PORT};
use crate::db::{health, migrations};
use crate::error::AppError;
use crate::repositories::settings as settings_repo;
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

/// Exporta la configuración local no sensible a un archivo JSON (sin secretos,
/// sin certificados). Se usa para llevar la configuración de un PC a otro o
/// como respaldo documental de los ajustes de la clínica.
#[tauri::command]
#[specta::specta]
pub fn export_local_config(
    state: State<'_, AppState>,
    dest_path: String,
) -> Result<String, AppError> {
    crate::auth::require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let settings = settings_repo::get(pooled.conn())?;

    // Construye un JSON solo con campos públicos (sin secretos ni certificados).
    let export = serde_json::json!({
        "clinic_name": settings.clinic_name,
        "clinic_nit": settings.clinic_nit,
        "address": settings.address,
        "phone": settings.phone,
        "city": settings.city,
        "tax_rate": settings.tax_rate,
        "currency": settings.currency,
        "signature_mode": settings.signature_mode,
        "vet_name": settings.vet_name,
        "vet_license": settings.vet_license,
        "smtp_host": settings.smtp_host,
        "smtp_port": settings.smtp_port,
        "smtp_tls": settings.smtp_tls,
        "smtp_username": settings.smtp_username,
        "smtp_from": settings.smtp_from,
        "tagline": settings.tagline,
        "grooming_default_duration": settings.grooming_default_duration,
        "reception_ticket_prefix": settings.reception_ticket_prefix,
        "logo": settings.logo,
        "certificate": settings.certificate,
    });

    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| AppError::Internal(format!("No se pudo serializar la configuración: {e}")))?;

    std::fs::write(&dest_path, json)
        .map_err(|e| AppError::Internal(format!("No se pudo escribir el archivo: {e}")))?;

    Ok(dest_path)
}

/// Estado del último backup del servidor Firebird.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub last_backup_at: Option<String>,
    pub last_backup_file: Option<String>,
}

/// Devuelve el estado del último backup del servidor Firebird.
/// Lee las claves `ops.last_backup_at` y `ops.last_backup_file` de la tabla OPS.
#[tauri::command]
#[specta::specta]
pub fn get_server_backup_status(
    state: State<'_, AppState>,
) -> Result<BackupStatus, AppError> {
    let mut pooled = state.pool.acquire()?;
    let conn = pooled.conn();

    let last_backup_at: Option<String> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM OPS WHERE KEY_NAME = 'ops.last_backup_at'",
            (),
        )
        .map_err(AppError::from)?
        .and_then(|(v,)| v);

    let last_backup_file: Option<String> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM OPS WHERE KEY_NAME = 'ops.last_backup_file'",
            (),
        )
        .map_err(AppError::from)?
        .and_then(|(v,)| v);

    Ok(BackupStatus {
        last_backup_at,
        last_backup_file,
    })
}
