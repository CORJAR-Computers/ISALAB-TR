use std::path::Path;

use tauri::State;

use crate::auth::{require_admin, require_session};
use crate::error::AppError;
use crate::models::settings::{ClinicBranding, ClinicSettings};
use crate::pdf_templates::validate_pkcs12;
use crate::repositories::auth as auth_repo;
use crate::repositories::clinic_assets as clinic_assets_repo;
use crate::repositories::logos as logos_repo;
use crate::repositories::settings as settings_repo;
use crate::state::AppState;

/// Extensiones de imagen aceptadas para el logo de la clínica.
const LOGO_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
/// Extensiones aceptadas para el certificado digital PKCS#12.
const PKCS12_EXTENSIONS: [&str; 2] = ["p12", "pfx"];

/// Marcador que se envía al frontend para indicar que un secreto existe pero
/// no se expone en claro. Cuando el admin guarda sin editar el campo, se
/// detecta este marcador y se conserva el valor ya almacenado (cifrado).
pub(crate) const REDACTED_SECRET: &str = "••••••••";

/// Valida que `path` exista y tenga una extensión PKCS#12 aceptada (.p12/.pfx).
/// Devuelve la extensión en minúsculas si es válida, o un `AppError` de
/// validación descriptivo en caso contrario.
#[allow(dead_code)]
fn validate_pkcs12_source(path: &Path) -> Result<String, AppError> {
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
        // Sin extensión se muestra "()" (vacío); con extensión, "(.ext)".
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa un archivo PKCS#12 (.p12 o .pfx)."
        )));
    }
    Ok(ext)
}

/// Datos de marca públicos de la clínica (nombre comercial y tagline).
///
/// No exige sesión: el login los usa para mostrar «Clínica Veterinaria» (o el
/// tagline configurado) antes de autenticar. No expone ningún secreto.
#[tauri::command]
#[specta::specta]
pub fn get_clinic_branding(state: State<'_, AppState>) -> Result<ClinicBranding, AppError> {
    let mut pooled = state.pool.acquire()?;
    settings_repo::get_branding(pooled.conn())
}

/// Configuración de la clínica (nombre, NIT, IVA, firma de reportes…).
///
/// Los secretos (`groq_api_key`, `pkcs12_password`) se devuelven siempre
/// redactados: nunca cruzan el IPC en claro. Además, para sesiones que no
/// son ADMIN también se ocultan `pkcs12_path` (defensa en profundidad, ya
/// que la UI oculta el formulario de configuración para no-admins).
#[tauri::command]
#[specta::specta]
pub fn get_clinic_settings(state: State<'_, AppState>) -> Result<ClinicSettings, AppError> {
    let session = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let mut settings = settings_repo::get(pooled.conn())?;

    // Los secretos nunca se exponen al frontend, ni siquiera para el admin.
    // El admin los re-ingresa desde el formulario; el campo vacío significa
    // "conservar el valor actual" (ver `save_clinic_settings`).
    settings.groq_api_key = settings
        .groq_api_key
        .as_ref()
        .map(|_| REDACTED_SECRET.to_string());
    settings.smtp_password = settings
        .smtp_password
        .as_ref()
        .map(|_| REDACTED_SECRET.to_string());
    settings.pkcs12_password = None;

    // Para sesiones que no son ADMIN se oculta el certificado (defensa en
    // profundidad; la UI oculta el formulario de configuración para no-admins).
    if session.role != "ADMIN" {
        settings.certificate = None;
    }

    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub fn save_clinic_settings(
    state: State<'_, AppState>,
    mut input: ClinicSettings,
) -> Result<ClinicSettings, AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    // Si el admin reenvía el marcador de secreto sin editar la clave de IA,
    // conservamos el valor ya almacenado (cifrado) en vez de sobrescribirlo
    // con el marcador.
    if input.groq_api_key.as_deref() == Some(REDACTED_SECRET) {
        let current = settings_repo::get(pooled.conn())?;
        input.groq_api_key = current.groq_api_key;
    }

    // Igual para la contraseña SMTP: el marcador significa "no la edité",
    // así que se conserva el valor cifrado ya almacenado.
    if input.smtp_password.as_deref() == Some(REDACTED_SECRET) {
        let current = settings_repo::get(pooled.conn())?;
        input.smtp_password = current.smtp_password;
    }

    // Si llega una contraseña PKCS#12, se guarda SOLO en memoria (nunca se
    // persiste). El certificado se importa con `import_pkcs12`.
    if let Some(ref pwd) = input.pkcs12_password {
        if !pwd.is_empty() {
            *state
                .pkcs12_password
                .lock()
                .map_err(|_| AppError::Internal("Certificado bloqueado".into()))? =
                Some(pwd.clone());
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
        )
        .ok();
    }

    Ok(result)
}

