use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Instant;

use rsfbclient::FbError;
use tauri::{AppHandle, Manager};

use crate::ai_cache::AiCache;
use crate::db::{bootstrap, DbPool};
use crate::models::auth::SessionUser;

/// Máximo de intentos fallidos antes de bloquear temporalmente el login.
pub const LOGIN_MAX_ATTEMPTS: u32 = 5;
/// Duración del bloqueo en segundos tras alcanzar el máximo de intentos.
pub const LOGIN_LOCKOUT_SECS: u64 = 300; // 5 minutos

/// Registro de intentos fallidos por nombre de usuario.
#[derive(Default)]
pub struct LoginAttempts {
    /// Número de intentos fallidos consecutivos.
    count: u32,
    /// Momento del último intento fallido (para calcular cooldown).
    last_attempt: Option<Instant>,
}

impl LoginAttempts {
    /// Devuelve `Some(segundos_restantes)` si la cuenta está bloqueada,
    /// o `None` si puede intentar.
    pub fn check_locked(&self) -> Option<u64> {
        if self.count < LOGIN_MAX_ATTEMPTS {
            return None;
        }
        let last = self.last_attempt?;
        let elapsed = last.elapsed().as_secs();
        if elapsed >= LOGIN_LOCKOUT_SECS {
            None // El bloqueo ya expiró
        } else {
            Some(LOGIN_LOCKOUT_SECS - elapsed)
        }
    }

    /// Registra un intento fallido más.
    pub fn record_failure(&mut self) {
        self.count += 1;
        self.last_attempt = Some(Instant::now());
    }

    /// Reinicia el contador (login exitoso).
    pub fn reset(&mut self) {
        self.count = 0;
        self.last_attempt = None;
    }
}

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
    /// Intentos fallidos de login por usuario (protección contra fuerza bruta).
    pub login_attempts: Mutex<HashMap<String, LoginAttempts>>,
    /// Cache de interpretaciones de IA para evitar llamadas repetidas a Groq.
    pub ai_cache: AiCache,
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
                    login_attempts: Mutex::new(HashMap::new()),
                    ai_cache: AiCache::new(),
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
                login_attempts: Mutex::new(HashMap::new()),
                ai_cache: AiCache::new(),
                listeners: Vec::new(),
            },
        }
    }
}
