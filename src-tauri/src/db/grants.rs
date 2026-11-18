//! Permisos del usuario de aplicación sobre el esquema.
//!
//! Firebird **no** admite `GRANT ... ON ALL TABLES IN DATABASE` (se comprobó
//! contra un servidor 5.0.3: el parser rechaza `ALL` tras `ON`). Por eso el
//! runner enumera los objetos de usuario y otorga permiso uno por uno.
//!
//! Esto además resuelve el problema de fondo: los permisos son una foto del
//! momento, así que cada corrida del runner vuelve a otorgarlos y las tablas,
//! vistas, generadores o procedimientos creados por migraciones nuevas quedan
//! cubiertos sin intervención manual.

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;

/// Privilegios DML que la app necesita en cada tabla.
const TABLE_PRIVILEGES: &str = "SELECT, INSERT, UPDATE, DELETE";

/// Resumen de lo otorgado (se imprime al final del runner).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GrantSummary {
    pub tables: usize,
    pub views: usize,
    pub sequences: usize,
    pub procedures: usize,
    pub functions: usize,
    /// `false` si no se pudo comprobar que el beneficiario exista (la cuenta
    /// del runner no tiene acceso a `SEC$USERS`).
    pub grantee_verified: bool,
}

impl GrantSummary {
    pub fn total(&self) -> usize {
        self.tables + self.views + self.sequences + self.procedures + self.functions
    }
}

/// Valida el nombre del usuario/rol beneficiario.
///
/// No puede ir como parámetro `?` (es un identificador SQL), así que se exige
/// el alfabeto de un identificador simple para descartar inyección.
fn validate_grantee(user: &str) -> Result<&str, AppError> {
    let user = user.trim();
    if user.is_empty() {
        return Err(AppError::Validation(
            "Indique el usuario de la aplicación al que otorgar permisos".into(),
        ));
    }
    let valid = user.len() <= 63
        && user
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if !valid {
        return Err(AppError::Validation(format!(
            "Nombre de usuario inválido para otorgar permisos: «{user}»"
        )));
    }
    Ok(user)
}

/// Entrecomilla un identificador de objeto y duplica las comillas internas.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Lista los nombres de una consulta de una sola columna.
fn names(conn: &mut SimpleConnection, sql: &str) -> Result<Vec<String>, AppError> {
    let rows: Vec<(String,)> = conn.query(sql, ()).map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(name,)| name.trim().to_string())
        .collect())
}

