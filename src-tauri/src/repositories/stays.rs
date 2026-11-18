use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::stay::{
    CheckOutStayInput, CreateStayEventInput, CreateStayInput, Resource, ResourceBoardItem, Stay,
    StayEvent, StayListItem,
};
use crate::repositories::next_id;

/// Tipos de estancia admitidos.
pub const STAY_KINDS: [&str; 3] = ["GUARDERIA", "HOSPEDAJE", "HOSPITALIZACION"];
/// Estados de una estancia.
pub const STAY_STATUSES: [&str; 3] = ["ABIERTA", "CERRADA", "CANCELADA"];
/// Tipos de evento de la bitácora de cuidados.
pub const STAY_EVENT_KINDS: [&str; 6] = [
    "COMIDA",
    "MEDICACION",
    "PASEO",
    "LIMPIEZA",
    "NOTA",
    "VETERINARIO",
];
/// Tipos de recurso (espacio físico).
pub const RESOURCE_KINDS: [&str; 6] = [
    "CANIL",
    "GATO",
    "RECEPERA",
    "QUIRURGICO",
    "AISLAMIENTO",
    "PERRERA",
];

type ResourceRow = (i32, String, String, i32, bool, Option<String>);

type StayRow = (
    i32,            // id
    String,         // code
    i32,            // patient_id
    String,         // patient_name
    i32,            // owner_id
    String,         // owner_name
    Option<String>, // owner_phone
    Option<i32>,    // visit_id
    Option<i32>,    // resource_id
    Option<String>, // resource_name
    String,         // kind
    String,         // status
    String,         // check_in_at
    Option<String>, // check_out_at
    f64,            // daily_rate
    Option<String>, // notes
    i32,            // days
);

type StayEventRow = (
    i32,            // id
    i32,            // stay_id
    String,         // kind
    String,         // occurred_at
    Option<String>, // notes
    Option<String>, // author_name
);

const STAY_SELECT: &str = "
    SELECT s.ID, s.CODE,
           p.ID, p.NAME,
           o.ID, o.FULL_NAME, o.PHONE,
           s.VISIT_ID, s.RESOURCE_ID, r.NAME,
           s.KIND, s.STATUS,
           LEFT(CAST(s.CHECK_IN_AT AS VARCHAR(60)), 19),
           LEFT(CAST(s.CHECK_OUT_AT AS VARCHAR(60)), 19),
           CAST(s.DAILY_RATE AS DOUBLE PRECISION),
           s.NOTES,
           -- Días de estancia (mínimo 1 el día del ingreso). Firebird 4 no
           -- tiene GREATEST, así que se usa CASE.
           CASE
               WHEN DATEDIFF(DAY FROM CAST(s.CHECK_IN_AT AS DATE)
                             TO CAST(COALESCE(s.CHECK_OUT_AT, CURRENT_TIMESTAMP) AS DATE)) < 1
                   THEN 1
               ELSE DATEDIFF(DAY FROM CAST(s.CHECK_IN_AT AS DATE)
                             TO CAST(COALESCE(s.CHECK_OUT_AT, CURRENT_TIMESTAMP) AS DATE))
           END
    FROM STAYS s
    JOIN PATIENTS p ON p.ID = s.PATIENT_ID
    JOIN OWNERS o ON o.ID = s.OWNER_ID
    LEFT JOIN RESOURCES r ON r.ID = s.RESOURCE_ID";

fn map_resource(r: ResourceRow) -> Resource {
    Resource {
        id: r.0,
        name: r.1,
        kind: r.2,
        capacity: r.3,
        active: r.4,
        notes: r.5,
    }
}

fn map_stay(r: StayRow) -> Stay {
    let days = r.16;
    Stay {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        owner_phone: r.6,
        visit_id: r.7,
        resource_id: r.8,
        resource_name: r.9,
        kind: r.10,
        status: r.11,
        check_in_at: r.12,
        check_out_at: r.13,
        daily_rate: r.14,
        notes: r.15,
        days,
        estimated_total: days as f64 * r.14,
    }
}

// ============================== RECURSOS ====================================

