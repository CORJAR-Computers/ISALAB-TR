use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::models::cash::{CashCountLine, CashMethodTotal, CashMovement, CashSession};
use crate::models::invoice::Invoice;
use crate::pdf_templates::builder::{
    draw_multiline, format_cop, sanitize, save_pdf, text_right, truncate, PdfBuilder, CONTENT_W,
    C_HEADER_BG, C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W,
};
use crate::pdf_templates::header::{draw_header, ClinicHeader};
use crate::pdf_templates::layout::{draw_footer, draw_grid, section_title};

/// Datos del comprobante de pago (recibo de factura).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReciboData {
    pub clinic: ClinicHeader,
    pub invoice: Invoice,
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
    draw_grid(
        &mut pdf,
        &[
            ("Fecha", inv.issue_date.clone()),
            ("Cliente", inv.owner_name.clone()),
            (
                "Paciente",
                inv.patient_name.clone().unwrap_or_else(|| "—".into()),
            ),
            (
                "Método de pago",
                payment_method_label(inv.payment_method.as_deref()),
            ),
            ("Estado", invoice_status_label(&inv.status).to_string()),
        ],
    );

    section_title(&mut pdf, "CONCEPTOS FACTURADOS");
    let row_h = 6.2;
    let header_y = pdf.y - 1.6;
    pdf.rect(
        MARGIN,
        pdf.y,
        CONTENT_W,
        row_h,
        Some(C_HEADER_BG),
        Some(C_RULE),
    );
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
    pdf.text(
        false,
        &format!("IVA ({}%)", inv.tax_rate),
        8.5,
        MARGIN + 95.0,
        pdf.y,
        C_MUTED,
    );
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

/// Datos del cierre de caja (arqueo, totales por método y movimientos).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashCloseData {
    pub clinic: ClinicHeader,
    pub session: CashSession,
    pub expected_cash: f64,
    pub counted_cash: f64,
    pub difference: f64,
    pub total_billed: f64,
    pub total_collected: f64,
    pub cash_in: f64,
    pub cash_out: f64,
    pub payments_count: i32,
    pub methods: Vec<CashMethodTotal>,
    pub movements: Vec<CashMovement>,
    pub count_lines: Vec<CashCountLine>,
}

fn movement_kind_label(kind: &str) -> &'static str {
    match kind {
        "PAGO_EFECTIVO" => "Pago en efectivo",
        "GASTO" => "Gasto",
        "RETIRO" => "Retiro",
        "DEVOLUCION" => "Devolución",
        "AJUSTE" => "Ajuste",
        _ => "Movimiento",
    }
}

/// Encabezado de una tabla de tres columnas (concepto, detalle, importe).
fn table_header(pdf: &mut PdfBuilder, concept: &str, detail: &str, amount: &str) {
    let row_h = 6.2;
    let header_y = pdf.y - 1.6;
    pdf.rect(
        MARGIN,
        pdf.y,
        CONTENT_W,
        row_h,
        Some(C_HEADER_BG),
        Some(C_RULE),
    );
    pdf.text(true, concept, 7.5, MARGIN + 2.0, header_y, C_TEXT);
    pdf.text(true, detail, 7.5, MARGIN + 62.0, header_y, C_TEXT);
    text_right(
        pdf,
        true,
        amount,
        7.5,
        PAGE_W - MARGIN - 2.0,
        header_y,
        C_TEXT,
    );
    pdf.y -= row_h;
}

