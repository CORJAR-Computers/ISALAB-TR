use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::appointment::{Appointment, AppointmentListItem, CreateAppointmentInput};
use crate::repositories::next_id;

/// Estados válidos de una cita (migración 0028).
pub const APPOINTMENT_STATUSES: [&str; 6] = [
    "PROGRAMADA",
    "CONFIRMADA",
    "EN_ATENCION",
    "ATENDIDA",
    "NO_ASISTIO",
    "CANCELADA",
];

// ============================== HELPERS =====================================

type AppointmentRow = (
    i32,            // id
    String,         // code
    i32,            // patient_id
    String,         // patient_name
    i32,            // owner_id
    String,         // owner_name
    i32,            // service_id
    String,         // service_name
    String,         // service_code
    i32,            // category_id
    String,         // category_name
    Option<i32>,    // professional_id
    Option<String>, // professional_name
    String,         // scheduled_at
    i32,            // duration_min
    String,         // status
    Option<String>, // notes
    String,         // created_at
    String,         // updated_at
);

fn map_appointment(r: AppointmentRow) -> Appointment {
    Appointment {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        service_id: r.6,
        service_name: r.7,
        service_code: r.8,
        category_id: r.9,
        category_name: r.10,
        professional_id: r.11,
        professional_name: r.12,
        scheduled_at: r.13,
        duration_min: r.14,
        status: r.15,
        notes: r.16,
        created_at: r.17,
        updated_at: r.18,
    }
}

type AppointmentListRow = (
    i32,            // id
    String,         // code
    i32,            // patient_id
    String,         // patient_name
    i32,            // owner_id
    String,         // owner_name
    i32,            // service_id
    String,         // service_name
    i32,            // category_id
    String,         // category_name
    String,         // scheduled_at
    i32,            // duration_min
    String,         // status
    Option<String>, // professional_name
);

fn map_appointment_list(r: AppointmentListRow) -> AppointmentListItem {
    AppointmentListItem {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        service_id: r.6,
        service_name: r.7,
        category_id: r.8,
        category_name: r.9,
        scheduled_at: r.10,
        duration_min: r.11,
        status: r.12,
        professional_name: r.13,
    }
}

const APPOINTMENT_SELECT: &str = "
    SELECT a.ID, a.CODE,
           a.PATIENT_ID, p.NAME,
           o.ID, o.FULL_NAME,
           a.SERVICE_ID, s.NAME, s.CODE,
           s.CATEGORY_ID, c.NAME,
           a.PROFESSIONAL_ID, u.FULL_NAME,
           LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 19),
           a.DURATION_MIN, a.STATUS, a.NOTES,
           LEFT(CAST(a.CREATED_AT AS VARCHAR(60)), 19),
           LEFT(CAST(a.UPDATED_AT AS VARCHAR(60)), 19)
    FROM APPOINTMENTS a
    JOIN PATIENTS p ON p.ID = a.PATIENT_ID
    JOIN OWNERS o ON o.ID = p.OWNER_ID
    JOIN SERVICES s ON s.ID = a.SERVICE_ID
    JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
    LEFT JOIN USERS u ON u.ID = a.PROFESSIONAL_ID";

const APPOINTMENT_LIST_SELECT: &str = "
    SELECT a.ID, a.CODE,
           a.PATIENT_ID, p.NAME,
           o.ID, o.FULL_NAME,
           a.SERVICE_ID, s.NAME,
           s.CATEGORY_ID, c.NAME,
           LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 19),
           a.DURATION_MIN, a.STATUS,
           u.FULL_NAME
    FROM APPOINTMENTS a
    JOIN PATIENTS p ON p.ID = a.PATIENT_ID
    JOIN OWNERS o ON o.ID = p.OWNER_ID
    JOIN SERVICES s ON s.ID = a.SERVICE_ID
    JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
    LEFT JOIN USERS u ON u.ID = a.PROFESSIONAL_ID";

