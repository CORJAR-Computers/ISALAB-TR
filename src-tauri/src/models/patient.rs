use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::owner::CreateOwnerInput;

/// Ficha de paciente con campos unidos (especie, raza, propietario, edad).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    pub id: i32,
    pub owner_id: i32,
    pub species_id: i32,
    pub breed_id: Option<i32>,
    pub name: String,
    /// M | F
    pub sex: String,
    /// YYYY-MM-DD
    pub birth_date: Option<String>,
    pub neutered: bool,
    pub color: Option<String>,
    pub microchip: Option<String>,
    pub active: bool,
    pub notes: Option<String>,
    // -- campos unidos (JOIN) --
    pub species_name: String,
    pub breed_name: Option<String>,
    pub owner_name: String,
    pub owner_phone: Option<String>,
    /// Calculada en SQL (DATEDIFF meses desde birth_date).
    pub age_months: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatePatientInput {
    pub owner: CreateOwnerInput,
    pub name: String,
    pub species_id: i32,
    pub breed_id: Option<i32>,
    pub sex: String,
    pub birth_date: Option<String>,
    pub neutered: bool,
    pub color: Option<String>,
    pub microchip: Option<String>,
    pub notes: Option<String>,
}
