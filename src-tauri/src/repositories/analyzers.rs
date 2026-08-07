use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::analyzer::{
    Analyzer, CreateAnalyzerInput, ReferenceRange, ReferenceRangeInput, UpdateAnalyzerInput,
};
use crate::repositories::next_id;

// ============================ EQUIPOS =======================================

type AnalyzerRow = (
    i32,
    String,
    String,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    i32,
);

fn map_analyzer(r: AnalyzerRow) -> Analyzer {
    Analyzer {
        id: r.0,
        code: r.1,
        name: r.2,
        manufacturer: r.3,
        model: r.4,
        is_active: r.5,
        notes: r.6,
        range_count: r.7,
    }
}

const ANALYZER_SELECT: &str = "
    SELECT a.ID, a.CODE, a.NAME, a.MANUFACTURER, a.MODEL, a.IS_ACTIVE, a.NOTES,
           (SELECT COUNT(*) FROM REFERENCE_RANGES rr WHERE rr.ANALYZER_ID = a.ID)
    FROM ANALYZERS a";

/// Lista los equipos con su nº de rangos configurados (activos primero).
pub fn list(conn: &mut SimpleConnection) -> Result<Vec<Analyzer>, AppError> {
    let rows: Vec<AnalyzerRow> = conn
        .query(
            &format!("{ANALYZER_SELECT} ORDER BY a.IS_ACTIVE DESC, a.NAME"),
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_analyzer).collect())
}

fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Analyzer>, AppError> {
    let row: Option<AnalyzerRow> = conn
        .query_first(&format!("{ANALYZER_SELECT} WHERE a.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_analyzer))
}

fn code_exists(
    conn: &mut SimpleConnection,
    code: &str,
    exclude_id: Option<i32>,
) -> Result<bool, AppError> {
    let row: Option<(i32,)> = match exclude_id {
        Some(id) => conn
            .query_first(
                "SELECT 1 FROM ANALYZERS WHERE UPPER(CODE) = UPPER(?) AND ID <> ?",
                (code, &id),
            )
            .map_err(AppError::from)?,
        None => conn
            .query_first(
                "SELECT 1 FROM ANALYZERS WHERE UPPER(CODE) = UPPER(?)",
                (code,),
            )
            .map_err(AppError::from)?,
    };
    Ok(row.is_some())
}

/// Crea un equipo analizador (código único).
pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateAnalyzerInput,
) -> Result<Analyzer, AppError> {
    let code = input.code.trim().to_uppercase();
    if code.is_empty() || input.name.trim().is_empty() {
        return Err(AppError::Validation(
            "El código y el nombre del equipo son obligatorios".into(),
        ));
    }
    if code_exists(conn, &code, None)? {
        return Err(AppError::Validation(format!(
            "Ya existe un equipo con el código {code}"
        )));
    }

    let id = next_id(conn, "GEN_ANALYZERS_ID")?;
    conn.execute(
        "INSERT INTO ANALYZERS (ID, CODE, NAME, MANUFACTURER, MODEL, NOTES)
         VALUES (?, ?, ?, ?, ?, ?)",
        (
            &id,
            &code,
            input.name.trim(),
            &input.manufacturer,
            &input.model,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    get(conn, id)?.ok_or_else(|| AppError::Internal("Equipo creado pero no recuperado".into()))
}

/// Actualiza los datos de un equipo (mantiene IS_ACTIVE y rangos).
pub fn update(
    conn: &mut SimpleConnection,
    input: &UpdateAnalyzerInput,
) -> Result<Analyzer, AppError> {
    let code = input.code.trim().to_uppercase();
    if code.is_empty() || input.name.trim().is_empty() {
        return Err(AppError::Validation(
            "El código y el nombre del equipo son obligatorios".into(),
        ));
    }
    if code_exists(conn, &code, Some(input.id))? {
        return Err(AppError::Validation(format!(
            "Ya existe un equipo con el código {code}"
        )));
    }
    if input.id == 1 {
        return Err(AppError::Validation(
            "El perfil GENERAL (lectura manual) no se puede renombrar".into(),
        ));
    }

    conn.execute(
        "UPDATE ANALYZERS
            SET CODE = ?, NAME = ?, MANUFACTURER = ?, MODEL = ?, NOTES = ?
          WHERE ID = ?",
        (
            &code,
            input.name.trim(),
            &input.manufacturer,
            &input.model,
            &input.notes,
            &input.id,
        ),
    )
    .map_err(AppError::from)?;

    get(conn, input.id)?
        .ok_or_else(|| AppError::Internal("Equipo actualizado pero no recuperado".into()))
}

/// Activa/desactiva un equipo (desactivado no aparece en el selector).
pub fn set_active(
    conn: &mut SimpleConnection,
    id: i32,
    active: bool,
) -> Result<Analyzer, AppError> {
    if id == 1 && !active {
        return Err(AppError::Validation(
            "El perfil GENERAL no se puede desactivar".into(),
        ));
    }
    conn.execute(
        "UPDATE ANALYZERS SET IS_ACTIVE = ? WHERE ID = ?",
        (&active, &id),
    )
    .map_err(AppError::from)?;
    get(conn, id)?.ok_or_else(|| AppError::NotFound(format!("Equipo {id} no encontrado")))
}

/// Elimina un equipo solo si no tiene muestras asociadas (sus rangos se borran).
pub fn delete(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    if id == 1 {
        return Err(AppError::Validation(
            "El perfil GENERAL (lectura manual) no se puede eliminar".into(),
        ));
    }
    let used: Option<(i32,)> = conn
        .query_first("SELECT 1 FROM SAMPLES WHERE ANALYZER_ID = ?", (&id,))
        .map_err(AppError::from)?;
    if used.is_some() {
        return Err(AppError::Validation(
            "El equipo tiene muestras asociadas: desactívalo en lugar de eliminarlo".into(),
        ));
    }
    conn.execute("DELETE FROM REFERENCE_RANGES WHERE ANALYZER_ID = ?", (&id,))
        .map_err(AppError::from)?;
    conn.execute("DELETE FROM ANALYZERS WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

// ========================== RANGOS DE REFERENCIA ============================

type ReferenceRangeRow = (
    i32,            // id
    i32,            // analyzer_id
    String,         // analyzer_name
    i32,            // analyte_id
    String,         // analyte_name
    Option<String>, // unit
    i32,            // species_id
    String,         // species_name
    Option<String>, // sex
    i32,            // age_min_months
    i32,            // age_max_months
    f64,            // min_value
    f64,            // max_value
    Option<f64>,    // critical_min
    Option<f64>,    // critical_max
    Option<String>, // notes
);

fn map_range(r: ReferenceRangeRow) -> ReferenceRange {
    ReferenceRange {
        id: r.0,
        analyzer_id: r.1,
        analyzer_name: r.2,
        analyte_id: r.3,
        analyte_name: r.4,
        unit: r.5,
        species_id: r.6,
        species_name: r.7,
        sex: r.8,
        age_min_months: r.9,
        age_max_months: r.10,
        min_value: r.11,
        max_value: r.12,
        critical_min: r.13,
        critical_max: r.14,
        notes: r.15,
    }
}

const RANGE_SELECT: &str = "
    SELECT rr.ID, rr.ANALYZER_ID, az.NAME, rr.ANALYTE_ID, a.NAME, a.UNIT,
           rr.SPECIES_ID, sp.NAME, rr.SEX,
           rr.AGE_MIN_MONTHS, rr.AGE_MAX_MONTHS,
           rr.MIN_VALUE, rr.MAX_VALUE, rr.CRITICAL_MIN, rr.CRITICAL_MAX, rr.NOTES
    FROM REFERENCE_RANGES rr
    JOIN ANALYZERS az ON az.ID = rr.ANALYZER_ID
    JOIN ANALYTES a ON a.ID = rr.ANALYTE_ID
    JOIN SPECIES sp ON sp.ID = rr.SPECIES_ID";

/// Rangos de un equipo (o de todos si `analyzer_id` es None), ordenados por
/// analito, especie y edad.
pub fn list_ranges(
    conn: &mut SimpleConnection,
    analyzer_id: Option<i32>,
) -> Result<Vec<ReferenceRange>, AppError> {
    let rows: Vec<ReferenceRangeRow> = match analyzer_id {
        Some(id) => conn
            .query(
                &format!("{RANGE_SELECT} WHERE rr.ANALYZER_ID = ? ORDER BY a.NAME, sp.NAME, rr.AGE_MIN_MONTHS"),
                (&id,),
            )
            .map_err(AppError::from)?,
        None => conn
            .query(
                &format!("{RANGE_SELECT} ORDER BY az.NAME, a.NAME, sp.NAME, rr.AGE_MIN_MONTHS"),
                (),
            )
            .map_err(AppError::from)?,
    };
    Ok(rows.into_iter().map(map_range).collect())
}

fn get_range(conn: &mut SimpleConnection, id: i32) -> Result<Option<ReferenceRange>, AppError> {
    let row: Option<ReferenceRangeRow> = conn
        .query_first(&format!("{RANGE_SELECT} WHERE rr.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_range))
}

/// Valida los campos de un rango y que no exista un duplicado equivalente.
fn validate_range(
    conn: &mut SimpleConnection,
    input: &ReferenceRangeInput,
    exclude_id: Option<i32>,
) -> Result<(), AppError> {
    if input.analyte_id <= 0 || input.species_id <= 0 {
        return Err(AppError::Validation(
            "Analito y especie son obligatorios".into(),
        ));
    }
    if input.age_min_months < 0 || input.age_max_months < input.age_min_months {
        return Err(AppError::Validation(
            "La franja de edad no es válida (máx ≥ mín ≥ 0)".into(),
        ));
    }
    if input.min_value > input.max_value {
        return Err(AppError::Validation(
            "El valor mínimo no puede superar el máximo".into(),
        ));
    }
    if let Some(sex) = input.sex.as_deref() {
        if !matches!(sex, "M" | "F") {
            return Err(AppError::Validation("Sexo debe ser M, F o vacío".into()));
        }
    }
    let sex = input
        .sex
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let duplicate: Option<(i32,)> = match exclude_id {
        Some(id) => conn
            .query_first(
                "SELECT 1 FROM REFERENCE_RANGES
                  WHERE ANALYZER_ID = ? AND ANALYTE_ID = ? AND SPECIES_ID = ?
                    AND (SEX IS NULL AND ? IS NULL OR SEX = ?)
                    AND AGE_MIN_MONTHS = ? AND AGE_MAX_MONTHS = ?
                    AND ID <> ?",
                (
                    &input.analyzer_id,
                    &input.analyte_id,
                    &input.species_id,
                    &sex,
                    &sex,
                    &input.age_min_months,
                    &input.age_max_months,
                    &id,
                ),
            )
            .map_err(AppError::from)?,
        None => conn
            .query_first(
                "SELECT 1 FROM REFERENCE_RANGES
                  WHERE ANALYZER_ID = ? AND ANALYTE_ID = ? AND SPECIES_ID = ?
                    AND (SEX IS NULL AND ? IS NULL OR SEX = ?)
                    AND AGE_MIN_MONTHS = ? AND AGE_MAX_MONTHS = ?",
                (
                    &input.analyzer_id,
                    &input.analyte_id,
                    &input.species_id,
                    &sex,
                    &sex,
                    &input.age_min_months,
                    &input.age_max_months,
                ),
            )
            .map_err(AppError::from)?,
    };
    if duplicate.is_some() {
        return Err(AppError::Validation(
            "Ya existe un rango equivalente para este equipo, analito, especie, sexo y edad".into(),
        ));
    }
    Ok(())
}

/// Crea un rango de referencia para un equipo.
pub fn create_range(
    conn: &mut SimpleConnection,
    input: &ReferenceRangeInput,
) -> Result<ReferenceRange, AppError> {
    validate_range(conn, input, None)?;

    let id = next_id(conn, "GEN_REFERENCE_RANGES_ID")?;
    let sex = input
        .sex
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    conn.execute(
        "INSERT INTO REFERENCE_RANGES
            (ID, ANALYZER_ID, ANALYTE_ID, SPECIES_ID, SEX,
             AGE_MIN_MONTHS, AGE_MAX_MONTHS, MIN_VALUE, MAX_VALUE,
             CRITICAL_MIN, CRITICAL_MAX, NOTES)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &input.analyzer_id,
            &input.analyte_id,
            &input.species_id,
            &sex,
            &input.age_min_months,
            &input.age_max_months,
            &input.min_value,
            &input.max_value,
            &input.critical_min,
            &input.critical_max,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    get_range(conn, id)?.ok_or_else(|| AppError::Internal("Rango creado pero no recuperado".into()))
}

/// Actualiza un rango existente.
pub fn update_range(
    conn: &mut SimpleConnection,
    id: i32,
    input: &ReferenceRangeInput,
) -> Result<ReferenceRange, AppError> {
    validate_range(conn, input, Some(id))?;

    let sex = input
        .sex
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    conn.execute(
        "UPDATE REFERENCE_RANGES
            SET ANALYZER_ID = ?, ANALYTE_ID = ?, SPECIES_ID = ?, SEX = ?,
                AGE_MIN_MONTHS = ?, AGE_MAX_MONTHS = ?,
                MIN_VALUE = ?, MAX_VALUE = ?, CRITICAL_MIN = ?, CRITICAL_MAX = ?, NOTES = ?
          WHERE ID = ?",
        (
            &input.analyzer_id,
            &input.analyte_id,
            &input.species_id,
            &sex,
            &input.age_min_months,
            &input.age_max_months,
            &input.min_value,
            &input.max_value,
            &input.critical_min,
            &input.critical_max,
            &input.notes,
            &id,
        ),
    )
    .map_err(AppError::from)?;

    get_range(conn, id)?.ok_or_else(|| AppError::NotFound(format!("Rango {id} no encontrado")))
}

/// Elimina un rango de referencia.
pub fn delete_range(conn: &mut SimpleConnection, id: i32) -> Result<(), AppError> {
    conn.execute("DELETE FROM REFERENCE_RANGES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::sample::{CreateSampleInput, RegisterResultInput};
    use crate::repositories::clinical_history;
    use crate::test_helpers::*;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_analyzers_seed_present() {
        let (mut conn, db_path) = setup();
        let analyzers = list(&mut conn).unwrap();

        assert!(analyzers.iter().any(|a| a.id == 1 && a.code == "GENERAL"));
        let mindray = analyzers
            .iter()
            .find(|a| a.code == "MINDRAY-B2800")
            .expect("MINDRAY B2800 sembrado");
        assert_eq!(mindray.manufacturer.as_deref(), Some("MINDRAY"));
        assert_eq!(mindray.model.as_deref(), Some("B2800"));
        assert!(mindray.is_active);

        // Los 47 rangos de la semilla pertenecen al perfil GENERAL.
        let general = analyzers.iter().find(|a| a.id == 1).unwrap();
        assert!(general.range_count >= 47);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_and_update_analyzer() {
        let (mut conn, db_path) = setup();
        let created = create(
            &mut conn,
            &CreateAnalyzerInput {
                code: "idexx-vettest".into(),
                name: "IDEXX VetTest".into(),
                manufacturer: Some("IDEXX".into()),
                model: Some("VetTest 8008".into()),
                notes: None,
            },
        )
        .unwrap();
        assert!(created.id > 2);
        assert_eq!(created.code, "IDEXx-VETTEST".to_uppercase());

        let updated = update(
            &mut conn,
            &UpdateAnalyzerInput {
                id: created.id,
                code: "idexx-vettest".into(),
                name: "IDEXX VetTest 2".into(),
                manufacturer: Some("IDEXX".into()),
                model: Some("VetTest 8008".into()),
                notes: Some("Química seca".into()),
            },
        )
        .unwrap();
        assert_eq!(updated.name, "IDEXX VetTest 2");
        assert_eq!(updated.notes.as_deref(), Some("Química seca"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_analyzer_duplicate_code_rejected() {
        let (mut conn, db_path) = setup();
        let err = create(
            &mut conn,
            &CreateAnalyzerInput {
                code: "mindray-b2800".into(),
                name: "Otro Mindray".into(),
                manufacturer: None,
                model: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("código MINDRAY-B2800"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_general_profile_cannot_be_deactivated_or_deleted() {
        let (mut conn, db_path) = setup();
        assert!(set_active(&mut conn, 1, false).is_err());
        assert!(delete(&mut conn, 1).is_err());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_delete_analyzer_blocked_when_used_by_samples() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);

        clinical_history::create_sample(
            &mut conn,
            &CreateSampleInput {
                patient_id,
                sample_type_id: 1,
                received_at: "2026-08-06 09:00:00".into(),
                collected_by: None,
                notes: None,
                analyzer_id: Some(2),
            },
        )
        .unwrap();

        let err = delete(&mut conn, 2).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("desactívalo"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_duplicate_range_rejected() {
        let (mut conn, db_path) = setup();
        insert_test_analyte(&mut conn);

        // La semilla ya tiene HCT canino adulto 37-55 para el perfil GENERAL.
        let err = create_range(
            &mut conn,
            &ReferenceRangeInput {
                analyzer_id: 1,
                analyte_id: 1,
                species_id: 1,
                sex: None,
                age_min_months: 12,
                age_max_months: 2400,
                min_value: 38.0,
                max_value: 56.0,
                critical_min: None,
                critical_max: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("Ya existe un rango equivalente"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_range_validates_values() {
        let (mut conn, db_path) = setup();
        insert_test_analyte(&mut conn);

        // min > max
        let err = create_range(
            &mut conn,
            &ReferenceRangeInput {
                analyzer_id: 2,
                analyte_id: 1,
                species_id: 1,
                sex: None,
                age_min_months: 0,
                age_max_months: 12,
                min_value: 60.0,
                max_value: 40.0,
                critical_min: None,
                critical_max: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("mínimo no puede superar"));

        // edad invertida
        let err = create_range(
            &mut conn,
            &ReferenceRangeInput {
                analyzer_id: 2,
                analyte_id: 1,
                species_id: 1,
                sex: None,
                age_min_months: 24,
                age_max_months: 12,
                min_value: 10.0,
                max_value: 20.0,
                critical_min: None,
                critical_max: None,
                notes: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("franja de edad"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_range_crud_for_mindray() {
        let (mut conn, db_path) = setup();
        insert_test_analyte(&mut conn);

        let r = create_range(
            &mut conn,
            &ReferenceRangeInput {
                analyzer_id: 2,
                analyte_id: 1,
                species_id: 1,
                sex: Some("F".into()),
                age_min_months: 0,
                age_max_months: 2400,
                min_value: 38.0,
                max_value: 58.0,
                critical_min: Some(20.0),
                critical_max: Some(70.0),
                notes: Some("Inserto MINDRAY".into()),
            },
        )
        .unwrap();
        assert_eq!(r.analyzer_name, "MINDRAY B2800");
        assert_eq!(r.analyte_name, "Hematocrito");
        assert_eq!(r.species_name, "Canino");
        assert_eq!(r.sex.as_deref(), Some("F"));
        assert_eq!(r.critical_min, Some(20.0));

        // listado filtrado por equipo
        let ranges = list_ranges(&mut conn, Some(2)).unwrap();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].id, r.id);

        // update
        let updated = update_range(
            &mut conn,
            r.id,
            &ReferenceRangeInput {
                analyzer_id: 2,
                analyte_id: 1,
                species_id: 1,
                sex: None,
                age_min_months: 0,
                age_max_months: 2400,
                min_value: 38.0,
                max_value: 60.0,
                critical_min: None,
                critical_max: None,
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(updated.max_value, 60.0);
        assert_eq!(updated.sex, None);

        // delete
        delete_range(&mut conn, r.id).unwrap();
        assert!(list_ranges(&mut conn, Some(2)).unwrap().is_empty());

        cleanup_test_db(&db_path);
    }

    /// La validación clínica usa el rango del equipo de la muestra y, si el
    /// equipo no tiene rango propio, respalda con el perfil GENERAL.
    #[test]
    fn test_validation_uses_analyzer_range_with_general_fallback() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn); // Canino, ~38 meses
        insert_test_sample_type(&mut conn);
        insert_test_analyte(&mut conn);
        insert_test_reference_range(&mut conn); // HCT canino 37-55 GENERAL

        // Rango específico del MINDRAY B2800 (equipo 2): HCT canino 40-60.
        create_range(
            &mut conn,
            &ReferenceRangeInput {
                analyzer_id: 2,
                analyte_id: 1,
                species_id: 1,
                sex: None,
                age_min_months: 12,
                age_max_months: 2400,
                min_value: 40.0,
                max_value: 60.0,
                critical_min: None,
                critical_max: None,
                notes: None,
            },
        )
        .unwrap();

        // Muestra procesada en el MINDRAY B2800.
        let sample = clinical_history::create_sample(
            &mut conn,
            &CreateSampleInput {
                patient_id,
                sample_type_id: 1,
                received_at: "2026-08-06 09:00:00".into(),
                collected_by: None,
                notes: None,
                analyzer_id: Some(2),
            },
        )
        .unwrap();
        assert_eq!(sample.analyzer_id, Some(2));
        assert_eq!(sample.analyzer_name.as_deref(), Some("MINDRAY B2800"));

        // 58% está fuera del GENERAL (37-55) pero dentro del MINDRAY (40-60).
        let res = clinical_history::register_lab_result(
            &mut conn,
            &RegisterResultInput {
                sample_id: sample.id,
                analyte_id: 1,
                value: 58.0,
            },
        )
        .unwrap();
        assert_eq!(res.status, "NORMAL");
        assert_eq!(res.ref_min, Some(40.0));
        assert_eq!(res.ref_max, Some(60.0));

        // 62% ya está fuera de ambos → ALTO.
        let res2 = clinical_history::register_lab_result(
            &mut conn,
            &RegisterResultInput {
                sample_id: sample.id,
                analyte_id: 1,
                value: 62.0,
            },
        )
        .unwrap();
        assert_eq!(res2.status, "ALTO");

        cleanup_test_db(&db_path);
    }

    /// Sin equipo (lectura manual) la validación cae al perfil GENERAL.
    #[test]
    fn test_validation_falls_back_to_general_without_analyzer() {
        let (mut conn, db_path) = setup();
        let patient_id = insert_test_patient(&mut conn);
        insert_test_sample_type(&mut conn);
        insert_test_analyte(&mut conn);
        insert_test_reference_range(&mut conn);

        let sample = clinical_history::create_sample(
            &mut conn,
            &CreateSampleInput {
                patient_id,
                sample_type_id: 1,
                received_at: "2026-08-06 09:00:00".into(),
                collected_by: None,
                notes: None,
                analyzer_id: None,
            },
        )
        .unwrap();
        assert_eq!(sample.analyzer_id, None);
        assert_eq!(sample.analyzer_name, None);

        let res = clinical_history::register_lab_result(
            &mut conn,
            &RegisterResultInput {
                sample_id: sample.id,
                analyte_id: 1,
                value: 58.0,
            },
        )
        .unwrap();
        assert_eq!(res.status, "ALTO"); // fuera del GENERAL 37-55
        assert_eq!(res.ref_max, Some(55.0));

        cleanup_test_db(&db_path);
    }
}
