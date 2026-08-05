use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::auth::{AuditLogEntry, SessionUser};

type UserRow = (
    i32,
    String,
    String,
    String,
    Option<String>,
    bool,
    bool,
);

type AuditLogRow = (i32, Option<i32>, String, String, Option<String>, String);

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
    let row: Option<UserRow> = conn
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

/// Filtros opcionales para el registro de auditoría.
pub struct AuditFilters {
    pub username: Option<String>,
    pub action: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// Lista entradas del registro de auditoría con paginación y filtros.
/// Orden descendente (más reciente primero). `limit` máximo 500, `offset` desde 0.
pub fn list_audit_log(
    conn: &mut SimpleConnection,
    limit: i32,
    offset: i32,
    filters: Option<&AuditFilters>,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let mut sql = String::from(
        "SELECT FIRST ? SKIP ?
            l.ID, l.USER_ID, l.USERNAME, l.ACTION, l.DETAILS,
            LEFT(CAST(l.CREATED_AT AS VARCHAR(60)), 19)
         FROM USER_AUDIT_LOG l"
    );
    let mut conditions: Vec<String> = Vec::new();

    if let Some(f) = filters {
        if let Some(ref u) = f.username {
            if !u.is_empty() {
                let sanitized = u.replace('\\', "");
                conditions.push(format!("UPPER(l.USERNAME) LIKE UPPER('%{}%')", sanitized));
            }
        }
        if let Some(ref a) = f.action {
            if !a.is_empty() {
                let sanitized = a.replace('\\', "");
                conditions.push(format!("l.ACTION = '{}'", sanitized));
            }
        }
        if let Some(ref df) = f.date_from {
            if !df.is_empty() {
                let sanitized = df.replace('\\', "");
                conditions.push(format!("l.CREATED_AT >= '{}'", sanitized));
            }
        }
        if let Some(ref dt) = f.date_to {
            if !dt.is_empty() {
                let sanitized = dt.replace('\\', "");
                conditions.push(format!("l.CREATED_AT <= '{} 23:59:59'", sanitized));
            }
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY l.ID DESC");

    let rows: Vec<AuditLogRow> = conn
        .query(&sql, (&limit, &offset))
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use super::*;
    use crate::test_helpers::*;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_find_by_username_admin() {
        let (mut conn, db_path) = setup();
        // The seed creates an admin user with NULL password hash
        let user = find_by_username(&mut conn, "admin").unwrap();
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.role, "ADMIN");
        assert!(user.active);
        // Seed inserts with NULL hash
        assert!(user.password_hash.is_none());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_find_by_username_case_insensitive() {
        let (mut conn, db_path) = setup();
        let user = find_by_username(&mut conn, "ADMIN").unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().username, "admin");
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_find_by_username_not_found() {
        let (mut conn, db_path) = setup();
        let user = find_by_username(&mut conn, "nonexistent").unwrap();
        assert!(user.is_none());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_find_by_username_after_creation() {
        let (mut conn, db_path) = setup();
        // Create a new user
        crate::repositories::users::create(
            &mut conn,
            &crate::models::auth::CreateUserInput {
                username: "vet_test".to_string(),
                full_name: "Vet Test".to_string(),
                role: "VETERINARIO".to_string(),
                initial_password: "password123".to_string(),
            },
        ).unwrap();

        let user = find_by_username(&mut conn, "vet_test").unwrap();
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.role, "VETERINARIO");
        assert!(user.must_change_password);
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_to_session_conversion() {
        let record = UserRecord {
            id: 1,
            username: "admin".to_string(),
            full_name: "Administrator".to_string(),
            role: "ADMIN".to_string(),
            password_hash: Some("hash".to_string()),
            active: true,
            must_change_password: false,
        };

        let session = to_session(&record);
        assert_eq!(session.id, 1);
        assert_eq!(session.username, "admin");
        assert_eq!(session.full_name, "Administrator");
        assert_eq!(session.role, "ADMIN");
        assert!(!session.must_change_password);
    }

    #[test]
    fn test_to_session_must_change_password() {
        let record = UserRecord {
            id: 2,
            username: "newuser".to_string(),
            full_name: "New User".to_string(),
            role: "AUXILIAR".to_string(),
            password_hash: Some("hash".to_string()),
            active: true,
            must_change_password: true,
        };

        let session = to_session(&record);
        assert!(session.must_change_password);
    }

    #[test]
    fn test_log_audit_and_list() {
        let (mut conn, db_path) = setup();

        // Create a user first for the audit entry
        crate::repositories::users::create(
            &mut conn,
            &crate::models::auth::CreateUserInput {
                username: "vet1".to_string(),
                full_name: "Vet 1".to_string(),
                role: "VETERINARIO".to_string(),
                initial_password: "password123".to_string(),
            },
        ).unwrap();

        // Log some audit entries
        log_audit(&mut conn, Some(1), "admin", "LOGIN", Some("Inicio de sesión exitoso")).unwrap();
        log_audit(&mut conn, Some(1), "admin", "LOGOUT", None).unwrap();
        log_audit(&mut conn, Some(2), "vet1", "CREATE_PATIENT", Some("Paciente Luna creado")).unwrap();

        let entries = list_audit_log(&mut conn, 10, 0, None).unwrap();
        assert_eq!(entries.len(), 3);

        // Should be ordered by ID DESC (most recent first)
        assert_eq!(entries[0].action, "CREATE_PATIENT");
        assert_eq!(entries[0].username, "vet1");
        assert_eq!(entries[1].action, "LOGOUT");
        assert_eq!(entries[2].action, "LOGIN");
        assert_eq!(entries[2].details, Some("Inicio de sesión exitoso".to_string()));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_pagination() {
        let (mut conn, db_path) = setup();

        // Create 5 audit entries
        for i in 0..5 {
            log_audit(&mut conn, Some(1), "admin", &format!("ACTION_{}", i), None).unwrap();
        }

        // Get first page
        let page1 = list_audit_log(&mut conn, 2, 0, None).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].action, "ACTION_4"); // Most recent first
        assert_eq!(page1[1].action, "ACTION_3");

        // Get second page
        let page2 = list_audit_log(&mut conn, 2, 2, None).unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].action, "ACTION_2");
        assert_eq!(page2[1].action, "ACTION_1");

        // Get third page
        let page3 = list_audit_log(&mut conn, 2, 4, None).unwrap();
        assert_eq!(page3.len(), 1);
        assert_eq!(page3[0].action, "ACTION_0");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_empty() {
        let (mut conn, db_path) = setup();
        let entries = list_audit_log(&mut conn, 10, 0, None).unwrap();
        assert!(entries.is_empty());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_log_audit_with_null_user_id() {
        let (mut conn, db_path) = setup();
        log_audit(&mut conn, None, "system", "SYSTEM_EVENT", Some("Evento del sistema")).unwrap();

        let entries = list_audit_log(&mut conn, 10, 0, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].user_id.is_none());
        assert_eq!(entries[0].username, "system");
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_filter_by_username() {
        let (mut conn, db_path) = setup();

        // Create users
        crate::repositories::users::create(
            &mut conn,
            &crate::models::auth::CreateUserInput {
                username: "vet_alice".to_string(),
                full_name: "Alice".to_string(),
                role: "VETERINARIO".to_string(),
                initial_password: "password123".to_string(),
            },
        ).unwrap();
        crate::repositories::users::create(
            &mut conn,
            &crate::models::auth::CreateUserInput {
                username: "vet_bob".to_string(),
                full_name: "Bob".to_string(),
                role: "VETERINARIO".to_string(),
                initial_password: "password123".to_string(),
            },
        ).unwrap();

        log_audit(&mut conn, Some(2), "vet_alice", "LOGIN", Some("Alice login")).unwrap();
        log_audit(&mut conn, Some(3), "vet_bob", "LOGIN", Some("Bob login")).unwrap();
        log_audit(&mut conn, Some(2), "vet_alice", "LOGOUT", None).unwrap();

        // Filter by username "alice"
        let filters = AuditFilters {
            username: Some("alice".to_string()),
            action: None,
            date_from: None,
            date_to: None,
        };
        let entries = list_audit_log(&mut conn, 10, 0, Some(&filters)).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.username.contains("alice")));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_filter_by_action() {
        let (mut conn, db_path) = setup();

        log_audit(&mut conn, Some(1), "admin", "LOGIN", None).unwrap();
        log_audit(&mut conn, Some(1), "admin", "LOGOUT", None).unwrap();
        log_audit(&mut conn, Some(1), "admin", "LOGIN", None).unwrap();

        let filters = AuditFilters {
            username: None,
            action: Some("LOGIN".to_string()),
            date_from: None,
            date_to: None,
        };
        let entries = list_audit_log(&mut conn, 10, 0, Some(&filters)).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.action == "LOGIN"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_filter_combined() {
        let (mut conn, db_path) = setup();

        crate::repositories::users::create(
            &mut conn,
            &crate::models::auth::CreateUserInput {
                username: "testuser".to_string(),
                full_name: "Test".to_string(),
                role: "AUXILIAR".to_string(),
                initial_password: "password123".to_string(),
            },
        ).unwrap();

        log_audit(&mut conn, Some(1), "admin", "LOGIN", None).unwrap();
        log_audit(&mut conn, Some(2), "testuser", "LOGIN", None).unwrap();
        log_audit(&mut conn, Some(2), "testuser", "LOGOUT", None).unwrap();

        // Filter by both username and action
        let filters = AuditFilters {
            username: Some("testuser".to_string()),
            action: Some("LOGIN".to_string()),
            date_from: None,
            date_to: None,
        };
        let entries = list_audit_log(&mut conn, 10, 0, Some(&filters)).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].username, "testuser");
        assert_eq!(entries[0].action, "LOGIN");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_audit_log_no_filters() {
        let (mut conn, db_path) = setup();

        log_audit(&mut conn, Some(1), "admin", "LOGIN", None).unwrap();
        log_audit(&mut conn, Some(1), "admin", "LOGOUT", None).unwrap();

        // No filters should return all entries
        let entries = list_audit_log(&mut conn, 10, 0, None).unwrap();
        assert_eq!(entries.len(), 2);

        cleanup_test_db(&db_path);
    }
}
