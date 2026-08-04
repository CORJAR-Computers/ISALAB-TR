use std::path::Path;

use printpdf::{
    BuiltinFont, Color, Line, LinePoint, Mm, Op, PaintMode, PdfDocument, PdfFontHandle,
    PdfPage, PdfSaveOptions, Point, Polygon, PolygonRing, Pt, RawImage, Rgb,
    TextItem, WindingOrder, XObject, XObjectId,
};

use crate::pdf_templates::header::{
    LEFT_LOGO_XOBJECT, RIGHT_LOGO_XOBJECT, WATERMARK_LOGO_XOBJECT,
};
pub use crate::pdf_templates::header::LOGO_W_MM;

/// Ancho/alto Carta (US Letter: 8.5" x 11" = 215.9mm x 279.4mm) y márgenes (mm).
/// printpdf usa el origen en la esquina inferior izquierda (y crece hacia arriba).
pub const PAGE_W: f32 = 215.9;
pub const PAGE_H: f32 = 279.4;
pub const MARGIN: f32 = 15.0;
pub const CONTENT_W: f32 = PAGE_W - MARGIN * 2.0; // 185.9

/// Factor de conversión mm → pt (1 pt = 25.4/72 mm).
pub const MM_TO_PT: f32 = 72.0 / 25.4;

// ---- Paleta (impresión en negro + acentos clínicos) ----
pub const C_TEXT: (u8, u8, u8) = (30, 30, 30);
pub const C_MUTED: (u8, u8, u8) = (110, 110, 110);
pub const C_RULE: (u8, u8, u8) = (190, 190, 190);
pub const C_HEADER_BG: (u8, u8, u8) = (232, 240, 238);
pub const C_NORMAL: (u8, u8, u8) = (40, 130, 70);
pub const C_ALTO: (u8, u8, u8) = (190, 110, 0);
pub const C_BAJO: (u8, u8, u8) = (190, 45, 45);
pub const C_SIN_RANGO: (u8, u8, u8) = (120, 120, 120);

pub const FONT: PdfFontHandle = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
pub const FONT_BOLD: PdfFontHandle = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);

/// Convierte una tupla RGB (0-255) al color normalizado de printpdf (0-1).
pub fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(Rgb {
        r: f32::from(c.0) / 255.0,
        g: f32::from(c.1) / 255.0,
        b: f32::from(c.2) / 255.0,
        icc_profile: None,
    })
}

/// Reemplaza caracteres fuera de WinAnsi/Latin-1 (las flechas, comillas
/// tipográficas, etc. romperían la fuente estándar Helvetica).
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{2013}' | '\u{2014}' => out.push('-'),
            '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' => out.push('\''),
            '\u{2026}' => out.push_str("..."),
            '\u{2191}' | '\u{2193}' | '\u{2192}' | '\u{2190}' => {}
            '·' | '•' => out.push_str(" - "),
            c if c.is_control() || (c as u32) > 0xFF => out.push('?'),
            c => out.push(c),
        }
    }
    out
}

/// Ancho aproximado de un texto en mm (para centrar títulos). Helvetica:
/// ~0.5em de ancho medio por carácter.
pub fn text_width_mm(text: &str, size_pt: f32) -> f32 {
    let chars = text.chars().filter(|c| !c.is_control()).count() as f32;
    size_pt * 0.5 * chars * 0.352778
}

/// Etiqueta corta para estados clínicos.
pub fn status_label(status: &str) -> &'static str {
    match status {
        "ALTO" => "ALTO",
        "BAJO" => "BAJO",
        "NORMAL" => "NORMAL",
        _ => "SIN RANGO",
    }
}

/// Color del estado (también aplicado al valor).
pub fn status_color(status: &str) -> (u8, u8, u8) {
    match status {
        "ALTO" => C_ALTO,
        "BAJO" => C_BAJO,
        "NORMAL" => C_NORMAL,
        _ => C_SIN_RANGO,
    }
}

