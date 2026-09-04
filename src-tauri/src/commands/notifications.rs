use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;
use tauri::State;

use crate::auth::{require_admin, require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::mail;
use crate::models::notification::NotificationLogEntry;
use crate::repositories::auth as auth_repo;
use crate::repositories::notifications as notifications_repo;
use crate::repositories::settings as settings_repo;
use crate::state::AppState;

/// Historial de notificaciones de una muestra (quién, cuándo, canal, estado).
#[tauri::command]
#[specta::specta]
pub fn list_sample_notifications(
    state: State<'_, AppState>,
    sample_id: i32,
) -> Result<Vec<NotificationLogEntry>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    notifications_repo::list_by_sample(pooled.conn(), sample_id)
}

/// Confirma (acknowledgment) los valores críticos recién registrados: persiste
/// una fila ACKNOWLEDGED por resultado, con usuario y fecha (CLSI GP47).
#[tauri::command]
#[specta::specta]
pub fn acknowledge_critical(
    state: State<'_, AppState>,
    sample_id: i32,
    result_ids: Vec<i32>,
) -> Result<Vec<NotificationLogEntry>, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    if result_ids.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un resultado crítico".into(),
        ));
    }
    let entries =
        notifications_repo::acknowledge(pooled.conn(), sample_id, &result_ids, &user.username)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        auth_repo::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "CRITICAL_ACKNOWLEDGED",
            Some(&format!(
                "Muestra {sample_id} · {} valor(es) crítico(s) confirmado(s)",
                result_ids.len()
            )),
        )
        .ok();
    }

    Ok(entries)
}

/// Envía por email al propietario un aviso de valor(es) crítico(s) y lo
/// registra en NOTIFICATION_LOG (SENT o FAILED según el resultado del envío).
#[tauri::command]
#[specta::specta]
pub fn send_critical_email(
    state: State<'_, AppState>,
    sample_id: i32,
    result_ids: Vec<i32>,
) -> Result<Vec<NotificationLogEntry>, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;

    if result_ids.is_empty() {
        return Err(AppError::Validation(
            "Debes seleccionar al menos un resultado crítico".into(),
        ));
    }

    // Destinatario: propietario de la muestra (requiere email registrado).
    let Some((owner_name, owner_email)) =
        notifications_repo::owner_email_for_sample(pooled.conn(), sample_id)?
    else {
        return Err(AppError::Validation(
            "El propietario no tiene un correo electrónico registrado. \
             Añade el email del propietario para enviar el aviso."
                .into(),
        ));
    };

    let settings = settings_repo::get(pooled.conn())?;
    let patient = notifications_repo::patient_name_for_sample(pooled.conn(), sample_id)?;

    // Detalle de los resultados para el cuerpo del correo.
    let results = result_details(pooled.conn(), &result_ids)?;
    if results.is_empty() {
        return Err(AppError::Validation(
            "Ninguno de los resultados seleccionados existe".into(),
        ));
    }

    let patient_name = patient.unwrap_or_else(|| "paciente".into());
    let subject = format!("⚠ ALERTA: Valor crítico de laboratorio — {patient_name}");
    let mut body = format!(
        "Hola {owner_name},\n\n\
         Le informamos que un resultado de laboratorio de *{patient_name}* \
         presenta un valor crítico que requiere atención inmediata:\n\n"
    );
    for (_, analyte, value, unit, status) in &results {
        let unit = unit.as_deref().unwrap_or("");
        let label = status_label(status);
        body.push_str(&format!("- {analyte}: {value} {unit} ({label})\n"));
    }
    body.push_str(
        "\nPor favor, contacte a su veterinario lo antes posible.\n\n\
         Este es un mensaje automático de ISALAB.\n",
    );

    // Intenta el envío y registra el resultado por cada resultado.
    let mut entries = Vec::with_capacity(results.len());
    match mail::send_email(&settings, &owner_name, &owner_email, &subject, &body) {
        Ok(()) => {
            for (rid, _, _, _, _) in &results {
                entries.push(notifications_repo::log(
                    pooled.conn(),
                    &notifications_repo::NewNotification {
                        result_id: Some(*rid),
                        sample_id,
                        channel: "EMAIL",
                        recipient_name: Some(&owner_name),
                        recipient_address: Some(&owner_email),
                        status: "SENT",
                        note: None,
                    },
                )?);
            }
            if let Ok(mut audit_conn) = state.pool.acquire() {
                auth_repo::log_audit(
                    audit_conn.conn(),
                    Some(user.id),
                    &user.username,
                    "CRITICAL_EMAIL_SENT",
                    Some(&format!(
                        "Muestra {sample_id} · {owner_email} · {} resultado(s)",
                        results.len()
                    )),
                )
                .ok();
            }
        }
        Err(e) => {
            for (rid, _, _, _, _) in &results {
                entries.push(notifications_repo::log(
                    pooled.conn(),
                    &notifications_repo::NewNotification {
                        result_id: Some(*rid),
                        sample_id,
                        channel: "EMAIL",
                        recipient_name: Some(&owner_name),
                        recipient_address: Some(&owner_email),
                        status: "FAILED",
                        note: Some(&e.to_string()),
                    },
                )?);
            }
            return Err(AppError::Internal(format!(
                "No se pudo enviar el correo: {e}"
            )));
        }
    }

    Ok(entries)
}

/// Prueba la conexión SMTP configurada sin enviar correos (solo ADMIN).
#[tauri::command]
#[specta::specta]
pub fn test_smtp_connection(state: State<'_, AppState>) -> Result<(), AppError> {
    require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let settings = settings_repo::get(pooled.conn())?;
    mail::test_connection(&settings)
}

/// (id, nombre, valor, unidad, estado) de un resultado para el cuerpo del correo.
type ResultDetail = (i32, String, String, Option<String>, String);

/// Detalle (id, nombre, valor, unidad, estado) de un lote de resultados.
fn result_details(
    conn: &mut SimpleConnection,
    result_ids: &[i32],
) -> Result<Vec<ResultDetail>, AppError> {
    let mut out = Vec::with_capacity(result_ids.len());
    for rid in result_ids {
        let row: Option<(String, f64, Option<String>, String)> = conn
            .query_first(
                "SELECT FIRST 1 a.NAME, r.RESULT_VALUE, a.UNIT, r.STATUS
                 FROM LAB_RESULTS r
                 JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
                 WHERE r.ID = ?",
                (rid,),
            )
            .map_err(AppError::from)?;
        if let Some((name, value, unit, status)) = row {
            out.push((*rid, name, format!("{value}"), unit, status));
        }
    }
    Ok(out)
}

fn status_label(status: &str) -> &str {
    match status {
        "CRITICO_ALTO" => "CRÍTICO ALTO",
        "CRITICO_BAJO" => "CRÍTICO BAJO",
        "ALTO" => "ALTO",
        "BAJO" => "BAJO",
        _ => status,
    }
}
