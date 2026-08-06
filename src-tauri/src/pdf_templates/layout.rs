//! Helpers de dibujo compartidos entre todos los tipos de reporte PDF.
//! Replicación fiel del formato institucional de reporte de laboratorio.

use barcoders::sym::code128::Code128;

use crate::models::patient::Patient;
use crate::pdf_templates::builder::{
    format_value, PdfBuilder, CONTENT_W, C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W,
};
use crate::pdf_templates::header::ClinicHeader;
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
    /// Contraseña del certificado PKCS#12 (solo para uso interno, nunca se serializa).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pkcs12_password: Option<String>,
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

/// Dibuja el bloque de datos del paciente y médico en 2 columnas estilo reporte oficial.
pub fn draw_patient_metadata_grid(
    pdf: &mut PdfBuilder,
    patient: &Patient,
    clinic: &ClinicHeader,
    signature: &ReportSignature,
    received_at: &str,
) {
    pdf.y -= 3.0;
    pdf.ensure_space(32.0);

    let edad = match patient.age_months {
        m if m < 12 => format!("{m} MESES"),
        m => format!("{} AÑOS", m / 12),
    };
    let sexo = if patient.sex == "M" {
        "MACHO"
    } else {
        "HEMBRA"
    };
    let vet = if signature.vet_name.is_empty() {
        "ISA RAMOS".to_string()
    } else {
        signature.vet_name.to_uppercase()
    };
    let city = clinic
        .city
        .as_deref()
        .unwrap_or("SINCELEJO, SUCRE")
        .to_uppercase();
    let date_str = received_at
        .split(' ')
        .next()
        .unwrap_or(received_at)
        .to_string();

    let left_rows = [
        ("NOMBRE", patient.name.to_uppercase()),
        ("ESPECIE", patient.species_name.to_uppercase()),
        (
            "RAZA",
            patient
                .breed_name
                .as_deref()
                .unwrap_or("CRIOLLO")
                .to_uppercase(),
        ),
        ("SEXO", sexo.to_string()),
        ("EDAD", edad),
    ];

    let right_rows = [
        ("MEDICO VETERINARIO", vet),
        ("EMPRESA", clinic.name.to_uppercase()),
        ("PROPIETARIO", patient.owner_name.to_uppercase()),
        ("MUNICIPIO", city),
        ("TOMA MUESTRA", date_str),
    ];

    let y_start = pdf.y;
    let row_h = 5.2;

    // Columna izquierda
    let x_left_label = MARGIN;
    let x_left_val = MARGIN + 28.0;
    for (i, (label, val)) in left_rows.iter().enumerate() {
        let y_pos = y_start - (i as f32 * row_h);
        pdf.text(true, label, 8.5, x_left_label, y_pos, C_TEXT);
        pdf.text(false, val, 8.5, x_left_val, y_pos, C_TEXT);
    }

    // Columna derecha
    let x_right_label = MARGIN + 92.0;
    let x_right_val = MARGIN + 132.0;
    for (i, (label, val)) in right_rows.iter().enumerate() {
        let y_pos = y_start - (i as f32 * row_h);
        pdf.text(true, label, 8.5, x_right_label, y_pos, C_TEXT);
        pdf.text(false, val, 8.5, x_right_val, y_pos, C_TEXT);
    }

    pdf.y = y_start - (5.0 * row_h) - 4.0;
}

