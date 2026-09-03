use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::panel::{Panel, PanelAnalyte, PanelInput};
use crate::repositories::next_id;

type PanelRow = (
    i32,
    String,
    Option<i32>,
    Option<String>,
    i32,
    bool,
    Option<String>,
    i32,
);

/// Lista los paneles activos (con el nº de analitos y el tipo de muestra).
pub fn list(conn: &mut SimpleConnection) -> Result<Vec<Panel>, AppError> {
    let rows: Vec<PanelRow> = conn
        .query(
            "SELECT p.ID, p.NAME, p.SAMPLE_TYPE_ID, st.NAME, p.SORT_ORDER,
                    p.IS_ACTIVE, p.NOTES,
                    (SELECT COUNT(*) FROM PANEL_ANALYTES pa WHERE pa.PANEL_ID = p.ID)
             FROM PANELS p
             LEFT JOIN SAMPLE_TYPES st ON st.ID = p.SAMPLE_TYPE_ID
             WHERE p.IS_ACTIVE = TRUE
             ORDER BY p.SORT_ORDER, p.NAME",
            (),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| Panel {
            id: r.0,
            name: r.1,
            sample_type_id: r.2,
            sample_type_name: r.3,
            sort_order: r.4,
            is_active: r.5,
            notes: r.6,
            analyte_count: r.7,
        })
        .collect())
}

/// Analitos de un panel, ordenados por secuencia.
pub fn list_analytes(
    conn: &mut SimpleConnection,
    panel_id: i32,
) -> Result<Vec<PanelAnalyte>, AppError> {
    let rows: Vec<(i32, String, Option<String>, i32)> = conn
        .query(
            "SELECT pa.ANALYTE_ID, a.NAME, a.UNIT, pa.SEQ
             FROM PANEL_ANALYTES pa
             JOIN ANALYTES a ON a.ID = pa.ANALYTE_ID
             WHERE pa.PANEL_ID = ?
             ORDER BY pa.SEQ, a.NAME",
            (&panel_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| PanelAnalyte {
            analyte_id: r.0,
            analyte_name: r.1,
            unit: r.2,
            seq: r.3,
        })
        .collect())
}

/// Crea o actualiza un panel, reemplazando su lista de analitos.
pub fn save(conn: &mut SimpleConnection, input: &PanelInput) -> Result<Panel, AppError> {
    if input.analyte_ids.is_empty() {
        return Err(AppError::Validation(
            "El panel debe tener al menos un analito".into(),
        ));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "El nombre del panel es obligatorio".into(),
        ));
    }

    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE PANELS
                    SET NAME = ?, SAMPLE_TYPE_ID = ?, SORT_ORDER = ?, NOTES = ?,
                        UPDATED_AT = CURRENT_TIMESTAMP
                  WHERE ID = ?",
                (
                    &name,
                    &input.sample_type_id,
                    &input.sort_order,
                    &input.notes,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            conn.execute("DELETE FROM PANEL_ANALYTES WHERE PANEL_ID = ?", (&id,))
                .map_err(AppError::from)?;
            id
        }
        None => {
            let nid = next_id(conn, "GEN_PANELS_ID")?;
            conn.execute(
                "INSERT INTO PANELS (ID, NAME, SAMPLE_TYPE_ID, SORT_ORDER, NOTES)
                 VALUES (?, ?, ?, ?, ?)",
                (
                    &nid,
                    &name,
                    &input.sample_type_id,
                    &input.sort_order,
                    &input.notes,
                ),
            )
            .map_err(AppError::from)?;
            nid
        }
    };

    for (seq, analyte_id) in input.analyte_ids.iter().enumerate() {
        let aid = next_id(conn, "GEN_PANEL_ANALYTES_ID")?;
        conn.execute(
            "INSERT INTO PANEL_ANALYTES (ID, PANEL_ID, ANALYTE_ID, SEQ) VALUES (?, ?, ?, ?)",
            (&aid, &id, analyte_id, &((seq as i32) * 10)),
        )
        .map_err(AppError::from)?;
    }

    let row: Option<PanelRow> = conn
        .query_first(
            "SELECT p.ID, p.NAME, p.SAMPLE_TYPE_ID, st.NAME, p.SORT_ORDER,
                    p.IS_ACTIVE, p.NOTES,
                    (SELECT COUNT(*) FROM PANEL_ANALYTES pa WHERE pa.PANEL_ID = p.ID)
             FROM PANELS p
             LEFT JOIN SAMPLE_TYPES st ON st.ID = p.SAMPLE_TYPE_ID
             WHERE p.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(|r| Panel {
        id: r.0,
        name: r.1,
        sample_type_id: r.2,
        sample_type_name: r.3,
        sort_order: r.4,
        is_active: r.5,
        notes: r.6,
        analyte_count: r.7,
    })
    .ok_or_else(|| AppError::Internal("Panel guardado pero no recuperado".into()))
}

/// Elimina un panel (cascade sobre PANEL_ANALYTES).
pub fn delete(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM PANELS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}
