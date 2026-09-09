//! Repositorio de catálogos: creación de analitos.
//!
//! El listado vive en `panels::list_analytes` (para la grilla de paneles);
//! aquí solo la creación, que valida unicidad por CODE de forma amigable
//! (la FK/UNIQUE de la BD es la última línea de defensa).

use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::species::{Analyte, CreateAnalyteInput};
use crate::repositories::next_id;

/// Crea un analito nuevo (activo por defecto). Devuelve el analito completo.
pub fn create_analyte(
    conn: &mut SimpleConnection,
    input: &CreateAnalyteInput,
) -> Result<Analyte, AppError> {
    let code = input.code.trim().to_uppercase();
    let name = input.name.trim();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::Validation(
            "Código y nombre del analito son obligatorios".into(),
        ));
    }
    if code.len() > 20 {
        return Err(AppError::Validation(
            "El código no puede superar 20 caracteres".into(),
        ));
    }

    // Unicidad amigable: la columna es UNIQUE, pero el error de la BD no es
    // accionable para el usuario.
    let dup: Option<(i32,)> = conn
        .query_first("SELECT ID FROM ANALYTES WHERE UPPER(CODE) = ?", (&code,))
        .map_err(AppError::from)?;
    if dup.is_some() {
        return Err(AppError::Validation(format!(
            "Ya existe un analito con el código {code}"
        )));
    }

    let id = next_id(conn, "GEN_ANALYTES_ID")?;
    conn.execute(
        "INSERT INTO ANALYTES (ID, CODE, NAME, UNIT, METHOD, DESCRIPTION, IS_ACTIVE)
         VALUES (?, ?, ?, ?, ?, ?, TRUE)",
        (
            &id,
            &code,
            &name,
            &input.unit,
            &input.method,
            &input.description,
        ),
    )
    .map_err(AppError::from)?;

    type AnalyteRow = (i32, String, String, Option<String>, Option<String>);
    let row: Option<AnalyteRow> = conn
        .query_first(
            "SELECT ID, CODE, NAME, UNIT, METHOD FROM ANALYTES WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(|r| Analyte {
        id: r.0,
        code: r.1,
        name: r.2,
        unit: r.3,
        method: r.4,
    })
    .ok_or_else(|| AppError::Internal("Analito creado pero no recuperado".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[test]
    fn create_and_duplicate_rejected() {
        let (mut conn, db_path) = setup_test_db();

        let created = create_analyte(
            &mut conn,
            &CreateAnalyteInput {
                code: "TGL".into(),
                name: "Triglicéridos".into(),
                unit: Some("mg/dL".into()),
                method: None,
                description: None,
            },
        )
        .expect("creación");
        assert_eq!(created.code, "TGL");
        assert!(created.id > 0);

        // El código se normaliza a mayúsculas: "tgl" choca con "TGL".
        let err = create_analyte(
            &mut conn,
            &CreateAnalyteInput {
                code: "tgl".into(),
                name: "Otro".into(),
                unit: None,
                method: None,
                description: None,
            },
        )
        .expect_err("duplicado debe fallar");
        assert!(err.to_string().contains("Ya existe un analito"));

        // Código inyectado desde el catalogo seed también choca.
        let err2 = create_analyte(
            &mut conn,
            &CreateAnalyteInput {
                code: "HCT".into(),
                name: "Hematocrito".into(),
                unit: None,
                method: None,
                description: None,
            },
        )
        .expect_err("seed dup debe fallar");
        assert!(err2.to_string().contains("Ya existe un analito"));

        // Vacíos → validación.
        let err3 = create_analyte(
            &mut conn,
            &CreateAnalyteInput {
                code: "  ".into(),
                name: "X".into(),
                unit: None,
                method: None,
                description: None,
            },
        )
        .expect_err("código vacío");
        assert!(err3.to_string().contains("obligatorios"));

        cleanup_test_db(&db_path);
    }
}