/// Comprueba que el beneficiario exista como usuario o rol.
///
/// Firebird acepta `GRANT` a usuarios y roles inexistentes: solo registra el
/// privilegio en el catálogo. Un nombre mal escrito pasaría inadvertido hasta
/// que la app fallara con «no permission for select access».
///
/// Devuelve `false` cuando no se puede verificar (cuenta sin acceso a
/// `SEC$USERS`, p. ej. un cliente embedded con la base de seguridad sin
/// inicializar): es un aviso, no un error. `Err` solo si se pudo comprobar y
/// el beneficiario no existe.
fn ensure_grantee_exists(conn: &mut SimpleConnection, grantee: &str) -> Result<bool, AppError> {
    let user: Result<Option<(i32,)>, _> = conn.query_first(
        "SELECT 1 FROM SEC$USERS WHERE UPPER(TRIM(SEC$USER_NAME)) = UPPER(?)",
        (grantee,),
    );

    match user {
        Ok(Some(_)) => Ok(true),
        Ok(None) => {
            let role: Result<Option<(i32,)>, _> = conn.query_first(
                "SELECT 1 FROM RDB$ROLES WHERE UPPER(TRIM(RDB$ROLE_NAME)) = UPPER(?)",
                (grantee,),
            );
            match role {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Err(AppError::Validation(format!(
                    "El usuario «{grantee}» no existe en el servidor Firebird. Créelo antes de ejecutar el runner:\n  CREATE USER {grantee} PASSWORD '<clave>' USING PLUGIN Srp;"
                ))),
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

/// Vuelve a otorgar los permisos de la app sobre todos los objetos de usuario.
///
/// Se ejecuta en una única transacción: si un GRANT falla no queda un esquema
/// con permisos a medias.
pub fn apply_app_grants(
    conn: &mut SimpleConnection,
    grantee: &str,
) -> Result<GrantSummary, AppError> {
    let grantee = validate_grantee(grantee)?;
    let grantee_verified = ensure_grantee_exists(conn, grantee)?;

    // Se leen antes de abrir la transacción de escritura.
    let tables = names(
        conn,
        "SELECT TRIM(RDB$RELATION_NAME) FROM RDB$RELATIONS
          WHERE COALESCE(RDB$SYSTEM_FLAG, 0) = 0 AND RDB$VIEW_BLR IS NULL
          ORDER BY 1",
    )?;
    let views = names(
        conn,
        "SELECT TRIM(RDB$RELATION_NAME) FROM RDB$RELATIONS
          WHERE COALESCE(RDB$SYSTEM_FLAG, 0) = 0 AND RDB$VIEW_BLR IS NOT NULL
          ORDER BY 1",
    )?;
    let sequences = names(
        conn,
        "SELECT TRIM(RDB$GENERATOR_NAME) FROM RDB$GENERATORS
          WHERE COALESCE(RDB$SYSTEM_FLAG, 0) = 0
          ORDER BY 1",
    )?;
    let procedures = names(
        conn,
        "SELECT TRIM(RDB$PROCEDURE_NAME) FROM RDB$PROCEDURES
          WHERE COALESCE(RDB$SYSTEM_FLAG, 0) = 0
            AND COALESCE(RDB$PACKAGE_NAME, '') = ''
          ORDER BY 1",
    )?;
    let functions = names(
        conn,
        "SELECT TRIM(RDB$FUNCTION_NAME) FROM RDB$FUNCTIONS
          WHERE COALESCE(RDB$SYSTEM_FLAG, 0) = 0
            AND COALESCE(RDB$PACKAGE_NAME, '') = ''
          ORDER BY 1",
    )?;

    let mut statements: Vec<String> = Vec::new();
    for table in &tables {
        statements.push(format!(
            "GRANT {TABLE_PRIVILEGES} ON TABLE {} TO {}",
            quote_ident(table),
            grantee
        ));
    }
    for view in &views {
        statements.push(format!(
            "GRANT SELECT ON {} TO {}",
            quote_ident(view),
            grantee
        ));
    }
    for sequence in &sequences {
        statements.push(format!(
            "GRANT USAGE ON SEQUENCE {} TO {}",
            quote_ident(sequence),
            grantee
        ));
    }
    for procedure in &procedures {
        statements.push(format!(
            "GRANT EXECUTE ON PROCEDURE {} TO {}",
            quote_ident(procedure),
            grantee
        ));
    }
    for function in &functions {
        statements.push(format!(
            "GRANT EXECUTE ON FUNCTION {} TO {}",
            quote_ident(function),
            grantee
        ));
    }

    conn.begin_transaction().map_err(|e| {
        AppError::Db(format!(
            "No se pudo iniciar la transacción de permisos: {e}"
        ))
    })?;

    let result = (|| -> Result<(), AppError> {
        for stmt in &statements {
            conn.execute(stmt, ()).map_err(|e| {
                AppError::Db(format!(
                    "No se pudo otorgar permisos a «{grantee}»: {e}\nSQL: {stmt}"
                ))
            })?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => conn
            .commit()
            .map_err(|e| AppError::Db(format!("No se pudieron confirmar los permisos: {e}")))?,
        Err(e) => {
            let _ = conn.rollback();
            return Err(e);
        }
    }

    Ok(GrantSummary {
        tables: tables.len(),
        views: views.len(),
        sequences: sequences.len(),
        procedures: procedures.len(),
        functions: functions.len(),
        grantee_verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{cleanup_test_db, setup_test_db};

    #[test]
    fn rejects_invalid_grantees() {
        assert!(validate_grantee("ISALAB_APP").is_ok());
        assert!(validate_grantee("").is_err());
        assert!(validate_grantee("APP; DROP DATABASE").is_err());
        assert!(validate_grantee("1APP").is_err());
        assert!(validate_grantee("APP USER").is_err());
    }

    #[test]
    fn quotes_identifiers() {
        assert_eq!(quote_ident("PATIENTS"), "\"PATIENTS\"");
        assert_eq!(quote_ident("A\"B"), "\"A\"\"B\"");
    }

    #[test]
    fn grants_every_schema_object_to_the_grantee() {
        let (mut conn, db_path) = setup_test_db();

        // Un rol local sirve de beneficiario sin tocar la base de seguridad.
        conn.execute("CREATE ROLE TEST_APP_GRANTS", ())
            .expect("crear rol de prueba");

        let summary = apply_app_grants(&mut conn, "TEST_APP_GRANTS").expect("otorgar permisos");
        assert!(summary.tables >= 30, "tablas otorgadas: {summary:?}");
        assert!(summary.sequences >= 1, "generadores otorgados: {summary:?}");
        assert!(summary.procedures >= 2, "procedimientos: {summary:?}");
        assert_eq!(summary.views, 0);

        // El privilegio quedó registrado en el catálogo del sistema. Firebird
        // guarda los privilegios con una letra (S = SELECT, I, U, D).
        let granted: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM RDB$USER_PRIVILEGES
                  WHERE TRIM(RDB$USER) = 'TEST_APP_GRANTS'
                    AND TRIM(RDB$RELATION_NAME) = 'PATIENTS'
                    AND TRIM(RDB$PRIVILEGE) = 'S'",
                (),
            )
            .expect("leer privilegios");
        assert_eq!(granted.map(|(n,)| n).unwrap_or(0), 1);

        // Volver a otorgar es idempotente (el runner se puede repetir).
        let again = apply_app_grants(&mut conn, "TEST_APP_GRANTS").expect("segunda pasada");
        assert_eq!(summary, again);

        drop(conn);
        cleanup_test_db(&db_path);
    }
}