/// Acumula operaciones printpdf por página (Carta, cursor vertical en mm).
pub struct PdfBuilder {
    pub pages: Vec<PdfPage>,
    pub ops: Vec<Op>,
    pub y: f32,
    /// Logo institucional de ISALAB (esquina superior izquierda).
    pub left_logo: Option<RawImage>,
    /// Logo de la empresa/clínica cliente (esquina superior derecha).
    pub right_logo: Option<RawImage>,
    /// Logo marca de agua suavizado (centro de la página).
    pub watermark_logo: Option<RawImage>,
}

impl PdfBuilder {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            ops: Vec::new(),
            y: PAGE_H - MARGIN,
            left_logo: None,
            right_logo: None,
            watermark_logo: None,
        }
    }

    pub fn flush(&mut self) {
        self.pages.push(PdfPage::new(
            Mm(PAGE_W),
            Mm(PAGE_H),
            std::mem::take(&mut self.ops),
        ));
    }

    pub fn new_page(&mut self) {
        self.flush();
        self.y = PAGE_H - MARGIN;
    }

    pub fn ensure_space(&mut self, height_mm: f32) {
        if self.y - height_mm < MARGIN {
            self.new_page();
        }
    }

    pub fn text(
        &mut self,
        bold: bool,
        text: &str,
        size_pt: f32,
        x_mm: f32,
        y_mm: f32,
        color: (u8, u8, u8),
    ) {
        self.ops.push(Op::StartTextSection);
        self.ops.push(Op::SetTextCursor {
            pos: Point::new(Mm(x_mm), Mm(y_mm)),
        });
        self.ops.push(Op::SetFillColor { col: rgb(color) });
        self.ops.push(Op::SetFont {
            font: if bold { FONT_BOLD } else { FONT },
            size: Pt(size_pt),
        });
        self.ops.push(Op::SetLineHeight {
            lh: Pt(size_pt * 1.2),
        });
        self.ops
            .push(Op::ShowText { items: vec![TextItem::Text(sanitize(text))] });
        self.ops.push(Op::EndTextSection);
    }

    pub fn text_centered(&mut self, bold: bool, text: &str, size_pt: f32, y_mm: f32, color: (u8, u8, u8)) {
        let w = text_width_mm(text, size_pt);
        self.text(bold, text, size_pt, (PAGE_W - w) / 2.0, y_mm, color);
    }

    pub fn rule(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: (u8, u8, u8)) {
        self.ops.push(Op::SetOutlineColor { col: rgb(color) });
        self.ops.push(Op::SetOutlineThickness { pt: Pt(0.4) });
        self.ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint {
                        p: Point::new(Mm(x1), Mm(y1)),
                        bezier: false,
                    },
                    LinePoint {
                        p: Point::new(Mm(x2), Mm(y2)),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        });
    }

    pub fn rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Option<(u8, u8, u8)>,
        outline: Option<(u8, u8, u8)>,
    ) {
        let mode = match (fill.is_some(), outline.is_some()) {
            (true, true) => PaintMode::FillStroke,
            (true, false) => PaintMode::Fill,
            _ => PaintMode::Stroke,
        };
        if let Some(c) = fill {
            self.ops.push(Op::SetFillColor { col: rgb(c) });
        }
        if let Some(c) = outline {
            self.ops.push(Op::SetOutlineColor { col: rgb(c) });
            self.ops.push(Op::SetOutlineThickness { pt: Pt(0.4) });
        }
        self.ops.push(Op::DrawPolygon {
            polygon: Polygon {
                rings: vec![PolygonRing {
                    points: vec![
                        LinePoint {
                            p: Point::new(Mm(x), Mm(y)),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(Mm(x + w), Mm(y)),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(Mm(x + w), Mm(y - h)),
                            bezier: false,
                        },
                        LinePoint {
                            p: Point::new(Mm(x), Mm(y - h)),
                            bezier: false,
                        },
                    ],
                }],
                mode,
                winding_order: WindingOrder::NonZero,
            },
        });
    }

    pub fn finish(mut self) -> Vec<PdfPage> {
        self.flush();
        self.pages
    }
}

