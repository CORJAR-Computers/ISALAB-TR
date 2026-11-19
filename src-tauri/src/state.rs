use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Instant;

use tauri::{AppHandle, Manager};

use crate::ai_cache::AiCache;
use crate::db::config::{self, DbConfig, DbMode};
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
    /// Configuración efectiva de la conexión (embedded o servidor remoto).
    pub db: DbConfig,
    pub schema_version: i32,
    pub init_error: Option<String>,
    /// Sesión activa (una única sesión local, como corresponde a una app de
    /// escritorio de un solo usuario a la vez).
    pub session: Mutex<Option<SessionUser>>,
    /// Intentos fallidos de login por usuario (protección contra fuerza bruta).
    pub login_attempts: Mutex<HashMap<String, LoginAttempts>>,
    /// Cache de interpretaciones de IA para evitar llamadas repetidas a Groq.
    pub ai_cache: AiCache,
    /// Contraseña del certificado PKCS#12 en memoria (para firmar PDFs).
    /// NUNCA se persiste en la base de datos: se pide de nuevo al reimportar
    /// o al reiniciar la app.
    pub pkcs12_password: Mutex<Option<String>>,
    #[allow(dead_code)]
    pub listeners: Vec<JoinHandle<()>>,
}

impl AppState {
    /// Inicializa la capa de datos (conexión, pool y listeners de eventos).
    ///
    /// Nunca aborta: si algo falla queda registrado en `init_error` y el
    /// frontend muestra el banner con la acción correctiva.
    pub fn init(app: &AppHandle) -> Self {
        let app_data = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&app_data);

        let embedded_db = app_data.join("isalab.fdb");
        let fbclient_path = app
            .path()
            .resolve(
                "binaries/firebird/fbclient.dll",
                tauri::path::BaseDirectory::Resource,
            )
            .unwrap_or_else(|_| PathBuf::from("binaries/firebird/fbclient.dll"));

        let (db, config_error) = match config::resolve(&app_data, &embedded_db, &fbclient_path) {
            Ok(db) => (db, None),
            Err(e) => (
                unconfigured_remote(&embedded_db, &fbclient_path),
                Some(e.to_string()),
            ),
        };

        // Clave compartida de la clínica: se deriva de la contraseña de conexión
        // (los tres PCs comparten usuario/clave de Firebird). No se guarda.
        crate::crypto::set_shared_password(&db.password);

        match bootstrap(&db) {
            Ok((pool, schema_version)) => {
                let listeners = crate::db::events::start_firebird_listeners(app.clone(), &db);
                Self {
                    pool,
                    db,
                    schema_version,
                    init_error: config_error,
                    session: Mutex::new(None),
                    login_attempts: Mutex::new(HashMap::new()),
                    ai_cache: AiCache::new(),
                    pkcs12_password: Mutex::new(None),
                    listeners,
                }
            }
            Err(e) => Self {
                pool: DbPool::empty(),
                db,
                schema_version: 0,
                init_error: Some(config_error.unwrap_or_else(|| e.to_string())),
                session: Mutex::new(None),
                login_attempts: Mutex::new(HashMap::new()),
                ai_cache: AiCache::new(),
                pkcs12_password: Mutex::new(None),
                listeners: Vec::new(),
            },
        }
    }
}

/// Configuración remota vacía: permite que la UI abra el diálogo de conexión
/// aunque el archivo guardado no se pueda descifrar.
fn unconfigured_remote(embedded_db: &std::path::Path, fbclient: &std::path::Path) -> DbConfig {
    DbConfig {
        mode: DbMode::Remote,
        ..DbConfig::embedded(embedded_db, fbclient)
    }
}
