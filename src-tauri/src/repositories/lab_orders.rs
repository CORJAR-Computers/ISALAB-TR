use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::lab_order::{
    AccessionOrderInput, CreateLabOrderInput, LabOrder, LabOrderItem, LabOrderListItem,
    OrderSampleRef,
};
use crate::models::sample::Sample;
use crate::repositories::next_id;
use crate::repositories::samples;

/// Estados permitidos de una orden de laboratorio.
pub const ORDER_OPEN: [&str; 3] = ["SOLICITADA", "RECIBIDA", "EN_PROCESO"];

pub(crate) type OrderRow = (
    i32,
    String,
    i32,
    String,
    String,
    String,
    Option<i32>,
    Option<String>,
    String,
    String,
    Option<String>,
    String,
);

/// Fila del listado: sin NOTES, con conteo de ítems.
pub(crate) type OrderListRow = (
    i32,
    String,
    i32,
    String,
    String,
    String,
    Option<i32>,
    Option<String>,
    String,
    String,
    i32,
    String,
);

pub(crate) type ItemRow = (
    i32,
    i32,
    Option<i32>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    i32,
);

/// Valida que la orden exista y devuelva su estado actual.
fn current_status(conn: &mut SimpleConnection, id: i32) -> Result<String, AppError> {
    let row: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM LAB_ORDERS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    row.map(|(s,)| s)
        .ok_or_else(|| AppError::NotFound(format!("Orden {id} no encontrada")))
}

pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateLabOrderInput,
) -> Result<LabOrder, AppError> {
    if input.items.is_empty() {
        return Err(AppError::Validation(
            "La orden debe incluir al menos una prueba".into(),
        ));
    }
    if !matches!(input.priority.as_str(), "NORMAL" | "URGENTE") {
        return Err(AppError::Validation(
            "Prioridad inválida (NORMAL | URGENTE)".into(),
        ));
    }

    // Valida los pacientes/paneles/analitos referenciados.
    let patient: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM PATIENTS WHERE ID = ? AND ACTIVE = TRUE",
            (&input.patient_id,),
        )
        .map_err(AppError::from)?;
    if patient.is_none() {
        return Err(AppError::Validation(format!(
            "El paciente {} no existe o está inactivo",
            input.patient_id
        )));
    }
    for it in &input.items {
        match (it.panel_id, it.analyte_id) {
            (Some(pid), None) => {
                let p: Option<(i32,)> = conn
                    .query_first(
                        "SELECT 1 FROM PANELS WHERE ID = ? AND IS_ACTIVE = TRUE",
                        (&pid,),
                    )
                    .map_err(AppError::from)?;
                if p.is_none() {
                    return Err(AppError::Validation(format!(
                        "El panel {pid} no existe o está inactivo"
                    )));
                }
            }
            (None, Some(aid)) => {
                let a: Option<(i32,)> = conn
                    .query_first(
                        "SELECT 1 FROM ANALYTES WHERE ID = ? AND IS_ACTIVE = TRUE",
                        (&aid,),
                    )
                    .map_err(AppError::from)?;
                if a.is_none() {
                    return Err(AppError::Validation(format!(
                        "El analito {aid} no existe o está inactivo"
                    )));
                }
            }
            _ => {
                return Err(AppError::Validation(
                    "Cada ítem debe referenciar un panel o un analito".into(),
                ));
            }
        }
    }

    let id = next_id(conn, "GEN_LAB_ORDERS_ID")?;
    // Si el cliente no envió fecha, se usa la hora local formateada igual que
    // el resto del sistema ("YYYY-MM-DD HH:MM:SS").
    let requested_at = input
        .requested_at
        .clone()
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

    conn.execute(
        "INSERT INTO LAB_ORDERS
            (ID, PATIENT_ID, CONSULTATION_ID, REQUESTED_BY, PRIORITY, STATUS,
             NOTES, REQUESTED_AT)
         VALUES (?, ?, ?, ?, ?, 'SOLICITADA', ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.consultation_id,
            &input.requested_by,
            &input.priority,
            &input.notes,
            &requested_at,
        ),
    )
    .map_err(AppError::from)?;

    // Ítems con SEQ correlativo.
    for (i, it) in input.items.iter().enumerate() {
        let item_id = next_id(conn, "GEN_LAB_ORDER_ITEMS_ID")?;
        conn.execute(
            "INSERT INTO LAB_ORDER_ITEMS (ID, ORDER_ID, PANEL_ID, ANALYTE_ID, SEQ)
             VALUES (?, ?, ?, ?, ?)",
            (&item_id, &id, &it.panel_id, &it.analyte_id, &(i as i32)),
        )
        .map_err(AppError::from)?;
    }

    get(conn, id)?.ok_or_else(|| AppError::Internal("Orden creada pero no recuperada".into()))
}

