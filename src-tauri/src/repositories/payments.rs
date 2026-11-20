use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::payment::{
    BillingMethodTotal, BillingReport, InvoiceBalance, Payment, PaymentAllocation, PaymentLine,
    PaymentMethod, RegisterPaymentInput,
};
use crate::repositories::{cash, next_id};

/// Tolerancia de comparación monetaria (D_AMOUNT tiene 2 decimales).
const MONEY_EPSILON: f64 = 0.01;

type PaymentRow = (
    i32,            // id
    String,         // code
    i32,            // owner_id
    String,         // owner_name
    Option<i32>,    // patient_id
    Option<String>, // patient_name
    Option<i32>,    // cash_session_id
    String,         // received_at
    f64,            // total
    String,         // status
    Option<String>, // notes
    Option<String>, // created_by_name
);

const PAYMENT_SELECT: &str = "
    SELECT p.ID, p.CODE, p.OWNER_ID, o.FULL_NAME, p.PATIENT_ID, pa.NAME,
           p.CASH_SESSION_ID, LEFT(CAST(p.RECEIVED_AT AS VARCHAR(60)), 19),
           CAST(p.TOTAL AS DOUBLE PRECISION), p.STATUS, p.NOTES, u.FULL_NAME
    FROM PAYMENTS p
    JOIN OWNERS o ON o.ID = p.OWNER_ID
    LEFT JOIN PATIENTS pa ON pa.ID = p.PATIENT_ID
    LEFT JOIN USERS u ON u.ID = p.CREATED_BY";

fn map_payment(
    r: PaymentRow,
    lines: Vec<PaymentLine>,
    allocations: Vec<PaymentAllocation>,
) -> Payment {
    Payment {
        id: r.0,
        code: r.1,
        owner_id: r.2,
        owner_name: r.3,
        patient_id: r.4,
        patient_name: r.5,
        cash_session_id: r.6,
        received_at: r.7,
        total: r.8,
        status: r.9,
        notes: r.10,
        created_by_name: r.11,
        lines,
        allocations,
    }
}

fn list_lines(conn: &mut SimpleConnection, payment_id: i32) -> Result<Vec<PaymentLine>, AppError> {
    let rows: Vec<(i32, String, String, String, f64, Option<String>)> = conn
        .query(
            "SELECT l.ID, l.METHOD_CODE, m.NAME, m.KIND,
                    CAST(l.AMOUNT AS DOUBLE PRECISION), l.REFERENCE
             FROM PAYMENT_LINES l
             JOIN PAYMENT_METHODS m ON m.CODE = l.METHOD_CODE
             WHERE l.PAYMENT_ID = ?
             ORDER BY l.ID",
            (&payment_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| PaymentLine {
            id: r.0,
            method_code: r.1,
            method_name: r.2,
            method_kind: r.3,
            amount: r.4,
            reference: r.5,
        })
        .collect())
}

fn list_allocations(
    conn: &mut SimpleConnection,
    payment_id: i32,
) -> Result<Vec<PaymentAllocation>, AppError> {
    let rows: Vec<(i32, i32, String, f64)> = conn
        .query(
            "SELECT a.ID, a.INVOICE_ID, i.INVOICE_NUMBER,
                    CAST(a.AMOUNT AS DOUBLE PRECISION)
             FROM PAYMENT_ALLOCATIONS a
             JOIN INVOICES i ON i.ID = a.INVOICE_ID
             WHERE a.PAYMENT_ID = ?
             ORDER BY a.ID",
            (&payment_id,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| PaymentAllocation {
            id: r.0,
            invoice_id: r.1,
            invoice_number: r.2,
            amount: r.3,
        })
        .collect())
}

fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<Payment>, AppError> {
    let row: Option<PaymentRow> = conn
        .query_first(&format!("{PAYMENT_SELECT} WHERE p.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    let Some(row) = row else { return Ok(None) };
    let lines = list_lines(conn, id)?;
    let allocations = list_allocations(conn, id)?;
    Ok(Some(map_payment(row, lines, allocations)))
}

/// Historial de recepciones aplicadas a una factura (incluye anuladas).
pub fn list_by_invoice(
    conn: &mut SimpleConnection,
    invoice_id: i32,
) -> Result<Vec<Payment>, AppError> {
    let ids: Vec<(i32,)> = conn
        .query(
            "SELECT DISTINCT a.PAYMENT_ID
             FROM PAYMENT_ALLOCATIONS a
             JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
             WHERE a.INVOICE_ID = ?
             ORDER BY a.PAYMENT_ID",
            (&invoice_id,),
        )
        .map_err(AppError::from)?;

    ids.into_iter()
        .map(|(id,)| get(conn, id)?.ok_or_else(|| AppError::Internal("Pago no recuperado".into())))
        .collect()
}

pub fn list_methods(
    conn: &mut SimpleConnection,
    include_inactive: bool,
) -> Result<Vec<PaymentMethod>, AppError> {
    let rows: Vec<(String, String, String, bool, bool, i32)> = conn
        .query(
            "SELECT CODE, NAME, KIND, REQUIRES_REFERENCE, ACTIVE, SORT_ORDER
             FROM PAYMENT_METHODS
             WHERE ACTIVE = TRUE OR ? = TRUE
             ORDER BY SORT_ORDER, CODE",
            (&include_inactive,),
        )
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .map(|r| PaymentMethod {
            code: r.0,
            name: r.1,
            kind: r.2,
            requires_reference: r.3,
            active: r.4,
            sort_order: r.5,
        })
        .collect())
}

/// Suma de asignaciones de pagos APLICADO sobre una factura.
fn paid_amount(conn: &mut SimpleConnection, invoice_id: i32) -> Result<f64, AppError> {
    let row: Option<(f64,)> = conn
        .query_first(
            "SELECT COALESCE(CAST(SUM(a.AMOUNT) AS DOUBLE PRECISION), 0)
             FROM PAYMENT_ALLOCATIONS a
             JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
             WHERE a.INVOICE_ID = ? AND p.STATUS = 'APLICADO'",
            (&invoice_id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(|(v,)| v).unwrap_or(0.0))
}

/// Recalcula el estado histórico de una factura a partir de sus abonos
/// vigentes: `PAGADA` si el saldo llegó a cero y `EMITIDA` si no. Nunca toca
/// una factura anulada. Es el único lugar donde se decide ese estado, para que
/// registrar y anular un pago no puedan divergir.
fn recompute_invoice_status(conn: &mut SimpleConnection, invoice_id: i32) -> Result<(), AppError> {
    let row: Option<(String, f64)> = conn
        .query_first(
            "SELECT STATUS, CAST(TOTAL AS DOUBLE PRECISION) FROM INVOICES WHERE ID = ?",
            (&invoice_id,),
        )
        .map_err(AppError::from)?;
    let Some((status, total)) = row else {
        return Ok(());
    };
    if status == "ANULADA" {
        return Ok(());
    }

    let paid = paid_amount(conn, invoice_id)?;
    let next = if paid + MONEY_EPSILON >= total {
        "PAGADA"
    } else {
        "EMITIDA"
    };
    conn.execute(
        "UPDATE INVOICES
            SET STATUS = ?, UPDATED_AT = CURRENT_TIMESTAMP
          WHERE ID = ? AND STATUS <> 'ANULADA'",
        (&next, &invoice_id),
    )
    .map_err(AppError::from)?;
    Ok(())
}

/// Saldo y estado de cobro derivados de una factura.
pub fn invoice_balance(
    conn: &mut SimpleConnection,
    invoice_id: i32,
) -> Result<InvoiceBalance, AppError> {
    let row: Option<(String, String, f64)> = conn
        .query_first(
            "SELECT INVOICE_NUMBER, STATUS, CAST(TOTAL AS DOUBLE PRECISION)
             FROM INVOICES WHERE ID = ?",
            (&invoice_id,),
        )
        .map_err(AppError::from)?;
    let Some((invoice_number, status, total)) = row else {
        return Err(AppError::NotFound(format!(
            "Factura {invoice_id} no encontrada"
        )));
    };

    let paid = paid_amount(conn, invoice_id)?;
    let balance = (total - paid).max(0.0);
    let payment_status = if status == "ANULADA" {
        "ANULADA"
    } else if balance <= MONEY_EPSILON {
        "PAGADA"
    } else if paid > MONEY_EPSILON {
        "ABONADA"
    } else {
        "PENDIENTE"
    };

    Ok(InvoiceBalance {
        invoice_id,
        invoice_number,
        total,
        paid_amount: paid,
        balance,
        payment_status: payment_status.into(),
        status,
    })
}

struct NormalizedLine {
    method_code: String,
    amount: f64,
    reference: Option<String>,
    kind: String,
}

/// Registra una recepción (abono o pago total) con una o varias formas de pago
/// y la aplica a una o varias facturas del mismo propietario.
pub fn register(
    conn: &mut SimpleConnection,
    input: &RegisterPaymentInput,
    user_id: Option<i32>,
) -> Result<Payment, AppError> {
    if input.lines.is_empty() {
        return Err(AppError::Validation(
            "El pago debe tener al menos una forma de pago".into(),
        ));
    }
    if input.allocations.is_empty() {
        return Err(AppError::Validation(
            "El pago debe aplicarse a al menos una factura".into(),
        ));
    }

    let mut lines = Vec::with_capacity(input.lines.len());
    let mut lines_total = 0.0;
    for line in &input.lines {
        if line.amount <= 0.0 {
            return Err(AppError::Validation(
                "Cada forma de pago debe tener un importe mayor que cero".into(),
            ));
        }
        let method_code = line.method_code.trim().to_uppercase();
        let method: Option<(String, bool)> = conn
            .query_first(
                "SELECT KIND, ACTIVE FROM PAYMENT_METHODS WHERE CODE = ?",
                (&method_code,),
            )
            .map_err(AppError::from)?;
        let Some((kind, active)) = method else {
            return Err(AppError::Validation(format!(
                "Método de pago desconocido: {method_code}"
            )));
        };
        if !active {
            return Err(AppError::Validation(format!(
                "El método de pago {method_code} está inactivo"
            )));
        }
        // La referencia viaja a VARCHAR(80): sin validar, un texto largo
        // terminaría en un error de truncamiento de Firebird en vez de un
        // mensaje de validación.
        let reference = line
            .reference
            .as_deref()
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(str::to_string);
        if let Some(reference) = &reference {
            if reference.chars().count() > 80 {
                return Err(AppError::Validation(
                    "La referencia del pago no puede superar 80 caracteres".into(),
                ));
            }
        }
        lines_total += line.amount;
        lines.push(NormalizedLine {
            method_code,
            amount: line.amount,
            reference,
            kind,
        });
    }

    let allocations_total: f64 = input.allocations.iter().map(|a| a.amount).sum();
    if (lines_total - allocations_total).abs() > MONEY_EPSILON {
        return Err(AppError::Validation(format!(
            "La suma de las formas de pago ({lines_total:.2}) no coincide con \
             lo aplicado a las facturas ({allocations_total:.2})"
        )));
    }

    // Ordenar por factura antes de bloquear: dos pagos que cubren las mismas
    // facturas en distinto orden no pueden quedar en deadlock.
    let mut ordered: Vec<_> = input.allocations.iter().collect();
    ordered.sort_by_key(|a| a.invoice_id);

    db::tx::in_transaction(conn, |conn| {
        let session = cash::require_open_session(conn)?;

        // El paciente es informativo, pero no puede ser de otro tutor: el
        // recibo quedaría atribuido a un paciente que no es del pagador.
        if let Some(patient_id) = input.patient_id {
            let owner: Option<(i32,)> = conn
                .query_first("SELECT OWNER_ID FROM PATIENTS WHERE ID = ?", (&patient_id,))
                .map_err(AppError::from)?;
            let Some((patient_owner,)) = owner else {
                return Err(AppError::Validation(format!(
                    "El paciente {patient_id} no existe"
                )));
            };
            if patient_owner != input.owner_id {
                return Err(AppError::Validation(
                    "El paciente del pago no pertenece al propietario indicado".into(),
                ));
            }
        }

        // Bloquear cada factura y validar saldo antes de insertar nada.
        let mut updates: Vec<i32> = Vec::with_capacity(ordered.len());
        for allocation in ordered {
            if allocation.amount <= 0.0 {
                return Err(AppError::Validation(
                    "Cada asignación debe tener un importe mayor que cero".into(),
                ));
            }
            if updates.contains(&allocation.invoice_id) {
                return Err(AppError::Validation(
                    "Una factura solo puede aparecer una vez en el pago".into(),
                ));
            }

            let row: Option<(i32, String, f64)> = conn
                .query_first(
                    "SELECT OWNER_ID, STATUS, CAST(TOTAL AS DOUBLE PRECISION)
                     FROM INVOICES WHERE ID = ? WITH LOCK",
                    (&allocation.invoice_id,),
                )
                .map_err(AppError::from)?;
            let Some((owner_id, status, total)) = row else {
                return Err(AppError::NotFound(format!(
                    "Factura {} no encontrada",
                    allocation.invoice_id
                )));
            };
            if owner_id != input.owner_id {
                return Err(AppError::Validation(format!(
                    "La factura {} pertenece a otro propietario",
                    allocation.invoice_id
                )));
            }
            if status == "ANULADA" {
                return Err(AppError::Validation(format!(
                    "La factura {} está anulada",
                    allocation.invoice_id
                )));
            }

            let paid = paid_amount(conn, allocation.invoice_id)?;
            let balance = (total - paid).max(0.0);
            if allocation.amount > balance + MONEY_EPSILON {
                return Err(AppError::Validation(format!(
                    "La asignación de {:.2} supera el saldo de {:.2} de la factura {}",
                    allocation.amount, balance, allocation.invoice_id
                )));
            }
            updates.push(allocation.invoice_id);
        }

        let payment_id = next_id(conn, "GEN_PAYMENTS_ID")?;
        conn.execute(
            "INSERT INTO PAYMENTS
                (ID, OWNER_ID, PATIENT_ID, CASH_SESSION_ID, TOTAL, NOTES, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                &payment_id,
                &input.owner_id,
                &input.patient_id,
                &session.id,
                &lines_total,
                &input.notes,
                &user_id,
            ),
        )
        .map_err(AppError::from)?;

        for line in &lines {
            let line_id = next_id(conn, "GEN_PAYMENT_LINES_ID")?;
            conn.execute(
                "INSERT INTO PAYMENT_LINES
                    (ID, PAYMENT_ID, METHOD_CODE, AMOUNT, REFERENCE)
                 VALUES (?, ?, ?, ?, ?)",
                (
                    &line_id,
                    &payment_id,
                    &line.method_code,
                    &line.amount,
                    &line.reference,
                ),
            )
            .map_err(AppError::from)?;

            // Solo el efectivo entra al arqueo de caja.
            if line.kind == "EFECTIVO" {
                let movement_id = next_id(conn, "GEN_CASH_MOVEMENTS_ID")?;
                conn.execute(
                    "INSERT INTO CASH_MOVEMENTS
                        (ID, CASH_SESSION_ID, KIND, AMOUNT, PAYMENT_ID, REASON, CREATED_BY)
                     VALUES (?, ?, 'PAGO_EFECTIVO', ?, ?, 'Pago en efectivo', ?)",
                    (
                        &movement_id,
                        &session.id,
                        &line.amount,
                        &payment_id,
                        &user_id,
                    ),
                )
                .map_err(AppError::from)?;
            }
        }

        for allocation in &input.allocations {
            let allocation_id = next_id(conn, "GEN_PAYMENT_ALLOCATIONS_ID")?;
            conn.execute(
                "INSERT INTO PAYMENT_ALLOCATIONS
                    (ID, PAYMENT_ID, INVOICE_ID, AMOUNT)
                 VALUES (?, ?, ?, ?)",
                (
                    &allocation_id,
                    &payment_id,
                    &allocation.invoice_id,
                    &allocation.amount,
                ),
            )
            .map_err(AppError::from)?;
        }

        for invoice_id in updates {
            recompute_invoice_status(conn, invoice_id)?;
        }

        get(conn, payment_id)?
            .ok_or_else(|| AppError::Internal("Pago creado pero no recuperado".into()))
    })
}

/// Anula una recepción: revierte el efectivo en su caja de origen (que debe
/// seguir abierta) y libera las asignaciones para que las facturas recuperen
/// su saldo real. Las líneas y asignaciones se conservan como rastro; es el
/// estado del pago el que lo excluye de los cálculos.
///
/// No se permite anular contra una caja cerrada: un cierre es inmutable y su
/// corrección posterior es un ajuste auditado, no una anulación encubierta.
pub fn void(
    conn: &mut SimpleConnection,
    payment_id: i32,
    reason: &str,
    user_id: Option<i32>,
) -> Result<Payment, AppError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::Validation(
            "El motivo de la anulación es obligatorio".into(),
        ));
    }
    if reason.chars().count() > 500 {
        return Err(AppError::Validation(
            "El motivo de la anulación no puede superar 500 caracteres".into(),
        ));
    }

    db::tx::in_transaction(conn, |conn| {
        let row: Option<(String, String, Option<i32>)> = conn
            .query_first(
                "SELECT CODE, STATUS, CASH_SESSION_ID
                   FROM PAYMENTS WHERE ID = ? WITH LOCK",
                (&payment_id,),
            )
            .map_err(AppError::from)?;
        let Some((code, status, session_id)) = row else {
            return Err(AppError::NotFound(format!(
                "Pago {payment_id} no encontrado"
            )));
        };
        if status != "APLICADO" {
            return Err(AppError::Validation(format!(
                "El pago {code} ya está anulado"
            )));
        }

        // Bloquear también la caja: un cierre concurrente debe esperar y no
        // puede congelar un arqueo con una devolución a medio registrar.
        if let Some(session_id) = session_id {
            let session_status: Option<(String,)> = conn
                .query_first(
                    "SELECT STATUS FROM CASH_SESSIONS WHERE ID = ? WITH LOCK",
                    (&session_id,),
                )
                .map_err(AppError::from)?;
            match session_status {
                Some((session,)) if session == "ABIERTA" => {}
                Some(_) => {
                    return Err(AppError::Validation(
                        "La caja de este pago ya está cerrada: no se puede anular la \
                         recepción. Registra un ajuste en la caja abierta."
                            .into(),
                    ));
                }
                None => {
                    return Err(AppError::Internal(format!(
                        "La caja {session_id} del pago no existe"
                    )))
                }
            }

            // Solo el efectivo salió de la caja: la devolución revierte esa
            // parte, no el total del pago (una transferencia no toca el arqueo).
            let cash: Option<(f64,)> = conn
                .query_first(
                    "SELECT CAST(COALESCE(SUM(l.AMOUNT), 0) AS DOUBLE PRECISION)
                     FROM PAYMENT_LINES l
                     JOIN PAYMENT_METHODS m ON m.CODE = l.METHOD_CODE
                     WHERE l.PAYMENT_ID = ? AND m.KIND = 'EFECTIVO'",
                    (&payment_id,),
                )
                .map_err(AppError::from)?;
            let cash = cash.map(|(v,)| v).unwrap_or(0.0);
            // Todo el efectivo registrado se devuelve, aunque sea un céntimo:
            // con `> MONEY_EPSILON` una recepción de 0,01 quedaba sin reverso y
            // la caja se quedaba un céntimo por encima de lo real.
            if cash > 0.0 {
                let movement_id = next_id(conn, "GEN_CASH_MOVEMENTS_ID")?;
                let detail = format!("Anulación {code}: {reason}");
                conn.execute(
                    "INSERT INTO CASH_MOVEMENTS
                        (ID, CASH_SESSION_ID, KIND, AMOUNT, PAYMENT_ID, REASON, CREATED_BY)
                     VALUES (?, ?, 'DEVOLUCION', ?, ?, ?, ?)",
                    (
                        &movement_id,
                        &session_id,
                        &(-cash),
                        &payment_id,
                        &detail,
                        &user_id,
                    ),
                )
                .map_err(AppError::from)?;
            }
        }

        conn.execute(
            "UPDATE PAYMENTS
                SET STATUS = 'ANULADO', VOIDED_AT = CURRENT_TIMESTAMP,
                    VOIDED_BY = ?, VOID_REASON = ?
              WHERE ID = ? AND STATUS = 'APLICADO'",
            (&user_id, &reason, &payment_id),
        )
        .map_err(AppError::from)?;

        let invoices: Vec<(i32,)> = conn
            .query(
                "SELECT INVOICE_ID FROM PAYMENT_ALLOCATIONS
                  WHERE PAYMENT_ID = ? ORDER BY INVOICE_ID",
                (&payment_id,),
            )
            .map_err(AppError::from)?;
        for (invoice_id,) in invoices {
            recompute_invoice_status(conn, invoice_id)?;
        }

        get(conn, payment_id)?
            .ok_or_else(|| AppError::Internal("Pago anulado pero no recuperado".into()))
    })
}

