use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::logo::SecondaryLogo;
use crate::repositories::next_id;

pub(crate) type SecondaryLogoRow = (
    i32,
    String,
    String,
    String, // created_at
);

pub(crate) fn map_secondary_logo(r: SecondaryLogoRow) -> SecondaryLogo {
    SecondaryLogo {
        id: r.0,
        name: r.1,
        logo_path: r.2,
        created_at: r.3,
    }
}

pub fn list(conn: &mut SimpleConnection) -> Result<Vec<SecondaryLogo>, AppError> {
    let rows: Vec<SecondaryLogoRow> = conn
        .query(
            "SELECT ID, NAME, LOGO_PATH, CAST(CREATED_AT AS VARCHAR(30)) FROM SECONDARY_LOGOS ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_secondary_logo).collect())
}

pub fn insert(
    conn: &mut SimpleConnection,
    name: &str,
    logo_path: &str,
) -> Result<SecondaryLogo, AppError> {
    let id = next_id(conn, "GEN_SECONDARY_LOGOS_SEQ")?;
    conn.execute(
        "INSERT INTO SECONDARY_LOGOS (ID, NAME, LOGO_PATH) VALUES (?, ?, ?)",
        (&id, name, logo_path),
    )
    .map_err(AppError::from)?;

    let row: Option<SecondaryLogoRow> = conn
        .query_first(
            "SELECT ID, NAME, LOGO_PATH, CAST(CREATED_AT AS VARCHAR(30)) FROM SECONDARY_LOGOS WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(map_secondary_logo)
        .ok_or_else(|| AppError::Internal("Logo no encontrado tras insertar".into()))
}

pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<SecondaryLogo>, AppError> {
    let row: Option<SecondaryLogoRow> = conn
        .query_first(
            "SELECT ID, NAME, LOGO_PATH, CAST(CREATED_AT AS VARCHAR(30)) FROM SECONDARY_LOGOS WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_secondary_logo))
}

pub fn delete(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM SECONDARY_LOGOS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}
