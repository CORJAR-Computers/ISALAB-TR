use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::auth;
use crate::error::AppError;
use crate::models::auth::{CreateUserInput, SessionUser, UserListItem};

/// Roles permitidos por el CHECK de la tabla USERS.
pub const ROLES: [&str; 3] = ["ADMIN", "VETERINARIO", "AUXILIAR"];

/// Mínimo de caracteres para las contraseñas.
pub const MIN_PASSWORD_LEN: usize = 6;

fn validate_role(role: &str) -> Result<(), AppError> {
    if ROLES.contains(&role) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Rol inválido: {role} (esperado ADMIN, VETERINARIO o AUXILIAR)"
        )))
    }
}

/// Listado de usuarios (sin hashes) ordenado por creación.
pub fn list(conn: &mut SimpleConnection) -> Result<Vec<UserListItem>, AppError> {
    let rows: Vec<(i32, String, String, String, bool, bool, String)> = conn
        .query(
            "SELECT ID, USERNAME, FULL_NAME, ROLE, ACTIVE, MUST_CHANGE_PASSWORD,
                    LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
             FROM USERS ORDER BY ID",
            (),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| UserListItem {
            id: r.0,
            username: r.1,
            full_name: r.2,
            role: r.3,
            active: r.4,
            must_change_password: r.5,
            created_at: r.6,
        })
        .collect())
}

/// Crea un usuario con contraseña inicial (Argon2id) y `MUST_CHANGE_PASSWORD`
/// activo: su primer inicio de sesión le exigirá cambiarla.
pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateUserInput,
) -> Result<UserListItem, AppError> {
    let username = input.username.trim();
    let full_name = input.full_name.trim();

    if username.is_empty() {
        return Err(AppError::Validation("El usuario es obligatorio".into()));
    }
    if full_name.is_empty() {
        return Err(AppError::Validation(
            "El nombre completo es obligatorio".into(),
        ));
    }
    validate_role(&input.role)?;
    if input.initial_password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::Validation(format!(
            "La contraseña debe tener al menos {MIN_PASSWORD_LEN} caracteres"
        )));
    }

    let exists: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM USERS WHERE UPPER(USERNAME) = UPPER(?)",
            (&username,),
        )
        .map_err(AppError::from)?;
    if exists.is_some() {
        return Err(AppError::Validation(format!(
            "El usuario '{username}' ya existe"
        )));
    }

    let hash = auth::hash_password(&input.initial_password)?;

    // El ID lo asigna el trigger BI_USERS (GEN_USERS_ID) al insertar NULL.
    conn.execute(
        "INSERT INTO USERS (USERNAME, FULL_NAME, ROLE, PASSWORD_HASH,
                            MUST_CHANGE_PASSWORD, ACTIVE)
         VALUES (?, ?, ?, ?, TRUE, TRUE)",
        (&username, &full_name, &input.role, &hash),
    )
    .map_err(AppError::from)?;

    let row: Option<(i32, String, String, String, bool, bool, String)> = conn
        .query_first(
            "SELECT ID, USERNAME, FULL_NAME, ROLE, ACTIVE, MUST_CHANGE_PASSWORD,
                    LEFT(CAST(CREATED_AT AS VARCHAR(60)), 19)
             FROM USERS WHERE UPPER(USERNAME) = UPPER(?)",
            (&username,),
        )
        .map_err(AppError::from)?;

    row.map(|r| UserListItem {
        id: r.0,
        username: r.1,
        full_name: r.2,
        role: r.3,
        active: r.4,
        must_change_password: r.5,
        created_at: r.6,
    })
    .ok_or_else(|| AppError::Internal("Usuario creado pero no recuperado".into()))
}

