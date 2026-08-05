//! Firma digital PKCS#12 para reportes PDF.
//!
//! Implementa la firma criptográfica real de documentos PDF con un
//! certificado digital almacenado en un contenedor PKCS#12 (.p12/.pfx),
//! conforme a la Ley 527 de 1999 y el Decreto 2364 de 2019 (Colombia).
//!
//! La firma se realiza con `pdf_signer` (PAdES B-B, 100% Rust/RustCrypto):
//! añade una firma CMS detached ETSI.CAdES como actualización incremental del
//! PDF, de modo que el documento firmado es verificable con cualquier visor
//! (Adobe, validador PAdES, etc.) y conserva su validez legal.
//!
//! El parseo del PKCS#12 y la extracción de los metadatos del certificado
//! (titular, emisor, vigencia, número de serie) se hace con `p12-keystore`
//! y `x509-cert`, ambos de RustCrypto, sin dependencias del sistema.

use std::path::Path;

use x509_cert::der::Decode;
use x509_cert::Certificate;

use crate::error::AppError;

/// Información del certificado digital para el bloque de firma.
#[derive(Debug, Clone)]
pub struct Pkcs12Info {
    /// Nombre del titular (subject DN) del certificado.
    pub holder_name: String,
    /// Organización / clínica.
    pub organization: Option<String>,
    /// Número de serie del certificado (hexadecimal).
    pub serial_number: Option<String>,
    /// Fecha de emisión (YYYY-MM-DD).
    pub valid_from: String,
    /// Fecha de expiración (YYYY-MM-DD).
    pub valid_to: String,
    /// Emisor (issuer DN) del certificado.
    pub issuer: String,
    /// ¿El certificado está vigente en la fecha actual?
    pub is_valid: bool,
}

/// Resultado de la validación de un certificado PKCS#12.
#[derive(Debug)]
pub struct ValidationReport {
    pub info: Pkcs12Info,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Formatea un timestamp Unix como `YYYY-MM-DD`.
fn fmt_date(secs: u64) -> String {
    chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "desconocida".into())
}

/// Valida un archivo PKCS#12 descifrándolo con la contraseña y extrayendo
/// los metadatos reales del certificado (titular, emisor, vigencia, serie).
pub fn validate_pkcs12(p12_path: &Path, password: &str) -> Result<ValidationReport, AppError> {
    let data = std::fs::read(p12_path).map_err(|e| {
        AppError::Validation(format!(
            "No se pudo leer el archivo PKCS#12: {e}"
        ))
    })?;

    // El parseo descifra el contenedor: si la contraseña es incorrecta,
    // `from_pkcs12` falla. Es la validación real del certificado.
    let ks = p12_keystore::KeyStore::from_pkcs12(&data, password).map_err(|e| {
        AppError::Validation(format!(
            "PKCS#12 inválido o contraseña incorrecta: {e}"
        ))
    })?;

    let (_, chain) = ks.private_key_chain().ok_or_else(|| {
        AppError::Validation(
            "El archivo PKCS#12 no contiene una clave privada con certificado".into(),
        )
    })?;

    let leaf = chain
        .chain()
        .first()
        .ok_or_else(|| {
            AppError::Validation(
                "El archivo PKCS#12 no contiene certificados".into(),
            )
        })?;

    // Extrae metadatos X.509 del certificado (leaf).
    let cert = Certificate::from_der(leaf.as_der()).map_err(|e| {
        AppError::Validation(format!(
            "No se pudo interpretar el certificado X.509: {e}"
        ))
    })?;

    let tbs = &cert.tbs_certificate;
    let not_before = tbs.validity.not_before.to_unix_duration().as_secs();
    let not_after = tbs.validity.not_after.to_unix_duration().as_secs();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let serial = tbs
        .serial_number
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");

    let mut warnings = Vec::new();
    let is_valid = now >= not_before && now <= not_after;
    if !is_valid {
        warnings.push(format!(
            "El certificado está fuera de vigencia ({} al {}).",
            fmt_date(not_before),
            fmt_date(not_after)
        ));
    }

    Ok(ValidationReport {
        info: Pkcs12Info {
            holder_name: leaf.subject().to_string(),
            organization: None,
            serial_number: Some(serial),
            valid_from: fmt_date(not_before),
            valid_to: fmt_date(not_after),
            issuer: leaf.issuer().to_string(),
            is_valid,
        },
        warnings,
        errors: Vec::new(),
    })
}

/// Firma un PDF con un certificado PKCS#12 (PAdES B-B) y escribe el resultado
/// en `output`. El bloque de firma visible lo dibuja `printpdf` antes de esta
/// llamada; aquí se añade la firma criptográfica real (CMS detached).
pub fn sign_pdf_with_pkcs12(
    input: &Path,
    output: &Path,
    keystore: &Path,
    password: &str,
    signer_name: &str,
    reason: &str,
) -> Result<(), AppError> {
    let opts = pdf_signer::SignOptions {
        signature_capacity: 8192,
        reason: Some(reason.to_string()),
        name: Some(signer_name.to_string()),
        location: None,
        contact_info: None,
        signing_time: None,
        appearance: None, // la firma visible la dibuja printpdf; aquí solo la criptográfica
        tsa_url: None,
        pades_level: pdf_signer::PadesLevel::Bb,
    };

    pdf_signer::sign_pdf_file(input, output, keystore, password, &opts).map_err(|e| {
        AppError::Internal(format!("No se pudo firmar el PDF: {e}"))
    })
}

