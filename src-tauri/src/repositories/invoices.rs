use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::invoice::{CreateInvoiceInput, Invoice, InvoiceItem, InvoiceListItem};
use crate::repositories::next_id;

pub(crate) type InvoiceListItemRow = (
    i32,            // id
    String,         // invoice_number
    String,         // owner_name
    Option<String>, // patient_name
    String,         // issue_date
    f64,            // total
    String,         // status
    Option<String>, // payment_method
    i32,            // item_count
    f64,            // paid_amount
    f64,            // balance
    String,         // payment_status
);

/// Subconsulta de abonos aplicados: suma solo asignaciones de pagos vigentes.
const PAID_SUBQUERY: &str = "COALESCE((SELECT CAST(SUM(a.AMOUNT) AS DOUBLE PRECISION)
        FROM PAYMENT_ALLOCATIONS a
        JOIN PAYMENTS pa ON pa.ID = a.PAYMENT_ID
        WHERE a.INVOICE_ID = i.ID AND pa.STATUS = 'APLICADO'), 0)";

/// Columnas de una factura con propietario/paciente unidos.
const INVOICE_SELECT: &str = "
    SELECT i.ID, i.INVOICE_NUMBER, i.PATIENT_ID, p.NAME,
           i.OWNER_ID, o.FULL_NAME, o.PHONE, i.CONSULTATION_ID,
           LEFT(CAST(i.ISSUE_DATE AS VARCHAR(60)), 19),
           CAST(i.SUBTOTAL AS DOUBLE PRECISION),
           CAST(i.TAX_RATE AS DOUBLE PRECISION),
           CAST(i.TAX_AMOUNT AS DOUBLE PRECISION),
           CAST(i.TOTAL AS DOUBLE PRECISION),
           i.STATUS, i.PAYMENT_METHOD, i.NOTES
    FROM INVOICES i
    JOIN OWNERS o ON o.ID = i.OWNER_ID
    LEFT JOIN PATIENTS p ON p.ID = i.PATIENT_ID";

pub(crate) type InvoiceRow = (
    i32,            // id
    String,         // invoice_number
    Option<i32>,    // patient_id
    Option<String>, // patient_name
    i32,            // owner_id
    String,         // owner_name
    Option<String>, // owner_phone
    Option<i32>,    // consultation_id
    String,         // issue_date
    f64,            // subtotal
    f64,            // tax_rate
    f64,            // tax_amount
    f64,            // total
    String,         // status
    Option<String>, // payment_method
    Option<String>, // notes
);

pub(crate) fn map_invoice(r: InvoiceRow, items: Vec<InvoiceItem>) -> Invoice {
    Invoice {
        id: r.0,
        invoice_number: r.1,
        patient_id: r.2,
        patient_name: r.3,
        owner_id: r.4,
        owner_name: r.5,
        owner_phone: r.6,
        consultation_id: r.7,
        issue_date: r.8,
        subtotal: r.9,
        tax_rate: r.10,
        tax_amount: r.11,
        total: r.12,
        status: r.13,
        payment_method: r.14,
        notes: r.15,
        items,
    }
}

fn list_items(conn: &mut SimpleConnection, invoice_id: i32) -> Result<Vec<InvoiceItem>, AppError> {
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

    // Cabecera + líneas en una sola transacción: con 3 PCs escribiendo, una
    // factura a medias (sin todos sus ítems) sería imposible de cobrar.
    db::tx::in_transaction(conn, |conn| {
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
                    (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE, LINE_TOTAL, SERVICE_ID)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    &item_id,
                    &id,
                    &it.description,
                    &it.quantity,
                    &it.unit_price,
                    &line_total,
                    &it.service_id,
                ),
            )
            .map_err(AppError::from)?;
        }

        get(conn, id)?.ok_or_else(|| AppError::Internal("Factura creada pero no recuperada".into()))
    })
}

