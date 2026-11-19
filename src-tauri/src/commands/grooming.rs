use base64::Engine;
use rsfbclient::prelude::*;
use tauri::State;

use crate::auth::{require_groom, require_session};
use crate::error::AppError;
use crate::models::grooming::{
    AddVisitAttachmentInput, FinishGroomingSessionInput, GroomingSession, VisitAttachment,
};
use crate::repositories::auth as auth_repo;
use crate::repositories::grooming as grooming_repo;
use crate::state::AppState;

/// Tamaño máximo de un adjunto (20 MB), igual que los adjuntos de resultados.
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;

fn log_groom_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut pooled) = state.pool.acquire() {
        auth_repo::log_audit(pooled.conn(), user_id, username, action, Some(detail)).ok();
    }
}

/// Ficha de sesión de la línea de servicio, o `None` si aún no existe.
#[tauri::command]
#[specta::specta]
pub fn get_grooming_session(
    state: State<'_, AppState>,
    visit_service_id: i32,
) -> Result<Option<GroomingSession>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    grooming_repo::get_grooming_session_by_service(pooled.conn(), visit_service_id)
}

/// Crea o actualiza la sesión de peluquería de una línea (1 sesión por línea).
#[tauri::command]
#[specta::specta]
pub fn upsert_grooming_session(
    state: State<'_, AppState>,
    input: crate::models::grooming::UpsertGroomingSessionInput,
) -> Result<GroomingSession, AppError> {
    let user = require_groom(&state)?;
    let mut pooled = state.pool.acquire()?;
    let session = grooming_repo::upsert_grooming_session(pooled.conn(), &input)?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "GROOMING_SESSION_SAVE",
        &format!(
            "Línea {} · paciente {} · estilo {}",
            input.visit_service_id,
            session.patient_name,
            session.style.as_deref().unwrap_or("-")
        ),
    );
    Ok(session)
}

/// Cierra la sesión (FINISHED_AT) y la línea de servicio pasa a COMPLETADA;
/// sugiere el próximo turno con NEXT_SUGGESTED_AT.
#[tauri::command]
#[specta::specta]
pub fn finish_grooming_session(
    state: State<'_, AppState>,
    visit_service_id: i32,
    input: FinishGroomingSessionInput,
) -> Result<GroomingSession, AppError> {
    let user = require_groom(&state)?;
    let mut pooled = state.pool.acquire()?;
    let session = grooming_repo::finish_grooming_session(pooled.conn(), visit_service_id, &input)?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "GROOMING_SESSION_FINISH",
        &format!(
            "Línea {visit_service_id} · paciente {} · próximo turno {}",
            session.patient_name,
            session.next_suggested_at.as_deref().unwrap_or("-")
        ),
    );
    Ok(session)
}

/// Tablero del día: líneas de peluquería de las visitas de una fecha.
#[tauri::command]
#[specta::specta]
pub fn list_grooming_day(
    state: State<'_, AppState>,
    date: String,
) -> Result<Vec<crate::models::grooming::GroomingDayLine>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    grooming_repo::list_grooming_day(pooled.conn(), &date)
}

/// Próximos turnos sugeridos por sesiones cerradas (recordatorios).
#[tauri::command]
#[specta::specta]
pub fn list_grooming_reminders(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::grooming::GroomingReminder>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    grooming_repo::list_grooming_reminders(pooled.conn())
}

/// Fotos antes/después (o documentos) de una visita, más recientes primero.
#[tauri::command]
#[specta::specta]
pub fn list_visit_attachments(
    state: State<'_, AppState>,
    visit_id: i32,
) -> Result<Vec<VisitAttachment>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    grooming_repo::list_visit_attachments(pooled.conn(), visit_id)
}

