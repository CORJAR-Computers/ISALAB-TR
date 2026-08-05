use serde::{Deserialize, Serialize};
use specta::Type;

/// Configuración de la clínica (tabla CLINIC_SETTINGS, claves planas).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicSettings {
    pub clinic_name: String,
    pub clinic_nit: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub logo_path: Option<String>,
    /// IVA por defecto (%).
    pub tax_rate: f64,
    pub currency: String,
    /// GRAPHIC (imagen) o DIGITAL (PKCS#12).
    pub signature_mode: String,
    pub vet_name: String,
    pub vet_license: Option<String>,
    pub groq_api_key: Option<String>,
    /// Ruta al archivo PKCS#12 (.p12/.pfx) para firma digital.
    pub pkcs12_path: Option<String>,
    /// Contraseña del certificado PKCS#12 (solo en memoria, nunca se persiste).
    pub pkcs12_password: Option<String>,
}

impl Default for ClinicSettings {
    fn default() -> Self {
        Self {
            clinic_name: "Mi Clínica Veterinaria".into(),
            clinic_nit: "900000000-0".into(),
            address: None,
            phone: None,
            city: Some("Bogotá D.C.".into()),
            logo_path: None,
            tax_rate: 19.0,
            currency: "COP".into(),
            signature_mode: "GRAPHIC".into(),
            vet_name: String::new(),
            vet_license: None,
            groq_api_key: None,
            pkcs12_path: None,
            pkcs12_password: None,
        }
    }
}
