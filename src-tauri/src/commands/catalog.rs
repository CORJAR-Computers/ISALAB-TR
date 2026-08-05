use rsfbclient::prelude::*;
use tauri::State;

use crate::auth::require_session;
use crate::error::AppError;
use crate::models::species::{Analyte, Breed, SampleType, Species, VaccineType};
use crate::state::AppState;

type SpeciesRow = (i32, String, String);
type BreedRow = (i32, i32, String);
type AnalyteRow = (i32, String, String, Option<String>, Option<String>);

#[tauri::command]
#[specta::specta]
pub fn list_species(state: State<'_, AppState>) -> Result<Vec<Species>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<SpeciesRow> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM SPECIES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Species {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_breeds(state: State<'_, AppState>, species_id: i32) -> Result<Vec<Breed>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<BreedRow> = pooled
        .conn()
        .query(
            "SELECT ID, SPECIES_ID, NAME FROM BREEDS
             WHERE SPECIES_ID = ? AND IS_ACTIVE = TRUE ORDER BY NAME",
            (&species_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Breed {
            id: r.0,
            species_id: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_sample_types(state: State<'_, AppState>) -> Result<Vec<SampleType>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<SpeciesRow> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM SAMPLE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| SampleType {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub fn list_analytes(state: State<'_, AppState>) -> Result<Vec<Analyte>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<AnalyteRow> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME, UNIT, METHOD FROM ANALYTES
             WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| Analyte {
            id: r.0,
            code: r.1,
            name: r.2,
            unit: r.3,
            method: r.4,
        })
        .collect())
}

