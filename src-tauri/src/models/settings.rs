use serde::{Deserialize, Serialize};
use specta::Type;

/// Datos de marca públicos de la clínica (nombre comercial y tagline).
///
/// No exigen sesión: el login los necesita para mostrar la marca antes de
/// que el usuario se autentique.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicBranding {
    /// Nombre comercial configurado (o el valor por defecto).
    pub clinic_name: String,
    /// Texto de marca (login, sidebar, About); NULL = usar "Clínica Veterinaria".
    pub tagline: Option<String>,
}

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
    /// Servidor SMTP para notificaciones por email (NULL = sin configurar).
    pub smtp_host: Option<String>,
    /// Puerto SMTP (587 STARTTLS, 465 TLS implícito).
    pub smtp_port: Option<i32>,
    /// NONE | STARTTLS | TLS
    pub smtp_tls: Option<String>,
    pub smtp_username: Option<String>,
    /// Contraseña SMTP (cifrada en BD con DPAPI; nunca se devuelve en claro).
    pub smtp_password: Option<String>,
    /// Remitente (dirección "de" de los correos).
    pub smtp_from: Option<String>,
    /// Texto de marca (login, sidebar, About); NULL = usar "Clínica Veterinaria".
    pub tagline: Option<String>,
    /// Duración por defecto de la sesión de estética (minutos) cuando el
    /// servicio del catálogo no la define.
    pub grooming_default_duration: Option<i32>,
    /// Prefijo del ticket de turno de recepción (opcional).
    pub reception_ticket_prefix: Option<String>,
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
            smtp_host: None,
            smtp_port: None,
            smtp_tls: None,
            smtp_username: None,
            smtp_password: None,
            smtp_from: None,
            tagline: None,
            grooming_default_duration: None,
            reception_ticket_prefix: None,
        }
    }
}
