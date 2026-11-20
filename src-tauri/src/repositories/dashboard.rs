use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::consultation::ConsultationListItem;
use crate::models::dashboard::{
    AnalyteCount, DailySampleVolume, DashboardStats, RevenueByCategory, SampleTypeTurnaround,
};
use crate::repositories::{
    appointments as appointments_repo, samples as samples_repo, surgeries as surgeries_repo,
    vaccines as vaccines_repo,
};

type ConsultationListItemRow = (
    i32,
    i32,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);

fn count(conn: &mut SimpleConnection, sql: &str) -> Result<i32, AppError> {
    conn.query_first(sql, ())
        .map_err(AppError::from)?
        .map(|(c,): (i32,)| c)
        .ok_or_else(|| AppError::Internal("COUNT sin resultado".into()))
}

fn count_where(conn: &mut SimpleConnection, sql: &str, p: &str) -> Result<i32, AppError> {
    conn.query_first(sql, (&p,))
        .map_err(AppError::from)?
        .map(|(c,): (i32,)| c)
        .ok_or_else(|| AppError::Internal("COUNT sin resultado".into()))
}

fn money(conn: &mut SimpleConnection, sql: &str) -> Result<f64, AppError> {
    conn.query_first(sql, ())
        .map_err(AppError::from)?
        .map(|(v,): (f64,)| v)
        .ok_or_else(|| AppError::Internal("SUM sin resultado".into()))
}