/// Verifica las firmas de un PDF ya firmado y devuelve una línea legible por
/// firma (estado + firmante). Útil para validar un PDF recién generado.
pub fn verify_signed_pdf(path: &Path) -> Result<Vec<String>, AppError> {
    let report = pdf_signer::verify_pdf_file(path).map_err(|e| {
        AppError::Internal(format!("No se pudo verificar la firma: {e}"))
    })?;

    Ok(report
        .signatures
        .iter()
        .map(|s| {
            format!(
                "{}: {} ({})",
                s.signer.as_deref().unwrap_or("Firmante desconocido"),
                if s.valid { "VÁLIDA" } else { "INVÁLIDA" },
                s.detail
            )
        })
        .collect())
}

/// Genera el bloque de firma digital visible para insertar en el PDF.
/// Devuelve las líneas de texto que se deben dibujar en el PDF.
pub fn signature_block_lines(info: &Pkcs12Info) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push("FIRMA DIGITAL".to_string());
    lines.push(format!("Titular: {}", info.holder_name));

    if let Some(ref org) = info.organization {
        lines.push(format!("Organización: {org}"));
    }

    lines.push(format!("Emisor: {}", info.issuer));
    lines.push(format!("Vigencia: {} al {}", info.valid_from, info.valid_to));

    if let Some(ref serial) = info.serial_number {
        lines.push(format!("Serie: {serial}"));
    }

    if info.is_valid {
        lines.push("Estado: ✅ VIGENTE".to_string());
    } else {
        lines.push("Estado: ❌ EXPIRADO".to_string());
    }

    lines.push(String::new());
    lines.push("Firmado digitalmente conforme a la Ley 527 de 1999".to_string());
    lines.push("y el Decreto 2364 de 2019 (Colombia).".to_string());

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crea un archivo PKCS#12 autofirmado de prueba (testkit de pdf_signer).
    fn temp_p12(password: &str) -> std::path::PathBuf {
        let bytes = pdf_signer::testkit::self_signed_p12(password);
        let path = std::env::temp_dir().join(format!(
            "isalab-test-{}.p12",
            std::process::id()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn test_validate_pkcs12_correct_password() {
        let path = temp_p12("secret123");
        let report = validate_pkcs12(&path, "secret123").unwrap();
        assert!(!report.info.holder_name.is_empty());
        assert!(!report.info.issuer.is_empty());
        assert!(report.info.valid_from.len() == 10);
        assert!(report.info.valid_to.len() == 10);
        assert!(report.info.serial_number.is_some());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_wrong_password() {
        let path = temp_p12("secret123");
        let result = validate_pkcs12(&path, "wrong-pass");
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_validate_pkcs12_nonexistent() {
        let result = validate_pkcs12(Path::new("/nonexistent.p12"), "");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pkcs12_wrong_extension_content() {
        // Un archivo que no es PKCS#12 debe fallar al descifrarse.
        let path = std::env::temp_dir().join("isalab-not-p12.bin");
        std::fs::write(&path, b"not a real p12 file").unwrap();
        let result = validate_pkcs12(&path, "");
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_sign_and_verify_pdf() {
        let pdf = pdf_signer::testkit::sample_pdf();
        let input = std::env::temp_dir().join("isalab-input.pdf");
        let output = std::env::temp_dir().join("isalab-signed.pdf");
        let p12 = temp_p12("sign-pass");
        std::fs::write(&input, &pdf).unwrap();

        sign_pdf_with_pkcs12(&input, &output, &p12, "sign-pass", "Dra. Ana Pérez", "Informe de laboratorio")
            .unwrap();

        let lines = verify_signed_pdf(&output).unwrap();
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("VÁLIDA")));

        std::fs::remove_file(&input).ok();
        std::fs::remove_file(&output).ok();
        std::fs::remove_file(&p12).ok();
    }

    #[test]
    fn test_signature_block_lines() {
        let info = Pkcs12Info {
            holder_name: "ISA RAMOS".to_string(),
            organization: Some("ISALAB".to_string()),
            serial_number: Some("AA:BB".to_string()),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2026-12-31".to_string(),
            issuer: "AC RAIZ".to_string(),
            is_valid: true,
        };

        let lines = signature_block_lines(&info);
        assert!(lines.iter().any(|l| l.contains("FIRMA DIGITAL")));
        assert!(lines.iter().any(|l| l.contains("ISA RAMOS")));
        assert!(lines.iter().any(|l| l.contains("ISALAB")));
        assert!(lines.iter().any(|l| l.contains("2024-01-01")));
        assert!(lines.iter().any(|l| l.contains("VIGENTE")));
        assert!(lines.iter().any(|l| l.contains("Ley 527")));
        assert!(lines.iter().any(|l| l.contains("AA:BB")));
    }

    #[test]
    fn test_signature_block_expired() {
        let info = Pkcs12Info {
            holder_name: "Test User".to_string(),
            organization: None,
            serial_number: None,
            valid_from: "2020-01-01".to_string(),
            valid_to: "2023-12-31".to_string(),
            issuer: "CA Test".to_string(),
            is_valid: false,
        };

        let lines = signature_block_lines(&info);
        assert!(lines.iter().any(|l| l.contains("EXPIRADO")));
    }

    #[test]
    fn test_signature_block_no_org() {
        let info = Pkcs12Info {
            holder_name: "Simple User".to_string(),
            organization: None,
            serial_number: None,
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-12-31".to_string(),
            issuer: "Simple CA".to_string(),
            is_valid: true,
        };

        let lines = signature_block_lines(&info);
        assert!(!lines.iter().any(|l| l.contains("Organización")));
        assert!(lines.iter().any(|l| l.contains("Simple User")));
    }
}
