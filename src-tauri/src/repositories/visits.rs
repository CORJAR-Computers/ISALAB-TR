use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::visit::{
    AddVisitServiceInput, UpdateVisitServiceStatusInput, Visit, VisitListItem, VisitService,
};
use crate::repositories::invoices as invoices_repo;
use crate::repositories::next_id;

/// Fila cruda de la cola y del historial de visitas (columnas leídas de Firebird).
type VisitListRow = (
    i32,
    String,
    i32,
    String,
    i32,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    f64,
);

/// Snapshot del catálogo de servicios para una línea de visita.
type ServiceSnapshot = (String, String, i32, String, f64, Option<f64>, bool);

/// Estados válidos de una visita (migración 0027).
pub const VISIT_STATUSES: [&str; 5] = [
    "EN_ESPERA",
    "EN_ATENCION",
    "FINALIZADA",
    "FACTURADA",
    "CANCELADA",
];

/// Estados válidos de una línea de servicio de visita.
pub const VISIT_SERVICE_STATUSES: [&str; 4] =
    ["PENDIENTE", "EN_PROCESO", "COMPLETADA", "CANCELADA"];

// ============================== HELPERS =====================================

type VisitServiceRow = (
    i32,            // id
    i32,            // visit_id
    i32,            // service_id
    String,         // service name
    String,         // service code
    i32,            // category_id
    String,         // category_name
    Option<i64>,    // professional_id (rsfbclient lee INTEGER NULL como i64)
    Option<String>, // professional_name
    i32,            // quantity
    f64,            // unit_price
    Option<f64>,    // tax_rate
    String,         // status
    Option<String>, // started_at
    Option<String>, // finished_at
    Option<String>, // notes
    i32,            // seq
);

fn map_visit_service(r: VisitServiceRow) -> VisitService {
    VisitService {
        id: r.0,
        visit_id: r.1,
        service_id: r.2,
        service_name: r.3,
        service_code: r.4,
        category_id: r.5,
        category_name: r.6,
        professional_id: r.7.map(|v| v as i32),
        professional_name: r.8,
        quantity: r.9,
        unit_price: r.10,
        tax_rate: r.11,
        status: r.12,
        started_at: r.13,
        finished_at: r.14,
        notes: r.15,
        seq: r.16,
    }
}

const VISIT_SERVICE_SELECT: &str = "
    SELECT vs.ID, vs.VISIT_ID, vs.SERVICE_ID, s.NAME, s.CODE,
           s.CATEGORY_ID, c.NAME,
           vs.PROFESSIONAL_ID, u.FULL_NAME,
           vs.QUANTITY,
           CAST(vs.UNIT_PRICE AS DOUBLE PRECISION),
           CAST(vs.TAX_RATE AS DOUBLE PRECISION),
           vs.STATUS,
           LEFT(CAST(vs.STARTED_AT AS VARCHAR(60)), 19),
           LEFT(CAST(vs.FINISHED_AT AS VARCHAR(60)), 19),
           vs.NOTES, vs.SEQ
    FROM VISIT_SERVICES vs
    JOIN SERVICES s ON s.ID = vs.SERVICE_ID
    JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
    LEFT JOIN USERS u ON u.ID = vs.PROFESSIONAL_ID";

fn list_services_of_visit(
    conn: &mut SimpleConnection,
    visit_id: i32,
) -> Result<Vec<VisitService>, AppError> {
    let rows: Vec<VisitServiceRow> = conn
        .query(
            &format!("{VISIT_SERVICE_SELECT} WHERE vs.VISIT_ID = ? ORDER BY vs.SEQ, vs.ID"),
            (&visit_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_visit_service).collect())
}

type VisitHeaderRow = (
    i32,            // id
    String,         // code
    i32,            // patient_id
    String,         // patient_name
    i32,            // owner_id
    String,         // owner_name
    Option<i64>,    // appointment_id (rsfbclient lee INTEGER con valor como i64)
    String,         // arrived_at
    String,         // priority
    String,         // status
    Option<String>, // notes
    Option<String>, // created_by_name
);

const VISIT_SELECT: &str = "
    SELECT v.ID, v.CODE, v.PATIENT_ID, p.NAME,
           v.OWNER_ID, o.FULL_NAME,
           v.APPOINTMENT_ID,
           LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 19),
           v.PRIORITY, v.STATUS, v.NOTES,
           cu.FULL_NAME
    FROM VISITS v
    JOIN PATIENTS p ON p.ID = v.PATIENT_ID
    JOIN OWNERS o ON o.ID = v.OWNER_ID
    LEFT JOIN USERS cu ON cu.ID = v.CREATED_BY";