/// Métricas del panel de control y listas de la agenda.
pub fn get_stats(conn: &mut SimpleConnection) -> Result<DashboardStats, AppError> {
    let patients_total = count(conn, "SELECT COUNT(*) FROM PATIENTS")?;
    let patients_active = count(conn, "SELECT COUNT(*) FROM PATIENTS WHERE ACTIVE = TRUE")?;
    let samples_total = count(conn, "SELECT COUNT(*) FROM SAMPLES")?;
    let samples_in_progress = count_where(
        conn,
        "SELECT COUNT(*) FROM SAMPLES WHERE STATUS = ?",
        "EN_PROCESO",
    )?;
    let samples_finished = count_where(
        conn,
        "SELECT COUNT(*) FROM SAMPLES WHERE STATUS = ?",
        "FINALIZADA",
    )?;
    let samples_cancelled = count_where(
        conn,
        "SELECT COUNT(*) FROM SAMPLES WHERE STATUS = ?",
        "ANULADA",
    )?;
    let abnormal_results = count(
        conn,
        "SELECT COUNT(*) FROM LAB_RESULTS WHERE STATUS IN ('ALTO', 'BAJO')",
    )?;

    // Tiempo promedio recepción → finalización (horas) sobre muestras ya
    // finalizadas con UPDATED_AT posterior a la recepción.
    let avg_processing_hours = conn
        .query_first(
            "SELECT COALESCE(CAST(AVG(DATEDIFF(HOUR FROM s.RECEIVED_AT TO s.UPDATED_AT)) AS DOUBLE PRECISION), 0)
             FROM SAMPLES s
             WHERE s.STATUS = 'FINALIZADA' AND s.UPDATED_AT >= s.RECEIVED_AT",
            (),
        )
        .map_err(AppError::from)?
        .map(|(v,): (f64,)| v)
        .unwrap_or(0.0);

    // Tiempo promedio de respuesta por tipo de muestra (minutos), usando la
    // misma métrica de tiempo transcurrido que el promedio global.
    let turnaround_by_sample_type = conn
        .query(
            "SELECT s.SAMPLE_TYPE_ID, st.NAME, COUNT(*),
                    CAST(AVG(DATEDIFF(MINUTE FROM s.RECEIVED_AT TO s.UPDATED_AT)) AS DOUBLE PRECISION)
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             WHERE s.STATUS = 'FINALIZADA' AND s.UPDATED_AT >= s.RECEIVED_AT
             GROUP BY s.SAMPLE_TYPE_ID, st.NAME
             ORDER BY 4 DESC",
            (),
        )
        .map_err(AppError::from)?
        .into_iter()
        .map(|(sample_type_id, sample_type_name, count, avg_minutes): (i32, String, i32, f64)| {
            SampleTypeTurnaround {
                sample_type_id,
                sample_type_name,
                avg_minutes,
                count,
            }
        })
        .collect();

    // % de muestras finalizadas con al menos un valor fuera de rango.
    let (finished_abnormal, finished_total): (i32, i32) = {
        let row: Option<(i32, i32)> = conn
            .query_first(
                "SELECT
                   (SELECT COUNT(DISTINCT r.SAMPLE_ID) FROM LAB_RESULTS r
                     JOIN SAMPLES s ON s.ID = r.SAMPLE_ID
                     WHERE s.STATUS = 'FINALIZADA' AND r.STATUS IN ('ALTO', 'BAJO')),
                   (SELECT COUNT(*) FROM SAMPLES WHERE STATUS = 'FINALIZADA')
                 FROM RDB$DATABASE",
                (),
            )
            .map_err(AppError::from)?;
        row.unwrap_or((0, 0))
    };
    let abnormal_rate = if finished_total > 0 {
        (finished_abnormal as f64 / finished_total as f64) * 100.0
    } else {
        0.0
    };

    // Volumen diario de los últimos 7 días (incluye días sin muestras).
    let weekly_volume = list_weekly_volume(conn)?;

    // Analitos más solicitados.
    let top_analytes = conn
        .query(
            "SELECT FIRST 5 a.NAME, COUNT(*) AS N
             FROM LAB_RESULTS r
             JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
             GROUP BY a.NAME
             ORDER BY N DESC, a.NAME",
            (),
        )
        .map_err(AppError::from)?
        .into_iter()
        .map(|(name, n): (String, i32)| AnalyteCount {
            analyte_name: name,
            count: n,
        })
        .collect();

    let consultations_pending = count_where(
        conn,
        "SELECT COUNT(*) FROM CONSULTATIONS WHERE STATUS = ?",
        "PENDIENTE",
    )?;
    let surgeries_programmed = count(
        conn,
        "SELECT COUNT(*) FROM SURGERIES WHERE STATUS IN ('PROGRAMADA', 'EN_CURSO')",
    )?;
    let vaccines_due = count(
        conn,
        "SELECT COUNT(*) FROM VACCINES WHERE NEXT_DOSE_AT IS NOT NULL AND NEXT_DOSE_AT <= CURRENT_DATE",
    )?;
    let invoices_unpaid = count_where(
        conn,
        "SELECT COUNT(*) FROM INVOICES WHERE STATUS = ?",
        "EMITIDA",
    )?;
    let revenue_total = conn
        .query_first(
            "SELECT COALESCE(CAST(SUM(TOTAL) AS DOUBLE PRECISION), 0) FROM INVOICES WHERE STATUS = 'PAGADA'",
            (),
        )
        .map_err(AppError::from)?
        .map(|(v,): (f64,)| v)
        .unwrap_or(0.0);

    // ---- Cobros y cartera ----
    // Cobrado por fecha de recepción (no por fecha de emisión de la factura):
    // es el dinero que entró, que es justo lo que no distingue `revenue_total`.
    let collected_today = money(
        conn,
        "SELECT CAST(COALESCE(SUM(TOTAL), 0) AS DOUBLE PRECISION)
           FROM PAYMENTS
          WHERE STATUS = 'APLICADO' AND CAST(RECEIVED_AT AS DATE) = CURRENT_DATE",
    )?;
    let collected_month = money(
        conn,
        "SELECT CAST(COALESCE(SUM(TOTAL), 0) AS DOUBLE PRECISION)
           FROM PAYMENTS
          WHERE STATUS = 'APLICADO'
            AND EXTRACT(YEAR FROM RECEIVED_AT) = EXTRACT(YEAR FROM CURRENT_DATE)
            AND EXTRACT(MONTH FROM RECEIVED_AT) = EXTRACT(MONTH FROM CURRENT_DATE)",
    )?;
    let receivable_total = money(
        conn,
        "SELECT CAST(COALESCE(SUM(CASE WHEN b.BALANCE > 0.01 THEN b.BALANCE ELSE 0 END), 0)
                     AS DOUBLE PRECISION)
           FROM (
               SELECT CAST(i.TOTAL AS DOUBLE PRECISION) - COALESCE((
                   SELECT CAST(SUM(a.AMOUNT) AS DOUBLE PRECISION)
                   FROM PAYMENT_ALLOCATIONS a
                   JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                   WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
               ), 0) AS BALANCE
               FROM INVOICES i WHERE i.STATUS <> 'ANULADA'
           ) b",
    )?;
    let receivable_count = count(
        conn,
        "SELECT COUNT(*) FROM INVOICES i
          WHERE i.STATUS <> 'ANULADA'
            AND CAST(i.TOTAL AS DOUBLE PRECISION) - COALESCE((
                SELECT CAST(SUM(a.AMOUNT) AS DOUBLE PRECISION)
                FROM PAYMENT_ALLOCATIONS a
                JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
            ), 0) > 0.01",
    )?;
    let cash_session_open = count_where(
        conn,
        "SELECT COUNT(*) FROM CASH_SESSIONS WHERE STATUS = ?",
        "ABIERTA",
    )? > 0;
    let last_cash_difference: Option<f64> = conn
        .query_first(
            "SELECT CAST(COALESCE(DIFFERENCE, 0) AS DOUBLE PRECISION)
               FROM CASH_SESSIONS
              WHERE STATUS = 'CERRADA'
              ORDER BY CLOSED_AT DESC ROWS 1",
            (),
        )
        .map_err(AppError::from)?
        .map(|(v,)| v);

    // ---- Recepción / clínica ----
    let visits_today = count(
        conn,
        "SELECT COUNT(*) FROM VISITS WHERE CAST(ARRIVED_AT AS DATE) = CURRENT_DATE",
    )?;
    let visits_waiting = count(
        conn,
        "SELECT COUNT(*) FROM VISITS WHERE STATUS = 'EN_ESPERA'",
    )?;
    let visits_urgent_waiting = count(
        conn,
        "SELECT COUNT(*) FROM VISITS WHERE STATUS = 'EN_ESPERA' AND PRIORITY = 'URGENTE'",
    )?;
    // Espera promedio de hoy: minutos entre la llegada y el primer STARTED_AT
    // de las líneas de la visita (solo visitas que ya iniciaron atención).
    let avg_wait_minutes = conn
        .query_first(
            "SELECT COALESCE(CAST(AVG(DATEDIFF(MINUTE FROM v.ARRIVED_AT TO w.STARTED))
                    AS DOUBLE PRECISION), 0)
             FROM VISITS v
             JOIN (SELECT VISIT_ID, MIN(STARTED_AT) AS STARTED
                     FROM VISIT_SERVICES
                    WHERE STARTED_AT IS NOT NULL
                 GROUP BY VISIT_ID) w
               ON w.VISIT_ID = v.ID
             WHERE CAST(v.ARRIVED_AT AS DATE) = CURRENT_DATE",
            (),
        )
        .map_err(AppError::from)?
        .map(|(v,): (f64,)| v)
        .unwrap_or(0.0);
    let appointments_today = count(
        conn,
        "SELECT COUNT(*) FROM APPOINTMENTS WHERE CAST(SCHEDULED_AT AS DATE) = CURRENT_DATE",
    )?;
    let grooming_sessions_today = count(
        conn,
        "SELECT COUNT(*)
           FROM VISIT_SERVICES vs
           JOIN SERVICES s ON s.ID = vs.SERVICE_ID
           JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
           JOIN VISITS v ON v.ID = vs.VISIT_ID
          WHERE c.CODE = 'PELUQUERIA'
            AND CAST(v.ARRIVED_AT AS DATE) = CURRENT_DATE",
    )?;
    let revenue_by_category = list_revenue_by_category(conn)?;

    let upcoming_consultations = list_upcoming_consultations(conn, 5)?;
    let upcoming_surgeries = surgeries_repo::list_upcoming(conn, 5)?;
    let upcoming_vaccines = vaccines_repo::list_upcoming(conn, 5)?;
    let upcoming_appointments = appointments_repo::list_upcoming_appointments(conn, 5)?;
    let mut recent = samples_repo::list(conn, None, None)?;
    recent.truncate(5);

    Ok(DashboardStats {
        patients_total,
        patients_active,
        samples_total,
        samples_in_progress,
        samples_finished,
        samples_cancelled,
        abnormal_results,
        avg_processing_hours,
        turnaround_by_sample_type,
        abnormal_rate,
        weekly_volume,
        top_analytes,
        consultations_pending,
        surgeries_programmed,
        vaccines_due,
        invoices_unpaid,
        revenue_total,
        collected_today,
        collected_month,
        receivable_total,
        receivable_count,
        cash_session_open,
        last_cash_difference,
        visits_today,
        visits_waiting,
        visits_urgent_waiting,
        avg_wait_minutes,
        appointments_today,
        grooming_sessions_today,
        revenue_by_category,
        upcoming_consultations,
        upcoming_surgeries,
        upcoming_vaccines,
        recent_samples: recent,
        upcoming_appointments,
    })
}

