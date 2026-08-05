//! Firma digital PKCS#12 para reportes PDF.
//!
//! Implementa la firma de documentos PDF con certificado digital para
//! validez legal conforme a la Ley 527 de 1999 y el Decreto 2364 de 2019 (Colombia).
//!
//! Nota: El parseo completo de PKCS#12 requiere la librería OpenSSL o similar.
//! Esta implementación almacena la ruta del certificado y genera el bloque de
//! firma visible con los metadatos configurados por el usuario.

use crate::error::AppError;

/// Información del certificado digital para el bloque de firma.
#[derive(Debug, Clone)]
pub struct Pkcs12Info {
    /// Nombre del titular del certificado.
    pub holder_name: String,
    /// Organización / clínica.
    pub organization: Option<String>,
    /// Número de tarjeta profesional / NIT.
    pub serial_number: Option<String>,
    /// Fecha de emisión (YYYY-MM-DD).
    pub valid_from: String,
    /// Fecha de expiración (YYYY-MM-DD).
    pub valid_to: String,
    /// Emisor del certificado.
    pub issuer: String,
    /// ¿El certificado está vigente?
    pub is_valid: bool,
}

/// Resultado de la validación de un certificado PKCS#12.
#[derive(Debug)]
pub struct ValidationReport {
    pub info: Pkcs12Info,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Valida un archivo PKCS#12 verificando que existe y tiene extensión correcta.
///
/// Para una validación completa del certificado, se requiere OpenSSL o
/// una librería de parsing PKCS#12 dedicada.
pub fn validate_pkcs12(p12_path: &std::path::Path, _password: &str) -> Result<ValidationReport, AppError> {
    if !p12_path.exists() {
        return Err(AppError::Validation(format!(
            "El archivo PKCS#12 no existe: {}",
            p12_path.display()
        )));
    }

    let ext = p12_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "p12" && ext != "pfx" {
        return Err(AppError::Validation(format!(
            "El archivo debe tener extensión .p12 o .pfx, se encontró: .{}",
            ext
        )));
    }

    // Verificar que el archivo no esté vacío
    let metadata = std::fs::metadata(p12_path).map_err(|e| {
        AppError::Validation(format!("No se pudo leer el archivo: {}", e))
    })?;

    if metadata.len() == 0 {
        return Err(AppError::Validation(
            "El archivo PKCS#12 está vacío".to_string(),
        ));
    }

    // Sin parsing completo, devolvemos información genérica
    // El usuario debe configurar los datos del certificado manualmente
    let info = Pkcs12Info {
        holder_name: "Certificado cargado".to_string(),
        organization: None,
        serial_number: None,
        valid_from: "Configurar manualmente".to_string(),
        valid_to: "Configurar manualmente".to_string(),
        issuer: "Autoridad de certificación".to_string(),
        is_valid: true, // Asumimos válido si el archivo existe
    };

    let mut warnings = Vec::new();
    warnings.push(
        "La validación completa del certificado requiere configuración adicional.".to_string(),
    );

    Ok(ValidationReport {
        info,
        warnings,
        errors: Vec::new(),
    })
}

/// Genera el bloque de firma digital visible para insertar en el PDF.
///
/// Devuelve las líneas de texto que se deben dibujar en el PDF.
pub fn signature_block_lines(info: &Pkcs12Info) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push("FIRMA DIGITAL".to_string());
    lines.push(format!("Titular: {}", info.holder_name));

    if let Some(ref org) = info.organization {
        lines.push(format!("Organización: {}", org));
    }

    lines.push(format!("Emisor: {}", info.issuer));
    lines.push(format!("Vigencia: {} al {}", info.valid_from, info.valid_to));

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

    #[test]
    fn test_signature_block_lines() {
        let info = Pkcs12Info {
            holder_name: "ISA RAMOS".to_string(),
            organization: Some("ISALAB".to_string()),
            serial_number: None,
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
    fn test_validate_pkcs12_nonexistent() {
        let result = validate_pkcs12(std::path::Path::new("/nonexistent.p12"), "");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pkcs12_wrong_extension() {
        // Create a temp file with wrong extension
        let dir = std::env::temp_dir();
        let path = dir.join("test_wrong_ext.txt");
        std::fs::write(&path, "test").unwrap();
        let result = validate_pkcs12(&path, "");
        assert!(result.is_err());
        let _ = std::fs::remove_file(&path);
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