// Duración del servicio desde el catálogo (fallback 30 min).
fn service_duration(conn: &mut SimpleConnection, service_id: i32) -> Result<i32, AppError> {
    let row: Option<(i32,)> = conn
        .query_first(
            "SELECT DURATION_MIN FROM SERVICES WHERE ID = ?",
            (&service_id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(|(d,)| d).unwrap_or(30))
}

/// Calcula el instante de fin: SCHEDULED_AT + DURATION_MIN minutos.
/// Se hace en Rust porque Firebird no acepta `INTERVAL` con parámetros
/// dinámicos en esta versión del driver.
fn ends_at(scheduled_at: &str, duration_min: i32) -> String {
    // Formato esperado: "YYYY-MM-DD HH:MM:SS"
    let year: i32 = scheduled_at[0..4].parse().unwrap_or(2000);
    let month: i32 = scheduled_at[5..7].parse().unwrap_or(1);
    let day: i32 = scheduled_at[8..10].parse().unwrap_or(1);
    let hour: i32 = scheduled_at[11..13].parse().unwrap_or(0);
    let min: i32 = scheduled_at[14..16].parse().unwrap_or(0);
    let sec: i32 = scheduled_at[17..19].parse().unwrap_or(0);

    let total_min = hour * 60 + min + duration_min;
    let new_hour = (total_min / 60) % 24;
    let new_min = total_min % 60;
    let days_add = total_min / (24 * 60);
    let new_day = day + days_add;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, new_day, new_hour, new_min, sec
    )
}

/// Comprueba solapamiento para el mismo profesional y/o paciente en el
/// intervalo [start, end). Excluye la cita `exclude_id` (para actualizaciones).
/// Solo considera estados no cancelados.
fn has_conflict(
    conn: &mut SimpleConnection,
    professional_id: Option<i32>,
    patient_id: i32,
    start: &str,
    end: &str,
    exclude_id: Option<i32>,
) -> Result<bool, AppError> {
    // Recupera candidatos del mismo paciente o profesional en el rango y
    // valida solapamiento en Rust.
    let sql = format!(
        "SELECT a.ID, a.PROFESSIONAL_ID, LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 19), a.DURATION_MIN, a.STATUS
         FROM APPOINTMENTS a
         WHERE (a.PATIENT_ID = ? OR a.PROFESSIONAL_ID = ?)
           AND a.STATUS <> 'CANCELADA'
           AND a.ID <> COALESCE(?, -1)
           AND a.SCHEDULED_AT < ?
           AND DATEADD(MINUTE, a.DURATION_MIN, a.SCHEDULED_AT) > ?
         ROWS 1",
    );

    let prof_param = professional_id.unwrap_or(0);
    let rows: Vec<(i32, Option<i32>, String, i32, String)> = conn
        .query(
            &sql,
            (
                &patient_id,
                &prof_param,
                &exclude_id.unwrap_or(0),
                &end,
                &start,
            ),
        )
        .map_err(AppError::from)?;

    Ok(!rows.is_empty())
}

// ============================== CREAR =======================================

pub fn create_appointment(
    conn: &mut SimpleConnection,
    input: &CreateAppointmentInput,
) -> Result<Appointment, AppError> {
    // Duración: la del input o la del catálogo.
    let duration_min = match input.duration_min {
        Some(d) if d > 0 => d,
        _ => service_duration(conn, input.service_id)?,
    };
    if duration_min <= 0 {
        return Err(AppError::Validation(
            "La duración debe ser mayor que cero".into(),
        ));
    }

    // El servicio debe existir y estar activo.
    let _snap: Option<(String, bool)> = conn
        .query_first(
            "SELECT NAME, IS_ACTIVE FROM SERVICES WHERE ID = ?",
            (&input.service_id,),
        )
        .map_err(AppError::from)?;
    let Some((_, is_active)) = _snap else {
        return Err(AppError::NotFound(format!(
            "Servicio {} no encontrado en el catálogo",
            input.service_id
        )));
    };
    if !is_active {
        return Err(AppError::Validation(format!(
            "El servicio está inactivo en el catálogo"
        )));
    }

    // Paciente debe existir.
    let patient: Option<(i32,)> = conn
        .query_first(
            "SELECT OWNER_ID FROM PATIENTS WHERE ID = ?",
            (&input.patient_id,),
        )
        .map_err(AppError::from)?;
    let Some((_owner_id,)) = patient else {
        return Err(AppError::NotFound(format!(
            "Paciente {} no encontrado",
            input.patient_id
        )));
    };

    // Profesional (si llega) debe existir.
    if let Some(pid) = input.professional_id {
        let exists: Option<(i32,)> = conn
            .query_first("SELECT ID FROM USERS WHERE ID = ?", (&pid,))
            .map_err(AppError::from)?;
        if exists.is_none() {
            return Err(AppError::Validation(format!(
                "El profesional {pid} no existe"
            )));
        }
    }

    // Validación de solapamiento.
    let end = ends_at(&input.scheduled_at, duration_min);

    // Validación + inserción en una transacción. Sin ella, dos PCs pueden
    // validar el mismo hueco a la vez y crear las dos citas: el SELECT no ve
    // el INSERT de la otra PC hasta que confirma.
    db::tx::in_transaction(conn, |conn| {
        if has_conflict(
            conn,
            input.professional_id,
            input.patient_id,
            &input.scheduled_at,
            &end,
            None,
        )? {
            return Err(AppError::Validation(
                "Ya existe una cita solapada para este paciente o profesional en ese horario"
                    .into(),
            ));
        }

        let id = next_id(conn, "GEN_APPOINTMENTS_ID")?;
        conn.execute(
            "INSERT INTO APPOINTMENTS
                (ID, PATIENT_ID, SERVICE_ID, PROFESSIONAL_ID, SCHEDULED_AT,
                 DURATION_MIN, STATUS, NOTES)
             VALUES (?, ?, ?, ?, ?, ?, 'PROGRAMADA', ?)",
            (
                &id,
                &input.patient_id,
                &input.service_id,
                &input.professional_id,
                &input.scheduled_at,
                &duration_min,
                &input.notes,
            ),
        )
        .map_err(AppError::from)?;

        get_appointment(conn, id)?
            .ok_or_else(|| AppError::Internal("Cita creada pero no recuperada".into()))
    })
}

