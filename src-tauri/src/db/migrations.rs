use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Migraciones versionadas, embebidas en el binario.
/// La versión es el índice + 1 (0001 → 1, 0002 → 2…).
pub const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_initial_schema",
        include_str!("../../migrations/0001_initial_schema.sql"),
    ),
    (
        "0002_seed_catalog",
        include_str!("../../migrations/0002_seed_catalog.sql"),
    ),
    (
        "0003_surgeries",
        include_str!("../../migrations/0003_surgeries.sql"),
    ),
    (
        "0004_user_audit_log",
        include_str!("../../migrations/0004_user_audit_log.sql"),
    ),
    (
        "0005_test_data",
        include_str!("../../migrations/0005_test_data.sql"),
    ),
    (
        "0006_patient_code",
        include_str!("../../migrations/0006_patient_code.sql"),
    ),
    (
        "0007_secondary_logos",
        include_str!("../../migrations/0007_secondary_logos.sql"),
    ),
    (
        "0008_patient_preferred_logo",
        include_str!("../../migrations/0008_patient_preferred_logo.sql"),
    ),
    (
        "0009_result_attachments",
        include_str!("../../migrations/0009_result_attachments.sql"),
    ),
    (
        "0010_analyzer_profiles",
        include_str!("../../migrations/0010_analyzer_profiles.sql"),
    ),
    (
        "0011_widen_password_hash",
        include_str!("../../migrations/0011_widen_password_hash.sql"),
    ),
    (
        "0012_panels",
        include_str!("../../migrations/0012_panels.sql"),
    ),
    (
        "0013_sample_quality",
        include_str!("../../migrations/0013_sample_quality.sql"),
    ),
    (
        "0014_qc_module",
        include_str!("../../migrations/0014_qc_module.sql"),
    ),
    (
        "0015_critical_values",
        include_str!("../../migrations/0015_critical_values.sql"),
    ),
    (
        "0016_sample_events",
        include_str!("../../migrations/0016_sample_events.sql"),
    ),
    (
        "0017_notifications",
        include_str!("../../migrations/0017_notifications.sql"),
    ),
    (
        "0018_lab_orders",
        include_str!("../../migrations/0018_lab_orders.sql"),
    ),
    (
        "0019_analyzer_sources",
        include_str!("../../migrations/0019_analyzer_sources.sql"),
    ),
    (
        "0020_urine_and_biochem_panels",
        include_str!("../../migrations/0020_urine_and_biochem_panels.sql"),
    ),
    (
        "0021_reference_values_catalog",
        include_str!("../../migrations/0021_reference_values_catalog.sql"),
    ),
    (
        "0022_custom_reference_ranges",
        include_str!("../../migrations/0022_custom_reference_ranges.sql"),
    ),
    (
        "0023_maintenance_indexes",
        include_str!("../../migrations/0023_maintenance_indexes.sql"),
    ),
    (
        "0024_drop_redundant_indexes",
        include_str!("../../migrations/0024_drop_redundant_indexes.sql"),
    ),
    (
        "0025_service_catalog",
        include_str!("../../migrations/0025_service_catalog.sql"),
    ),
    (
        "0026_clinic_roles",
        include_str!("../../migrations/0026_clinic_roles.sql"),
    ),
    (
        "0027_visits",
        include_str!("../../migrations/0027_visits.sql"),
    ),
    (
        "0028_appointments",
        include_str!("../../migrations/0028_appointments.sql"),
    ),
    (
        "0029_grooming",
        include_str!("../../migrations/0029_grooming.sql"),
    ),
    (
        "0030_clinic_indexes",
        include_str!("../../migrations/0030_clinic_indexes.sql"),
    ),
    (
        "0031_settings",
        include_str!("../../migrations/0031_settings.sql"),
    ),
    (
        "0032_stays",
        include_str!("../../migrations/0032_stays.sql"),
    ),
    (
        "0033_products",
        include_str!("../../migrations/0033_products.sql"),
    ),
    (
        "0034_attachment_blobs",
        include_str!("../../migrations/0034_attachment_blobs.sql"),
    ),
    (
        "0035_ops_table",
        include_str!("../../migrations/0035_ops_table.sql"),
    ),
];

