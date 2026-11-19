use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::db;
use crate::error::AppError;
use crate::models::cash::{
    CashCountLine, CashMethodTotal, CashMovement, CashSession, CashSessionSummary, CloseCashInput,
};
use crate::repositories::next_id;

/// Tolerancia de comparación monetaria (D_AMOUNT tiene 2 decimales).
const MONEY_EPSILON: f64 = 0.01;

type CashSessionRow = (
    i32,            // id
    String,         // code
    String,         // opened_at
    i32,            // opened_by
    Option<String>, // opened_by_name
    f64,            // opening_float
    String,         // status
    Option<String>, // closed_at
    Option<String>, // closed_by_name
    Option<f64>,    // expected_cash
    Option<f64>,    // counted_cash
    Option<f64>,    // difference
    Option<f64>,    // total_billed
    Option<f64>,    // total_collected
    Option<String>, // notes
);

const CASH_SESSION_SELECT: &str = "
    SELECT s.ID, s.CODE, LEFT(CAST(s.OPENED_AT AS VARCHAR(60)), 19),
           s.OPENED_BY, u.FULL_NAME,
           CAST(s.OPENING_FLOAT AS DOUBLE PRECISION), s.STATUS,
           LEFT(CAST(s.CLOSED_AT AS VARCHAR(60)), 19), cu.FULL_NAME,
           CAST(s.EXPECTED_CASH AS DOUBLE PRECISION),
           CAST(s.COUNTED_CASH AS DOUBLE PRECISION),
           CAST(s.DIFFERENCE AS DOUBLE PRECISION),
           CAST(s.TOTAL_BILLED AS DOUBLE PRECISION),
           CAST(s.TOTAL_COLLECTED AS DOUBLE PRECISION),
           s.NOTES
    FROM CASH_SESSIONS s
    LEFT JOIN USERS u ON u.ID = s.OPENED_BY
    LEFT JOIN USERS cu ON cu.ID = s.CLOSED_BY";

type MovementRow = (
    i32,            // id
    i32,            // session id
    String,         // kind
    f64,            // amount
    Option<i32>,    // payment id
    Option<String>, // reason
    Option<String>, // created_by_name
    String,         // created_at
);

fn map_movement(r: MovementRow) -> CashMovement {
    CashMovement {
        id: r.0,
        cash_session_id: r.1,
        kind: r.2,
        amount: r.3,
        payment_id: r.4,
        reason: r.5,
        created_by_name: r.6,
        created_at: r.7,
    }
}

fn map_session(r: CashSessionRow) -> CashSession {
    CashSession {
        id: r.0,
        code: r.1,
        opened_at: r.2,
        opened_by: r.3,
        opened_by_name: r.4,
        opening_float: r.5,
        status: r.6,
        closed_at: r.7,
        closed_by_name: r.8,
        expected_cash: r.9,
        counted_cash: r.10,
        difference: r.11,
        total_billed: r.12,
        total_collected: r.13,
        notes: r.14,
    }
}

/// Sesión por ID (abierta o cerrada).
pub fn get(conn: &mut SimpleConnection, id: i32) -> Result<Option<CashSession>, AppError> {
    let row: Option<CashSessionRow> = conn
        .query_first(&format!("{CASH_SESSION_SELECT} WHERE s.ID = ?"), (&id,))
        .map_err(AppError::from)?;
    Ok(row.map(map_session))
}

/// Sesión ABIERTA actual, si existe.
pub fn get_open_session(conn: &mut SimpleConnection) -> Result<Option<CashSession>, AppError> {
    let row: Option<CashSessionRow> = conn
        .query_first(
            &format!("{CASH_SESSION_SELECT} WHERE s.STATUS = 'ABIERTA'"),
            (),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_session))
}

