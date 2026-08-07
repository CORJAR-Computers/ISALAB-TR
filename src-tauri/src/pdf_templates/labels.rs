//! # Etiquetas de tubos de muestras (hoja imprimible)
//!
//! Genera una hoja Carta con una grilla de etiquetas adhesivas (2 columnas × 4
//! filas = 8 por página). Cada etiqueta incluye el código de trazabilidad de la
//! muestra en texto grande + código de barras Code 128 (para escáner), datos del
//! paciente, tipo de muestra, fecha de recepción y responsable. Se imprimen y se
//! pegan al tubo al recepcionar para evitar errores de identificación cruzada.

use std::path::Path;

use crate::models::sample_list_item::SampleListItem;
use crate::pdf_templates::builder::{
    sanitize, save_pdf, text_right, PdfBuilder, C_MUTED, C_RULE, C_TEXT, PAGE_H, PAGE_W,
};
use crate::pdf_templates::layout::{code128_width, draw_code128};

/// Margen exterior de la página (mm).
const PAGE_MARGIN: f32 = 10.0;
/// Separación horizontal entre etiquetas (mm).
const GAP_X: f32 = 6.0;
/// Separación vertical entre etiquetas (mm).
const GAP_Y: f32 = 8.0;
/// Columnas y filas de la grilla.
const COLS: usize = 2;
const ROWS: usize = 4;

/// Colores por estado de la muestra (barra superior de la etiqueta).
fn status_color(status: &str) -> (u8, u8, u8) {
    match status {
        "RECIBIDA" => (40, 130, 70),   // verde
        "EN_PROCESO" => (190, 110, 0), // ámbar
        "FINALIZADA" => (0, 100, 180), // azul
        "ANULADA" => (190, 45, 45),    // rojo
        _ => C_MUTED,
    }
}

fn status_label(status: &str) -> String {
    match status {
        "RECIBIDA" => "RECIBIDA".to_string(),
        "EN_PROCESO" => "EN PROCESO".to_string(),
        "FINALIZADA" => "FINALIZADA".to_string(),
        "ANULADA" => "ANULADA".to_string(),
        _ => status.to_string(),
    }
}

/// Dibuja una etiqueta en la celda con esquina superior izquierda `(x, y)`.
/// `y` crece hacia abajo en el espacio del PdfBuilder.
fn draw_label(pdf: &mut PdfBuilder, s: &SampleListItem, x: f32, y: f32, w: f32, h: f32) {
    // Marco de corte (línea fina para guillotinar).
    pdf.rect(x, y, w, h, None, Some(C_RULE));

    // Barra de estado superior.
    let bar_h = 6.0;
    pdf.rect(x, y, w, bar_h, Some(status_color(&s.status)), None);
    pdf.text(true, "ISALAB", 7.0, x + 3.0, y - 4.4, (255, 255, 255));
    text_right(
        pdf,
        true,
        &status_label(&s.status),
        7.0,
        x + w - 3.0,
        y - 4.4,
        (255, 255, 255),
    );

    // Código de la muestra (grande, legible).
    pdf.text(true, &sanitize(&s.code), 16.0, x + 3.0, y - 13.0, C_TEXT);

    // Código de barras centrado.
    let barcode_h = 16.0;
    if let Some(bw) = code128_width(&s.code, 0.33) {
        draw_code128(pdf, &s.code, x + (w - bw) / 2.0, y - 18.0, barcode_h, 0.33);
    }

    // Datos del paciente.
    pdf.text(
        true,
        &sanitize(&s.patient_name),
        11.0,
        x + 3.0,
        y - 38.0,
        C_TEXT,
    );
    let mut line2 = s.species_name.clone();
    if !line2.is_empty() {
        line2.push_str(" · ");
    }
    line2.push_str(&s.sample_type_name);
    pdf.text(false, &sanitize(&line2), 8.5, x + 3.0, y - 43.5, C_MUTED);

    // Fecha de recepción y responsable.
    let date = s.received_at.split(' ').next().unwrap_or(&s.received_at);
    let collected = s
        .collected_by
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .map(|c| format!("Responsable: {c}"))
        .unwrap_or_default();
    let mut line3 = format!("Recibida: {date}");
    if !collected.is_empty() {
        line3.push_str("  ·  ");
        line3.push_str(&collected);
    }
    pdf.text(false, &sanitize(&line3), 7.5, x + 3.0, y - 49.0, C_TEXT);

    // Separador inferior + propietario.
    let owner = if s.owner_name.is_empty() {
        String::new()
    } else {
        format!("Propietario: {}", s.owner_name)
    };
    pdf.text(false, &sanitize(&owner), 7.0, x + 3.0, y - h + 4.0, C_MUTED);
}