/// Indica si una migración de datos de demostración debe aplicarse.
///
/// La migración `0005_test_data` inserta pacientes, muestras y facturas
/// ficticios pensados **solo para desarrollo**. En builds de release (la
/// distribución que se instala en las clínicas) se omite para no ensuciar la
/// BD de producción; se puede forzar su aplicación con la variable de entorno
/// `ISALAB_SEED_DEMO=1` (p. ej. para pruebas manuales sobre un instalador).
fn should_seed_demo_data() -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    std::env::var("ISALAB_SEED_DEMO")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Versión de esquema que espera esta versión de la app.
pub fn expected_version() -> i32 {
    MIGRATIONS.len() as i32
}

/// Contexto de ejecución de las migraciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationContext {
    /// Permite que la migración 0005 inserte los datos de demostración.
    pub allow_demo_data: bool,
}

impl MigrationContext {
    /// Base local (desarrollo y tests): decide según el build.
    pub fn local() -> Self {
        Self {
            allow_demo_data: should_seed_demo_data(),
        }
    }

    /// Runner sobre la base de la clínica.
    ///
    /// Un build de debug conectado por accidente a la base real no debe
    /// insertar pacientes y facturas ficticias: los datos demo solo se
    /// aceptan si el destino es este mismo equipo.
    pub fn for_target(host: &str) -> Self {
        Self {
            allow_demo_data: should_seed_demo_data() && is_loopback(host),
        }
    }
}

/// `true` si `host` apunta a este equipo (`localhost`, `127.0.0.0/8`, `::1`).
pub fn is_loopback(host: &str) -> bool {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// `true` si la tabla `SCHEMA_MIGRATIONS` ya existe en la base.
fn schema_table_exists(conn: &mut SimpleConnection) -> Result<bool, AppError> {
    let exists: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM rdb$relations WHERE rdb$relation_name = 'SCHEMA_MIGRATIONS'",
            (),
        )
        .map_err(AppError::from)?;
    Ok(exists.is_some())
}

/// Versión aplicada más alta, o 0 si la base aún no tiene esquema.
///
/// Solo cuentan las migraciones en estado `DONE`: una fila `RUNNING` es una
/// migración que empezó y no terminó.
pub fn applied_version(conn: &mut SimpleConnection) -> Result<i32, AppError> {
    if !schema_table_exists(conn)? {
        return Ok(0);
    }
    let max: Option<(Option<i32>,)> = conn
        .query_first(
            "SELECT MAX(VERSION) FROM SCHEMA_MIGRATIONS WHERE STATUS = 'DONE'",
            (),
        )
        .map_err(AppError::from)?;
    Ok(max.and_then(|(v,)| v).unwrap_or(0))
}

/// Migraciones que todavía no están registradas como aplicadas.
///
/// No modifica nada: lo usa `--status` del runner para informar sin migrar.
pub fn pending_migrations(
    conn: &mut SimpleConnection,
) -> Result<Vec<(i32, &'static str)>, AppError> {
    let applied: Vec<(i32,)> = if schema_table_exists(conn)? {
        conn.query(
            "SELECT VERSION FROM SCHEMA_MIGRATIONS WHERE STATUS = 'DONE' ORDER BY VERSION",
            (),
        )
        .map_err(AppError::from)?
    } else {
        Vec::new()
    };

    Ok(MIGRATIONS
        .iter()
        .enumerate()
        .map(|(idx, (name, _))| ((idx + 1) as i32, *name))
        .filter(|(version, _)| !applied.iter().any(|(applied,)| applied == version))
        .collect())
}

