pub mod events;
pub mod migrations;
pub mod seed;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Conexiones de uso general para los comandos.
const POOL_SIZE: usize = 4;

/// Pool simple de conexiones Firebird Embedded.
///
/// Firebird Embedded admite varias conexiones por proceso; el pool las
/// reutiliza. `SimpleConnection` es `Send`, por lo que puede moverse entre
/// hilos (comandos de Tauri).
#[derive(Clone)]
pub struct DbPool(Arc<Mutex<VecDeque<SimpleConnection>>>);

impl DbPool {
    pub fn empty() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::new())))
    }

    pub fn acquire(&self) -> Result<PooledConn, AppError> {
        let mut q = self
            .0
            .lock()
            .map_err(|_| AppError::Db("Pool de conexiones bloqueado".into()))?;
        let conn = q.pop_front().ok_or_else(|| {
            AppError::Db("Sin conexiones Firebird disponibles (motor no inicializado)".into())
        })?;
        Ok(PooledConn {
            pool: self.clone(),
            conn: Some(conn),
        })
    }

    fn release(&self, conn: SimpleConnection) {
        if let Ok(mut q) = self.0.lock() {
            if q.len() < POOL_SIZE {
                q.push_back(conn);
            }
        }
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

/// Abre una conexión Embedded a una base existente.
pub fn new_connection(db_path: &Path, fbclient: &Path) -> Result<SimpleConnection, AppError> {
    let mut b = rsfbclient::builder_native()
        .with_dyn_load(fbclient.to_string_lossy().to_string())
        .with_embedded();
    b.user("SYSDBA");
    b.db_name(normalize_db_path(db_path));
    let conn = b.connect().map_err(|e| {
        AppError::Db(format!(
            "No se pudo conectar a Firebird Embedded ({}): {e}",
            db_path.display()
        ))
    })?;
    Ok(conn.into())
}

/// Crea la base de datos (.fdb) si no existe y devuelve una conexión.
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

/// Arranque: crea la DB si falta, ejecuta migraciones pendientes y monta el pool.
pub fn bootstrap(db_path: &Path, fbclient: &Path) -> Result<(DbPool, i32), AppError> {
    if !fbclient.exists() {
        return Err(AppError::Db(format!(
            "fbclient.dll no encontrado en {}. Descarga Firebird 5 Embedded y coloca la librería ahí (ver README).",
            fbclient.display()
        )));
    }

    let exists = db_path.exists();
    let mut first = if exists {
        new_connection(db_path, fbclient)?
    } else {
        create_database(db_path, fbclient)?
    };

    let schema_version = migrations::run_migrations(&mut first)?;

    // Primer arranque: contraseña por defecto del admin (admin123).
    seed::ensure_default_admin(&mut first)?;

    let pool = DbPool(Arc::new(Mutex::new(VecDeque::new())));
    pool.release(first);
    for _ in 1..POOL_SIZE {
        match new_connection(db_path, fbclient) {
            Ok(c) => pool.release(c),
            Err(_) => break,
        }
    }

    Ok((pool, schema_version))
}
