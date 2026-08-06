use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryLogo {
    pub id: i32,
    pub name: String,
    pub logo_path: String,
    pub created_at: String,
}
