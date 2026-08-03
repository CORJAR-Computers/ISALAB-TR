use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::models::invoice::Invoice;
use crate::pdf_templates::builder::{
    draw_multiline, format_cop, sanitize, save_pdf, text_right, truncate, CONTENT_W, C_HEADER_BG,
    C_MUTED, C_RULE, C_TEXT, MARGIN, PAGE_W, PdfBuilder,
};
use crate::pdf_templates::clinical::{draw_footer, draw_grid, section_title};
use crate::pdf_templates::header::{draw_header, ClinicHeader};

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
