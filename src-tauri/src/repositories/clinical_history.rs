use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::clinical_history::ClinicalHistory;
use crate::models::consultation::{Consultation, ConsultationListItem, CreateConsultationInput};
use crate::models::sample::{CreateSampleInput, LabResult, RegisterResultInput, Sample};
use crate::repositories::{
    next_id, patient as patient_repo, samples as samples_repo, vaccines as vaccines_repo,
};

type ConsultationRow = (
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
);

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

type SampleRow = (
    i32,
    String,
    i32,
    i32,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i32>,    // analyzer_id
    Option<String>, // analyzer_name
);

type LabResultRow = (
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
);

pub fn get_clinical_history(
    conn: &mut SimpleConnection,
    patient_id: i32,
) -> Result<ClinicalHistory, AppError> {
    let patient = patient_repo::get(conn, patient_id)?
        .ok_or_else(|| AppError::NotFound(format!("Paciente {patient_id} no encontrado")))?;

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
    let rows: Vec<ConsultationRow> = conn
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

    let rows: Vec<ConsultationListItemRow> = conn
        .query(sql, (&status, &status, &like, &like, &like, &like))
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
    let (current,) =
        current.ok_or_else(|| AppError::NotFound(format!("Consulta {id} no encontrada")))?;

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

    let row: Option<ConsultationListItemRow> = conn
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
    let row: Option<ConsultationRow> = conn
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

    let row: Option<ConsultationRow> = conn
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

fn list_samples(conn: &mut SimpleConnection, patient_id: i32) -> Result<Vec<Sample>, AppError> {
    let rows: Vec<SampleRow> = conn
        .query(
            "SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
                    LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                    s.STATUS, s.COLLECTED_BY, s.NOTES,
                    s.ANALYZER_ID, az.NAME
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             LEFT JOIN ANALYZERS az ON az.ID = s.ANALYZER_ID
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
            analyzer_id: r.9,
            analyzer_name: r.10,
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
            (ID, PATIENT_ID, SAMPLE_TYPE_ID, RECEIVED_AT, STATUS, COLLECTED_BY, NOTES, ANALYZER_ID)
         VALUES (?, ?, ?, ?, 'RECIBIDA', ?, ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.sample_type_id,
            &input.received_at,
            &input.collected_by,
            &input.notes,
            &input.analyzer_id,
        ),
    )
    .map_err(AppError::from)?;

    let row: Option<SampleRow> = conn
        .query_first(
            "SELECT s.ID, s.CODE, s.PATIENT_ID, s.SAMPLE_TYPE_ID, st.NAME,
                    LEFT(CAST(s.RECEIVED_AT AS VARCHAR(60)), 19),
                    s.STATUS, s.COLLECTED_BY, s.NOTES,
                    s.ANALYZER_ID, az.NAME
             FROM SAMPLES s
             JOIN SAMPLE_TYPES st ON st.ID = s.SAMPLE_TYPE_ID
             LEFT JOIN ANALYZERS az ON az.ID = s.ANALYZER_ID
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
        analyzer_id: r.9,
        analyzer_name: r.10,
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
            (
                &nid,
                &input.sample_id,
                &input.analyte_id,
                &rr_id,
                &input.value,
                &status,
            ),
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
    let row: Option<LabResultRow> = conn
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::consultation::CreateConsultationInput;
    use crate::models::sample::{CreateSampleInput, RegisterResultInput};
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        let (conn, db_path) = setup_test_db();
        (conn, db_path)
    }

    #[test]
    fn test_create_and_get_consultation() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let input = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-04 10:00:00".to_string(),
            reason: "Vacunación anual".to_string(),
            anamnesis: Some("Paciente sana".to_string()),
            physical_exam: Some("Estado general bueno".to_string()),
            diagnosis: Some("Saludable".to_string()),
            treatment_plan: Some("Aplicar vacuna antirrábica".to_string()),
            status: "PENDIENTE".to_string(),
        };

        let consultation = create_consultation(&mut conn, &input).unwrap();
        assert_eq!(consultation.patient_id, patient_id);
        assert_eq!(consultation.reason, "Vacunación anual");
        assert_eq!(consultation.status, "PENDIENTE");
        assert!(consultation.id > 0);

        // Verificar que se puede recuperar
        let fetched = get_consultation(&mut conn, consultation.id).unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.reason, "Vacunación anual");
        assert_eq!(fetched.anamnesis, Some("Paciente sana".to_string()));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_consultation_not_found() {
        let (mut conn, db_path) = setup();
        let result = get_consultation(&mut conn, 999).unwrap();
        assert!(result.is_none());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_agenda_no_filters() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Crear 2 consultas
        for i in 0..2 {
            let input = CreateConsultationInput {
                patient_id,
                consultation_date: format!("2026-08-0{} 10:00:00", i + 1),
                reason: format!("Consulta {}", i + 1),
                anamnesis: None,
                physical_exam: None,
                diagnosis: None,
                treatment_plan: None,
                status: "PENDIENTE".to_string(),
            };
            create_consultation(&mut conn, &input).unwrap();
        }

        let agenda = list_agenda(&mut conn, None, None).unwrap();
        assert_eq!(agenda.len(), 2);
        // Ordenadas por fecha DESC
        assert_eq!(agenda[0].reason, "Consulta 2");
        assert_eq!(agenda[1].reason, "Consulta 1");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_agenda_filter_by_status() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        // Crear una PENDIENTE y una COMPLETADA
        let input1 = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-01 10:00:00".to_string(),
            reason: "Primera".to_string(),
            anamnesis: None,
            physical_exam: None,
            diagnosis: None,
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        let c1 = create_consultation(&mut conn, &input1).unwrap();

        let input2 = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-02 10:00:00".to_string(),
            reason: "Segunda".to_string(),
            anamnesis: None,
            physical_exam: None,
            diagnosis: None,
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        let c2 = create_consultation(&mut conn, &input2).unwrap();
        set_consultation_status(&mut conn, c2.id, "COMPLETADA").unwrap();

        // Filtrar solo PENDIENTE
        let pending = list_agenda(&mut conn, Some("PENDIENTE"), None).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, c1.id);

        // Filtrar solo COMPLETADA
        let completed = list_agenda(&mut conn, Some("COMPLETADA"), None).unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, c2.id);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_agenda_search() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let input = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-01 10:00:00".to_string(),
            reason: "Fiebre alta".to_string(),
            anamnesis: None,
            physical_exam: None,
            diagnosis: None,
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        create_consultation(&mut conn, &input).unwrap();

        // Buscar por nombre del paciente (Luna)
        let results = list_agenda(&mut conn, None, Some("Luna")).unwrap();
        assert_eq!(results.len(), 1);

        // Buscar por motivo (Fiebre)
        let results = list_agenda(&mut conn, None, Some("Fiebre")).unwrap();
        assert_eq!(results.len(), 1);

        // Buscar por nombre del propietario (Juan)
        let results = list_agenda(&mut conn, None, Some("Juan")).unwrap();
        assert_eq!(results.len(), 1);

        // Sin resultados
        let results = list_agenda(&mut conn, None, Some("NoExiste")).unwrap();
        assert_eq!(results.len(), 0);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_consultation_status_valid_transitions() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let input = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-01 10:00:00".to_string(),
            reason: "Control".to_string(),
            anamnesis: None,
            physical_exam: None,
            diagnosis: None,
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        let c = create_consultation(&mut conn, &input).unwrap();

        // PENDIENTE → COMPLETADA
        let updated = set_consultation_status(&mut conn, c.id, "COMPLETADA").unwrap();
        assert_eq!(updated.status, "COMPLETADA");

        // Resetear a PENDIENTE para probar CANCELADA
        conn.execute(
            "UPDATE CONSULTATIONS SET STATUS = 'PENDIENTE' WHERE ID = ?",
            (&c.id,),
        )
        .unwrap();

        // PENDIENTE → CANCELADA
        let updated = set_consultation_status(&mut conn, c.id, "CANCELADA").unwrap();
        assert_eq!(updated.status, "CANCELADA");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_consultation_status_invalid_transition() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);

        let input = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-01 10:00:00".to_string(),
            reason: "Control".to_string(),
            anamnesis: None,
            physical_exam: None,
            diagnosis: None,
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        let c = create_consultation(&mut conn, &input).unwrap();

        // Completar la consulta
        set_consultation_status(&mut conn, c.id, "COMPLETADA").unwrap();

        // COMPLETADA → CANCELADA no está permitido
        let result = set_consultation_status(&mut conn, c.id, "CANCELADA");
        assert!(result.is_err());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_consultation_status_not_found() {
        let (mut conn, db_path) = setup();
        let result = set_consultation_status(&mut conn, 999, "COMPLETADA");
        assert!(result.is_err());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_clinical_history_full() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);
        insert_test_analyte(&mut conn);

        // Crear una consulta
        let consultation_input = CreateConsultationInput {
            patient_id,
            consultation_date: "2026-08-01 10:00:00".to_string(),
            reason: "Consulta de control".to_string(),
            anamnesis: Some("Sin novedad".to_string()),
            physical_exam: None,
            diagnosis: Some("Sano".to_string()),
            treatment_plan: None,
            status: "PENDIENTE".to_string(),
        };
        create_consultation(&mut conn, &consultation_input).unwrap();

        // Crear una muestra con resultado
        let sample_input = CreateSampleInput {
            patient_id,
            sample_type_id: 1,
            received_at: "2026-08-01 11:00:00".to_string(),
            collected_by: Some("Dr. García".to_string()),
            notes: None,
            analyzer_id: None,
        };
        let sample = create_sample(&mut conn, &sample_input).unwrap();

        let result_input = RegisterResultInput {
            sample_id: sample.id,
            analyte_id: 1,
            value: 45.0,
        };
        register_lab_result(&mut conn, &result_input).unwrap();

        // Obtener historial completo
        let history = get_clinical_history(&mut conn, patient_id).unwrap();

        // Verificar paciente
        assert_eq!(history.patient.id, patient_id);
        assert_eq!(history.patient.name, "Luna");

        // Verificar propietario
        assert!(history.owner.is_some());
        let owner = history.owner.unwrap();
        assert_eq!(owner.full_name, "Juan Pérez");

        // Verificar consultas
        assert_eq!(history.consultations.len(), 1);
        assert_eq!(history.consultations[0].reason, "Consulta de control");

        // Verificar muestras
        assert_eq!(history.samples.len(), 1);
        assert_eq!(history.samples[0].sample_type_name, "Sangre total (EDTA)");
        assert_eq!(history.samples[0].results.len(), 1);
        assert_eq!(history.samples[0].results[0].analyte_name, "Hematocrito");
        assert_eq!(history.samples[0].results[0].value, 45.0);

        // Verificar vacunas (vacío en este test)
        assert_eq!(history.vaccines.len(), 0);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_clinical_history_patient_not_found() {
        let (mut conn, db_path) = setup();
        let result = get_clinical_history(&mut conn, 999);
        assert!(result.is_err());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_sample_from_clinical_history() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);

        let input = CreateSampleInput {
            patient_id,
            sample_type_id: 1,
            received_at: "2026-08-04 09:30:00".to_string(),
            collected_by: Some("Enf. López".to_string()),
            notes: Some("Muestra en ayunas".to_string()),
            analyzer_id: None,
        };

        let sample = create_sample(&mut conn, &input).unwrap();
        assert!(sample.id > 0);
        assert_eq!(sample.patient_id, patient_id);
        assert_eq!(sample.sample_type_name, "Sangre total (EDTA)");
        assert_eq!(sample.status, "RECIBIDA");
        assert_eq!(sample.collected_by, Some("Enf. López".to_string()));
        assert_eq!(sample.notes, Some("Muestra en ayunas".to_string()));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_register_lab_result_with_validation() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);
        insert_test_analyte(&mut conn);
        insert_test_reference_range(&mut conn);

        // Crear muestra
        let sample_input = CreateSampleInput {
            patient_id,
            sample_type_id: 1,
            received_at: "2026-08-04 10:00:00".to_string(),
            collected_by: None,
            notes: None,
            analyzer_id: None,
        };
        let sample = create_sample(&mut conn, &sample_input).unwrap();

        // Registrar resultado dentro del rango (37-55)
        let input = RegisterResultInput {
            sample_id: sample.id,
            analyte_id: 1,
            value: 45.0,
        };
        let result = register_lab_result(&mut conn, &input).unwrap();
        assert_eq!(result.value, 45.0);
        assert_eq!(result.sample_id, sample.id);
        assert_eq!(result.analyte_name, "Hematocrito");
        assert!(result.ref_min.is_some());
        assert!(result.ref_max.is_some());

        // Verificar que la muestra cambió a EN_PROCESO
        let sample = samples_repo::get(&mut conn, sample.id).unwrap().unwrap();
        assert_eq!(sample.status, "EN_PROCESO");

        // Actualizar el mismo resultado (upsert)
        let input2 = RegisterResultInput {
            sample_id: sample.id,
            analyte_id: 1,
            value: 48.0,
        };
        let result2 = register_lab_result(&mut conn, &input2).unwrap();
        assert_eq!(result2.value, 48.0);
        assert_eq!(result2.id, result.id); // Mismo ID

        cleanup_test_db(&db_path);
    }
}

/// Conteo de consultas agrupado por estado (para las pestañas de la agenda).
pub fn count_consultations_by_status(
    conn: &mut SimpleConnection,
) -> Result<Vec<crate::models::status_count::StatusCount>, AppError> {
    let rows: Vec<(String, i32)> = conn
        .query(
            "SELECT STATUS, COUNT(*) FROM CONSULTATIONS GROUP BY STATUS ORDER BY STATUS",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(status, count)| crate::models::status_count::StatusCount { status, count })
        .collect())
}
