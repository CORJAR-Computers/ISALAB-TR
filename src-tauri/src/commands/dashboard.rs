use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::dashboard::DashboardStats;
use crate::repositories::dashboard as dashboard_repo;
use crate::state::AppState;

/// Métricas del panel de control: contadores, agenda de próximas citas y
/// cirugías, refuerzos de vacunación y últimas muestras.
#[tauri::command]
#[specta::specta]
pub fn get_dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    dashboard_repo::get_stats(pooled.conn())
}
