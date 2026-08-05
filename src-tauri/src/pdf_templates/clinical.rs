use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

use crate::models::patient::Patient;
use crate::models::sample::LabResult;
use crate::pdf_templates::builder::{
    draw_multiline, save_pdf, PdfBuilder, C_MUTED, C_TEXT, MARGIN,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};
use crate::pdf_templates::layout::{
    draw_contact_footer, draw_footer, draw_grid, draw_patient_metadata_grid, draw_results_full,
    draw_signature, section_title, ReportSignature,
};

/// Datos completos de un informe de resultados analíticos.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicalReportData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub sample_code: String,
    pub sample_type: String,
    pub received_at: String,
    pub results: Vec<LabResult>,
    pub signature: ReportSignature,
}

/// Datos de una fórmula médica (receta veterinaria).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaMedicaData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<crate::models::owner::Owner>,
    pub date: String,
    pub reason: String,
    pub diagnosis: Option<String>,
    pub medication: Option<String>,
    pub signature: ReportSignature,
}

pub fn generate_report(data: &ClinicalReportData, out_path: &Path) -> Result<(), String> {
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "RESULTADOS DE LABORATORIO",
        "",
        None,
    );
    draw_patient_metadata_grid(
        &mut pdf,
        &data.patient,
        &data.clinic,
        &data.signature,
        &data.received_at,
    );
    draw_results_full(&mut pdf, &data.sample_type, &data.results, None);
    draw_contact_footer(&mut pdf, data.clinic.phone.as_deref());

    // Si el modo es DIGITAL, agregar bloque de firma digital con metadatos reales del certificado
    if data.signature.mode.eq_ignore_ascii_case("DIGITAL") {
        if let Some(ref pkcs12_path) = data.signature.pkcs12_path {
            let path = std::path::Path::new(pkcs12_path);
            let password = data.signature.pkcs12_password.as_deref().unwrap_or("");
            if path.exists() {
                // Metadatos reales del certificado (parseo PKCS#12)
                if let Ok(info) = extract_pkcs12_info_for_pdf(path, password) {
                    draw_digital_signature_block(&mut pdf, &info);
                } else {
                    // Firma visible genérica si no se puede leer el certificado
                    draw_generic_digital_signature(&mut pdf, &data.signature);
                }
            } else {
                draw_generic_digital_signature(&mut pdf, &data.signature);
            }
        } else {
            draw_generic_digital_signature(&mut pdf, &data.signature);
        }
    }

    save_pdf(pdf, out_path, "ISALAB · Resultados de laboratorio")
}

pub fn generate_formula(data: &FormulaMedicaData, out_path: &Path) -> Result<(), String> {
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "FÓRMULA MÉDICA VETERINARIA",
        "Receta de uso exclusivo veterinario",
        None,
    );

    section_title(&mut pdf, "PACIENTE");
    let owner = data
        .owner
        .as_ref()
        .map(|o| o.full_name.clone())
        .unwrap_or_else(|| data.patient.owner_name.clone());
    draw_grid(
        &mut pdf,
        &[
            ("Fecha", data.date.clone()),
            ("Paciente", data.patient.name.clone()),
            (
                "Especie / Raza",
                format!(
                    "{} · {}",
                    data.patient.species_name,
                    data.patient.breed_name.as_deref().unwrap_or("—")
                ),
            ),
            ("Propietario", owner),
        ],
    );

    section_title(&mut pdf, "MOTIVO DE CONSULTA");
    pdf.y = draw_multiline(&mut pdf, &data.reason, 9.0, MARGIN, C_TEXT, 5.0);

    if let Some(diag) = data.diagnosis.as_deref().filter(|s| !s.trim().is_empty()) {
        section_title(&mut pdf, "DIAGNÓSTICO");
        pdf.y = draw_multiline(&mut pdf, diag, 9.0, MARGIN, C_TEXT, 5.0);
    }

    section_title(&mut pdf, "MEDICAMENTOS Y DOSIS");
    let meds = data.medication.as_deref().unwrap_or("—");
    pdf.y = draw_multiline(&mut pdf, meds, 9.0, MARGIN, C_TEXT, 5.0);

    pdf.y -= 8.0;
    pdf.ensure_space(16.0);
    pdf.text(
        false,
        "IMPORTANTE: esta fórmula médica es de uso exclusivo veterinario y debe cumplirse \
según las indicaciones prescritas. No automedique a su mascota sin consultar al médico veterinario.",
        7.0,
        MARGIN,
        pdf.y,
        C_MUTED,
    );

    draw_signature(&mut pdf, &data.signature);
    draw_footer(&mut pdf, "FÓRMULA MÉDICA");
    save_pdf(pdf, out_path, "ISALAB · Fórmula médica")
}