/// Valida una fecha `AAAA-MM-DD` (el rango se compara como DATE en Firebird).
fn normalize_date(label: &str, value: &str) -> Result<String, AppError> {
    let value = value.trim();
    let bytes = value.as_bytes();
    let shape_ok = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    if !shape_ok {
        return Err(AppError::Validation(format!(
            "Fecha {label} inválida: usa AAAA-MM-DD"
        )));
    }
    Ok(value.to_string())
}

/// Reporte del período: facturado (por emisión) vs. cobrado (por recepción),
/// con el saldo pendiente de lo facturado y el desglose por método.
///
/// Leer `INVOICES.STATUS = 'PAGADA'` como ingreso mezcla las dos fechas; este
/// reporte las separa justamente para eso.
pub fn billing_report(
    conn: &mut SimpleConnection,
    from: &str,
    to: &str,
) -> Result<BillingReport, AppError> {
    let from = normalize_date("inicial", from)?;
    let to = normalize_date("final", to)?;
    if from > to {
        return Err(AppError::Validation(
            "La fecha inicial no puede ser posterior a la final".into(),
        ));
    }

    let range = "CAST(CAST(? AS VARCHAR(10)) AS DATE) AND CAST(CAST(? AS VARCHAR(10)) AS DATE)";

    let billed: Option<(f64, f64, i32)> = conn
        .query_first(
            &format!(
                "SELECT CAST(COALESCE(SUM(TOTAL), 0) AS DOUBLE PRECISION),
                        CAST(COALESCE(SUM(TAX_AMOUNT), 0) AS DOUBLE PRECISION),
                        COUNT(*)
                 FROM INVOICES
                 WHERE STATUS <> 'ANULADA'
                   AND CAST(ISSUE_DATE AS DATE) BETWEEN {range}"
            ),
            (&from, &to),
        )
        .map_err(AppError::from)?;
    let (billed_total, billed_tax, billed_count) = billed.unwrap_or((0.0, 0.0, 0));

    let collected: Option<(f64, i32)> = conn
        .query_first(
            &format!(
                "SELECT CAST(COALESCE(SUM(TOTAL), 0) AS DOUBLE PRECISION), COUNT(*)
                 FROM PAYMENTS
                 WHERE STATUS = 'APLICADO'
                   AND CAST(RECEIVED_AT AS DATE) BETWEEN {range}"
            ),
            (&from, &to),
        )
        .map_err(AppError::from)?;
    let (collected_total, collected_count) = collected.unwrap_or((0.0, 0));

    // Saldo de lo facturado en el período: se calcula con los abonos vigentes,
    // así una anulación devuelve la factura a la cartera del mes en que se emitió.
    let outstanding: Option<(f64,)> = conn
        .query_first(
            &format!(
                "SELECT CAST(COALESCE(SUM(CASE WHEN b.BALANCE > 0.01 THEN b.BALANCE ELSE 0 END), 0)
                             AS DOUBLE PRECISION)
                 FROM (
                     SELECT CAST(i.TOTAL AS DOUBLE PRECISION) - COALESCE((
                         SELECT CAST(SUM(a.AMOUNT) AS DOUBLE PRECISION)
                         FROM PAYMENT_ALLOCATIONS a
                         JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                         WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
                     ), 0) AS BALANCE
                     FROM INVOICES i
                     WHERE i.STATUS <> 'ANULADA'
                       AND CAST(i.ISSUE_DATE AS DATE) BETWEEN {range}
                 ) b"
            ),
            (&from, &to),
        )
        .map_err(AppError::from)?;
    let outstanding_total = outstanding.map(|(v,)| v).unwrap_or(0.0);

    let methods: Vec<BillingMethodTotal> = conn
        .query(
            &format!(
                "SELECT l.METHOD_CODE, m.NAME, CAST(SUM(l.AMOUNT) AS DOUBLE PRECISION)
                 FROM PAYMENT_LINES l
                 JOIN PAYMENTS p ON p.ID = l.PAYMENT_ID
                 JOIN PAYMENT_METHODS m ON m.CODE = l.METHOD_CODE
                 WHERE p.STATUS = 'APLICADO'
                   AND CAST(p.RECEIVED_AT AS DATE) BETWEEN {range}
                 GROUP BY l.METHOD_CODE, m.NAME, m.SORT_ORDER
                 ORDER BY m.SORT_ORDER, l.METHOD_CODE"
            ),
            (&from, &to),
        )
        .map_err(AppError::from)?
        .into_iter()
        .map(
            |(method_code, method_name, amount): (String, String, f64)| BillingMethodTotal {
                method_code,
                method_name,
                amount,
            },
        )
        .collect();

    Ok(BillingReport {
        from_date: from,
        to_date: to,
        billed_total,
        billed_tax,
        billed_count,
        collected_total,
        collected_count,
        outstanding_total,
        methods,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::{run_migrations_with, MigrationContext};
    use crate::models::cash::{CashCountInput, CloseCashInput};
    use crate::models::payment::{CreatePaymentAllocationInput, CreatePaymentLineInput};
    use crate::test_helpers::{cleanup_test_db, setup_empty_test_db, setup_test_db};
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        let (mut conn, db_path) = setup_test_db();
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (1, 'CC', '111', 'Tutor Uno')",
            (),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (2, 'CC', '222', 'Tutor Dos')",
            (),
        )
        .unwrap();
        cash::open_session(&mut conn, 100000.0, None, 1).unwrap();
        (conn, db_path)
    }

    fn insert_invoice(conn: &mut SimpleConnection, id: i32, owner_id: i32, total: f64) {
        conn.execute(
            "INSERT INTO INVOICES
                (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT,
                 TOTAL, STATUS)
             VALUES (?, ?, ?, ?, 0, 0, ?, 'EMITIDA')",
            (&id, &format!("FAC-T{id:03}"), &owner_id, &total, &total),
        )
        .unwrap();
    }

    fn line(method: &str, amount: f64) -> CreatePaymentLineInput {
        CreatePaymentLineInput {
            method_code: method.into(),
            amount,
            reference: None,
        }
    }

    fn allocation(invoice_id: i32, amount: f64) -> CreatePaymentAllocationInput {
        CreatePaymentAllocationInput { invoice_id, amount }
    }

    fn input(
        owner_id: i32,
        lines: Vec<CreatePaymentLineInput>,
        allocations: Vec<CreatePaymentAllocationInput>,
    ) -> RegisterPaymentInput {
        RegisterPaymentInput {
            owner_id,
            patient_id: None,
            notes: None,
            lines,
            allocations,
        }
    }

    #[test]
    fn partial_payment_leaves_invoice_abonada() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 40000.0)],
                vec![allocation(1, 40000.0)],
            ),
            Some(1),
        )
        .unwrap();

        assert_eq!(payment.total, 40000.0);
        assert_eq!(payment.lines.len(), 1);
        assert_eq!(payment.allocations.len(), 1);
        assert!(payment.code.starts_with("REC-"));

        let balance = invoice_balance(&mut conn, 1).unwrap();
        assert_eq!(balance.payment_status, "ABONADA");
        assert!((balance.paid_amount - 40000.0).abs() < 0.01);
        assert!((balance.balance - 60000.0).abs() < 0.01);
        assert_eq!(balance.status, "EMITIDA");

        cleanup_test_db(&db_path);
    }

    #[test]
    fn second_payment_completes_invoice_and_reports_pagada() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 40000.0)],
                vec![allocation(1, 40000.0)],
            ),
            Some(1),
        )
        .unwrap();
        register(
            &mut conn,
            &input(
                1,
                vec![line("TRANSFERENCIA", 60000.0)],
                vec![allocation(1, 60000.0)],
            ),
            Some(1),
        )
        .unwrap();

        let balance = invoice_balance(&mut conn, 1).unwrap();
        assert_eq!(balance.payment_status, "PAGADA");
        assert_eq!(balance.status, "PAGADA");
        assert!((balance.balance - 0.0).abs() < 0.01);

        // Solo el pago en efectivo dejó movimiento de caja.
        let movements: Option<(i32,)> = conn
            .query_first(
                "SELECT COUNT(*) FROM CASH_MOVEMENTS WHERE KIND = 'PAGO_EFECTIVO'",
                (),
            )
            .unwrap();
        assert_eq!(movements.map(|(n,)| n).unwrap_or(0), 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn mixed_payment_creates_two_lines_and_one_receipt() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 60000.0), line("TRANSFERENCIA", 40000.0)],
                vec![allocation(1, 100000.0)],
            ),
            Some(1),
        )
        .unwrap();

        assert_eq!(payment.lines.len(), 2);
        assert_eq!(payment.lines[0].method_kind, "EFECTIVO");
        assert_eq!(payment.lines[1].method_kind, "ELECTRONICO");
        assert!((payment.total - 100000.0).abs() < 0.01);
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PAGADA"
        );

        cleanup_test_db(&db_path);
    }

    #[test]
    fn payment_can_cover_two_invoices_of_the_same_owner() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 50000.0);
        insert_invoice(&mut conn, 2, 1, 50000.0);

        register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 100000.0)],
                vec![allocation(1, 50000.0), allocation(2, 50000.0)],
            ),
            Some(1),
        )
        .unwrap();

        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PAGADA"
        );
        assert_eq!(
            invoice_balance(&mut conn, 2).unwrap().payment_status,
            "PAGADA"
        );

        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_allocation_above_invoice_balance() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 120000.0)],
                vec![allocation(1, 120000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_when_lines_do_not_match_allocations() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 90000.0)],
                vec![allocation(1, 100000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_invoice_from_another_owner() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 2, 50000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 50000.0)],
                vec![allocation(1, 50000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn requires_an_open_cash_session() {
        let (mut conn, db_path) = setup_test_db();
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (1, 'CC', '111', 'Tutor Uno')",
            (),
        )
        .unwrap();
        insert_invoice(&mut conn, 1, 1, 50000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 50000.0)],
                vec![allocation(1, 50000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn cannot_open_a_second_cash_session() {
        let (mut conn, db_path) = setup();
        let result = cash::open_session(&mut conn, 0.0, None, 1);
        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_invoice_appearing_twice_in_the_same_payment() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 100000.0)],
                vec![allocation(1, 50000.0), allocation(1, 50000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_patient_from_another_owner() {
        let (mut conn, db_path) = setup();
        conn.execute(
            "INSERT INTO PATIENTS (ID, OWNER_ID, SPECIES_ID, NAME, SEX)
             VALUES (9001, 2, (SELECT FIRST 1 ID FROM SPECIES), 'Ajeno', 'M')",
            (),
        )
        .unwrap();
        insert_invoice(&mut conn, 1, 1, 50000.0);

        // El tutor 1 paga su factura indicando el paciente del tutor 2.
        let mut payment_input = input(
            1,
            vec![line("EFECTIVO", 50000.0)],
            vec![allocation(1, 50000.0)],
        );
        payment_input.patient_id = Some(9001);

        let result = register(&mut conn, &payment_input, Some(1));
        assert!(matches!(result, Err(AppError::Validation(_))));

        // Nada quedó aplicado: la factura sigue pendiente.
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PENDIENTE"
        );

        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_reference_longer_than_the_column() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 50000.0);

        let result = register(
            &mut conn,
            &input(
                1,
                vec![CreatePaymentLineInput {
                    method_code: "TRANSFERENCIA".into(),
                    amount: 50000.0,
                    reference: Some("X".repeat(81)),
                }],
                vec![allocation(1, 50000.0)],
            ),
            Some(1),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn legacy_paid_invoices_are_backfilled_as_payments() {
        let (mut conn, db_path) = setup_empty_test_db();
        run_migrations_with(
            &mut conn,
            MigrationContext {
                allow_demo_data: true,
            },
        )
        .unwrap();

        let row: Option<(String, f64)> = conn
            .query_first(
                "SELECT l.METHOD_CODE, CAST(l.AMOUNT AS DOUBLE PRECISION)
                 FROM PAYMENTS p
                 JOIN PAYMENT_LINES l ON l.PAYMENT_ID = p.ID
                 WHERE p.CODE = 'LEG-F-2026-0001'",
                (),
            )
            .unwrap();
        let (method, amount) = row.expect("factura PAGADA migrada");
        assert_eq!(method, "TRANSFERENCIA");
        assert!((amount - 178500.0).abs() < 0.01);

        let allocation: Option<(f64,)> = conn
            .query_first(
                "SELECT CAST(a.AMOUNT AS DOUBLE PRECISION)
                 FROM PAYMENT_ALLOCATIONS a
                 JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
                 WHERE p.CODE = 'LEG-F-2026-0001'",
                (),
            )
            .unwrap();
        assert!((allocation.map(|(v,)| v).unwrap_or(0.0) - 178500.0).abs() < 0.01);

        cleanup_test_db(&db_path);
    }

    fn count_cash_movements(conn: &mut SimpleConnection, payment_id: i32) -> (i32, f64) {
        let row: Option<(i32, f64)> = conn
            .query_first(
                "SELECT COUNT(*), CAST(COALESCE(SUM(AMOUNT), 0) AS DOUBLE PRECISION)
                 FROM CASH_MOVEMENTS
                 WHERE KIND = 'DEVOLUCION' AND PAYMENT_ID = ?",
                (&payment_id,),
            )
            .unwrap();
        row.unwrap_or((0, 0.0))
    }

    #[test]
    fn void_payment_reverses_cash_and_restores_invoice_balance() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 40000.0)],
                vec![allocation(1, 40000.0)],
            ),
            Some(1),
        )
        .unwrap();
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "ABONADA"
        );

        let voided = void(&mut conn, payment.id, "Importe digitado por error", Some(1)).unwrap();
        assert_eq!(voided.status, "ANULADO");
        // Las líneas y asignaciones se conservan como rastro.
        assert_eq!(voided.lines.len(), 1);
        assert_eq!(voided.allocations.len(), 1);

        let balance = invoice_balance(&mut conn, 1).unwrap();
        assert_eq!(balance.payment_status, "PENDIENTE");
        assert!((balance.paid_amount - 0.0).abs() < 0.01);
        assert!((balance.balance - 100000.0).abs() < 0.01);
        assert_eq!(balance.status, "EMITIDA");

        // El efectivo esperado vuelve a la base: base 100.000 + 40.000 - 40.000.
        let session = cash::get_open_session(&mut conn).unwrap().unwrap();
        let summary = cash::summary(&mut conn, session.id).unwrap();
        assert!((summary.expected_cash - 100000.0).abs() < 0.01);
        assert!((summary.total_collected - 0.0).abs() < 0.01);

        let (count, amount) = count_cash_movements(&mut conn, payment.id);
        assert_eq!(count, 1);
        assert!((amount + 40000.0).abs() < 0.01);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn void_reverses_only_the_cash_part_of_a_mixed_payment() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);

        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 60000.0), line("TRANSFERENCIA", 40000.0)],
                vec![allocation(1, 100000.0)],
            ),
            Some(1),
        )
        .unwrap();

        void(&mut conn, payment.id, "Pago duplicado", Some(1)).unwrap();

        let (count, amount) = count_cash_movements(&mut conn, payment.id);
        assert_eq!(count, 1);
        assert!((amount + 60000.0).abs() < 0.01);
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PENDIENTE"
        );

        cleanup_test_db(&db_path);
    }

    #[test]
    fn void_requires_a_reason() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 50000.0);
        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 50000.0)],
                vec![allocation(1, 50000.0)],
            ),
            Some(1),
        )
        .unwrap();

        let result = void(&mut conn, payment.id, "   ", Some(1));
        assert!(matches!(result, Err(AppError::Validation(_))));
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PAGADA"
        );

        cleanup_test_db(&db_path);
    }

    #[test]
    fn void_rejects_a_payment_whose_cash_session_is_closed() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);
        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 100000.0)],
                vec![allocation(1, 100000.0)],
            ),
            Some(1),
        )
        .unwrap();

        cash::close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![CashCountInput {
                    denomination: 200000.0,
                    quantity: 1,
                }],
                notes: None,
            },
            Some(1),
        )
        .unwrap();

        let result = void(&mut conn, payment.id, "Cobro equivocado", Some(1));
        assert!(matches!(result, Err(AppError::Validation(_))));
        // Nada cambió: el pago sigue vigente y la factura pagada.
        assert_eq!(
            invoice_balance(&mut conn, 1).unwrap().payment_status,
            "PAGADA"
        );
        assert_eq!(count_cash_movements(&mut conn, payment.id).0, 0);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn void_cannot_be_applied_twice() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 50000.0);
        let payment = register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 50000.0)],
                vec![allocation(1, 50000.0)],
            ),
            Some(1),
        )
        .unwrap();

        void(&mut conn, payment.id, "Primera anulación", Some(1)).unwrap();
        let second = void(&mut conn, payment.id, "Segunda anulación", Some(1));
        assert!(matches!(second, Err(AppError::Validation(_))));
        // Solo se revirtió una vez.
        assert_eq!(count_cash_movements(&mut conn, payment.id).0, 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn void_returns_not_found_for_unknown_payment() {
        let (mut conn, db_path) = setup();
        let result = void(&mut conn, 9999, "No existe", Some(1));
        assert!(matches!(result, Err(AppError::NotFound(_))));
        cleanup_test_db(&db_path);
    }

    #[test]
    fn billing_report_separates_billed_from_collected() {
        let (mut conn, db_path) = setup();
        insert_invoice(&mut conn, 1, 1, 100000.0);
        // Factura de fuera del período: no debe sumar en ninguna cifra.
        conn.execute(
            "INSERT INTO INVOICES
                (ID, INVOICE_NUMBER, OWNER_ID, ISSUE_DATE, SUBTOTAL, TAX_RATE,
                 TAX_AMOUNT, TOTAL, STATUS)
             VALUES (2, 'FAC-ANT', 1, '1999-12-31 10:00:00', 500000, 0, 0, 500000, 'EMITIDA')",
            (),
        )
        .unwrap();

        register(
            &mut conn,
            &input(
                1,
                vec![line("EFECTIVO", 40000.0)],
                vec![allocation(1, 40000.0)],
            ),
            Some(1),
        )
        .unwrap();

        let report = billing_report(&mut conn, "2000-01-01", "2099-12-31").unwrap();
        assert_eq!(report.billed_count, 1);
        assert!((report.billed_total - 100000.0).abs() < 0.01);
        assert!((report.billed_tax - 0.0).abs() < 0.01);
        assert_eq!(report.collected_count, 1);
        assert!((report.collected_total - 40000.0).abs() < 0.01);
        assert!((report.outstanding_total - 60000.0).abs() < 0.01);
        assert_eq!(report.methods.len(), 1);
        assert_eq!(report.methods[0].method_code, "EFECTIVO");
        assert!((report.methods[0].amount - 40000.0).abs() < 0.01);

        // Anular el pago lo saca de "cobrado" y devuelve el saldo a la cartera.
        let payments = list_by_invoice(&mut conn, 1).unwrap();
        void(&mut conn, payments[0].id, "Cobro duplicado", Some(1)).unwrap();
        let after = billing_report(&mut conn, "2000-01-01", "2099-12-31").unwrap();
        assert!((after.collected_total - 0.0).abs() < 0.01);
        assert_eq!(after.collected_count, 0);
        assert!((after.outstanding_total - 100000.0).abs() < 0.01);
        assert!(after.methods.is_empty());
        // Lo facturado no cambia: la factura sigue vigente.
        assert!((after.billed_total - 100000.0).abs() < 0.01);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn billing_report_rejects_an_inverted_or_invalid_range() {
        let (mut conn, db_path) = setup();

        let inverted = billing_report(&mut conn, "2026-05-10", "2026-05-01");
        assert!(matches!(inverted, Err(AppError::Validation(_))));

        let invalid = billing_report(&mut conn, "2026-5-1", "2026-05-31");
        assert!(matches!(invalid, Err(AppError::Validation(_))));

        cleanup_test_db(&db_path);
    }
}
