use std::path::Path;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::sample::LabResult;
use crate::pdf_templates::builder::{
    draw_multiline, format_value, save_pdf, status_color, status_label,
    CONTENT_W, C_HEADER_BG, C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W, PdfBuilder,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};

/// Configuración de firma para reportes.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportSignature {
    pub mode: String,
    pub vet_name: String,
    pub vet_license: Option<String>,
    pub signature_image_path: Option<String>,
    pub pkcs12_path: Option<String>,
}

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
    pub owner: Option<Owner>,
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
        crate::pdf_templates::builder::sanitize(&data.sample_type)
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

pub fn section_title(pdf: &mut PdfBuilder, title: &str) {
    pdf.y -= 4.0;
    pdf.ensure_space(30.0);
    pdf.text(true, title, 8.5, MARGIN, pdf.y, C_MUTED);
    pdf.y -= 6.0;
}

pub fn draw_grid(pdf: &mut PdfBuilder, rows: &[(&str, String)]) {
    let col_w = CONTENT_W / 2.0;
    for (i, (label, value)) in rows.iter().enumerate() {
        pdf.ensure_space(6.0);
        let x = if i % 2 == 0 { MARGIN } else { MARGIN + col_w };
        pdf.text(true, label, 7.5, x, pdf.y, C_MUTED);
        pdf.text(false, value, 9.0, x + 26.0, pdf.y, C_TEXT);
        if i % 2 == 1 {
            pdf.y -= 5.8;
        }
    }
    pdf.y -= 4.0;
    pdf.rule(MARGIN, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
}

pub fn draw_patient_block(pdf: &mut PdfBuilder, patient: &Patient) {
    section_title(pdf, "PACIENTE Y PROPIETARIO");

    let edad = match patient.age_months {
        m if m < 12 => format!("{m} meses"),
        m => format!("{} años", m / 12),
    };
    let sexo = if patient.sex == "M" { "Macho" } else { "Hembra" };

    let rows: Vec<(&str, String)> = vec![
        ("Paciente", patient.name.clone()),
        (
            "Especie / Raza",
            format!(
                "{} · {}",
                patient.species_name,
                patient.breed_name.as_deref().unwrap_or("—")
            ),
        ),
        (
            "Sexo",
            format!("{sexo}{}", if patient.neutered { " · Esterilizado" } else { "" }),
        ),
        ("Edad", edad),
        ("Propietario", patient.owner_name.clone()),
        (
            "Identificación",
            patient
                .microchip
                .as_deref()
                .map(|m| format!("Chip {m}"))
                .unwrap_or_else(|| "—".into()),
        ),
    ];
    draw_grid(pdf, &rows);
}

pub fn draw_results(pdf: &mut PdfBuilder, results: &[LabResult]) {
    pdf.y -= 4.0;
    pdf.ensure_space(16.0);
    pdf.text(true, "RESULTADOS ANALÍTICOS", 8.5, MARGIN, pdf.y, C_MUTED);
    pdf.y -= 6.0;

    if results.is_empty() {
        pdf.text(false, "Sin resultados cargados para esta muestra.", 9.0, MARGIN, pdf.y, C_MUTED);
        return;
    }

    let cols: [(f32, &str); 5] = [
        (62.0, "ANALITO"),
        (24.0, "RESULTADO"),
        (20.0, "UNIDAD"),
        (40.0, "RANGO REF."),
        (34.0, "ESTADO"),
    ];
    let row_h = 6.2;

    pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, Some(C_HEADER_BG), Some(C_RULE));
    let mut x = MARGIN + 2.0;
    for (w, label) in cols {
        pdf.text(true, label, 7.5, x, pdf.y - 1.6, C_TEXT);
        x += w;
    }
    pdf.y -= row_h;

    for r in results {
        pdf.ensure_space(row_h + 1.0);
        let color = status_color(&r.status);
        pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, None, Some(C_RULE));
        let mut x = MARGIN + 2.0;

        pdf.text(false, &r.analyte_name, 8.5, x, pdf.y - 1.6, C_TEXT);
        x += cols[0].0;
        pdf.text(true, &format_value(r.value), 8.5, x, pdf.y - 1.6, color);
        x += cols[1].0;
        pdf.text(false, r.unit.as_deref().unwrap_or("—"), 7.5, x, pdf.y - 1.6, C_MUTED);
        x += cols[2].0;
        let range = match (r.ref_min, r.ref_max) {
            (Some(min), Some(max)) => format!("{min} – {max}"),
            _ => "—".to_string(),
        };
        pdf.text(false, &range, 7.5, x, pdf.y - 1.6, C_TEXT);
        x += cols[3].0;
        pdf.text(true, status_label(&r.status), 7.5, x, pdf.y - 1.6, color);
        pdf.y -= row_h;
    }
    pdf.y -= 4.0;
    pdf.rule(MARGIN, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
}

pub fn draw_signature(pdf: &mut PdfBuilder, signature: &ReportSignature) {
    pdf.y -= 14.0;
    pdf.ensure_space(34.0);

    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;

    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_RULE);
    pdf.y -= 7.0;
    let vet = if signature.vet_name.is_empty() {
        "Médico Veterinario".to_string()
    } else {
        signature.vet_name.clone()
    };
    pdf.text(true, &vet, 9.0, x, pdf.y, C_TEXT);
    if let Some(lic) = signature.vet_license.as_deref().filter(|l| !l.is_empty()) {
        pdf.y -= 5.0;
        pdf.text(false, &format!("Tarjeta profesional MVZ {lic}"), 7.5, x, pdf.y, C_MUTED);
    }
    pdf.y -= 5.0;
    pdf.text(false, "Firma", 7.5, x, pdf.y, C_MUTED);
}

pub fn draw_lab_note(pdf: &mut PdfBuilder) {
    pdf.y -= 14.0;
    pdf.ensure_space(14.0);
    pdf.text(
        false,
        "Los valores marcados como ALTO o BAJO se encuentran fuera del rango de referencia para la especie, sexo y edad del paciente.",
        7.0,
        MARGIN,
        pdf.y,
        C_MUTED,
    );
}

pub fn draw_footer(pdf: &mut PdfBuilder, label: &str) {
    let footer = format!("ISALAB · Documento generado automáticamente · {label}");
    pdf.text(false, &footer, 7.0, MARGIN, MARGIN - 4.0, C_MUTED);
}