/// Listado de facturas con filtro por estado de cobro (`paymentStatus`) o por
/// estado histórico (`status`), más búsqueda libre (número, propietario o
/// paciente). El saldo y el estado de cobro se derivan en SQL.
pub fn list(
    conn: &mut SimpleConnection,
    status: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<InvoiceListItem>, AppError> {
    let like = search
        .map(|s| format!("%{}%", s.trim()))
        .filter(|s| !s.trim_matches('%').is_empty());

    let sql = format!(
        "SELECT y.ID, y.INVOICE_NUMBER, o.FULL_NAME, p.NAME, y.ISSUE_DATE,
                y.TOTAL, y.STATUS, y.PAYMENT_METHOD,
                (SELECT COUNT(*) FROM INVOICE_ITEMS ii WHERE ii.INVOICE_ID = y.ID),
                y.PAID, y.BALANCE, y.PAYMENT_STATUS
         FROM (
             SELECT x.ID, x.INVOICE_NUMBER, x.OWNER_ID, x.PATIENT_ID,
                    x.ISSUE_DATE, x.TOTAL, x.STATUS, x.PAYMENT_METHOD, x.PAID,
                    CASE WHEN x.TOTAL - x.PAID < 0 THEN 0
                         ELSE x.TOTAL - x.PAID END AS BALANCE,
                    CASE
                        WHEN x.STATUS = 'ANULADA' THEN 'ANULADA'
                        WHEN x.TOTAL - x.PAID <= 0.01 THEN 'PAGADA'
                        WHEN x.PAID > 0.01 THEN 'ABONADA'
                        ELSE 'PENDIENTE'
                    END AS PAYMENT_STATUS
             FROM (
                 SELECT i.ID, i.INVOICE_NUMBER, i.OWNER_ID, i.PATIENT_ID,
                        LEFT(CAST(i.ISSUE_DATE AS VARCHAR(60)), 19) AS ISSUE_DATE,
                        CAST(i.TOTAL AS DOUBLE PRECISION) AS TOTAL,
                        i.STATUS, i.PAYMENT_METHOD,
                        {PAID_SUBQUERY} AS PAID
                 FROM INVOICES i
             ) x
         ) y
         JOIN OWNERS o ON o.ID = y.OWNER_ID
         LEFT JOIN PATIENTS p ON p.ID = y.PATIENT_ID
         WHERE (? IS NULL OR y.PAYMENT_STATUS = ? OR y.STATUS = ?)
           AND (? IS NULL
                OR UPPER(y.INVOICE_NUMBER) LIKE UPPER(?)
                OR UPPER(o.FULL_NAME) LIKE UPPER(?)
                OR UPPER(COALESCE(p.NAME, '')) LIKE UPPER(?))
         ORDER BY y.ISSUE_DATE DESC, y.ID DESC"
    );

    let rows: Vec<InvoiceListItemRow> = conn
        .query(
            &sql,
            (&status, &status, &status, &like, &like, &like, &like),
        )
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
            paid_amount: r.9,
            balance: r.10,
            payment_status: r.11,
        })
        .collect())
}