/// Ingresos del mes en curso por categoría de servicio (facturas PAGADA con
/// líneas vinculadas al catálogo vía INVOICE_ITEMS.SERVICE_ID).
fn list_revenue_by_category(conn: &mut SimpleConnection) -> Result<Vec<RevenueByCategory>, AppError> {
    let rows: Vec<(i32, String, f64)> = conn
        .query(
            "SELECT s.CATEGORY_ID, c.NAME,
                    COALESCE(CAST(SUM(ii.LINE_TOTAL) AS DOUBLE PRECISION), 0)
             FROM INVOICES i
             JOIN INVOICE_ITEMS ii ON ii.INVOICE_ID = i.ID
             JOIN SERVICES s ON s.ID = ii.SERVICE_ID
             JOIN SERVICE_CATEGORIES c ON c.ID = s.CATEGORY_ID
             WHERE i.STATUS = 'PAGADA'
               AND EXTRACT(YEAR FROM i.ISSUE_DATE) = EXTRACT(YEAR FROM CURRENT_DATE)
               AND EXTRACT(MONTH FROM i.ISSUE_DATE) = EXTRACT(MONTH FROM CURRENT_DATE)
             GROUP BY s.CATEGORY_ID, c.NAME
             ORDER BY 3 DESC",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(category_id, category_name, revenue)| RevenueByCategory {
            category_id,
            category_name,
            revenue,
        })
        .collect())
}