/// Visita completa o `None`.
pub fn get_visit(conn: &mut SimpleConnection, id: i32) -> Result<Option<Visit>, AppError> {
    let row: Option<VisitHeaderRow> = conn
        .query_first(&format!("{VISIT_SELECT} WHERE v.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    let Some(r) = row else { return Ok(None) };

    let services = list_services_of_visit(conn, id)?;
    Ok(Some(Visit {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        appointment_id: r.6.map(|v| v as i32),
        arrived_at: r.7,
        priority: r.8,
        status: r.9,
        notes: r.10,
        created_by_name: r.11,
        services,
    }))
}

/// IVA global de la clínica (CLINIC_SETTINGS 'invoice.tax_rate'); 19% de
/// respaldo si la clave no existe (igual que en facturación).
fn default_tax_rate(conn: &mut SimpleConnection) -> Result<f64, AppError> {
    let row: Option<(Option<String>,)> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM CLINIC_SETTINGS WHERE KEY_NAME = 'invoice.tax_rate'",
            (),
        )
        .map_err(AppError::from)?;
    Ok(row
        .and_then(|(v,)| v)
        .and_then(|v| v.parse().ok())
        .unwrap_or(19.0))
}

// ============================== CREAR =======================================

/// Registra el check-in de una visita con sus servicios iniciales. El precio
/// de cada línea se congela desde el catálogo (SERVICES) en este momento.
/// `appointment_id` vincula la visita a la cita de origen (fase 3).
pub fn create_visit(
    conn: &mut SimpleConnection,
    input: &crate::models::visit::CreateVisitInput,
    created_by: Option<i32>,
    appointment_id: Option<i32>,
) -> Result<Visit, AppError> {
    let priority = input.priority.as_deref().unwrap_or("NORMAL");
    if !matches!(priority, "NORMAL" | "URGENTE") {
        return Err(AppError::Validation(format!(
            "Prioridad inválida: {priority} (esperado NORMAL o URGENTE)"
        )));
    }
    if input.services.is_empty() {
        return Err(AppError::Validation(
            "La visita debe tener al menos un servicio".into(),
        ));
    }

    // Propietario: si no llega, el del paciente.
    let patient: Option<(i32,)> = conn
        .query_first(
            "SELECT OWNER_ID FROM PATIENTS WHERE ID = ?",
            (&input.patient_id,),
        )
        .map_err(AppError::from)?;
    let Some((patient_owner,)) = patient else {
        return Err(AppError::NotFound(format!(
            "Paciente {} no encontrado",
            input.patient_id
        )));
    };
    let owner_id = input.owner_id.unwrap_or(patient_owner);

    // Cabecera + líneas de servicio en una sola transacción: una visita sin
    // todas sus líneas distorsiona la cola, la facturación y el tablero.
    db::tx::in_transaction(conn, |conn| {
        let id = next_id(conn, "GEN_VISITS_ID")?;
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, APPOINTMENT_ID, PRIORITY, NOTES, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (&id, &input.patient_id, &owner_id, &appointment_id, &priority, &input.notes, &created_by),
        )
        .map_err(AppError::from)?;

        for svc in &input.services {
            insert_visit_service(
                conn,
                id,
                svc.service_id,
                svc.professional_id,
                svc.quantity,
                svc.notes.as_deref(),
            )?;
        }

        get_visit(conn, id)?
            .ok_or_else(|| AppError::Internal("Visita creada pero no recuperada".into()))
    })
}

// ============================== LISTADO =====================================

/// Lista la cola de visitas con filtros: fecha (YYYY-MM-DD), estado y texto
/// libre (código, paciente o propietario). Urgentes primero, luego por llegada.
pub fn list_visits(
    conn: &mut SimpleConnection,
    date: Option<&str>,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<VisitListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT v.ID, v.CODE, v.PATIENT_ID, p.NAME,
               v.OWNER_ID, o.FULL_NAME,
               LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 19),
               v.PRIORITY, v.STATUS, v.NOTES,
               COALESCE((SELECT LIST(s2.NAME, ' + ')
                           FROM VISIT_SERVICES vs2
                           JOIN SERVICES s2 ON s2.ID = vs2.SERVICE_ID
                          WHERE vs2.VISIT_ID = v.ID
                            AND vs2.STATUS <> 'CANCELADA'), ''),
               COALESCE((SELECT SUM(CAST(vs3.UNIT_PRICE AS DOUBLE PRECISION) * vs3.QUANTITY)
                           FROM VISIT_SERVICES vs3
                          WHERE vs3.VISIT_ID = v.ID
                            AND vs3.STATUS <> 'CANCELADA'), 0.0)
        FROM VISITS v
        JOIN PATIENTS p ON p.ID = v.PATIENT_ID
        JOIN OWNERS o ON o.ID = v.OWNER_ID
        WHERE (? IS NULL OR LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 10) = ?)
          AND (? IS NULL OR v.STATUS = ?)
          AND (? IS NULL
               OR UPPER(v.CODE) LIKE UPPER(?)
               OR UPPER(p.NAME) LIKE UPPER(?)
               OR UPPER(o.FULL_NAME) LIKE UPPER(?))
        ORDER BY CASE v.PRIORITY WHEN 'URGENTE' THEN 0 ELSE 1 END, v.ARRIVED_AT, v.ID";

    let rows: Vec<VisitListRow> = conn
        .query(
            sql,
            (&date, &date, &status, &status, &like, &like, &like, &like),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| VisitListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            owner_id: r.4,
            owner_name: r.5,
            arrived_at: r.6,
            priority: r.7,
            status: r.8,
            notes: r.9,
            services_summary: r.10,
            estimated_total: r.11,
        })
        .collect())
}

