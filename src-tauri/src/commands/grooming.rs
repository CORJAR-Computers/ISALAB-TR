use std::path::PathBuf;

use rsfbclient::prelude::*;
use tauri::{AppHandle, Manager, State};

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

/// Copia una foto (antes/después) o documento a app_data/attachments y la
/// asocia a la visita. Devuelve el adjunto creado para refrescar la UI.
#[tauri::command]
#[specta::specta]
pub fn add_visit_attachment(
    state: State<'_, AppState>,
    app: AppHandle,
    source_path: String,
    input: AddVisitAttachmentInput,
) -> Result<VisitAttachment, AppError> {
    let user = require_groom(&state)?;

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

    let size = std::fs::metadata(&src)
        .map_err(|e| AppError::Internal(format!("No se pudo leer el archivo: {e}")))?
        .len();
    if size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::Validation(
            "El archivo supera el límite de 20 MB".into(),
        ));
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
        return Err(AppError::NotFound(format!(
            "Visita {} no encontrada",
            input.visit_id
        )));
    }

    let attachments_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(format!("Sin carpeta de datos: {e}")))?
        .join("attachments");
    std::fs::create_dir_all(&attachments_dir)
        .map_err(|e| AppError::Internal(format!("No se pudo crear attachments: {e}")))?;

    let uuid = uuid::Uuid::new_v4().to_string();
    let dest = attachments_dir.join(format!("{uuid}.{ext}"));
    std::fs::copy(&src, &dest)
        .map_err(|e| AppError::Internal(format!("No se pudo copiar el archivo: {e}")))?;

    let attachment = grooming_repo::insert_visit_attachment(
        pooled.conn(),
        input.visit_id,
        &file_name,
        &dest.display().to_string(),
        Some(mime),
        kind,
    )?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_ATTACHMENT_ADDED",
        &format!(
            "Adjunto \"{file_name}\" ({}) a la visita {}",
            input.kind.as_deref().unwrap_or("DOCUMENT"),
            input.visit_id
        ),
    );
    Ok(attachment)
}

/// Elimina un adjunto de visita: borra el archivo y su registro.
#[tauri::command]
#[specta::specta]
pub fn delete_visit_attachment(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let user = require_groom(&state)?;
    let mut pooled = state.pool.acquire()?;

    let Some(att) = grooming_repo::get_visit_attachment(pooled.conn(), id)? else {
        return Err(AppError::NotFound(format!("Adjunto {id} no encontrado")));
    };

    let path = PathBuf::from(&att.file_path);
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    grooming_repo::delete_visit_attachment(pooled.conn(), id)?;

    log_groom_audit(
        &state,
        Some(user.id),
        &user.username,
        "VISIT_ATTACHMENT_DELETED",
        &format!("Adjunto \"{}\" de la visita {}", att.file_name, att.visit_id),
    );
    Ok(())
}
