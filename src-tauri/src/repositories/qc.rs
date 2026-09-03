use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::qc::{
    QcAnalyzerStatus, QcChartData, QcChartPoint, QcControlMaterial, QcMaterialInput, QcRun,
    QcRunInput, QcRunMeasurement, QcTarget,
};
use crate::repositories::next_id;

// ============================ WESTGARD MULTIRULES ===========================

/// Resultado de evaluar las reglas multirregla de Westgard sobre una serie de
/// puntuaciones z (historial del mismo analito/material + la medición nueva).
#[derive(Debug, Clone, PartialEq)]
pub struct WestgardEvaluation {
    /// True si se viola alguna regla de rechazo (1_3s, 2_2s, R_4s, 4_1s, 10x).
    pub rejected: bool,
    /// Reglas violadas (1_2s es solo aviso, no rechaza la corrida).
    pub rules_violated: Vec<String>,
}

fn sign(z: f64) -> i8 {
    if z > 0.0 {
        1
    } else if z < 0.0 {
        -1
    } else {
        0
    }
}

/// Evalúa las reglas Westgard sobre un historial de z-scores (del más antiguo
/// al más reciente). Las reglas de rechazo son 1_3s, 2_2s, R_4s, 4_1s y 10x;
/// 1_2s es una regla de advertencia que por sí sola no rechaza la corrida.
pub fn evaluate_westgard(history: &[f64]) -> WestgardEvaluation {
    let mut rules: Vec<String> = Vec::new();

    // 1_3s — un valor a más de 3 SD (error aleatorio).
    if history.iter().any(|z| z.abs() >= 3.0) {
        rules.push("1_3s".to_string());
    }

    // 2_2s — dos valores consecutivos a más de 2 SD en el mismo lado (error
    // sistemático incipiente).
    for w in history.windows(2) {
        if w[0].abs() >= 2.0 && w[1].abs() >= 2.0 && sign(w[0]) != 0 && sign(w[0]) == sign(w[1]) {
            rules.push("2_2s".to_string());
            break;
        }
    }

    // R_4s — rango entre dos valores consecutivos >= 4 SD (error aleatorio).
    for w in history.windows(2) {
        if (w[0] - w[1]).abs() >= 4.0 {
            rules.push("R_4s".to_string());
            break;
        }
    }

    // 4_1s — cuatro valores consecutivos a >= 1 SD en el mismo lado.
    for w in history.windows(4) {
        if w[0].abs() >= 1.0
            && w[0].signum() != 0.0
            && w.iter()
                .all(|z| z.abs() >= 1.0 && z.signum() == w[0].signum())
        {
            rules.push("4_1s".to_string());
            break;
        }
    }

    // 10x — diez valores consecutivos en el mismo lado (deriva persistente).
    for w in history.windows(10) {
        if w[0].signum() != 0.0 && w.iter().all(|z| z.signum() == w[0].signum()) {
            rules.push("10x".to_string());
            break;
        }
    }

    // 1_2s — advertencia (no rechaza por sí sola).
    if history.iter().any(|z| z.abs() >= 2.0) {
        rules.push("1_2s".to_string());
    }

    let rejection_rules = ["1_3s", "2_2s", "R_4s", "4_1s", "10x"];
    let rejected = rules.iter().any(|r| rejection_rules.contains(&r.as_str()));

    WestgardEvaluation {
        rejected,
        rules_violated: rules,
    }
}

/// Reglas recién violadas al añadir el punto `history[last]`.
fn new_violations_at_point(history: &[f64]) -> Vec<String> {
    let now = evaluate_westgard(history).rules_violated;
    if history.len() <= 1 {
        return now;
    }
    let before = evaluate_westgard(&history[..history.len() - 1]).rules_violated;
    now.into_iter().filter(|r| !before.contains(r)).collect()
}

// ============================ CONTROL MATERIALS =============================

type MaterialRow = (
    i32,
    String,
    i32,
    String,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    i32,
);

fn map_material(r: MaterialRow) -> QcControlMaterial {
    QcControlMaterial {
        id: r.0,
        name: r.1,
        analyzer_id: r.2,
        analyzer_name: r.3,
        lot: r.4,
        expires_at: r.5,
        is_active: r.6,
        notes: r.7,
        target_count: r.8,
    }
}

