use std::path::PathBuf;

use rsfbclient::prelude::*;
use tauri::{AppHandle, Manager, State};

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

/// Copia una foto de placa, frotis o electroforesis a la carpeta de datos de
/// la app (app_data/attachments) y la asocia al resultado. Devuelve el
/// adjunto creado para refrescar la UI.
#[tauri::command]
#[specta::specta]
pub fn attach_result_file(
    state: State<'_, AppState>,
    app: AppHandle,
    result_id: i32,
    source_path: String,
) -> Result<ResultAttachment, AppError> {
    let user = require_vet_or_admin(&state)?;

    let src = PathBuf::from(&source_path);
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

    // Límite de tamaño para no saturar la carpeta de datos.
    let size = std::fs::metadata(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el archivo: {e}")))?
        .len();
    if size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::Validation(
            "El archivo supera el límite de 20 MB".into(),
        ));
    }

    // El nombre mostrado se deriva del archivo en el servidor (nunca del
    // cliente), con saneado y truncado a 255 chars (VARCHAR en la BD).
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

    let attachments_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Sin carpeta de datos: {e}")))?
        .join("attachments");
    std::fs::create_dir_all(&attachments_dir)
        .map_err(|e| AppError::Internal(format!("No se pudo crear attachments: {e}")))?;

    // Nombre único (uuid) para evitar colisiones y rutas no seguras.
    let uuid = uuid::Uuid::new_v4().to_string();
    let dest = attachments_dir.join(format!("{uuid}.{ext}"));
    std::fs::copy(&src, &dest)
        .map_err(|e| AppError::Internal(format!("No se pudo copiar el archivo: {e}")))?;

    let attachment = attachments_repo::insert(
        pooled.conn(),
        result_id,
        &file_name,
        &dest.display().to_string(),
        Some(mime),
    )?;

    // Auditoría de adjunto añadido.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "RESULT_ATTACHMENT_ADDED",
            Some(&format!(
                "Adjunto \"{file_name}\" al resultado {result_id} → {}",
                dest.display()
            )),
        )
        .ok();
    }

    Ok(attachment)
}

/// Elimina un adjunto: borra el archivo de la carpeta de datos y su registro.
#[tauri::command]
#[specta::specta]
pub fn delete_result_attachment(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    let Some(att) = attachments_repo::get(pooled.conn(), id)? else {
        return Err(AppError::NotFound(format!("Adjunto {id} no encontrado")));
    };

    let path = PathBuf::from(&att.file_path);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    attachments_repo::delete(pooled.conn(), id)?;

    // Auditoría de adjunto eliminado.
    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "RESULT_ATTACHMENT_DELETED",
            Some(&format!(
                "Adjunto \"{}\" del resultado {}",
                att.file_name, att.result_id
            )),
        )
        .ok();
    }

    Ok(())
}
