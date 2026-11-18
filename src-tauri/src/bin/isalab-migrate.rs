//! Runner administrativo del esquema de ISALAB sobre un servidor Firebird.
//!
//! La aplicación **nunca** aplica DDL cuando trabaja en modo remoto: solo
//! verifica la versión del esquema. Este binario es el único que migra, crea
//! la contraseña inicial del administrador y vuelve a otorgar los permisos del
//! usuario de aplicación. Se ejecuta una vez por actualización, idealmente
//! desde el propio servidor.
//!
//! ```text
//! isalab-migrate --host 192.168.1.10 --database ISALAB --user SYSDBA
//! isalab-migrate --status
//! ```
//!
//! La contraseña se toma de `--password` o de `ISALAB_DB_PASSWORD`; nunca se
//! escribe en el registro ni se imprime.

use std::path::PathBuf;
use std::time::Duration;

use isalab_lib::db::config::{self, DbConfig, DbMode};
use isalab_lib::db::{self, grants, migrations, seed};
use isalab_lib::error::AppError;

const HELP: &str = "\
isalab-migrate — runner de esquema para el servidor Firebird de ISALAB

USO
  isalab-migrate [opciones]            aplica las migraciones pendientes
  isalab-migrate --status              solo informa (no modifica la base)

OPCIONES
  --host <host>        Servidor Firebird (def. ISALAB_DB_HOST o localhost)
  --port <puerto>      Puerto TCP (def. ISALAB_DB_PORT o 3050)
  --database <nombre>  Alias o ruta de la base (def. ISALAB_DB_NAME o ISALAB)
  --user <usuario>     Cuenta con permisos de DDL (def. ISALAB_DB_USER o SYSDBA)
  --password <clave>   Contraseña (o variable ISALAB_DB_PASSWORD)
  --app-user <usuario> Beneficiario de los permisos (def. ISALAB_APP)
  --fbclient <ruta>    fbclient.dll a usar (def. FIREBIRD_DIR o junto al .exe)
  --seed-demo          Incluye los datos de demostración (solo localhost)
  --no-demo            Nunca incluye datos de demostración (recomendado)
  --status             Muestra la versión actual y las migraciones pendientes
  -h, --help           Muestra esta ayuda

EJEMPLOS
  set ISALAB_DB_PASSWORD=******
  isalab-migrate --host 192.168.1.10
  isalab-migrate --status --host 192.168.1.10
";

#[derive(Debug)]
struct Args {
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    app_user: String,
    fbclient: Option<PathBuf>,
    status: bool,
    seed_demo: bool,
    no_demo: bool,
}

impl Default for Args {
    fn default() -> Self {
        let env = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        Self {
            host: env("ISALAB_DB_HOST").unwrap_or_else(|| "localhost".into()),
            port: env("ISALAB_DB_PORT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(config::DEFAULT_PORT),
            database: env("ISALAB_DB_NAME").unwrap_or_else(|| config::DEFAULT_DATABASE.into()),
            user: env("ISALAB_DB_USER").unwrap_or_else(|| "SYSDBA".into()),
            password: env("ISALAB_DB_PASSWORD").unwrap_or_default(),
            app_user: config::DEFAULT_USER.into(),
            fbclient: None,
            status: false,
            seed_demo: false,
            no_demo: false,
        }
    }
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!();
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, AppError> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);

