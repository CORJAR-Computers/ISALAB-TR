use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Migraciones versionadas, embebidas en el binario.
/// La versión es el índice + 1 (0001 → 1, 0002 → 2…).
pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial_schema",
        include_str!("../../migrations/0001_initial_schema.sql"),
    ),
    (
        "0002_seed_catalog",
        include_str!("../../migrations/0002_seed_catalog.sql"),
    ),
    (
        "0003_surgeries",
        include_str!("../../migrations/0003_surgeries.sql"),
    ),
    (
        "0004_user_audit_log",
        include_str!("../../migrations/0004_user_audit_log.sql"),
    ),
    (
        "0005_test_data",
        include_str!("../../migrations/0005_test_data.sql"),
    ),
    (
        "0006_patient_code",
        include_str!("../../migrations/0006_patient_code.sql"),
    ),
];

/// Aplica las migraciones pendientes. Devuelve la versión de schema resultante.
pub fn run_migrations(conn: &mut SimpleConnection) -> Result<i32, AppError> {
    ensure_schema_table(conn)?;

    let applied: Vec<(i32,)> = conn
        .query("SELECT VERSION FROM SCHEMA_MIGRATIONS ORDER BY VERSION", ())
        .map_err(AppError::from)?;

    for (idx, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (idx + 1) as i32;
        if applied.iter().any(|(v,)| *v == version) {
            continue;
        }

        for stmt in split_statements(sql)? {
            conn.execute(&stmt, ()).map_err(|e| {
                AppError::Db(format!(
                    "Migración {name} (v{version}) — sentencia fallida: {e}\nSQL: {stmt}"
                ))
            })?;
        }

        conn.execute(
            "INSERT INTO SCHEMA_MIGRATIONS (VERSION, NAME) VALUES (?, ?)",
            (&version, name),
        )
        .map_err(AppError::from)?;
    }

    let max: Option<(i32,)> = conn
        .query_first("SELECT MAX(VERSION) FROM SCHEMA_MIGRATIONS", ())
        .map_err(AppError::from)?;
    Ok(max.map(|(v,)| v).unwrap_or(0))
}

fn ensure_schema_table(conn: &mut SimpleConnection) -> Result<(), AppError> {
    let exists: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM rdb$relations WHERE rdb$relation_name = 'SCHEMA_MIGRATIONS'",
            (),
        )
        .map_err(AppError::from)?;
    let exists = exists.is_some();

    if !exists {
        conn.execute(
            "CREATE TABLE SCHEMA_MIGRATIONS (
                VERSION    INTEGER NOT NULL PRIMARY KEY,
                NAME       VARCHAR(100) NOT NULL,
                APPLIED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
            )",
            (),
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

/// Divide un script SQL en sentencias individuales respetando `SET TERM`
/// (necesario para crear stored procedures/triggers con ';' internos).
fn split_statements(sql: &str) -> Result<Vec<String>, AppError> {
    let mut statements = Vec::new();
    let mut terminator = ";".to_string();
    let mut current = String::new();

    for raw_line in sql.lines() {
        let line = raw_line.trim();

        if line.is_empty() {
            continue;
        }
        if line.starts_with("--") {
            continue;
        }
        // Directiva de terminador: "SET TERM ^ ;" o "SET TERM ; ^"
        if line.to_ascii_uppercase().starts_with("SET TERM") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                terminator = parts[2].to_string();
            }
            continue;
        }

        if current.is_empty() {
            current.push_str(raw_line);
        } else {
            current.push('\n');
            current.push_str(raw_line);
        }

        if current.trim_end().ends_with(&terminator) {
            let stmt = current
                .trim()
                .strip_suffix(&terminator)
                .unwrap_or(current.trim())
                .trim()
                .to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        return Err(AppError::Db(
            "Migración SQL incompleta: falta el terminador de sentencia".into(),
        ));
    }

    Ok(statements)
}
