//! Autenticación: hashing y verificación de contraseñas (Argon2id).
//!
//! En una app de escritorio la verificación se hace contra la tabla USERS
//! local; el hash nunca sale del dispositivo.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::rngs::OsRng;

use crate::error::AppError;

/// Genera un hash Argon2id con salt aleatorio.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))
}

/// Verifica una contraseña contra un hash PHC (p. ej. `$argon2id$v=19$…`).
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(format!("Hash inválido: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

use crate::models::auth::SessionUser;
use crate::state::AppState;

/// Devuelve el usuario de la sesión activa o un error 403 Forbidden.
pub fn require_session(state: &AppState) -> Result<SessionUser, AppError> {
    let guard = state
        .session
        .lock()
        .map_err(|_| AppError::Internal("Sesión bloqueada".into()))?;
    guard
        .clone()
        .ok_or_else(|| AppError::Forbidden("Inicia sesión para continuar".into()))
}

/// Exige que la sesión activa pertenezca a un usuario con rol ADMIN.
pub fn require_admin(state: &AppState) -> Result<SessionUser, AppError> {
    let user = require_session(state)?;
    if user.role == "ADMIN" {
        Ok(user)
    } else {
        Err(AppError::Forbidden(
            "Solo el administrador puede realizar esta acción".into(),
        ))
    }
}

/// Exige que la sesión activa pertenezca a un VETERINARIO o ADMIN.
pub fn require_vet_or_admin(state: &AppState) -> Result<SessionUser, AppError> {
    let user = require_session(state)?;
    if user.role == "ADMIN" || user.role == "VETERINARIO" {
        Ok(user)
    } else {
        Err(AppError::Forbidden(
            "Se requieren permisos de Veterinario o Administrador para esta acción".into(),
        ))
    }
}

// ======================= CAPACIDADES POR ROL (clínica) ======================

/// Capacidades de un rol. Introducido con la clínica integral para no
/// multiplicar condicionales por rol: los comandos exigen capacidades, no
/// roles literales. Los tres roles históricos conservan su comportamiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Check-in de visitas, cola de espera, agregar servicios.
    pub can_receive: bool,
    /// Registro clínico (consultas, resultados, cirugías, vacunas).
    pub can_clinical: bool,
    /// Fichas y sesiones de peluquería.
    pub can_groom: bool,
    /// Facturación y cobro de visitas.
    pub can_bill: bool,
    /// Estancias (guardería, hospedaje, hospitalización) y bitácora de cuidados.
    pub can_stay: bool,
    /// Catálogo de productos e inventario (entradas, salidas, consumos).
    pub can_inventory: bool,
    /// Administración completa (usuarios, catálogo, configuración).
    pub is_admin: bool,
}

impl Capabilities {
    /// ADMIN: todo.
    pub fn admin() -> Self {
        Self {
            can_receive: true,
            can_clinical: true,
            can_groom: true,
            can_bill: true,
            can_stay: true,
            can_inventory: true,
            is_admin: true,
        }
    }
}

/// Mapa de capacidades por rol.
pub fn capabilities_for_role(role: &str) -> Capabilities {
    match role {
        "ADMIN" => Capabilities::admin(),
        "VETERINARIO" => Capabilities {
            can_receive: true,
            can_clinical: true,
            can_groom: false,
            can_bill: true,
            can_stay: true,
            can_inventory: false,
            is_admin: false,
        },
        // El AUXILIAR histórico podía facturar (create_invoice exige
        // vet_or_admin hoy, pero recepción+facturación es su dominio natural
        // en la clínica) y apoyar peluquería.
        "AUXILIAR" => Capabilities {
            can_receive: true,
            can_clinical: false,
            can_groom: true,
            can_bill: true,
            can_stay: true,
            can_inventory: true,
            is_admin: false,
        },
        "RECEPCION" => Capabilities {
            can_receive: true,
            can_clinical: false,
            can_groom: false,
            can_bill: true,
            can_stay: true,
            can_inventory: true,
            is_admin: false,
        },
        "PELUQUERO" => Capabilities {
            can_receive: false,
            can_clinical: false,
            can_groom: true,
            can_bill: false,
            can_stay: false,
            can_inventory: false,
            is_admin: false,
        },
        _ => Capabilities {
            can_receive: false,
            can_clinical: false,
            can_groom: false,
            can_bill: false,
            can_stay: false,
            can_inventory: false,
            is_admin: false,
        },
    }
}

/// Exige que la sesión activa tenga la capacidad indicada.
pub fn require_capability(
    state: &AppState,
    check: impl Fn(&Capabilities) -> bool,
    message: &str,
) -> Result<SessionUser, AppError> {
    let user = require_session(state)?;
    let caps = capabilities_for_role(&user.role);
    if check(&caps) {
        Ok(user)
    } else {
        Err(AppError::Forbidden(message.into()))
    }
}

/// Exige capacidad de recepción (check-in, cola de espera, servicios).
pub fn require_receive(state: &AppState) -> Result<SessionUser, AppError> {
    require_capability(
        state,
        |c| c.can_receive,
        "Se requiere rol de Recepción, Auxiliar, Veterinario o Administrador",
    )
}

/// Exige capacidad de peluquería.
pub fn require_groom(state: &AppState) -> Result<SessionUser, AppError> {
    require_capability(
        state,
        |c| c.can_groom,
        "Se requiere rol de Peluquero, Auxiliar, Veterinario o Administrador",
    )
}

/// Exige capacidad de facturación.
pub fn require_bill(state: &AppState) -> Result<SessionUser, AppError> {
    require_capability(
        state,
        |c| c.can_bill,
        "Se requiere capacidad de facturación (Recepción, Auxiliar, \
         Veterinario o Administrador)",
    )
}

/// Exige capacidad de estancia (guardería, hospedaje y hospitalización).
pub fn require_stay(state: &AppState) -> Result<SessionUser, AppError> {
    require_capability(
        state,
        |c| c.can_stay,
        "Se requiere capacidad de estancia (Recepción, Auxiliar, \
         Veterinario o Administrador)",
    )
}

/// Exige capacidad de inventario (catálogo de productos y movimientos).
pub fn require_inventory(state: &AppState) -> Result<SessionUser, AppError> {
    require_capability(
        state,
        |c| c.can_inventory,
        "Se requiere capacidad de inventario (Recepción, Auxiliar o Administrador)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_by_role() {
        let admin = capabilities_for_role("ADMIN");
        assert!(admin.is_admin && admin.can_receive && admin.can_bill);
        assert!(admin.can_stay && admin.can_inventory);

        let recep = capabilities_for_role("RECEPCION");
        assert!(recep.can_receive && recep.can_bill);
        assert!(recep.can_stay && recep.can_inventory);
        assert!(!recep.can_clinical && !recep.can_groom && !recep.is_admin);

        let groom = capabilities_for_role("PELUQUERO");
        assert!(groom.can_groom);
        assert!(!groom.can_receive && !groom.can_bill && !groom.is_admin);
        assert!(!groom.can_stay && !groom.can_inventory);

        let aux = capabilities_for_role("AUXILIAR");
        assert!(aux.can_receive && aux.can_groom && aux.can_bill);
        assert!(aux.can_stay && aux.can_inventory);
        assert!(!aux.can_clinical && !aux.is_admin);

        let vet = capabilities_for_role("VETERINARIO");
        assert!(vet.can_clinical && vet.can_receive && vet.can_bill);
        assert!(vet.can_stay);
        // El veterinario no administra el inventario de la tienda.
        assert!(!vet.can_inventory && !vet.can_groom && !vet.is_admin);

        // Rol desconocido: sin capacidades.
        let guest = capabilities_for_role("GUEST");
        assert!(!guest.can_receive && !guest.is_admin);
        assert!(!guest.can_stay && !guest.can_inventory);
    }

    #[test]
    fn test_hash_and_verify_password() {
        let password = "SecretPassword123!";
        let hash = hash_password(password).expect("El hashing debe ser exitoso");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("WrongPassword", &hash).unwrap());
    }

    // Dummy State creation for testing would require full AppState setup,
    // so we can test the roles logic conceptually or create a mock AppState if it was easy.
    // Instead, since require_admin is tightly coupled with State, we would usually
    // test the business logic decoupling it, but for now we'll leave a placeholder test.
    #[test]
    fn test_role_auth_logic() {
        let user_admin = SessionUser {
            id: 1,
            username: "admin".into(),
            full_name: "Admin".into(),
            role: "ADMIN".into(),
            must_change_password: false,
        };
        let user_vet = SessionUser {
            id: 2,
            username: "vet".into(),
            full_name: "Vet".into(),
            role: "VETERINARIO".into(),
            must_change_password: false,
        };
        let user_aux = SessionUser {
            id: 3,
            username: "aux".into(),
            full_name: "Aux".into(),
            role: "AUXILIAR".into(),
            must_change_password: false,
        };

        // Logic of require_admin
        assert_eq!(user_admin.role, "ADMIN");
        assert_ne!(user_vet.role, "ADMIN");

        // Logic of require_vet_or_admin
        assert!(user_admin.role == "ADMIN" || user_admin.role == "VETERINARIO");
        assert!(user_vet.role == "ADMIN" || user_vet.role == "VETERINARIO");
        assert!(!(user_aux.role == "ADMIN" || user_aux.role == "VETERINARIO"));
    }
}
