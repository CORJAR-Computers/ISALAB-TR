//! Helper de testing: crea una base de datos temporal con Firebird Embedded
//! para tests de integración. Se ejecuta solo con `#[cfg(test)]`.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db::config::DbConfig;
use crate::db::{connect, create_database, migrations};
use crate::error::AppError;

// ── Resolución de fbclient.dll ─────────────────────────────────────────────
//
// Orden de búsqueda (el primero que exista gana):
//
//   1. Variable de entorno FIREBIRD_DIR  (establecida por .cargo/config.toml
//      o manualmente en CI).
//   2. binaries/firebird/fbclient.dll   (ruta bundled del proyecto).
//   3. ../Firebird-5.0.3.1683-0-windows-x64/ (Firebird incluido en el repo).
//
// Además, el directorio resuelto se añade al PATH del proceso para que
// Firebird pueda encontrar plugins/, firebird.msg y las ICU DLLs.

/// Resuelve la ruta al directorio de Firebird y al fbclient.dll.
/// Inyecta el directorio al PATH del proceso para que Firebird Embedded
/// encuentre sus dependencias (plugins/, firebird.msg, ICU dlls, etc.).
fn resolve_firebird_dir() -> PathBuf {
    // Candidatos en orden de prioridad.
    let candidates: &[PathBuf] = &[
        // 1. FIREBIRD_DIR env var (seteada por .cargo/config.toml o CI)
        std::env::var("FIREBIRD_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_default(),
        // 2. Bundled dentro de src-tauri/binaries/firebird/
        PathBuf::from("binaries/firebird"),
        // 3. Firebird incluido en la raíz del repo (un nivel arriba de src-tauri/)
        PathBuf::from("../Firebird-5.0.3.1683-0-windows-x64"),
    ];

    for dir in candidates {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let dll = dir.join("fbclient.dll");
        if dll.exists() {
            // Inyectar al PATH del proceso para que Firebird encuentre
            // plugins/, firebird.msg e ICU en tiempo de ejecución.
            inject_to_path(dir);
            return dir.clone();
        }
    }

    panic!(
        "fbclient.dll no encontrado. Intenté:\n\
         - $FIREBIRD_DIR\n\
         - binaries/firebird/fbclient.dll\n\
         - ../Firebird-5.0.3.1683-0-windows-x64/fbclient.dll\n\
         \n\
         Opciones:\n\
         1. Ejecuta tests desde src-tauri/ (cd src-tauri && cargo test)\n\
         2. Define FIREBIRD_DIR apuntando al directorio de Firebird 5\n\
         3. Copia fbclient.dll y plugins/ a src-tauri/binaries/firebird/"
    );
}

/// Añade `dir` al PATH del proceso actual (idempotente).
fn inject_to_path(dir: &Path) {
    let abs = match dir.canonicalize() {
        Ok(p) => p,
        Err(_) => dir.to_path_buf(),
    };
    let abs_str = abs.to_string_lossy().to_string();

    let current_path = std::env::var("PATH").unwrap_or_default();

    // No añadir si ya está en el PATH (evita duplicados en tests paralelos).
    if !current_path
        .split(';')
        .any(|p| p.eq_ignore_ascii_case(&abs_str))
    {
        let new_path = format!("{};{}", abs_str, current_path);
        // SAFETY: solo se llama desde tests (single-process).
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
    }
}

/// Ruta al fbclient.dll resuelto.
pub fn fbclient_path() -> PathBuf {
    resolve_firebird_dir().join("fbclient.dll")
}

/// Ruta única para una base temporal de tests.
fn temp_db_path(kind: &str) -> PathBuf {
    let thread_id = std::thread::current().id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("isalab_{kind}_{thread_id:?}_{timestamp}.fdb"))
}

/// Conexión de prueba: cierra el attachment y borra el `.fdb` al soltarse.
///
/// Antes cada test llamaba a [`cleanup_test_db`] mientras su propia conexión
/// seguía abierta. En Windows el archivo está bloqueado por el motor, así que
/// `remove_file` fallaba en silencio y cada pasada dejaba ~436 `.fdb` en
/// `%TEMP%`. Envolver la conexión permite cerrarla **antes** de borrar, en el
/// mismo punto y sin depender de que cada test se acuerde del orden.
///
/// Se comporta como una `SimpleConnection` (`Deref`/`DerefMut`), así que
/// `conn.execute(..)`, `repo::fn(&mut conn, ..)` y `drop(conn)` siguen igual.
pub struct TestDb {
    // `Option` para poder cerrar la conexión en `drop` antes de tocar el disco.
    conn: Option<SimpleConnection>,
    path: PathBuf,
}