/// Volumen de muestras recibidas por día en los últimos 7 días (incluye días
/// sin muestras con conteo 0, para dibujar la tendencia completa).
fn list_weekly_volume(conn: &mut SimpleConnection) -> Result<Vec<DailySampleVolume>, AppError> {
    let sql = "
        SELECT d.DIA, COUNT(s.ID) AS N
        FROM (
            SELECT CAST(DATEADD(-6 DAY TO CURRENT_DATE) AS DATE) AS DIA FROM RDB$DATABASE
            UNION ALL SELECT CAST(DATEADD(-5 DAY TO CURRENT_DATE) AS DATE) FROM RDB$DATABASE
            UNION ALL SELECT CAST(DATEADD(-4 DAY TO CURRENT_DATE) AS DATE) FROM RDB$DATABASE
            UNION ALL SELECT CAST(DATEADD(-3 DAY TO CURRENT_DATE) AS DATE) FROM RDB$DATABASE
            UNION ALL SELECT CAST(DATEADD(-2 DAY TO CURRENT_DATE) AS DATE) FROM RDB$DATABASE
            UNION ALL SELECT CAST(DATEADD(-1 DAY TO CURRENT_DATE) AS DATE) FROM RDB$DATABASE
            UNION ALL SELECT CAST(CURRENT_DATE AS DATE) FROM RDB$DATABASE
        ) d
        LEFT JOIN SAMPLES s ON CAST(s.RECEIVED_AT AS DATE) = d.DIA
        GROUP BY d.DIA
        ORDER BY d.DIA";
    let rows: Vec<(String, i32)> = conn.query(sql, ()).map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(date, count)| DailySampleVolume { date, count })
        .collect())
}