/// Cambia la contraseña del usuario actual: verifica la contraseña vigente,
/// hashea la nueva y limpia `MUST_CHANGE_PASSWORD`. Devuelve la sesión
/// actualizada.
pub fn change_password(
    conn: &mut SimpleConnection,
    user_id: i32,
    current_password: &str,
    new_password: &str,
) -> Result<SessionUser, AppError> {
    if new_password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::Validation(format!(
            "La contraseña debe tener al menos {MIN_PASSWORD_LEN} caracteres"
        )));
    }

    let row: Option<(i32, String, String, String, Option<String>)> = conn
        .query_first(
            "SELECT ID, USERNAME, FULL_NAME, ROLE, PASSWORD_HASH
             FROM USERS WHERE ID = ?",
            (&user_id,),
        )
        .map_err(AppError::from)?;

    let (id, username, full_name, role, hash) =
        row.ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;

    let hash =
        hash.ok_or_else(|| AppError::Validation("Usuario sin contraseña configurada".into()))?;
    if !auth::verify_password(current_password, &hash)? {
        return Err(AppError::Validation(
            "La contraseña actual no es correcta".into(),
        ));
    }

    let new_hash = auth::hash_password(new_password)?;
    conn.execute(
        "UPDATE USERS
            SET PASSWORD_HASH = ?, MUST_CHANGE_PASSWORD = FALSE,
                UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&new_hash, &id),
    )
    .map_err(AppError::from)?;

    Ok(SessionUser {
        id,
        username,
        full_name,
        role,
        must_change_password: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_list_users_empty() {
        let (mut conn, db_path) = setup();
        // After cleanup, only default admin might exist
        let users = list(&mut conn).unwrap();
        // The seed creates a default admin
        assert!(!users.is_empty());
        assert_eq!(users[0].username, "admin");
        assert_eq!(users[0].role, "ADMIN");
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_success() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "vet1".to_string(),
            full_name: "Dr. Carlos López".to_string(),
            role: "VETERINARIO".to_string(),
            initial_password: "password123".to_string(),
        };

        let user = create(&mut conn, &input).unwrap();
        assert!(user.id > 0);
        assert_eq!(user.username, "vet1");
        assert_eq!(user.full_name, "Dr. Carlos López");
        assert_eq!(user.role, "VETERINARIO");
        assert!(user.must_change_password);
        assert!(user.active);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_all_roles() {
        let (mut conn, db_path) = setup();

        for (i, role) in ROLES.iter().enumerate() {
            let input = CreateUserInput {
                username: format!("user_{}", i),
                full_name: format!("User {}", i),
                role: role.to_string(),
                initial_password: "password123".to_string(),
            };
            let user = create(&mut conn, &input).unwrap();
            assert_eq!(user.role, *role);
        }

        let users = list(&mut conn).unwrap();
        // 1 admin (seed) + 3 new users
        assert_eq!(users.len(), 4);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_duplicate_username() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "password123".to_string(),
        };
        create(&mut conn, &input).unwrap();

        // Try to create another user with same username
        let input2 = CreateUserInput {
            username: "testuser".to_string(),
            full_name: "Another User".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "password123".to_string(),
        };
        let result = create(&mut conn, &input2);
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_empty_username() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "".to_string(),
            full_name: "Test User".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "password123".to_string(),
        };
        let result = create(&mut conn, &input);
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_empty_full_name() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "testuser".to_string(),
            full_name: "".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "password123".to_string(),
        };
        let result = create(&mut conn, &input);
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_invalid_role() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "INVALID_ROLE".to_string(),
            initial_password: "password123".to_string(),
        };
        let result = create(&mut conn, &input);
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_short_password() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "testuser".to_string(),
            full_name: "Test User".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "12345".to_string(), // 5 chars < MIN_PASSWORD_LEN
        };
        let result = create(&mut conn, &input);
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_user_username_trimmed() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "  vet2  ".to_string(),
            full_name: "Dr. Ana".to_string(),
            role: "VETERINARIO".to_string(),
            initial_password: "password123".to_string(),
        };
        let user = create(&mut conn, &input).unwrap();
        assert_eq!(user.username, "vet2"); // trimmed

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_users_after_creation() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "vet3".to_string(),
            full_name: "Dr. López".to_string(),
            role: "VETERINARIO".to_string(),
            initial_password: "password123".to_string(),
        };
        create(&mut conn, &input).unwrap();

        let users = list(&mut conn).unwrap();
        // 1 admin (seed) + 1 new user
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.username == "vet3"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_change_password_success() {
        let (mut conn, db_path) = setup();

        // Create a user
        let input = CreateUserInput {
            username: "chpwd_user".to_string(),
            full_name: "Change Pwd User".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "oldpass123".to_string(),
        };
        let user = create(&mut conn, &input).unwrap();
        assert!(user.must_change_password);

        // Change password
        let session = change_password(&mut conn, user.id, "oldpass123", "newpass456").unwrap();
        assert_eq!(session.id, user.id);
        assert_eq!(session.username, "chpwd_user");
        assert!(!session.must_change_password);

        // Verify can login with new password (by checking hash)
        let row: Option<(Option<String>,)> = conn
            .query_first("SELECT PASSWORD_HASH FROM USERS WHERE ID = ?", (&user.id,))
            .unwrap();
        let hash = row.unwrap().0.unwrap();
        assert!(auth::verify_password("newpass456", &hash).unwrap());
        assert!(!auth::verify_password("oldpass123", &hash).unwrap());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_change_password_wrong_current() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "chpwd_user2".to_string(),
            full_name: "Change Pwd User 2".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "correctpass".to_string(),
        };
        let user = create(&mut conn, &input).unwrap();

        // Try with wrong current password
        let result = change_password(&mut conn, user.id, "wrongpass", "newpass123");
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_change_password_too_short() {
        let (mut conn, db_path) = setup();

        let input = CreateUserInput {
            username: "chpwd_user3".to_string(),
            full_name: "Change Pwd User 3".to_string(),
            role: "AUXILIAR".to_string(),
            initial_password: "validpass".to_string(),
        };
        let user = create(&mut conn, &input).unwrap();

        // Try to change to short password
        let result = change_password(&mut conn, user.id, "validpass", "12345");
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_change_password_nonexistent_user() {
        let (mut conn, db_path) = setup();
        let result = change_password(&mut conn, 999, "anypass", "newpass123");
        assert!(result.is_err());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_validate_role_function() {
        assert!(validate_role("ADMIN").is_ok());
        assert!(validate_role("VETERINARIO").is_ok());
        assert!(validate_role("AUXILIAR").is_ok());
        assert!(validate_role("INVALID").is_err());
        assert!(validate_role("").is_err());
    }
}