// ============================== LISTADO =====================================

/// Lista citas con filtros opcionales: fecha (YYYY-MM-DD), estado y
/// profesional. Ordenadas por fecha/hora.
pub fn list_appointments(
    conn: &mut SimpleConnection,
    date: Option<&str>,
    status: Option<&str>,
    professional_id: Option<i32>,
) -> Result<Vec<AppointmentListItem>, AppError> {
    let sql = format!(
        "{APPOINTMENT_LIST_SELECT}
         WHERE (? IS NULL OR LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 10) = ?)
           AND (? IS NULL OR a.STATUS = ?)
           AND (? IS NULL OR a.PROFESSIONAL_ID = ?)
         ORDER BY a.SCHEDULED_AT, a.ID"
    );

    let date_opt = date.map(|s| s.to_string());
    let prof_opt = professional_id.map(|p| p as i32);

    let rows: Vec<AppointmentListRow> = conn
        .query(
            &sql,
            (&date_opt, &date_opt, &status, &status, &prof_opt, &prof_opt),
        )
        .map_err(AppError::from)?;

    Ok(rows.into_iter().map(map_appointment_list).collect())
}

/// Próximas citas de la agenda (PROGRAMADA/CONFIRMADA desde ahora, máx.
/// `limit`, ascendente por fecha/hora) para el panel de control.
pub fn list_upcoming_appointments(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<AppointmentListItem>, AppError> {
    let sql = format!(
        "SELECT FIRST {limit} a.ID, a.CODE,
            a.PATIENT_ID, p.NAME,
            o.ID, o.FULL_NAME,
            a.SERVICE_ID, s.NAME,
            s.CATEGORY_ID, c.NAME,
            LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 19),
            a.DURATION_MIN, a.STATUS,
            u.FULL_NAME
     FROM APPOINTMENTS a
     JOIN PATIENTS p ON p.ID = a.PATIENT_ID
     JOIN OWNERS o ON o.ID = p.OWNER_ID
     JOIN SERVICES s ON s.ID = a.SERVICE_ID
     JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
     LEFT JOIN USERS u ON u.ID = a.PROFESSIONAL_ID
     WHERE a.STATUS IN ('PROGRAMADA', 'CONFIRMADA')
       AND a.SCHEDULED_AT >= CURRENT_TIMESTAMP
     ORDER BY a.SCHEDULED_AT, a.ID"
    );

    let rows: Vec<AppointmentListRow> = conn.query(&sql, ()).map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_appointment_list).collect())
}