/// Lista los materiales de control con su equipo y nº de objetivos.
pub fn list_control_materials(
    conn: &mut SimpleConnection,
) -> Result<Vec<QcControlMaterial>, AppError> {
    let rows: Vec<MaterialRow> = conn
        .query(
            "SELECT cm.ID, cm.NAME, cm.ANALYZER_ID, az.NAME,
                    cm.LOT,
                    CASE WHEN cm.EXPIRES_AT IS NULL THEN NULL
                         ELSE CAST(cm.EXPIRES_AT AS VARCHAR(10)) END,
                    cm.IS_ACTIVE, cm.NOTES,
                    (SELECT COUNT(*) FROM QC_TARGETS t WHERE t.CONTROL_MATERIAL_ID = cm.ID)
             FROM QC_CONTROL_MATERIALS cm
             JOIN ANALYZERS az ON az.ID = cm.ANALYZER_ID
             WHERE cm.IS_ACTIVE = TRUE
             ORDER BY cm.NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_material).collect())
}

type TargetRow = (i32, i32, i32, String, Option<String>, f64, f64);

/// Objetivos (media/SD) de un material de control.
pub fn list_targets(
    conn: &mut SimpleConnection,
    material_id: i32,
) -> Result<Vec<QcTarget>, AppError> {
    let rows: Vec<TargetRow> = conn
        .query(
            "SELECT t.ID, t.CONTROL_MATERIAL_ID, t.ANALYTE_ID, a.NAME, a.UNIT, t.MEAN, t.SD
             FROM QC_TARGETS t
             JOIN ANALYTES a ON a.ID = t.ANALYTE_ID
             WHERE t.CONTROL_MATERIAL_ID = ?
             ORDER BY a.NAME",
            (&material_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| QcTarget {
            id: r.0,
            control_material_id: r.1,
            analyte_id: r.2,
            analyte_name: r.3,
            unit: r.4,
            mean: r.5,
            sd: r.6,
        })
        .collect())
}

/// Crea o actualiza un material de control, reemplazando sus objetivos.
pub fn save_control_material(
    conn: &mut SimpleConnection,
    input: &QcMaterialInput,
) -> Result<QcControlMaterial, AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "El nombre del material de control es obligatorio".into(),
        ));
    }
    if input.targets.is_empty() {
        return Err(AppError::Validation(
            "El material debe tener al menos un analito objetivo".into(),
        ));
    }
    for t in &input.targets {
        if t.sd <= 0.0 {
            return Err(AppError::Validation(
                "La desviación estándar debe ser mayor que cero".into(),
            ));
        }
    }

    let id = match input.id {
        Some(id) => {
            conn.execute(
                "UPDATE QC_CONTROL_MATERIALS
                    SET NAME = ?, ANALYZER_ID = ?, LOT = ?, EXPIRES_AT = ?, NOTES = ?,
                        UPDATED_AT = CURRENT_TIMESTAMP
                  WHERE ID = ?",
                (
                    &name,
                    &input.analyzer_id,
                    &input.lot,
                    &input.expires_at,
                    &input.notes,
                    &id,
                ),
            )
            .map_err(AppError::from)?;
            conn.execute(
                "DELETE FROM QC_TARGETS WHERE CONTROL_MATERIAL_ID = ?",
                (&id,),
            )
            .map_err(AppError::from)?;
            id
        }
        None => {
            let nid = next_id(conn, "GEN_QC_CONTROL_MATERIALS_ID")?;
            conn.execute(
                "INSERT INTO QC_CONTROL_MATERIALS (ID, NAME, ANALYZER_ID, LOT, EXPIRES_AT, NOTES)
                 VALUES (?, ?, ?, ?, ?, ?)",
                (
                    &nid,
                    &name,
                    &input.analyzer_id,
                    &input.lot,
                    &input.expires_at,
                    &input.notes,
                ),
            )
            .map_err(AppError::from)?;
            nid
        }
    };

    for t in &input.targets {
        let tid = next_id(conn, "GEN_QC_TARGETS_ID")?;
        conn.execute(
            "INSERT INTO QC_TARGETS (ID, CONTROL_MATERIAL_ID, ANALYTE_ID, MEAN, SD)
             VALUES (?, ?, ?, ?, ?)",
            (&tid, &id, &t.analyte_id, &t.mean, &t.sd),
        )
        .map_err(AppError::from)?;
    }

    let row: Option<MaterialRow> = conn
        .query_first(
            "SELECT cm.ID, cm.NAME, cm.ANALYZER_ID, az.NAME,
                    cm.LOT,
                    CASE WHEN cm.EXPIRES_AT IS NULL THEN NULL
                         ELSE CAST(cm.EXPIRES_AT AS VARCHAR(10)) END,
                    cm.IS_ACTIVE, cm.NOTES,
                    (SELECT COUNT(*) FROM QC_TARGETS t WHERE t.CONTROL_MATERIAL_ID = cm.ID)
             FROM QC_CONTROL_MATERIALS cm
             JOIN ANALYZERS az ON az.ID = cm.ANALYZER_ID
             WHERE cm.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(map_material)
        .ok_or_else(|| AppError::Internal("Material guardado pero no recuperado".into()))
}

