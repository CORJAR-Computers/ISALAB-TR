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

#[tauri::command]
#[specta::specta]
pub fn create_local_backup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    dest_path: String,
) -> Result<String, AppError> {
    use std::fs::File;
    use std::io::{Read, Write};
    use walkdir::WalkDir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|e| AppError::Internal(e.to_string()))?;
    
    let file = File::create(&dest_path).map_err(|e| AppError::Internal(format!("Error creando archivo zip: {}", e)))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let it = WalkDir::new(&app_data_dir).into_iter().filter_map(|e| e.ok());

    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(&app_data_dir).unwrap();
        let name_str = name.to_string_lossy().replace("\\", "/");

        if path.is_file() {
            if name_str.ends_with(".lock") {
                continue; // Saltar locks temporales
            }
            zip.start_file(name_str, options).map_err(|e| AppError::Internal(e.to_string()))?;
            let mut f = File::open(path).map_err(|e| AppError::Internal(e.to_string()))?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| AppError::Internal(e.to_string()))?;
            zip.write_all(&buffer).map_err(|e| AppError::Internal(e.to_string()))?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name_str, options).map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    zip.finish().map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(dest_path)
}