/// Guarda las páginas compiladas en `out_path`, incrustando los logos en los recursos del documento.
pub fn save_pdf(mut pdf: PdfBuilder, out_path: &Path, doc_title: &str) -> Result<(), String> {
    let mut doc = PdfDocument::new(doc_title);
    if let Some(logo) = pdf.left_logo.take() {
        doc.resources.xobjects.map.insert(
            XObjectId(LEFT_LOGO_XOBJECT.to_string()),
            XObject::Image(logo),
        );
    }
    if let Some(logo) = pdf.right_logo.take() {
        doc.resources.xobjects.map.insert(
            XObjectId(RIGHT_LOGO_XOBJECT.to_string()),
            XObject::Image(logo),
        );
    }
    if let Some(logo) = pdf.watermark_logo.take() {
        doc.resources.xobjects.map.insert(
            XObjectId(WATERMARK_LOGO_XOBJECT.to_string()),
            XObject::Image(logo),
        );
    }
    let bytes = doc
        .with_pages(pdf.finish())
        .save(&PdfSaveOptions::default(), &mut Vec::new());

    std::fs::write(out_path, bytes).map_err(|e| {
        format!(
            "No se pudo crear el PDF en {}: {e}",
            out_path.display()
        )
    })?;
    Ok(())
}

