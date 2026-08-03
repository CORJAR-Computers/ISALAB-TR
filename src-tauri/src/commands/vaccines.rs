use tauri::State;

use crate::commands::current_user;
use crate::error::AppError;
use crate::models::vaccine::{CreateVaccineInput, Vaccine, VaccineListItem};
use crate::repositories::vaccines as vaccines_repo;
use crate::state::AppState;

/// Registra una vacuna/desparasitación atribuyéndola al veterinario de la
/// sesión activa. Dispara el refresco del historial clínico en el frontend.
#[tauri::command]
#[specta::specta]
pub fn create_vaccine(
    state: State<'_, AppState>,
    input: CreateVaccineInput,
) -> Result<Vaccine, AppError> {
    let user = current_user(&state)?;
    let mut pooled = state.pool.acquire()?;
    vaccines_repo::create(pooled.conn(), &input, Some(user.id))
}

/// Listado global de vacunación (búsqueda por paciente, propietario o vacuna).
#[tauri::command]
#[specta::specta]
pub fn list_vaccines(
    state: State<'_, AppState>,
    search: Option<String>,
) -> Result<Vec<VaccineListItem>, AppError> {
    let mut pooled = state.pool.acquire()?;
    vaccines_repo::list(pooled.conn(), search.as_deref())
}