/// Lista los espacios físicos de la clínica (activos primero).
pub fn list_resources(
    conn: &mut SimpleConnection,
    include_inactive: bool,
) -> Result<Vec<Resource>, AppError> {
    let sql = format!(
        "SELECT FIRST 100 r.ID, r.NAME, r.KIND, r.CAPACITY, r.ACTIVE, r.NOTES
         FROM RESOURCES r
         {}
         ORDER BY r.ACTIVE DESC, r.KIND, r.NAME",
        if include_inactive {
            ""
        } else {
            "WHERE r.ACTIVE = TRUE"
        }
    );
    let rows: Vec<ResourceRow> = conn.query(&sql, ()).map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_resource).collect())
}

/// Tablero de estancias: recursos con su ocupación actual.
pub fn list_resource_board(
    conn: &mut SimpleConnection,
) -> Result<Vec<ResourceBoardItem>, AppError> {
    let rows: Vec<ResourceRow> = conn
        .query(
            "SELECT FIRST 100 r.ID, r.NAME, r.KIND, r.CAPACITY, r.ACTIVE, r.NOTES
             FROM RESOURCES r
             WHERE r.ACTIVE = TRUE
             ORDER BY r.KIND, r.NAME",
            (),
        )
        .map_err(AppError::from)?;

    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        let occupied: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM STAYS WHERE RESOURCE_ID = ? AND STATUS = 'ABIERTA'",
                (&r.0,),
            )
            .map_err(AppError::from)?;
        let needs_vet: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM STAYS
                  WHERE RESOURCE_ID = ? AND STATUS = 'ABIERTA'
                    AND KIND = 'HOSPITALIZACION'",
                (&r.0,),
            )
            .map_err(AppError::from)?;
        items.push(ResourceBoardItem {
            resource: map_resource(r),
            occupied: occupied.map(|(v,)| v).unwrap_or(0),
            needs_vet: needs_vet.map(|(v,)| v).unwrap_or(0),
        });
    }
    Ok(items)
}

/// Crea o actualiza un recurso (canil, gatera, box…).
pub fn save_resource(
    conn: &mut SimpleConnection,
    input: &crate::models::stay::SaveResourceInput,
) -> Result<Resource, AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("El nombre del recurso es requerido".into()));
    }
    let kind = input.kind.trim().to_uppercase();
    if !RESOURCE_KINDS.contains(&kind.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo de recurso inválido. Usa uno de: {}",
            RESOURCE_KINDS.join(", ")
        )));
    }
    let capacity = input.capacity.unwrap_or(1);
    if capacity < 1 {
        return Err(AppError::Validation(
            "La capacidad del recurso debe ser al menos 1".into(),
        ));
    }
    let active = input.active.unwrap_or(true);

    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE RESOURCES SET NAME = ?, KIND = ?, CAPACITY = ?, ACTIVE = ?,
                    NOTES = ?, UPDATED_AT = CURRENT_TIMESTAMP
                 WHERE ID = ?",
                (
                    &name,
                    &kind,
                    &capacity,
                    &active,
                    &input.notes,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            id
        }
        None => {
            let id = next_id(conn, "GEN_RESOURCES_ID")?;
            conn.execute(
                "INSERT INTO RESOURCES (ID, NAME, KIND, CAPACITY, ACTIVE, NOTES)
                 VALUES (?, ?, ?, ?, ?, ?)",
                (&id, &name, &kind, &capacity, &active, &input.notes),
            )
            .map_err(AppError::from)?;
            id
        }
    };

    let row: Option<ResourceRow> = conn
        .query_first(
            "SELECT ID, NAME, KIND, CAPACITY, ACTIVE, NOTES FROM RESOURCES WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    row.map(map_resource)
        .ok_or_else(|| AppError::Internal("Recurso guardado pero no recuperado".into()))
}

// =============================== ESTANCIAS ==================================