pub fn format_value(v: f64) -> String {
    if v == v.round() {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Divide un texto en líneas de ≤ `max_chars` respetando saltos de línea.
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.split('\n') {
        let mut current = String::new();
        for word in raw.split(' ') {
            if !current.is_empty() && current.len() + word.len() + 1 > max_chars {
                lines.push(current.clone());
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.trim().is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Dibuja texto multilínea (con ajuste a 95 caracteres) y devuelve el nuevo y.
pub fn draw_multiline(
    pdf: &mut PdfBuilder,
    text: &str,
    size_pt: f32,
    x: f32,
    color: (u8, u8, u8),
    line_h: f32,
) -> f32 {
    for line in wrap_text(text, 95) {
        pdf.ensure_space(line_h + 1.0);
        pdf.text(false, &sanitize(&line), size_pt, x, pdf.y, color);
        pdf.y -= line_h;
    }
    pdf.y
}

/// Texto alineado a la derecha a partir de `x_right` (borde derecho).
pub fn text_right(
    pdf: &mut PdfBuilder,
    bold: bool,
    text: &str,
    size_pt: f32,
    x_right: f32,
    y: f32,
    color: (u8, u8, u8),
) {
    let w = text_width_mm(text, size_pt);
    pdf.text(bold, text, size_pt, x_right - w, y, color);
}

/// Trunca un texto a `max_chars` añadiendo "...".
pub fn truncate(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

/// Formatea un monto como COP (separador de miles ".", decimales ",").
pub fn format_cop(v: f64) -> String {
    let negative = v < 0.0;
    let abs = v.abs();
    let whole = abs.floor() as i64;
    let cents = ((abs - whole as f64) * 100.0).round() as i64;
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    let sign = if negative { "-" } else { "" };
    format!("{sign}${out},{cents:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- sanitize ----

    #[test]
    fn test_sanitize_em_dash() {
        assert_eq!(sanitize("Muestra – Test"), "Muestra - Test");
    }

    #[test]
    fn test_sanitize_middle_dot() {
        assert_eq!(sanitize("· Punto"), " -  Punto");
    }

    #[test]
    fn test_sanitize_typographic_quotes() {
        assert_eq!(sanitize("'hola'"), "'hola'");
        assert_eq!(sanitize("\u{201C}test\u{201D}"), "'test'");
    }

    #[test]
    fn test_sanitize_ellipsis() {
        assert_eq!(sanitize("esperando…"), "esperando...");
    }

    #[test]
    fn test_sanitize_removes_arrows() {
        assert_eq!(sanitize("→"), "");
        assert_eq!(sanitize("↑↓←"), "");
    }

    #[test]
    fn test_sanitize_control_chars() {
        assert_eq!(sanitize("a\x00b"), "a?b");
    }

    // ---- format_value ----

    #[test]
    fn test_format_value_integer() {
        assert_eq!(format_value(12.0), "12");
    }

    #[test]
    fn test_format_value_decimal() {
        assert_eq!(format_value(12.567), "12.57");
    }

    #[test]
    fn test_format_value_zero() {
        assert_eq!(format_value(0.0), "0");
    }

    #[test]
    fn test_format_value_exact_half() {
        assert_eq!(format_value(3.50), "3.50");
    }

    // ---- format_cop ----

    #[test]
    fn test_format_cop_millions() {
        assert_eq!(format_cop(1500000.0), "$1.500.000,00");
    }

    #[test]
    fn test_format_cop_with_cents() {
        assert_eq!(format_cop(4500.5), "$4.500,50");
    }

    #[test]
    fn test_format_cop_zero() {
        assert_eq!(format_cop(0.0), "$0,00");
    }

    #[test]
    fn test_format_cop_small() {
        assert_eq!(format_cop(99.99), "$99,99");
    }

    #[test]
    fn test_format_cop_negative() {
        assert_eq!(format_cop(-1500.0), "-$1.500,00");
    }

    // ---- status_label / status_color ----

    #[test]
    fn test_status_label_and_color() {
        assert_eq!(status_label("ALTO"), "ALTO");
        assert_eq!(status_label("BAJO"), "BAJO");
        assert_eq!(status_label("NORMAL"), "NORMAL");
        assert_eq!(status_label("DESCONOCIDO"), "SIN RANGO");
        assert_eq!(status_color("ALTO"), C_ALTO);
        assert_eq!(status_color("BAJO"), C_BAJO);
        assert_eq!(status_color("NORMAL"), C_NORMAL);
    }

    // ---- truncate ----

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hola", 10), "hola");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("12345", 5), "12345");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("abcdefghij", 5), "abcde...");
    }

    // ---- wrap_text ----

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("hola mundo", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hola mundo");
    }

    #[test]
    fn test_wrap_text_newline() {
        let lines = wrap_text("línea 1\nlínea 2", 80);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "línea 1");
        assert_eq!(lines[1], "línea 2");
    }

    #[test]
    fn test_wrap_text_long_line() {
        let text = "a ".repeat(50);
        let lines = wrap_text(&text, 20);
        assert!(lines.len() > 1);
    }

    #[test]
    fn test_wrap_text_empty() {
        let lines = wrap_text("", 80);
        assert_eq!(lines.len(), 1);
    }

    // ---- text_width_mm ----

    #[test]
    fn test_text_width_mm_positive() {
        let w = text_width_mm("Test", 12.0);
        assert!(w > 0.0);
    }

    #[test]
    fn test_text_width_mm_empty() {
        let w = text_width_mm("", 12.0);
        assert!((w - 0.0).abs() < f32::EPSILON);
    }

    // ---- rgb ----

    #[test]
    fn test_rgb_black() {
        let c = rgb((0, 0, 0));
        if let Color::Rgb(rgb) = c {
            assert!((rgb.r - 0.0).abs() < f32::EPSILON);
            assert!((rgb.g - 0.0).abs() < f32::EPSILON);
            assert!((rgb.b - 0.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected Rgb color");
        }
    }

    #[test]
    fn test_rgb_white() {
        let c = rgb((255, 255, 255));
        if let Color::Rgb(rgb) = c {
            assert!((rgb.r - 1.0).abs() < 0.01);
            assert!((rgb.g - 1.0).abs() < 0.01);
            assert!((rgb.b - 1.0).abs() < 0.01);
        } else {
            panic!("Expected Rgb color");
        }
    }

    // ---- PdfBuilder ----

    #[test]
    fn test_pdf_builder_new_starts_at_margin() {
        let pdf = PdfBuilder::new();
        assert!((pdf.y - (PAGE_H - MARGIN)).abs() < f32::EPSILON);
        assert!(pdf.ops.is_empty());
        assert!(pdf.pages.is_empty());
    }
}
