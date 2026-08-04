use printpdf::{Op, PdfWarnMsg, Pt, RawImage, XObjectId, XObjectTransform};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::pdf_templates::builder::{
    PdfBuilder, C_MUTED, C_TEXT, MARGIN, MM_TO_PT, PAGE_H, PAGE_W,
};

/// Ancho estándar del logo en el encabezado (mm); la altura se deriva del ratio.
pub const LOGO_W_MM: f32 = 42.0;

/// Nombres de los XObjects de los logos en los recursos del PDF.
pub const LEFT_LOGO_XOBJECT: &str = "ISALAB-LEFT-LOGO";
pub const RIGHT_LOGO_XOBJECT: &str = "ISALAB-RIGHT-LOGO";
pub const WATERMARK_LOGO_XOBJECT: &str = "ISALAB-WATERMARK-LOGO";

/// Logo por defecto incrustado en el binario (logo_sidebar.png de la raíz).
pub const DEFAULT_LOGO: &[u8] = include_bytes!("../../../logo_sidebar.png");

/// Datos de la clínica para el encabezado del informe.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClinicHeader {
    pub name: String,
    pub nit: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub city: Option<String>,
    pub logo_path: Option<String>,
}

/// Carga el logo izquierdo de ISALAB: prueba archivos locales de la app o el incrustado.
pub fn load_left_logo() -> Option<RawImage> {
    let decode = |bytes: &[u8]| -> Option<RawImage> {
        let mut warnings: Vec<PdfWarnMsg> = Vec::new();
        RawImage::decode_from_bytes(bytes, &mut warnings).ok()
    };

    for candidate in &[
        "dist/isalab.png",
        "isalab.png",
        "isalab_icon.png",
        "dist/icon.png",
    ] {
        if let Ok(bytes) = std::fs::read(candidate) {
            if let Some(img) = decode(&bytes) {
                return Some(img);
            }
        }
    }
    decode(DEFAULT_LOGO)
}

/// Carga el logo de ISALAB y le aplica un suavizado ultrasuave (~7% de intensidad) para la marca de agua.
pub fn load_watermark_logo() -> Option<RawImage> {
    let mut img = load_left_logo()?;
    let factor = 0.09f32; // 9% de opacidad para que sea ultrasuave y no interfiera con el texto

    if let printpdf::RawImageData::U8(ref mut data) = img.pixels {
        let total_pixels = img.width * img.height;
        if total_pixels > 0 && !data.is_empty() {
            let chunk_size = data.len() / total_pixels;

            if chunk_size == 3 || chunk_size == 4 {
                for pixel in data.chunks_exact_mut(chunk_size) {
                    let r = pixel[0] as f32;
                    let g = pixel[1] as f32;
                    let b = pixel[2] as f32;

                    if chunk_size == 4 {
                        let a = pixel[3];
                        if a < 20 {
                            // Mantener pixel transparente
                            pixel[0] = 255;
                            pixel[1] = 255;
                            pixel[2] = 255;
                            pixel[3] = 0;
                            continue;
                        }
                    }

                    // Blending suave de RGB hacia blanco puro (255)
                    pixel[0] = (255.0 - (255.0 - r) * factor).round() as u8;
                    pixel[1] = (255.0 - (255.0 - g) * factor).round() as u8;
                    pixel[2] = (255.0 - (255.0 - b) * factor).round() as u8;
                }
            }
        }
    }

    Some(img)
}

/// Carga el logo derecho (empresa cliente): primero `clinic.logo_path` y luego archivos en dist.
pub fn load_right_logo(clinic: &ClinicHeader) -> Option<RawImage> {
    let decode = |bytes: &[u8]| -> Option<RawImage> {
        let mut warnings: Vec<PdfWarnMsg> = Vec::new();
        RawImage::decode_from_bytes(bytes, &mut warnings).ok()
    };

    if let Some(path) = clinic.logo_path.as_deref().filter(|p| !p.is_empty()) {
        if let Ok(bytes) = std::fs::read(path) {
            if let Some(img) = decode(&bytes) {
                return Some(img);
            }
        }
    }

    for candidate in &["dist/cv_ruffos_house.png", "cv_ruffos_house.png"] {
        if let Ok(bytes) = std::fs::read(candidate) {
            if let Some(img) = decode(&bytes) {
                return Some(img);
            }
        }
    }
    None
}

