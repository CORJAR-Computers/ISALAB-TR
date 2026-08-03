use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::error::AppError;
use crate::models::invoice::{
    CreateInvoiceInput, Invoice, InvoiceItem, InvoiceListItem,
};
use crate::repositories::next_id;

/// Columnas de una factura con propietario/paciente unidos.
const INVOICE_SELECT: &str = "
    SELECT i.ID, i.INVOICE_NUMBER, i.PATIENT_ID, p.NAME,
           i.OWNER_ID, o.FULL_NAME, i.CONSULTATION_ID,
           LEFT(CAST(i.ISSUE_DATE AS VARCHAR(60)), 19),
           CAST(i.SUBTOTAL AS DOUBLE PRECISION),
           CAST(i.TAX_RATE AS DOUBLE PRECISION),
           CAST(i.TAX_AMOUNT AS DOUBLE PRECISION),
           CAST(i.TOTAL AS DOUBLE PRECISION),
           i.STATUS, i.PAYMENT_METHOD, i.NOTES
    FROM INVOICES i
    JOIN OWNERS o ON o.ID = i.OWNER_ID
    LEFT JOIN PATIENTS p ON p.ID = i.PATIENT_ID";

type InvoiceRow = (
    i32,             // id
    String,          // invoice_number
    Option<i32>,     // patient_id
    Option<String>,  // patient_name
    i32,             // owner_id
    String,          // owner_name
    Option<i32>,     // consultation_id
    String,          // issue_date
    f64,             // subtotal
    f64,             // tax_rate
    f64,             // tax_amount
    f64,             // total
    String,          // status
    Option<String>,  // payment_method
    Option<String>,  // notes
);

fn map_invoice(r: InvoiceRow, items: Vec<InvoiceItem>) -> Invoice {
    Invoice {
        id: r.0,
        invoice_number: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        consultation_id: r.6,
        issue_date: r.7,
        subtotal: r.8,
        tax_rate: r.9,
        tax_amount: r.10,
        total: r.11,
        status: r.12,
        payment_method: r.13,
        notes: r.14,
        items,
    }
}

fn list_items(
    conn: &mut SimpleConnection,
    invoice_id: i32,
) -> Result<Vec<InvoiceItem>, AppError> {
    let rows: Vec<(i32, String, i32, f64, f64)> = conn
        .query(
            "SELECT ID, DESCRIPTION, QUANTITY,
                    CAST(UNIT_PRICE AS DOUBLE PRECISION),
                    CAST(LINE_TOTAL AS DOUBLE PRECISION)
             FROM INVOICE_ITEMS WHERE INVOICE_ID = ? ORDER BY ID",
            (&invoice_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| InvoiceItem {
            id: r.0,
            description: r.1,
            quantity: r.2,
            unit_price: r.3,
            line_total: r.4,
        })
        .collect())
}

/// IVA por defecto de la configuración (CLINIC_SETTINGS 'invoice.tax_rate').
fn default_tax_rate(conn: &mut SimpleConnection) -> Result<f64, AppError> {
    let row: Option<(Option<String>,)> = conn
        .query_first(
            "SELECT VALUE_TEXT FROM CLINIC_SETTINGS WHERE KEY_NAME = 'invoice.tax_rate'",
            (),
        )
        .map_err(AppError::from)?;
    Ok(row
        .and_then(|(v,)| v)
        .and_then(|v| v.parse().ok())
        .unwrap_or(19.0))
}

/// Factura completa (con items) o `None`.
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Invoice>, AppError> {
    let row: Option<InvoiceRow> = conn
        .query_first(&format!("{INVOICE_SELECT} WHERE i.ID = ?"), (&id,))
        .map_err(AppError::from)?;

    let Some(row) = row else { return Ok(None) };
    let items = list_items(conn, id)?;
    Ok(Some(map_invoice(row, items)))
}