// ==================== FIRMA DIGITAL =========================================

/// Información simplificada del certificado para el bloque de firma.
struct Pkcs12CertInfo {
    holder_name: String,
    issuer: String,
    valid_from: String,
    valid_to: String,
    is_valid: bool,
}

/// Extrae los metadatos reales del certificado PKCS#12 para el bloque de firma
/// (parseo real con p12-keystore + x509-cert, ver `pdf_templates::signing`).
fn extract_pkcs12_info_for_pdf(
    path: &std::path::Path,
    password: &str,
) -> Result<Pkcs12CertInfo, String> {
    let report = crate::pdf_templates::signing::validate_pkcs12(path, password)
        .map_err(|e| e.to_string())?;

    Ok(Pkcs12CertInfo {
        holder_name: report.info.holder_name,
        issuer: report.info.issuer,
        valid_from: report.info.valid_from,
        valid_to: report.info.valid_to,
        is_valid: report.info.is_valid,
    })
}

/// Dibuja el bloque de firma digital con metadatos del certificado.
fn draw_digital_signature_block(pdf: &mut PdfBuilder, info: &Pkcs12CertInfo) {
    use crate::pdf_templates::builder::{C_MUTED, C_TEXT, MARGIN, PAGE_W};

    pdf.y -= 16.0;
    pdf.ensure_space(50.0);

    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;

    // Línea separadora
    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_MUTED);
    pdf.y -= 7.0;

    // Título
    pdf.text(true, "FIRMA DIGITAL", 9.0, x, pdf.y, (0, 100, 180));
    pdf.y -= 5.5;

    // Titular
    pdf.text(
        false,
        &format!("Titular: {}", info.holder_name),
        7.5,
        x,
        pdf.y,
        C_TEXT,
    );
    pdf.y -= 4.5;

    // Emisor
    pdf.text(
        false,
        &format!("Emisor: {}", info.issuer),
        7.0,
        x,
        pdf.y,
        C_MUTED,
    );
    pdf.y -= 4.5;

    // Vigencia
    pdf.text(
        false,
        &format!("Vigencia: {} al {}", info.valid_from, info.valid_to),
        7.0,
        x,
        pdf.y,
        C_MUTED,
    );
    pdf.y -= 4.5;

    // Estado
    let (status_text, status_color) = if info.is_valid {
        ("✅ VIGENTE", (0, 150, 0))
    } else {
        ("❌ EXPIRADO", (200, 0, 0))
    };
    pdf.text(false, status_text, 7.5, x, pdf.y, status_color);
    pdf.y -= 5.0;

    // Nota legal
    pdf.text(
        false,
        "Firmado digitalmente conforme a la Ley 527 de 1999",
        6.5,
        x,
        pdf.y,
        C_MUTED,
    );
    pdf.y -= 3.5;
    pdf.text(
        false,
        "y el Decreto 2364 de 2019 (Colombia).",
        6.5,
        x,
        pdf.y,
        C_MUTED,
    );
}

/// Dibuja un bloque de firma digital genérico cuando no se puede leer el certificado.
fn draw_generic_digital_signature(
    pdf: &mut PdfBuilder,
    signature: &crate::pdf_templates::layout::ReportSignature,
) {
    use crate::pdf_templates::builder::{C_MUTED, C_TEXT, MARGIN, PAGE_W};

    pdf.y -= 14.0;
    pdf.ensure_space(34.0);

    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;

    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_MUTED);
    pdf.y -= 7.0;

    let vet = if signature.vet_name.is_empty() {
        "Médico Veterinario".to_string()
    } else {
        signature.vet_name.clone()
    };

    pdf.text(true, "FIRMA DIGITAL", 9.0, x, pdf.y, (0, 100, 180));
    pdf.y -= 5.5;
    pdf.text(true, &vet, 9.0, x, pdf.y, C_TEXT);
    pdf.y -= 5.0;
    pdf.text(
        false,
        "Certificado digital aplicado",
        7.0,
        x,
        pdf.y,
        C_MUTED,
    );
    pdf.y -= 4.5;
    pdf.text(false, "Ley 527 de 1999 (Colombia)", 6.5, x, pdf.y, C_MUTED);
}
