//! Helpers de dibujo compartidos entre todos los tipos de reporte PDF.
//!
//! Centraliza funciones de layout (títulos de sección, grillas de datos,
//! bloques de paciente, firmas, notas y pies de página) que antes estaban
//! dispersos en `clinical.rs`, creando dependencias cruzadas innecesarias.

use crate::models::patient::Patient;
use crate::pdf_templates::builder::{
    format_value, status_color, status_label,
    CONTENT_W, C_HEADER_BG, C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W, PdfBuilder,
};
use serde::{Deserialize, Serialize};
use specta::Type;

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

/// Dibuja un título de sección (label muted, con separador posterior).
pub fn section_title(pdf: &mut PdfBuilder, title: &str) {
    pdf.y -= 4.0;
    pdf.ensure_space(30.0);
    pdf.text(true, title, 8.5, MARGIN, pdf.y, C_MUTED);
    pdf.y -= 6.0;
}

/// Dibuja una grilla de pares label→value en dos columnas.
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

/// Dibuja el bloque de datos del paciente y propietario.
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

/// Dibuja la tabla de resultados analíticos con alineación perfecta, columnas definidas y colores de estado.
pub fn draw_results(pdf: &mut PdfBuilder, results: &[crate::models::sample::LabResult]) {
    pdf.y -= 4.0;
    pdf.ensure_space(20.0);
    pdf.text(true, "RESULTADOS ANALÍTICOS", 8.5, MARGIN, pdf.y, C_MUTED);
    pdf.y -= 6.0;

    if results.is_empty() {
        pdf.text(false, "Sin resultados cargados para esta muestra.", 9.0, MARGIN, pdf.y, C_MUTED);
        return;
    }

    let cols: [(f32, &str); 5] = [
        (60.0, "ANALITO"),
        (28.0, "RESULTADO"),
        (24.0, "UNIDAD"),
        (40.0, "RANGO REF."),
        (33.9, "ESTADO"),
    ];
    let row_h = 7.0;

    // Encabezado de la tabla
    pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, Some(C_HEADER_BG), Some(C_RULE));
    let mut x_col = MARGIN;
    for (w, label) in cols {
        pdf.text(true, label, 7.5, x_col + 3.0, pdf.y - 4.8, C_TEXT);
        x_col += w;
        if x_col < MARGIN + CONTENT_W - 1.0 {
            pdf.rule(x_col, pdf.y, x_col, pdf.y - row_h, C_RULE);
        }
    }
    pdf.y -= row_h;

    // Filas de datos
    for (idx, r) in results.iter().enumerate() {
        pdf.ensure_space(row_h + 1.0);
        let bg_color = if idx % 2 == 1 {
            Some((248, 250, 252))
        } else {
            None
        };
        let color = status_color(&r.status);
        pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, bg_color, Some(C_RULE));

        let mut x = MARGIN;

        // 1. Analito
        pdf.text(false, &r.analyte_name, 8.5, x + 3.0, pdf.y - 4.8, C_TEXT);
        x += cols[0].0;
        pdf.rule(x, pdf.y, x, pdf.y - row_h, C_RULE);

        // 2. Resultado (Destacado en negrita y color según rango)
        pdf.text(true, &format_value(r.value), 8.5, x + 3.0, pdf.y - 4.8, color);
        x += cols[1].0;
        pdf.rule(x, pdf.y, x, pdf.y - row_h, C_RULE);

        // 3. Unidad
        pdf.text(false, r.unit.as_deref().unwrap_or("—"), 7.5, x + 3.0, pdf.y - 4.8, C_MUTED);
        x += cols[2].0;
        pdf.rule(x, pdf.y, x, pdf.y - row_h, C_RULE);

        // 4. Rango de Referencia
        let range = match (r.ref_min, r.ref_max) {
            (Some(min), Some(max)) => format!("{min} – {max}"),
            _ => "—".to_string(),
        };
        pdf.text(false, &range, 7.5, x + 3.0, pdf.y - 4.8, C_TEXT);
        x += cols[3].0;
        pdf.rule(x, pdf.y, x, pdf.y - row_h, C_RULE);

        // 5. Estado
        pdf.text(true, status_label(&r.status), 7.5, x + 3.0, pdf.y - 4.8, color);

        pdf.y -= row_h;
    }
    pdf.y -= 4.0;
    pdf.rule(MARGIN, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
}

/// Dibuja el bloque de firma del veterinario.
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

/// Dibuja la nota sobre valores fuera de rango.
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

/// Dibuja el pie de página del documento.
pub fn draw_footer(pdf: &mut PdfBuilder, label: &str) {
    let footer = format!("ISALAB · Documento generado automáticamente · {label}");
    pdf.text(false, &footer, 7.0, MARGIN, MARGIN - 4.0, C_MUTED);
}