/// Lista estancias con filtros opcionales: estado, tipo y paciente.
pub fn list_stays(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    kind: Option<&str>,
    patient_id: Option<i32>,
) -> Result<Vec<StayListItem>, AppError> {
    let sql = format!(
        "{STAY_SELECT}
         WHERE (? IS NULL OR s.STATUS = ?)
           AND (? IS NULL OR s.KIND = ?)
           AND (? IS NULL OR s.PATIENT_ID = ?)
         ORDER BY CASE WHEN s.STATUS = 'ABIERTA' THEN 0 ELSE 1 END,
                  s.CHECK_IN_AT DESC"
    );
    let status_opt = status.map(|s| s.to_string());
    let kind_opt = kind.map(|s| s.to_string());

    let rows: Vec<StayRow> = conn
        .query(
            &sql,
            (
                &status_opt,
                &status_opt,
                &kind_opt,
                &kind_opt,
                &patient_id,
                &patient_id,
            ),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let stay = map_stay(r);
            StayListItem {
                id: stay.id,
                code: stay.code,
                patient_id: stay.patient_id,
                patient_name: stay.patient_name,
                owner_name: stay.owner_name,
                resource_name: stay.resource_name,
                kind: stay.kind,
                status: stay.status,
                check_in_at: stay.check_in_at,
                check_out_at: stay.check_out_at,
                days: stay.days,
                estimated_total: stay.estimated_total,
            }
        })
        .collect())
}

/// Estancia completa por id.
pub fn get_stay(conn: &mut SimpleConnection, id: i32) -> Result<Option<Stay>, AppError> {
    let row: Option<StayRow> = conn
        .query_first(&format!("{STAY_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_stay))
}

/// Ingreso de una estancia (guardería, hospedaje u hospitalización).
pub fn create_stay(
    conn: &mut SimpleConnection,
    input: &CreateStayInput,
    user_id: Option<i32>,
) -> Result<Stay, AppError> {
    let kind = input.kind.trim().to_uppercase();
    if !STAY_KINDS.contains(&kind.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo de estancia inválido. Usa uno de: {}",
            STAY_KINDS.join(", ")
        )));
    }

    // Paciente y propietario.
    let patient: Option<(i32, String)> = conn
        .query_first("SELECT ID, OWNER_ID FROM PATIENTS WHERE ID = ?", (&input.patient_id,))
        .map_err(AppError::from)?;
    let Some((_, owner_id)) = patient else {
        return Err(AppError::NotFound(format!(
            "Paciente {} no encontrado",
            input.patient_id
        )));
    };

    // El recurso debe existir y estar activo; y tener sitio libre.
    if let Some(resource_id) = input.resource_id {
        let resource: Option<(String, i32, bool)> = conn
            .query_first(
                "SELECT NAME, CAPACITY, ACTIVE FROM RESOURCES WHERE ID = ?",
                (&resource_id,),
            )
            .map_err(AppError::from)?;
        let Some((name, capacity, active)) = resource else {
            return Err(AppError::NotFound(format!(
                "Recurso {resource_id} no encontrado"
            )));
        };
        if !active {
            return Err(AppError::Validation(format!(
                "El recurso «{name}» está inactivo"
            )));
        }
        let occupied: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM STAYS WHERE RESOURCE_ID = ? AND STATUS = 'ABIERTA'",
                (&resource_id,),
            )
            .map_err(AppError::from)?;
        let occupied = occupied.map(|(v,)| v).unwrap_or(0);
        if occupied >= capacity {
            return Err(AppError::Validation(format!(
                "El recurso «{name}» no tiene espacio disponible"
            )));
        }
    }

    let id = next_id(conn, "GEN_STAYS_ID")?;
    let check_in = input.check_in_at.clone().unwrap_or_else(|| "CURRENT_TIMESTAMP".into());
    let daily_rate = input.daily_rate.unwrap_or(0.0);

    // La fecha de ingreso se enlaza como parámetro o se deja en
    // CURRENT_TIMESTAMP: Firebird no admite un parámetro dentro de una
    // expresión, así que cada variante lleva su propia lista de parámetros.
    if input.check_in_at.is_some() {
        conn.execute(
            "INSERT INTO STAYS (ID, PATIENT_ID, OWNER_ID, VISIT_ID, RESOURCE_ID, KIND,
                                 STATUS, CHECK_IN_AT, DAILY_RATE, NOTES, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?, 'ABIERTA', ?, ?, ?, ?)",
            (
                &id,
                &input.patient_id,
                &owner_id,
                &input.visit_id,
                &input.resource_id,
                &kind,
                &check_in,
                &daily_rate,
                &input.notes,
                &user_id,
            ),
        )
    } else {
        conn.execute(
            "INSERT INTO STAYS (ID, PATIENT_ID, OWNER_ID, VISIT_ID, RESOURCE_ID, KIND,
                                 STATUS, CHECK_IN_AT, DAILY_RATE, NOTES, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?, 'ABIERTA', CURRENT_TIMESTAMP, ?, ?, ?)",
            (
                &id,
                &input.patient_id,
                &owner_id,
                &input.visit_id,
                &input.resource_id,
                &kind,
                &daily_rate,
                &input.notes,
                &user_id,
            ),
        )
    }
    .map_err(AppError::from)?;

    get_stay(conn, id)?
        .ok_or_else(|| AppError::Internal("Estancia creada pero no recuperada".into()))
}

