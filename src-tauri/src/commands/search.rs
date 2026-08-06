use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::search::GlobalSearchResult;
use crate::repositories::search as search_repo;
use crate::state::AppState;

/// Búsqueda global (paleta Ctrl+K): pacientes, muestras, facturas y cirugías
/// por código o nombre, con su destino de navegación en la UI.
#[tauri::command]
#[specta::specta]
pub fn global_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<GlobalSearchResult>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    search_repo::global_search(pooled.conn(), &query)
}