/// Historial: todas las visitas de un paciente (recepción), de la más
/// reciente a la más antigua, con el resumen de servicios y el total estimado.
pub fn list_visits_by_patient(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<VisitListItem>, AppError> {
    let sql = "
        SELECT v.ID, v.CODE, v.PATIENT_ID, p.NAME,
               v.OWNER_ID, o.FULL_NAME,
               LEFT(CAST(v.ARRIVED_AT AS VARCHAR(60)), 19),
               v.PRIORITY, v.STATUS, v.NOTES,
               COALESCE((SELECT LIST(s2.NAME, ' + ')
                           FROM VISIT_SERVICES vs2
                           JOIN SERVICES s2 ON s2.ID = vs2.SERVICE_ID
                          WHERE vs2.VISIT_ID = v.ID
                            AND vs2.STATUS <> 'CANCELADA'), ''),
               COALESCE((SELECT SUM(CAST(vs3.UNIT_PRICE AS DOUBLE PRECISION) * vs3.QUANTITY)
                           FROM VISIT_SERVICES vs3
                          WHERE vs3.VISIT_ID = v.ID
                            AND vs3.STATUS <> 'CANCELADA'), 0.0)
        FROM VISITS v
        JOIN PATIENTS p ON p.ID = v.PATIENT_ID
        JOIN OWNERS o ON o.ID = v.OWNER_ID
        WHERE v.PATIENT_ID = ?
        ORDER BY v.ARRIVED_AT DESC, v.ID DESC";

    let rows: Vec<VisitListRow> = conn.query(sql, (&patient_id,)).map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| VisitListItem {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            patient_name: r.3,
            owner_id: r.4,
            owner_name: r.5,
            arrived_at: r.6,
            priority: r.7,
            status: r.8,
            notes: r.9,
            services_summary: r.10,
            estimated_total: r.11,
        })
        .collect())
}

// ============================== ESTADOS =====================================
/// Cambia el estado de la visita validando la transición.
/// FACTURADA solo se establece vía `checkout_visit`.
pub fn set_visit_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<Visit, AppError> {
    // Estados de origen admitidos por cada transición.
    let from: &[&str] = match status {
        "EN_ATENCION" => &["EN_ESPERA"],
        "FINALIZADA" => &["EN_ATENCION"],
        "CANCELADA" => &["EN_ESPERA", "EN_ATENCION", "FINALIZADA"],
        "FACTURADA" => &["FINALIZADA"],
        _ => &[],
    };
    if from.is_empty() {
        return Err(AppError::Validation(format!(
            "Estado de visita inválido: {status}"
        )));
    }

    // Compare-and-set: si otra PC ya movió la visita (p. ej. la facturó), este
    // UPDATE afecta 0 filas en lugar de devolverla a un estado anterior.
    // Los estados son un conjunto fijo interno: se interpolan sin riesgo.
    let allowed = from
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE VISITS SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ? AND STATUS IN ({allowed})"
    );
    let updated = conn.execute(&sql, (&status, &id)).map_err(AppError::from)?;

    if updated == 0 {
        let current: Option<(String,)> = conn
            .query_first("SELECT STATUS FROM VISITS WHERE ID = ?", (&id,))
            .map_err(AppError::from)?;
        let Some((current,)) = current else {
            return Err(AppError::NotFound(format!("Visita {id} no encontrada")));
        };
        return Err(AppError::Validation(format!(
            "Transición de estado inválida: {current} → {status}"
        )));
    }

    get_visit(conn, id)?
        .ok_or_else(|| AppError::Internal("Visita actualizada pero no recuperada".into()))
}

// ============================ SERVICIOS DE VISITA ===========================

