use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::clinical_history::ClinicalHistory;
use crate::models::consultation::{
    Consultation, ConsultationListItem, CreateConsultationInput,
};
use crate::models::sample::{CreateSampleInput, LabResult, RegisterResultInput, Sample};
use crate::repositories::{
    next_id, patient as patient_repo, samples as samples_repo, vaccines as vaccines_repo,
};

pub fn get_clinical_history(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<ClinicalHistory, AppError> {
    let patient = patient_repo::get(conn, patient_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Paciente {patient_id} no encontrado"))
    })?;

    let owner = patient_repo::get_owner(conn, patient.owner_id)?;
    let consultations = list_consultations(conn, patient_id)?;
    let vaccines = vaccines_repo::by_patient(conn, patient_id)?;
    let samples = list_samples(conn, patient_id)?;

    Ok(ClinicalHistory {
        patient,
        owner,
        consultations,
        vaccines,
        samples,
    })
}

// ============================ CONSULTAS =====================================

fn list_consultations(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<Consultation>, AppError> {
    let rows: Vec<(
        i32,
        i32,
        Option<i32>,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    )> = conn
        .query(
            "SELECT c.ID, c.PATIENT_ID, c.VETERINARIAN_ID,
                    LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
                    c.REASON, c.ANAMNESIS, c.PHYSICAL_EXAM, c.DIAGNOSIS,
                    c.TREATMENT_PLAN, c.STATUS, u.FULL_NAME
             FROM CONSULTATIONS c
             LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
             WHERE c.PATIENT_ID = ?
             ORDER BY c.CONSULTATION_DATE DESC",
            (&patient_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| Consultation {
            id: r.0,
            patient_id: r.1,
            veterinarian_id: r.2,
            consultation_date: r.3,
            reason: r.4,
            anamnesis: r.5,
            physical_exam: r.6,
            diagnosis: r.7,
            treatment_plan: r.8,
            status: r.9,
            veterinarian_name: r.10,
        })
        .collect())
}

/// Agenda: listado global de consultas con filtros opcionales por estado y
/// búsqueda (paciente, propietario o motivo).
pub fn list_agenda(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<ConsultationListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT c.ID, c.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME,
               LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
               c.REASON, c.STATUS, u.FULL_NAME
        FROM CONSULTATIONS c
        JOIN PATIENTS p ON p.ID = c.PATIENT_ID
        JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
        JOIN OWNERS o ON o.ID = p.OWNER_ID
        LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
        WHERE (? IS NULL OR c.STATUS = ?)
          AND (? IS NULL
               OR UPPER(p.NAME) LIKE UPPER(?)
               OR UPPER(o.FULL_NAME) LIKE UPPER(?)
               OR UPPER(c.REASON) LIKE UPPER(?))
        ORDER BY c.CONSULTATION_DATE DESC, c.ID DESC";

    let rows: Vec<(
        i32, i32, String, String, String, String, String, String, Option<String>,
    )> = conn
        .query(&sql, (&status, &status, &like, &like, &like, &like))
        .map_err(AppError::from)?;

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

/// Agenda: cambia el estado de una consulta (PENDIENTE → COMPLETADA/CANCELADA)
/// y devuelve el item actualizado para refrescar la vista.
pub fn set_consultation_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<ConsultationListItem, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM CONSULTATIONS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) = current.ok_or_else(|| {
        AppError::NotFound(format!("Consulta {id} no encontrada"))
    })?;

    let allowed = match status {
        "COMPLETADA" | "CANCELADA" => current == "PENDIENTE",
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status} \
             (PENDIENTE→COMPLETADA, →CANCELADA)"
        )));
    }

    conn.execute(
        "UPDATE CONSULTATIONS
            SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&status, &id),
    )
    .map_err(AppError::from)?;

    let row: Option<(
        i32, i32, String, String, String, String, String, String, Option<String>,
    )> = conn
        .query_first(
            "SELECT c.ID, c.PATIENT_ID, p.NAME, sp.NAME, o.FULL_NAME,
                    LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
                    c.REASON, c.STATUS, u.FULL_NAME
             FROM CONSULTATIONS c
             JOIN PATIENTS p ON p.ID = c.PATIENT_ID
             JOIN SPECIES sp ON sp.ID = p.SPECIES_ID
             JOIN OWNERS o ON o.ID = p.OWNER_ID
             LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
             WHERE c.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(|r| ConsultationListItem {
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
    .ok_or_else(|| AppError::Internal("Consulta actualizada pero no recuperada".into()))
}

/// Ficha de una consulta (para la fórmula médica).
pub fn get_consultation(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<Consultation>, AppError> {
    let row: Option<(
        i32,
        i32,
        Option<i32>,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    )> = conn
        .query_first(
            "SELECT c.ID, c.PATIENT_ID, c.VETERINARIAN_ID,
                    LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
                    c.REASON, c.ANAMNESIS, c.PHYSICAL_EXAM, c.DIAGNOSIS,
                    c.TREATMENT_PLAN, c.STATUS, u.FULL_NAME
             FROM CONSULTATIONS c
             LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
             WHERE c.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    Ok(row.map(|r| Consultation {
        id: r.0,
        patient_id: r.1,
        veterinarian_id: r.2,
        consultation_date: r.3,
        reason: r.4,
        anamnesis: r.5,
        physical_exam: r.6,
        diagnosis: r.7,
        treatment_plan: r.8,
        status: r.9,
        veterinarian_name: r.10,
    }))
}

pub fn create_consultation(
    conn: &mut SimpleConnection,
    input: &CreateConsultationInput,
) -> Result<Consultation, AppError> {
    let id = next_id(conn, "GEN_CONSULTATIONS_ID")?;
    conn.execute(
        "INSERT INTO CONSULTATIONS
            (ID, PATIENT_ID, CONSULTATION_DATE, REASON, ANAMNESIS,
             PHYSICAL_EXAM, DIAGNOSIS, TREATMENT_PLAN, STATUS)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.consultation_date,
            &input.reason,
            &input.anamnesis,
            &input.physical_exam,
            &input.diagnosis,
            &input.treatment_plan,
            &input.status,
        ),
    )
    .map_err(AppError::from)?;

    let row: Option<(
        i32,
        i32,
        Option<i32>,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    )> = conn
        .query_first(
            "SELECT c.ID, c.PATIENT_ID, c.VETERINARIAN_ID,
                    LEFT(CAST(c.CONSULTATION_DATE AS VARCHAR(60)), 19),
                    c.REASON, c.ANAMNESIS, c.PHYSICAL_EXAM, c.DIAGNOSIS,
                    c.TREATMENT_PLAN, c.STATUS, u.FULL_NAME
             FROM CONSULTATIONS c
             LEFT JOIN USERS u ON u.ID = c.VETERINARIAN_ID
             WHERE c.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(|r| Consultation {
        id: r.0,
        patient_id: r.1,
        veterinarian_id: r.2,
        consultation_date: r.3,
        reason: r.4,
        anamnesis: r.5,
        physical_exam: r.6,
        diagnosis: r.7,
        treatment_plan: r.8,
        status: r.9,
        veterinarian_name: r.10,
    })
    .ok_or_else(|| AppError::Internal("Consulta creada pero no recuperada".into()))
}

// ============================ VACUNAS =======================================
// Las vacunas de un paciente viven en repositories/vaccines.rs::by_patient
// (reutilizado por el historial y el módulo de vacunación).

// ============================ MUESTRAS ======================================

fn list_samples(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<Vec<Sample>, AppError> {
    let rows: Vec<(
        i32,
        String,
        i32,
        i32,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = conn
        .query(
            "SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
                    LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                    s.STATUS, s.COLLECTED_BY, s.NOTES
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             WHERE s.PATIENT_ID = ?
             ORDER BY s.RECEIVED_AT DESC",
            (&patient_id,),
        )
        .map_err(AppError::from)?;

    let mut samples = Vec::new();
    for r in rows {
        let results = samples_repo::list_results(conn, r.0)?;
        samples.push(Sample {
            id: r.0,
            code: r.1,
            patient_id: r.2,
            sample_type_id: r.3,
            sample_type_name: r.4,
            received_at: r.5,
            status: r.6,
            collected_by: r.7,
            notes: r.8,
            results,
        });
    }
    Ok(samples)
}

pub fn create_sample(
    conn: &mut SimpleConnection,
    input: &CreateSampleInput,
) -> Result<Sample, AppError> {
    let id = next_id(conn, "GEN_SAMPLES_ID")?;
    conn.execute(
        "INSERT INTO SAMPLES
            (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS, COLLECTED_BY, NOTES)
         VALUES (?, ?, ?, ?, 'RECIBIDA', ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.sample_type_id,
            &input.received_at,
            &input.collected_by,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    let row: Option<(
        i32,
        String,
        i32,
        i32,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = conn
        .query_first(
            "SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
                    LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                    s.STATUS, s.COLLECTED_BY, s.NOTES
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             WHERE s.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(|r| Sample {
        id: r.0,
        code: r.1,
        patient_id: r.2,
        sample_type_id: r.3,
        sample_type_name: r.4,
        received_at: r.5,
        status: r.6,
        collected_by: r.7,
        notes: r.8,
        results: Vec::new(),
    })
    .ok_or_else(|| AppError::Internal("Muestra creada pero no recuperada".into()))
}

// ========================== RESULTADOS ======================================

// `list_results` vive en repositories/samples.rs (se reutiliza desde la
// mesa de trabajo del laboratorio y el generador de PDF).


/// Registra (o actualiza) un resultado validándolo contra los rangos de
/// referencia del paciente mediante SP_VALIDATE_ANALYTICAL_RESULT. Mueve la
/// muestra a EN_PROCESO (dispara SAMPLE_CHANGED → frontend); la finalización
/// manual llega con `set_sample_status("FINALIZADA")` desde el laboratorio.
pub fn register_lab_result(
    conn: &mut SimpleConnection,
    input: &RegisterResultInput,
) -> Result<LabResult, AppError> {
    // 1. Validación clínica en el servidor de base de datos.
    let validation: Option<(Option<i32>, String)> = conn
        .query_first(
            "SELECT RR_ID, STATUS FROM SP_VALIDATE_ANALYTICAL_RESULT(?, ?, ?)",
            (&input.sample_id, &input.analyte_id, &input.value),
        )
        .map_err(AppError::from)?;
    let (rr_id, status) = validation.unwrap_or((None, "SIN_RANGO".to_string()));

    // 2. Upsert del resultado (uno por analito y muestra).
    let existing: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM LAB_RESULTS WHERE SAMPLE_ID = ? AND ANALYTE_ID = ?",
            (&input.sample_id, &input.analyte_id),
        )
        .map_err(AppError::from)?;

    let id = if let Some((rid,)) = existing {
        conn.execute(
            "UPDATE LAB_RESULTS
                SET RESULT_VALUE = ?, REFERENCE_RANGE_ID = ?, STATUS = ?,
                    ANALYZED_AT = CURRENT_TIMESTAMP, UPDATED_AT = CURRENT_TIMESTAMP
              WHERE ID = ?",
            (&input.value, &rr_id, &status, &rid),
        )
        .map_err(AppError::from)?;
        rid
    } else {
        let nid = next_id(conn, "GEN_LAB_RESULTS_ID")?;
        conn.execute(
            "INSERT INTO LAB_RESULTS
                (ID, SAMPLE_ID, ANALYTE_ID, REFERENCE_RANGE_ID, RESULT_VALUE, STATUS, ANALYZED_AT)
             VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
            (&nid, &input.sample_id, &input.analyte_id, &rr_id, &input.value, &status),
        )
        .map_err(AppError::from)?;
        nid
    };

    // 3. La muestra pasa a EN_PROCESO (trazabilidad de estado).
    conn.execute(
        "UPDATE SAMPLES
            SET STATUS = 'EN_PROCESO', UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ? AND STATUS <> 'ANULADA'",
        (&input.sample_id,),
    )
    .map_err(AppError::from)?;

    // 4. Devuelve el resultado completo con su rango.
    let row: Option<(
        i32,
        i32,
        i32,
        String,
        Option<String>,
        f64,
        String,
        Option<f64>,
        Option<f64>,
        Option<String>,
    )> = conn
        .query_first(
            "SELECT r.ID, r.SAMPLE_ID, r.ANALYTE_ID, a.NAME, a.UNIT,
                    r.RESULT_VALUE, r.STATUS,
                    rr.MIN_VALUE, rr.MAX_VALUE,
                    LEFT(CAST(r.ANALYZED_AT AS VARCHAR(60)), 19)
             FROM LAB_RESULTS r
             JOIN ANALYTES a ON a.ID = r.ANALYTE_ID
             LEFT JOIN REFERENCE_RANGES rr ON rr.ID = r.REFERENCE_RANGE_ID
             WHERE r.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    row.map(samples_repo::map_lab_result)
        .ok_or_else(|| AppError::Internal("Resultado creado pero no recuperado".into()))
}