/// Catálogo de vacunas del esquema (Rabia, Polivalente, FeLV…).
#[tauri::command]
#[specta::specta]
pub fn list_vaccine_types(state: State<'_, AppState>) -> Result<Vec<VaccineType>, AppError> {
    require_session(&state)?;
    let mut pooled = state.pool.acquire()?;
    let rows: Vec<SpeciesRow> = pooled
        .conn()
        .query(
            "SELECT ID, CODE, NAME FROM VACCINE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|r| VaccineType {
            id: r.0,
            code: r.1,
            name: r.2,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use rsfbclient::SimpleConnection;
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        setup_test_db()
    }

    #[test]
    fn test_list_species_from_seed() {
        let (mut conn, db_path) = setup();
        let rows: Vec<SpeciesRow> = conn
            .query(
                "SELECT ID, CODE, NAME FROM SPECIES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Seed inserts 9 species
        assert!(rows.len() >= 9);
        // Check Canino exists
        assert!(rows.iter().any(|r| r.1 == "CAN" && r.2 == "Canino"));
        // Check Felino exists
        assert!(rows.iter().any(|r| r.1 == "FEL" && r.2 == "Felino"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_breeds_for_canino() {
        let (mut conn, db_path) = setup();
        let rows: Vec<BreedRow> = conn
            .query(
                "SELECT ID, SPECIES_ID, NAME FROM BREEDS
                 WHERE SPECIES_ID = 1 AND IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Seed inserts multiple canino breeds
        assert!(rows.len() >= 10);
        // All should have species_id = 1
        assert!(rows.iter().all(|r| r.1 == 1));
        // Check specific breeds exist
        assert!(rows.iter().any(|r| r.2 == "Labrador Retriever"));
        assert!(rows.iter().any(|r| r.2 == "Beagle"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_breeds_for_felino() {
        let (mut conn, db_path) = setup();
        let rows: Vec<BreedRow> = conn
            .query(
                "SELECT ID, SPECIES_ID, NAME FROM BREEDS
                 WHERE SPECIES_ID = 2 AND IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        assert!(rows.len() >= 8);
        assert!(rows.iter().all(|r| r.1 == 2));
        assert!(rows.iter().any(|r| r.2 == "Siamés"));
        assert!(rows.iter().any(|r| r.2 == "Persa"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_breeds_empty_for_unknown_species() {
        let (mut conn, db_path) = setup();
        let rows: Vec<BreedRow> = conn
            .query(
                "SELECT ID, SPECIES_ID, NAME FROM BREEDS
                 WHERE SPECIES_ID = 999 AND IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        assert!(rows.is_empty());
        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_sample_types_from_seed() {
        let (mut conn, db_path) = setup();
        let rows: Vec<SpeciesRow> = conn
            .query(
                "SELECT ID, CODE, NAME FROM SAMPLE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Seed inserts 10 sample types
        assert!(rows.len() >= 10);
        assert!(rows
            .iter()
            .any(|r| r.1 == "SANGRE" && r.2 == "Sangre total (EDTA)"));
        assert!(rows.iter().any(|r| r.1 == "SUERO"));
        assert!(rows.iter().any(|r| r.1 == "ORINA"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_analytes_from_seed() {
        let (mut conn, db_path) = setup();
        let rows: Vec<AnalyteRow> = conn
            .query(
                "SELECT ID, CODE, NAME, UNIT, METHOD FROM ANALYTES
                 WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Seed inserts 18 analytes
        assert!(rows.len() >= 18);
        // Check specific analytes
        assert!(rows.iter().any(|r| r.1 == "HCT" && r.2 == "Hematocrito"));
        assert!(rows.iter().any(|r| r.1 == "GLU" && r.2 == "Glucosa"));
        assert!(rows.iter().any(|r| r.1 == "CREA" && r.2 == "Creatinina"));

        // Verify units exist
        assert!(rows
            .iter()
            .any(|r| r.1 == "HCT" && r.3.as_deref() == Some("%")));
        assert!(rows
            .iter()
            .any(|r| r.1 == "GLU" && r.3.as_deref() == Some("mg/dL")));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_vaccine_types_from_seed() {
        let (mut conn, db_path) = setup();
        let rows: Vec<SpeciesRow> = conn
            .query(
                "SELECT ID, CODE, NAME FROM VACCINE_TYPES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Seed inserts 8 vaccine types
        assert!(rows.len() >= 8);
        assert!(rows.iter().any(|r| r.1 == "RABIA" && r.2 == "Rabia"));
        assert!(rows.iter().any(|r| r.1 == "POLI_CANINA"));
        assert!(rows.iter().any(|r| r.1 == "LEUCEMIA_FELINA"));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_species_sorted_by_name() {
        let (mut conn, db_path) = setup();
        let rows: Vec<SpeciesRow> = conn
            .query(
                "SELECT ID, CODE, NAME FROM SPECIES WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Verify sorted alphabetically
        for window in rows.windows(2) {
            assert!(
                window[0].2 <= window[1].2,
                "Species not sorted: {} > {}",
                window[0].2,
                window[1].2
            );
        }

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_breeds_sorted_by_name() {
        let (mut conn, db_path) = setup();
        let rows: Vec<BreedRow> = conn
            .query(
                "SELECT ID, SPECIES_ID, NAME FROM BREEDS
                 WHERE SPECIES_ID = 1 AND IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // Verify sorted alphabetically
        for window in rows.windows(2) {
            assert!(
                window[0].2 <= window[1].2,
                "Breeds not sorted: {} > {}",
                window[0].2,
                window[1].2
            );
        }

        cleanup_test_db(&db_path);
    }

    #[test]
    fn test_analytes_have_required_fields() {
        let (mut conn, db_path) = setup();
        let rows: Vec<AnalyteRow> = conn
            .query(
                "SELECT ID, CODE, NAME, UNIT, METHOD FROM ANALYTES
                 WHERE IS_ACTIVE = TRUE ORDER BY NAME",
                (),
            )
            .unwrap();

        // All analytes should have code, name, and at least one of unit/method
        for row in &rows {
            assert!(!row.1.is_empty(), "Analyte {} has empty code", row.0);
            assert!(!row.2.is_empty(), "Analyte {} has empty name", row.0);
            // unit or method should exist
            assert!(
                row.3.is_some() || row.4.is_some(),
                "Analyte {} ({}) has neither unit nor method",
                row.0,
                row.2
            );
        }

        cleanup_test_db(&db_path);
    }
}