/// Lee un archivo adjunto de visita y lo guarda como BLOB en Firebird.
#[tauri::command]
#[specta::specta]
pub fn add_visit_attachment(
    state: State<'_, AppState>,
    source_path: String,
    input: AddVisitAttachmentInput,
) -> Result<VisitAttachment, AppError> {
    let user = require_groom(&state)?;

    let src = std::path::PathBuf::from(&source_path);
    if !src.exists() {
        return Err(AppError::Validation("El archivo seleccionado ya no existe".into()));
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    // Fotos (BEFORE/AFTER) y documentos de la visita.
    let Some(mime) = (match ext.as_str() {
        "png" => Some("image/png".into()),
        "jpg" | "jpeg" => Some("image/jpeg".into()),
        "webp" => Some("image/webp".into()),
        "gif" => Some("image/gif".into()),
        "pdf" => Some("application/pdf".into()),
        _ => None,
    }) else {
        let ext_label = if ext.is_empty() {
            "()".to_string()
        } else {
            format!("(.{ext})")
        };
        return Err(AppError::Validation(format!(
            "Formato no soportado {ext_label}. Usa PNG, JPG, WebP, GIF o PDF."
        )));
    };

    // Lee el contenido una sola vez y lo envía al BLOB de la base.
    let content = std::fs::read(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el archivo: {e}")))?;
    let size = content.len() as u64;
    if size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::Validation("El archivo supera el límite de 20 MB".into()));
    }

    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "adjunto".to_string());
    let file_name: String = file_name.chars().take(255).collect();

    let kind = input
        .kind
        .as_deref()
        .map(|k| k.trim().to_uppercase())
        .filter(|k| matches!(k.as_str(), "BEFORE" | "AFTER" | "DOCUMENT"));

    let mut pooled = state.pool.acquire()?;

    // La visita debe existir (error amigable en vez de violación de FK).
    let visit_exists: Option<(i32,)> = pooled
        .conn()
        .query_first("SELECT 1 FROM VISITS WHERE ID = ?", (&input.visit_id,))
        .map_err(AppError::from)?;
    if visit_exists.is_none() {
        return Err(AppError::NotFound(format!("Visita {} no encontrada", input.visit_id)));
    }

    let attachment = grooming_repo::insert_visit_attachment(
        pooled.conn(),
        input.visit_id,
        &file_name,
        Some(mime),
        kind,
        &content,
    )?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_ATTACHMENT_ADDED",
        &format!("Adjunto \"{file_name}\" ({size} bytes) a la visita {}", input.visit_id),
    );
    Ok(attachment)
}

/// Elimina un adjunto de visita: borra su registro y el BLOB almacenado en la base.
#[tauri::command]
#[specta::specta]
pub fn delete_visit_attachment(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let user = require_groom(&state)?;
    let mut pooled = state.pool.acquire()?;

    let Some(att) = grooming_repo::get_visit_attachment(pooled.conn(), id)? else {
        return Err(AppError::NotFound(format!("Adjunto {id} no encontrado")));
    };

    grooming_repo::delete_visit_attachment(pooled.conn(), id)?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_ATTACHMENT_DELETED",
        &format!(
            "Adjunto \"{}\" de la visita {} ({} bytes)",
            att.file_name, att.visit_id, att.size_bytes
        ),
    );
    Ok(())
}

/// Devuelve el contenido (BLOB) de un adjunto de visita como `data:` URL.
#[tauri::command]
#[specta::specta]
pub fn get_visit_attachment_content(
    state: State<'_, AppState>,
    id: i32,
) -> Result<crate::models::attachment::AttachmentContent, AppError> {
    let _ = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    grooming_repo::content_visit_attachment(pooled.conn(), id)
        .map_err(|e| AppError::NotFound(format!("Adjunto {id} no disponible: {e}")))
}

/// Abre el archivo adjunto de visita en el visor por defecto del SO
/// (materializa el BLOB en %TEMP% y lo lanza).
#[tauri::command]
#[specta::specta]
pub fn open_visit_attachment(
    state: State<'_, AppState>,
    id: i32,
) -> Result<(), AppError> {
    let _ = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let content = grooming_repo::content_visit_attachment(pooled.conn(), id)
        .map_err(|e| AppError::NotFound(format!("Adjunto {id} no disponible: {e}")))?;

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("isalab_visit_{}_{}", id, content.file_name));

    // data_url es "data:mime;base64,<bytes>"; decodificamos a bytes crudos.
    let b64 = content
        .data_url
        .split(',')
        .nth(1)
        .unwrap_or("");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| AppError::Internal(format!("Base64 inválido: {e}")))?;
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| AppError::Internal(format!("No se pudo escribir el temporal: {e}")))?;

    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &tmp_path.to_string_lossy()])
        .spawn()
        .map_err(|e| AppError::Internal(format!("No se pudo lanzar el visor: {e}")))?;

    // Limpieza best-effort del temporal (se ignora si el visor lo usa aún).
    let _ = std::fs::remove_file(&tmp_path);

    Ok(())
}
