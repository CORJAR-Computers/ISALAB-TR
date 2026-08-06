use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::vaccine::Vaccine;
use crate::pdf_templates::builder::{
    sanitize, save_pdf, truncate, PdfBuilder, CONTENT_W, C_HEADER_BG, C_MUTED, C_RULE, C_TEXT,
    MARGIN, PAGE_H, PAGE_W,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};
use crate::pdf_templates::layout::{draw_code128_centered, draw_footer, draw_grid, section_title};

/// Datos del certificado/carnet de vacunación.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacunacionData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub vaccines: Vec<Vaccine>,
    pub certificate_number: String,
    pub issued_at: String,
    pub veterinarian: String,
}

pub fn generate_vacunacion(data: &VacunacionData, out_path: &Path) -> Result<(), String> {
    let mut pdf = PdfBuilder::new();
    // Marco del certificado.
    pdf.rect(
        MARGIN - 2.0,
        PAGE_H - MARGIN + 2.0,
        CONTENT_W + 4.0,
        PAGE_H - 2.0 * MARGIN + 4.0,
        None,
        Some(C_TEXT),
    );

    draw_header(
        &mut pdf,
        &data.clinic,
        "CERTIFICADO DE VACUNACIÓN",
        "Registro oficial de inmunización animal",
        None,
    );

    section_title(&mut pdf, "DATOS DEL CERTIFICADO");
    draw_grid(
        &mut pdf,
        &[
            ("No. certificado", data.certificate_number.clone()),
            ("Fecha de emisión", data.issued_at.clone()),
            (
                "Médico veterinario",
                if data.veterinarian.is_empty() {
                    "—".into()
                } else {
                    data.veterinarian.clone()
                },
            ),
        ],
    );

    let owner = data
        .owner
        .as_ref()
        .map(|o| o.full_name.clone())
        .unwrap_or_else(|| data.patient.owner_name.clone());
    let mut owner_rows: Vec<(&str, String)> = vec![("Nombre", owner)];
    if let Some(o) = data.owner.as_ref() {
        if !o.document_number.is_empty() {
            owner_rows.push((
                "Documento",
                format!("{} {}", o.document_type, o.document_number),
            ));
        }
        if let Some(p) = o.phone.as_deref().filter(|p| !p.is_empty()) {
            owner_rows.push(("Teléfono", p.to_string()));
        }
    }
    section_title(&mut pdf, "PROPIETARIO");
    draw_grid(&mut pdf, &owner_rows);

    let edad = match data.patient.age_months {
        m if m < 12 => format!("{m} meses"),
        m => format!("{} años", m / 12),
    };
    section_title(&mut pdf, "PACIENTE");
    draw_grid(
        &mut pdf,
        &[
            ("Nombre", data.patient.name.clone()),
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
            ("Edad", edad),
        ],
    );

    // Código de barras del paciente (Code 128): se escanea en recepción para
    // abrir la ficha con el escáner de Pacientes (código PAC-…). Módulos de
    // 0.33 mm (X-dimension estándar de impresión) y ~12 mm de alto.
    draw_code128_centered(&mut pdf, &data.patient.code, 12.0, 0.33);

    section_title(&mut pdf, "VACUNAS ADMINISTRADAS");
    if data.vaccines.is_empty() {
        pdf.text(
            false,
            "Sin vacunas registradas.",
            9.0,
            MARGIN,
            pdf.y,
            C_MUTED,
        );
        pdf.y -= 5.0;
    } else {
        let row_h = 6.2;
        let cols: [(f32, &str); 5] = [
            (58.0, "VACUNA"),
            (34.0, "FECHA"),
            (26.0, "LOTE"),
            (34.0, "REFUERZO"),
            (28.0, "VETERINARIO"),
        ];
        pdf.rect(
            MARGIN,
            pdf.y,
            CONTENT_W,
            row_h,
            Some(C_HEADER_BG),
            Some(C_RULE),
        );
        let mut x = MARGIN + 2.0;
        for (w, label) in cols {
            pdf.text(true, label, 7.0, x, pdf.y - 1.6, C_TEXT);
            x += w;
        }
        pdf.y -= row_h;

        for v in &data.vaccines {
            pdf.ensure_space(row_h + 1.0);
            pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, None, Some(C_RULE));
            let mut x = MARGIN + 2.0;
            pdf.text(
                false,
                &truncate(&v.vaccine_name, 20),
                7.5,
                x,
                pdf.y - 1.6,
                C_TEXT,
            );
            x += cols[0].0;
            pdf.text(false, &v.administered_at, 7.0, x, pdf.y - 1.6, C_TEXT);
            x += cols[1].0;
            pdf.text(
                false,
                v.lot.as_deref().unwrap_or("—"),
                7.0,
                x,
                pdf.y - 1.6,
                C_TEXT,
            );
            x += cols[2].0;
            pdf.text(
                false,
                v.next_dose_at.as_deref().unwrap_or("—"),
                7.0,
                x,
                pdf.y - 1.6,
                C_TEXT,
            );
            x += cols[3].0;
            pdf.text(
                false,
                &truncate(v.veterinarian_name.as_deref().unwrap_or("—"), 15),
                7.0,
                x,
                pdf.y - 1.6,
                C_TEXT,
            );
            pdf.y -= row_h;
        }
    }

    // Próximo refuerzo destacado.
    pdf.y -= 6.0;
    pdf.ensure_space(20.0);
    let next = data
        .vaccines
        .iter()
        .filter(|v| v.next_dose_at.is_some())
        .min_by(|a, b| a.next_dose_at.cmp(&b.next_dose_at));
    if let Some(nv) = next {
        let label = format!(
            "PRÓXIMO REFUERZO · {} — {}",
            nv.vaccine_name,
            nv.next_dose_at.as_deref().unwrap_or("")
        );
        pdf.rect(
            MARGIN,
            pdf.y,
            CONTENT_W,
            10.0,
            Some(C_HEADER_BG),
            Some(C_RULE),
        );
        pdf.text(
            true,
            &sanitize(&label),
            9.0,
            MARGIN + 4.0,
            pdf.y - 4.0,
            C_TEXT,
        );
        pdf.y -= 14.0;
    } else if !data.vaccines.is_empty() {
        pdf.text(
            false,
            "Al día: sin refuerzos pendientes de programar.",
            8.0,
            MARGIN,
            pdf.y,
            C_MUTED,
        );
        pdf.y -= 6.0;
    }

    // Firma del veterinario.
    pdf.y -= 10.0;
    pdf.ensure_space(28.0);
    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;
    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_RULE);
    pdf.y -= 7.0;
    let vet = if data.veterinarian.is_empty() {
        "Médico Veterinario".to_string()
    } else {
        data.veterinarian.clone()
    };
    pdf.text(true, &truncate(&vet, 32), 9.0, x, pdf.y, C_TEXT);
    pdf.y -= 5.0;
    pdf.text(false, "Firma", 7.5, x, pdf.y, C_MUTED);

    draw_footer(
        &mut pdf,
        &format!("Certificado {}", data.certificate_number),
    );
    save_pdf(pdf, out_path, "ISALAB · Certificado de vacunación")
}
