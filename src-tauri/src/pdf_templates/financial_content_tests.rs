//! Tests de **contenido** de los documentos financieros.
//!
//! Los tests de layout comprueban que el PDF se arma; estos generan el
//! documento real y extraen su texto para verificar lo que el cliente recibe:
//! número de factura, importes formateados, titular y la diferencia de arqueo
//! con su signo. Un importe mal calculado o un campo que se dejó de pintar
//! falla aquí y no en la clínica.

use std::path::Path;

use super::{generate_cash_close, generate_recibo, CashCloseData, ReciboData};
use crate::models::cash::CashSession;
use crate::models::invoice::{Invoice, InvoiceItem};
use crate::pdf_templates::header::ClinicHeader;

fn clinic() -> ClinicHeader {
    ClinicHeader {
        name: "Clínica Veterinaria Central".into(),
        nit: "900.123.456-7".into(),
        address: Some("Calle 10 # 5-20".into()),
        phone: Some("300 555 0000".into()),
        city: Some("Bogotá".into()),
        logo_bytes: None,
    }
}

fn invoice() -> Invoice {
    Invoice {
        id: 42,
        invoice_number: "FAC-000042".into(),
        patient_id: Some(7),
        patient_name: Some("Luna".into()),
        owner_id: 3,
        owner_name: "Ana Pérez".into(),
        owner_phone: Some("300 555 0000".into()),
        consultation_id: None,
        issue_date: "2026-11-21 10:15:00".into(),
        subtotal: 1_000_000.0,
        tax_rate: 19.0,
        tax_amount: 190_000.0,
        total: 1_190_000.0,
        status: "PAGADA".into(),
        payment_method: Some("EFECTIVO".into()),
        notes: None,
        items: vec![InvoiceItem {
            id: 1,
            description: "Consulta especializada".into(),
            quantity: 2,
            unit_price: 500_000.0,
            line_total: 1_000_000.0,
        }],
    }
}

fn session() -> CashSession {
    CashSession {
        id: 1,
        code: "CAJ-00000001".into(),
        opened_at: "2026-11-21 08:00:00".into(),
        opened_by: 1,
        opened_by_name: Some("Administrador".into()),
        opening_float: 100_000.0,
        status: "CERRADA".into(),
        closed_at: Some("2026-11-21 18:00:00".into()),
        closed_by_name: Some("Administrador".into()),
        expected_cash: Some(250_000.0),
        counted_cash: Some(245_000.0),
        difference: Some(-5_000.0),
        total_billed: Some(1_190_000.0),
        total_collected: Some(1_190_000.0),
        notes: None,
    }
}

/// Convierte a texto los literales `(…)` de los content streams del PDF.
///
/// printpdf escribe con fuentes builtin (WinAnsi), así que el texto visible es
/// literal; `lopdf` ya descomprime el stream por nosotros.
fn pdf_text(path: &Path) -> String {
    let doc = lopdf::Document::load(path).expect("el PDF debe ser legible");
    let mut out = String::new();
    for (_page, page_id) in doc.get_pages() {
        // lopdf 0.44 devuelve el contenido ya descomprimido (sin Result).
        let bytes = doc.get_page_content(page_id);
        let mut i = 0;
        while i < bytes.len() {
            // printpdf escribe los textos con acentos como cadenas hex
            // (<41E9...>) para conservar WinAnsi; los ASCII van literales.
            if bytes[i] == b'<' && bytes.get(i + 1) != Some(&b'<') {
                i += 1;
                let mut hex = String::new();
                while i < bytes.len() && bytes[i] != b'>' {
                    if !bytes[i].is_ascii_whitespace() {
                        hex.push(bytes[i] as char);
                    }
                    i += 1;
                }
                i += 1;
                let digits: Vec<u8> = hex.bytes().collect();
                let mut decoded: Vec<u8> = Vec::new();
                for pair in digits.chunks(2) {
                    if pair.len() == 2 {
                        let text = format!("{}{}", pair[0] as char, pair[1] as char);
                        if let Ok(byte) = u8::from_str_radix(&text, 16) {
                            decoded.push(byte);
                        }
                    }
                }
                // UTF-16BE (bytes alternando 0x00) o WinAnsi de un byte.
                let utf16: Vec<u16> = decoded
                    .chunks(2)
                    .filter(|c| c.len() == 2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                let is_utf16 = decoded.len() % 2 == 0
                    && !decoded.is_empty()
                    && decoded.iter().step_by(2).all(|&b| b == 0);
                if is_utf16 {
                    out.push_str(&String::from_utf16_lossy(&utf16));
                } else {
                    out.extend(decoded.iter().map(|&b| b as char));
                }
                out.push('\n');
                continue;
            }
            if bytes[i] != b'(' {
                i += 1;
                continue;
            }
            i += 1;
            let mut depth = 1usize;
            let mut buf: Vec<u8> = Vec::new();
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'\\' if i + 1 < bytes.len() => {
                        match bytes[i + 1] {
                            b'(' | b')' | b'\\' => buf.push(bytes[i + 1]),
                            b'n' => buf.push(b'\n'),
                            b'r' => buf.push(b'\r'),
                            b't' => buf.push(b'\t'),
                            _ => {}
                        }
                        i += 2;
                    }
                    b'(' => {
                        depth += 1;
                        buf.push(b'(');
                        i += 1;
                    }
                    b')' => {
                        depth -= 1;
                        i += 1;
                        if depth > 0 {
                            buf.push(b')');
                        }
                    }
                    other => {
                        buf.push(other);
                        i += 1;
                    }
                }
            }
            // WinAnsi ≈ Latin-1 para los acentos que usa la clínica.
            out.extend(buf.iter().map(|&b| b as char));
            out.push('\n');
        }
    }
    out
}