/// Dibuja una marca de agua central sutil con el logo institucional de ISALAB.
pub fn draw_watermark(pdf: &mut PdfBuilder) {
    if pdf.watermark_logo.is_none() {
        pdf.watermark_logo = load_watermark_logo();
    }
    if let Some(logo) = &pdf.watermark_logo {
        let (w_px, h_px) = (logo.width as f32, logo.height as f32);
        let wm_w_mm = 100.0f32;
        let wm_h_mm = if w_px > 0.0 {
            (wm_w_mm * h_px / w_px).min(95.0)
        } else {
            wm_w_mm
        };
        let x_center = (PAGE_W - wm_w_mm) / 2.0;
        let y_center = (PAGE_H - wm_h_mm) / 2.0;

        pdf.ops.push(Op::UseXobject {
            id: XObjectId(WATERMARK_LOGO_XOBJECT.to_string()),
            transform: XObjectTransform {
                translate_x: Some(Pt(x_center * MM_TO_PT)),
                translate_y: Some(Pt(y_center * MM_TO_PT)),
                scale_x: Some(wm_w_mm * MM_TO_PT),
                scale_y: Some(wm_h_mm * MM_TO_PT),
                no_auto_scale: true,
                ..Default::default()
            },
        });
    }
}

/// Encabezado institucional: Logo ISALAB a la izquierda, datos de ISALAB en el centro,
/// logo de la empresa a la derecha, y título del informe.
pub fn draw_header(
    pdf: &mut PdfBuilder,
    clinic: &ClinicHeader,
    title: &str,
    subtitle: &str,
    extra: Option<&str>,
) {
    draw_watermark(pdf);

    let start_y = PAGE_H - 14.0;
    pdf.y = start_y;

    if pdf.left_logo.is_none() {
        pdf.left_logo = load_left_logo();
    }
    if pdf.right_logo.is_none() {
        pdf.right_logo = load_right_logo(clinic);
    }

    let mut left_h_mm = 0.0f32;
    let mut right_h_mm = 0.0f32;

    // 1. Logo izquierdo (ISALAB)
    if let Some(logo) = &pdf.left_logo {
        let (w_px, h_px) = (logo.width as f32, logo.height as f32);
        left_h_mm = if w_px > 0.0 {
            (LOGO_W_MM * h_px / w_px).min(22.0)
        } else {
            LOGO_W_MM
        };
        let x_left = MARGIN;
        let y_bottom = start_y - left_h_mm;
        pdf.ops.push(Op::UseXobject {
            id: XObjectId(LEFT_LOGO_XOBJECT.to_string()),
            transform: XObjectTransform {
                translate_x: Some(Pt(x_left * MM_TO_PT)),
                translate_y: Some(Pt(y_bottom * MM_TO_PT)),
                scale_x: Some(LOGO_W_MM * MM_TO_PT),
                scale_y: Some(left_h_mm * MM_TO_PT),
                no_auto_scale: true,
                ..Default::default()
            },
        });
    }

    // 2. Logo derecho (Empresa cliente / cv_ruffos_house.png)
    if let Some(logo) = &pdf.right_logo {
        let (w_px, h_px) = (logo.width as f32, logo.height as f32);
        right_h_mm = if w_px > 0.0 {
            (LOGO_W_MM * h_px / w_px).min(22.0)
        } else {
            LOGO_W_MM
        };
        let x_right = PAGE_W - MARGIN - LOGO_W_MM;
        let y_bottom = start_y - right_h_mm;
        pdf.ops.push(Op::UseXobject {
            id: XObjectId(RIGHT_LOGO_XOBJECT.to_string()),
            transform: XObjectTransform {
                translate_x: Some(Pt(x_right * MM_TO_PT)),
                translate_y: Some(Pt(y_bottom * MM_TO_PT)),
                scale_x: Some(LOGO_W_MM * MM_TO_PT),
                scale_y: Some(right_h_mm * MM_TO_PT),
                no_auto_scale: true,
                ..Default::default()
            },
        });
    }

    // 3. Datos de ISALAB en el centro
    let mut text_y = start_y - 2.0;
    pdf.text_centered(true, &clinic.name, 13.0, text_y, C_TEXT);

    let meta = format!(
        "NIT {nit}{city}",
        nit = clinic.nit,
        city = clinic
            .city
            .as_deref()
            .map(|c| format!(" · {c}"))
            .unwrap_or_default()
    );
    text_y -= 5.5;
    pdf.text_centered(false, &meta, 8.5, text_y, C_MUTED);

    let contact: Vec<String> = [clinic.address.as_deref(), clinic.phone.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if !contact.is_empty() {
        text_y -= 4.5;
        pdf.text_centered(false, &contact.join(" · "), 8.5, text_y, C_MUTED);
    }

    let max_h = left_h_mm.max(right_h_mm).max(start_y - text_y + 2.0);
    pdf.y = start_y - max_h - 2.0;

    // Título principal en azul cian (#0082C8) centrado
    let title_color = (0, 130, 200);
    pdf.text_centered(true, title, 13.5, pdf.y, title_color);
    pdf.y -= 5.0;
    if !subtitle.is_empty() {
        pdf.text_centered(false, subtitle, 8.5, pdf.y, C_TEXT);
        pdf.y -= 4.5;
    }
    if let Some(line) = extra {
        pdf.text_centered(false, line, 8.0, pdf.y, C_MUTED);
        pdf.y -= 4.5;
    }
    pdf.y -= 2.0;
}
