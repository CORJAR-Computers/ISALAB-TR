use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::surgery::Surgery;
use crate::pdf_templates::builder::{
    draw_multiline, save_pdf, truncate, CONTENT_W, C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W,
    PdfBuilder,
};
use crate::pdf_templates::clinical::{
    draw_footer, draw_grid, draw_signature, section_title, ReportSignature,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};

pub const CONSENT_DECLARATION: &str = "Autorizo al médico veterinario y al personal de la clínica a \
realizar sobre mi mascota el procedimiento descrito. Declaro haber sido informado(a) de los \
riesgos, beneficios y alternativas, y de los cuidados posteriores necesarios. Entiendo que el \
resultado del procedimiento no está garantizado y que pueden presentarse complicaciones \
inherentes a todo procedimiento médico.";

pub const CONSENT_RISKS: &str = "Como en todo procedimiento médico existen riesgos inherentes: \
reacciones adversas a la anestesia, sangrado, infección, edema, dehiscencia de herida y, en \
casos excepcionales, complicaciones que pueden comprometer la vida del paciente. El equipo \
veterinario tomará todas las medidas preventivas y correctivas a su alcance.";

/// Datos del consentimiento informado (procedimiento quirúrgico).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentimientoData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub attention_code: String,
    pub procedure_type: String,
    pub procedure_date: String,
    pub description: Option<String>,
    pub post_care: Option<String>,
    pub veterinarian: String,
}

/// Datos del reporte/certificado quirúrgico.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CirugiaData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub surgery: Surgery,
    pub signature: ReportSignature,
}

fn surgery_status_label(s: &str) -> &'static str {
    match s {
        "PROGRAMADA" => "Programada",
        "EN_CURSO" => "En curso",
        "COMPLETADA" => "Completada",
        "CANCELADA" => "Cancelada",
        _ => "—",
    }
}

pub fn generate_consentimiento(data: &ConsentimientoData, out_path: &Path) -> Result<(), String> {
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "CONSENTIMIENTO INFORMADO",
        "Procedimiento quirúrgico",
        None,
    );

    section_title(&mut pdf, "INFORMACIÓN DEL PROCEDIMIENTO");
    draw_grid(&mut pdf, &[
        ("Fecha", data.procedure_date.clone()),
        ("Tipo de procedimiento", data.procedure_type.clone()),
        ("Código de atención", data.attention_code.clone()),
    ]);

    let owner_display = data
        .owner
        .as_ref()
        .map(|o| o.full_name.clone())
        .unwrap_or_else(|| data.patient.owner_name.clone());
    let mut rows: Vec<(&str, String)> = vec![
        ("Paciente", data.patient.name.clone()),
        (
            "Especie / Raza",
            format!(
                "{} · {}",
                data.patient.species_name,
                data.patient.breed_name.as_deref().unwrap_or("—")
            ),
        ),
        ("Propietario", owner_display.clone()),
    ];
    if let Some(o) = data.owner.as_ref() {
        if !o.document_number.is_empty() {
            rows.push((
                "Documento",
                format!("{} {}", o.document_type, o.document_number),
            ));
        }
        if let Some(p) = o.phone.as_deref().filter(|p| !p.is_empty()) {
            rows.push(("Teléfono", p.to_string()));
        }
    }
    section_title(&mut pdf, "DATOS DEL PACIENTE Y PROPIETARIO");
    draw_grid(&mut pdf, &rows);

    section_title(&mut pdf, "DESCRIPCIÓN DEL PROCEDIMIENTO");
    let desc = data.description.as_deref().unwrap_or("—");
    pdf.y = draw_multiline(&mut pdf, desc, 9.0, MARGIN, C_TEXT, 5.0);

    section_title(&mut pdf, "RIESGOS Y COMPLICACIONES POTENCIALES");
    pdf.y = draw_multiline(&mut pdf, CONSENT_RISKS, 9.0, MARGIN, C_TEXT, 5.0);

    section_title(&mut pdf, "CUIDADOS POST-PROCEDIMIENTO");
    let care = data.post_care.as_deref().unwrap_or("—");
    pdf.y = draw_multiline(&mut pdf, care, 9.0, MARGIN, C_TEXT, 5.0);

    section_title(&mut pdf, "DECLARACIÓN Y AUTORIZACIÓN");
    pdf.y = draw_multiline(&mut pdf, CONSENT_DECLARATION, 9.0, MARGIN, C_TEXT, 5.0);

    // Firmas (propietario y veterinario) en paralelo.
    pdf.y -= 16.0;
    pdf.ensure_space(30.0);
    let sig_y = pdf.y;
    let half = CONTENT_W / 2.0 - 12.0;
    pdf.rule(MARGIN, sig_y, MARGIN + half, sig_y, C_RULE);
    pdf.text(true, &truncate(&owner_display, 28), 8.5, MARGIN, sig_y - 6.0, C_TEXT);
    pdf.text(false, "Firma del propietario", 7.5, MARGIN, sig_y - 11.0, C_MUTED);
    let x2 = PAGE_W - MARGIN - half;
    pdf.rule(x2, sig_y, PAGE_W - MARGIN, sig_y, C_RULE);
    pdf.text(true, &truncate(&data.veterinarian, 28), 8.5, x2, sig_y - 6.0, C_TEXT);
    pdf.text(false, "Firma del médico veterinario", 7.5, x2, sig_y - 11.0, C_MUTED);
    pdf.y = sig_y - 13.0;

    draw_footer(&mut pdf, "CONSENTIMIENTO INFORMADO");
    save_pdf(pdf, out_path, "ISALAB · Consentimiento informado")
}

