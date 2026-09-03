use tauri::State;

use crate::auth::{require_session, require_vet_or_admin};
use crate::error::AppError;
use crate::models::qc::{
    QcAnalyzerStatus, QcChartData, QcControlMaterial, QcMaterialInput, QcRun, QcRunInput, QcTarget,
};
use crate::repositories::qc as qc_repo;
use crate::state::AppState;

/// Lista los materiales de control activos.
#[tauri::command]
#[specta::specta]
pub fn list_qc_materials(state: State<'_, AppState>) -> Result<Vec<QcControlMaterial>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::list_control_materials(pooled.conn())
}

/// Objetivos (media/SD) de un material de control.
#[tauri::command]
#[specta::specta]
pub fn list_qc_targets(
    state: State<'_, AppState>,
    control_material_id: i32,
) -> Result<Vec<QcTarget>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::list_targets(pooled.conn(), control_material_id)
}

/// Crea o actualiza un material de control con sus objetivos.
#[tauri::command]
#[specta::specta]
pub fn save_qc_material(
    state: State<'_, AppState>,
    input: QcMaterialInput,
) -> Result<QcControlMaterial, AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::save_control_material(pooled.conn(), &input)
}

/// Elimina un material de control (cascade sobre objetivos, corridas y mediciones).
#[tauri::command]
#[specta::specta]
pub fn delete_qc_material(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::delete_control_material(pooled.conn(), id)
}

/// Registra una corrida de control evaluando las reglas de Westgard.
#[tauri::command]
#[specta::specta]
pub fn record_qc_run(state: State<'_, AppState>, input: QcRunInput) -> Result<QcRun, AppError> {
    let user = require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    let run = qc_repo::record_run(pooled.conn(), &input, &user.username)?;

    if let Ok(mut audit_conn) = state.pool.acquire() {
        crate::repositories::auth::log_audit(
            audit_conn.conn(),
            Some(user.id),
            &user.username,
            "QC_RUN",
            Some(&format!(
                "Corrida QC {} · {} · {} mediciones",
                run.control_name,
                run.status,
                run.measurements.len()
            )),
        )
        .ok();
    }

    Ok(run)
}

/// Lista corridas de control (opcionalmente de un material).
#[tauri::command]
#[specta::specta]
pub fn list_qc_runs(
    state: State<'_, AppState>,
    control_material_id: Option<i32>,
) -> Result<Vec<QcRun>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::list_runs(pooled.conn(), control_material_id)
}

/// Elimina una corrida de control (con sus mediciones).
#[tauri::command]
#[specta::specta]
pub fn delete_qc_run(state: State<'_, AppState>, id: i32) -> Result<(), AppError> {
    require_vet_or_admin(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::delete_run(pooled.conn(), id)
}

/// Datos para el gráfico Levey-Jennings de un analito.
#[tauri::command]
#[specta::specta]
pub fn get_qc_chart(
    state: State<'_, AppState>,
    control_material_id: i32,
    analyte_id: i32,
) -> Result<Option<QcChartData>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::get_chart(pooled.conn(), control_material_id, analyte_id)
}

/// Estado de la última corrida QC por analizador (badge de alerta).
#[tauri::command]
#[specta::specta]
pub fn list_qc_analyzer_status(
    state: State<'_, AppState>,
) -> Result<Vec<QcAnalyzerStatus>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    qc_repo::list_analyzer_status(pooled.conn())
}
