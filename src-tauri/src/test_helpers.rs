//! Helper de testing: crea una base de datos temporal con Firebird Embedded
//! para tests de integración. Se ejecuta solo con `#[cfg(test)]`.

use std::path::{Path, PathBuf};

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db::{create_database, migrations, new_connection};
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
fn fbclient_path() -> PathBuf {
    resolve_firebird_dir().join("fbclient.dll")
}

/// Crea una base de datos temporal para tests con nombre único.
/// Devuelve la conexión y la ruta de la DB.
pub fn setup_test_db() -> (SimpleConnection, PathBuf) {
    let thread_id = std::thread::current().id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let db_path =
        std::env::temp_dir().join(format!("isalab_test_{:?}_{}.fdb", thread_id, timestamp));

    // Asegurar que no existe una DB previa
    let _ = std::fs::remove_file(&db_path);

    let fbclient = fbclient_path();
    if !fbclient.exists() {
        panic!(
            "fbclient.dll no encontrado en {}. Los tests de integración requieren Firebird 5 Embedded.",
            fbclient.display()
        );
    }

    // Crear la DB
    let mut conn =
        create_database(&db_path, &fbclient).expect("No se pudo crear la base de datos de prueba");

    // Ejecutar migraciones
    let _version =
        migrations::run_migrations(&mut conn).expect("Las migraciones fallaron en la DB de prueba");

    // Limpiar datos de prueba de la migración 0005 (son para demo, no para tests)
    let cleanup_statements = [
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
    for stmt in &cleanup_statements {
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

    (conn, db_path)
}

/// Crea una nueva conexión a la DB de prueba existente.
pub fn test_connection(db_path: &Path) -> Result<SimpleConnection, AppError> {
    let fbclient = fbclient_path();
    new_connection(db_path, &fbclient)
}

/// Limpia la base de datos de prueba.
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