/// Próximas consultas PENDIENTE de la agenda (desde ahora, ascendente).
pub fn list_upcoming_consultations(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<ConsultationListItem>, AppError> {
    let sql = format!(
        "SELECT FIRST {limit} c.ID, c.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME,
                LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
                c.REASON, c.STATUS, u.FULL_NAME
         FROM CONSULTATIONS c
         JOIN PATIENTS p ON p.ID = c.PATIENT_ID
         JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
         JOIN OWNERS o ON o.ID = p.OWNER_ID
         LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
         WHERE c.STATUS = 'PENDIENTE' AND c.CONSULTATION_DATE >= CURRENT_TIMESTAMP
         ORDER BY c.CONSULTATION_DATE ASC"
    );

    let rows: Vec<ConsultationListItemRow> = conn.query(&sql, ()).map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| ConsultationListItem {
            id: r.0,
            patient_id: r.1,
            patient_name: r.2,
            species_name: r.3,
            owner_name: r.4,
            consultation_date: r.5,
            reason: r.6,
            status: r.7,
            veterinarian_name: r.8,
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
    fn test_get_stats_empty_database() {
        let (mut conn, db_path) = setup();
        let stats = get_stats(&mut conn).unwrap();

        // All counts should be 0 or defaults
        assert_eq!(stats.patients_total, 0);
        assert_eq!(stats.patients_active, 0);
        assert_eq!(stats.samples_total, 0);
        assert_eq!(stats.samples_in_progress, 0);
        assert_eq!(stats.samples_finished, 0);
        assert_eq!(stats.samples_cancelled, 0);
        assert_eq!(stats.abnormal_results, 0);
        assert_eq!(stats.avg_processing_hours, 0.0);
        assert!(stats.turnaround_by_sample_type.is_empty());
        assert_eq!(stats.abnormal_rate, 0.0);
        // La tendencia semanal siempre trae los 7 días (con 0 muestras).
        assert_eq!(stats.weekly_volume.len(), 7);
        assert!(stats.weekly_volume.iter().all(|d| d.count == 0));
        assert!(stats.top_analytes.is_empty());
        assert_eq!(stats.consultations_pending, 0);
        assert_eq!(stats.surgeries_programmed, 0);
        assert_eq!(stats.vaccines_due, 0);
        assert_eq!(stats.invoices_unpaid, 0);
        assert_eq!(stats.revenue_total, 0.0);
        // Cobros y cartera (fase D)
        assert_eq!(stats.collected_today, 0.0);
        assert_eq!(stats.collected_month, 0.0);
        assert_eq!(stats.receivable_total, 0.0);
        assert_eq!(stats.receivable_count, 0);
        assert!(!stats.cash_session_open);
        assert_eq!(stats.last_cash_difference, None);
        assert!(stats.upcoming_consultations.is_empty());
        assert!(stats.upcoming_surgeries.is_empty());
        assert!(stats.upcoming_vaccines.is_empty());
        assert!(stats.recent_samples.is_empty());
        // Clínica (fase 5)
        assert_eq!(stats.visits_today, 0);
        assert_eq!(stats.visits_waiting, 0);
        assert_eq!(stats.visits_urgent_waiting, 0);
        assert_eq!(stats.avg_wait_minutes, 0.0);
        assert_eq!(stats.appointments_today, 0);
        assert_eq!(stats.grooming_sessions_today, 0);
        assert!(stats.revenue_by_category.is_empty());
        assert!(stats.upcoming_appointments.is_empty());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_with_patients() {
        let (mut conn, db_path) = setup();
        insert_test_patient(&mut conn);

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.patients_total, 1);
        assert_eq!(stats.patients_active, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_with_consultations() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Create a pending consultation
        conn.execute(
            "INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, STATUS)
             VALUES (1, ?, CURRENT_TIMESTAMP, 'Consulta de prueba', 'PENDIENTE')",
            (&patient_id,),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.consultations_pending, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_with_samples() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);

        // Create samples in different statuses
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (1, ?, 1, CURRENT_TIMESTAMP, 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (2, ?, 1, CURRENT_TIMESTAMP, 'EN_PROCESO')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (3, ?, 1, CURRENT_TIMESTAMP, 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.samples_total, 3);
        assert_eq!(stats.samples_in_progress, 1);
        assert_eq!(stats.samples_finished, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_turnaround_by_sample_type() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        // Catálogo sembrado por las migraciones: id 1 = Sangre total (EDTA),
        // id 4 = Orina (los INSERT con ID explícito colisionarían con la semilla).

        // Fechas fijas: DATEDIFF determinista (60, 180 y 45 minutos).
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, UPDATED_AT, STATUS)
             VALUES (1, ?, 1, '2026-08-01 10:00:00', '2026-08-01 11:00:00', 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, UPDATED_AT, STATUS)
             VALUES (2, ?, 1, '2026-08-01 10:00:00', '2026-08-01 13:00:00', 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, UPDATED_AT, STATUS)
             VALUES (3, ?, 4, '2026-08-01 10:00:00', '2026-08-01 10:45:00', 'FINALIZADA')",
            (&patient_id,),
        )
        .unwrap();
        // No finalizada: no debe contar.
        conn.execute(
            "INSERT INTO SAMPLES (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS)
             VALUES (4, ?, 1, CURRENT_TIMESTAMP, 'RECIBIDA')",
            (&patient_id,),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.turnaround_by_sample_type.len(), 2);

        let sangre = stats
            .turnaround_by_sample_type
            .iter()
            .find(|t| t.sample_type_id == 1)
            .unwrap();
        assert_eq!(sangre.count, 2);
        assert!((sangre.avg_minutes - 120.0).abs() < 0.01); // (60 + 180) / 2

        let orina = stats
            .turnaround_by_sample_type
            .iter()
            .find(|t| t.sample_type_id == 4)
            .unwrap();
        assert_eq!(orina.count, 1);
        assert!((orina.avg_minutes - 45.0).abs() < 0.01);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_with_surgeries() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO SURGERIES (ID, PATIENT_ID, SURGERY_TYPE, SCHEDULED_AT, STATUS)
             VALUES (1, ?, 'Castración', CURRENT_TIMESTAMP, 'PROGRAMADA')",
            (&patient_id,),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.surgeries_programmed, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_stats_with_invoices() {
        let (mut conn, db_path) = setup();
        insert_test_patient(&mut conn);

        // Create unpaid invoice
        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (1, 1, 'FAC-0001', CURRENT_TIMESTAMP, 100000, 'EMITIDA')",
            (),
        )
        .unwrap();

        // Create paid invoice
        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (2, 1, 'FAC-0002', CURRENT_TIMESTAMP, 200000, 'PAGADA')",
            (),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.invoices_unpaid, 1);
        assert!((stats.revenue_total - 200000.0).abs() < 0.01);

        cleanup_test_db(&db_path);
    }

    /// Métricas de recepción/clínica (fase 5): visitas de hoy, cola de espera,
    /// espera promedio, citas y sesiones de estética de hoy e ingresos del mes
    /// por categoría.
    #[test]
    fn test_get_stats_clinic_metrics() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Visita de hoy EN_ESPERA (urgente): cuenta en la cola.
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, ARRIVED_AT, PRIORITY, STATUS)
             VALUES (1, ?, 1, CURRENT_TIMESTAMP, 'URGENTE', 'EN_ESPERA')",
            (&patient_id,),
        )
        .unwrap();
        // Visita de hoy con línea iniciada hace 30 min: espera promedio 30.
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, ARRIVED_AT, STATUS)
             VALUES (2, ?, 1, DATEADD(-30 MINUTE TO CURRENT_TIMESTAMP), 'EN_ATENCION')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO VISIT_SERVICES (ID, VISIT_ID, SERVICE_ID, UNIT_PRICE, STATUS, STARTED_AT)
             VALUES (1, 2, 1, 50000, 'EN_PROCESO', CURRENT_TIMESTAMP)",
            (),
        )
        .unwrap();
        // Visita de ayer: no cuenta en las métricas de hoy.
        conn.execute(
            "INSERT INTO VISITS (ID, PATIENT_ID, OWNER_ID, ARRIVED_AT, STATUS)
             VALUES (3, ?, 1, DATEADD(-1 DAY TO CURRENT_TIMESTAMP), 'EN_ESPERA')",
            (&patient_id,),
        )
        .unwrap();

        // Cita de hoy (y otra de mañana, que no cuenta para hoy).
        conn.execute(
            "INSERT INTO APPOINTMENTS (ID, CODE, PATIENT_ID, SERVICE_ID, SCHEDULED_AT,
                                        DURATION_MIN, STATUS)
             VALUES (1, 'C-2026-0001', ?, 1, CURRENT_TIMESTAMP, 30, 'PROGRAMADA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO APPOINTMENTS (ID, CODE, PATIENT_ID, SERVICE_ID, SCHEDULED_AT,
                                        DURATION_MIN, STATUS)
             VALUES (2, 'C-2026-0002', ?, 1, DATEADD(1 DAY TO CURRENT_TIMESTAMP), 30, 'PROGRAMADA')",
            (&patient_id,),
        )
        .unwrap();

        // Línea de estética (PELUQUERIA) en visita de hoy.
        conn.execute(
            "INSERT INTO VISIT_SERVICES (ID, VISIT_ID, SERVICE_ID, UNIT_PRICE, STATUS)
             VALUES (2, 1, 11, 60000, 'PENDIENTE')",
            (),
        )
        .unwrap();

        // Factura PAGADA de este mes con línea del catálogo (servicio 1).
        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, PATIENT_ID, INVOICE_NUMBER, ISSUE_DATE,
                                    TOTAL, STATUS)
             VALUES (1, 1, ?, 'FAC-0001', CURRENT_TIMESTAMP, 95000, 'PAGADA')",
            (&patient_id,),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO INVOICE_ITEMS (ID, INVOICE_ID, SERVICE_ID, DESCRIPTION, QUANTITY,
                                         UNIT_PRICE, LINE_TOTAL)
             VALUES (1, 1, 1, 'Consulta general', 1, 95000, 95000)",
            (),
        )
        .unwrap();

        // Próximas citas (1 de hoy ya pasó de las futuras de mañana).
        let stats = get_stats(&mut conn).unwrap();
        assert_eq!(stats.visits_today, 2);
        // Cola de espera: la urgente de hoy y la de ayer siguen EN_ESPERA.
        assert_eq!(stats.visits_waiting, 2);
        assert_eq!(stats.visits_urgent_waiting, 1);
        assert!((stats.avg_wait_minutes - 30.0).abs() < 0.01);
        assert_eq!(stats.appointments_today, 1);
        assert_eq!(stats.grooming_sessions_today, 1);
        assert_eq!(stats.revenue_by_category.len(), 1);
        assert!((stats.revenue_by_category[0].revenue - 95000.0).abs() < 0.01);
        assert!(!stats.upcoming_appointments.is_empty());

        cleanup_test_db(&db_path);
    }

    /// El listado de próximas citas del panel solo trae PROGRAMADA/CONFIRMADA
    /// desde ahora, en orden ascendente y limitado.
    #[test]
    fn test_list_upcoming_appointments() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        for i in 1..=3 {
            let code = format!("C-2026-000{i}");
            // La fecha se construye con un literal: Firebird no acepta un
            // parámetro enlazado dentro de DATEADD.
            let sql = format!(
                "INSERT INTO APPOINTMENTS (ID, CODE, PATIENT_ID, SERVICE_ID, SCHEDULED_AT,
                                            DURATION_MIN, STATUS)
                 VALUES (?, ?, ?, 1, DATEADD({i} HOUR TO CURRENT_TIMESTAMP),
                         30, 'PROGRAMADA')"
            );
            conn.execute(&sql, (&i, &code, &patient_id)).unwrap();
        }
        // Una cancelada no aparece.
        conn.execute(
            "INSERT INTO APPOINTMENTS (ID, CODE, PATIENT_ID, SERVICE_ID, SCHEDULED_AT,
                                        DURATION_MIN, STATUS)
             VALUES (9, 'C-2026-0009', ?, 1, DATEADD(9 HOUR TO CURRENT_TIMESTAMP),
                     30, 'CANCELADA')",
            (&patient_id,),
        )
        .unwrap();

        let items = appointments_repo::list_upcoming_appointments(&mut conn, 2).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].status, "PROGRAMADA");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_upcoming_consultations_empty() {
        let (mut conn, db_path) = setup();
        let consultations = list_upcoming_consultations(&mut conn, 5).unwrap();
        assert!(consultations.is_empty());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_upcoming_consultations_with_data() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Create a future pending consultation
        conn.execute(
            "INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, STATUS)
             VALUES (1, ?, DATEADD(1 DAY TO CURRENT_TIMESTAMP), 'Consulta futura', 'PENDIENTE')",
            (&patient_id,),
        )
        .unwrap();

        let consultations = list_upcoming_consultations(&mut conn, 5).unwrap();
        assert_eq!(consultations.len(), 1);
        assert_eq!(consultations[0].reason, "Consulta futura");
        assert_eq!(consultations[0].status, "PENDIENTE");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_upcoming_consultations_excludes_past() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Create a past consultation
        conn.execute(
            "INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, STATUS)
             VALUES (1, ?, DATEADD(-1 DAY TO CURRENT_TIMESTAMP), 'Consulta pasada', 'PENDIENTE')",
            (&patient_id,),
        )
        .unwrap();

        let consultations = list_upcoming_consultations(&mut conn, 5).unwrap();
        assert!(consultations.is_empty());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_upcoming_consultations_excludes_completed() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Create a completed consultation
        conn.execute(
            "INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, STATUS)
             VALUES (1, ?, DATEADD(1 DAY TO CURRENT_TIMESTAMP), 'Consulta completada', 'COMPLETADA')",
            (&patient_id,),
        ).unwrap();

        let consultations = list_upcoming_consultations(&mut conn, 5).unwrap();
        assert!(consultations.is_empty());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_upcoming_consultations_limit() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Create 3 future consultations
        for i in 1..=3 {
            conn.execute(
                "INSERT INTO CONSULTATIONS (ID, PATIENT_ID, CONSULTATION_DATE, REASON, STATUS)
                 VALUES (?, ?, DATEADD(? DAY TO CURRENT_TIMESTAMP), ?, 'PENDIENTE')",
                (&i, &patient_id, &i, &format!("Consulta {}", i)),
            )
            .unwrap();
        }

        // Request only 2
        let consultations = list_upcoming_consultations(&mut conn, 2).unwrap();
        assert_eq!(consultations.len(), 2);

        cleanup_test_db(&db_path);
    }

    /// Cobros y cartera (fase D): cobrado por recepción, saldo pendiente y caja.
    #[test]
    fn test_get_stats_billing_metrics() {
        use crate::models::payment::{
            CreatePaymentAllocationInput, CreatePaymentLineInput, RegisterPaymentInput,
        };
        use crate::repositories::{cash, payments};

        let (mut conn, db_path) = setup();
        insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (1, 1, 'FAC-9001', CURRENT_TIMESTAMP, 100000, 'EMITIDA')",
            (),
        )
        .unwrap();

        cash::open_session(&mut conn, 100000.0, None, 1).unwrap();
        payments::register(
            &mut conn,
            &RegisterPaymentInput {
                owner_id: 1,
                patient_id: None,
                notes: None,
                lines: vec![CreatePaymentLineInput {
                    method_code: "EFECTIVO".into(),
                    amount: 40000.0,
                    reference: None,
                }],
                allocations: vec![CreatePaymentAllocationInput {
                    invoice_id: 1,
                    amount: 40000.0,
                }],
            },
            Some(1),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert!((stats.collected_today - 40000.0).abs() < 0.01);
        assert!((stats.collected_month - 40000.0).abs() < 0.01);
        assert!((stats.receivable_total - 60000.0).abs() < 0.01);
        assert_eq!(stats.receivable_count, 1);
        assert!(stats.cash_session_open);
        assert_eq!(stats.last_cash_difference, None);

        cleanup_test_db(&db_path);
    }

    /// Una factura anulada no entra en la cartera pendiente.
    #[test]
    fn test_get_stats_receivable_ignores_cancelled_invoices() {
        let (mut conn, db_path) = setup();
        insert_test_patient(&mut conn);

        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (1, 1, 'FAC-9010', CURRENT_TIMESTAMP, 80000, 'EMITIDA')",
            (),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO INVOICES (ID, OWNER_ID, INVOICE_NUMBER, ISSUE_DATE, TOTAL, STATUS)
             VALUES (2, 1, 'FAC-9011', CURRENT_TIMESTAMP, 90000, 'ANULADA')",
            (),
        )
        .unwrap();

        let stats = get_stats(&mut conn).unwrap();
        assert!((stats.receivable_total - 80000.0).abs() < 0.01);
        assert_eq!(stats.receivable_count, 1);
        assert!(!stats.cash_session_open);

        cleanup_test_db(&db_path);
    }
}