/// Check-out de una estancia: la cierra y libera el recurso.
pub fn check_out_stay(
    conn: &mut SimpleConnection,
    id: i32,
    input: &CheckOutStayInput,
) -> Result<Stay, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM STAYS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let Some((status,)) = current else {
        return Err(AppError::NotFound(format!("Estancia {id} no encontrada")));
    };
    if status != "ABIERTA" {
        return Err(AppError::Validation(
            "Solo se puede cerrar una estancia abierta".into(),
        ));
    }

    // La fecha de salida se enlaza o se deja en CURRENT_TIMESTAMP: cada
    // variante lleva su propia lista de parámetros.
    if input.check_out_at.is_some() {
        let check_out = input.check_out_at.clone().unwrap_or_default();
        conn.execute(
            "UPDATE STAYS SET STATUS = 'CERRADA', CHECK_OUT_AT = ?,
                NOTES = COALESCE(?, NOTES), UPDATED_AT = CURRENT_TIMESTAMP
             WHERE ID = ?",
            (&check_out, &input.notes, &id),
        )
    } else {
        conn.execute(
            "UPDATE STAYS SET STATUS = 'CERRADA', CHECK_OUT_AT = CURRENT_TIMESTAMP,
                NOTES = COALESCE(?, NOTES), UPDATED_AT = CURRENT_TIMESTAMP
             WHERE ID = ?",
            (&input.notes, &id),
        )
    }
    .map_err(AppError::from)?;

    get_stay(conn, id)?
        .ok_or_else(|| AppError::Internal("Estancia cerrada pero no recuperada".into()))
}

/// Cancela una estancia abierta (ingreso mal registrado).
pub fn cancel_stay(conn: &mut SimpleConnection, id: i32) -> Result<Stay, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM STAYS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let Some((status,)) = current else {
        return Err(AppError::NotFound(format!("Estancia {id} no encontrada")));
    };
    if status != "ABIERTA" {
        return Err(AppError::Validation(
            "Solo se puede cancelar una estancia abierta".into(),
        ));
    }
    conn.execute(
        "UPDATE STAYS SET STATUS = 'CANCELADA', CHECK_OUT_AT = CURRENT_TIMESTAMP,
            UPDATED_AT = CURRENT_TIMESTAMP
         WHERE ID = ?",
        (&id,),
    )
    .map_err(AppError::from)?;

    get_stay(conn, id)?
        .ok_or_else(|| AppError::Internal("Estancia cancelada pero no recuperada".into()))
}

// ============================== BITÁCORA ====================================

