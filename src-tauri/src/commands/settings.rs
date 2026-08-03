use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::auth::{require_admin, require_session};
use crate::error::AppError;
use crate::models::settings::ClinicSettings;
use crate::repositories::auth as auth_repo;
use crate::repositories::settings as settings_repo;
use crate::state::AppState;

/// Extensiones de imagen aceptadas para el logo de la clínica.
const LOGO_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Configuración de la clínica (nombre, NIT, IVA, firma de reportes…).
#[tauri::command]
#[specta::specta]
pub fn get_clinic_settings(
    state: State<'_, AppState>,
) -> Result<ClinicSettings, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    settings_repo::get(pooled.conn())
}

#[tauri::command]
#[specta::specta]
pub fn save_clinic_settings(
    state: State<'_, AppState>,
    input: ClinicSettings,
) -> Result<ClinicSettings, AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let result = settings_repo::save(pooled.conn(), &input)?;

    // Auditoría de cambio de configuración.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "SETTINGS_CHANGED",
            Some(&format!("Clínica: {}", input.clinic_name)),
        ).ok();
    }

    Ok(result)
}

/// Copia el logo seleccionado a la carpeta de datos de la app y devuelve su
/// ruta. De este modo el logo persiste aunque se mueva/borre el original.
#[tauri::command]
#[specta::specta]
pub fn import_clinic_logo(
    state: State<'_, AppState>,
    app: AppHandle,
    source_path: String,
) -> Result<String, AppError> {
    let admin = require_admin(&state)?;
    let src = PathBuf::from(&source_path);
    if !src.exists() {
        return Err(AppError::Validation(
            "El archivo de logo seleccionado ya no existe".into(),
        ));
    }

    // Valida extensión de imagen conocida.
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !LOGO_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "Formato no soportado (.{ext}). Usa PNG, JPG o WebP."
        )));
    }

    let assets_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Sin carpeta de datos: {e}")))?
        .join("assets");
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| AppError::Internal(format!("No se pudo crear assets: {e}")))?;

    let dest = assets_dir.join(format!("clinic-logo.{ext}"));
    std::fs::copy(&src, &dest).map_err(|e| {
        AppError::Internal(format!("No se pudo copiar el logo: {e}"))
    })?;

    // Auditoría de importación de logo.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "LOGO_IMPORTED",
            Some(&format!("Logo: {}", dest.display())),
        ).ok();
    }

    Ok(dest.display().to_string())
}