    while let Some(arg) = it.next() {
        let mut value = |name: &str| -> Result<String, AppError> {
            it.next()
                .ok_or_else(|| AppError::Validation(format!("Falta el valor de {name}")))
        };
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(0);
            }
            "--host" => args.host = value("--host")?,
            "--port" => {
                args.port = value("--port")?
                    .parse()
                    .map_err(|_| AppError::Validation("Puerto inválido".into()))?
            }
            "--database" | "--db" => args.database = value("--database")?,
            "--user" => args.user = value("--user")?,
            "--password" => args.password = value("--password")?,
            "--app-user" => args.app_user = value("--app-user")?,
            "--fbclient" => args.fbclient = Some(PathBuf::from(value("--fbclient")?)),
            "--status" | "--check" => args.status = true,
            "--seed-demo" => args.seed_demo = true,
            "--no-demo" => args.no_demo = true,
            other => {
                return Err(AppError::Validation(format!(
                    "Opción desconocida: «{other}». Use --help."
                )))
            }
        }
    }

    if args.seed_demo && args.no_demo {
        return Err(AppError::Validation(
            "--seed-demo y --no-demo son excluyentes: elija uno.".into(),
        ));
    }

    if args.password.trim().is_empty() {
        return Err(AppError::Validation(
            "Falta la contraseña. Use --password o la variable ISALAB_DB_PASSWORD.".into(),
        ));
    }

    let cfg = DbConfig {
        mode: DbMode::Remote,
        host: args.host.trim().to_string(),
        port: args.port,
        database: args.database.trim().to_string(),
        user: args.user.trim().to_string(),
        password: args.password.clone(),
        // Un runner puede correr sobre una red lenta o un servidor que tarda
        // en arrancar: un minuto es más tolerante que los 5 s de la app.
        connect_timeout: Duration::from_secs(60),
        fbclient: resolve_fbclient(args.fbclient.clone()),
        embedded_path: PathBuf::from("isalab.fdb"),
    };

    println!("Servidor: {}", cfg.target());
    println!("Usuario:  {}", cfg.user);

    let mut conn = db::connect(&cfg)?;
    println!("Conexión establecida.");

    if args.status {
        return print_status(&mut conn);
    }

    let ctx = if args.no_demo {
        migrations::MigrationContext {
            allow_demo_data: false,
        }
    } else if args.seed_demo {
        if !migrations::is_loopback(&cfg.host) {
            return Err(AppError::Validation(format!(
                "--seed-demo solo se permite contra localhost; el destino es «{}». La base de la clínica nunca lleva datos de demostración.",
                cfg.host
            )));
        }
        migrations::MigrationContext {
            allow_demo_data: true,
        }
    } else {
        migrations::MigrationContext::for_target(&cfg.host)
    };

    let before = migrations::applied_version(&mut conn)?;
    let expected = migrations::expected_version();
    println!("Esquema actual: v{before} (esperado v{expected})");

    if before == expected {
        println!("No hay migraciones pendientes.");
    } else {
        if before > expected {
            return Err(AppError::Db(format!(
                "La base usa el esquema v{before}, más nuevo que el que entiende este runner (v{expected}). Actualice el runner/app antes de continuar."
            )));
        }
        println!("Aplicando migraciones pendientes…");
    }

    let version = migrations::run_migrations_with(&mut conn, ctx)?;
    println!("Esquema actualizado a v{version}.");

    // Un único UPDATE (no uno por cliente): fija `admin123` con cambio
    // obligatorio si el administrador aún no tiene contraseña.
    if seed::ensure_default_admin(&mut conn)? {
        println!("Contraseña inicial de «admin» establecida (cambio obligatorio al entrar).");
    }

    let summary = grants::apply_app_grants(&mut conn, &args.app_user)?;
    println!(
        "Permisos otorgados a «{}»: {} tablas, {} vistas, {} generadores, {} procedimientos{}.",
        args.app_user,
        summary.tables,
        summary.views,
        summary.sequences,
        summary.procedures,
        if summary.functions > 0 {
            format!(", {} funciones", summary.functions)
        } else {
            String::new()
        }
    );
    if !summary.grantee_verified {
        println!(
            "Aviso: no se pudo verificar que «{}» exista en el servidor (la cuenta no puede leer SEC$USERS).",
            args.app_user
        );
    }

    // Mantenimiento que ya no hacen los clientes: poda de la telemetría de
    // eventos con más de 30 días. Best-effort.
    match db::maintenance::prune_event_log_conn(&mut conn) {
        Ok(removed) => println!("Poda de EVENT_LOG: {removed} filas antiguas eliminadas."),
        Err(e) => println!("Aviso: no se pudo podar EVENT_LOG: {e}"),
    }

    println!();
    println!("Listo. Los equipos de la clínica pueden abrir la aplicación.");
    Ok(0)
}

fn print_status(conn: &mut rsfbclient::SimpleConnection) -> Result<i32, AppError> {
    let current = migrations::applied_version(conn)?;
    let expected = migrations::expected_version();
    let pending = migrations::pending_migrations(conn)?;
    let unfinished = migrations::unfinished_migrations(conn)?;

    println!("Esquema actual: v{current}");
    println!("Esperado por esta versión: v{expected}");

    if !unfinished.is_empty() {
        println!();
        println!("ATENCIÓN: hay migraciones sin terminar:");
        for (version, name) in &unfinished {
            println!("  v{version:<3} {name}");
        }
        println!("Revise el esquema y marque la versión como aplicada cuando esté correcta:");
        if let Some((version, _)) = unfinished.first() {
            println!("  UPDATE SCHEMA_MIGRATIONS SET STATUS = 'DONE' WHERE VERSION = {version};");
        }
        return Ok(0);
    }

    if pending.is_empty() {
        println!("Estado: al día.");
        return Ok(0);
    }

    println!("Pendientes: {}", pending.len());
    for (version, name) in pending {
        println!("  v{version:<3} {name}");
    }
    println!("Ejecute el runner sin --status para aplicarlas.");
    Ok(0)
}

/// Resuelve `fbclient.dll` con el mismo criterio que la app en desarrollo.
///
/// Si no encuentra ningún archivo devuelve el nombre suelto para que Windows lo
/// busque en el PATH (útil cuando el cliente Firebird está instalado a nivel de
/// sistema).
fn resolve_fbclient(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Some(dir) = std::env::var_os("FIREBIRD_DIR") {
        let candidate = PathBuf::from(dir).join("fbclient.dll");
        if candidate.exists() {
            return candidate;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("fbclient.dll");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    let bundled = PathBuf::from("binaries/firebird/fbclient.dll");
    if bundled.exists() {
        return bundled;
    }
    PathBuf::from("fbclient.dll")
}
