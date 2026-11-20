use tauri::State;

use crate::auth::{require_admin, require_inventory, require_session};
use crate::error::AppError;
use crate::models::product::{
    CreateStockMovementInput, Product, ProductStockItem, SaveProductInput, StockMovement,
};
use crate::repositories::auth as auth_repo;
use crate::repositories::products as products_repo;
use crate::state::AppState;

fn log_inventory_audit(
    state: &AppState,
    user_id: Option<i32>,
    username: &str,
    action: &str,
    detail: &str,
) {
    if let Ok(mut pooled) = state.pool.acquire() {
        auth_repo::log_audit(pooled.conn(), user_id, username, action, Some(detail)).ok();
    }
}

/// Catálogo de productos con su stock actual.
#[tauri::command]
#[specta::specta]
pub fn list_products(
    state: State<'_, AppState>,
    search: Option<String>,
    category: Option<String>,
    include_inactive: Option<bool>,
) -> Result<Vec<ProductStockItem>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    products_repo::list_products(
        pooled.conn(),
        search.as_deref(),
        category.as_deref(),
        include_inactive.unwrap_or(false),
    )
}

/// Producto del catálogo por id.
#[tauri::command]
#[specta::specta]
pub fn get_product(state: State<'_, AppState>, id: i32) -> Result<Option<Product>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    products_repo::get_product(pooled.conn(), id)
}

/// Crea o edita un producto del catálogo (solo ADMIN).
#[tauri::command]
#[specta::specta]
pub fn save_product(
    state: State<'_, AppState>,
    input: SaveProductInput,
) -> Result<Product, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let product = products_repo::save_product(pooled.conn(), &input)?;

    log_inventory_audit(
        &state,
        Some(user.id),
        &user.username,
        "PRODUCT_SAVE",
        &format!("Producto {} · {}", product.code, product.name),
    );
    Ok(product)
}

/// Activa o desactiva un producto del catálogo (solo ADMIN).
#[tauri::command]
#[specta::specta]
pub fn set_product_active(
    state: State<'_, AppState>,
    id: i32,
    active: bool,
) -> Result<Product, AppError> {
    let user = require_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let product = products_repo::set_product_active(pooled.conn(), id, active)?;

    log_inventory_audit(
        &state,
        Some(user.id),
        &user.username,
        "PRODUCT_STATUS",
        &format!(
            "Producto {} · {}",
            product.code,
            if active { "activo" } else { "inactivo" }
        ),
    );
    Ok(product)
}

/// Registra un movimiento de inventario (entrada, salida, consumo…).
#[tauri::command]
#[specta::specta]
pub fn add_stock_movement(
    state: State<'_, AppState>,
    input: CreateStockMovementInput,
) -> Result<StockMovement, AppError> {
    let user = require_inventory(&state)?;
    let mut pooled = state.pool.acquire()?;
    let movement = products_repo::add_stock_movement(pooled.conn(), &input, Some(user.id))?;

    log_inventory_audit(
        &state,
        Some(user.id),
        &user.username,
        "STOCK_MOVEMENT",
        &format!(
            "{} · {} · {} unidad(es)",
            movement.kind, movement.product_name, movement.quantity
        ),
    );
    Ok(movement)
}

/// Histórico de movimientos de inventario.
#[tauri::command]
#[specta::specta]
pub fn list_stock_movements(
    state: State<'_, AppState>,
    product_id: Option<i32>,
    limit: Option<i32>,
) -> Result<Vec<StockMovement>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    products_repo::list_stock_movements(pooled.conn(), product_id, limit.unwrap_or(50))
}