/// Cita completa o `None`.
pub fn get_appointment(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<Appointment>, AppError> {
    let row: Option<AppointmentRow> = conn
        .query_first(&format!("{APPOINTMENT_SELECT} WHERE a.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_appointment))
}

// ============================== ESTADOS =====================================

/// Cambia el estado de una cita validando la transición.
pub fn set_appointment_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
    notes: Option<&str>,
) -> Result<Appointment, AppError> {
    // Estados de origen admitidos por cada transición.
    let from: &[&str] = match status {
        "CONFIRMADA" => &["PROGRAMADA"],
        "EN_ATENCION" => &["PROGRAMADA", "CONFIRMADA"],
        "ATENDIDA" => &["EN_ATENCION"],
        "NO_ASISTIO" => &["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
        "CANCELADA" => &["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
        _ => &[],
    };
    if from.is_empty() {
        return Err(AppError::Validation(format!(
            "Estado de cita inválido: {status}"
        )));
    }

    // Compare-and-set: la transición válida va en el WHERE, así que si otra PC
    // ya movió la cita, este UPDATE afecta 0 filas en vez de sobrescribirla.
    // Los estados son un conjunto fijo interno: se interpolan sin riesgo.
    let allowed = from
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE APPOINTMENTS SET STATUS = ?, NOTES = COALESCE(?, NOTES),
         UPDATED_AT = CURRENT_TIMESTAMP
         WHERE ID = ? AND STATUS IN ({allowed})"
    );
    let updated = conn
        .execute(&sql, (&status, &notes, &id))
        .map_err(AppError::from)?;

    if updated == 0 {
        // Para el mensaje, distinguir "no existe" de "cambió en otra PC".
        let current: Option<(String,)> = conn
            .query_first("SELECT STATUS FROM APPOINTMENTS WHERE ID = ?", (&id,))
            .map_err(AppError::from)?;
        let Some((current,)) = current else {
            return Err(AppError::NotFound(format!("Cita {id} no encontrada")));
        };
        return Err(AppError::Validation(format!(
            "Transición de estado inválida: {current} → {status}"
        )));
    }

    get_appointment(conn, id)?
        .ok_or_else(|| AppError::Internal("Cita actualizada pero no recuperada".into()))
}

/// Anula una cita (CANCELADA) si aún no fue atendida.
pub fn cancel_appointment(conn: &mut SimpleConnection, id: i32) -> Result<Appointment, AppError> {
    set_appointment_status(conn, id, "CANCELADA", None)
}

/// Check-in: convierte la cita en una visita. Crea la VISITA con el servicio
/// de la cita como línea (precio congelado) y marca la cita como ATENDIDA.
pub fn check_in_appointment(
    conn: &mut SimpleConnection,
    id: i32,
    created_by: Option<i32>,
) -> Result<crate::models::visit::Visit, AppError> {
    // Visita + línea de servicio + claim de la cita en una sola transacción.
    db::tx::in_transaction(conn, |conn| {
        let appt: Option<(i32, i32, i32, i32, String, Option<i32>)> = conn
            .query_first(
                "SELECT a.ID, a.PATIENT_ID, o.ID, a.SERVICE_ID,
                        a.STATUS, a.PROFESSIONAL_ID
                 FROM APPOINTMENTS a
                 JOIN PATIENTS p ON p.ID = a.PATIENT_ID
                 JOIN OWNERS o ON o.ID = p.OWNER_ID
                 WHERE a.ID = ?",
                (&id,),
            )
            .map_err(AppError::from)?;
        let Some((_appt_id, patient_id, owner_id, service_id, status, professional_id)) = appt
        else {
            return Err(AppError::NotFound(format!("Cita {id} no encontrada")));
        };
        if status != "PROGRAMADA" && status != "CONFIRMADA" {
            return Err(AppError::Validation(
                "Solo se puede hacer check-in de citas PROGRAMADA o CONFIRMADA".into(),
            ));
        }

        // Claim de la cita (compare-and-set): solo aplica si sigue PROGRAMADA o
        // CONFIRMADA. Con dos PCs haciendo check-in a la vez, el segundo espera
        // (transacción en modo WAIT) y al reevaluar ve 0 filas: no puede crear
        // dos visitas para la misma cita.
        let claimed = conn
            .execute(
                "UPDATE APPOINTMENTS SET STATUS = 'ATENDIDA', UPDATED_AT = CURRENT_TIMESTAMP
                 WHERE ID = ? AND STATUS IN ('PROGRAMADA', 'CONFIRMADA')",
                (&id,),
            )
            .map_err(AppError::from)?;
        if claimed == 0 {
            return Err(AppError::Validation(
                "La cita ya fue atendida desde otro puesto de trabajo".into(),
            ));
        }

        // Crear la visita (primero sin appointment_id, se vincula tras el INSERT
        // para evitar problemas de mapeo de tipos en rsfbclient).
        let priority = "NORMAL";
        let visit_id = next_id(conn, "GEN_VISITS_ID")?;
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, PRIORITY, STATUS, CREATED_BY)
             VALUES (?, ?, ?, ?, 'EN_ESPERA', ?)",
            (&visit_id, &patient_id, &owner_id, &priority, &created_by),
        )
        .map_err(AppError::from)?;

        // Snapshot del servicio: precio e IVA del catálogo.
        let snap: Option<(f64, Option<f64>)> = conn
            .query_first(
                "SELECT CAST(s.PRICE AS DOUBLE PRECISION), CAST(s.TAX_RATE AS DOUBLE PRECISION)
                 FROM SERVICES s WHERE s.ID = ?",
                (&service_id,),
            )
            .map_err(AppError::from)?;
        let Some((price, tax_rate)) = snap else {
            return Err(AppError::NotFound(format!(
                "Servicio {service_id} no encontrado en el catálogo"
            )));
        };

        let line_id = next_id(conn, "GEN_VISIT_SERVICES_ID")?;
        conn.execute(
            "INSERT INTO VISIT_SERVICES
                (ID, VISIT_ID, SERVICE_ID, PROFESSIONAL_ID, QUANTITY,
                 UNIT_PRICE, TAX_RATE, STATUS, NOTES, SEQ)
             VALUES (?, ?, ?, ?, 1, ?, ?, 'PENDIENTE', ?, 1)",
            (
                &line_id,
                &visit_id,
                &service_id,
                &professional_id,
                &price,
                &tax_rate,
                &format!("Desde cita {_appt_id}"),
            ),
        )
        .map_err(AppError::from)?;

        // Vincular la visita con la cita (el estado ya se fijó en el claim).
        conn.execute(
            "UPDATE VISITS SET APPOINTMENT_ID = ? WHERE ID = ?",
            (&id, &visit_id),
        )
        .map_err(AppError::from)?;

        // Leer code y arrived_at de la visita recién creada.
        let visit_header: Option<(String, String)> = conn
            .query_first(
                "SELECT CODE, LEFT(CAST(ARRIVED_AT AS VARCHAR(60)), 19)
                 FROM VISITS WHERE ID = ?",
                (&visit_id,),
            )
            .map_err(AppError::from)?;
        let Some((code, arrived_at)) = visit_header else {
            return Err(AppError::Internal(
                "Visita creada pero no recuperada".into(),
            ));
        };

        let patient_name_row: Option<(String,)> = conn
            .query_first("SELECT NAME FROM PATIENTS WHERE ID = ?", (&patient_id,))
            .map_err(AppError::from)?;
        let patient_name = patient_name_row.map(|(n,)| n).unwrap_or_default();

        let owner_name_row: Option<(String,)> = conn
            .query_first("SELECT FULL_NAME FROM OWNERS WHERE ID = ?", (&owner_id,))
            .map_err(AppError::from)?;
        let owner_name = owner_name_row.map(|(n,)| n).unwrap_or_default();

        let service_name_row: Option<(String,)> = conn
            .query_first("SELECT NAME FROM SERVICES WHERE ID = ?", (&service_id,))
            .map_err(AppError::from)?;
        let service_name = service_name_row.map(|(n,)| n).unwrap_or_default();

        let service_code_row: Option<(String,)> = conn
            .query_first("SELECT CODE FROM SERVICES WHERE ID = ?", (&service_id,))
            .map_err(AppError::from)?;
        let service_code = service_code_row.map(|(c,)| c).unwrap_or_default();

        let service = crate::models::visit::VisitService {
            id: line_id,
            visit_id,
            service_id,
            service_name,
            service_code,
            category_id: 0,
            category_name: String::new(),
            professional_id,
            professional_name: None,
            quantity: 1,
            unit_price: price,
            tax_rate,
            status: "PENDIENTE".to_string(),
            started_at: None,
            finished_at: None,
            notes: Some(format!("Desde cita {_appt_id}")),
            seq: 1,
        };

        Ok(crate::models::visit::Visit {
            id: visit_id,
            code,
            patient_id,
            patient_name,
            owner_id,
            owner_name,
            appointment_id: Some(id),
            arrived_at,
            priority: priority.to_string(),
            status: "EN_ESPERA".to_string(),
            notes: Some(format!("Desde cita {_appt_id}")),
            created_by_name: None,
            services: vec![service],
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
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

    fn appt_input(patient_id: i32, service_id: i32, scheduled_at: &str) -> CreateAppointmentInput {
        CreateAppointmentInput {
            patient_id,
            service_id,
            professional_id: None,
            scheduled_at: scheduled_at.to_string(),
            duration_min: None,
            notes: None,
        }
    }

    #[test]
    fn test_create_appointment_code_and_status() {
        let (mut conn, db_path) = setup();
        let appt = create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();

        assert!(appt.code.starts_with("C-"));
        assert_eq!(appt.status, "PROGRAMADA");
        assert_eq!(appt.patient_id, 1);
        assert_eq!(appt.service_id, 1);
        assert_eq!(appt.duration_min, 30); // catálogo: Consulta general = 30 min
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_appointment_status_machine() {
        let (mut conn, db_path) = setup();
        let appt = create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();

        // PROGRAMADA → CONFIRMADA → EN_ATENCION → ATENDIDA
        let a = set_appointment_status(&mut conn, appt.id, "CONFIRMADA", None).unwrap();
        assert_eq!(a.status, "CONFIRMADA");
        let a = set_appointment_status(&mut conn, appt.id, "EN_ATENCION", None).unwrap();
        assert_eq!(a.status, "EN_ATENCION");
        let a = set_appointment_status(&mut conn, appt.id, "ATENDIDA", None).unwrap();
        assert_eq!(a.status, "ATENDIDA");

        // Transición inválida: ATENDIDA → CONFIRMADA
        assert!(set_appointment_status(&mut conn, appt.id, "CONFIRMADA", None).is_err());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_appointment_cancel() {
        let (mut conn, db_path) = setup();
        let appt = create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();

        let a = cancel_appointment(&mut conn, appt.id).unwrap();
        assert_eq!(a.status, "CANCELADA");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_appointment_conflict_same_patient() {
        let (mut conn, db_path) = setup();
        // Cita a las 10:00, duración 30 min (10:00-10:30).
        create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();

        // Misma paciente, intervalo que se cruza (10:15-10:45): conflicto.
        let err =
            create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:15:00")).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Paciente diferente, mismo horario: sin conflicto.
        let appt2 =
            create_appointment(&mut conn, &appt_input(2, 1, "2026-12-01 10:00:00")).unwrap();
        assert_eq!(appt2.patient_id, 2);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_appointment_conflict_same_professional() {
        let (mut conn, db_path) = setup();
        conn.execute(
            "INSERT INTO USERS (ID, USERNAME, FULL_NAME, ROLE, PASSWORD_HASH, MUST_CHANGE_PASSWORD, ACTIVE)
             VALUES (10, 'vet_test', 'Vet Test', 'VETERINARIO', 'hash', FALSE, TRUE)",
            (),
        )
        .unwrap();

        // Cita con profesional 10 a las 10:00.
        create_appointment(
            &mut conn,
            &CreateAppointmentInput {
                patient_id: 1,
                service_id: 1,
                professional_id: Some(10),
                scheduled_at: "2026-12-01 10:00:00".to_string(),
                duration_min: None,
                notes: None,
            },
        )
        .unwrap();

        // Otro paciente, mismo profesional, hora que se cruza: conflicto.
        let err = create_appointment(
            &mut conn,
            &CreateAppointmentInput {
                patient_id: 2,
                service_id: 1,
                professional_id: Some(10),
                scheduled_at: "2026-12-01 10:15:00".to_string(),
                duration_min: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_check_in_appointment_creates_visit() {
        let (mut conn, db_path) = setup();
        let appt = create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();

        let visit = check_in_appointment(&mut conn, appt.id, None).unwrap();

        // La visita quedó EN_ESPERA con APPOINTMENT_ID vinculado.
        assert_eq!(visit.status, "EN_ESPERA");
        assert_eq!(visit.appointment_id, Some(appt.id));
        assert_eq!(visit.patient_id, 1);
        assert_eq!(visit.services.len(), 1);
        assert_eq!(visit.services[0].service_id, 1);

        // La cita pasó a ATENDIDA.
        let a = get_appointment(&mut conn, appt.id).unwrap().unwrap();
        assert_eq!(a.status, "ATENDIDA");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_appointments_filters() {
        let (mut conn, db_path) = setup();
        create_appointment(&mut conn, &appt_input(1, 1, "2026-12-01 10:00:00")).unwrap();
        create_appointment(&mut conn, &appt_input(2, 1, "2026-12-01 11:00:00")).unwrap();

        let all = list_appointments(&mut conn, None, None, None).unwrap();
        assert_eq!(all.len(), 2);

        let by_date = list_appointments(&mut conn, Some("2026-12-01"), None, None).unwrap();
        assert_eq!(by_date.len(), 2);

        let by_status = list_appointments(&mut conn, None, Some("PROGRAMADA"), None).unwrap();
        assert_eq!(by_status.len(), 2);

        test_helpers::cleanup_test_db(&db_path);
    }
}