/// Listado global de órdenes con filtros por estado y búsqueda (código,
/// paciente o propietario). Usa el patrón `? IS NULL` (parámetros fijos).
pub fn list(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<LabOrderListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());
    // `status` vacío = sin filtro (patrón `? = ''`); `like` None = sin búsqueda.
    let st = status.unwrap_or("");
    let lk = like.as_deref();

    let sql = "
        SELECT o.ID, o.CODE, o.PATIENT_ID, p.NAME, sp.NAME, ow.FULL_NAME,
                o.CONSULTATION_ID, o.REQUESTED_BY, o.PRIORITY, o.STATUS,
                (SELECT COUNT(*) FROM LAB_ORDER_ITEMS li WHERE li.ORDER_ID = o.ID),
                LEFT(CAST(o.REQUESTED_AT AS VARCHAR(60)), 19)
         FROM LAB_ORDERS o
         JOIN PATIENTS p ON p.ID = o.PATIENT_ID
         JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
         JOIN OWNERS ow ON ow.ID = p.OWNER_ID
         WHERE (? = '' OR o.STATUS = ?)
           AND (? IS NULL OR UPPER(o.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(ow.FULL_NAME) LIKE UPPER(?))
         ORDER BY o.REQUESTED_AT DESC, o.ID DESC";
    let rows: Vec<OrderListRow> = conn
        .query(sql, (&st, &st, &lk, &lk, &lk, &lk))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| LabOrderListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            species_name: r.4,
            owner_name: r.5,
            consultation_id: r.6,
            requested_by: r.7,
            priority: r.8,
            status: r.9,
            item_count: r.10,
            requested_at: r.11,
        })
        .collect())
}

/// Órdenes pendientes de un paciente (para la vista de historial clínico).
pub fn list_for_patient(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<LabOrderListItem>, AppError> {
    let rows: Vec<OrderListRow> = conn
        .query(
            "SELECT o.ID, o.CODE, o.PATIENT_ID, p.NAME, sp.NAME, ow.FULL_NAME,
                    o.CONSULTATION_ID, o.REQUESTED_BY, o.PRIORITY, o.STATUS,
                    (SELECT COUNT(*) FROM LAB_ORDER_ITEMS li WHERE li.ORDER_ID = o.ID),
                    LEFT(CAST(o.REQUESTED_AT AS VARCHAR(60)), 19)
             FROM LAB_ORDERS o
             JOIN PATIENTS p ON p.ID = o.PATIENT_ID
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS ow ON ow.ID = p.OWNER_ID
             WHERE o.PATIENT_ID = ? AND o.STATUS <> 'ANULADA'
             ORDER BY o.REQUESTED_AT DESC",
            (&patient_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| LabOrderListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            species_name: r.4,
            owner_name: r.5,
            consultation_id: r.6,
            requested_by: r.7,
            priority: r.8,
            status: r.9,
            item_count: r.10,
            requested_at: r.11,
        })
        .collect())
}

