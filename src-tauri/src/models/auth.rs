use serde::{Deserialize, Serialize};
use specta::Type;

/// Usuario autenticado (sin hash de contraseña).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionUser {
    pub id: i32,
    pub username: String,
    pub full_name: String,
    /// ADMIN | VETERINARIO | AUXILIAR
    pub role: String,
    /// `true` si el usuario debe fijar una contraseña propia al iniciar sesión.
    pub must_change_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

/// Fila del listado de usuarios (nunca expone el hash).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UserListItem {
    pub id: i32,
    pub username: String,
    pub full_name: String,
    /// ADMIN | VETERINARIO | AUXILIAR
    pub role: String,
    pub active: bool,
    pub must_change_password: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub username: String,
    pub full_name: String,
    /// ADMIN | VETERINARIO | AUXILIAR
    pub role: String,
    /// Contraseña inicial; el usuario deberá cambiarla en su primer acceso.
    pub initial_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordInput {
    pub current_password: String,
    pub new_password: String,
}
