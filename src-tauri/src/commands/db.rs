use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DbHealth {
    pub ok: bool,
    pub message: String,
    pub db_path: String,
    pub fbclient_found: bool,
    pub fbclient_path: String,
    pub schema_version: i32,
}

/// Estado de la conexión Firebird Embedded (banner de setup en la UI).
#[tauri::command]
#[specta::specta]
pub fn db_health(state: State<'_, AppState>) -> DbHealth {
    let fbclient_found = state.fbclient_path.exists();
    let ok = state.init_error.is_none() && fbclient_found;

    DbHealth {
        ok,
        message: state
            .init_error
            .clone()
            .unwrap_or_else(|| "Firebird Embedded operativo".to_string()),
        db_path: state.db_path.display().to_string(),
        fbclient_found,
        fbclient_path: state.fbclient_path.display().to_string(),
        schema_version: state.schema_version,
    }
}
