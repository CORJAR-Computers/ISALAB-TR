//! Acceso a Firebird: configuración, pool de conexiones y arranque.
//!
//! En modo remoto la app es solo un cliente: no crea la base, no aplica
//! migraciones y no siembra datos. Eso lo hace el runner administrativo.

pub mod config;
pub mod events;
pub mod grants;
pub mod health;
pub mod maintenance;
pub mod migrations;
pub mod seed;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rsfbclient::prelude::*;
use rsfbclient::{FbError, SimpleConnection};

use crate::error::AppError;
use config::{DbConfig, DbMode};

/// Conexiones de uso general para los comandos.
pub const POOL_SIZE: usize = 4;

/// Espera máxima por una conexión libre antes de devolver error.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Una conexión ociosa se revalida si lleva más de este tiempo sin usarse.
///
/// Cubre el caso más común de caída: el servidor se reinicia mientras la app
/// está inactiva. Revalidar solo las conexiones frías evita un viaje de red
/// extra en cada comando.
const IDLE_REVALIDATE_AFTER: Duration = Duration::from_secs(2);

struct IdleConn {
    conn: SimpleConnection,
    released_at: Instant,
}

#[derive(Default)]
struct PoolInner {
    idle: VecDeque<IdleConn>,
    /// Conexiones abiertas: ociosas + en uso.
    open: usize,
    config: Option<DbConfig>,
    /// Fuerza la revalidación de las conexiones ociosas en el próximo acquire.
    stale: bool,
}

/// Pool de conexiones Firebird, válido para embedded y remoto.
#[derive(Clone)]
pub struct DbPool {
    inner: Arc<(Mutex<PoolInner>, Condvar)>,
}

impl DbPool {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new((Mutex::new(PoolInner::default()), Condvar::new())),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, PoolInner>, AppError> {
        self.inner
            .0
            .lock()
            .map_err(|_| AppError::Db("Pool de conexiones bloqueado".into()))
    }

    /// Configuración con la que se abren las conexiones nuevas.
    pub fn config(&self) -> Option<DbConfig> {
        self.lock().ok().and_then(|inner| inner.config.clone())
    }

    /// Toma una conexión del pool, abriendo una nueva si hay cupo.
    ///
    /// Si todas las conexiones están en uso espera hasta [`ACQUIRE_TIMEOUT`] en
    /// lugar de fallar al instante. Las conexiones ociosas que llevan tiempo
    /// sin usarse se validan antes de entregarlas, para no devolver una
    /// conexión muerta tras un reinicio del servidor.
    pub fn acquire(&self) -> Result<PooledConn, AppError> {
        let (_, cvar) = &*self.inner;
        let deadline = Instant::now() + ACQUIRE_TIMEOUT;

        loop {
            let mut inner = self.lock()?;

            if let Some(idle) = inner.idle.pop_front() {
                let revalidate = inner.stale || idle.released_at.elapsed() >= IDLE_REVALIDATE_AFTER;
                inner.stale = false;
                drop(inner);

                let mut conn = idle.conn;
                if revalidate && !is_alive(&mut conn) {
                    // Conexión muerta: se descarta y se intenta con otra/una nueva.
                    self.close_slot();
                    continue;
                }
                return Ok(self.wrap(conn));
            }

            let Some(cfg) = inner.config.clone() else {
                return Err(AppError::Db(
                    "Sin conexión Firebird disponible: la app no pudo inicializar la base.".into(),
                ));
            };

            if inner.open < POOL_SIZE {
                inner.open += 1;
                drop(inner);
                return match connect(&cfg) {
                    Ok(conn) => Ok(self.wrap(conn)),
                    Err(e) => {
                        self.close_slot();
                        Err(e)
                    }
                };
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AppError::Db(
                    "Tiempo de espera agotado esperando una conexión Firebird libre".into(),
                ));
            }
            let (_guard, _timeout) = cvar
                .wait_timeout(inner, remaining)
                .map_err(|_| AppError::Db("Pool de conexiones bloqueado".into()))?;
            // Se vuelve a intentar: otra conexión pudo liberarse o expiró el plazo.
        }
    }

    /// Descarta las conexiones ociosas y obliga a revalidar las que están en uso.
    ///
    /// Se llama cuando un ping falla o cuando cambia la configuración: al
    /// volver el servidor el pool se repuebla solo.
    pub fn invalidate(&self) {
        if let Ok(mut inner) = self.lock() {
            let dropped = inner.idle.len();
            inner.idle.clear();
            inner.open = inner.open.saturating_sub(dropped);
            inner.stale = true;
            self.inner.1.notify_all();
        }
    }

    fn wrap(&self, conn: SimpleConnection) -> PooledConn {
        PooledConn {
            pool: self.clone(),
            conn: Some(conn),
        }
    }

    fn add(&self, conn: SimpleConnection) {
        if let Ok(mut inner) = self.lock() {
            inner.open += 1;
        }
        self.release(conn);
    }

    fn close_slot(&self) {
        if let Ok(mut inner) = self.lock() {
            inner.open = inner.open.saturating_sub(1);
        }
        self.inner.1.notify_one();
    }

    fn release(&self, conn: SimpleConnection) {
        if let Ok(mut inner) = self.lock() {
            if inner.config.is_some() && inner.idle.len() < POOL_SIZE {
                inner.idle.push_back(IdleConn {
                    conn,
                    released_at: Instant::now(),
                });
                self.inner.1.notify_one();
                return;
            }
            inner.open = inner.open.saturating_sub(1);
        }
        // Si no se guardó, la conexión se cierra al salir de este ámbito.
        self.inner.1.notify_one();
    }
}

