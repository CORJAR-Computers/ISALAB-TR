//! # Reportes médicos PDF (generación server-side en Rust)
//!
//! Implementado con `printpdf` 0.12 (API de operaciones: `Op`): compone el
//! informe A4 (encabezado con datos de la clínica, ficha del paciente, tabla
//! de resultados con valores fuera de rango resaltados y bloque de firma) y lo
//! guarda en `app_data/reports/<codigo>.pdf`. El frontend solo recibe la ruta;
//! nunca se genera PDF en el navegador.
//!
//! Nota de alcance: la firma se renderiza como bloque de texto. La firma
//! digital PKCS#12 (`DIGITAL`) devuelve un error explícito por ahora.

use std::path::Path;

use printpdf::{
    BuiltinFont, Color, Line, LinePoint, Mm, Op, PaintMode, PdfDocument, PdfFontHandle,
    PdfPage, PdfSaveOptions, PdfWarnMsg, Point, Polygon, PolygonRing, Pt, RawImage, Rgb,
    TextItem, WindingOrder, XObject, XObjectId, XObjectTransform,
};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::invoice::Invoice;
use crate::models::owner::Owner;
use crate::models::patient::Patient;
use crate::models::sample::LabResult;
use crate::models::surgery::Surgery;
use crate::models::vaccine::Vaccine;

/// Configuración de firma para reportes.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportSignature {
    /// GRAPHIC (imagen de firma) o DIGITAL (certificado PKCS#12).
    pub mode: String,
    pub vet_name: String,
    pub vet_license: Option<String>,
    pub signature_image_path: Option<String>,
    pub pkcs12_path: Option<String>,
}

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
    /// YYYY-MM-DD HH:MM:SS
    pub date: String,
    pub reason: String,
    pub diagnosis: Option<String>,
    /// Tratamiento prescrito (medicamentos y dosis).
    pub medication: Option<String>,
    pub signature: ReportSignature,
}

/// Datos del consentimiento informado (procedimiento quirúrgico).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentimientoData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<Owner>,
    /// Código de atención (id de la cirugía).
    pub attention_code: String,
    pub procedure_type: String,
    /// YYYY-MM-DD HH:MM:SS
    pub procedure_date: String,
    pub description: Option<String>,
    pub post_care: Option<String>,
    pub veterinarian: String,
}

/// Datos del comprobante de pago (recibo de factura).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReciboData {
    pub clinic: ClinicHeader,
    pub invoice: Invoice,
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

/// Datos del certificado/carnet de vacunación.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VacunacionData {
    pub clinic: ClinicHeader,
    pub patient: Patient,
    pub owner: Option<Owner>,
    pub vaccines: Vec<Vaccine>,
    pub certificate_number: String,
    /// YYYY-MM-DD HH:MM:SS
    pub issued_at: String,
    pub veterinarian: String,
}

/// Ancho/alto Carta (US Letter: 8.5" x 11" = 215.9mm x 279.4mm) y márgenes (mm).
/// printpdf usa el origen en la esquina inferior izquierda (y crece hacia arriba).
const PAGE_W: f32 = 215.9;
const PAGE_H: f32 = 279.4;
const MARGIN: f32 = 15.0;
const CONTENT_W: f32 = PAGE_W - MARGIN * 2.0; // 185.9

/// Nombres de los XObjects de los logos en los recursos del PDF.
const LEFT_LOGO_XOBJECT: &str = "ISALAB-LEFT-LOGO";
const RIGHT_LOGO_XOBJECT: &str = "ISALAB-RIGHT-LOGO";
/// Factor de conversión mm → pt (1 pt = 25.4/72 mm).
const MM_TO_PT: f32 = 72.0 / 25.4;
/// Ancho estándar del logo en el encabezado (mm); la altura se deriva del ratio.
const LOGO_W_MM: f32 = 30.0;
/// Logo por defecto incrustado en el binario (logo_sidebar.png de la raíz).
const DEFAULT_LOGO: &[u8] = include_bytes!("../../../logo_sidebar.png");