/// Exige una caja abierta y bloquea su fila hasta el final de la transacción.
///
/// El bloqueo hace que un cierre concurrente espere y no pueda congelar un
/// arqueo sin los pagos que están entrando en ese momento. `WITH LOCK` no se
/// admite en consultas con JOIN, por eso se bloquea el ID y luego se lee la
/// sesión completa.
pub fn require_open_session(conn: &mut SimpleConnection) -> Result<CashSession, AppError> {
    let id: Option<(i32,)> = conn
        .query_first(
            "SELECT ID FROM CASH_SESSIONS WHERE STATUS = 'ABIERTA' WITH LOCK",
            (),
        )
        .map_err(AppError::from)?;
    let Some((id,)) = id else {
        return Err(AppError::Validation(
            "No hay una caja abierta. Abre la caja antes de cobrar.".into(),
        ));
    };
    get(conn, id)?.ok_or_else(|| AppError::Internal("Caja abierta no recuperada".into()))
}

/// Abre una sesión de caja con su base inicial.
///
/// La unicidad de la caja abierta la garantiza UX_CASH_SESSIONS_OPEN; la
/// comprobación previa solo produce un mensaje claro en el caso normal.
pub fn open_session(
    conn: &mut SimpleConnection,
    opening_float: f64,
    notes: Option<String>,
    user_id: i32,
) -> Result<CashSession, AppError> {
    if opening_float < 0.0 {
        return Err(AppError::Validation(
            "La base inicial de caja no puede ser negativa".into(),
        ));
    }

    db::tx::in_transaction(conn, |conn| {
        if get_open_session(conn)?.is_some() {
            return Err(AppError::Validation(
                "Ya hay una caja abierta. Ciérrala antes de abrir otra.".into(),
            ));
        }

        let id = next_id(conn, "GEN_CASH_SESSIONS_ID")?;
        conn.execute(
            "INSERT INTO CASH_SESSIONS (ID, OPENED_BY, OPENING_FLOAT, NOTES)
             VALUES (?, ?, ?, ?)",
            (&id, &user_id, &opening_float, &notes),
        )
        .map_err(|e| {
            let text = e.to_string().to_uppercase();
            if text.contains("UNIQUE") || text.contains("UX_CASH_SESSIONS_OPEN") {
                AppError::Validation(
                    "Ya hay una caja abierta. Ciérrala antes de abrir otra.".into(),
                )
            } else {
                AppError::from(e)
            }
        })?;

        get(conn, id)?.ok_or_else(|| AppError::Internal("Caja creada pero no recuperada".into()))
    })
}

/// Movimientos de una sesión, del más reciente al más antiguo.
pub fn list_movements(
    conn: &mut SimpleConnection,
    session_id: i32,
) -> Result<Vec<CashMovement>, AppError> {
    let rows: Vec<MovementRow> = conn
        .query(
            "SELECT m.ID, m.CASH_SESSION_ID, m.KIND,
                    CAST(m.AMOUNT AS DOUBLE PRECISION), m.PAYMENT_ID, m.REASON,
                    u.FULL_NAME, LEFT(CAST(m.CREATED_AT AS VARCHAR(60)), 19)
             FROM CASH_MOVEMENTS m
             LEFT JOIN USERS u ON u.ID = m.CREATED_BY
             WHERE m.CASH_SESSION_ID = ?
             ORDER BY m.ID DESC",
            (&session_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_movement).collect())
}