/// Lee el archivo de logo y lo almacena como BLOB en `CLINIC_ASSETS` (KIND =
/// LOGO). Los tres PCs ven el mismo logo (no depende de carpetas compartidas).
#[tauri::command]
#[specta::specta]
pub fn import_clinic_logo(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<crate::models::settings::ClinicAsset, AppError> {
    let admin = require_admin(&state)?;
    let src = std::path::PathBuf::from(&source_path);
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
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa PNG, JPG o WebP."
        )));
    }

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    let content = std::fs::read(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el logo: {e}")))?;

    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| "clinic-logo".to_string());

    let mut pooled = state.pool.acquire()?;
    let asset = clinic_assets_repo::upsert(
        pooled.conn(),
        "LOGO",
        Some(&file_name),
        Some(mime),
        &content,
    )?;

    // Auditoría de importación de logo.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "LOGO_IMPORTED",
            Some(&format!("Logo: {} ({} bytes)", file_name, content.len())),
        )
        .ok();
    }

    Ok(asset)
}

/// Elimina el logo principal de `CLINIC_ASSETS`.
#[tauri::command]
#[specta::specta]
pub fn delete_clinic_logo(state: State<'_, AppState>) -> Result<(), AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    clinic_assets_repo::delete(pooled.conn(), "LOGO")?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "LOGO_DELETED",
            Some("Logo principal eliminado".into()),
        )
        .ok();
    }
    Ok(())
}

/// Devuelve el contenido (BLOB) del logo principal como `data:` URL.
#[tauri::command]
#[specta::specta]
pub fn get_clinic_logo_content(
    state: State<'_, AppState>,
) -> Result<Option<Vec<u8>>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    Ok(clinic_assets_repo::content(pooled.conn(), "LOGO"))
}

#[tauri::command]
#[specta::specta]
pub fn list_secondary_logos(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::logo::SecondaryLogo>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    logos_repo::list(pooled.conn())
}

#[tauri::command]
#[specta::specta]
pub fn import_secondary_logo(
    state: State<'_, AppState>,
    name: String,
    source_path: String,
) -> Result<crate::models::logo::SecondaryLogo, AppError> {
    let admin = require_admin(&state)?;
    let src = std::path::PathBuf::from(&source_path);
    if !src.exists() {
        return Err(AppError::Validation(
            "El archivo de logo seleccionado ya no existe".into(),
        ));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !LOGO_EXTENSIONS.contains(&ext.as_str()) {
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa PNG, JPG o WebP."
        )));
    }

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    let content = std::fs::read(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el logo: {e}")))?;

    let mut pooled = state.pool.acquire()?;
    let logo = logos_repo::insert(pooled.conn(), &name, Some(mime), &content)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "SECONDARY_LOGO_IMPORTED",
            Some(&format!("Logo: {} ({} bytes)", name, content.len())),
        )
        .ok();
    }

    Ok(logo)
}

