use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::auth::SessionUser;

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