impl TestDb {
    fn new(conn: SimpleConnection, path: PathBuf) -> Self {
        Self {
            conn: Some(conn),
            path,
        }
    }

    /// Ruta del `.fdb` temporal (útil para reabrirlo o inspeccionarlo).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for TestDb {
    type Target = SimpleConnection;

    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("conexión de prueba ya cerrada")
    }
}

impl DerefMut for TestDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("conexión de prueba ya cerrada")
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // El orden importa: primero el attachment (libera el handle del .fdb),
        // después el borrado. Al revés nunca funciona en Windows.
        drop(self.conn.take());
        remove_db_files(&self.path);
    }
}

/// Borra el `.fdb` y sus auxiliares, reintentando brevemente.
///
/// Cerrar un attachment embedded no es instantáneo: el motor suelta el archivo
/// unos milisegundos después. Un único `remove_file` se ejecuta demasiado
/// pronto y deja el fichero huérfano, así que se insiste durante ~1 s. Si otro
/// test aún tiene una conexión abierta al mismo archivo, se rinde sin ruido:
/// el siguiente intento del guard (o el SO al cerrar el proceso) lo libera.
fn remove_db_files(db_path: &Path) {
    let aux = [
        db_path.to_path_buf(),
        db_path.with_extension("fdb.tmp"),
        db_path.with_extension("fdb.lock"),
    ];

    for path in aux {
        for _ in 0..40 {
            match std::fs::remove_file(&path) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
        }
    }
}

/// Crea una base de datos temporal vacía (sin migraciones) con nombre único.
///
/// La usan los tests del runner: necesitan partir de cero para comprobar el
/// registro de versiones y el rollback de una migración fallida.
pub fn setup_empty_test_db() -> (TestDb, PathBuf) {
    let db_path = temp_db_path("empty");
    let _ = std::fs::remove_file(&db_path);

    let fbclient = fbclient_path();
    if !fbclient.exists() {
        panic!(
            "fbclient.dll no encontrado en {}. Los tests de integración requieren Firebird 5 Embedded.",
            fbclient.display()
        );
    }

    let conn =
        create_database(&db_path, &fbclient).expect("No se pudo crear la base de datos de prueba");
    (TestDb::new(conn, db_path.clone()), db_path)
}

/// Molde de base de datos: se crea y migra **una vez por proceso**.
///
/// Antes cada test creaba una base desde cero y ejecutaba las 36 migraciones
/// (~2 s por test, ~660 s la suite completa). Ahora todas las pruebas parten
/// de una copia de este archivo ya migrado y limpio: la copia en disco cuesta
/// milisegundos y cada test sigue teniendo su base aislada.
static TEMPLATE_DB: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// Deja la base sin datos de demostración y con los generadores a cero, que es
/// el estado del que parten los tests de repositorio.
fn limpiar_para_tests(conn: &mut SimpleConnection) {
    let cleanup_statements = [
        "DELETE FROM CASH_SESSION_TOTALS",
        "DELETE FROM CASH_COUNT_LINES",
        "DELETE FROM CASH_MOVEMENTS",
        "DELETE FROM PAYMENT_ALLOCATIONS",
        "DELETE FROM PAYMENT_LINES",
        "DELETE FROM PAYMENTS",
        "DELETE FROM CASH_SESSIONS",
        "DELETE FROM INVOICE_ITEMS",
        "DELETE FROM INVOICES",
        "DELETE FROM SURGERIES",
        "DELETE FROM LAB_RESULTS",
        "DELETE FROM SAMPLES",
        "DELETE FROM CONSULTATIONS",
        "DELETE FROM VACCINES",
        "DELETE FROM PATIENTS",
        "DELETE FROM OWNERS",
    ];
    for stmt in cleanup_statements {
        conn.execute(stmt, ()).ok();
    }

    // Reiniciar generadores para tests limpios
    // Para USERS, necesitamos reiniciar al max ID existente + 1
    let max_user_id: Option<(i32,)> = conn
        .query_first("SELECT MAX(ID) FROM USERS", ())
        .ok()
        .flatten();
    let user_gen_value = max_user_id.map(|(id,)| id).unwrap_or(0);

    let reset_generators = [
        "SET GENERATOR GEN_OWNERS_ID TO 0",
        "SET GENERATOR GEN_PATIENTS_ID TO 0",
        "SET GENERATOR GEN_PATIENT_CODE_SEQ TO 0",
        "SET GENERATOR GEN_CONSULTATIONS_ID TO 0",
        "SET GENERATOR GEN_SAMPLES_ID TO 0",
        "SET GENERATOR GEN_LAB_RESULTS_ID TO 0",
        "SET GENERATOR GEN_SURGERIES_ID TO 0",
        "SET GENERATOR GEN_INVOICES_ID TO 0",
        "SET GENERATOR GEN_INVOICE_ITEMS_ID TO 0",
    ];
    for stmt in &reset_generators {
        conn.execute(stmt, ()).ok();
    }

    // Reset USERS generator to max ID to avoid conflicts with seed data
    conn.execute(
        &format!("SET GENERATOR GEN_USERS_ID TO {}", user_gen_value),
        (),
    )
    .ok();
}