pub fn generate_cash_close(data: &CashCloseData, out_path: &Path) -> Result<(), String> {
    let s = &data.session;
    let mut pdf = PdfBuilder::new();
    draw_header(
        &mut pdf,
        &data.clinic,
        "CIERRE DE CAJA",
        &format!("Caja {}", s.code),
        None,
    );

    section_title(&mut pdf, "DATOS DE LA SESIÓN");
    draw_grid(
        &mut pdf,
        &[
            ("Apertura", s.opened_at.clone()),
            ("Cierre", s.closed_at.clone().unwrap_or_else(|| "—".into())),
            (
                "Abierta por",
                s.opened_by_name.clone().unwrap_or_else(|| "—".into()),
            ),
            (
                "Cerrada por",
                s.closed_by_name.clone().unwrap_or_else(|| "—".into()),
            ),
            ("Base inicial", format_cop(data.session.opening_float)),
            ("Recibos", data.payments_count.to_string()),
        ],
    );

    section_title(&mut pdf, "RESUMEN");
    let resumen = [
        ("Facturado en la sesión", format_cop(data.total_billed)),
        ("Cobrado en la sesión", format_cop(data.total_collected)),
        ("Ingresos de caja", format_cop(data.cash_in)),
        ("Egresos de caja", format_cop(data.cash_out)),
        ("Efectivo esperado", format_cop(data.expected_cash)),
        ("Efectivo contado", format_cop(data.counted_cash)),
        ("Diferencia", format_cop(data.difference)),
    ];
    for (label, value) in resumen {
        pdf.ensure_space(6.0);
        pdf.text(false, label, 8.5, MARGIN, pdf.y, C_MUTED);
        let y = pdf.y;
        text_right(
            &mut pdf,
            true,
            &value,
            8.5,
            PAGE_W - MARGIN - 2.0,
            y,
            C_TEXT,
        );
        pdf.y -= 5.5;
    }

    if !data.methods.is_empty() {
        section_title(&mut pdf, "TOTALES POR MÉTODO");
        table_header(&mut pdf, "MÉTODO", "", "IMPORTE");
        for method in &data.methods {
            pdf.ensure_space(7.2);
            pdf.rect(MARGIN, pdf.y, CONTENT_W, 6.2, None, Some(C_RULE));
            let row_y = pdf.y - 1.6;
            pdf.text(
                false,
                &sanitize(&truncate(&method.method_name, 50)),
                8.5,
                MARGIN + 2.0,
                row_y,
                C_TEXT,
            );
            text_right(
                &mut pdf,
                false,
                &format_cop(method.amount),
                8.5,
                PAGE_W - MARGIN - 2.0,
                row_y,
                C_TEXT,
            );
            pdf.y -= 6.2;
        }
    }

    if !data.movements.is_empty() {
        section_title(&mut pdf, "MOVIMIENTOS DE CAJA");
        table_header(&mut pdf, "TIPO", "MOTIVO", "IMPORTE");
        for movement in &data.movements {
            pdf.ensure_space(7.2);
            pdf.rect(MARGIN, pdf.y, CONTENT_W, 6.2, None, Some(C_RULE));
            let row_y = pdf.y - 1.6;
            pdf.text(
                false,
                movement_kind_label(&movement.kind),
                8.5,
                MARGIN + 2.0,
                row_y,
                C_TEXT,
            );
            pdf.text(
                false,
                &sanitize(&truncate(movement.reason.as_deref().unwrap_or("—"), 40)),
                8.0,
                MARGIN + 62.0,
                row_y,
                C_MUTED,
            );
            text_right(
                &mut pdf,
                true,
                &format_cop(movement.amount),
                8.5,
                PAGE_W - MARGIN - 2.0,
                row_y,
                C_TEXT,
            );
            pdf.y -= 6.2;
        }
    }

    section_title(&mut pdf, "ARQUEO POR DENOMINACIÓN");
    table_header(&mut pdf, "DENOMINACIÓN", "CANTIDAD", "SUBTOTAL");
    for line in &data.count_lines {
        pdf.ensure_space(7.2);
        pdf.rect(MARGIN, pdf.y, CONTENT_W, 6.2, None, Some(C_RULE));
        let row_y = pdf.y - 1.6;
        pdf.text(
            false,
            &format_cop(line.denomination),
            8.5,
            MARGIN + 2.0,
            row_y,
            C_TEXT,
        );
        pdf.text(
            false,
            &line.quantity.to_string(),
            8.5,
            MARGIN + 62.0,
            row_y,
            C_TEXT,
        );
        text_right(
            &mut pdf,
            false,
            &format_cop(line.subtotal),
            8.5,
            PAGE_W - MARGIN - 2.0,
            row_y,
            C_TEXT,
        );
        pdf.y -= 6.2;
    }

    pdf.y -= 3.0;
    pdf.ensure_space(24.0);
    pdf.rule(MARGIN + 95.0, pdf.y, PAGE_W - MARGIN, pdf.y, C_RULE);
    pdf.y -= 6.0;
    pdf.text(true, "TOTAL CONTADO", 9.0, MARGIN + 95.0, pdf.y, C_TEXT);
    let y = pdf.y;
    text_right(
        &mut pdf,
        true,
        &format_cop(data.counted_cash),
        9.0,
        PAGE_W - MARGIN - 2.0,
        y,
        C_TEXT,
    );
    pdf.y -= 7.0;

    if let Some(notes) = data
        .session
        .notes
        .as_deref()
        .filter(|n| !n.trim().is_empty())
    {
        section_title(&mut pdf, "NOTAS");
        pdf.y = draw_multiline(&mut pdf, notes, 8.5, MARGIN, C_TEXT, 4.8);
    }

    pdf.y -= 16.0;
    pdf.ensure_space(30.0);
    let sig_w = 90.0;
    let x = PAGE_W - MARGIN - sig_w;
    pdf.rule(x, pdf.y, x + sig_w, pdf.y, C_RULE);
    pdf.y -= 7.0;
    pdf.text(true, "Administración ISALAB", 9.0, x, pdf.y, C_TEXT);
    pdf.y -= 5.0;
    pdf.text(false, "Firma autorizada", 7.5, x, pdf.y, C_MUTED);

    draw_footer(&mut pdf, &format!("Cierre de caja {}", s.code));
    save_pdf(pdf, out_path, "ISALAB · Cierre de caja")
}