// ---- Paleta (impresión en negro + acentos clínicos) ----
const C_TEXT: (u8, u8, u8) = (30, 30, 30);
const C_MUTED: (u8, u8, u8) = (110, 110, 110);
const C_RULE: (u8, u8, u8) = (190, 190, 190);
const C_HEADER_BG: (u8, u8, u8) = (232, 240, 238);
const C_NORMAL: (u8, u8, u8) = (40, 130, 70);
const C_ALTO: (u8, u8, u8) = (190, 110, 0);
const C_BAJO: (u8, u8, u8) = (190, 45, 45);
const C_SIN_RANGO: (u8, u8, u8) = (120, 120, 120);

const FONT: PdfFontHandle = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
const FONT_BOLD: PdfFontHandle = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);

/// Convierte una tupla RGB (0-255) al color normalizado de printpdf (0-1).
fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(Rgb {
        r: f32::from(c.0) / 255.0,
        g: f32::from(c.1) / 255.0,
        b: f32::from(c.2) / 255.0,
        icc_profile: None,
    })
}

/// Reemplaza caracteres fuera de WinAnsi/Latin-1 (las flechas, comillas
/// tipográficas, etc. romperían la fuente estándar Helvetica).
fn sanitize(text: &str) -> String {
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
fn text_width_mm(text: &str, size_pt: f32) -> f32 {
    let chars = text.chars().filter(|c| !c.is_control()).count() as f32;
    size_pt * 0.5 * chars * 0.352778
}

/// Etiqueta corta para estados clínicos.
fn status_label(status: &str) -> &'static str {
    match status {
        "ALTO" => "ALTO",
        "BAJO" => "BAJO",
        "NORMAL" => "NORMAL",
        _ => "SIN RANGO",
    }
}

/// Color del estado (también aplicado al valor).
fn status_color(status: &str) -> (u8, u8, u8) {
    match status {
        "ALTO" => C_ALTO,
        "BAJO" => C_BAJO,
        "NORMAL" => C_NORMAL,
        _ => C_SIN_RANGO,
    }
}

/// Acumula operaciones printpdf por página (A4, cursor vertical en mm).
struct PdfBuilder {
    pages: Vec<PdfPage>,
    ops: Vec<Op>,
    y: f32,
    /// Logo institucional de ISALAB (esquina superior izquierda).
    left_logo: Option<RawImage>,
    /// Logo de la empresa/clínica cliente (esquina superior derecha).
    right_logo: Option<RawImage>,
}