/// Añade un evento a la bitácora de cuidados de la estancia.
pub fn add_stay_event(
    conn: &mut SimpleConnection,
    stay_id: i32,
    input: &CreateStayEventInput,
    user_id: Option<i32>,
) -> Result<StayEvent, AppError> {
    let exists: Option<(i32,)> = conn
        .query_first("SELECT ID FROM STAYS WHERE ID = ?", (&stay_id,))
        .map_err(AppError::from)?;
    if exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Estancia {stay_id} no encontrada"
        )));
    }

    let kind = input.kind.trim().to_uppercase();
    if !STAY_EVENT_KINDS.contains(&kind.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo de evento inválido. Usa uno de: {}",
            STAY_EVENT_KINDS.join(", ")
        )));
    }

    let id = next_id(conn, "GEN_STAY_EVENTS_ID")?;
    let occurred = input.occurred_at.clone().unwrap_or_default();
    // Igual que en el alta de estancia: cada variante lleva su propia lista
    // de parámetros, porque CURRENT_TIMESTAMP no ocupa un marcador.
    if input.occurred_at.is_some() {
        conn.execute(
            "INSERT INTO STAY_EVENTS (ID, STAY_ID, KIND, OCCURRED_AT, NOTES, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?)",
            (&id, &stay_id, &kind, &occurred, &input.notes, &user_id),
        )
    } else {
        conn.execute(
            "INSERT INTO STAY_EVENTS (ID, STAY_ID, KIND, OCCURRED_AT, NOTES, CREATED_BY)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP, ?, ?)",
            (&id, &stay_id, &kind, &input.notes, &user_id),
        )
    }
    .map_err(AppError::from)?;

    let row: Option<StayEventRow> = conn
        .query_first(
            "SELECT e.ID, e.STAY_ID, e.KIND,
                    LEFT(CAST(e.OCCURRED_AT AS VARCHAR(60)), 19), e.NOTES, u.FULL_NAME
             FROM STAY_EVENTS e
             LEFT JOIN USERS u ON u.ID = e.CREATED_BY
             WHERE e.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    row.map(|r| StayEvent {
        id: r.0,
        stay_id: r.1,
        kind: r.2,
        occurred_at: r.3,
        notes: r.4,
        author_name: r.5,
    })
    .ok_or_else(|| AppError::Internal("Evento creado pero no recuperado".into()))
}