/// Emite una factura con sus items. Calcula subtotal, IVA (configurable o el
/// de la clínica) y total; el número lo asigna el trigger BI_INVOICES
/// (FAC-000001…).
pub fn create(
    conn: &mut SimpleConnection,
    input: &CreateInvoiceInput,
) -> Result<Invoice, AppError> {
    if input.items.is_empty() {
        return Err(AppError::Validation(
            "La factura debe tener al menos un item".into(),
        ));
    }
    for it in &input.items {
        if it.description.trim().is_empty() {
            return Err(AppError::Validation(
                "Cada item debe tener una descripción".into(),
            ));
        }
        if it.quantity <= 0 {
            return Err(AppError::Validation(
                "Las cantidades deben ser mayores que cero".into(),
            ));
        }
        if it.unit_price < 0.0 {
            return Err(AppError::Validation(
                "El precio unitario no puede ser negativo".into(),
            ));
        }
    }

    let subtotal: f64 = input
        .items
        .iter()
        .map(|it| it.quantity as f64 * it.unit_price)
        .sum();
    let tax_rate = match input.tax_rate {
        Some(rate) => rate,
        None => default_tax_rate(conn)?,
    };
    let tax_amount = subtotal * tax_rate / 100.0;
    let total = subtotal + tax_amount;

    let id = next_id(conn, "GEN_INVOICES_ID")?;
    conn.execute(
        "INSERT INTO INVOICES
            (ID, PATIENT_ID, OWNER_ID, CONSULTATION_ID, SUBTOTAL, TAX_RATE,
             TAX_AMOUNT, TOTAL, STATUS, PAYMENT_METHOD, NOTES)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'EMITIDA', ?, ?)",
        (
            &id,
            &input.patient_id,
            &input.owner_id,
            &input.consultation_id,
            &subtotal,
            &tax_rate,
            &tax_amount,
            &total,
            &input.payment_method,
            &input.notes,
        ),
    )
    .map_err(AppError::from)?;

    for it in &input.items {
        let line_total = it.quantity as f64 * it.unit_price;
        let item_id = next_id(conn, "GEN_INVOICE_ITEMS_ID")?;
        conn.execute(
            "INSERT INTO INVOICE_ITEMS
                (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE, LINE_TOTAL)
             VALUES (?, ?, ?, ?, ?, ?)",
            (&item_id, &id, &it.description, &it.quantity, &it.unit_price, &line_total),
        )
        .map_err(AppError::from)?;
    }

    get(conn, id)?.ok_or_else(|| {
        AppError::Internal("Factura creada pero no recuperada".into())
    })
}

/// Listado de facturas con filtros opcionales por estado y búsqueda
/// (número, propietario o paciente).
pub fn list(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<InvoiceListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = "
        SELECT i.ID, i.INVOICE_NUMBER, o.FULL_NAME, p.NAME,
               LEFT(CAST(i.ISSUE_DATE AS VARCHAR(60)), 19),
               CAST(i.TOTAL AS DOUBLE PRECISION),
               i.STATUS, i.PAYMENT_METHOD,
               (SELECT COUNT(*) FROM INVOICE_ITEMS ii WHERE ii.INVOICE_ID = i.ID)
        FROM INVOICES i
        JOIN OWNERS o ON o.ID = i.OWNER_ID
        LEFT JOIN PATIENTS p ON p.ID = i.PATIENT_ID
        WHERE (? IS NULL OR i.STATUS = ?)
          AND (? IS NULL
               OR UPPER(i.INVOICE_NUMBER) LIKE UPPER(?)
               OR UPPER(o.FULL_NAME) LIKE UPPER(?)
               OR UPPER(COALESCE(p.NAME, '')) LIKE UPPER(?))
        ORDER BY i.ISSUE_DATE DESC, i.ID DESC";

    let rows: Vec<(
        i32, String, String, Option<String>, String, f64, String,
        Option<String>, i32,
    )> = conn
        .query(&sql, (&status, &status, &like, &like, &like, &like))
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| InvoiceListItem {
            id: r.0,
            invoice_number: r.1,
            owner_name: r.2,
            patient_name: r.3,
            issue_date: r.4,
            total: r.5,
            status: r.6,
            payment_method: r.7,
            item_count: r.8,
        })
        .collect())
}

/// Cambia el estado de una factura: EMITIDA → PAGADA/ANULADA, PAGADA → ANULADA.
pub fn set_status(
    conn: &mut SimpleConnection,
    id: i32,
    status: &str,
) -> Result<Invoice, AppError> {
    let current: Option<(String,)> = conn
        .query_first("SELECT STATUS FROM INVOICES WHERE ID = ?", (&id,))
        .map_err(AppError::from)?;
    let (current,) = current.ok_or_else(|| {
        AppError::NotFound(format!("Factura {id} no encontrada"))
    })?;

    let allowed = match status {
        "PAGADA" => current == "EMITIDA",
        "ANULADA" => matches!(current.as_str(), "EMITIDA" | "PAGADA"),
        _ => false,
    };
    if !allowed {
        return Err(AppError::Validation(format!(
            "Transición de estado no permitida: {current} → {status} \
             (EMITIDA→PAGADA, →ANULADA)"
        )));
    }

    conn.execute(
        "UPDATE INVOICES
            SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ?",
        (&status, &id),
    )
    .map_err(AppError::from)?;

    get(conn, id)?.ok_or_else(|| {
        AppError::Internal("Factura actualizada pero no recuperada".into())
    })
}
