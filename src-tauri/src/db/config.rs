//! Configuración de la conexión a Firebird.
//!
//! La app puede abrir la base de dos formas:
//!
//! - [`DbMode::Embedded`]: motor local sobre un `.fdb` del equipo (desarrollo y
//!   tests). Crea la base, aplica migraciones y siembra datos.
//! - [`DbMode::Remote`]: un único servidor Firebird 5 en la red de la clínica.
//!   La app solo se conecta; el esquema lo administra el runner.
//!
//! Orden de resolución (el primero que define un valor gana):
//!
//! 1. Variables de entorno `ISALAB_DB_*` (útil para CI y soporte).
//! 2. Archivo `%APPDATA%\com.isalab.clinic\db-connection.json`, con la
//!    contraseña cifrada con DPAPI (`enc:v1:<base64>`).
//! 3. Valor por defecto: embedded si la base local ya existe o es un build de
//!    desarrollo; si no, remoto sin configurar (la UI pide los datos).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::crypto;
use crate::error::AppError;

/// Puerto TCP por defecto del servidor Firebird.
pub const DEFAULT_PORT: u16 = 3050;
/// Usuario de aplicación creado por el administrador del servidor.
pub const DEFAULT_USER: &str = "ISALAB_APP";
/// Alias de la base en `databases.conf` del servidor.
pub const DEFAULT_DATABASE: &str = "ISALAB";
/// Tiempo máximo para abrir una conexión (y para el ping de salud).
pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 5_000;
/// Archivo de configuración dentro del directorio de datos de la app.
pub const CONNECTION_FILE: &str = "db-connection.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum DbMode {
    Embedded,
    Remote,
}

impl DbMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DbMode::Embedded => "embedded",
            DbMode::Remote => "remote",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "embedded" | "local" => Some(DbMode::Embedded),
            "remote" | "server" | "tcp" => Some(DbMode::Remote),
            _ => None,
        }
    }
}

/// Estado de la conexión que se muestra en el banner de la UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DbStatus {
    /// Remoto sin host/usuario definidos.
    NotConfigured,
    /// No se encontró `fbclient.dll` junto a la app.
    ClientMissing,
    /// El servidor no responde (apagado, IP incorrecta, firewall, cable).
    ServerUnreachable,
    /// El servidor respondió pero rechazó las credenciales.
    AuthFailed,
    /// El esquema es más antiguo que el que espera esta versión de la app.
    SchemaTooOld,
    /// El esquema es más nuevo que el que entiende esta versión de la app.
    SchemaTooNew,
    /// Conexión y esquema correctos.
    Connected,
}

impl DbStatus {
    pub fn is_ok(self) -> bool {
        matches!(self, DbStatus::Connected)
    }
}

/// Configuración efectiva de la conexión, resuelta al arrancar.
///
/// Contiene la contraseña en claro en memoria: nunca se serializa ni se
/// registra en logs. En disco solo vive cifrada con DPAPI.
#[derive(Clone)]
pub struct DbConfig {
    pub mode: DbMode,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub connect_timeout: Duration,
    pub fbclient: PathBuf,
    pub embedded_path: PathBuf,
}

impl DbConfig {
    /// Configuración local (desarrollo, tests y transición).
    pub fn embedded(db_path: impl Into<PathBuf>, fbclient: impl Into<PathBuf>) -> Self {
        Self {
            mode: DbMode::Embedded,
            host: String::new(),
            port: DEFAULT_PORT,
            database: String::new(),
            user: "SYSDBA".to_string(),
            password: String::new(),
            connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS),
            fbclient: fbclient.into(),
            embedded_path: db_path.into(),
        }
    }

    pub fn is_remote(&self) -> bool {
        self.mode == DbMode::Remote
    }

    /// Un remoto sin host o sin usuario no se puede usar todavía.
    pub fn is_configured(&self) -> bool {
        match self.mode {
            DbMode::Embedded => true,
            DbMode::Remote => !self.host.trim().is_empty() && !self.user.trim().is_empty(),
        }
    }

    /// Descripción corta del destino, para la UI y los mensajes de error.
    pub fn target(&self) -> String {
        match self.mode {
            DbMode::Embedded => format!("Embedded · {}", self.embedded_path.display()),
            DbMode::Remote if self.host.trim().is_empty() => "(servidor sin configurar)".into(),
            DbMode::Remote => format!("{}/{} · {}", self.host.trim(), self.port, self.database),
        }
    }
}