/// Devuelve una orden completa (items + muestras accesionadas).
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<LabOrder>, AppError> {
    let row: Option<OrderRow> = conn
        .query_first(
            "SELECT o.ID, o.CODE, o.PATIENT_ID, p.NAME, sp.NAME, ow.FULL_NAME,
                    o.CONSULTATION_ID, o.REQUESTED_BY, o.PRIORITY, o.STATUS,
                    o.NOTES, LEFT(CAST(o.REQUESTED_AT AS VARCHAR(60)), 19)
             FROM LAB_ORDERS o
             JOIN PATIENTS p ON p.ID = o.PATIENT_ID
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS ow ON ow.ID = p.OWNER_ID
             WHERE o.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    let Some(r) = row else {
        return Ok(None);
    };

    let items: Vec<ItemRow> = conn
        .query(
            "SELECT li.ID, li.ORDER_ID, li.PANEL_ID, p.NAME,
                    p.SAMPLE_TYPE_ID, st.NAME,
                    li.ANALYTE_ID, a.NAME, a.UNIT, li.SEQ
             FROM LAB_ORDER_ITEMS li
             LEFT JOIN PANELS p ON p.ID = li.PANEL_ID
             LEFT JOIN SAMPLE_TYPES st ON st.ID = p.SAMPLE_TYPE_ID
             LEFT JOIN ANALYTES a ON a.ID = li.ANALYTE_ID
             WHERE li.ORDER_ID = ?
             ORDER BY li.SEQ",
            (&id,),
        )
        .map_err(AppError::from)?;
    let items = items
        .into_iter()
        .map(|i| LabOrderItem {
            id: i.0,
            order_id: i.1,
            panel_id: i.2,
            panel_name: i.3,
            panel_sample_type_id: i.4,
            panel_sample_type_name: i.5,
            analyte_id: i.6,
            analyte_name: i.7,
            unit: i.8,
            seq: i.9,
        })
        .collect();

    // Muestras accesionadas desde esta orden.
    let samples: Vec<(i32, String, String, String)> = conn
        .query(
            "SELECT s.ID, s.CODE, st.NAME, s.STATUS
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             WHERE s.ORDER_ID = ?
             ORDER BY s.ID",
            (&id,),
        )
        .map_err(AppError::from)?;
    let samples = samples
        .into_iter()
        .map(|s| OrderSampleRef {
            id: s.0,
            code: s.1,
            sample_type_name: s.2,
            status: s.3,
        })
        .collect();

    Ok(Some(LabOrder {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        patient_name: r.3,
        species_name: r.4,
        owner_name: r.5,
        consultation_id: r.6,
        requested_by: r.7,
        priority: r.8,
        status: r.9,
        notes: r.10,
        requested_at: r.11,
        items,
        samples,
    }))
}

/// Orden de la que proviene una muestra accesionada (si aplica).
pub fn get_for_sample(
    conn: &mut SimpleConnection,
    sample_id: i32,
) -> Result<Option<LabOrder>, AppError> {
    let row: Option<(Option<i32>,)> = conn
        .query_first("SELECT ORDER_ID FROM SAMPLES WHERE ID = ?", (&sample_id,))
        .map_err(AppError::from)?;
    match row.and_then(|(o,)| o) {
        Some(order_id) => get(conn, order_id),
        None => Ok(None),
    }
}

/// Conteo por estado (para las pestañas de órdenes).
pub fn count_by_status(conn: &mut SimpleConnection) -> Result<Vec<(String, i32)>, AppError> {
    let rows: Vec<(String, i32)> = conn
        .query(
            "SELECT STATUS, COUNT(*) FROM LAB_ORDERS GROUP BY STATUS ORDER BY 1",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows)
}

/// Cambia el estado validando la transición:
/// SOLICITADA → RECIBIDA → EN_PROCESO → COMPLETADA; ANULADA desde cualquier
/// estado abierto. COMPLETADA exige al menos una muestra accesionada
/// finalizada y ninguna pendiente (RECIBIDA/EN_PROCESO).
pub fn set_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<LabOrder, AppError> {
    let current = current_status(conn, id)?;

    let allowed = match status {
        "RECIBIDA" => current == "SOLICITADA",
        "EN_PROCESO" => matches!(current.as_str(), "SOLICITADA" | "RECIBIDA"),
        "COMPLETADA" => {
            if !matches!(current.as_str(), "RECIBIDA" | "EN_PROCESO") {
                false
            } else {
                let open: Option<(i32,)> = conn
                    .query_first(
                        "SELECT FIRST 1 1 FROM SAMPLES
                         WHERE ORDER_ID = ? AND STATUS IN ('RECIBIDA', 'EN_PROCESO')",
                        (&id,),
                    )
                    .map_err(AppError::from)?;
                let done: Option<(i32,)> = conn
                    .query_first(
                        "SELECT FIRST 1 1 FROM SAMPLES
                         WHERE ORDER_ID = ? AND STATUS = 'FINALIZADA'",
                        (&id,),
                    )
                    .map_err(AppError::from)?;
                open.is_none() && done.is_some()
            }
        }
        "ANULADA" => ORDER_OPEN.contains(&current.as_str()),
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status}"
        )));
    }

    conn.execute(
        "UPDATE LAB_ORDERS SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP WHERE ID = ?",
        (&status, &id),
    )
    .map_err(AppError::from)?;

    get(conn, id)?.ok_or_else(|| AppError::Internal("Orden actualizada pero no recuperada".into()))
}

