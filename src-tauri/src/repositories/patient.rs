use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::owner::{CreateOwnerInput, Owner};
use crate::models::patient::{CreatePatientInput, Patient};
use crate::repositories::next_id;

pub(crate) type OwnerRow = (
    i32, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>,
);

/// Columnas unidas de un paciente (especie, raza, propietario, edad).
const PATIENT_SELECT: &str = "
    SELECT p.ID, p.OWNER_ID, p.SPECIES_ID, p.BREED_ID, p.NAME, p.SEX,
           CAST(p.BIRTH_DATE AS VARCHAR(10)), p.NEUTERED, p.COLOR, p.MICROCHIP,
           p.ACTIVE, p.NOTES,
           s.NAME,
           b.NAME,
           o.FULL_NAME,
           o.PHONE,
           COALESCE(CAST(DATEDIFF(MONTH, p.BIRTH_DATE, CURRENT_TIMESTAMP) AS INTEGER), 0)
    FROM PATIENTS p
    JOIN SPECIES s ON s.ID = p.SPECIES_ID
    LEFT JOIN BREEDS b ON b.ID = p.BREED_ID
    JOIN OWNERS o ON o.ID = p.OWNER_ID";

pub(crate) type PatientRow = (
    i32,          // id
    i32,          // owner_id
    i32,          // species_id
    Option<i32>,  // breed_id
    String,       // name
    String,       // sex
    Option<String>, // birth_date
    bool,         // neutered
    Option<String>, // color
    Option<String>, // microchip
    bool,         // active
    Option<String>, // notes
    String,       // species_name
    Option<String>, // breed_name
    String,       // owner_name
    Option<String>, // owner_phone
    i32,          // age_months
);

pub(crate) fn map_patient(r: PatientRow) -> Patient {
    Patient {
        id: r.0,
        owner_id: r.1,
        species_id: r.2,
        breed_id: r.3,
        name: r.4,
        sex: r.5,
        birth_date: r.6,
        neutered: r.7,
        color: r.8,
        microchip: r.9,
        active: r.10,
        notes: r.11,
        species_name: r.12,
        breed_name: r.13,
        owner_name: r.14,
        owner_phone: r.15,
        age_months: r.16,
    }
}

pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Patient>, AppError> {
    let row: Option<PatientRow> = conn
        .query_first(&format!("{PATIENT_SELECT} WHERE p.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_patient))
}

pub fn list(
    conn: &mut SimpleConnection,
    search: Option<&str>,
) -> Result<Vec<Patient>, AppError> {
    let rows: Vec<PatientRow> = match search {
        Some(term) if !term.trim().is_empty() => {
            let like = format!("%{}%", term.trim());
            conn.query(
                &format!(
                    "{PATIENT_SELECT}
                     WHERE UPPER(p.NAME) LIKE UPPER(?)
                        OR UPPER(o.FULL_NAME) LIKE UPPER(?)
                        OR p.MICROCHIP LIKE ?
                        OR o.DOCUMENT_NUMBER LIKE ?
                     ORDER BY p.NAME"
                ),
                (&like, &like, &like, &like),
            )
            .map_err(AppError::from)?
        }
        _ => conn
            .query(&format!("{PATIENT_SELECT} ORDER BY p.NAME"), ())
            .map_err(AppError::from)?,
    };
    Ok(rows.into_iter().map(map_patient).collect())
}

pub fn create(
    conn: &mut SimpleConnection,
    input: &CreatePatientInput,
) -> Result<Patient, AppError> {
    let owner_id = find_or_create_owner(conn, &input.owner)?;
    let id = next_id(conn, "GEN_PATIENTS_ID")?;

    conn.execute(
        "INSERT INTO PATIENTS
            (ID, OWNER_ID, SPECIES_ID, BREED_ID, NAME, SEX, BIRTH_DATE,
             NEUTERED, COLOR, MICROCHIP, NOTES)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &owner_id,
            &input.species_id,
            &input.breed_id,
            &input.name,
            &input.sex,
            &input.birth_date,
            &input.neutered,
            &input.color,
            &input.microchip,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    get(conn, id)?.ok_or_else(|| {
        AppError::Internal("Paciente creado pero no recuperado".into())
    })
}

/// Busca al propietario por documento o lo crea.
fn find_or_create_owner(
    conn: &mut SimpleConnection,
    input: &CreateOwnerInput,
) -> Result<i32, AppError> {
    let existing: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM OWNERS
             WHERE DOCUMENT_TYPE = ? AND DOCUMENT_NUMBER = ?",
            (&input.document_type, &input.document_number),
        )
        .map_err(AppError::from)?;

    if let Some((id,)) = existing {
        return Ok(id);
    }

    let id = next_id(conn, "GEN_OWNERS_ID")?;
    conn.execute(
        "INSERT INTO OWNERS
            (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE, EMAIL, ADDRESS, CITY)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &id,
            &input.document_type,
            &input.document_number,
            &input.full_name,
            &input.phone,
            &input.email,
            &input.address,
            &input.city,
        ),
    )
    .map_err(AppError::from)?;
    Ok(id)
}

pub fn get_owner(conn: &mut SimpleConnection, id: i32) -> Result<Option<Owner>, AppError> {
    let row: Option<OwnerRow> = conn
        .query_first(
            "SELECT ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE, EMAIL, ADDRESS, CITY
             FROM OWNERS WHERE ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;

    Ok(row.map(|r| Owner {
        id: r.0,
        document_type: r.1,
        document_number: r.2,
        full_name: r.3,
        phone: r.4,
        email: r.5,
        address: r.6,
        city: r.7,
    }))
}

/// Listado de propietarios (para facturación y búsquedas) con filtro por
/// nombre, documento o ciudad.
pub fn list_owners(
    conn: &mut SimpleConnection,
    search: Option<&str>,
) -> Result<Vec<Owner>, AppError> {
    let rows: Vec<OwnerRow> = match search {
        Some(term) if !term.trim().is_empty() => {
            let like = format!("%{}%", term.trim());
            conn.query(
                "SELECT ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE, EMAIL, ADDRESS, CITY
                 FROM OWNERS
                 WHERE UPPER(FULL_NAME) LIKE UPPER(?)
                    OR DOCUMENT_NUMBER LIKE ?
                    OR UPPER(COALESCE(CITY, '')) LIKE UPPER(?)
                 ORDER BY FULL_NAME",
                (&like, &like, &like),
            )
            .map_err(AppError::from)?
        }
        _ => conn
            .query(
                "SELECT ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME, PHONE, EMAIL, ADDRESS, CITY
                 FROM OWNERS ORDER BY FULL_NAME",
                (),
            )
            .map_err(AppError::from)?,
    };

    Ok(rows
        .into_iter()
        .map(|r| Owner {
            id: r.0,
            document_type: r.1,
            document_number: r.2,
            full_name: r.3,
            phone: r.4,
            email: r.5,
            address: r.6,
            city: r.7,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_patient_row() -> PatientRow {
        (
            1,                // id
            10,               // owner_id
            1,                // species_id (Canino)
            Some(5),          // breed_id (Beagle)
            "Luna".into(),    // name
            "F".into(),       // sex
            Some("2023-06-15".into()), // birth_date
            true,             // neutered
            Some("Marrón".into()), // color
            Some("CHIP-12345".into()), // microchip
            true,             // active
            Some("Paciente de prueba".into()), // notes
            "Canino".into(), // species_name
            Some("Beagle".into()), // breed_name
            "Juan Pérez".into(), // owner_name
            Some("+57 300 1234567".into()), // owner_phone
            24,               // age_months
        )
    }

    #[test]
    fn test_map_patient_fields() {
        let row = sample_patient_row();
        let patient = map_patient(row);

        assert_eq!(patient.id, 1);
        assert_eq!(patient.owner_id, 10);
        assert_eq!(patient.species_id, 1);
        assert_eq!(patient.breed_id, Some(5));
        assert_eq!(patient.name, "Luna");
        assert_eq!(patient.sex, "F");
        assert_eq!(patient.birth_date.as_deref(), Some("2023-06-15"));
        assert!(patient.neutered);
        assert_eq!(patient.color.as_deref(), Some("Marrón"));
        assert_eq!(patient.microchip.as_deref(), Some("CHIP-12345"));
        assert!(patient.active);
        assert_eq!(patient.notes.as_deref(), Some("Paciente de prueba"));
        assert_eq!(patient.species_name, "Canino");
        assert_eq!(patient.breed_name.as_deref(), Some("Beagle"));
        assert_eq!(patient.owner_name, "Juan Pérez");
        assert_eq!(patient.owner_phone.as_deref(), Some("+57 300 1234567"));
        assert_eq!(patient.age_months, 24);
    }

    #[test]
    fn test_map_patient_optional_fields_none() {
        let row: PatientRow = (
            2, 20, 2, None, "Michi".into(), "M".into(),
            None, false, None, None, true, None,
            "Felino".into(), None, "María López".into(), None, 6,
        );
        let patient = map_patient(row);

        assert_eq!(patient.id, 2);
        assert_eq!(patient.breed_id, None);
        assert_eq!(patient.birth_date, None);
        assert!(!patient.neutered);
        assert_eq!(patient.color, None);
        assert_eq!(patient.microchip, None);
        assert_eq!(patient.notes, None);
        assert_eq!(patient.breed_name, None);
        assert_eq!(patient.owner_phone, None);
    }

    #[test]
    fn test_owner_row_mapping() {
        let row: OwnerRow = (
            10, "CC".into(), "1234567890".into(), "Juan Pérez".into(),
            Some("+57 300 1234567".into()), Some("juan@test.com".into()),
            Some("Calle 123".into()), Some("Bogotá".into()),
        );
        let owner = Owner {
            id: row.0,
            document_type: row.1,
            document_number: row.2,
            full_name: row.3,
            phone: row.4,
            email: row.5,
            address: row.6,
            city: row.7,
        };

        assert_eq!(owner.id, 10);
        assert_eq!(owner.document_type, "CC");
        assert_eq!(owner.document_number, "1234567890");
        assert_eq!(owner.full_name, "Juan Pérez");
        assert_eq!(owner.phone.as_deref(), Some("+57 300 1234567"));
        assert_eq!(owner.email.as_deref(), Some("juan@test.com"));
        assert_eq!(owner.address.as_deref(), Some("Calle 123"));
        assert_eq!(owner.city.as_deref(), Some("Bogotá"));
    }
}

// ========================= INTEGRATION TESTS ================================
// Estos tests requieren Firebird 5 Embedded (fbclient.dll).
// Se ejecutan con una DB temporal que se crea y destruye por test.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (SimpleConnection, std::path::PathBuf) {
        test_helpers::setup_test_db()
    }

    #[test]
    fn test_get_patient_existing() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        let patient = get(&mut conn, 1).unwrap();
        assert!(patient.is_some());
        let p = patient.unwrap();
        assert_eq!(p.id, 1);
        assert_eq!(p.name, "Luna");
        assert_eq!(p.species_name, "Canino");
        assert_eq!(p.owner_name, "Juan Pérez");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_patient_not_found() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        let patient = get(&mut conn, 999).unwrap();
        assert!(patient.is_none());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_patients_no_search() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        let patients = list(&mut conn, None).unwrap();
        assert_eq!(patients.len(), 1);
        assert_eq!(patients[0].name, "Luna");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_patients_with_search() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        // Buscar por nombre
        let patients = list(&mut conn, Some("Luna")).unwrap();
        assert_eq!(patients.len(), 1);

        // Buscar por nombre del propietario
        let patients = list(&mut conn, Some("Juan")).unwrap();
        assert_eq!(patients.len(), 1);

        // Buscar algo que no existe
        let patients = list(&mut conn, Some("NoExiste")).unwrap();
        assert_eq!(patients.len(), 0);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_patient() {
        let (mut conn, db_path) = setup();
        // Preparar especie y raza
        conn.execute("INSERT OR UPDATE INTO SPECIES (ID, CODE, NAME) VALUES (1, 'CAN', 'Canino')", ()).ok();
        conn.execute("INSERT OR UPDATE INTO BREEDS (ID, SPECIES_ID, NAME) VALUES (1, 1, 'Beagle')", ()).ok();

        let input = CreatePatientInput {
            owner: CreateOwnerInput {
                document_type: "CC".into(),
                document_number: "9876543210".into(),
                full_name: "María López".into(),
                phone: Some("+57 310 9876543".into()),
                email: Some("maria@test.com".into()),
                address: None,
                city: Some("Medellín".into()),
            },
            name: "Max".into(),
            species_id: 1,
            breed_id: Some(1),
            sex: "M".into(),
            birth_date: Some("2024-01-20".into()),
            neutered: false,
            color: Some("Negro".into()),
            microchip: None,
            notes: Some("Puppy test".into()),
        };

        let patient = create(&mut conn, &input).unwrap();
        assert_eq!(patient.name, "Max");
        assert_eq!(patient.sex, "M");
        assert_eq!(patient.species_name, "Canino");
        assert_eq!(patient.owner_name, "María López");
        assert!(patient.id > 0);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_owner() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        let owner = get_owner(&mut conn, 1).unwrap();
        assert!(owner.is_some());
        let o = owner.unwrap();
        assert_eq!(o.id, 1);
        assert_eq!(o.full_name, "Juan Pérez");
        assert_eq!(o.document_type, "CC");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_owners() {
        let (mut conn, db_path) = setup();
        test_helpers::insert_test_patient(&mut conn);

        let owners = list_owners(&mut conn, None).unwrap();
        assert_eq!(owners.len(), 1);
        assert_eq!(owners[0].full_name, "Juan Pérez");

        // Buscar por nombre
        let owners = list_owners(&mut conn, Some("Juan")).unwrap();
        assert_eq!(owners.len(), 1);

        test_helpers::cleanup_test_db(&db_path);
    }
}
