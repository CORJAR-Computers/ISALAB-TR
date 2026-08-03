use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::owner::{CreateOwnerInput, Owner};
use crate::models::patient::{CreatePatientInput, Patient};
use crate::repositories::next_id;

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

type PatientRow = (
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

fn map_patient(r: PatientRow) -> Patient {
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
    let row: Option<(i32, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>)> =
        conn.query_first(
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
    let rows: Vec<(
        i32, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>,
    )> = match search {
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