/// Líneas de arqueo guardadas en una sesión cerrada.
pub fn list_count_lines(
    conn: &mut SimpleConnection,
    session_id: i32,
) -> Result<Vec<CashCountLine>, AppError> {
    let rows: Vec<(f64, i32, f64)> = conn
        .query(
            "SELECT CAST(DENOMINATION AS DOUBLE PRECISION), QUANTITY,
                    CAST(SUBTOTAL AS DOUBLE PRECISION)
             FROM CASH_COUNT_LINES
             WHERE CASH_SESSION_ID = ?
             ORDER BY DENOMINATION DESC",
            (&session_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .filter(|(_, quantity, _)| *quantity > 0)
        .map(|(denomination, quantity, subtotal)| CashCountLine {
            denomination,
            quantity,
            subtotal,
        })
        .collect())
}

/// Cifras derivadas de una sesión: efectivo esperado, cobros y métodos.
struct SessionFigures {
    expected_cash: f64,
    cash_in: f64,
    cash_out: f64,
    total_billed: f64,
    total_collected: f64,
    payments_count: i32,
    methods: Vec<CashMethodTotal>,
}

fn figures(
    conn: &mut SimpleConnection,
    session: &CashSession,
) -> Result<SessionFigures, AppError> {
    let totals: Option<(f64, f64, f64)> = conn
        .query_first(
            "SELECT CAST(COALESCE(SUM(CASE WHEN AMOUNT > 0 THEN AMOUNT ELSE 0 END), 0)
                        AS DOUBLE PRECISION),
                    CAST(COALESCE(SUM(CASE WHEN AMOUNT < 0 THEN -AMOUNT ELSE 0 END), 0)
                        AS DOUBLE PRECISION),
                    CAST(COALESCE(SUM(AMOUNT), 0) AS DOUBLE PRECISION)
             FROM CASH_MOVEMENTS WHERE CASH_SESSION_ID = ?",
            (&session.id,),
        )
        .map_err(AppError::from)?;
    let (cash_in, cash_out, movements_total) = totals.unwrap_or((0.0, 0.0, 0.0));

    let collected: Option<(f64, i32)> = conn
        .query_first(
            "SELECT CAST(COALESCE(SUM(TOTAL), 0) AS DOUBLE PRECISION), COUNT(*)
             FROM PAYMENTS
             WHERE CASH_SESSION_ID = ? AND STATUS = 'APLICADO'",
            (&session.id,),
        )
        .map_err(AppError::from)?;
    let (total_collected, payments_count) = collected.unwrap_or((0.0, 0));

    // Facturado durante la sesión: facturas emitidas entre la apertura y el
    // cierre (o ahora, si sigue abierta).
    let billed: Option<(f64,)> = conn
        .query_first(
            "SELECT CAST(COALESCE(SUM(i.TOTAL), 0) AS DOUBLE PRECISION)
             FROM INVOICES i
             JOIN CASH_SESSIONS s ON s.ID = ?
             WHERE i.STATUS <> 'ANULADA'
               AND i.ISSUE_DATE >= s.OPENED_AT
               AND i.ISSUE_DATE <= COALESCE(s.CLOSED_AT, CURRENT_TIMESTAMP)",
            (&session.id,),
        )
        .map_err(AppError::from)?;
    let total_billed = billed.map(|(v,)| v).unwrap_or(0.0);

    let method_rows: Vec<(String, String, f64)> = conn
        .query(
            "SELECT l.METHOD_CODE, m.NAME, CAST(SUM(l.AMOUNT) AS DOUBLE PRECISION)
             FROM PAYMENT_LINES l
             JOIN PAYMENTS p ON p.ID = l.PAYMENT_ID
             JOIN PAYMENT_METHODS m ON m.CODE = l.METHOD_CODE
             WHERE p.CASH_SESSION_ID = ? AND p.STATUS = 'APLICADO'
             GROUP BY l.METHOD_CODE, m.NAME, m.SORT_ORDER
             ORDER BY m.SORT_ORDER, l.METHOD_CODE",
            (&session.id,),
        )
        .map_err(AppError::from)?;

    Ok(SessionFigures {
        expected_cash: session.opening_float + movements_total,
        cash_in,
        cash_out,
        total_billed,
        total_collected,
        payments_count,
        methods: method_rows
            .into_iter()
            .map(|(method_code, method_name, amount)| CashMethodTotal {
                method_code,
                method_name,
                amount,
            })
            .collect(),
    })
}

