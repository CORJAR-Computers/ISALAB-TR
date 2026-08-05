use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::auth::{require_admin, require_session};
use crate::error::AppError;
use crate::models::settings::ClinicSettings;
use crate::pdf_templates::validate_pkcs12;
use crate::repositories::auth as auth_repo;
use crate::repositories::settings as settings_repo;
use crate::state::AppState;

/// Extensiones de imagen aceptadas para el logo de la clínica.
const LOGO_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
/// Extensiones aceptadas para el certificado digital PKCS#12.
const PKCS12_EXTENSIONS: [&str; 2] = ["p12", "pfx"];

/// Valida que `path` exista y tenga una extensión PKCS#12 aceptada (.p12/.pfx).
/// Devuelve la extensión en minúsculas si es válida, o un `AppError` de
/// validación descriptivo en caso contrario.
fn validate_pkcs12_source(path: &PathBuf) -> Result<String, AppError> {
    if !path.exists() {
        return Err(AppError::Validation(
            "El archivo de certificado seleccionado ya no existe".into(),
        ));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !PKCS12_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!(
            "Formato no soportado (.{ext}). Usa un archivo PKCS#12 (.p12 o .pfx)."
        )));
    }
    Ok(ext)
}

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

    // Si llega una contraseña PKCS#12, se valida contra el certificado
    // configurado y se guarda SOLO en memoria (nunca se persiste).
    if let Some(ref pwd) = input.pkcs12_password {
        if !pwd.is_empty() {
            if let Some(ref p12) = input.pkcs12_path {
                crate::pdf_templates::validate_pkcs12(
                    std::path::Path::new(p12),
                    pwd,
                )?;
                *state.pkcs12_password.lock().unwrap() = Some(pwd.clone());
            } else {
                return Err(AppError::Validation(
                    "Se indicó una contraseña PKCS#12 pero no hay certificado configurado. Importa el certificado primero.".into(),
                ));
            }
        }
    }

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

/// Valida y copia un certificado digital PKCS#12 (.p12/.pfx) a la carpeta de
/// datos de la app. Devuelve la ruta persistida para guardarla en la
/// configuración. La contraseña solo se usa para la validación y NUNCA se
/// guarda en la base de datos (se pide de nuevo al firmar).
#[tauri::command]
#[specta::specta]
pub fn import_pkcs12(
    state: State<'_, AppState>,
    app: AppHandle,
    source_path: String,
    password: String,
) -> Result<String, AppError> {
    let admin = require_admin(&state)?;
    let src = PathBuf::from(&source_path);
    let ext = validate_pkcs12_source(&src)?;

    // Validación real: descifra el PKCS#12 con la contraseña y extrae metadatos.
    validate_pkcs12(&src, &password)?;
    // La contraseña queda en memoria para poder firmar reportes; nunca en BD.
    *state.pkcs12_password.lock().unwrap() = Some(password);

    let assets_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Sin carpeta de datos: {e}")))?
        .join("assets");
    std::fs::create_dir_all(&assets_dir)
        .map_err(|e| AppError::Internal(format!("No se pudo crear assets: {e}")))?;

    let dest = assets_dir.join(format!("clinic-certificate.{ext}"));

    // Elimina certificados previos con otra extensión (.p12/.pfx) para no
    // dejar archivos huérfanos en la carpeta de datos.
    for prev_ext in PKCS12_EXTENSIONS {
        let prev = assets_dir.join(format!("clinic-certificate.{prev_ext}"));
        if prev.exists() && prev != dest {
            let _ = std::fs::remove_file(&prev);
        }
    }

    std::fs::copy(&src, &dest).map_err(|e| {
        AppError::Internal(format!("No se pudo copiar el certificado: {e}"))
    })?;

    // Auditoría de importación de certificado.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "CERTIFICATE_IMPORTED",
            Some(&format!("PKCS#12: {}", dest.display())),
        )
        .ok();
    }

    Ok(dest.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_validate_pkcs12_source_p12() {
        let path = temp_file("isalab-cert.p12", b"dummy");
        let ext = validate_pkcs12_source(&path).unwrap();
        assert_eq!(ext, "p12");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_source_pfx_uppercase() {
        let path = temp_file("isalab-cert.PFX", b"dummy");
        let ext = validate_pkcs12_source(&path).unwrap();
        assert_eq!(ext, "pfx");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_source_wrong_extension() {
        let path = temp_file("isalab-cert.txt", b"dummy");
        let err = validate_pkcs12_source(&path).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("Formato no soportado"));
        assert!(err.to_string().contains(".txt"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_source_no_extension() {
        let path = temp_file("isalab-cert", b"dummy");
        let err = validate_pkcs12_source(&path).unwrap_err();
        assert!(err.to_string().contains("Formato no soportado"));
        // Sin extensión, el nombre del archivo se muestra vacío entre paréntesis.
        assert!(err.to_string().contains("()."));
        assert!(err.to_string().contains(".p12 o .pfx"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_source_nonexistent() {
        let path = std::env::temp_dir().join("isalab-no-existe.p12");
        let err = validate_pkcs12_source(&path).unwrap_err();
        assert!(err.to_string().contains("ya no existe"));
    }

    #[test]
    fn test_validate_pkcs12_source_directory() {
        // Un directorio no tiene extensión útil y existe; cae en formato no soportado.
        let path = std::env::temp_dir();
        let err = validate_pkcs12_source(&path).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }
}