#[test]
fn el_comprobante_lleva_numero_titular_e_importes() {
    let path = std::env::temp_dir().join("isalab_recibo_contenido.pdf");
    generate_recibo(
        &ReciboData {
            clinic: clinic(),
            invoice: invoice(),
        },
        &path,
    )
    .expect("generar el comprobante");

    let text = pdf_text(&path);
    let _ = std::fs::remove_file(&path);

    assert!(
        text.contains("FAC-000042"),
        "falta el número de factura:\n{text}"
    );
    assert!(text.contains("Ana Pérez"), "falta el titular:\n{text}");
    assert!(text.contains("Luna"), "falta el paciente:\n{text}");
    assert!(
        text.contains("Consulta especializada"),
        "falta el concepto:\n{text}"
    );
    // Formato COP con separador de miles y dos decimales.
    assert!(
        text.contains("$1.190.000,00"),
        "falta el total formateado:\n{text}"
    );
    assert!(
        text.contains("$1.000.000,00"),
        "falta el importe de la línea:\n{text}"
    );
}

#[test]
fn el_cierre_muestra_esperado_contado_y_diferencia_con_signo() {
    let path = std::env::temp_dir().join("isalab_cierre_contenido.pdf");
    generate_cash_close(
        &CashCloseData {
            clinic: clinic(),
            session: session(),
            expected_cash: 250_000.0,
            counted_cash: 245_000.0,
            difference: -5_000.0,
            total_billed: 1_190_000.0,
            total_collected: 1_190_000.0,
            cash_in: 1_190_000.0,
            cash_out: 100_000.0,
            payments_count: 3,
            methods: Vec::new(),
            movements: Vec::new(),
            count_lines: Vec::new(),
        },
        &path,
    )
    .expect("generar el cierre");

    let text = pdf_text(&path);
    let _ = std::fs::remove_file(&path);

    assert!(text.contains("CIERRE DE CAJA"), "falta el título:\n{text}");
    assert!(
        text.contains("CAJ-00000001"),
        "falta el código de caja:\n{text}"
    );
    assert!(
        text.contains("$250.000,00"),
        "falta el efectivo esperado:\n{text}"
    );
    assert!(
        text.contains("$245.000,00"),
        "falta el efectivo contado:\n{text}"
    );
    // Un faltante se imprime negativo: el signo es parte del documento.
    assert!(
        text.contains("-$5.000,00"),
        "la diferencia debe llevar signo:\n{text}"
    );
}

#[test]
fn nombres_largos_e_importes_de_siete_cifras_se_dibujan_completos() {
    let mut inv = invoice();
    inv.owner_name = "María Fernanda de los Ángeles Rodríguez-Cárdenas y Asociados".into();
    inv.total = 9_999_999.99;
    inv.items = vec![InvoiceItem {
        id: 2,
        description: "Perfil completo con ecografía abdominal y hospitalización".into(),
        quantity: 3,
        unit_price: 3_333_333.33,
        line_total: 9_999_999.99,
    }];

    let path = std::env::temp_dir().join("isalab_recibo_largo.pdf");
    generate_recibo(
        &ReciboData {
            clinic: clinic(),
            invoice: inv,
        },
        &path,
    )
    .expect("generar el comprobante largo");

    let text = pdf_text(&path);
    let _ = std::fs::remove_file(&path);

    assert!(
        text.contains("María Fernanda"),
        "el nombre largo debe empezar completo:\n{text}"
    );
    assert!(
        text.contains("$9.999.999,99"),
        "un importe de siete cifras no debe truncarse:\n{text}"
    );
}
