use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::search::GlobalSearchResult;

const PATIENT_PRIORITY: i32 = 0;
const VISIT_PRIORITY: i32 = 1;
const APPOINTMENT_PRIORITY: i32 = 2;
const SAMPLE_PRIORITY: i32 = 3;
const INVOICE_PRIORITY: i32 = 4;
const SURGERY_PRIORITY: i32 = 5;
const SERVICE_PRIORITY: i32 = 6;

fn like(query: &str) -> String {
    format!("%{}%", query.trim())
}

/// Formatea un monto sin decimales con separador de miles (estilo es-CO).
fn format_amount(total: f64) -> String {
    let int = total as i64;
    let digits = int.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if int < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// Búsqueda global entre pacientes, visitas, citas, muestras, facturas,
/// cirugías y servicios, por código o nombre. Devuelve hasta 8 resultados por
/// entidad, ordenados con los coincidencias de prefijo primero y agrupados
/// por tipo.
pub fn global_search(
    conn: &mut SimpleConnection,
    query: &str,
) -> Result<Vec<GlobalSearchResult>, AppError> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let q: String = q.chars().take(100).collect();
    let like = like(&q);
    let q_lower = q.to_lowercase();

    let mut results = Vec::new();

    // ---- Pacientes (por código PAC-, nombre o propietario) ----
    let rows: Vec<(i32, String, String, String, String)> = conn
        .query(
            "SELECT FIRST 8 p.ID, p.CODE, p.NAME, sp.NAME, o.FULL_NAME
             FROM PATIENTS p
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             WHERE UPPER(p.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
             ORDER BY p.NAME",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, code, name, species, owner) in rows {
        results.push(GlobalSearchResult {
            kind: "patient".into(),
            id,
            title: name,
            subtitle: format!("{species} · {owner}"),
            code: Some(code),
        });
    }

    // ---- Visitas de recepción (por código V-, paciente o propietario) ----
    let rows: Vec<(i32, String, String, String, String, String)> = conn
        .query(
            "SELECT FIRST 8 v.ID, v.CODE, p.NAME, v.STATUS, o.FULL_NAME,
                    COALESCE((SELECT LIST(s2.NAME, ' + ')
                                FROM VISIT_SERVICES vs2
                                JOIN SERVICES s2 ON s2.ID = vs2.SERVICE_ID
                               WHERE vs2.VISIT_ID = v.ID
                                 AND vs2.STATUS <> 'CANCELADA'), '')
             FROM VISITS v
             JOIN PATIENTS p ON p.ID = v.PATIENT_ID
             JOIN OWNERS o ON o.ID = v.OWNER_ID
             WHERE UPPER(v.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
             ORDER BY v.ARRIVED_AT DESC",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, code, patient, status, owner, services) in rows {
        let summary = if services.is_empty() {
            owner
        } else {
            format!("{owner} · {services}")
        };
        results.push(GlobalSearchResult {
            kind: "visit".into(),
            id,
            title: patient,
            subtitle: format!("{status} · {summary}"),
            code: Some(code),
        });
    }

    // ---- Citas de la agenda (por código C-, paciente o propietario) ----
    let rows: Vec<(i32, String, String, String, String, String)> = conn
        .query(
            "SELECT FIRST 8 a.ID, a.CODE, p.NAME, s.NAME, a.STATUS,
                    LEFT(CAST(a.SCHEDULED_AT AS VARCHAR(60)), 16)
             FROM APPOINTMENTS a
             JOIN PATIENTS p ON p.ID = a.PATIENT_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             JOIN SERVICES s ON s.ID = a.SERVICE_ID
             WHERE UPPER(a.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
             ORDER BY a.SCHEDULED_AT DESC",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, code, patient, service, status, scheduled_at) in rows {
        results.push(GlobalSearchResult {
            kind: "appointment".into(),
            id,
            title: patient,
            subtitle: format!("{service} · {status} · {scheduled_at}"),
            code: Some(code),
        });
    }

    // ---- Muestras (por código M-, paciente o propietario) ----
    let rows: Vec<(i32, String, String, String, String, String)> = conn
        .query(
            "SELECT FIRST 8 s.ID, s.CODE, p.NAME, st.NAME, s.STATUS, o.FULL_NAME
             FROM SAMPLES s
             JOIN PATIENTS p ON p.ID = s.PATIENT_ID
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             WHERE UPPER(s.CODE) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
             ORDER BY s.RECEIVED_AT DESC",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, code, patient, sample_type, status, owner) in rows {
        results.push(GlobalSearchResult {
            kind: "sample".into(),
            id,
            title: patient,
            subtitle: format!("{sample_type} · {status} · {owner}"),
            code: Some(code),
        });
    }

    // ---- Facturas (por número FAC-, cliente o paciente) ----
    let rows: Vec<(i32, String, String, Option<String>, String, f64)> = conn
        .query(
            "SELECT FIRST 8 i.ID, i.INVOICE_NUMBER, o.FULL_NAME, p.NAME, i.STATUS,
                    CAST(i.TOTAL AS DOUBLE PRECISION)
             FROM INVOICES i
             JOIN OWNERS o ON o.ID = i.OWNER_ID
             LEFT JOIN PATIENTS p ON p.ID = i.PATIENT_ID
             WHERE UPPER(i.INVOICE_NUMBER) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
                OR UPPER(p.NAME) LIKE UPPER(?)
             ORDER BY i.ISSUE_DATE DESC",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, number, owner, patient, status, total) in rows {
        let patient = patient.unwrap_or_else(|| "—".to_string());
        results.push(GlobalSearchResult {
            kind: "invoice".into(),
            id,
            title: owner,
            subtitle: format!("{patient} · {status} · ${}", format_amount(total)),
            code: Some(number),
        });
    }

    // ---- Cirugías (por paciente, propietario o tipo) ----
    let rows: Vec<(i32, String, String, String, String, String)> = conn
        .query(
            "SELECT FIRST 8 s.ID, p.NAME, sp.NAME, o.FULL_NAME, s.SURGERY_TYPE, s.STATUS
             FROM SURGERIES s
             JOIN PATIENTS p ON p.ID = s.PATIENT_ID
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             WHERE UPPER(p.NAME) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
                OR UPPER(s.SURGERY_TYPE) LIKE UPPER(?)
             ORDER BY s.SCHEDULED_AT DESC",
            (&like, &like, &like),
        )
        .map_err(AppError::from)?;
    for (id, patient, species, owner, surgery_type, _status) in rows {
        results.push(GlobalSearchResult {
            kind: "surgery".into(),
            id,
            title: patient,
            subtitle: format!("{species} · {owner} · {surgery_type}"),
            code: None,
        });
    }

    // ---- Servicios del catálogo (por nombre o código) ----
    let rows: Vec<(i32, String, String, String, f64)> = conn
        .query(
            "SELECT FIRST 8 s.ID, s.CODE, s.NAME, c.NAME,
                    CAST(s.PRICE AS DOUBLE PRECISION)
             FROM SERVICES s
             JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
             WHERE UPPER(s.NAME) LIKE UPPER(?)
                OR UPPER(s.CODE) LIKE UPPER(?)
             ORDER BY s.NAME",
            (&like, &like),
        )
        .map_err(AppError::from)?;
    for (id, code, name, category, price) in rows {
        results.push(GlobalSearchResult {
            kind: "service".into(),
            id,
            title: name,
            subtitle: format!("{category} · ${}", format_amount(price)),
            code: Some(code),
        });
    }

    // Relevancia: coincidencias por prefijo primero, luego por tipo.
    results.sort_by_key(|r| {
        let is_prefix = r.title.to_lowercase().starts_with(&q_lower)
            || r.code
                .as_deref()
                .is_some_and(|c| c.to_lowercase().starts_with(&q_lower));
        let priority = match r.kind.as_str() {
            "patient" => PATIENT_PRIORITY,
            "visit" => VISIT_PRIORITY,
            "appointment" => APPOINTMENT_PRIORITY,
            "sample" => SAMPLE_PRIORITY,
            "invoice" => INVOICE_PRIORITY,
            "service" => SERVICE_PRIORITY,
            _ => SURGERY_PRIORITY,
        };
        (if is_prefix { 0 } else { 1 }, priority)
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        let patient_id = test_helpers::insert_test_patient(&mut conn);
        test_helpers::insert_test_sample_type(&mut conn);

        // Muestra de Luna
        conn.execute(
            "INSERT INTO SAMPLES (ID, CODE, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, 'M-2026-0001', ?, 1, '2026-08-01 10:00:00', 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        // Factura de Juan Pérez
        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, PATIENT_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (1, 1, ?, 'FAC-0001', CURRENT_TIMESTAMP, 150000, 'EMITIDA')",
            (&patient_id,),
        )
        .unwrap();
        // Cirugía de Luna
        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Castración', DATEADD(1 DAY TO CURRENT_TIMESTAMP), 'PROGRAMADA')",
            (&patient_id,),
        )
        .unwrap();
        // Visita de recepción de Luna con una línea de servicio
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, ARRIVED_AT, STATUS)
             VALUES (1, ?, 1, CURRENT_TIMESTAMP, 'EN_ESPERA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO VISIT_SERVICES (ID, VISIT_ID, SERVICE_ID, UNIT_PRICE, STATUS)
             VALUES (1, 1, 1, 60000, 'PENDIENTE')",
            (),
        )
        .unwrap();
        // Cita de agenda de Luna
        conn.execute(
            "INSERT INTO APPOINTMENTS (ID, CODE, PATIENT_ID, SERVICE_ID, SCHEDULED_AT,
                                        DURATION_MIN, STATUS)
             VALUES (1, 'C-2026-0001', ?, 1, DATEADD(1 DAY TO CURRENT_TIMESTAMP),
                     30, 'PROGRAMADA')",
            (&patient_id,),
        )
        .unwrap();

        (conn, db_path)
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(0.0), "0");
        assert_eq!(format_amount(150000.0), "150.000");
        assert_eq!(format_amount(999.0), "999");
        assert_eq!(format_amount(1234567.0), "1.234.567");
        assert_eq!(format_amount(-45000.0), "-45.000");
    }

    #[test]
    fn test_search_empty_query() {
        let (mut conn, db_path) = setup();
        assert!(global_search(&mut conn, "   ").unwrap().is_empty());
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_by_patient_name() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "Luna").unwrap();
        // Paciente, muestra y cirugía comparten el nombre "Luna".
        let kinds: Vec<&str> = results.iter().map(|r| r.kind.as_str()).collect();
        assert!(kinds.contains(&"patient"));
        assert!(kinds.contains(&"sample"));
        assert!(kinds.contains(&"surgery"));
        // El prefijo "Luna" ordena los pacientes primero.
        assert_eq!(results[0].kind, "patient");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_by_code() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "FAC-0001").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "invoice");
        assert_eq!(results[0].code.as_deref(), Some("FAC-0001"));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_by_sample_code() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "M-2026-0001").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "sample");
        assert_eq!(results[0].id, 1);
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_by_owner_name() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "Juan").unwrap();
        assert!(results.iter().any(|r| r.kind == "invoice"));
        assert!(results.iter().any(|r| r.kind == "patient"));
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_no_results() {
        let (mut conn, db_path) = setup();
        assert!(global_search(&mut conn, "zzz_inexistente")
            .unwrap()
            .is_empty());
        test_helpers::cleanup_test_db(&db_path);
    }

    /// Fase 5: la búsqueda también cubre visitas, citas y servicios.
    #[test]
    fn test_search_by_patient_name_includes_clinic_entities() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "Luna").unwrap();
        let kinds: Vec<&str> = results.iter().map(|r| r.kind.as_str()).collect();
        assert!(kinds.contains(&"patient"));
        assert!(kinds.contains(&"visit"));
        assert!(kinds.contains(&"appointment"));
        // El paciente sigue primero por prioridad.
        assert_eq!(results[0].kind, "patient");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_by_appointment_code() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "C-2026-0001").unwrap();
        // El código del paciente (PAC-2026-0001) contiene el mismo fragmento,
        // así que se busca la cita por tipo y no por posición.
        let appointment = results
            .iter()
            .find(|r| r.kind == "appointment")
            .expect("cita encontrada");
        assert_eq!(appointment.code.as_deref(), Some("C-2026-0001"));
        assert_eq!(appointment.title, "Luna");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_visits_by_code_prefix() {
        let (mut conn, db_path) = setup();
        // El código lo genera el trigger: V-<año>-0001.
        let results = global_search(&mut conn, "V-").unwrap();
        assert!(results.iter().any(|r| r.kind == "visit"));
        let visit = results.iter().find(|r| r.kind == "visit").unwrap();
        assert_eq!(visit.title, "Luna");
        assert_eq!(visit.subtitle, "EN_ESPERA · Juan Pérez · Consulta general");
        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_search_service_by_name() {
        let (mut conn, db_path) = setup();
        let results = global_search(&mut conn, "Consulta general").unwrap();
        let service = results
            .iter()
            .find(|r| r.kind == "service")
            .expect("servicio encontrado");
        assert_eq!(service.title, "Consulta general");
        assert!(service.subtitle.contains("$60.000"));
        test_helpers::cleanup_test_db(&db_path);
    }
}
