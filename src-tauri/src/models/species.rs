use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Species {
    pub id: i32,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Breed {
    pub id: i32,
    pub species_id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SampleType {
    pub id: i32,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Analyte {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub unit: Option<String>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VaccineType {
    pub id: i32,
    pub code: String,
    pub name: String,
}