/// Anula una factura. No existe transición manual a `PAGADA`: una factura
/// queda pagada solo cuando sus abonos vigentes cubren el total, y eso lo
/// decide `payments::register`. Tampoco se anula una factura con pagos
/// vigentes: primero se anulan los pagos, que liberan el saldo.
///
/// El UPDATE es un compare-and-set: valida estado, ausencia de pagos y
/// concurrencia en una sola sentencia atómica (otra PC puede estar cobrando).
pub fn set_status(conn: &mut SimpleConnection, id: i32, status: &str) -> Result<Invoice, AppError> {
    let status = status.trim().to_uppercase();
    if status != "ANULADA" {
        return Err(AppError::Validation(
            "El único cambio manual de estado es la anulación: una factura se marca \
             como pagada al registrar sus pagos."
                .into(),
        ));
    }

    let updated = conn
        .execute(
            "UPDATE INVOICES
                SET STATUS = 'ANULADA', UPDATED_AT = CURRENT_TIMESTAMP
              WHERE ID = ? AND STATUS IN ('EMITIDA', 'PAGADA')
                AND NOT EXISTS (
                    SELECT 1 FROM PAYMENT_ALLOCATIONS a
                    JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                    WHERE a.INVOICE_ID = ? AND p.STATUS = 'APLICADO'
                )",
            (&id, &id),
        )
        .map_err(AppError::from)?;

    if updated == 0 {
        // Diagnóstico del porqué: no existe, ya estaba anulada o tiene pagos.
        let row: Option<(String, f64)> = conn
            .query_first(
                "SELECT i.STATUS, CAST(COALESCE((
                     SELECT SUM(a.AMOUNT)
                     FROM PAYMENT_ALLOCATIONS a
                     JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                     WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
                 ), 0) AS DOUBLE PRECISION)
                 FROM INVOICES i WHERE i.ID = ?",
                (&id,),
            )
            .map_err(AppError::from)?;
        let Some((current, paid)) = row else {
            return Err(AppError::NotFound(format!("Factura {id} no encontrada")));
        };
        if current == "ANULADA" {
            return Err(AppError::Validation(format!(
                "La factura {id} ya está anulada"
            )));
        }
        if paid > 0.01 {
            return Err(AppError::Validation(format!(
                "La factura tiene pagos vigentes por {paid:.2}: anula primero los pagos \
                 antes de anularla"
            )));
        }
        return Err(AppError::Validation(format!(
            "No se puede anular una factura en estado {current}"
        )));
    }

    get(conn, id)?
        .ok_or_else(|| AppError::Internal("Factura actualizada pero no recuperada".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_invoice_all_fields() {
        let row: InvoiceRow = (
            1,                              // id
            "FAC-000001".into(),            // invoice_number
            Some(10),                       // patient_id
            Some("Luna".into()),            // patient_name
            100,                            // owner_id
            "Juan Pérez".into(),            // owner_name
            Some("+57 300 1234567".into()), // owner_phone
            Some(5),                        // consultation_id
            "2026-08-01 10:00:00".into(),   // issue_date
            150000.0,                       // subtotal
            19.0,                           // tax_rate
            28500.0,                        // tax_amount
            178500.0,                       // total
            "EMITIDA".into(),               // status
            Some("EFECTIVO".into()),        // payment_method
            Some("Pago completo".into()),   // notes
        );
        let items = vec![InvoiceItem {
            id: 1,
            description: "Consulta general".into(),
            quantity: 1,
            unit_price: 150000.0,
            line_total: 150000.0,
        }];
        let invoice = map_invoice(row, items);

        assert_eq!(invoice.id, 1);
        assert_eq!(invoice.invoice_number, "FAC-000001");
        assert_eq!(invoice.patient_id, Some(10));
        assert_eq!(invoice.patient_name.as_deref(), Some("Luna"));
        assert_eq!(invoice.owner_id, 100);
        assert_eq!(invoice.owner_name, "Juan Pérez");
        assert_eq!(invoice.owner_phone.as_deref(), Some("+57 300 1234567"));
        assert_eq!(invoice.consultation_id, Some(5));
        assert_eq!(invoice.issue_date, "2026-08-01 10:00:00");
        assert!((invoice.subtotal - 150000.0).abs() < f64::EPSILON);
        assert!((invoice.tax_rate - 19.0).abs() < f64::EPSILON);
        assert!((invoice.tax_amount - 28500.0).abs() < f64::EPSILON);
        assert!((invoice.total - 178500.0).abs() < f64::EPSILON);
        assert_eq!(invoice.status, "EMITIDA");
        assert_eq!(invoice.payment_method.as_deref(), Some("EFECTIVO"));
        assert_eq!(invoice.notes.as_deref(), Some("Pago completo"));
        assert_eq!(invoice.items.len(), 1);
        assert_eq!(invoice.items[0].description, "Consulta general");
    }

    #[test]
    fn test_map_invoice_optional_fields_none() {
        let row: InvoiceRow = (
            2,
            "FAC-000002".into(),
            None,
            None,
            200,
            "María López".into(),
            None,
            None,
            "2026-08-02 14:00:00".into(),
            0.0,
            19.0,
            0.0,
            0.0,
            "EMITIDA".into(),
            None,
            None,
        );
        let invoice = map_invoice(row, vec![]);

        assert_eq!(invoice.id, 2);
        assert_eq!(invoice.patient_id, None);
        assert_eq!(invoice.patient_name, None);
        assert_eq!(invoice.owner_phone, None);
        assert_eq!(invoice.consultation_id, None);
        assert_eq!(invoice.payment_method, None);
        assert_eq!(invoice.notes, None);
        assert!(invoice.items.is_empty());
    }

    #[test]
    fn test_invoice_list_item_row_mapping() {
        let row: InvoiceListItemRow = (
            1,
            "FAC-000001".into(),
            "Juan Pérez".into(),
            Some("Luna".into()),
            "2026-08-01 10:00:00".into(),
            178500.0,
            "EMITIDA".into(),
            Some("EFECTIVO".into()),
            3,
            50000.0,
            128500.0,
            "ABONADA".into(),
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "FAC-000001");
        assert_eq!(row.2, "Juan Pérez");
        assert_eq!(row.3.as_deref(), Some("Luna"));
        assert!((row.5 - 178500.0).abs() < f64::EPSILON);
        assert_eq!(row.8, 3); // item_count
        assert_eq!(row.11, "ABONADA");
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_helpers;

    fn setup() -> (crate::test_helpers::TestDb, std::path::PathBuf) {
        let (mut conn, db_path) = test_helpers::setup_test_db();
        // Insertar propietario para facturación
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (1, 'CC', '1234567890', 'Juan Pérez')",
            (),
        )
        .unwrap();
        (conn, db_path)
    }

    #[test]
    fn test_create_invoice() {
        let (mut conn, db_path) = setup();

        let input = CreateInvoiceInput {
            patient_id: None,
            owner_id: 1,
            consultation_id: None,
            tax_rate: Some(19.0),
            payment_method: Some("EFECTIVO".into()),
            notes: Some("Test invoice".into()),
            items: vec![
                crate::models::invoice::CreateInvoiceItemInput {
                    description: "Consulta general".into(),
                    quantity: 1,
                    unit_price: 100000.0,
                    service_id: Some(1),
                },
                crate::models::invoice::CreateInvoiceItemInput {
                    description: "Laboratorio".into(),
                    quantity: 2,
                    unit_price: 50000.0,
                    service_id: Some(9),
                },
            ],
        };

        let invoice = create(&mut conn, &input).unwrap();
        assert_eq!(invoice.owner_name, "Juan Pérez");
        assert_eq!(invoice.status, "EMITIDA");
        assert_eq!(invoice.items.len(), 2);
        // Subtotal = 100000 + 100000 = 200000
        assert!((invoice.subtotal - 200000.0).abs() < 0.01);
        // Tax = 200000 * 19% = 38000
        assert!((invoice.tax_amount - 38000.0).abs() < 0.01);
        // Total = 238000
        assert!((invoice.total - 238000.0).abs() < 0.01);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_create_invoice_empty_items_fails() {
        let (mut conn, db_path) = setup();

        let input = CreateInvoiceInput {
            patient_id: None,
            owner_id: 1,
            consultation_id: None,
            tax_rate: Some(19.0),
            payment_method: None,
            notes: None,
            items: vec![],
        };

        let result = create(&mut conn, &input);
        assert!(result.is_err());

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_get_invoice() {
        let (mut conn, db_path) = setup();

        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 19.0, 19000.0, 119000.0, 'EMITIDA')",
            (),
        ).unwrap();
        conn.execute(
            "INSERT INTO INVOICE_ITEMS (ID, INVOICE_ID, DESCRIPTION, QUANTITY, UNIT_PRICE, LINE_TOTAL)
             VALUES (1, 1, 'Consulta', 1, 100000.0, 100000.0)",
            (),
        ).unwrap();

        let invoice = get(&mut conn, 1).unwrap();
        assert!(invoice.is_some());
        let inv = invoice.unwrap();
        assert_eq!(inv.invoice_number, "FAC-000001");
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].description, "Consulta");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_list_invoices() {
        let (mut conn, db_path) = setup();

        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 19.0, 19000.0, 119000.0, 'EMITIDA')",
            (),
        ).unwrap();
        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (2, 'FAC-000002', 1, 50000.0, 19.0, 9500.0, 59500.0, 'PAGADA')",
            (),
        ).unwrap();

        // Filtrar por estado
        let invoices = list(&mut conn, Some("EMITIDA"), None).unwrap();
        assert_eq!(invoices.len(), 1);
        assert_eq!(invoices[0].invoice_number, "FAC-000001");

        // Sin filtro
        let invoices = list(&mut conn, None, None).unwrap();
        assert_eq!(invoices.len(), 2);

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_invoice_status_cannot_mark_as_paid_manually() {
        let (mut conn, db_path) = setup();

        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 19.0, 19000.0, 119000.0, 'EMITIDA')",
            (),
        ).unwrap();

        // El atajo a PAGADA quedó cerrado: solo un pago real la marca pagada.
        let result = set_status(&mut conn, 1, "PAGADA");
        assert!(matches!(result, Err(AppError::Validation(_))));
        assert_eq!(get(&mut conn, 1).unwrap().unwrap().status, "EMITIDA");

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_invoice_status_anula_una_sola_vez() {
        let (mut conn, db_path) = setup();

        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 19.0, 19000.0, 119000.0, 'EMITIDA')",
            (),
        ).unwrap();

        let updated = set_status(&mut conn, 1, "ANULADA").unwrap();
        assert_eq!(updated.status, "ANULADA");

        // Anular dos veces no es una transición válida.
        let again = set_status(&mut conn, 1, "ANULADA");
        assert!(matches!(again, Err(AppError::Validation(_))));

        test_helpers::cleanup_test_db(&db_path);
    }

    #[test]
    fn test_set_invoice_status_invalid_transition() {
        let (mut conn, db_path) = setup();

        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 19.0, 19000.0, 119000.0, 'EMITIDA')",
            (),
        ).unwrap();

        // EMITIDA -> EMITIDA (no permitido)
        let result = set_status(&mut conn, 1, "EMITIDA");
        assert!(result.is_err());

        test_helpers::cleanup_test_db(&db_path);
    }

    /// Una factura con abonos vigentes no se anula: primero hay que anular los
    /// pagos (que devuelven el saldo). Con el pago anulado, sí.
    #[test]
    fn test_cannot_cancel_invoice_with_active_payments() {
        use crate::models::payment::{
            CreatePaymentAllocationInput, CreatePaymentLineInput, RegisterPaymentInput,
        };
        use crate::repositories::{cash, payments};

        let (mut conn, db_path) = setup();
        conn.execute(
            "INSERT INTO INVOICES (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
             VALUES (1, 'FAC-000001', 1, 100000.0, 0, 0, 100000.0, 'EMITIDA')",
            (),
        )
        .unwrap();
        cash::open_session(&mut conn, 0.0, None, 1).unwrap();
        let payment = payments::register(
            &mut conn,
            &RegisterPaymentInput {
                owner_id: 1,
                patient_id: None,
                notes: None,
                lines: vec![CreatePaymentLineInput {
                    method_code: "EFECTIVO".into(),
                    amount: 40000.0,
                    reference: None,
                }],
                allocations: vec![CreatePaymentAllocationInput {
                    invoice_id: 1,
                    amount: 40000.0,
                }],
            },
            Some(1),
        )
        .unwrap();

        let blocked = set_status(&mut conn, 1, "ANULADA");
        assert!(matches!(blocked, Err(AppError::Validation(_))));
        assert_eq!(get(&mut conn, 1).unwrap().unwrap().status, "EMITIDA");

        // Anulado el pago, el saldo vuelve y la factura se puede anular.
        payments::void(&mut conn, payment.id, "Cobro equivocado", Some(1)).unwrap();
        assert_eq!(
            set_status(&mut conn, 1, "ANULADA").unwrap().status,
            "ANULADA"
        );

        test_helpers::cleanup_test_db(&db_path);
    }
}