pub fn delete_control_material(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM QC_CONTROL_MATERIALS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

// ============================ CORRIDAS DE CONTROL ===========================

type MeasurementRow = (
    i32,
    i32,
    i32,
    String,
    Option<String>,
    f64,
    Option<f64>,
    Option<String>,
);

fn map_measurement(r: MeasurementRow) -> QcRunMeasurement {
    QcRunMeasurement {
        id: r.0,
        qc_run_id: r.1,
        analyte_id: r.2,
        analyte_name: r.3,
        unit: r.4,
        value: r.5,
        z_score: r.6,
        violation: r.7,
    }
}

type RunRow = (
    i32,
    i32,
    String,
    i32,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

fn map_run(r: RunRow) -> QcRun {
    QcRun {
        id: r.0,
        control_material_id: r.1,
        control_name: r.2,
        analyzer_id: r.3,
        analyzer_name: r.4,
        run_date: r.5,
        status: r.6,
        notes: r.7,
        created_by: r.8,
        measurements: Vec::new(),
    }
}

/// Historial de z-scores recientes del analito para el mismo material
/// (los más antiguos primero), para la evaluación multirregla.
fn z_history(
    conn: &mut SimpleConnection,
    material_id: i32,
    analyte_id: i32,
) -> Result<Vec<f64>, AppError> {
    let rows: Vec<(f64,)> = conn
        .query(
            "SELECT FIRST 30 m.Z_SCORE
             FROM QC_RUN_MEASUREMENTS m
             JOIN QC_RUNS r ON r.ID = m.QC_RUN_ID
             WHERE r.CONTROL_MATERIAL_ID = ? AND m.ANALYTE_ID = ?
               AND m.Z_SCORE IS NOT NULL
             ORDER BY r.RUN_DATE ASC, r.ID ASC",
            (&material_id, &analyte_id),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(|(z,)| z).collect())
}

/// Registra una corrida de control evaluando las reglas de Westgard por
/// analito contra el historial reciente del mismo material.
pub fn record_run(
    conn: &mut SimpleConnection,
    input: &QcRunInput,
    username: &str,
) -> Result<QcRun, AppError> {
    // Material y sus objetivos.
    let material: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM QC_CONTROL_MATERIALS WHERE ID = ? AND IS_ACTIVE = TRUE",
            (&input.control_material_id,),
        )
        .map_err(AppError::from)?;
    let (material_id,) = material
        .ok_or_else(|| AppError::NotFound("Material de control no encontrado o inactivo".into()))?;

    let targets: Vec<(i32, f64, f64)> = conn
        .query(
            "SELECT ANALYTE_ID, MEAN, SD FROM QC_TARGETS WHERE CONTROL_MATERIAL_ID = ?",
            (&input.control_material_id,),
        )
        .map_err(AppError::from)?;
    if targets.is_empty() {
        return Err(AppError::Validation(
            "El material de control no tiene analitos objetivo".into(),
        ));
    }

    // Analito → (mean, sd).
    let target_map: std::collections::HashMap<i32, (f64, f64)> =
        targets.iter().map(|&(a, m, s)| (a, (m, s))).collect();

    // Validar y calcular z-scores en el orden dado.
    let mut z_scores: Vec<f64> = Vec::with_capacity(input.measurements.len());
    for m in &input.measurements {
        let (mean, sd) = target_map.get(&m.analyte_id).copied().ok_or_else(|| {
            AppError::Validation(format!(
                "El analito {} no tiene objetivo en este material de control",
                m.analyte_id
            ))
        })?;
        z_scores.push((m.value - mean) / sd);
    }

    // Evaluar Westgard por analito con historial (incluye el punto nuevo).
    let mut violations_by_analyte: std::collections::HashMap<i32, Vec<String>> =
        std::collections::HashMap::new();
    let mut any_rejected = false;
    for (idx, m) in input.measurements.iter().enumerate() {
        let mut history = z_history(conn, material_id, m.analyte_id)?;
        history.push(z_scores[idx]);
        let new = new_violations_at_point(&history);
        let rejected = evaluate_westgard(&history).rejected;
        if rejected {
            any_rejected = true;
        }
        violations_by_analyte.insert(m.analyte_id, new);
    }

    // Persistir la corrida.
    let analyzer_id: i32 = {
        let row: Option<(i32,)> = conn
            .query_first(
                "SELECT ANALYZER_ID FROM QC_CONTROL_MATERIALS WHERE ID = ?",
                (&material_id,),
            )
            .map_err(AppError::from)?;
        row.map(|(a,)| a).unwrap_or(1)
    };
    let run_id = next_id(conn, "GEN_QC_RUNS_ID")?;
    let status = if any_rejected {
        "RECHAZADO"
    } else {
        "ACEPTADO"
    };
    conn.execute(
        "INSERT INTO QC_RUNS (ID, CONTROL_MATERIAL_ID, ANALYZER_ID, STATUS, NOTES, CREATED_BY)
         VALUES (?, ?, ?, ?, ?, ?)",
        (
            &run_id,
            &material_id,
            &analyzer_id,
            &status,
            &input.notes,
            &username,
        ),
    )
    .map_err(AppError::from)?;

    for (idx, m) in input.measurements.iter().enumerate() {
        let mid = next_id(conn, "GEN_QC_RUN_MEASUREMENTS_ID")?;
        let violation = violations_by_analyte
            .get(&m.analyte_id)
            .filter(|v| !v.is_empty())
            .map(|v| v.join(", "));
        conn.execute(
            "INSERT INTO QC_RUN_MEASUREMENTS
                (ID, QC_RUN_ID, ANALYTE_ID, RESULT_VALUE, Z_SCORE, VIOLATION)
             VALUES (?, ?, ?, ?, ?, ?)",
            (
                &mid,
                &run_id,
                &m.analyte_id,
                &m.value,
                &z_scores[idx],
                &violation,
            ),
        )
        .map_err(AppError::from)?;
    }

    get_run(conn, run_id)
}

/// Devuelve una corrida con sus mediciones.
pub fn get_run(conn: &mut SimpleConnection, run_id: i32) -> Result<QcRun, AppError> {
    let row: Option<RunRow> = conn
        .query_first(
            "SELECT r.ID, r.CONTROL_MATERIAL_ID, cm.NAME, r.ANALYZER_ID, az.NAME,
                    LEFT(CAST(r.RUN_DATE AS VARCHAR(60)), 19),
                    r.STATUS, r.NOTES, u.USERNAME
             FROM QC_RUNS r
             JOIN QC_CONTROL_MATERIALS cm ON cm.ID = r.CONTROL_MATERIAL_ID
             JOIN ANALYZERS az ON az.ID = r.ANALYZER_ID
             LEFT JOIN USERS u ON u.ID = r.CREATED_BY
             WHERE r.ID = ?",
            (&run_id,),
        )
        .map_err(AppError::from)?;
    let mut run = row
        .map(map_run)
        .ok_or_else(|| AppError::NotFound(format!("Corrida QC {run_id} no encontrada")))?;

    let rows: Vec<MeasurementRow> = conn
        .query(
            "SELECT m.ID, m.QC_RUN_ID, m.ANALYTE_ID, a.NAME, a.UNIT,
                    m.RESULT_VALUE, m.Z_SCORE, m.VIOLATION
             FROM QC_RUN_MEASUREMENTS m
             JOIN ANALYTES a ON a.ID = m.ANALYTE_ID
             WHERE m.QC_RUN_ID = ?
             ORDER BY a.NAME",
            (&run_id,),
        )
        .map_err(AppError::from)?;
    run.measurements = rows.into_iter().map(map_measurement).collect();
    Ok(run)
}

/// Lista corridas de control (opcionalmente de un material), más recientes primero.
pub fn list_runs(
    conn: &mut SimpleConnection,
    material_id: Option<i32>,
) -> Result<Vec<QcRun>, AppError> {
    let sql = "
        SELECT r.ID, r.CONTROL_MATERIAL_ID, cm.NAME, r.ANALYZER_ID, az.NAME,
               LEFT(CAST(r.RUN_DATE AS VARCHAR(60)), 19),
               r.STATUS, r.NOTES, u.USERNAME
        FROM QC_RUNS r
        JOIN QC_CONTROL_MATERIALS cm ON cm.ID = r.CONTROL_MATERIAL_ID
        JOIN ANALYZERS az ON az.ID = r.ANALYZER_ID
        LEFT JOIN USERS u ON u.ID = r.CREATED_BY
        WHERE (? IS NULL OR r.CONTROL_MATERIAL_ID = ?)
        ORDER BY r.RUN_DATE DESC, r.ID DESC";
    let rows: Vec<RunRow> = conn
        .query(sql, (&material_id, &material_id))
        .map_err(AppError::from)?;

    let mut runs: Vec<QcRun> = rows.into_iter().map(map_run).collect();
    for run in &mut runs {
        let rows: Vec<MeasurementRow> = conn
            .query(
                "SELECT m.ID, m.QC_RUN_ID, m.ANALYTE_ID, a.NAME, a.UNIT,
                        m.RESULT_VALUE, m.Z_SCORE, m.VIOLATION
                 FROM QC_RUN_MEASUREMENTS m
                 JOIN ANALYTES a ON a.ID = m.ANALYTE_ID
                 WHERE m.QC_RUN_ID = ?
                 ORDER BY a.NAME",
                (&run.id,),
            )
            .map_err(AppError::from)?;
        run.measurements = rows.into_iter().map(map_measurement).collect();
    }
    Ok(runs)
}

pub fn delete_run(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM QC_RUNS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

// ============================ GRÁFICO L-J ===================================

/// Datos Levey-Jennings de un analito: objetivo, bandas y últimos puntos.
pub fn get_chart(
    conn: &mut SimpleConnection,
    material_id: i32,
    analyte_id: i32,
) -> Result<Option<QcChartData>, AppError> {
    let target: Option<(f64, f64)> = conn
        .query_first(
            "SELECT MEAN, SD FROM QC_TARGETS
             WHERE CONTROL_MATERIAL_ID = ? AND ANALYTE_ID = ?",
            (&material_id, &analyte_id),
        )
        .map_err(AppError::from)?;
    let Some((mean, sd)) = target else {
        return Ok(None);
    };

    let analyte_name: Option<(String, Option<String>)> = conn
        .query_first(
            "SELECT NAME, UNIT FROM ANALYTES WHERE ID = ?",
            (&analyte_id,),
        )
        .map_err(AppError::from)?;
    let (analyte_name, unit) = analyte_name.unwrap_or((format!("Analito {analyte_id}"), None));

    type ChartRow = (i32, String, f64, f64, Option<String>);
    let points: Vec<ChartRow> = conn
        .query(
            "SELECT FIRST 30 m.ID, r.RUN_DATE, m.RESULT_VALUE, m.Z_SCORE, m.VIOLATION
             FROM QC_RUN_MEASUREMENTS m
             JOIN QC_RUNS r ON r.ID = m.QC_RUN_ID
             WHERE r.CONTROL_MATERIAL_ID = ? AND m.ANALYTE_ID = ?
               AND m.Z_SCORE IS NOT NULL
             ORDER BY r.RUN_DATE ASC, r.ID ASC",
            (&material_id, &analyte_id),
        )
        .map_err(AppError::from)?;

    Ok(Some(QcChartData {
        control_material_id: material_id,
        analyte_id,
        analyte_name,
        unit,
        mean,
        sd,
        points: points
            .into_iter()
            .map(|p| QcChartPoint {
                run_id: p.0,
                run_date: p.1,
                value: p.2,
                z_score: p.3,
                violation: p.4,
            })
            .collect(),
    }))
}

/// Último estado de corrida QC por analizador (badge de alerta en la UI).
pub fn list_analyzer_status(
    conn: &mut SimpleConnection,
) -> Result<Vec<QcAnalyzerStatus>, AppError> {
    let rows: Vec<(i32, Option<String>)> = conn
        .query(
            "SELECT a.ID,
                    (SELECT FIRST 1 r.STATUS FROM QC_RUNS r
                      WHERE r.ANALYZER_ID = a.ID ORDER BY r.RUN_DATE DESC, r.ID DESC)
             FROM ANALYZERS a
             WHERE a.IS_ACTIVE = TRUE AND a.CODE <> 'GENERAL'
             ORDER BY a.NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(analyzer_id, latest_status)| QcAnalyzerStatus {
            analyzer_id,
            latest_status,
        })
        .collect())
}

// ============================ TESTS =========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_westgard_accepts_normal_run() {
        // Todos dentro de ±2 SD → aceptado, sin violaciones.
        let ev = evaluate_westgard(&[0.5, -0.2, 1.1, -1.3, 0.9]);
        assert!(!ev.rejected);
        assert!(ev.rules_violated.is_empty());
    }

    #[test]
    fn test_westgard_1_3s_rejects() {
        let ev = evaluate_westgard(&[0.5, 3.2]);
        assert!(ev.rejected);
        assert!(ev.rules_violated.contains(&"1_3s".to_string()));
    }

    #[test]
    fn test_westgard_1_2s_is_warning_only() {
        let ev = evaluate_westgard(&[2.1]);
        assert!(!ev.rejected, "1_2s sola no rechaza");
        assert!(ev.rules_violated.contains(&"1_2s".to_string()));
    }

    #[test]
    fn test_westgard_2_2s_rejects() {
        // Dos consecutivos ≥ 2 SD en el mismo lado.
        let ev = evaluate_westgard(&[2.2, 2.4, -0.5]);
        assert!(ev.rejected);
        assert!(ev.rules_violated.contains(&"2_2s".to_string()));
    }

    #[test]
    fn test_westgard_r_4s_rejects() {
        // Rango entre consecutivos ≥ 4 SD.
        let ev = evaluate_westgard(&[2.3, -1.9]);
        assert!(ev.rejected);
        assert!(ev.rules_violated.contains(&"R_4s".to_string()));
    }

    #[test]
    fn test_westgard_4_1s_rejects() {
        // Cuatro consecutivos ≥ 1 SD en el mismo lado.
        let ev = evaluate_westgard(&[1.2, 1.5, 1.1, 1.4, -0.3]);
        assert!(ev.rejected);
        assert!(ev.rules_violated.contains(&"4_1s".to_string()));
    }

    #[test]
    fn test_westgard_10x_rejects() {
        // Diez consecutivos en el mismo lado (deriva).
        let ev = evaluate_westgard(&[0.5, 0.6, 0.4, 0.7, 0.5, 0.8, 0.6, 0.5, 0.7, 0.9]);
        assert!(ev.rejected);
        assert!(ev.rules_violated.contains(&"10x".to_string()));
    }

    #[test]
    fn test_westgard_10x_not_triggered_with_flip() {
        // Un cambio de signo rompe la racha de 10x.
        let ev = evaluate_westgard(&[0.5, 0.6, 0.4, 0.7, 0.5, -0.2, 0.6, 0.5, 0.7, 0.9]);
        assert!(!ev.rejected);
        assert!(!ev.rules_violated.contains(&"10x".to_string()));
    }
}