#[tauri::command]
#[specta::specta]
pub fn delete_secondary_logo(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    if let Some(logo) = logos_repo::get(pooled.conn(), id)? {
        logos_repo::delete(pooled.conn(), id)?;

        if let Ok(mut audit_conn) = state.pool.acquire() {
            auth_repo::log_audit(
                audit_conn.conn(),
                Some(admin.id),
                &admin.username,
                "SECONDARY_LOGO_DELETED",
                Some(&format!("Logo: {} ({})", logo.name, logo.id)),
            )
            .ok();
        }
    }

    Ok(())
}

/// Devuelve el contenido (BLOB) de un logo secundario como `data:` URL.
#[tauri::command]
#[specta::specta]
pub fn get_secondary_logo_content(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<Vec<u8>>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    Ok(logos_repo::content(pooled.conn(), id))
}

/// Importa un certificado PKCS#12 (.p12/.pfx) como BLOB en `CLINIC_ASSETS`
/// (KIND = CERT_P12). Valida el contenedor y guarda la contraseña solo en
/// memoria (state).
#[tauri::command]
#[specta::specta]
pub fn import_pkcs12(
    state: State<'_, AppState>,
    source_path: String,
    password: String,
) -> Result<crate::models::settings::ClinicAsset, AppError> {
    let admin = require_admin(&state)?;
    let src = std::path::PathBuf::from(&source_path);
    if !src.exists() {
        return Err(AppError::Validation(
            "El archivo de certificado seleccionado ya no existe".into(),
        ));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !PKCS12_EXTENSIONS.contains(&ext.as_str()) {
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa .p12 o .pfx."
        )));
    }

    let content = std::fs::read(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el certificado: {e}")))?;

    // Valida que sea un PKCS#12 válido con la contraseña dada (materializa
    // en %TEMP% porque validate_pkcs12 lee desde ruta de archivo).
    let tmp_path = std::env::temp_dir().join(format!("isalab_pkcs12_{}.tmp", admin.id));
    std::fs::write(&tmp_path, &content)
        .map_err(|e| AppError::Internal(format!("No se pudo escribir el temporal: {e}")))?;
    let validation = validate_pkcs12(&tmp_path, &password)
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            e
        })?;
    let _ = std::fs::remove_file(&tmp_path);

    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|| "certificado.p12".to_string());

    let mime = "application/x-pkcs12";

    let mut pooled = state.pool.acquire()?;
    let asset = clinic_assets_repo::upsert(
        pooled.conn(),
        "CERT_P12",
        Some(&file_name),
        Some(mime),
        &content,
    )?;

    // Guarda la contraseña solo en memoria (nunca en BD).
    *state
        .pkcs12_password
        .lock()
        .map_err(|_| AppError::Internal("Certificado bloqueado".into()))? =
        Some(password);

    // Auditoría.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "PKCS12_IMPORTED",
            Some(&format!(
                "Certificado: {} (titular: {})",
                file_name, validation.info.holder_name
            )),
        )
        .ok();
    }

    Ok(asset)
}

/// Elimina el certificado PKCS#12 de `CLINIC_ASSETS` y de la memoria.
#[tauri::command]
#[specta::specta]
pub fn delete_pkcs12(state: State<'_, AppState>) -> Result<(), AppError> {
    let admin = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    clinic_assets_repo::delete(pooled.conn(), "CERT_P12")?;

    *state
        .pkcs12_password
        .lock()
        .map_err(|_| AppError::Internal("Certificado bloqueado".into()))? = None;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(admin.id),
            &admin.username,
            "PKCS12_DELETED",
            Some("Certificado PKCS#12 eliminado".into()),
        )
        .ok();
    }
    Ok(())
}

/// Devuelve el contenido (BLOB) del certificado PKCS#12 para firmar.
#[tauri::command]
#[specta::specta]
pub fn get_pkcs12_content(state: State<'_, AppState>) -> Result<Option<Vec<u8>>, AppError> {
    let _ = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    Ok(clinic_assets_repo::content(pooled.conn(), "CERT_P12"))
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