/// Valida y agrega una línea de servicio a la visita, congelando precio e IVA
/// desde el catálogo. Evita duplicar un servicio con línea activa (ajusta la
/// cantidad de la línea existente en su lugar).
pub fn add_visit_service(
    conn: &mut SimpleConnection,
    visit_id: i32,
    input: &AddVisitServiceInput,
) -> Result<VisitService, AppError> {
    if input.quantity <= 0 {
        return Err(AppError::Validation(
            "La cantidad debe ser mayor que cero".into(),
        ));
    }

    let visit_status: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM VISITS WHERE ID = ?", (&visit_id,))
        .map_err(AppError::from)?;
    let (visit_status,) = visit_status
        .ok_or_else(|| AppError::NotFound(format!("Visita {visit_id} no encontrada")))?;
    if matches!(visit_status.as_str(), "FACTURADA" | "CANCELADA") {
        return Err(AppError::Validation(
            "No se pueden agregar servicios a una visita facturada o cancelada".into(),
        ));
    }

    insert_visit_service(
        conn,
        visit_id,
        input.service_id,
        input.professional_id,
        input.quantity,
        input.notes.as_deref(),
    )
}

/// Inserta la línea (con snapshot de precio). Usado por `create_visit` y
/// `add_visit_service`.
fn insert_visit_service(
    conn: &mut SimpleConnection,
    visit_id: i32,
    service_id: i32,
    professional_id: Option<i32>,
    quantity: i32,
    notes: Option<&str>,
) -> Result<VisitService, AppError> {
    if quantity <= 0 {
        return Err(AppError::Validation(
            "La cantidad debe ser mayor que cero".into(),
        ));
    }

    // Snapshot del catálogo: precio, IVA y datos para mostrar.
    let snap: Option<ServiceSnapshot> = conn
        .query_first(
            "SELECT s.CODE, s.NAME, s.CATEGORY_ID, c.NAME,
                    CAST(s.PRICE AS DOUBLE PRECISION),
                    CAST(s.TAX_RATE AS DOUBLE PRECISION),
                    s.IS_ACTIVE
             FROM SERVICES s
             JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
             WHERE s.ID = ?",
            (&service_id,),
        )
        .map_err(AppError::from)?;
    let Some((s_code, s_name, category_id, category_name, price, tax_rate, is_active)) = snap
    else {
        return Err(AppError::NotFound(format!(
            "Servicio {service_id} no encontrado en el catálogo"
        )));
    };
    if !is_active {
        return Err(AppError::Validation(format!(
            "El servicio {s_code} está inactivo en el catálogo"
        )));
    }

    // Sin líneas duplicadas activas del mismo servicio.
    let dup: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM VISIT_SERVICES
              WHERE VISIT_ID = ? AND SERVICE_ID = ? AND STATUS <> 'CANCELADA'",
            (&visit_id, &service_id),
        )
        .map_err(AppError::from)?;
    if dup.is_some() {
        return Err(AppError::Validation(format!(
            "El servicio {s_code} ya está en la visita; ajusta la cantidad de la línea existente"
        )));
    }

    // Profesional (si llega) debe existir.
    if let Some(pid) = professional_id {
        let exists: Option<(i32,)> = conn
            .query_first("SELECT ID FROM USERS WHERE ID = ?", (&pid,))
            .map_err(AppError::from)?;
        if exists.is_none() {
            return Err(AppError::Validation(format!(
                "El profesional {pid} no existe"
            )));
        }
    }

    let seq: (i32,) = conn
        .query_first(
            "SELECT COALESCE(MAX(SEQ), 0) + 1 FROM VISIT_SERVICES WHERE VISIT_ID = ?",
            (&visit_id,),
        )
        .map_err(AppError::from)?
        .unwrap_or((1,));

    let line_id = next_id(conn, "GEN_VISIT_SERVICES_ID")?;
    conn.execute(
        "INSERT INTO VISIT_SERVICES
            (ID, VISIT_ID, SERVICE_ID, PROFESSIONAL_ID, QUANTITY,
             UNIT_PRICE, TAX_RATE, STATUS, NOTES, SEQ)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'PENDIENTE', ?, ?)",
        (
            &line_id,
            &visit_id,
            &service_id,
            &professional_id,
            &quantity,
            &price,
            &tax_rate,
            &notes,
            &seq.0,
        ),
    )
    .map_err(AppError::from)?;

    let _ = (s_name, category_id, category_name); // usados por el mapeo de abajo
    let rows: Vec<VisitServiceRow> = conn
        .query(
            &format!("{VISIT_SERVICE_SELECT} WHERE vs.ID = ?"),
            (&line_id,),
        )
        .map_err(AppError::from)?;
    rows.into_iter()
        .next()
        .map(map_visit_service)
        .ok_or_else(|| AppError::Internal("Línea creada pero no recuperada".into()))
}