/// Genera la hoja de etiquetas para las muestras dadas y la guarda en
/// `out_path`. Reordena las muestras por fecha de recepción (más reciente
/// primero) y las reparte en páginas de 8.
pub fn generate_sample_labels(samples: &[SampleListItem], out_path: &Path) -> Result<(), String> {
    if samples.is_empty() {
        return Err("No hay muestras para etiquetar".into());
    }

    let mut pdf = PdfBuilder::new();
    pdf.y = PAGE_H - PAGE_MARGIN;

    let usable_w = PAGE_W - PAGE_MARGIN * 2.0;
    let usable_h = PAGE_H - PAGE_MARGIN * 2.0;
    let label_w = (usable_w - GAP_X * (COLS as f32 - 1.0)) / COLS as f32;
    let label_h = (usable_h - GAP_Y * (ROWS as f32 - 1.0)) / ROWS as f32;

    let per_page = COLS * ROWS;
    let mut placed = 0;
    for s in samples {
        let idx = placed % per_page;
        if idx == 0 && placed > 0 {
            pdf.new_page();
            pdf.y = PAGE_H - PAGE_MARGIN;
        }
        let col = (idx % COLS) as f32;
        let row = (idx / COLS) as f32;
        let x = PAGE_MARGIN + col * (label_w + GAP_X);
        let y = PAGE_MARGIN + row * (label_h + GAP_Y);
        draw_label(&mut pdf, s, x, y, label_w, label_h);
        placed += 1;
    }

    save_pdf(pdf, out_path, "Etiquetas de muestras")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(code: &str, status: &str) -> SampleListItem {
        SampleListItem {
            id: 1,
            code: code.into(),
            patient_id: 1,
            patient_name: "Luna".into(),
            owner_name: "Juan Pérez".into(),
            species_name: "Canino".into(),
            sample_type_id: 1,
            sample_type_name: "Sangre total (EDTA)".into(),
            received_at: "2026-08-01 10:30:00".into(),
            status: status.into(),
            collected_by: Some("Dr. Ramos".into()),
            notes: None,
            result_count: 0,
            abnormal_count: 0,
        }
    }

    #[test]
    fn test_status_label_mapping() {
        assert_eq!(status_label("EN_PROCESO"), "EN PROCESO");
        assert_eq!(status_label("FINALIZADA"), "FINALIZADA");
        assert_eq!(status_label("desconocido"), "desconocido");
    }

    #[test]
    fn test_generate_single_label_creates_file() {
        let tmp = std::env::temp_dir().join(format!("isalab-label-{}.pdf", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        let result = generate_sample_labels(&[sample("M-2026-0001", "RECIBIDA")], &tmp);
        assert!(result.is_ok(), "{result:?}");
        let meta = std::fs::metadata(&tmp).expect("pdf debe existir");
        assert!(meta.len() > 500);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_generate_empty_labels_errors() {
        let tmp = std::env::temp_dir().join("isalab-label-empty.pdf");
        let result = generate_sample_labels(&[], &tmp);
        assert!(result.is_err());
    }

    #[test]
    fn test_label_geometry_fits_page() {
        let usable_w = PAGE_W - PAGE_MARGIN * 2.0;
        let usable_h = PAGE_H - PAGE_MARGIN * 2.0;
        let label_w = (usable_w - GAP_X) / 2.0;
        let label_h = (usable_h - GAP_Y * 3.0) / 4.0;
        // 2 columnas no se solapan.
        assert!(2.0 * label_w + GAP_X <= usable_w + 0.01);
        // 4 filas no se solapan.
        assert!(4.0 * label_h + GAP_Y * 3.0 <= usable_h + 0.01);
        // Proporción razonable de etiqueta adhesiva.
        assert!(label_w > 60.0 && label_h > 30.0);
    }
}
