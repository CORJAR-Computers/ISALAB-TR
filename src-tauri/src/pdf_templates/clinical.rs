use std::path::Path;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::patient::Patient;
use crate::models::sample::LabResult;
use crate::pdf_templates::builder::{
    draw_multiline, save_pdf, sanitize, C_MUTED, C_TEXT, MARGIN, PdfBuilder,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};
use crate::pdf_templates::layout::{
    draw_footer, draw_grid, draw_lab_note, draw_patient_block, draw_results, draw_signature,
    section_title, ReportSignature,
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
    if data.signature.mode.eq_ignore_ascii_case("DIGITAL") {
        return Err(
            "La firma digital (PKCS#12) aún no está implementada; selecciona firma GRAPHIC en Configuración."
                .to_string(),
        );
    }

    let mut pdf = PdfBuilder::new();
    let subtitle = format!(
        "Muestra {} · {}",
        data.sample_code,
        sanitize(&data.sample_type)
    );
    let recv = format!("Fecha de recepción: {}", data.received_at);
    draw_header(
        &mut pdf,
        &data.clinic,
        "INFORME DE RESULTADOS DE LABORATORIO",
        &subtitle,
        Some(&recv),
    );
    draw_patient_block(&mut pdf, &data.patient);
    draw_results(&mut pdf, &data.results);
    draw_signature(&mut pdf, &data.signature);
    draw_lab_note(&mut pdf);
    draw_footer(&mut pdf, &format!("Muestra {}", data.sample_code));

    save_pdf(pdf, out_path, "ISALAB · Informe de resultados de laboratorio")
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
    draw_grid(&mut pdf, &[
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
    ]);

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