/// Construye el molde: base nueva, migrada y limpia, con la conexión cerrada
/// para que el archivo se pueda copiar.
fn crear_molde() -> Result<PathBuf, String> {
    let db_path = temp_db_path("template");
    let _ = std::fs::remove_file(&db_path);

    let fbclient = fbclient_path();
    if !fbclient.exists() {
        return Err(format!(
            "fbclient.dll no encontrado en {}",
            fbclient.display()
        ));
    }

    let mut conn = create_database(&db_path, &fbclient)
        .map_err(|e| format!("no se pudo crear la base molde: {e}"))?;
    migrations::run_migrations(&mut conn)
        .map_err(|e| format!("las migraciones del molde fallaron: {e}"))?;
    limpiar_para_tests(&mut conn);
    // Firebird vuelca y suelta el archivo al soltar la conexión.
    drop(conn);

    Ok(db_path)
}

/// Crea una base de datos temporal para tests con nombre único.
///
/// Es una copia del molde migrado: mismo esquema y mismos generadores que la
/// creación desde cero, sin repetir las migraciones en cada prueba. La conexión
/// va envuelta en [`TestDb`]: al soltarla se cierra y se borra el archivo.
pub fn setup_test_db() -> (TestDb, PathBuf) {
    let template = TEMPLATE_DB
        .get_or_init(crear_molde)
        .as_ref()
        .unwrap_or_else(|e| panic!("No se pudo preparar la base molde de los tests: {e}"))
        .clone();

    let db_path = temp_db_path("test");
    let _ = std::fs::remove_file(&db_path);
    std::fs::copy(&template, &db_path).unwrap_or_else(|e| {
        panic!(
            "No se pudo copiar la base molde {} a {}: {e}",
            template.display(),
            db_path.display()
        )
    });

    let conn =
        test_connection(&db_path).expect("No se pudo abrir la copia de la base de datos de prueba");
    (TestDb::new(conn, db_path.clone()), db_path)
}

/// Plazo para abrir una base de test.
///
/// El de producción (5 s, [`crate::db::config::DEFAULT_CONNECT_TIMEOUT_MS`]) se
/// queda corto con la suite completa: 435 tests abren bases en paralelo y el
/// motor embedded, bajo esa carga, tarda más en arrancar. Un fallo aquí no
/// describe un problema de la app sino del entorno del test, así que el plazo es
/// propio, holgado y acotado: si de verdad no hay motor, el test falla en 20 s,
/// no se queda esperando.
const TEST_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// Intentos de conexión de un test ante un error inmediato (no un timeout).
const TEST_CONNECT_ATTEMPTS: u32 = 3;

/// Crea una nueva conexión a la DB de prueba existente.
///
/// Usa un timeout más generoso que la app y reintenta hasta
/// [`TEST_CONNECT_ATTEMPTS`] veces los errores inmediatos (por ejemplo, una
/// copia del molde que otro test acaba de liberar). Un timeout no se reintenta:
/// cada intento deja un hilo todavía vivo que podría abrir el archivo *después*
/// de que el guard lo haya borrado.
pub fn test_connection(db_path: &Path) -> Result<SimpleConnection, AppError> {
    let fbclient = fbclient_path();
    let mut cfg = DbConfig::embedded(db_path, &fbclient);
    cfg.connect_timeout = TEST_CONNECT_TIMEOUT;

    let mut last_error = None;
    for attempt in 1..=TEST_CONNECT_ATTEMPTS {
        let started = Instant::now();
        match connect(&cfg) {
            Ok(conn) => return Ok(conn),
            Err(err) => {
                let timed_out = started.elapsed() >= TEST_CONNECT_TIMEOUT;
                last_error = Some(err);
                if timed_out || attempt == TEST_CONNECT_ATTEMPTS {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100 * u64::from(attempt)));
            }
        }
    }

    Err(last_error.expect("TEST_CONNECT_ATTEMPTS debe ser al menos 1"))
}