/// Conteo de facturas agrupado por estado (para las pestañas de facturación).
pub fn count_by_status(
    conn: &mut SimpleConnection,
) -> Result<Vec<crate::models::status_count::StatusCount>, AppError> {
    // Cuenta por estado de cobro derivado, no por el estado histórico: la UI
    // usa estas pestañas (Pendientes / Abonadas / Pagadas / Anuladas).
    let sql = format!(
        "SELECT PAYMENT_STATUS, COUNT(*)
         FROM (
             SELECT CASE
                        WHEN x.STATUS = 'ANULADA' THEN 'ANULADA'
                        WHEN x.TOTAL - x.PAID <= 0.01 THEN 'PAGADA'
                        WHEN x.PAID > 0.01 THEN 'ABONADA'
                        ELSE 'PENDIENTE'
                    END AS PAYMENT_STATUS
             FROM (
                 SELECT i.STATUS, CAST(i.TOTAL AS DOUBLE PRECISION) AS TOTAL,
                        {PAID_SUBQUERY} AS PAID
                 FROM INVOICES i
             ) x
         )
         GROUP BY PAYMENT_STATUS
         ORDER BY PAYMENT_STATUS"
    );
    let rows: Vec<(String, i32)> = conn.query(&sql, ()).map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(status, count)| crate::models::status_count::StatusCount { status, count })
        .collect())
}