impl PdfBuilder {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            ops: Vec::new(),
            y: PAGE_H - MARGIN,
            left_logo: None,
            right_logo: None,
        }
    }

    fn flush(&mut self) {
        self.pages.push(PdfPage::new(
            Mm(PAGE_W),
            Mm(PAGE_H),
            std::mem::take(&mut self.ops),
        ));
    }

    fn new_page(&mut self) {
        self.flush();
        self.y = PAGE_H - MARGIN;
    }

    fn ensure_space(&mut self, height_mm: f32) {
        if self.y - height_mm < MARGIN {
            self.new_page();
        }
    }

    fn text(
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

    fn text_centered(&mut self, bold: bool, text: &str, size_pt: f32, y_mm: f32, color: (u8, u8, u8)) {
        let w = text_width_mm(text, size_pt);
        self.text(bold, text, size_pt, (PAGE_W - w) / 2.0, y_mm, color);
    }

    fn rule(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: (u8, u8, u8)) {
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

    fn rect(
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
}

/// Genera el PDF del informe y devuelve la ruta del archivo.
///
/// Firma: modo GRAPHIC renderiza el bloque de texto; DIGITAL (PKCS#12) aún no
/// está implementado y devuelve un error explícito.
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

/// Carga el logo izquierdo de ISALAB: prueba archivos locales de la app o el incrustado.
fn load_left_logo() -> Option<RawImage> {
    let decode = |bytes: &[u8]| -> Option<RawImage> {
        let mut warnings: Vec<PdfWarnMsg> = Vec::new();
        RawImage::decode_from_bytes(bytes, &mut warnings).ok()
    };

    for candidate in &["dist/isalab.png", "isalab.png", "isalab_icon.png", "dist/icon.png"] {
        if let Ok(bytes) = std::fs::read(candidate) {
            if let Some(img) = decode(&bytes) {
                return Some(img);
            }
        }
    }
    decode(DEFAULT_LOGO)
}

/// Carga el logo derecho (empresa cliente): primero `clinic.logo_path` y luego archivos en dist.
fn load_right_logo(clinic: &ClinicHeader) -> Option<RawImage> {
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

/// Guarda las páginas compiladas en `out_path`, incrustando los logos en los recursos del documento.
fn save_pdf(mut pdf: PdfBuilder, out_path: &Path, doc_title: &str) -> Result<(), String> {
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

impl PdfBuilder {
    fn finish(mut self) -> Vec<PdfPage> {
        self.flush();
        self.pages
    }
}

/// Encabezado institucional: Logo ISALAB a la izquierda, datos de ISALAB en el centro,
/// logo de la empresa a la derecha, y título del informe.
fn draw_header(
    pdf: &mut PdfBuilder,
    clinic: &ClinicHeader,
    title: &str,
    subtitle: &str,
    extra: Option<&str>,
) {
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
        left_h_mm = if w_px > 0.0 { (LOGO_W_MM * h_px / w_px).min(22.0) } else { LOGO_W_MM };
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
        right_h_mm = if w_px > 0.0 { (LOGO_W_MM * h_px / w_px).min(22.0) } else { LOGO_W_MM };
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
    pdf.y = start_y - max_h - 4.0;

    pdf.rule(MARGIN, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
    pdf.y -= 7.0;
    pdf.text_centered(true, title, 11.5, pdf.y, C_TEXT);
    pdf.y -= 5.0;
    pdf.text_centered(false, subtitle, 8.5, pdf.y, C_TEXT);
    if let Some(line) = extra {
        pdf.y -= 4.5;
        pdf.text_centered(false, line, 8.0, pdf.y, C_MUTED);
    }
    pdf.y -= 5.5;
    pdf.rule(MARGIN, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
}

fn draw_patient_block(pdf: &mut PdfBuilder, patient: &Patient) {
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

fn draw_results(pdf: &mut PdfBuilder, results: &[LabResult]) {
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

fn format_value(v: f64) -> String {
    if v == v.round() {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Bloque de firma del médico veterinario (usado por laboratorio, fórmula,
/// cirugía y carnet de vacunación).
fn draw_signature(pdf: &mut PdfBuilder, signature: &ReportSignature) {
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

/// Nota de interpretación específica del informe de laboratorio.
fn draw_lab_note(pdf: &mut PdfBuilder) {
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

fn draw_footer(pdf: &mut PdfBuilder, label: &str) {
    let footer = format!("ISALAB · Documento generado automáticamente · {label}");
    pdf.text(false, &footer, 7.0, MARGIN, MARGIN - 4.0, C_MUTED);
}

// ============================================================================
// Helpers genéricos para los documentos clínicos
// ============================================================================

/// Título de sección (etiqueta gris con salto de bloque).
fn section_title(pdf: &mut PdfBuilder, title: &str) {
    pdf.y -= 4.0;
    pdf.ensure_space(30.0);
    pdf.text(true, title, 8.5, MARGIN, pdf.y, C_MUTED);
    pdf.y -= 6.0;
}

/// Rejilla de pares etiqueta/valor en dos columnas (como la ficha del lab).
fn draw_grid(pdf: &mut PdfBuilder, rows: &[(&str, String)]) {
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

/// Divide un texto en líneas de ≤ `max_chars` respetando saltos de línea.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
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
fn draw_multiline(
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
fn text_right(
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
fn truncate(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

/// Formatea un monto como COP (separador de miles ".", decimales ",").
fn format_cop(v: f64) -> String {
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

fn surgery_status_label(s: &str) -> &'static str {
    match s {
        "PROGRAMADA" => "Programada",
        "EN_CURSO" => "En curso",
        "COMPLETADA" => "Completada",
        "CANCELADA" => "Cancelada",
        _ => "—",
    }
}

fn invoice_status_label(s: &str) -> &'static str {
    match s {
        "EMITIDA" => "Emitida",
        "PAGADA" => "Pagada",
        "ANULADA" => "Anulada",
        _ => "—",
    }
}

fn payment_method_label(s: Option<&str>) -> String {
    match s.unwrap_or("") {
        "EFECTIVO" => "Efectivo".into(),
        "TRANSFERENCIA" => "Transferencia".into(),
        "TARJETA_CREDITO" => "Tarjeta crédito".into(),
        "TARJETA_DEBITO" => "Tarjeta débito".into(),
        other => truncate(other, 30),
    }
}

const CONSENT_DECLARATION: &str = "Autorizo al médico veterinario y al personal de la clínica a \
realizar sobre mi mascota el procedimiento descrito. Declaro haber sido informado(a) de los \
riesgos, beneficios y alternativas, y de los cuidados posteriores necesarios. Entiendo que el \
resultado del procedimiento no está garantizado y que pueden presentarse complicaciones \
inherentes a todo procedimiento médico.";

const CONSENT_RISKS: &str = "Como en todo procedimiento médico existen riesgos inherentes: \
reacciones adversas a la anestesia, sangrado, infección, edema, dehiscencia de herida y, en \
casos excepcionales, complicaciones que pueden comprometer la vida del paciente. El equipo \
veterinario tomará todas las medidas preventivas y correctivas a su alcance.";

// ============================================================================
// Fórmula médica (receta veterinaria)
// ============================================================================

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

// ============================================================================
// Consentimiento informado
// ============================================================================

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

// ============================================================================
// Comprobante de pago (recibo de factura)
// ============================================================================

pub fn generate_recibo(data: &ReciboData, out_path: &Path) -> Result<(), String> {
    let inv = &data.invoice;
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "COMPROBANTE DE PAGO",
        &format!("Factura {}", inv.invoice_number),
        None,
    );

    section_title(&mut pdf, "DATOS DEL COMPROBANTE");
    draw_grid(&mut pdf, &[
        ("Fecha", inv.issue_date.clone()),
        ("Cliente", inv.owner_name.clone()),
        ("Paciente", inv.patient_name.clone().unwrap_or_else(|| "—".into())),
        (
            "Método de pago",
            payment_method_label(inv.payment_method.as_deref()),
        ),
        ("Estado", invoice_status_label(&inv.status).to_string()),
    ]);

    section_title(&mut pdf, "CONCEPTOS FACTURADOS");
    let row_h = 6.2;
    let header_y = pdf.y - 1.6;
    pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, Some(C_HEADER_BG), Some(C_RULE));
    pdf.text(true, "CONCEPTO", 7.5, MARGIN + 2.0, header_y, C_TEXT);
    text_right(
        &mut pdf,
        true,
        "IMPORTE",
        7.5,
        PAGE_W - MARGIN - 2.0,
        header_y,
        C_TEXT,
    );
    pdf.y -= row_h;

    for it in &inv.items {
        pdf.ensure_space(row_h + 1.0);
        pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, None, Some(C_RULE));
        let row_y = pdf.y - 1.6;
        pdf.text(
            false,
            &sanitize(&truncate(&it.description, 78)),
            8.5,
            MARGIN + 2.0,
            row_y,
            C_TEXT,
        );
        pdf.text(
            false,
            &format!("{} × {}", it.quantity, format_cop(it.unit_price)),
            7.5,
            MARGIN + 95.0,
            row_y,
            C_MUTED,
        );
        text_right(
            &mut pdf,
            true,
            &format_cop(it.line_total),
            8.5,
            PAGE_W - MARGIN - 2.0,
            row_y,
            C_TEXT,
        );
        pdf.y -= row_h;
    }

    // Totales.
    pdf.y -= 6.0;
    pdf.ensure_space(30.0);
    pdf.text(false, "Subtotal", 8.5, MARGIN + 95.0, pdf.y, C_MUTED);
    let y = pdf.y;
    text_right(
        &mut pdf,
        false,
        &format_cop(inv.subtotal),
        8.5,
        PAGE_W - MARGIN - 2.0,
        y,
        C_TEXT,
    );
    pdf.y -= 5.5;
    pdf.text(false, &format!("IVA ({}%)", inv.tax_rate), 8.5, MARGIN + 95.0, pdf.y, C_MUTED);
    let y = pdf.y;
    text_right(
        &mut pdf,
        false,
        &format_cop(inv.tax_amount),
        8.5,
        PAGE_W - MARGIN - 2.0,
        y,
        C_TEXT,
    );
    pdf.y -= 5.5;
    pdf.rule(MARGIN + 95.0, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
    pdf.y -= 6.0;
    pdf.text(true, "TOTAL", 9.0, MARGIN + 95.0, pdf.y, C_TEXT);
    let y = pdf.y;
    text_right(
        &mut pdf,
        true,
        &format_cop(inv.total),
        9.0,
        PAGE_W - MARGIN - 2.0,
        y,
        C_TEXT,
    );
    pdf.y -= 6.0;

    if let Some(notes) = inv.notes.as_deref().filter(|n| !n.trim().is_empty()) {
        section_title(&mut pdf, "NOTAS");
        pdf.y = draw_multiline(&mut pdf, notes, 8.5, MARGIN, C_TEXT, 4.8);
    }

    // Firma autorizada.
    pdf.y -= 16.0;
    pdf.ensure_space(30.0);
    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;
    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_RULE);
    pdf.y -= 7.0;
    pdf.text(true, "Administración ISALAB", 9.0, x, pdf.y, C_TEXT);
    pdf.y -= 5.0;
    pdf.text(false, "Firma autorizada", 7.5, x, pdf.y, C_MUTED);

    draw_footer(&mut pdf, &format!("Recibo {}", inv.invoice_number));
    save_pdf(pdf, out_path, "ISALAB · Comprobante de pago")
}

// ============================================================================
// Reporte / certificado quirúrgico
// ============================================================================

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

// ============================================================================
// Certificado / carnet de vacunación
// ============================================================================

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
    draw_grid(&mut pdf, &[
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
    ]);

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
    draw_grid(&mut pdf, &[
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
    ]);

    section_title(&mut pdf, "VACUNAS ADMINISTRADAS");
    if data.vaccines.is_empty() {
        pdf.text(false, "Sin vacunas registradas.", 9.0, MARGIN, pdf.y, C_MUTED);
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
        pdf.rect(MARGIN, pdf.y, CONTENT_W, row_h, Some(C_HEADER_BG), Some(C_RULE));
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
            pdf.text(false, &truncate(&v.vaccine_name, 20), 7.5, x, pdf.y - 1.6, C_TEXT);
            x += cols[0].0;
            pdf.text(false, &v.administered_at, 7.0, x, pdf.y - 1.6, C_TEXT);
            x += cols[1].0;
            pdf.text(false, v.lot.as_deref().unwrap_or("—"), 7.0, x, pdf.y - 1.6, C_TEXT);
            x += cols[2].0;
            pdf.text(false, v.next_dose_at.as_deref().unwrap_or("—"), 7.0, x, pdf.y - 1.6, C_TEXT);
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
        pdf.rect(MARGIN, pdf.y, CONTENT_W, 10.0, Some(C_HEADER_BG), Some(C_RULE));
        pdf.text(true, &sanitize(&label), 9.0, MARGIN + 4.0, pdf.y - 4.0, C_TEXT);
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

    draw_footer(&mut pdf, &format!("Certificado {}", data.certificate_number));
    save_pdf(pdf, out_path, "ISALAB · Certificado de vacunación")
}