/// Totales por método congelados al cerrar la sesión.
fn frozen_totals(
    conn: &mut SimpleConnection,
    session_id: i32,
) -> Result<Vec<CashMethodTotal>, AppError> {
    let rows: Vec<(String, String, f64)> = conn
        .query(
            "SELECT t.METHOD_CODE, COALESCE(m.NAME, t.METHOD_CODE),
                    CAST(t.AMOUNT AS DOUBLE PRECISION)
             FROM CASH_SESSION_TOTALS t
             LEFT JOIN PAYMENT_METHODS m ON m.CODE = t.METHOD_CODE
             WHERE t.CASH_SESSION_ID = ?
             ORDER BY m.SORT_ORDER, t.METHOD_CODE",
            (&session_id,),
        )
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .map(|(method_code, method_name, amount)| CashMethodTotal {
            method_code,
            method_name,
            amount,
        })
        .collect())
}

/// Resumen de una sesión (cifras en vivo o el snapshot si ya cerró).
pub fn summary(
    conn: &mut SimpleConnection,
    session_id: i32,
) -> Result<CashSessionSummary, AppError> {
    let session = get(conn, session_id)?
        .ok_or_else(|| AppError::NotFound(format!("Caja {session_id} no encontrada")))?;
    let figures = figures(conn, &session)?;
    let count_lines = list_count_lines(conn, session_id)?;
    // Una sesión cerrada muestra los totales congelados al cerrar, no un
    // recálculo que cambiaría si después se anulara un pago.
    let methods = if session.status == "CERRADA" {
        let frozen = frozen_totals(conn, session_id)?;
        if frozen.is_empty() {
            figures.methods
        } else {
            frozen
        }
    } else {
        figures.methods
    };

    let counted_from_lines: f64 = count_lines.iter().map(|l| l.subtotal).sum();
    let expected_cash = session.expected_cash.unwrap_or(figures.expected_cash);
    let counted_cash = session.counted_cash.or_else(|| {
        if count_lines.is_empty() {
            None
        } else {
            Some(counted_from_lines)
        }
    });
    let difference = session
        .difference
        .or_else(|| counted_cash.map(|c| c - expected_cash));
    let total_billed = session.total_billed.unwrap_or(figures.total_billed);
    let total_collected = session.total_collected.unwrap_or(figures.total_collected);

    Ok(CashSessionSummary {
        opening_float: session.opening_float,
        expected_cash,
        counted_cash,
        difference,
        total_billed,
        total_collected,
        cash_in: figures.cash_in,
        cash_out: figures.cash_out,
        payments_count: figures.payments_count,
        methods,
        movements: list_movements(conn, session_id)?,
        count_lines,
        session,
    })
}