/// Quita una línea PENDIENTE de una visita aún no facturada/cancelada.
pub fn remove_visit_service(
    conn: &mut SimpleConnection,
    visit_id: i32,
    line_id: i32,
) -> Result<(), AppError> {
    let row: Option<(String, String)> = conn
        .query_first(
            "SELECT vs.STATUS, v.STATUS
             FROM VISIT_SERVICES vs JOIN VISITS v ON v.ID = vs.VISIT_ID
             WHERE vs.ID = ? AND vs.VISIT_ID = ?",
            (&line_id, &visit_id),
        )
        .map_err(AppError::from)?;
    let Some((line_status, visit_status)) = row else {
        return Err(AppError::NotFound(format!(
            "Línea {line_id} no encontrada en la visita {visit_id}"
        )));
    };
    if matches!(visit_status.as_str(), "FACTURADA" | "CANCELADA") {
        return Err(AppError::Validation(
            "No se pueden quitar servicios de una visita facturada o cancelada".into(),
        ));
    }
    if line_status != "PENDIENTE" {
        return Err(AppError::Validation(format!(
            "Solo se pueden quitar líneas pendientes (esta está {line_status})"
        )));
    }

    conn.execute("DELETE FROM VISIT_SERVICES WHERE ID = ?", (&line_id,))
        .map_err(AppError::from)?;
    Ok(())
}

/// Cambia el estado de una línea de servicio (flujo de trabajo del
/// profesional): PENDIENTE → EN_PROCESO → COMPLETADA (o CANCELADA).
/// Registra STARTED_AT / FINISHED_AT automáticamente.
pub fn update_visit_service_status(
    conn: &mut SimpleConnection,
    visit_id: i32,
    line_id: i32,
    input: &UpdateVisitServiceStatusInput,
) -> Result<VisitService, AppError> {
    let row: Option<(String, String)> = conn
        .query_first(
            "SELECT vs.STATUS, v.STATUS
             FROM VISIT_SERVICES vs JOIN VISITS v ON v.ID = vs.VISIT_ID
             WHERE vs.ID = ? AND vs.VISIT_ID = ?",
            (&line_id, &visit_id),
        )
        .map_err(AppError::from)?;
    let Some((line_status, visit_status)) = row else {
        return Err(AppError::NotFound(format!(
            "Línea {line_id} no encontrada en la visita {visit_id}"
        )));
    };
    if matches!(visit_status.as_str(), "FACTURADA" | "CANCELADA") {
        return Err(AppError::Validation(
            "La visita ya está facturada o cancelada; sus líneas no cambian".into(),
        ));
    }

    let allowed = match input.status.as_str() {
        "EN_PROCESO" => line_status == "PENDIENTE",
        "COMPLETADA" => matches!(line_status.as_str(), "PENDIENTE" | "EN_PROCESO"),
        "CANCELADA" => matches!(line_status.as_str(), "PENDIENTE" | "EN_PROCESO"),
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de línea inválida: {line_status} → {}",
            input.status
        )));
    }

    conn.execute(
        "UPDATE VISIT_SERVICES
            SET STATUS = ?,
                STARTED_AT = CASE WHEN ? = 'EN_PROCESO' THEN CURRENT_TIMESTAMP ELSE STARTED_AT END,
                FINISHED_AT = CASE WHEN ? = 'COMPLETADA' THEN CURRENT_TIMESTAMP ELSE FINISHED_AT END,
                PROFESSIONAL_ID = COALESCE(?, PROFESSIONAL_ID)
          WHERE ID = ?",
        (
            &input.status,
            &input.status,
            &input.status,
            &input.professional_id,
            &line_id,
        ),
    )
    .map_err(AppError::from)?;

    let rows: Vec<VisitServiceRow> = conn
        .query(
            &format!("{VISIT_SERVICE_SELECT} WHERE vs.ID = ?"),
            (&line_id,),
        )
        .map_err(AppError::from)?;
    rows.into_iter()
        .next()
        .map(map_visit_service)
        .ok_or_else(|| AppError::Internal("Línea actualizada pero no recuperada".into()))
}

// ============================== CHECKOUT ====================================

