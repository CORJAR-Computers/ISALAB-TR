use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::JoinHandle;

use rsfbclient::FbError;
use tauri::{AppHandle, Manager};

use crate::db::{bootstrap, DbPool};
use crate::models::auth::SessionUser;

/// Estado global gestionado por Tauri (accesible en cada comando).
pub struct AppState {
    pub pool: DbPool,
    pub db_path: PathBuf,
    pub fbclient_path: PathBuf,
    pub schema_version: i32,
    pub init_error: Option<String>,
    /// Sesión activa (una única sesión local, como corresponde a una app de
    /// escritorio de un solo usuario a la vez).
    pub session: Mutex<Option<SessionUser>>,
    #[allow(dead_code)]
    pub listeners: Vec<JoinHandle<Result<(), FbError>>>,
}

impl AppState {
    /// Inicializa Firebird Embedded (crea DB si falta, migra, monta el pool y
    /// los listeners de eventos). Nunca aborta: si algo falla queda registrado
    /// en `init_error` y el frontend muestra el banner de setup.
    pub fn init(app: &AppHandle) -> Self {
        let app_data = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&app_data);

        let db_path = app_data.join("isalab.fdb");
        let fbclient_path = app
            .path()
            .resolve(
                "binaries/firebird/fbclient.dll",
                tauri::path::BaseDirectory::Resource,
            )
            .unwrap_or_else(|_| PathBuf::from("binaries/firebird/fbclient.dll"));

        match bootstrap(&db_path, &fbclient_path) {
            Ok((pool, schema_version)) => {
                let listeners =
                    crate::db::events::start_firebird_listeners(
                        app.clone(),
                        &db_path,
                        &fbclient_path,
                    )
                    .unwrap_or_default();
                Self {
                    pool,
                    db_path,
                    fbclient_path,
                    schema_version,
                    init_error: None,
                    session: Mutex::new(None),
                    listeners,
                }
            }
            Err(e) => Self {
                pool: DbPool::empty(),
                db_path,
                fbclient_path,
                schema_version: 0,
                init_error: Some(e.to_string()),
                session: Mutex::new(None),
                listeners: Vec::new(),
            },
        }
    }
}