/// Bitácora de una estancia (más reciente primero).
pub fn list_stay_events(
    conn: &mut SimpleConnection,
    stay_id: i32,
) -> Result<Vec<StayEvent>, AppError> {
    let rows: Vec<StayEventRow> = conn
        .query(
            "SELECT e.ID, e.STAY_ID, e.KIND,
                    LEFT(CAST(e.OCCURRED_AT AS VARCHAR(60)), 19), e.NOTES, u.FULL_NAME
             FROM STAY_EVENTS e
             LEFT JOIN USERS u ON u.ID = e.CREATED_BY
             WHERE e.STAY_ID = ?
             ORDER BY e.OCCURRED_AT DESC, e.ID DESC",
            (&stay_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| StayEvent {
            id: r.0,
            stay_id: r.1,
            kind: r.2,
            occurred_at: r.3,
            notes: r.4,
            author_name: r.5,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_save_resource_and_board() {
        let (mut conn, db_path) = setup();

        let resource = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: None,
                name: "Canil 1".to_string(),
                kind: "CANIL".to_string(),
                capacity: Some(1),
                active: None,
                notes: Some("Near the window".to_string()),
            },
        )
        .unwrap();
        assert_eq!(resource.name, "Canil 1");
        assert_eq!(resource.kind, "CANIL");
        assert_eq!(resource.capacity, 1);
        assert!(resource.active);

        // El tablero lo muestra con occupancy 0.
        let board = list_resource_board(&mut conn).unwrap();
        assert_eq!(board.len(), 1);
        assert_eq!(board[0].resource.id, resource.id);
        assert_eq!(board[0].occupied, 0);

        // Editar: cambia nombre y capacidad.
        let updated = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: Some(resource.id),
                name: "Canil 1 (grande)".to_string(),
                kind: "CANIL".to_string(),
                capacity: Some(2),
                active: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Canil 1 (grande)");
        assert_eq!(updated.capacity, 2);

        // Los inactivos se ocultan del listado por defecto.
        save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: Some(resource.id),
                name: "Canil 1 (grande)".to_string(),
                kind: "CANIL".to_string(),
                capacity: Some(2),
                active: Some(false),
                notes: None,
            },
        )
        .unwrap();
        assert!(list_resources(&mut conn, false).unwrap().is_empty());
        assert_eq!(list_resources(&mut conn, true).unwrap().len(), 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_save_resource_validation() {
        let (mut conn, db_path) = setup();

        let err = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: None,
                name: "  ".to_string(),
                kind: "CANIL".to_string(),
                capacity: None,
                active: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        let err = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: None,
                name: "Canil".to_string(),
                kind: "INVALIDO".to_string(),
                capacity: None,
                active: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_stay_and_check_out() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let resource = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: None,
                name: "Canil A".to_string(),
                kind: "CANIL".to_string(),
                capacity: Some(1),
                active: None,
                notes: None,
            },
        )
        .unwrap();

        let stay = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: Some(resource.id),
                kind: "HOSPEDAJE".to_string(),
                check_in_at: None,
                daily_rate: Some(45000.0),
                notes: Some("Trae su comida".to_string()),
            },
            None,
        )
        .unwrap();

        assert_eq!(stay.status, "ABIERTA");
        assert_eq!(stay.kind, "HOSPEDAJE");
        assert_eq!(stay.patient_name, "Luna");
        assert_eq!(stay.owner_name, "Juan Pérez");
        assert_eq!(stay.resource_name.as_deref(), Some("Canil A"));
        assert!(stay.code.starts_with("E-"));
        assert_eq!(stay.days, 1);
        assert!((stay.estimated_total - 45000.0).abs() < 0.01);

        // El tablero refleja la ocupación.
        let board = list_resource_board(&mut conn).unwrap();
        assert_eq!(board[0].occupied, 1);

        // Listado de estancias.
        let open = list_stays(&mut conn, Some("ABIERTA"), None, None).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].code, stay.code);

        // Check-out: la estancia queda cerrada.
        let closed = check_out_stay(
            &mut conn,
            stay.id,
            &CheckOutStayInput {
                check_out_at: None,
                notes: Some("Se fue-owner".to_string()),
            },
        )
        .unwrap();
        assert_eq!(closed.status, "CERRADA");
        assert!(closed.check_out_at.is_some());

        // El recurso vuelve a estar libre.
        let board = list_resource_board(&mut conn).unwrap();
        assert_eq!(board[0].occupied, 0);

        // No se puede cerrar dos veces.
        let err = check_out_stay(
            &mut conn,
            stay.id,
            &CheckOutStayInput {
                check_out_at: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_stay_resource_capacity_and_validation() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let resource = save_resource(
            &mut conn,
            &crate::models::stay::SaveResourceInput {
                id: None,
                name: "Gatera 1".to_string(),
                kind: "GATO".to_string(),
                capacity: Some(1),
                active: None,
                notes: None,
            },
        )
        .unwrap();

        create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: Some(resource.id),
                kind: "GUARDERIA".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap();

        // Sin sitio: la segunda estancia se rechaza.
        let err = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: Some(resource.id),
                kind: "GUARDERIA".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Tipo de estancia inválido.
        let err = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: None,
                kind: "PISCINA".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Paciente inexistente.
        let err = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id: 9999,
                visit_id: None,
                resource_id: None,
                kind: "GUARDERIA".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_stay_events_log() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let stay = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: None,
                kind: "HOSPITALIZACION".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap();

        let event = add_stay_event(
            &mut conn,
            stay.id,
            &CreateStayEventInput {
                kind: "MEDICACION".to_string(),
                occurred_at: None,
                notes: Some("Antibiótico 1 ml".to_string()),
            },
            None,
        )
        .unwrap();
        assert_eq!(event.kind, "MEDICACION");
        assert_eq!(event.stay_id, stay.id);

        let events = list_stay_events(&mut conn, stay.id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].notes.as_deref(), Some("Antibiótico 1 ml"));

        // Tipo de evento inválido.
        let err = add_stay_event(
            &mut conn,
            stay.id,
            &CreateStayEventInput {
                kind: "BAILE".to_string(),
                occurred_at: None,
                notes: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_cancel_stay() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let stay = create_stay(
            &mut conn,
            &CreateStayInput {
                patient_id,
                visit_id: None,
                resource_id: None,
                kind: "GUARDERIA".to_string(),
                check_in_at: None,
                daily_rate: None,
                notes: None,
            },
            None,
        )
        .unwrap();

        let cancelled = cancel_stay(&mut conn, stay.id).unwrap();
        assert_eq!(cancelled.status, "CANCELADA");

        // Ya no se puede cancelar.
        let err = cancel_stay(&mut conn, stay.id).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        cleanup_test_db(&db_path);
    }
}