/// Cobra la visita: genera la factura a partir de las líneas no canceladas
/// (precios congelados) y marca la visita como FACTURADA. Una visita solo se
/// puede facturar una vez (guard en Rust; Firebird no soporta UNIQUE parcial).
///
/// El IVA se calcula por línea: la tarifa congelada del servicio, o la global
/// de la clínica si la línea no define una. El INVOICES.TAX_RATE guarda la
/// global como referencia.
pub fn checkout_visit(
    conn: &mut SimpleConnection,
    visit_id: i32,
    payment_method: Option<&str>,
    notes: Option<&str>,
) -> Result<crate::models::invoice::Invoice, AppError> {
    let header: Option<(String, String, i32, i32)> = conn
        .query_first(
            "SELECT CODE, STATUS, PATIENT_ID, OWNER_ID FROM VISITS WHERE ID = ?",
            (&visit_id,),
        )
        .map_err(AppError::from)?;
    let Some((_code, status, patient_id, owner_id)) = header else {
        return Err(AppError::NotFound(format!(
            "Visita {visit_id} no encontrada"
        )));
    };
    if status != "FINALIZADA" {
        return Err(AppError::Validation(format!(
            "Solo se pueden facturar visitas FINALIZADA (esta está {status})"
        )));
    }

    // Guard: una factura por visita.
    let already: Option<(i32,)> = conn
        .query_first("SELECT ID FROM INVOICES WHERE VISIT_ID = ?", (&visit_id,))
        .map_err(AppError::from)?;
    if already.is_some() {
        return Err(AppError::Validation(
            "Esta visita ya tiene una factura asociada".into(),
        ));
    }

    // Líneas a cobrar (snapshot congelado).
    let lines: Vec<(i32, String, i32, f64, Option<f64>)> = conn
        .query(
            "SELECT vs.SERVICE_ID, s.NAME, vs.QUANTITY,
                    CAST(vs.UNIT_PRICE AS DOUBLE PRECISION),
                    CAST(vs.TAX_RATE AS DOUBLE PRECISION)
             FROM VISIT_SERVICES vs
             JOIN SERVICES s ON s.ID = vs.SERVICE_ID
             WHERE vs.VISIT_ID = ? AND vs.STATUS <> 'CANCELADA'
             ORDER BY vs.SEQ, vs.ID",
            (&visit_id,),
        )
        .map_err(AppError::from)?;
    if lines.is_empty() {
        return Err(AppError::Validation(
            "La visita no tiene servicios para facturar".into(),
        ));
    }

    let global_rate = default_tax_rate(conn)?;
    let subtotal: f64 = lines.iter().map(|(_, _, q, p, _)| *q as f64 * *p).sum();
    let tax_amount: f64 = lines
        .iter()
        .map(|(_, _, q, p, rate)| {
            let rate = rate.unwrap_or(global_rate);
            *q as f64 * *p * rate / 100.0
        })
        .sum();
    let total = subtotal + tax_amount;

    // Factura (el número la asigna el trigger BI_INVOICES).
    let invoice_id = next_id(conn, "GEN_INVOICES_ID")?;
    conn.execute(
        "INSERT INTO INVOICES
            (ID, PATIENT_ID, OWNER_ID, VISIT_ID, SUBTOTAL, TAX_RATE,
             TAX_AMOUNT, TOTAL, STATUS, PAYMENT_METHOD, NOTES)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'EMITIDA', ?, ?)",
        (
            &invoice_id,
            &patient_id,
            &owner_id,
            &visit_id,
            &subtotal,
            &global_rate,
            &tax_amount,
            &total,
            &payment_method,
            &notes,
        ),
    )
    .map_err(AppError::from)?;

    for (service_id, service_name, quantity, unit_price, _rate) in &lines {
        let item_id = next_id(conn, "GEN_INVOICE_ITEMS_ID")?;
        let line_total = *quantity as f64 * *unit_price;
        let description = format!("{service_name} x{quantity}");
        conn.execute(
            "INSERT INTO INVOICE_ITEMS
                (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE,
                 LINE_TOTAL, SERVICE_ID, VISIT_ID)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &item_id,
                &invoice_id,
                &description,
                quantity,
                unit_price,
                &line_total,
                service_id,
                &visit_id,
            ),
        )
        .map_err(AppError::from)?;
    }

    conn.execute(
        "UPDATE VISITS SET STATUS = 'FACTURADA', UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&visit_id,),
    )
    .map_err(AppError::from)?;

    invoices_repo::get(conn, invoice_id)?
        .ok_or_else(|| AppError::Internal("Factura creada pero no recuperada".into()))
}