/// Guard que devuelve la conexión al pool al soltarse.
pub struct PooledConn {
    pool: DbPool,
    conn: Option<SimpleConnection>,
}

impl PooledConn {
    pub fn conn(&mut self) -> &mut SimpleConnection {
        self.conn.as_mut().expect("conexión ya liberada")
    }
}

impl Drop for PooledConn {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            self.pool.release(conn);
        }
    }
}

/// Firebird en Windows acepta barras normales en rutas.
fn normalize_db_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Comprueba que una conexión sigue viva con la consulta más barata posible.
fn is_alive(conn: &mut SimpleConnection) -> bool {
    let probe: Result<Option<(i16,)>, FbError> = conn.query_first("SELECT 1 FROM RDB$DATABASE", ());
    probe.is_ok()
}

/// Abre una conexión síncrona según la configuración (sin timeout propio).
fn connect_blocking(cfg: &DbConfig) -> Result<SimpleConnection, FbError> {
    let builder =
        rsfbclient::builder_native().with_dyn_load(cfg.fbclient.to_string_lossy().to_string());

    match cfg.mode {
        DbMode::Embedded => {
            let mut b = builder.with_embedded();
            b.user("SYSDBA");
            b.db_name(normalize_db_path(&cfg.embedded_path));
            b.connect().map(Into::into)
        }
        DbMode::Remote => {
            let mut b = builder.with_remote();
            b.user(cfg.user.clone());
            b.pass(cfg.password.clone());
            b.host(cfg.host.clone());
            b.port(cfg.port);
            b.db_name(cfg.database.clone());
            b.connect().map(Into::into)
        }
    }
}

/// Abre una conexión respetando `connect_timeout`.
///
/// `rsfbclient` no expone un timeout de conexión, así que la apertura se hace
/// en un hilo aparte y se espera como máximo ese plazo. El hilo abandonado
/// termina solo cuando el sistema operativo corta el intento.
pub fn connect(cfg: &DbConfig) -> Result<SimpleConnection, AppError> {
    let timeout = cfg.connect_timeout;
    let (tx, rx) = mpsc::channel();
    let worker = cfg.clone();
    thread::spawn(move || {
        let _ = tx.send(connect_blocking(&worker));
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(conn)) => Ok(conn),
        Ok(Err(e)) => Err(AppError::Db(format!(
            "No se pudo conectar con Firebird ({}): {e}",
            cfg.target()
        ))),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(AppError::Db(format!(
            "Tiempo de espera agotado al conectar con Firebird ({}). El servidor no responde.",
            cfg.target()
        ))),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(AppError::Db(format!(
            "El intento de conexión con Firebird ({}) terminó de forma inesperada",
            cfg.target()
        ))),
    }
}

/// Abre una conexión Embedded a una base existente.
///
/// Se conserva para tests y herramientas locales; la app usa [`connect`].
pub fn new_connection(db_path: &Path, fbclient: &Path) -> Result<SimpleConnection, AppError> {
    connect(&DbConfig::embedded(db_path, fbclient))
}

/// Crea la base de datos (.fdb) si no existe y devuelve una conexión.
///
/// Solo tiene sentido en modo embedded (desarrollo, tests, herramientas). El
/// servidor crea la base una vez, como SYSDBA.
pub fn create_database(db_path: &Path, fbclient: &Path) -> Result<SimpleConnection, AppError> {
    let mut b = rsfbclient::builder_native()
        .with_dyn_load(fbclient.to_string_lossy().to_string())
        .with_embedded();
    b.user("SYSDBA");
    b.db_name(normalize_db_path(db_path));
    b.page_size(16384);
    let conn = b.create_database().map_err(|e| {
        AppError::Db(format!(
            "No se pudo crear la base de datos ({}): {e}",
            db_path.display()
        ))
    })?;
    Ok(conn.into())
}