/// Accesiona la orden: crea una muestra (tubo) de `sample_type_id` ligada a
/// la orden y avanza SOLICITADA → RECIBIDA si corresponde. Devuelve la
/// muestra creada completa (con resultados vacíos).
pub fn accession(
    conn: &mut SimpleConnection,
    input: &AccessionOrderInput,
) -> Result<Sample, AppError> {
    let status = current_status(conn, input.order_id)?;
    if !ORDER_OPEN.contains(&status.as_str()) {
        return Err(AppError::Validation(format!(
            "No se puede accesionar una orden {status}"
        )));
    }

    // Valida el tipo de muestra.
    let st: Option<(i32,)> = conn
        .query_first(
            "SELECT 1 FROM SAMPLE_TYPES WHERE ID = ? AND IS_ACTIVE = TRUE",
            (&input.sample_type_id,),
        )
        .map_err(AppError::from)?;
    if st.is_none() {
        return Err(AppError::Validation(format!(
            "El tipo de muestra {} no existe",
            input.sample_type_id
        )));
    }

    // Crea la muestra (ORDER_ID liga la orden) igual que la recepción normal.
    let sample_id = next_id(conn, "GEN_SAMPLES_ID")?;
    let received_at = input
        .received_at
        .clone()
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    conn.execute(
        "INSERT INTO SAMPLES
            (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS, COLLECTED_BY,
             NOTES, ORDER_ID)
         VALUES (?, (SELECT PATIENT_ID FROM LAB_ORDERS WHERE ID = ?), ?, ?,
                 'RECIBIDA', ?, ?, ?)",
        (
            &sample_id,
            &input.order_id,
            &input.sample_type_id,
            &received_at,
            &input.collected_by,
            &input.notes,
            &input.order_id,
        ),
    )
    .map_err(AppError::from)?;

    // SOLICITADA → RECIBIDA la primera vez que se accesiona.
    if status == "SOLICITADA" {
        conn.execute(
            "UPDATE LAB_ORDERS SET STATUS = 'RECIBIDA', UPDATED_AT = CURRENT_TIMESTAMP WHERE ID = ?",
            (&input.order_id,),
        )
        .map_err(AppError::from)?;
    }

    samples::get(conn, sample_id)?
        .ok_or_else(|| AppError::Internal("Muestra accesionada pero no recuperada".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::lab_order::CreateLabOrderItemInput;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    fn insert_order_fixture(conn: &mut SimpleConnection, patient_id: i32, with_panel: bool) -> i32 {
        let input = CreateLabOrderInput {
            patient_id,
            consultation_id: None,
            requested_by: Some("Dra. Test".into()),
            priority: "NORMAL".into(),
            notes: None,
            requested_at: Some("2026-08-10 09:00:00".into()),
            items: vec![
                CreateLabOrderItemInput {
                    panel_id: if with_panel { Some(1) } else { None },
                    analyte_id: if with_panel { None } else { Some(1) },
                },
                CreateLabOrderItemInput {
                    panel_id: None,
                    analyte_id: Some(1),
                },
            ],
        };
        create(conn, &input).unwrap().id
    }

    #[test]
    fn test_create_and_get_order() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);

        let input = CreateLabOrderInput {
            patient_id,
            consultation_id: None,
            requested_by: Some("Dra. Ana".into()),
            priority: "URGENTE".into(),
            notes: Some("Repetir si hemólisis".into()),
            requested_at: Some("2026-08-10 09:30:00".into()),
            items: vec![CreateLabOrderItemInput {
                panel_id: Some(1),
                analyte_id: None,
            }],
        };
        let order = create(&mut conn, &input).unwrap();
        assert!(order.code.starts_with("O-2026-"));
        assert_eq!(order.status, "SOLICITADA");
        assert_eq!(order.priority, "URGENTE");
        assert_eq!(order.items.len(), 1);
        assert_eq!(
            order.items[0].panel_name.as_deref(),
            Some("Hemograma completo")
        );

        // Estado por defecto NORMAL cuando no se manda priority... (no aplica aquí)
        let got = get(&mut conn, order.id).unwrap().unwrap();
        assert_eq!(got.items.len(), 1);
        assert_eq!(got.samples.len(), 0);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_order_requires_items() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        let input = CreateLabOrderInput {
            patient_id,
            consultation_id: None,
            requested_by: None,
            priority: "NORMAL".into(),
            notes: None,
            requested_at: None,
            items: vec![],
        };
        assert!(create(&mut conn, &input).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_accession_creates_sample_and_advances_status() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);
        let order_id = insert_order_fixture(&mut conn, patient_id, true);

        let acc = AccessionOrderInput {
            order_id,
            sample_type_id: 1,
            received_at: Some("2026-08-11 08:00:00".into()),
            collected_by: Some("Técnico 1".into()),
            notes: None,
        };
        let sample = accession(&mut conn, &acc).unwrap();
        assert!(sample.code.starts_with("M-2026-"));
        assert_eq!(sample.status, "RECIBIDA");

        // La orden avanzó a RECIBIDA y lista la muestra accesionada.
        let order = get(&mut conn, order_id).unwrap().unwrap();
        assert_eq!(order.status, "RECIBIDA");
        assert_eq!(order.samples.len(), 1);
        assert_eq!(order.samples[0].id, sample.id);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_status_transitions() {
        let (mut conn, db_path) = setup();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);
        test_helpers::insert_test_analyte(&mut conn);
        let order_id = insert_order_fixture(&mut conn, patient_id, true);

        // SOLICITADA → EN_PROCESO directo está permitido.
        let o = set_status(&mut conn, order_id, "EN_PROCESO").unwrap();
        assert_eq!(o.status, "EN_PROCESO");

        // COMPLETADA sin muestras finalizadas → error.
        assert!(set_status(&mut conn, order_id, "COMPLETADA").is_err());

        // ANULADA desde EN_PROCESO.
        let o = set_status(&mut conn, order_id, "ANULADA").unwrap();
        assert_eq!(o.status, "ANULADA");

        // De ANULADA no se vuelve.
        assert!(set_status(&mut conn, order_id, "RECIBIDA").is_err());

        test_helpers::cleanup_test_db(&db_path);
    }
}
