//! Envío de correos SMTP (lettre) para notificaciones de valores críticos.
//! Soporta los tres modos TLS comunes: NONE (local), STARTTLS (587) y TLS
//! implícito (465). La configuración vive en `ClinicSettings` (claves smtp.*).

use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::{Message, SmtpTransport, Transport};

use crate::error::AppError;
use crate::models::settings::ClinicSettings;

/// Construye el transporte SMTP según el modo TLS configurado.
fn transport(settings: &ClinicSettings) -> Result<SmtpTransport, AppError> {
    let host = settings
        .smtp_host
        .as_deref()
        .filter(|h| !h.trim().is_empty())
        .ok_or_else(|| AppError::Validation("No hay servidor SMTP configurado".into()))?;
    let port = settings.smtp_port.unwrap_or(587);
    let tls = settings.smtp_tls.as_deref().unwrap_or("STARTTLS");

    // TLS implícito (SMTPS, 465) → relay; STARTTLS (587) → starttls_relay;
    // NONE → builder_dangerous con Tls::None (relay local de confianza).
    let builder = match tls {
        "TLS" => SmtpTransport::relay(host)
            .map_err(|e| AppError::Validation(format!("Servidor SMTP inválido: {e}")))?,
        "NONE" => SmtpTransport::builder_dangerous(host).tls(Tls::None),
        _ => SmtpTransport::starttls_relay(host)
            .map_err(|e| AppError::Validation(format!("Servidor SMTP inválido: {e}")))?,
    }
    .port(port as u16);

    let builder = match (
        settings.smtp_username.as_deref(),
        settings.smtp_password.as_deref(),
    ) {
        (Some(u), Some(p)) if !u.trim().is_empty() && !p.is_empty() => {
            builder.credentials(Credentials::new(u.to_string(), p.to_string()))
        }
        _ => builder,
    };

    Ok(builder.build())
}

/// Envía un correo en texto plano. Devuelve Ok(()) si el servidor lo aceptó.
pub fn send_email(
    settings: &ClinicSettings,
    to_name: &str,
    to_address: &str,
    subject: &str,
    body: &str,
) -> Result<(), AppError> {
    let from = settings
        .smtp_from
        .as_deref()
        .filter(|f| !f.trim().is_empty())
        .ok_or_else(|| AppError::Validation("No hay remitente SMTP configurado".into()))?;

    let from_mb: Mailbox = from
        .parse()
        .map_err(|e| AppError::Validation(format!("Remitente SMTP inválido: {e}")))?;
    let to_mb: Mailbox = format!("{to_name} <{to_address}>")
        .parse()
        .map_err(|e| AppError::Validation(format!("Destinatario inválido: {e}")))?;

    let message = Message::builder()
        .from(from_mb)
        .to(to_mb)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| AppError::Validation(format!("Correo inválido: {e}")))?;

    let mailer = transport(settings)?;
    mailer
        .send(&message)
        .map(|_| ())
        .map_err(|e| AppError::Internal(format!("Fallo SMTP: {e}")))
}

/// Prueba de conexión con el comando NOOP (sin enviar correos).
pub fn test_connection(settings: &ClinicSettings) -> Result<(), AppError> {
    let mailer = transport(settings)?;
    let ok = mailer
        .test_connection()
        .map_err(|e| AppError::Internal(format!("Fallo SMTP: {e}")))?;
    if ok {
        Ok(())
    } else {
        Err(AppError::Internal(
            "El servidor SMTP respondió pero la conexión no está activa".into(),
        ))
    }
}
