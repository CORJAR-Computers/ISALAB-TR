use rsfbclient::prelude::*;
use tauri::State;

use crate::auth::require_vet_or_admin;
use crate::error::AppError;
use crate::models::sample::ResultAttachment;
use crate::repositories::attachments as attachments_repo;
use crate::repositories::auth as auth_repo;
use crate::state::AppState;

/// Tamaño máximo de un adjunto (20 MB) para no saturar la carpeta de datos.
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;

fn mime_for_ext(ext: &str) -> Option<String> {
    Some(match ext {
        "png" => "image/png".into(),
        "jpg" | "jpeg" => "image/jpeg".into(),
        "webp" => "image/webp".into(),
        "gif" => "image/gif".into(),
        _ => return None,
    })
}

/// Lee un archivo adjunto y lo guarda como BLOB en Firebird. Los tres PCs de
/// la clínica ven el mismo archivo (no depende de carpetas compartidas).
#[tauri::command]
#[specta::specta]
pub fn attach_result_file(
    state: State<'_, AppState>,
    result_id: i32,
    source_path: String,
) -> Result<ResultAttachment, AppError> {
    let user = require_vet_or_admin(&state)?;

    let src = std::path::PathBuf::from(&source_path);
    if !src.exists() {
        return Err(AppError::Validation(
            "El archivo seleccionado ya no existe".into(),
        ));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let Some(mime) = mime_for_ext(&ext) else {
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa PNG, JPG, WebP o GIF."
        )));
    };

    // Lee el contenido una sola vez y lo envía al BLOB de la base (misma
    // transacción que el registro del adjunto).
    let content = std::fs::read(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el archivo: {e}")))?;
    let size = content.len() as u64;
    if size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::Validation(
            "El archivo supera el límite de 20 MB".into(),
        ));
    }

    // El nombre mostrado se deriva del archivo en el servidor, con saneado y
    // truncado a 255 chars (VARCHAR en la BD).
    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "adjunto".to_string());
    let file_name: String = file_name.chars().take(255).collect();

    // El resultado debe existir (error amigable en vez de violación de FK).
    let mut pooled = state.pool.acquire()?;
    let result_exists: Option<(i32,)> = pooled
        .conn()
        .query_first("SELECT 1 FROM LAB_RESULTS WHERE ID = ?", (&result_id,))
        .map_err(AppError::from)?;
    if result_exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Resultado {result_id} no encontrado"
        )));
    }

    let attachment =
        attachments_repo::insert(pooled.conn(), result_id, &file_name, Some(mime), &content)?;

    // Auditoría de adjunto añadido.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "RESULT_ATTACHMENT_ADDED",
            Some(&format!(
                "Adjunto \"{file_name}\" al resultado {result_id} ({size} bytes)"
            )),
        )
        .ok();
    }

    Ok(attachment)
}

/// Elimina un adjunto: borra su registro y el BLOB almacenado en la base.
#[tauri::command]
#[specta::specta]
pub fn delete_result_attachment(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    let Some(att) = attachments_repo::get(pooled.conn(), id)? else {
        return Err(AppError::NotFound(format!("Adjunto {id} no encontrado")));
    };

    attachments_repo::delete(pooled.conn(), id)?;

    // Auditoría de adjunto eliminado.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "RESULT_ATTACHMENT_DELETED",
            Some(&format!(
                "Adjunto \"{}\" del resultado {} ({} bytes)",
                att.file_name, att.result_id, att.size_bytes
            )),
        )
        .ok();
    }

    Ok(())
}

/// Devuelve el contenido (BLOB) de un adjunto como `data:` URL listo para el
/// webview (los tres PCs ven el mismo archivo).
#[tauri::command]
#[specta::specta]
pub fn get_result_attachment_content(
    state: State<'_, AppState>,
    id: i32,
) -> Result<crate::models::attachment::AttachmentContent, AppError> {
    let _ = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    attachments_repo::content(pooled.conn(), id)
        .map_err(|e| AppError::NotFound(format!("Adjunto {id} no disponible: {e}")))
}
