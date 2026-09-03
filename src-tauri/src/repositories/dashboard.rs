use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::consultation::ConsultationListItem;
use crate::models::dashboard::{
    AnalyteCount, DailySampleVolume, DashboardStats, SampleTypeTurnaround,
};
use crate::repositories::{
    samples as samples_repo, surgeries as surgeries_repo, vaccines as vaccines_repo,
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

    let upcoming_consultations = list_upcoming_consultations(conn, 5)?;
    let upcoming_surgeries = surgeries_repo::list_upcoming(conn, 5)?;
    let upcoming_vaccines = vaccines_repo::list_upcoming(conn, 5)?;
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
        upcoming_consultations,
        upcoming_surgeries,
        upcoming_vaccines,
        recent_samples: recent,
    })
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
        assert!(stats.upcoming_consultations.is_empty());
        assert!(stats.upcoming_surgeries.is_empty());
        assert!(stats.upcoming_vaccines.is_empty());
        assert!(stats.recent_samples.is_empty());

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
}
