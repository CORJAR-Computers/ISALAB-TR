use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::auth::{AuditLogEntry, SessionUser};

/// Fila de USERS (incluye el hash cuando la consulta lo pide).
pub struct UserRecord {
    pub id: i32,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub password_hash: Option<String>,
    pub active: bool,
    pub must_change_password: bool,
}

pub fn find_by_username(
    conn: &mut SimpleConnection,
    username: &str,
) -> Result<Option<UserRecord>, AppError> {
    let row: Option<(
        i32,
        String,
        String,
        String,
        Option<String>,
        bool,
        bool,
    )> = conn
        .query_first(
            "SELECT ID, USERNAME, FULL_NAME, ROLE, PASSWORD_HASH, ACTIVE,
                    MUST_CHANGE_PASSWORD
             FROM USERS WHERE UPPER(USERNAME) = UPPER(?)",
            (&username,),
        )
        .map_err(AppError::from)?;

    Ok(row.map(|r| UserRecord {
        id: r.0,
        username: r.1,
        full_name: r.2,
        role: r.3,
        password_hash: r.4,
        active: r.5,
        must_change_password: r.6,
    }))
}

pub fn to_session(user: &UserRecord) -> SessionUser {
    SessionUser {
        id: user.id,
        username: user.username.clone(),
        full_name: user.full_name.clone(),
        role: user.role.clone(),
        must_change_password: user.must_change_password,
    }
}

/// Registra una acción en la tabla de auditoría (USER_AUDIT_LOG).
pub fn log_audit(
    conn: &mut SimpleConnection,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    details: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO USER_AUDIT_LOG (USER_ID, USERNAME, ACTION, DETAILS) VALUES (?, ?, ?, ?)",
        (&user_id, &username, &action, &details),
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// Lista entradas del registro de auditoría con paginación.
/// Orden descendente (más reciente primero). `limit` máximo 500, `offset` desde 0.
pub fn list_audit_log(
    conn: &mut SimpleConnection,
    limit: i32,
    offset: i32,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let rows: Vec<(i32, Option<i32>, String, String, Option<String>, String)> = conn
        .query(
            "SELECT FIRST ? SKIP ?
                l.ID, l.USER_ID, l.USERNAME, l.ACTION, l.DETAILS,
                LEFT(CAST(l.CREATED_AT AS VARCHAR(60)), 19)
             FROM USER_AUDIT_LOG l
             ORDER BY l.ID DESC",
            (&limit, &offset),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| AuditLogEntry {
            id: r.0,
            user_id: r.1,
            username: r.2,
            action: r.3,
            details: r.4,
            created_at: r.5,
        })
        .collect())
}
