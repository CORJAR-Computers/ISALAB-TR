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

    let (id, username, full_name, role, hash) = row
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;

    let hash = hash.ok_or_else(|| {
        AppError::Validation("Usuario sin contraseña configurada".into())
    })?;
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