/// Registra un movimiento manual (gasto, retiro o ajuste) en la caja abierta.
///
/// El importe llega siempre positivo desde la UI; gastos y retiros se guardan
/// negativos para que el efectivo esperado sea la suma firmada.
pub fn add_movement(
    conn: &mut SimpleConnection,
    kind: &str,
    amount: f64,
    reason: &str,
    user_id: Option<i32>,
) -> Result<CashMovement, AppError> {
    let kind = kind.trim().to_uppercase();
    if !matches!(kind.as_str(), "GASTO" | "RETIRO" | "AJUSTE") {
        return Err(AppError::Validation(
            "Tipo de movimiento inválido: usa GASTO, RETIRO o AJUSTE".into(),
        ));
    }
    if !amount.is_finite() || amount.abs() <= MONEY_EPSILON {
        return Err(AppError::Validation(
            "El importe del movimiento debe ser distinto de cero".into(),
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(AppError::Validation(
            "El motivo del movimiento es obligatorio".into(),
        ));
    }
    let signed = if kind == "AJUSTE" {
        amount
    } else {
        -amount.abs()
    };

    db::tx::in_transaction(conn, |conn| {
        let session = require_open_session(conn)?;
        let id = next_id(conn, "GEN_CASH_MOVEMENTS_ID")?;
        conn.execute(
            "INSERT INTO CASH_MOVEMENTS
                (ID, CASH_SESSION_ID, KIND, AMOUNT, REASON, CREATED_BY)
             VALUES (?, ?, ?, ?, ?, ?)",
            (&id, &session.id, &kind, &signed, &reason, &user_id),
        )
        .map_err(AppError::from)?;

        get_movement(conn, id)?
            .ok_or_else(|| AppError::Internal("Movimiento creado pero no recuperado".into()))
    })
}

fn get_movement(
    conn: &mut SimpleConnection,
    id: i32,
) -> Result<Option<CashMovement>, AppError> {
    let row: Option<MovementRow> = conn
        .query_first(
            "SELECT m.ID, m.CASH_SESSION_ID, m.KIND,
                    CAST(m.AMOUNT AS DOUBLE PRECISION), m.PAYMENT_ID, m.REASON,
                    u.FULL_NAME, LEFT(CAST(m.CREATED_AT AS VARCHAR(60)), 19)
             FROM CASH_MOVEMENTS m
             LEFT JOIN USERS u ON u.ID = m.CREATED_BY
             WHERE m.ID = ?",
            (&id,),
        )
        .map_err(AppError::from)?;
    Ok(row.map(map_movement))
}

/// Cierra la caja abierta: guarda el arqueo por denominación, los totales por
/// método y el snapshot de cifras. Después no se aceptan más pagos ni
/// movimientos en esa sesión.
pub fn close_session(
    conn: &mut SimpleConnection,
    input: &CloseCashInput,
    user_id: Option<i32>,
) -> Result<CashSessionSummary, AppError> {
    if input.count_lines.is_empty() {
        return Err(AppError::Validation(
            "Registra al menos una denominación en el arqueo".into(),
        ));
    }
    for line in &input.count_lines {
        if !line.denomination.is_finite() || line.denomination <= 0.0 {
            return Err(AppError::Validation(
                "Cada denominación del arqueo debe ser mayor que cero".into(),
            ));
        }
        if line.quantity < 0 {
            return Err(AppError::Validation(
                "La cantidad de billetes o monedas no puede ser negativa".into(),
            ));
        }
    }

    db::tx::in_transaction(conn, |conn| {
        let id: Option<(i32,)> = conn
            .query_first(
                "SELECT ID FROM CASH_SESSIONS WHERE STATUS = 'ABIERTA' WITH LOCK",
                (),
            )
            .map_err(AppError::from)?;
        let Some((id,)) = id else {
            return Err(AppError::Validation(
                "No hay una caja abierta que cerrar.".into(),
            ));
        };
        let session = get(conn, id)?
            .ok_or_else(|| AppError::Internal("Caja abierta no recuperada".into()))?;
        let figures = figures(conn, &session)?;

        let counted_cash: f64 = input
            .count_lines
            .iter()
            .map(|l| l.denomination * l.quantity as f64)
            .sum();
        let difference = counted_cash - figures.expected_cash;

        for line in &input.count_lines {
            if line.quantity == 0 {
                continue;
            }
            let line_id = next_id(conn, "GEN_CASH_COUNT_LINES_ID")?;
            conn.execute(
                "INSERT INTO CASH_COUNT_LINES
                    (ID, CASH_SESSION_ID, DENOMINATION, QUANTITY, SUBTOTAL)
                 VALUES (?, ?, ?, ?, ?)",
                (
                    &line_id,
                    &id,
                    &line.denomination,
                    &line.quantity,
                    &(line.denomination * line.quantity as f64),
                ),
            )
            .map_err(AppError::from)?;
        }

        // Totales por método congelados para el PDF y la auditoría.
        for method in &figures.methods {
            let total_id = next_id(conn, "GEN_CASH_SESSION_TOTALS_ID")?;
            conn.execute(
                "INSERT INTO CASH_SESSION_TOTALS
                    (ID, CASH_SESSION_ID, METHOD_CODE, AMOUNT)
                 VALUES (?, ?, ?, ?)",
                (&total_id, &id, &method.method_code, &method.amount),
            )
            .map_err(AppError::from)?;
        }

        conn.execute(
            "UPDATE CASH_SESSIONS
                SET STATUS = 'CERRADA', CLOSED_AT = CURRENT_TIMESTAMP, CLOSED_BY = ?,
                    EXPECTED_CASH = ?, COUNTED_CASH = ?, DIFFERENCE = ?,
                    TOTAL_BILLED = ?, TOTAL_COLLECTED = ?,
                    NOTES = COALESCE(?, NOTES)
              WHERE ID = ? AND STATUS = 'ABIERTA'",
            (
                &user_id,
                &figures.expected_cash,
                &counted_cash,
                &difference,
                &figures.total_billed,
                &figures.total_collected,
                &input.notes,
                &id,
            ),
        )
        .map_err(AppError::from)?;

        summary(conn, id)
    })
}

/// Historial de sesiones, más reciente primero.
pub fn list_sessions(
    conn: &mut SimpleConnection,
    limit: i32,
) -> Result<Vec<CashSession>, AppError> {
    let limit = limit.clamp(1, 200);
    let rows: Vec<CashSessionRow> = conn
        .query(
            &format!("{CASH_SESSION_SELECT} ORDER BY s.ID DESC ROWS 1 TO {limit}"),
            (),
        )
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(map_session).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cash::{CashCountInput, CloseCashInput};
    use crate::repositories::payments;
    use crate::test_helpers::{cleanup_test_db, setup_test_db};
    use std::path::PathBuf;

    fn setup() -> (SimpleConnection, PathBuf) {
        let (mut conn, db_path) = setup_test_db();
        conn.execute(
            "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
             VALUES (1, 'CC', '111', 'Tutor Uno')",
            (),
        )
        .unwrap();
        (conn, db_path)
    }

    fn insert_invoice(conn: &mut SimpleConnection, total: f64) {
        conn.execute(
            "INSERT INTO INVOICES
                (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT,
                 TOTAL, STATUS)
             VALUES (1, 'FAC-CAJA', 1, ?, 0, 0, ?, 'EMITIDA')",
            (&total, &total),
        )
        .unwrap();
    }

    fn pay_cash(conn: &mut SimpleConnection, amount: f64) {
        payments::register(
            conn,
            &crate::models::payment::RegisterPaymentInput {
                owner_id: 1,
                patient_id: None,
                notes: None,
                lines: vec![crate::models::payment::CreatePaymentLineInput {
                    method_code: "EFECTIVO".into(),
                    amount,
                    reference: None,
                }],
                allocations: vec![crate::models::payment::CreatePaymentAllocationInput {
                    invoice_id: 1,
                    amount,
                }],
            },
            Some(1),
        )
        .unwrap();
    }

    fn count(denomination: f64, quantity: i32) -> CashCountInput {
        CashCountInput {
            denomination,
            quantity,
        }
    }

    #[test]
    fn expense_reduces_expected_cash() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 100000.0, None, 1).unwrap();

        let movement = add_movement(&mut conn, "GASTO", 20000.0, "Insumos", Some(1)).unwrap();
        assert_eq!(movement.kind, "GASTO");
        assert!((movement.amount + 20000.0).abs() < 0.01);

        let open = get_open_session(&mut conn).unwrap().unwrap();
        let summary = summary(&mut conn, open.id).unwrap();
        assert!((summary.expected_cash - 80000.0).abs() < 0.01);
        assert!((summary.cash_out - 20000.0).abs() < 0.01);
        assert_eq!(summary.movements.len(), 1);

        cleanup_test_db(&db_path);
    }

    #[test]
    fn rejects_movement_without_reason_or_with_bad_kind() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 0.0, None, 1).unwrap();

        assert!(matches!(
            add_movement(&mut conn, "GASTO", 1000.0, "   ", Some(1)),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            add_movement(&mut conn, "PAGO_EFECTIVO", 1000.0, "x", Some(1)),
            Err(AppError::Validation(_))
        ));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn close_saves_snapshot_and_count_lines() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 100000.0, None, 1).unwrap();
        insert_invoice(&mut conn, 40000.0);
        pay_cash(&mut conn, 40000.0);
        add_movement(&mut conn, "RETIRO", 10000.0, "Retiro a bóveda", None).unwrap();

        let open = get_open_session(&mut conn).unwrap().unwrap();
        let before = summary(&mut conn, open.id).unwrap();
        assert!((before.expected_cash - 130000.0).abs() < 0.01);
        assert!((before.total_collected - 40000.0).abs() < 0.01);
        assert!((before.total_billed - 40000.0).abs() < 0.01);
        assert_eq!(before.payments_count, 1);

        let closed = close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![count(100000.0, 1), count(20000.0, 1), count(10000.0, 1)],
                notes: Some("Cierre de prueba".into()),
            },
            None,
        )
        .unwrap();

        assert_eq!(closed.session.status, "CERRADA");
        assert!((closed.counted_cash.unwrap() - 130000.0).abs() < 0.01);
        assert!((closed.expected_cash - 130000.0).abs() < 0.01);
        assert!((closed.difference.unwrap()).abs() < 0.01);
        assert_eq!(closed.count_lines.len(), 3);
        assert_eq!(closed.methods.len(), 1);
        assert_eq!(closed.methods[0].method_code, "EFECTIVO");
        assert!(get_open_session(&mut conn).unwrap().is_none());

        cleanup_test_db(&db_path);
    }

    #[test]
    fn close_computes_shortage_and_rejects_second_close() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 50000.0, None, 1).unwrap();

        let closed = close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![count(50000.0, 1)],
                notes: None,
            },
            Some(1),
        )
        .unwrap();
        assert!((closed.counted_cash.unwrap() - 50000.0).abs() < 0.01);

        // Una caja cerrada no admite un segundo cierre.
        let again = close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![count(1000.0, 1)],
                notes: None,
            },
            Some(1),
        );
        assert!(matches!(again, Err(AppError::Validation(_))));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn closed_session_blocks_payments_and_movements() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 0.0, None, 1).unwrap();
        insert_invoice(&mut conn, 50000.0);
        close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![count(50000.0, 1)],
                notes: None,
            },
            Some(1),
        )
        .unwrap();

        assert!(matches!(
            add_movement(&mut conn, "GASTO", 5000.0, "Después del cierre", Some(1)),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            payments::register(
                &mut conn,
                &crate::models::payment::RegisterPaymentInput {
                    owner_id: 1,
                    patient_id: None,
                    notes: None,
                    lines: vec![crate::models::payment::CreatePaymentLineInput {
                        method_code: "EFECTIVO".into(),
                        amount: 50000.0,
                        reference: None,
                    }],
                    allocations: vec![crate::models::payment::CreatePaymentAllocationInput {
                        invoice_id: 1,
                        amount: 50000.0,
                    }],
                },
                Some(1),
            ),
            Err(AppError::Validation(_))
        ));

        cleanup_test_db(&db_path);
    }

    #[test]
    fn list_sessions_returns_most_recent_first() {
        let (mut conn, db_path) = setup();
        open_session(&mut conn, 10000.0, None, 1).unwrap();
        close_session(
            &mut conn,
            &CloseCashInput {
                count_lines: vec![count(10000.0, 1)],
                notes: None,
            },
            Some(1),
        )
        .unwrap();
        open_session(&mut conn, 20000.0, None, 1).unwrap();

        let sessions = list_sessions(&mut conn, 10).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].status, "ABIERTA");
        assert_eq!(sessions[1].status, "CERRADA");

        cleanup_test_db(&db_path);
    }
}