/// Contenido del archivo `db-connection.json`. La contraseña va cifrada.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionFile {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(default = "default_user")]
    pub user: String,
    /// `enc:v1:<base64>` (DPAPI) o vacío.
    #[serde(default)]
    pub password: String,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_database() -> String {
    DEFAULT_DATABASE.to_string()
}

fn default_user() -> String {
    DEFAULT_USER.to_string()
}

/// Ruta del archivo de configuración de conexión.
pub fn connection_file_path(app_data: &Path) -> PathBuf {
    app_data.join(CONNECTION_FILE)
}

/// Lee el archivo de conexión si existe y es JSON válido.
///
/// Un archivo corrupto se trata como ausente: el usuario puede volver a
/// configurar la conexión desde la UI en lugar de quedar bloqueado.
pub fn load_file(app_data: &Path) -> Option<ConnectionFile> {
    let raw = std::fs::read_to_string(connection_file_path(app_data)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Guarda la configuración cifrando la contraseña con DPAPI.
pub fn save_file(app_data: &Path, cfg: &DbConfig) -> Result<PathBuf, AppError> {
    std::fs::create_dir_all(app_data)
        .map_err(|e| AppError::Internal(format!("No se pudo crear {}: {e}", app_data.display())))?;

    let password = if cfg.password.is_empty() {
        String::new()
    } else {
        crypto::encrypt_local(&cfg.password)?
    };

    let file = ConnectionFile {
        mode: Some(cfg.mode.as_str().to_string()),
        host: cfg.host.trim().to_string(),
        port: cfg.port,
        database: cfg.database.trim().to_string(),
        user: cfg.user.trim().to_string(),
        password,
    };

    let path = connection_file_path(app_data);
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| AppError::Internal(format!("No se pudo serializar la conexión: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| AppError::Internal(format!("No se pudo escribir {}: {e}", path.display())))?;
    Ok(path)
}

/// Valores tomados del entorno. Se separan para poder probar la resolución
/// sin mutar variables de proceso (los tests corren en paralelo).
#[derive(Debug, Clone, Default)]
pub struct EnvOverrides {
    pub mode: Option<String>,
    pub host: Option<String>,
    pub port: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub timeout_ms: Option<String>,
}

impl EnvOverrides {
    pub fn from_env() -> Self {
        let read = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        Self {
            mode: read("ISALAB_DB_MODE"),
            host: read("ISALAB_DB_HOST"),
            port: read("ISALAB_DB_PORT"),
            database: read("ISALAB_DB_NAME"),
            user: read("ISALAB_DB_USER"),
            password: read("ISALAB_DB_PASSWORD"),
            timeout_ms: read("ISALAB_DB_TIMEOUT_MS"),
        }
    }
}

/// Resuelve la configuración efectiva combinando entorno, archivo y defaults.
pub fn resolve(app_data: &Path, embedded_db: &Path, fbclient: &Path) -> Result<DbConfig, AppError> {
    resolve_parts(
        EnvOverrides::from_env(),
        load_file(app_data),
        embedded_db,
        fbclient,
    )
}

/// Núcleo de [`resolve`], sin acceso a entorno ni disco (testeable).
pub fn resolve_parts(
    env: EnvOverrides,
    file: Option<ConnectionFile>,
    embedded_db: &Path,
    fbclient: &Path,
) -> Result<DbConfig, AppError> {
    let timeout_ms = env
        .timeout_ms
        .as_deref()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v >= 500)
        .unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS);

    // Una variable de entorno de host implica modo remoto aunque no se defina
    // ISALAB_DB_MODE (es la forma cómoda de apuntar a un servidor de pruebas).
    let explicit_mode = env.mode.as_deref().and_then(DbMode::parse).or_else(|| {
        file.as_ref()
            .and_then(|f| f.mode.as_deref())
            .and_then(DbMode::parse)
    });
    let mode = explicit_mode.unwrap_or_else(|| {
        if env.host.is_some() {
            DbMode::Remote
        } else if cfg!(debug_assertions) || embedded_db.exists() {
            DbMode::Embedded
        } else {
            DbMode::Remote
        }
    });

    if mode == DbMode::Embedded {
        return Ok(DbConfig::embedded(embedded_db, fbclient));
    }

    let file = file.unwrap_or_default();
    let host = env.host.unwrap_or(file.host).trim().to_string();
    let port = env
        .port
        .as_deref()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(file.port);
    let database = env
        .database
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(file.database);
    let user = env
        .user
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(file.user);

    // La contraseña del entorno es texto plano (CI/soporte); la del archivo
    // viene cifrada y hay que descifrarla con DPAPI del usuario actual.
    let password = match env.password {
        Some(plain) => plain,
        None => decode_password(&file.password)?,
    };

    Ok(DbConfig {
        mode: DbMode::Remote,
        host,
        port: if port == 0 { DEFAULT_PORT } else { port },
        database: if database.trim().is_empty() {
            DEFAULT_DATABASE.to_string()
        } else {
            database
        },
        user: if user.trim().is_empty() {
            DEFAULT_USER.to_string()
        } else {
            user
        },
        password,
        connect_timeout: Duration::from_millis(timeout_ms),
        fbclient: fbclient.to_path_buf(),
        embedded_path: embedded_db.to_path_buf(),
    })
}

/// Descifra la contraseña almacenada. Acepta texto plano solo por
/// compatibilidad con archivos escritos a mano (se re-cifra al guardar).
fn decode_password(stored: &str) -> Result<String, AppError> {
    let stored = stored.trim();
    if stored.is_empty() {
        return Ok(String::new());
    }
    if !crypto::is_encrypted(stored) {
        return Ok(stored.to_string());
    }
    // El archivo de conexión se cifra con DPAPI del equipo. Si por error se
    // guardó con el formato compartido (`enc:v2:`), no se puede derivar la
    // clave antes de abrir la base: se pide reconfigurar la conexión.
    if crypto::is_local_encrypted(stored) {
        return crypto::decrypt_local(stored)?.ok_or_else(|| {
            AppError::Internal("La contraseña de la conexión no se pudo descifrar".into())
        });
    }
    Err(AppError::Internal(
        "La contraseña de la conexión usa un formato no válido para este archivo".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fbclient() -> PathBuf {
        PathBuf::from("binaries/firebird/fbclient.dll")
    }

    fn embedded_db() -> PathBuf {
        PathBuf::from("app/isalab.fdb")
    }

    fn remote_file() -> ConnectionFile {
        ConnectionFile {
            mode: Some("remote".into()),
            host: "192.168.1.10".into(),
            port: 3050,
            database: "ISALAB".into(),
            user: "ISALAB_APP".into(),
            password: String::new(),
        }
    }

    #[test]
    fn parses_mode_aliases() {
        assert_eq!(DbMode::parse("REMOTE"), Some(DbMode::Remote));
        assert_eq!(DbMode::parse("server"), Some(DbMode::Remote));
        assert_eq!(DbMode::parse("embedded"), Some(DbMode::Embedded));
        assert_eq!(DbMode::parse("otro"), None);
    }

    #[test]
    fn uses_the_file_when_no_environment_is_set() {
        let cfg = resolve_parts(
            EnvOverrides::default(),
            Some(remote_file()),
            &embedded_db(),
            &fbclient(),
        )
        .unwrap();
        assert_eq!(cfg.mode, DbMode::Remote);
        assert_eq!(cfg.host, "192.168.1.10");
        assert_eq!(cfg.target(), "192.168.1.10/3050 · ISALAB");
        assert!(cfg.is_configured());
    }

    #[test]
    fn environment_overrides_the_file() {
        let env = EnvOverrides {
            host: Some("192.168.1.50".into()),
            port: Some("3051".into()),
            user: Some("OTRO".into()),
            password: Some("clave".into()),
            ..Default::default()
        };
        let cfg = resolve_parts(env, Some(remote_file()), &embedded_db(), &fbclient()).unwrap();
        assert_eq!(cfg.host, "192.168.1.50");
        assert_eq!(cfg.port, 3051);
        assert_eq!(cfg.user, "OTRO");
        assert_eq!(cfg.password, "clave");
    }

    #[test]
    fn a_host_in_the_environment_implies_remote_mode() {
        let env = EnvOverrides {
            host: Some("10.0.0.5".into()),
            ..Default::default()
        };
        let cfg = resolve_parts(env, None, &embedded_db(), &fbclient()).unwrap();
        assert_eq!(cfg.mode, DbMode::Remote);
        assert_eq!(cfg.database, DEFAULT_DATABASE);
        assert_eq!(cfg.user, DEFAULT_USER);
    }

    #[test]
    fn embedded_mode_ignores_remote_fields() {
        let env = EnvOverrides {
            mode: Some("embedded".into()),
            host: Some("10.0.0.5".into()),
            ..Default::default()
        };
        let cfg = resolve_parts(env, Some(remote_file()), &embedded_db(), &fbclient()).unwrap();
        assert_eq!(cfg.mode, DbMode::Embedded);
        assert_eq!(cfg.user, "SYSDBA");
        assert_eq!(cfg.target(), "Embedded · app/isalab.fdb");
    }

    #[test]
    fn remote_without_host_is_not_configured() {
        let env = EnvOverrides {
            mode: Some("remote".into()),
            ..Default::default()
        };
        let cfg = resolve_parts(env, None, &embedded_db(), &fbclient()).unwrap();
        assert!(!cfg.is_configured());
        assert_eq!(cfg.target(), "(servidor sin configurar)");
    }

    #[test]
    fn clamps_invalid_environment_values() {
        let env = EnvOverrides {
            mode: Some("remote".into()),
            host: Some("  10.0.0.9  ".into()),
            port: Some("70000".into()),
            timeout_ms: Some("10".into()),
            ..Default::default()
        };
        let cfg = resolve_parts(env, Some(remote_file()), &embedded_db(), &fbclient()).unwrap();
        assert_eq!(cfg.host, "10.0.0.9");
        assert_eq!(cfg.port, DEFAULT_PORT); // puerto inválido → default
        assert_eq!(
            cfg.connect_timeout.as_millis(),
            DEFAULT_CONNECT_TIMEOUT_MS as u128
        );
    }

    #[test]
    fn plaintext_password_is_accepted_for_hand_written_files() {
        let mut file = remote_file();
        file.password = "directa".into();
        let cfg = resolve_parts(
            EnvOverrides::default(),
            Some(file),
            &embedded_db(),
            &fbclient(),
        )
        .unwrap();
        assert_eq!(cfg.password, "directa");
    }

    #[test]
    fn connection_file_roundtrip_encrypts_the_password() {
        let dir = std::env::temp_dir().join(format!(
            "isalab_cfg_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let cfg = DbConfig {
            mode: DbMode::Remote,
            host: "192.168.1.10".into(),
            port: 3050,
            database: "ISALAB".into(),
            user: "ISALAB_APP".into(),
            password: "secreta".into(),
            connect_timeout: Duration::from_secs(5),
            fbclient: fbclient(),
            embedded_path: embedded_db(),
        };

        let path = save_file(&dir, &cfg).expect("guardar conexión");
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            !raw.contains("secreta"),
            "la contraseña no debe ir en claro"
        );

        let loaded = load_file(&dir).expect("leer conexión");
        let resolved = resolve_parts(
            EnvOverrides::default(),
            Some(loaded),
            &embedded_db(),
            &fbclient(),
        )
        .unwrap();
        assert_eq!(resolved.host, cfg.host);
        assert_eq!(resolved.password, "secreta");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
