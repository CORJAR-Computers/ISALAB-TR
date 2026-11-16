use tauri::State;

use crate::auth::{require_admin, require_session};
use crate::error::AppError;
use crate::models::service::{
    SaveServiceCategoryInput, SaveServiceInput, Service, ServiceCategory,
};
use crate::repositories::services as services_repo;
use crate::state::AppState;

/// Lista las categorías del catálogo (operativas; admin puede ver inactivas).
#[tauri::command]
#[specta::specta]
pub fn list_service_categories(
    state: State<'_, AppState>,
) -> Result<Vec<ServiceCategory>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    services_repo::list_categories(pooled.conn(), false)
}

/// Lista TODAS las categorías, incluidas las inactivas (solo ADMIN).
#[tauri::command]
#[specta::specta]
pub fn list_all_service_categories(
    state: State<'_, AppState>,
) -> Result<Vec<ServiceCategory>, AppError> {
    require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    services_repo::list_categories(pooled.conn(), true)
}

/// Crea o actualiza una categoría del catálogo (ADMIN).
#[tauri::command]
#[specta::specta]
pub fn save_service_category(
    state: State<'_, AppState>,
    input: SaveServiceCategoryInput,
) -> Result<ServiceCategory, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let category = services_repo::save_category(pooled.conn(), &input)?;

    log_service_audit(
        &state,
        Some(user.id),
        &user.username,
        "SERVICE_CATEGORY",
        &format!("Categoría {} ({}) guardada", category.code, category.name),
    );
    Ok(category)
}

/// Da de baja una categoría (baja lógica si tiene servicios; ADMIN).
#[tauri::command]
#[specta::specta]
pub fn delete_service_category(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    services_repo::delete_category(pooled.conn(), id)?;

    log_service_audit(
        &state,
        Some(user.id),
        &user.username,
        "SERVICE_CATEGORY",
        &format!("Categoría {id} dada de baja"),
    );
    Ok(())
}

/// Lista servicios del catálogo con filtros (lectura para toda la sesión).
#[tauri::command]
#[specta::specta]
pub fn list_services(
    state: State<'_, AppState>,
    category_id: Option<i32>,
    search: Option<String>,
    include_inactive: Option<bool>,
) -> Result<Vec<Service>, AppError> {
    let user = require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    // Solo ADMIN puede pedir inactivos.
    let include_inactive = include_inactive.unwrap_or(false) && user.role == "ADMIN";
    services_repo::list_services(
        pooled.conn(),
        category_id,
        search.as_deref(),
        include_inactive,
    )
}

/// Obtiene un servicio por ID.
#[tauri::command]
#[specta::specta]
pub fn get_service(state: State<'_, AppState>, id: i32) -> Result<Option<Service>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    services_repo::get_service(pooled.conn(), id)
}

/// Crea o actualiza un servicio del catálogo (ADMIN).
#[tauri::command]
#[specta::specta]
pub fn save_service(
    state: State<'_, AppState>,
    input: SaveServiceInput,
) -> Result<Service, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let service = services_repo::save_service(pooled.conn(), &input)?;

    log_service_audit(&state, Some(user.id), &user.username, "SERVICE", &format!(
        "Servicio {} ({}) · precio {} · duración {} min",
        service.code, service.name, service.price, service.duration_min
    ));
    Ok(service)
}

/// Activa o desactiva un servicio (baja lógica; ADMIN).
#[tauri::command]
#[specta::specta]
pub fn set_service_active(
    state: State<'_, AppState>,
    id: i32,
    active: bool,
) -> Result<Service, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let service = services_repo::set_service_active(pooled.conn(), id, active)?;

    log_service_audit(
        &state,
        Some(user.id),
        &user.username,
        "SERVICE",
        &format!(
            "Servicio {} ({}) {}",
            service.code,
            service.name,
            if active { "activado" } else { "desactivado" }
        ),
    );
    Ok(service)
}

/// Registra en la auditoría (best-effort: no bloquea la operación).
fn log_service_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(conn.conn(), user_id, username, action, Some(detail))
            .ok();
    }
}