/// Migraciones que quedaron a medias (`RUNNING`), con su nombre.
pub fn unfinished_migrations(conn: &mut SimpleConnection) -> Result<Vec<(i32, String)>, AppError> {
    if !schema_table_exists(conn)? {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, String)> = conn
        .query(
            "SELECT VERSION, TRIM(NAME) FROM SCHEMA_MIGRATIONS
              WHERE STATUS = 'RUNNING' ORDER BY VERSION",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows)
}

/// Aplica las migraciones pendientes (base local). Devuelve la versión final.
pub fn run_migrations(conn: &mut SimpleConnection) -> Result<i32, AppError> {
    run_migrations_with(conn, MigrationContext::local())
}

/// Aplica las migraciones pendientes con el contexto indicado.
///
/// **Atomicidad:** Firebird no permite usar metadatos nuevos en la misma
/// transacción que los creó (probado contra 5.0.3: tras `ALTER TABLE … ADD`
/// una sentencia posterior que usa la columna falla con «Column unknown»
/// hasta hacer `COMMIT`). Por eso no se puede envolver una migración completa
/// en una transacción: las que mezclan DDL y DML (backfills) se ejecutan
/// sentencia a sentencia.
///
/// Lo que sí se garantiza es que ninguna migración a medias pase inadvertida:
/// antes de ejecutarla se registra en estado `RUNNING`, y solo al terminar
/// todas sus sentencias pasa a `DONE`. Si falla, los clientes siguen viendo la
/// versión anterior y el runner se niega a continuar hasta que se repare.
pub fn run_migrations_with(
    conn: &mut SimpleConnection,
    ctx: MigrationContext,
) -> Result<i32, AppError> {
    ensure_schema_table(conn)?;

    let unfinished = unfinished_migrations(conn)?;
    if let Some((version, name)) = unfinished.first() {
        return Err(AppError::Db(format!(
            "La migración {name} (v{version}) quedó a medias en una ejecución anterior. \
             Revise el esquema del servidor y, cuando esté correcto, márquela como aplicada:\n  \
             UPDATE SCHEMA_MIGRATIONS SET STATUS = 'DONE' WHERE VERSION = {version};\n\
             Hay {} migración(es) sin terminar.",
            unfinished.len()
        )));
    }

    let applied: Vec<(i32,)> = conn
        .query(
            "SELECT VERSION FROM SCHEMA_MIGRATIONS WHERE STATUS = 'DONE' ORDER BY VERSION",
            (),
        )
        .map_err(AppError::from)?;

    for (idx, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (idx + 1) as i32;
        if applied.iter().any(|(applied,)| *applied == version) {
            continue;
        }
        apply_migration(conn, version, name, sql, ctx)?;
    }

    applied_version(conn)
}

fn ensure_schema_table(conn: &mut SimpleConnection) -> Result<(), AppError> {
    if !schema_table_exists(conn)? {
        conn.execute(
            "CREATE TABLE SCHEMA_MIGRATIONS (
                VERSION    INTEGER NOT NULL PRIMARY KEY,
                NAME       VARCHAR(100) NOT NULL,
                STATUS     VARCHAR(10) DEFAULT 'DONE' NOT NULL,
                APPLIED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
            )",
            (),
        )
        .map_err(AppError::from)?;
        return Ok(());
    }

    // Bases creadas por versiones anteriores del runner: sin STATUS.
    let has_status: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM RDB$RELATION_FIELDS
              WHERE TRIM(RDB$RELATION_NAME) = 'SCHEMA_MIGRATIONS'
                AND TRIM(RDB$FIELD_NAME) = 'STATUS'",
            (),
        )
        .map_err(AppError::from)?;
    if has_status.is_none() {
        conn.execute(
            "ALTER TABLE SCHEMA_MIGRATIONS
                ADD STATUS VARCHAR(10) DEFAULT 'DONE' NOT NULL",
            (),
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

/// Aplica una migración y la marca como `DONE`.
///
/// El registro `RUNNING` se escribe antes de tocar el esquema: si el proceso
/// muere o una sentencia falla, la fila queda visible y bloquea la siguiente
/// corrida en lugar de dar la migración por aplicada.
fn apply_migration(
    conn: &mut SimpleConnection,
    version: i32,
    name: &str,
    sql: &str,
    ctx: MigrationContext,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO SCHEMA_MIGRATIONS (VERSION, NAME, STATUS) VALUES (?, ?, 'RUNNING')",
        (&version, name),
    )
    .map_err(|e| {
        AppError::Db(format!(
            "No se pudo registrar la migración {name} (v{version}) como en curso: {e}"
        ))
    })?;

    // La migración de datos de demostración (0005) se marca como aplicada
    // sin ejecutar sus INSERT cuando el destino no admite datos demo.
    if !(name == "0005_test_data" && !ctx.allow_demo_data) {
        let statements = split_statements(sql)?;
        for stmt in statements {
            conn.execute(&stmt, ()).map_err(|e| {
                AppError::Db(format!(
                    "Migración {name} (v{version}) — sentencia fallida: {e}\nSQL: {stmt}\n\
                     La migración quedó marcada como RUNNING; revise el esquema antes de reintentar."
                ))
            })?;
        }
    }

    conn.execute(
        "UPDATE SCHEMA_MIGRATIONS SET STATUS = 'DONE', APPLIED_AT = CURRENT_TIMESTAMP
          WHERE VERSION = ?",
        (&version,),
    )
    .map_err(|e| {
        AppError::Db(format!(
            "Migración {name} (v{version}) aplicada pero no se pudo marcar como terminada: {e}"
        ))
    })?;

    Ok(())
}

/// Divide un script SQL en sentencias individuales respetando `SET TERM`
/// (necesario para crear stored procedures/triggers con ';' internos).
fn split_statements(sql: &str) -> Result<Vec<String>, AppError> {
    let mut statements = Vec::new();
    let mut terminator = ";".to_string();
    let mut current = String::new();

    for raw_line in sql.lines() {
        let line = raw_line.trim();

        if line.is_empty() {
            continue;
        }
        if line.starts_with("--") || line.starts_with("//") {
            continue;
        }
        // Directiva de terminador: "SET TERM ^ ;" o "SET TERM ^" (3 tokens,
        // la forma habitual de isql). Se acepta también el de 4 tokens.
        if line.to_ascii_uppercase().starts_with("SET TERM") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                terminator = parts[2].to_string();
            }
            continue;
        }

        if current.is_empty() {
            current.push_str(raw_line);
        } else {
            current.push('\n');
            current.push_str(raw_line);
        }

        if current.trim_end().ends_with(&terminator) {
            let stmt = current
                .trim()
                .strip_suffix(&terminator)
                .unwrap_or(current.trim())
                .trim()
                .to_string();
            if !stmt.is_empty() {
                statements.push(stmt);
            }
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        return Err(AppError::Db(
            "Migración SQL incompleta: falta el terminador de sentencia".into(),
        ));
    }

    Ok(statements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{cleanup_test_db, setup_empty_test_db};

    fn no_demo() -> MigrationContext {
        MigrationContext {
            allow_demo_data: false,
        }
    }

    #[test]
    fn demo_data_is_only_allowed_on_loopback_targets() {
        assert!(is_loopback("localhost"));
        assert!(is_loopback("LOCALHOST"));
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback(" 127.4.3.2 "));
        assert!(is_loopback("::1"));
        assert!(is_loopback("[::1]"));
        assert!(!is_loopback("192.168.1.10"));
        assert!(!is_loopback("isalab-srv"));
        assert!(!is_loopback(""));
    }

    #[test]
    fn migrates_an_empty_database_without_demo_data() {
        let (mut conn, db_path) = setup_empty_test_db();

        assert_eq!(applied_version(&mut conn).expect("versión inicial"), 0);
        let pending = pending_migrations(&mut conn).expect("pendientes");
        assert_eq!(pending.len(), expected_version() as usize);

        let version = run_migrations_with(&mut conn, no_demo()).expect("migrar base vacía");
        assert_eq!(version, expected_version());
        assert!(pending_migrations(&mut conn)
            .expect("pendientes")
            .is_empty());

        // La base de la clínica arranca limpia: 0005 quedó registrada pero sin
        // insertar pacientes de demostración.
        let patients: Option<(i32,)> = conn
            .query_first("SELECT COUNT(*) FROM PATIENTS", ())
            .expect("contar pacientes");
        assert_eq!(patients.map(|(n,)| n).unwrap_or(-1), 0);

        // Repetir el runner no vuelve a aplicar nada ni falla.
        let again = run_migrations_with(&mut conn, no_demo()).expect("segunda pasada");
        assert_eq!(again, version);

        // Todas las filas quedaron en DONE (ninguna a medias).
        let done: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM SCHEMA_MIGRATIONS WHERE STATUS = 'DONE'",
                (),
            )
            .expect("contar migraciones");
        assert_eq!(done.map(|(n,)| n).unwrap_or(0), version);

        drop(conn);
        cleanup_test_db(&db_path);
    }

    #[test]
    fn a_failed_migration_is_marked_running_and_blocks_the_runner() {
        let (mut conn, db_path) = setup_empty_test_db();
        ensure_schema_table(&mut conn).expect("tabla de versiones");

        let sql = "CREATE TABLE Z_PARTIAL_PROBE (ID INTEGER);\nESTO NO ES SQL;";
        let err = apply_migration(&mut conn, 1, "0001_partial_probe", sql, no_demo())
            .expect_err("la sentencia inválida debe fallar");
        assert!(
            err.to_string().contains("RUNNING"),
            "el error debe avisar del estado: {err}"
        );

        // Firebird no deja revertir el DDL ya confirmado (los metadatos nuevos
        // no son visibles hasta el commit), pero la versión NO cuenta como
        // aplicada y la migración queda marcada como en curso.
        assert_eq!(
            applied_version(&mut conn).expect("versión"),
            0,
            "una migración fallida no cuenta como aplicada"
        );
        let running: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM SCHEMA_MIGRATIONS WHERE STATUS = 'RUNNING'",
                (),
            )
            .expect("contar en curso");
        assert_eq!(running.map(|(n,)| n).unwrap_or(0), 1);

        // El runner se niega a continuar hasta que se repare a mano.
        let err = run_migrations_with(&mut conn, no_demo())
            .expect_err("el runner no debe continuar con una migración a medias");
        assert!(err.to_string().contains("quedó a medias"), "{err}");

        drop(conn);
        cleanup_test_db(&db_path);
    }

    #[test]
    fn a_version_cannot_be_registered_twice() {
        let (mut conn, db_path) = setup_empty_test_db();
        ensure_schema_table(&mut conn).expect("tabla de versiones");

        apply_migration(
            &mut conn,
            1,
            "0001_duplicate_probe",
            "CREATE TABLE Z_DUPLICATE_PROBE (ID INTEGER);",
            no_demo(),
        )
        .expect("primera aplicación");

        // Un segundo runner (o el mismo reintentando) no puede volver a
        // registrar la versión: la PK la rechaza y la transacción se revierte.
        let err = apply_migration(&mut conn, 1, "0001_duplicate_probe", "", no_demo());
        assert!(err.is_err(), "la versión duplicada debe rechazarse");

        let count: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM SCHEMA_MIGRATIONS WHERE VERSION = 1",
                (),
            )
            .expect("contar versiones");
        assert_eq!(count.map(|(n,)| n).unwrap_or(0), 1);

        drop(conn);
        cleanup_test_db(&db_path);
    }
}