/// Arranque de la capa de datos.
///
/// - Embedded: crea la base si falta, aplica migraciones y siembra el admin.
/// - Remoto: solo conecta y verifica el esquema; si está desactualizado,
///   devuelve un error explicativo (el runner es quien migra).
pub fn bootstrap(cfg: &DbConfig) -> Result<(DbPool, i32), AppError> {
    if !cfg.fbclient.exists() {
        return Err(AppError::Db(format!(
            "fbclient.dll no encontrado en {}. Reinstala la aplicación (ver README).",
            cfg.fbclient.display()
        )));
    }

    let mut first = match cfg.mode {
        DbMode::Embedded => {
            if cfg.embedded_path.exists() {
                connect(cfg)?
            } else {
                create_database(&cfg.embedded_path, &cfg.fbclient)?
            }
        }
        DbMode::Remote => {
            if !cfg.is_configured() {
                return Err(AppError::Db(
                    "Falta configurar el servidor Firebird (host y usuario). Use «Configurar servidor»."
                        .into(),
                ));
            }
            connect(cfg)?
        }
    };

    let schema_version = if cfg.mode == DbMode::Embedded {
        let version = migrations::run_migrations(&mut first)?;

        // Primer arranque: contraseña por defecto del admin (admin123).
        seed::ensure_default_admin(&mut first)?;

        // Mantenimiento: poda de la telemetría de eventos (EVENT_LOG > 30 días).
        // Best-effort: un fallo no impide arrancar.
        let _ = maintenance::prune_event_log_conn(&mut first);
        version
    } else {
        // El cliente no crea, migra ni siembra: solo verifica.
        let version = health::schema_version_of(&mut first)?;
        let expected = migrations::expected_version();
        if version != expected {
            return Err(AppError::Db(if version < expected {
                format!(
                    "La base de datos del servidor está en el esquema v{version} y esta versión de la app requiere v{expected}. Ejecute el runner de migraciones en el servidor."
                )
            } else {
                format!(
                    "La base de datos del servidor usa el esquema v{version}, más nuevo que el que entiende esta app (v{expected}). Actualice la aplicación."
                )
            }));
        }
        version
    };

    let pool = DbPool::empty();
    {
        let mut inner = pool.lock()?;
        inner.config = Some(cfg.clone());
    }
    pool.add(first);
    for _ in 1..POOL_SIZE {
        match connect(cfg) {
            Ok(c) => pool.add(c),
            Err(_) => break,
        }
    }

    Ok((pool, schema_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pool_reports_a_clear_error() {
        let pool = DbPool::empty();
        let err = pool.acquire().err().expect("el pool vacío debe fallar");
        assert!(err.to_string().contains("no pudo inicializar"));
    }

    #[test]
    fn connect_reports_a_missing_client_library() {
        let cfg = DbConfig {
            mode: DbMode::Remote,
            host: "127.0.0.1".into(),
            port: 3050,
            database: "ISALAB".into(),
            user: "ISALAB_APP".into(),
            password: "x".into(),
            connect_timeout: Duration::from_millis(500),
            fbclient: Path::new("no-existe/fbclient.dll").to_path_buf(),
            embedded_path: Path::new("no-existe.fdb").to_path_buf(),
        };
        let result = connect(&cfg);
        assert!(result.is_err(), "sin fbclient no se puede conectar");
        let err = result.err().unwrap();
        assert!(err.to_string().contains("No se pudo conectar con Firebird"));
    }

    #[test]
    fn embedded_bootstrap_prepares_the_pool_and_the_schema() {
        use crate::test_helpers::{cleanup_test_db, fbclient_path, setup_test_db};

        // Base ya migrada: bootstrap debe verificar, no recrear, y montar pool.
        let (conn, db_path) = setup_test_db();
        drop(conn);

        let cfg = DbConfig::embedded(&db_path, fbclient_path());
        let (pool, version) = bootstrap(&cfg).expect("bootstrap embedded");
        assert_eq!(version, migrations::expected_version());

        let mut conn = pool.acquire().expect("conexión del pool");
        let row: Option<(i16,)> = conn
            .conn()
            .query_first("SELECT 1 FROM RDB$DATABASE", ())
            .expect("consulta en el pool");
        assert_eq!(row, Some((1,)));
        drop(conn);

        cleanup_test_db(&db_path);
    }
}