// ============================== TESTS =======================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visit::CreateVisitInput;
    use crate::test_helpers;

    fn setup() -> (crate::test_helpers::TestDb, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        // El helper limpia el seed de demo: creamos pacientes de prueba.
        // Paciente 1 "Luna" (propietario 1); paciente 2 "Rocky" (propietario 2).
        test_helpers::insert_test_patient(&mut conn);
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (2, 'CC', '0987654321', 'María Gómez')",
            (),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, NAME, SEX, ACTIVE)
             VALUES (2, 2, 1, 'Rocky', 'M', TRUE)",
            (),
        )
        .unwrap();
        (conn, db_path)
    }

    fn svc(service_id: i32, quantity: i32) -> AddVisitServiceInput {
        AddVisitServiceInput {
            service_id,
            professional_id: None,
            quantity,
            notes: None,
        }
    }

    /// Visita de prueba: paciente 1 (propietario 1) con Consulta general (1)
    /// y Perfil básico de laboratorio (9, 60000).
    fn create_test_visit(conn: &mut SimpleConnection) -> Visit {
        let input = CreateVisitInput {
            patient_id: 1,
            owner_id: None,
            priority: Some("NORMAL".into()),
            notes: Some("Llega cojeando".into()),
            services: vec![svc(1, 1), svc(9, 1)],
        };
        create_visit(conn, &input, None, None).unwrap()
    }

    #[test]
    fn test_create_visit_snapshots_prices() {
        let (mut conn, db_path) = setup();
        let visit = create_test_visit(&mut conn);

        assert!(visit.code.starts_with("V-"));
        assert_eq!(visit.status, "EN_ESPERA");
        assert_eq!(visit.owner_id, 1);
        assert_eq!(visit.services.len(), 2);

        // Precio congelado del seed 0025: CONS-GEN = 60000, LAB-BAS = 60000.
        let cons = &visit.services[0];
        assert_eq!(cons.service_code, "CONS-GEN");
        assert!((cons.unit_price - 60000.0).abs() < 0.01);
        assert_eq!(cons.status, "PENDIENTE");
        assert_eq!(cons.seq, 1);

        // El total estimado del listado suma las líneas activas.
        let list = list_visits(&mut conn, None, None, None).unwrap();
        let item = list.iter().find(|v| v.id == visit.id).unwrap();
        assert_eq!(
            item.services_summary,
            "Consulta general + Perfil básico de laboratorio"
        );
        assert!((item.estimated_total - 120000.0).abs() < 0.01);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_visit_status_machine() {
        let (mut conn, db_path) = setup();
        let visit = create_test_visit(&mut conn);

        // Transiciones válidas.
        let v = set_visit_status(&mut conn, visit.id, "EN_ATENCION").unwrap();
        assert_eq!(v.status, "EN_ATENCION");
        let v = set_visit_status(&mut conn, visit.id, "FINALIZADA").unwrap();
        assert_eq!(v.status, "FINALIZADA");
        let v = set_visit_status(&mut conn, visit.id, "CANCELADA").unwrap();
        assert_eq!(v.status, "CANCELADA");

        // Transiciones inválidas.
        let v2 = create_test_visit(&mut conn);
        assert!(set_visit_status(&mut conn, v2.id, "FINALIZADA").is_err()); // salto EN_ESPERA→FINALIZADA
        assert!(set_visit_status(&mut conn, v2.id, "BOGUS").is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_service_lines_lifecycle() {
        let (mut conn, db_path) = setup();
        let visit = create_test_visit(&mut conn);
        let line = visit.services[0].id;

        // Avanza a EN_PROCESO (registra STARTED_AT).
        let l = update_visit_service_status(
            &mut conn,
            visit.id,
            line,
            &UpdateVisitServiceStatusInput {
                status: "EN_PROCESO".into(),
                professional_id: None,
            },
        )
        .unwrap();
        assert_eq!(l.status, "EN_PROCESO");
        assert!(l.started_at.is_some());
        assert!(l.finished_at.is_none());

        // EN_PROCESO → COMPLETADA (registra FINISHED_AT).
        assert!(update_visit_service_status(
            &mut conn,
            visit.id,
            line,
            &UpdateVisitServiceStatusInput {
                status: "COMPLETADA".into(),
                professional_id: None,
            },
        )
        .is_ok());
        // COMPLETADA → EN_PROCESO inválido.
        assert!(update_visit_service_status(
            &mut conn,
            visit.id,
            line,
            &UpdateVisitServiceStatusInput {
                status: "EN_PROCESO".into(),
                professional_id: None,
            },
        )
        .is_err());

        // Una línea COMPLETADA no se puede quitar.
        assert!(remove_visit_service(&mut conn, visit.id, line).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_add_service_snapshot_and_duplicate_guard() {
        let (mut conn, db_path) = setup();
        let visit = create_test_visit(&mut conn);

        // Duplicado del mismo servicio rechazado (guard anti-líneas gemelas).
        let err = add_visit_service(&mut conn, visit.id, &svc(1, 1)).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Servicio inactivo rechazado.
        conn.execute("UPDATE SERVICES SET IS_ACTIVE = FALSE WHERE ID = 5", ())
            .unwrap();
        let err = add_visit_service(&mut conn, visit.id, &svc(5, 1)).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        conn.execute("UPDATE SERVICES SET IS_ACTIVE = TRUE WHERE ID = 5", ())
            .unwrap();

        // Un servicio nuevo toma el precio vigente del catálogo.
        conn.execute("UPDATE SERVICES SET PRICE = 99000 WHERE ID = 5", ())
            .unwrap();
        let l = add_visit_service(&mut conn, visit.id, &svc(5, 1)).unwrap();
        assert!((l.unit_price - 99000.0).abs() < 0.01);
        assert_eq!(l.seq, 3);

        // Quitar una línea PENDIENTE sí funciona.
        remove_visit_service(&mut conn, visit.id, l.id).unwrap();
        let v = get_visit(&mut conn, visit.id).unwrap().unwrap();
        assert_eq!(v.services.len(), 2);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_checkout_creates_invoice_once() {
        let (mut conn, db_path) = setup();
        let visit = create_test_visit(&mut conn);

        // No se puede facturar sin FINALIZADA.
        let err = checkout_visit(&mut conn, visit.id, Some("EFECTIVO"), None).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        set_visit_status(&mut conn, visit.id, "EN_ATENCION").unwrap();
        set_visit_status(&mut conn, visit.id, "FINALIZADA").unwrap();

        let invoice = checkout_visit(&mut conn, visit.id, Some("EFECTIVO"), None).unwrap();
        assert!(invoice.invoice_number.starts_with("FAC-"));
        assert_eq!(invoice.items.len(), 2);
        assert!((invoice.subtotal - 120000.0).abs() < 0.01);
        // IVA global por defecto del seed (settings): 19%.
        assert!((invoice.tax_amount - 120000.0 * 0.19).abs() < 0.01);
        assert!((invoice.total - 120000.0 * 1.19).abs() < 0.01);
        assert_eq!(invoice.patient_id, Some(1));
        assert_eq!(invoice.owner_id, 1);

        // La visita quedó FACTURADA y ya no admite segundo cobro.
        let v = get_visit(&mut conn, visit.id).unwrap().unwrap();
        assert_eq!(v.status, "FACTURADA");
        let err = checkout_visit(&mut conn, visit.id, None, None).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Ni cambios de estado ni líneas nuevas.
        assert!(set_visit_status(&mut conn, visit.id, "CANCELADA").is_err());
        assert!(add_visit_service(&mut conn, visit.id, &svc(3, 1)).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_checkout_per_service_tax_rate() {
        let (mut conn, db_path) = setup();
        // Servicio con IVA propio (19%) y otro sin IVA (0%).
        conn.execute("UPDATE SERVICES SET TAX_RATE = 19 WHERE ID = 1", ())
            .unwrap();
        conn.execute("UPDATE SERVICES SET TAX_RATE = 0 WHERE ID = 9", ())
            .unwrap();

        let input = CreateVisitInput {
            patient_id: 1,
            owner_id: None,
            priority: None,
            notes: None,
            services: vec![svc(1, 1), svc(9, 1)],
        };
        let visit = create_visit(&mut conn, &input, None, None).unwrap();
        set_visit_status(&mut conn, visit.id, "EN_ATENCION").unwrap();
        set_visit_status(&mut conn, visit.id, "FINALIZADA").unwrap();

        let invoice = checkout_visit(&mut conn, visit.id, None, None).unwrap();
        // Solo la línea de consulta paga IVA: 60000 * 0.19.
        assert!((invoice.tax_amount - 60000.0 * 0.19).abs() < 0.01);
        assert!((invoice.total - (60000.0 * 1.19 + 60000.0)).abs() < 0.01);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_visits_filters() {
        let (mut conn, db_path) = setup();
        let v1 = create_test_visit(&mut conn);
        set_visit_status(&mut conn, v1.id, "EN_ATENCION").unwrap();

        let input = CreateVisitInput {
            patient_id: 2,
            owner_id: None,
            priority: Some("URGENTE".into()),
            notes: None,
            services: vec![svc(3, 1)],
        };
        let v2 = create_visit(&mut conn, &input, None, None).unwrap();

        // Por estado.
        let waiting = list_visits(&mut conn, None, Some("EN_ESPERA"), None).unwrap();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].id, v2.id);
        assert_eq!(waiting[0].priority, "URGENTE");

        // Por texto (paciente).
        let by_name = list_visits(&mut conn, None, None, Some("luna")).unwrap();
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].id, v1.id);

        // Urgentes primero en el orden.
        let all = list_visits(&mut conn, None, None, None).unwrap();
        assert_eq!(all[0].id, v2.id);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_visit_validations() {
        let (mut conn, db_path) = setup();

        // Sin servicios.
        let input = CreateVisitInput {
            patient_id: 1,
            owner_id: None,
            priority: None,
            notes: None,
            services: vec![],
        };
        assert!(create_visit(&mut conn, &input, None, None).is_err());

        // Paciente inexistente.
        let input = CreateVisitInput {
            patient_id: 999,
            owner_id: None,
            priority: None,
            notes: None,
            services: vec![svc(1, 1)],
        };
        assert!(create_visit(&mut conn, &input, None, None).is_err());

        // Prioridad inválida.
        let input = CreateVisitInput {
            patient_id: 1,
            owner_id: None,
            priority: Some("ALTA".into()),
            notes: None,
            services: vec![svc(1, 1)],
        };
        assert!(create_visit(&mut conn, &input, None, None).is_err());

        // Cantidad inválida en una línea.
        let input = CreateVisitInput {
            patient_id: 1,
            owner_id: None,
            priority: None,
            notes: None,
            services: vec![svc(1, 0)],
        };
        assert!(create_visit(&mut conn, &input, None, None).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }
}