pub fn generate_cirugia(data: &CirugiaData, out_path: &Path) -> Result<(), String> {
    let s = &data.surgery;
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "REPORTE QUIRÚRGICO",
        "Registro de procedimiento quirúrgico",
        None,
    );

    let owner = data
        .owner
        .as_ref()
        .map(|o| o.full_name.clone())
        .unwrap_or_else(|| data.patient.owner_name.clone());
    section_title(&mut pdf, "DATOS DEL PACIENTE Y PROPIETARIO");
    draw_grid(&mut pdf, &[
        ("Paciente", data.patient.name.clone()),
        (
            "Especie / Raza",
            format!(
                "{} · {}",
                data.patient.species_name,
                data.patient.breed_name.as_deref().unwrap_or("—")
            ),
        ),
        (
            "Sexo",
            if data.patient.sex == "M" {
                "Macho".to_string()
            } else {
                "Hembra".to_string()
            },
        ),
        ("Propietario", owner),
    ]);

    section_title(&mut pdf, "PROCEDIMIENTO");
    draw_grid(&mut pdf, &[
        ("Tipo de cirugía", s.surgery_type.clone()),
        ("Fecha programada", s.scheduled_at.clone()),
        (
            "Anestesia",
            s.anesthesia_type.clone().unwrap_or_else(|| "—".into()),
        ),
        ("Estado", surgery_status_label(&s.status).to_string()),
        (
            "Veterinario",
            s.veterinarian_name.clone().unwrap_or_else(|| "—".into()),
        ),
    ]);

    section_title(&mut pdf, "NOTAS PREOPERATORIAS");
    pdf.y = draw_multiline(
        &mut pdf,
        s.preoperative_notes.as_deref().unwrap_or("—"),
        9.0,
        MARGIN,
        C_TEXT,
        5.0,
    );

    section_title(&mut pdf, "NOTAS POSTOPERATORIAS");
    pdf.y = draw_multiline(
        &mut pdf,
        s.postoperative_notes.as_deref().unwrap_or("—"),
        9.0,
        MARGIN,
        C_TEXT,
        5.0,
    );

    draw_signature(&mut pdf, &data.signature);
    draw_footer(&mut pdf, &format!("Reporte quirúrgico {}", s.id));
    save_pdf(pdf, out_path, "ISALAB · Reporte quirúrgico")
}
