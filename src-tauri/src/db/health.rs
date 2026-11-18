//! Diagnóstico de la conexión Firebird.
//!
//! `ping` abre una conexión nueva (respetando el timeout configurado), ejecuta
//! una consulta trivial, lee la versión del esquema y clasifica el resultado en
//! un [`DbStatus`] accionable para la UI. No usa el pool a propósito: el
//! diagnóstico debe funcionar incluso si el pool está vacío o con conexiones
//! muertas.

use std::time::Instant;

use rsfbclient::prelude::*;

use super::config::{DbConfig, DbStatus};
use super::migrations;
use crate::error::AppError;

/// Resultado de un ping de salud.
#[derive(Debug, Clone)]
pub struct DbPing {
    pub status: DbStatus,
    pub message: String,
    pub schema_version: i32,
    pub latency_ms: u32,
}

impl DbPing {
    fn failed(status: DbStatus, message: String, latency_ms: u32) -> Self {
        Self {
            status,
            message,
            schema_version: 0,
            latency_ms,
        }
    }
}

/// Compara la versión de esquema encontrada con la esperada por la app.
pub fn schema_status(found: i32, expected: i32) -> DbStatus {
    match found.cmp(&expected) {
        std::cmp::Ordering::Less => DbStatus::SchemaTooOld,
        std::cmp::Ordering::Greater => DbStatus::SchemaTooNew,
        std::cmp::Ordering::Equal => DbStatus::Connected,
    }
}

/// Clasifica un error de conexión/consulta por su texto.
///
/// `rsfbclient` no expone un enum de errores de red, así que se inspeccionan
/// los mensajes de Firebird/OS. Los marcadores de autenticación se evalúan
/// antes que los de red porque un fallo de login también menciona "connect".
pub fn classify(message: &str) -> DbStatus {
    let msg = message.to_lowercase();

    const AUTH: &[&str] = &[
        "password",
        "your login",
        "user name",
        "username",
        "login failed",
        "not defined",
        "no tiene permiso",
        "insufficient",
        "authentication",
        "credenciales",
        "srp",
    ];
    if AUTH.iter().any(|needle| msg.contains(needle)) {
        return DbStatus::AuthFailed;
    }

    DbStatus::ServerUnreachable
}

/// Ejecuta un ping completo contra la configuración dada.
pub fn ping(cfg: &DbConfig) -> DbPing {
    let started = Instant::now();
    let elapsed = |t: Instant| -> u32 { t.elapsed().as_millis().min(u32::MAX as u128) as u32 };

    if !cfg.fbclient.exists() {
        return DbPing::failed(
            DbStatus::ClientMissing,
            format!(
                "No se encontró el cliente Firebird (fbclient.dll) en {}.",
                cfg.fbclient.display()
            ),
            elapsed(started),
        );
    }

    if !cfg.is_configured() {
        return DbPing::failed(
            DbStatus::NotConfigured,
            "Falta configurar el servidor Firebird (host y usuario).".into(),
            elapsed(started),
        );
    }

    let mut conn = match super::connect(cfg) {
        Ok(conn) => conn,
        Err(e) => {
            let message = e.to_string();
            let status = classify(&message);
            let detail = match status {
                DbStatus::AuthFailed => format!(
                    "El servidor rechazó las credenciales de «{}». Revise usuario y contraseña.",
                    cfg.user
                ),
                _ => format!(
                    "No hay conexión con el servidor {}. Verifique que esté encendido, que la IP sea correcta y que el puerto {} esté abierto.",
                    cfg.target(),
                    cfg.port
                ),
            };
            return DbPing::failed(status, format!("{detail} ({message})"), elapsed(started));
        }
    };

    let probe: Result<Option<(i16,)>, _> = conn.query_first("SELECT 1 FROM RDB$DATABASE", ());
    if let Err(e) = probe {
        let message = e.to_string();
        return DbPing::failed(
            classify(&message),
            format!("La conexión se abrió pero no respondió: {message}"),
            elapsed(started),
        );
    }

    let expected = migrations::expected_version();
    let found = match schema_version_of(&mut conn) {
        Ok(version) => version,
        Err(e) => {
            let message = e.to_string();
            return DbPing::failed(
                classify(&message),
                format!("No se pudo leer la versión del esquema: {message}"),
                elapsed(started),
            );
        }
    };

    let latency = elapsed(started);
    let status = schema_status(found, expected);
    let message = match status {
        DbStatus::SchemaTooOld => format!(
            "La base de datos está en el esquema v{found} y esta app requiere v{expected}. Ejecute el actualizador de esquema (runner) en el servidor."
        ),
        DbStatus::SchemaTooNew => format!(
            "La base de datos usa el esquema v{found}, más nuevo que el que entiende esta app (v{expected}). Actualice la aplicación."
        ),
        _ => "Firebird 5 operativo".to_string(),
    };

    DbPing {
        status,
        message,
        schema_version: found,
        latency_ms: latency,
    }
}

/// Versión actual del esquema, o 0 si la tabla de migraciones no existe.
///
/// Solo cuentan las migraciones terminadas: una ejecución interrumpida deja la
/// fila en `RUNNING` y la app debe seguir viendo la versión anterior.
pub fn schema_version_of(conn: &mut rsfbclient::SimpleConnection) -> Result<i32, AppError> {
    migrations::applied_version(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_an_old_schema() {
        assert_eq!(schema_status(32, 33), DbStatus::SchemaTooOld);
    }

    #[test]
    fn detects_a_newer_schema() {
        assert_eq!(schema_status(34, 33), DbStatus::SchemaTooNew);
    }

    #[test]
    fn accepts_the_expected_schema() {
        assert_eq!(schema_status(33, 33), DbStatus::Connected);
    }

    #[test]
    fn classifies_authentication_errors() {
        assert_eq!(
            classify("Your user name and password are not defined. Ask your database administrator to set up a Firebird login."),
            DbStatus::AuthFailed
        );
        assert_eq!(
            classify("SQL error -902: bad password"),
            DbStatus::AuthFailed
        );
    }

    #[test]
    fn classifies_network_errors() {
        assert_eq!(
            classify("Unable to complete network request to host \"192.168.1.10\"."),
            DbStatus::ServerUnreachable
        );
        assert_eq!(
            classify("unavailable database"),
            DbStatus::ServerUnreachable
        );
        assert_eq!(
            classify("io error: connection refused"),
            DbStatus::ServerUnreachable
        );
    }

    #[test]
    fn ping_reports_missing_configuration() {
        let cfg = DbConfig {
            mode: super::super::config::DbMode::Remote,
            host: String::new(),
            port: 3050,
            database: "ISALAB".into(),
            user: "ISALAB_APP".into(),
            password: String::new(),
            connect_timeout: std::time::Duration::from_millis(500),
            fbclient: std::path::PathBuf::from("no-existe/fbclient.dll"),
            embedded_path: std::path::PathBuf::from("isalab.fdb"),
        };
        let ping = ping(&cfg);
        assert_eq!(ping.status, DbStatus::ClientMissing);
    }
}