/// Dibuja el bloque de datos del paciente y propietario (formato estándar).
pub fn draw_patient_block(pdf: &mut PdfBuilder, patient: &Patient) {
    section_title(pdf, "PACIENTE Y PROPIETARIO");

    let edad = match patient.age_months {
        m if m < 12 => format!("{m} meses"),
        m => format!("{} años", m / 12),
    };
    let sexo = if patient.sex == "M" {
        "Macho"
    } else {
        "Hembra"
    };

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
            format!(
                "{sexo}{}",
                if patient.neutered {
                    " · Esterilizado"
                } else {
                    ""
                }
            ),
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

/// Dibuja la tabla de resultados analíticos idéntica al modelo de reporte de laboratorio.
pub fn draw_results(pdf: &mut PdfBuilder, results: &[crate::models::sample::LabResult]) {
    draw_results_full(pdf, "HEMATOLOGÍA", results, None);
}

pub fn draw_results_full(
    pdf: &mut PdfBuilder,
    sample_type: &str,
    results: &[crate::models::sample::LabResult],
    notes: Option<&str>,
) {
    pdf.y -= 4.0;
    pdf.ensure_space(25.0);

    // Título de la categoría (ej: HEMATOLOGÍA CONTROL) en azul cian (#0082C8)
    let cat_title = format!("{} CONTROL", sample_type.to_uppercase());
    pdf.text_centered(true, &cat_title, 11.0, pdf.y, (0, 130, 200));
    pdf.y -= 6.0;

    if results.is_empty() {
        pdf.text(
            false,
            "Sin resultados cargados para esta muestra.",
            9.0,
            MARGIN,
            pdf.y,
            C_MUTED,
        );
        return;
    }

    // Anchos de columna en mm (Suma = 185.9 CONTENT_W)
    let w_item = 68.0;
    let w_res = 28.0;
    let w_unit = 44.0;
    let row_h = 6.8;

    // Encabezado con barra azul cian (#2B78C2)
    let header_bg = (43, 120, 194); // #2B78C2
    pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, Some(header_bg), None);

    let y_text = pdf.y - 4.8;
    pdf.text(true, "Ítem", 8.5, MARGIN + 4.0, y_text, (255, 255, 255));
    pdf.text(
        true,
        "Resultados",
        8.5,
        MARGIN + w_item + 2.0,
        y_text,
        (255, 255, 255),
    );
    pdf.text(
        true,
        "Unidades",
        8.5,
        MARGIN + w_item + w_res + 12.0,
        y_text,
        (255, 255, 255),
    );
    pdf.text(
        true,
        "Referencias",
        8.5,
        MARGIN + w_item + w_res + w_unit + 10.0,
        y_text,
        (255, 255, 255),
    );

    pdf.y -= row_h;

    // Filas de datos
    for (idx, r) in results.iter().enumerate() {
        pdf.ensure_space(row_h + 1.0);
        let bg_color = if idx % 2 == 1 {
            Some((244, 247, 250)) // #F4F7FA
        } else {
            Some((255, 255, 255))
        };

        pdf.rect(
            MARGIN,
            pdf.y,
            CONTENT_W,
            row_h,
            bg_color,
            Some((226, 232, 240)),
        );
        let y_row_text = pdf.y - 4.8;

        // 1. Ítem (Nombre + Código corto ej: WBC#)
        pdf.text(
            false,
            &r.analyte_name,
            8.5,
            MARGIN + 4.0,
            y_row_text,
            C_TEXT,
        );
        let code = get_analyte_code(&r.analyte_name);
        if !code.is_empty() {
            pdf.text(false, code, 7.5, MARGIN + 48.0, y_row_text, C_MUTED);
        }

        // 2. Resultados (Destacado en Negrita, Rojo si ALTO, Azul si BAJO, Negro si NORMAL)
        let val_str = format_value(r.value);
        let val_color = match r.status.as_str() {
            "ALTO" => (217, 56, 56), // Red #D93838
            "BAJO" => (0, 130, 200), // Blue #0082C8
            _ => C_TEXT,
        };
        pdf.text(
            true,
            &val_str,
            8.5,
            MARGIN + w_item + 6.0,
            y_row_text,
            val_color,
        );

        // 3. Unidades
        let unit_str = r.unit.as_deref().unwrap_or("—");
        pdf.text(
            false,
            unit_str,
            8.5,
            MARGIN + w_item + w_res + 12.0,
            y_row_text,
            (60, 60, 60),
        );

        // 4. Referencias (Min - Max)
        let ref_str = match (r.ref_min, r.ref_max) {
            (Some(min), Some(max)) => format!("{min:.1} – {max:.1}"),
            _ => "—".to_string(),
        };
        pdf.text(
            false,
            &ref_str,
            8.5,
            MARGIN + w_item + w_res + w_unit + 10.0,
            y_row_text,
            (60, 60, 60),
        );

        pdf.y -= row_h;
    }

    pdf.y -= 3.0;
    // Nota técnica
    pdf.text(true, "Técnica:", 7.5, MARGIN, pdf.y, C_TEXT);
    pdf.text(
        false,
        " Lectura Automatizada: MINDRAY B2800 RUFFOS LABS",
        7.5,
        MARGIN + 12.0,
        pdf.y,
        (80, 80, 80),
    );
    pdf.y -= 7.0;

    // Observaciones
    pdf.ensure_space(20.0);
    pdf.text(true, "Observaciones:", 8.5, MARGIN, pdf.y, C_TEXT);
    pdf.y -= 5.0;

    if let Some(n) = notes.filter(|s| !s.trim().is_empty()) {
        for line in n.lines() {
            pdf.text(false, line, 8.0, MARGIN, pdf.y, C_TEXT);
            pdf.y -= 4.5;
        }
    } else {
        // Resumen diagnóstico por líneas
        let has_wb_issue = results.iter().any(|r| {
            (r.status == "ALTO" || r.status == "BAJO")
                && (r.analyte_name.contains("Linfo") || r.analyte_name.contains("Neutro"))
        });
        let line_white = if has_wb_issue {
            "LINFOCITOSIS CON NEUTROPENIA RELATIVA"
        } else {
            "EN RANGO"
        };

        pdf.text(true, "Línea Roja ", 8.0, MARGIN + 4.0, pdf.y, C_TEXT);
        pdf.text(false, "EN RANGO", 8.0, MARGIN + 28.0, pdf.y, (80, 80, 80));
        pdf.y -= 4.5;

        pdf.text(true, "Línea Blanca ", 8.0, MARGIN + 4.0, pdf.y, C_TEXT);
        pdf.text(false, line_white, 8.0, MARGIN + 28.0, pdf.y, (80, 80, 80));
        pdf.y -= 4.5;

        pdf.text(true, "Plaquetas ", 8.0, MARGIN + 4.0, pdf.y, C_TEXT);
        pdf.text(false, "EN RANGO", 8.0, MARGIN + 28.0, pdf.y, (80, 80, 80));
        pdf.y -= 4.5;
    }
}

