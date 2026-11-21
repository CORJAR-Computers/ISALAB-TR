//! Canario de esquema para el smoke de instalación.
//!
//! Ejercita el binario **real** (sin mocks) contra un Firebird 5 Embedded
//! recién creado: aplica todas las migraciones, comprueba que el esquema queda
//! en la versión que espera esta build, que el catálogo y el usuario inicial
//! existen y que una escritura real se confirma y se puede leer. Lo usa
//! `.github/workflows/smoke-real.yml` en el runner Windows después de instalar
//! el paquete NSIS.
//!
//! ```text
//! isalab-smoke [--db <ruta.fdb>] [--fbclient <ruta>]
//! ```
//!
//! Sale con código 0 e imprime `SMOKE OK` si todo cuadra; con código 1 y el
//! motivo concreto si algo falla.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use isalab_lib::db::{self, migrations};
use rsfbclient::{Execute, Queryable, SimpleConnection};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("SMOKE FALLÓ: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db_path = match value_of(&args, "--db") {
        Some(path) => PathBuf::from(path),
        None => std::env::temp_dir().join(format!("isalab-smoke-{}.fdb", std::process::id())),
    };
    let fbclient = resolve_fbclient(value_of(&args, "--fbclient").map(PathBuf::from))?;

    // Una base previa del mismo nombre haría fallar `create_database`: el smoke
    // siempre parte de cero.
    let _ = std::fs::remove_file(&db_path);

    println!("SMOKE: creando base {}", db_path.display());
    let mut conn = db::create_database(&db_path, &fbclient)
        .map_err(|e| format!("no se pudo crear la base: {e}"))?;

    println!("SMOKE: aplicando migraciones");
    let expected = migrations::expected_version();
    let version = migrations::run_migrations_with(
        &mut conn,
        migrations::MigrationContext {
            allow_demo_data: false,
        },
    )
    .map_err(|e| format!("las migraciones fallaron: {e}"))?;
    if version != expected {
        return Err(format!(
            "esquema en v{version}, la build espera v{expected}"
        ));
    }
    let pending = migrations::pending_migrations(&mut conn)
        .map_err(|e| format!("no se pudo leer el estado de migraciones: {e}"))?;
    if !pending.is_empty() {
        let names: Vec<String> = pending
            .iter()
            .map(|(v, name)| format!("{name} (v{v})"))
            .collect();
        return Err(format!(
            "quedaron migraciones pendientes: {}",
            names.join(", ")
        ));
    }

    // Catálogo base: sin especies no hay pacientes y la clínica no arranca.
    let species: i32 = scalar(&mut conn, "SELECT COUNT(*) FROM SPECIES")?;
    if species == 0 {
        return Err("el catálogo de especies quedó vacío".into());
    }

    // Usuario inicial (se crea con contraseña NULL y MUST_CHANGE_PASSWORD).
    let admins: i32 = scalar(
        &mut conn,
        "SELECT COUNT(*) FROM USERS WHERE TRIM(ROLE) = 'ADMIN' AND ACTIVE = TRUE",
    )?;
    if admins == 0 {
        return Err("no hay ningún usuario ADMIN activo".into());
    }

    // Escritura real: alta, lectura y borrado de un propietario. Verifica
    // triggers, generadores y commits, no solo el DDL.
    conn.execute(
        "INSERT INTO OWNERS (DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
         VALUES ('CC', 'SMOKE-0001', 'Smoke ISALAB')",
        (),
    )
    .map_err(|e| format!("el alta de prueba falló: {e}"))?;
    let found: Option<(String,)> = conn
        .query_first(
            "SELECT TRIM(FULL_NAME) FROM OWNERS WHERE DOCUMENT_NUMBER = 'SMOKE-0001'",
            (),
        )
        .map_err(|e| format!("la lectura de prueba falló: {e}"))?;
    if found.as_ref().map(|(name,)| name.as_str()) != Some("Smoke ISALAB") {
        return Err("el propietario insertado no se pudo leer de vuelta".into());
    }
    conn.execute(
        "DELETE FROM OWNERS WHERE DOCUMENT_NUMBER = 'SMOKE-0001'",
        (),
    )
    .map_err(|e| format!("la limpieza de prueba falló: {e}"))?;

    // Repetir el runner no debe mover la versión (idempotencia).
    let again = migrations::run_migrations_with(
        &mut conn,
        migrations::MigrationContext {
            allow_demo_data: false,
        },
    )
    .map_err(|e| format!("la segunda corrida de migraciones falló: {e}"))?;
    if again != expected {
        return Err(format!(
            "la segunda corrida dejó el esquema en v{again}, se esperaba v{expected}"
        ));
    }

    println!("SMOKE OK: esquema v{version}, {species} especies, escritura verificada");
    drop(conn);
    let _ = std::fs::remove_file(&db_path);
    Ok(())
}

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|idx| args.get(idx + 1))
        .cloned()
}

fn scalar(conn: &mut SimpleConnection, sql: &str) -> Result<i32, String> {
    let row: Option<(i32,)> = conn
        .query_first(sql, ())
        .map_err(|e| format!("consulta fallida ({sql}): {e}"))?;
    Ok(row.map(|(n,)| n).unwrap_or(0))
}

/// Busca `fbclient.dll` en el orden que usa la app: ruta explícita,
/// `FIREBIRD_DIR`, junto al ejecutable y en la carpeta de recursos del bundle.
fn resolve_fbclient(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        let dll = if path.is_dir() {
            path.join("fbclient.dll")
        } else {
            path
        };
        if dll.exists() {
            return Ok(dll);
        }
        return Err(format!("no existe {}", dll.display()));
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("FIREBIRD_DIR") {
        candidates.push(PathBuf::from(dir).join("fbclient.dll"));
    }
    candidates.push(exe_dir.join("fbclient.dll"));
    candidates.push(exe_dir.join("firebird").join("fbclient.dll"));
    candidates.push(PathBuf::from("binaries/firebird/fbclient.dll"));

    for dll in candidates {
        if dll.exists() {
            return Ok(dll);
        }
    }
    Err(
        "fbclient.dll no encontrado; pasa --fbclient o define FIREBIRD_DIR apuntando a Firebird 5"
            .into(),
    )
}
