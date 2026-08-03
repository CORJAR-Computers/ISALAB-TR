use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::consultation::ConsultationListItem;
use crate::models::dashboard::DashboardStats;
use crate::repositories::{samples as samples_repo, surgeries as surgeries_repo, vaccines as vaccines_repo};

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
    let patients_active = count(
        conn,
        "SELECT COUNT(*) FROM PATIENTS WHERE ACTIVE = TRUE",
    )?;
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

    let rows: Vec<(
        i32, i32, String, String, String, String, String, String, Option<String>,
    )> = conn.query(&sql, ()).map_err(AppError::from)?;

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