fn get_analyte_code(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("leucocito") && !n.contains("%") {
        "WBC#"
    } else if n.contains("linfocito") && !n.contains("%") {
        "LYM#"
    } else if n.contains("monocito") && !n.contains("%") {
        "MON#"
    } else if (n.contains("neutrófilo") || n.contains("neutrofilo")) && !n.contains("%") {
        "NEU#"
    } else if (n.contains("eosinófilo") || n.contains("eosinofilo")) && !n.contains("%") {
        "EOS#"
    } else if n.contains("linfocito") && n.contains("%") {
        "LYM%"
    } else if n.contains("monocito") && n.contains("%") {
        "MON%"
    } else if (n.contains("neutrófilo") || n.contains("neutrofilo")) && n.contains("%") {
        "NEU%"
    } else if (n.contains("eosinófilo") || n.contains("eosinofilo")) && n.contains("%") {
        "EOS%"
    } else if n.contains("eritrocito") || n.contains("glóbulos rojos") {
        "RBC"
    } else if n.contains("hemoglobina") {
        "Hb"
    } else if n.contains("hematocrito") {
        "HCT"
    } else if n.contains("plaqueta") {
        "PLT"
    } else if n.contains("mcv") || n.contains("vcm") {
        "MCV"
    } else if n.contains("mch") || n.contains("hcm") {
        "MCH"
    } else if n.contains("mchc") || n.contains("chcm") {
        "MCHC"
    } else {
        ""
    }
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
        pdf.text(
            false,
            &format!("Tarjeta profesional MVZ {lic}"),
            7.5,
            x,
            pdf.y,
            C_MUTED,
        );
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

/// Dibuja el número de contacto centrado al final de la página.
pub fn draw_contact_footer(pdf: &mut PdfBuilder, phone: Option<&str>) {
    let ph = phone.unwrap_or("(+57) 314 6754530");
    pdf.text_centered(false, "Numero de contacto", 8.5, MARGIN + 4.0, C_MUTED);
    pdf.text_centered(true, ph, 9.5, MARGIN - 1.0, C_TEXT);
}

// ---------- Código de barras (Code 128) ----------

/// Marcador de charset inicial que exige barcoders v2 (Ɓ = character-set B,
/// cubre ASCII imprimible: letras, dígitos y guiones de los códigos PAC-…).
const CODE128_SET_B: &str = "\u{0181}";

/// Ancho en mm del código de barras Code 128 de `text` (sin dibujarlo), o
/// `None` si el texto no es codificable.
pub fn code128_width(text: &str, module_mm: f32) -> Option<f32> {
    // barcoders no rechaza el marcador sin datos: un código vacío sería ilegible.
    if text.trim().is_empty() {
        return None;
    }
    let code = Code128::new(format!("{CODE128_SET_B}{text}")).ok()?;
    let bits = code.encode();
    if bits.is_empty() {
        return None;
    }
    Some(bits.len() as f32 * module_mm)
}

/// Dibuja un código de barras Code 128 en `(x, y)` (esquina superior
/// izquierda) con `height_mm` de alto y módulos de `module_mm`. Los bits de
/// `encode()` se dibujan como barras vectoriales de un módulo (las barras
/// adyacentes se fusionan en barras más anchas, como exige el estándar).
/// Devuelve el ancho total en mm (0 si no se pudo codificar).
pub fn draw_code128(
    pdf: &mut PdfBuilder,
    text: &str,
    x: f32,
    y: f32,
    height_mm: f32,
    module_mm: f32,
) -> f32 {
    if text.trim().is_empty() {
        return 0.0;
    }
    let Ok(code) = Code128::new(format!("{CODE128_SET_B}{text}")) else {
        return 0.0;
    };
    let bits = code.encode();
    if bits.is_empty() {
        return 0.0;
    }
    let mut x = x;
    for &bit in &bits {
        if bit == 1 {
            pdf.rect(x, y, module_mm, height_mm, Some(C_TEXT), None);
        }
        x += module_mm;
    }
    bits.len() as f32 * module_mm
}

/// Dibuja el código de barras centrado horizontalmente en la página y, debajo,
/// el código en texto legible (para humanos). No dibuja nada si el texto no es
/// codificable.
pub fn draw_code128_centered(pdf: &mut PdfBuilder, text: &str, height_mm: f32, module_mm: f32) {
    let Some(total) = code128_width(text, module_mm) else {
        return;
    };
    pdf.ensure_space(height_mm + 10.0);
    draw_code128(
        pdf,
        text,
        (PAGE_W - total) / 2.0,
        pdf.y,
        height_mm,
        module_mm,
    );
    // Gap de 3 mm para que el ascensor del texto no toque las barras.
    pdf.y -= height_mm + 3.0;
    pdf.text_centered(false, text, 7.5, pdf.y, C_MUTED);
    pdf.y -= 4.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code128_width_pac_code() {
        // "PAC-2026-0001" (13 chars, set B): start 11 + 13*11 + check 11 +
        // stop 13 + term 1 = 179 módulos. A 0.33 mm ≈ 59 mm.
        let w = code128_width("PAC-2026-0001", 0.33).expect("código válido");
        assert!(w > 50.0 && w < 70.0, "ancho inesperado: {w}");
    }

    #[test]
    fn test_code128_invalid_text_returns_none() {
        assert!(code128_width("", 0.33).is_none());
        // Caracteres fuera del charset B (p. ej. ñ) no codifican.
        assert!(code128_width("PAC-Ñ-1", 0.33).is_none());
    }

    #[test]
    fn test_draw_code128_draws_bars_and_returns_width() {
        let mut pdf = PdfBuilder::new();
        let before = pdf.ops.len();
        let w = draw_code128(&mut pdf, "PAC-2026-0001", 20.0, 250.0, 12.0, 0.33);
        assert!(w > 50.0 && w < 70.0);
        assert!(pdf.ops.len() > before, "debe añadir operaciones de dibujo");
    }

    #[test]
    fn test_draw_code128_invalid_draws_nothing() {
        let mut pdf = PdfBuilder::new();
        let before = pdf.ops.len();
        let w = draw_code128(&mut pdf, "", 20.0, 250.0, 12.0, 0.33);
        assert_eq!(w, 0.0);
        assert_eq!(pdf.ops.len(), before);
    }

    #[test]
    fn test_code128_bits_start_and_end_with_bar() {
        let code = Code128::new(format!("{CODE128_SET_B}PAC-2026-0001")).unwrap();
        let bits = code.encode();
        assert_eq!(bits.first(), Some(&1), "debe empezar con barra");
        assert_eq!(bits.last(), Some(&1), "debe terminar con barra");
        // 13 caracteres en set B: start 11 + 13*11 + check 11 + stop 11 + term 2 = 178.
        assert_eq!(bits.len(), 178);
    }
}