/// Borra la base de datos de prueba de un camino sin guard.
///
/// Es best-effort y **no** reintenta: si la conexión sigue abierta el archivo
/// está bloqueado y no se puede borrar ahora. La limpieza real la hace el
/// `Drop` de [`TestDb`] al cerrar la conexión; esta función queda para rutas
/// sueltas que no pasan por el guard.
pub fn cleanup_test_db(db_path: &Path) {
    let _ = std::fs::remove_file(db_path);
    // También eliminar archivos auxiliares de Firebird
    let _ = std::fs::remove_file(db_path.with_extension("fdb.tmp"));
    let _ = std::fs::remove_file(db_path.with_extension("fdb.lock"));
}

/// Fixture: inserta un paciente de prueba y devuelve su ID.
pub fn insert_test_patient(conn: &mut SimpleConnection) -> i32 {
    // Insertar especie
    conn.execute(
        "INSERT INTO SPECIES (ID, CODE, NAME) VALUES (1, 'CAN', 'Canino')",
        (),
    )
    .ok();

    // Insertar raza
    conn.execute(
        "INSERT INTO BREEDS (ID, SPECIES_ID, NAME) VALUES (1, 1, 'Beagle')",
        (),
    )
    .ok();

    // Insertar propietario
    conn.execute(
        "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE)
         VALUES (1, 'CC', '1234567890', 'Juan Pérez', '+57 300 1234567')",
        (),
    )
    .ok();

    // Insertar paciente
    conn.execute(
        "INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE, ACTIVE)
         VALUES (1, 1, 1, 1, 'Luna', 'F', '2023-06-15', TRUE)",
        (),
    )
    .ok();

    1
}

/// Fixture: inserta un tipo de muestra.
pub fn insert_test_sample_type(conn: &mut SimpleConnection) {
    conn.execute(
        "INSERT INTO SAMPLE_TYPES (ID, CODE, NAME) VALUES (1, 'SANGRE', 'Sangre total (EDTA)')",
        (),
    )
    .ok();
}

/// Fixture: inserta un analito.
pub fn insert_test_analyte(conn: &mut SimpleConnection) {
    conn.execute(
        "INSERT INTO ANALYTES (ID, CODE, NAME, UNIT, METHOD)
         VALUES (1, 'HCT', 'Hematocrito', '%', 'Microhematocrito')",
        (),
    )
    .ok();
}

/// Fixture: inserta un rango de referencia.
pub fn insert_test_reference_range(conn: &mut SimpleConnection) {
    conn.execute(
        "INSERT INTO REFERENCE_RANGES
         (ID, ANALYTE_ID, SPECIES_ID, SEX, AGE_MIN_MONTHS, AGE_MAX_MONTHS, MIN_VALUE, MAX_VALUE)
         VALUES (1, 1, 1, NULL, 0, 2400, 37.0, 55.0)",
        (),
    )
    .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El `.fdb` de un test no debe sobrevivir a la conexión: si el guard deja
    /// el archivo, cada pasada acumula cientos de ficheros en `%TEMP%`.
    #[test]
    fn el_guard_borra_el_fichero_al_cerrar_la_conexion() {
        let (mut conn, db_path) = setup_test_db();
        assert!(db_path.exists(), "la copia del molde debe existir");
        conn.execute("SELECT 1 FROM RDB$DATABASE", ()).ok();

        drop(conn);

        assert!(
            !db_path.exists(),
            "el .fdb debe borrarse al soltar el guard: {}",
            db_path.display()
        );
    }

    #[test]
    fn la_conexion_de_test_usa_un_timeout_propio_mas_holgado() {
        let (conn, db_path) = setup_test_db();
        let fbclient = fbclient_path();
        let mut cfg = DbConfig::embedded(&db_path, &fbclient);
        cfg.connect_timeout = TEST_CONNECT_TIMEOUT;
        assert_eq!(cfg.connect_timeout, Duration::from_secs(20));
        drop(conn);
    }
}
