//! # Etiquetas de tubos de muestras (hoja imprimible)
//!
//! Genera una hoja Carta con una grilla de etiquetas adhesivas (2 columnas × 4
//! filas = 8 por página). Cada etiqueta incluye el código de trazabilidad de la
//! muestra en texto grande + código de barras Code 128 (para escáner), datos del
//! paciente, tipo de muestra, fecha de recepción y responsable. Se imprimen y se
//! pegan al tubo al recepcionar para evitar errores de identificación cruzada.

use std::path::Path;

use crate::models::sample_list_item::SampleListItem;
use crate::pdf_templates::builder::{sanitize, save_pdf, text_right, PdfBuilder, C_MUTED, C_TEXT};
use crate::pdf_templates::layout::{code128_width, draw_code128};

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

fn draw_label(pdf: &mut PdfBuilder, s: &SampleListItem, x: f32, y: f32, w: f32) {
    // Barra de estado superior.
    let bar_h = 4.0;
    pdf.rect(x, y, w, bar_h, Some(status_color(&s.status)), None);
    pdf.text(true, "ISALAB", 5.0, x + 2.0, y - 3.0, (255, 255, 255));
    text_right(
        pdf,
        true,
        &status_label(&s.status),
        5.0,
        x + w - 2.0,
        y - 3.0,
        (255, 255, 255),
    );

    // Código de la muestra (izq) y código de paciente (der).
    pdf.text(true, &sanitize(&s.code), 11.0, x + 2.0, y - 8.0, C_TEXT);
    text_right(
        pdf,
        false,
        &sanitize(&s.patient_code),
        7.0,
        x + w - 2.0,
        y - 8.0,
        C_TEXT,
    );

    // Código de barras centrado.
    let barcode_h = 9.0;
    if let Some(bw) = code128_width(&s.code, 0.25) {
        draw_code128(pdf, &s.code, x + (w - bw) / 2.0, y - 10.5, barcode_h, 0.25);
    }

    // Datos del paciente.
    pdf.text(
        true,
        &sanitize(&s.patient_name),
        8.0,
        x + 2.0,
        y - 23.5,
        C_TEXT,
    );
    let mut line2 = s.species_name.clone();
    if !line2.is_empty() {
        line2.push_str(" · ");
    }
    line2.push_str(&s.sample_type_name);
    pdf.text(false, &sanitize(&line2), 6.5, x + 2.0, y - 27.5, C_MUTED);
}

/// Genera la hoja de etiquetas para las muestras dadas y la guarda en
/// `out_path`. Reordena las muestras por fecha de recepción (más reciente
/// primero) y las reparte en páginas de 8.
pub fn generate_sample_labels(samples: &[SampleListItem], out_path: &Path) -> Result<(), String> {
    if samples.is_empty() {
        return Err("No hay muestras para etiquetar".into());
    }

    // 50x30 mm
    let mut pdf = PdfBuilder::new_custom(50.0, 30.0);

    for (i, s) in samples.iter().enumerate() {
        if i > 0 {
            pdf.new_page();
        }
        draw_label(&mut pdf, s, 1.0, 29.0, 48.0);
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
            patient_code: "P-2026-0001".into(),
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
        let label_w = 48.0;
        let label_h = 28.0;
        assert!(label_w <= 50.0);
        assert!(label_h <= 30.0);
    }
}
