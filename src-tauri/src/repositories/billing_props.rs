//! Property tests de caja, abonos y anulación de pagos.
//!
//! Genera secuencias aleatorias de facturas, abonos mixtos (efectivo +
//! transferencia, uno o varios pagos por factura, pagos que cubren dos facturas
//! a la vez) y anulaciones, y comprueba que los totales derivados cuadran con
//! sus movimientos:
//!
//!   * factura:  `pagado` = suma de asignaciones de pagos APLICADOS, y
//!               `saldo` = max(0, total − pagado), con el estado coherente;
//!   * caja:     `esperado` = fondo inicial + suma firmada de movimientos,
//!               `entrado`/`salido` = movimientos positivos/negativos, y
//!               `cobrado`/`nº pagos` = recepciones vigentes de la sesión;
//!   * conservación: lo facturado = saldo pendiente + abonos vigentes, y cada
//!               pago aplica exactamente su total.
//!
//! Un caso que falle se reduce (proptest) y se imprime con su semilla.

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use rsfbclient::prelude::*;
use rsfbclient::SimpleConnection;

use crate::models::payment::{
    CreatePaymentAllocationInput, CreatePaymentLineInput, RegisterPaymentInput,
};
use crate::repositories::{cash, payments};
use crate::test_helpers::{cleanup_test_db, setup_test_db};

const OWNER_ID: i32 = 1;
const USER_ID: i32 = 1;
/// Tolerancia en pesos: los importes se generan en céntimos enteros, así que
/// solo el redondeo de Firebird a `DOUBLE PRECISION` puede moverlos.
const TOL: f64 = 0.005;

/// Un abono: qué facturas cubre, qué porcentaje del saldo de cada una, cuánto
/// va en efectivo y si después se anula.
#[derive(Debug, Clone)]
struct Step {
    invoices: Vec<usize>,
    percent: u32,
    cash_percent: u32,
    void: bool,
}

fn case_strategy() -> impl Strategy<Value = (Vec<u32>, Vec<Step>, u32)> {
    (
        prop::collection::vec(100u32..=400_000, 1..=4),
        prop::collection::vec(
            (
                prop::collection::vec(0usize..4, 1..=2),
                1u32..=100,
                0u32..=100,
                any::<bool>(),
            ),
            0..=10,
        )
        .prop_map(|raw| {
            raw.into_iter()
                .map(|(invoices, percent, cash_percent, void)| Step {
                    invoices,
                    percent,
                    cash_percent,
                    void,
                })
                .collect()
        }),
        0u32..=200_000,
    )
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOL
}

/// Deja las tablas del flujo de dinero vacías para el siguiente caso.
fn reset(conn: &mut SimpleConnection) {
    for statement in [
        "DELETE FROM PAYMENT_ALLOCATIONS",
        "DELETE FROM PAYMENT_LINES",
        "DELETE FROM CASH_MOVEMENTS",
        "DELETE FROM PAYMENTS",
        "DELETE FROM CASH_COUNT_LINES",
        "DELETE FROM CASH_SESSION_TOTALS",
        "DELETE FROM CASH_SESSIONS",
        "DELETE FROM INVOICES",
    ] {
        conn.execute(statement, ()).unwrap();
    }
}

fn insert_invoice(
    conn: &mut SimpleConnection,
    id: i32,
    total_cents: u64,
) -> Result<(), TestCaseError> {
    let total = total_cents as f64 / 100.0;
    conn.execute(
        "INSERT INTO INVOICES
            (ID, INVOICE_NUMBER, OWNER_ID, SUBTOTAL, TAX_RATE, TAX_AMOUNT, TOTAL, STATUS)
         VALUES (?, ?, ?, ?, 0, 0, ?, 'EMITIDA')",
        (&id, &format!("FAC-P{id:03}"), &OWNER_ID, &total, &total),
    )
    .map_err(|e| TestCaseError::fail(format!("no se pudo crear la factura: {e}")))?;
    Ok(())
}

/// Lo que el modelo del test espera de una factura.
#[derive(Debug, Default, Clone, Copy)]
struct InvoiceModel {
    total_cents: u64,
    paid_cents: u64,
}

fn run_case(
    conn: &mut SimpleConnection,
    totals: &[u32],
    steps: &[Step],
    opening_cents: u32,
) -> Result<(), TestCaseError> {
    reset(conn);
    let mut model: Vec<InvoiceModel> = totals
        .iter()
        .map(|total| InvoiceModel {
            total_cents: *total as u64,
            paid_cents: 0,
        })
        .collect();
    for (index, invoice) in model.iter().enumerate() {
        insert_invoice(conn, index as i32 + 1, invoice.total_cents)?;
    }

    let opening = opening_cents as f64 / 100.0;
    let session = cash::open_session(conn, opening, None, USER_ID)
        .map_err(|e| TestCaseError::fail(format!("no se pudo abrir la caja: {e}")))?;

    // Efectivo de las recepciones vigentes y de las anuladas, en céntimos.
    let mut active_cash: u64 = 0;
    let mut voided_cash: u64 = 0;
    let mut active_total: u64 = 0;
    let mut active_count: usize = 0;

    for (step_index, step) in steps.iter().enumerate() {
        // Una factura solo puede aparecer una vez por pago.
        let mut targets: Vec<usize> = Vec::new();
        for invoice in &step.invoices {
            let index = *invoice % totals.len();
            if !targets.contains(&index) {
                targets.push(index);
            }
        }

        let mut allocations: Vec<(usize, u64)> = Vec::new();
        for index in targets {
            let remaining = model[index]
                .total_cents
                .saturating_sub(model[index].paid_cents);
            if remaining == 0 {
                continue;
            }
            let amount = ((remaining * step.percent as u64) / 100)
                .max(1)
                .min(remaining);
            allocations.push((index, amount));
        }
        if allocations.is_empty() {
            continue;
        }

        let total_cents: u64 = allocations.iter().map(|(_, amount)| amount).sum();
        let cash_cents = total_cents * step.cash_percent as u64 / 100;
        let electronic_cents = total_cents - cash_cents;

        let mut lines = Vec::new();
        if cash_cents > 0 {
            lines.push(CreatePaymentLineInput {
                method_code: "EFECTIVO".into(),
                amount: cash_cents as f64 / 100.0,
                reference: None,
            });
        }
        if electronic_cents > 0 {
            lines.push(CreatePaymentLineInput {
                method_code: "TRANSFERENCIA".into(),
                amount: electronic_cents as f64 / 100.0,
                reference: Some(format!("REF-{step_index}")),
            });
        }

        let input = RegisterPaymentInput {
            owner_id: OWNER_ID,
            patient_id: None,
            notes: None,
            lines,
            allocations: allocations
                .iter()
                .map(|(index, amount)| CreatePaymentAllocationInput {
                    invoice_id: *index as i32 + 1,
                    amount: *amount as f64 / 100.0,
                })
                .collect(),
        };

        let payment = payments::register(conn, &input, Some(USER_ID)).map_err(|e| {
            TestCaseError::fail(format!("el abono debía registrarse ({e}): {input:?}"))
        })?;
        // Invariante del propio recibo.
        prop_assert!(
            close(payment.total, total_cents as f64 / 100.0),
            "el recibo cobró {} pero las asignaciones suman {}",
            payment.total,
            total_cents as f64 / 100.0
        );
        let allocated: f64 = payment.allocations.iter().map(|a| a.amount).sum();
        prop_assert!(
            close(allocated, payment.total),
            "las asignaciones del recibo suman {allocated} y su total es {}",
            payment.total
        );

        if step.void {
            payments::void(conn, payment.id, "prueba de propiedad", Some(USER_ID))
                .map_err(|e| TestCaseError::fail(format!("la anulación debía funcionar: {e}")))?;
            voided_cash += cash_cents;
        } else {
            for (index, amount) in &allocations {
                model[*index].paid_cents += amount;
            }
            active_cash += cash_cents;
            active_total += total_cents;
            active_count += 1;
        }
    }

    // ---------- Facturas: el saldo cuadra con sus asignaciones ----------
    let mut total_invoiced: u64 = 0;
    let mut total_balance: u64 = 0;
    for (index, invoice) in model.iter().enumerate() {
        let balance = payments::invoice_balance(conn, index as i32 + 1)
            .map_err(|e| TestCaseError::fail(format!("no se pudo leer la factura: {e}")))?;
        let expected_balance = invoice.total_cents - invoice.paid_cents;
        // El backend compara saldos con tolerancia de un céntimo (plan-abonos-caja
        // §3.4): un residuo de 0.01 cuenta como cobrado y un abono de 0.01 como cero.
        let expected_status = if expected_balance <= 1 {
            "PAGADA"
        } else if invoice.paid_cents > 1 {
            "ABONADA"
        } else {
            "PENDIENTE"
        };

        prop_assert!(
            close(balance.total, invoice.total_cents as f64 / 100.0),
            "factura {}: total {} ≠ esperado {}",
            balance.invoice_number,
            balance.total,
            invoice.total_cents as f64 / 100.0
        );
        prop_assert!(
            close(balance.paid_amount, invoice.paid_cents as f64 / 100.0),
            "factura {}: pagado {} ≠ abonos vigentes {} (pasos: {steps:?})",
            balance.invoice_number,
            balance.paid_amount,
            invoice.paid_cents as f64 / 100.0
        );
        prop_assert!(
            close(balance.balance, expected_balance as f64 / 100.0),
            "factura {}: saldo {} ≠ esperado {}",
            balance.invoice_number,
            balance.balance,
            expected_balance as f64 / 100.0
        );
        prop_assert_eq!(
            balance.payment_status.as_str(),
            expected_status,
            "factura {} con pagado {} y saldo {}",
            balance.invoice_number,
            balance.paid_amount,
            balance.balance
        );

        total_invoiced += invoice.total_cents;
        total_balance += expected_balance;
    }

    // ---------- Caja: el efectivo cuadra con sus movimientos ----------
    let summary = cash::summary(conn, session.id)
        .map_err(|e| TestCaseError::fail(format!("no se pudo leer la caja: {e}")))?;
    // Las recepciones anuladas dejan su ingreso y su devolución en la caja
    // (rastro de auditoría), así que se cancelan: el efectivo neto solo lleva
    // el de las vigentes. Sin embargo `cash_in`/`cash_out` los ven a ambos.
    let expected_cash = opening_cents as i64 + active_cash as i64;

    prop_assert!(
        close(summary.opening_float, opening),
        "fondo inicial {} ≠ {}",
        summary.opening_float,
        opening
    );
    prop_assert!(
        close(summary.expected_cash, expected_cash as f64 / 100.0),
        "efectivo esperado {} ≠ fondo {opening} + recepciones vigentes {} (anuladas devueltas: {}; pasos: {steps:?})",
        summary.expected_cash,
        active_cash as f64 / 100.0,
        voided_cash as f64 / 100.0
    );
    prop_assert!(
        close(summary.cash_in, (active_cash + voided_cash) as f64 / 100.0),
        "efectivo entrado {} ≠ {} de las recepciones (vigentes + anuladas)",
        summary.cash_in,
        (active_cash + voided_cash) as f64 / 100.0
    );
    prop_assert!(
        close(summary.cash_out, voided_cash as f64 / 100.0),
        "efectivo devuelto {} ≠ {} de las anulaciones",
        summary.cash_out,
        voided_cash as f64 / 100.0
    );
    prop_assert!(
        close(summary.total_collected, active_total as f64 / 100.0),
        "cobrado {} ≠ {} de las recepciones vigentes",
        summary.total_collected,
        active_total as f64 / 100.0
    );
    prop_assert_eq!(
        summary.payments_count,
        active_count as i32,
        "nº de pagos vigentes"
    );

    // Cada movimiento de caja explica el total esperado.
    let movements_total: f64 = summary.movements.iter().map(|m| m.amount).sum();
    prop_assert!(
        close(summary.expected_cash, opening + movements_total),
        "efectivo esperado {} ≠ fondo {} + movimientos {movements_total}",
        summary.expected_cash,
        opening
    );

    // ---------- Conservación ----------
    let total_paid: u64 = model.iter().map(|invoice| invoice.paid_cents).sum();
    prop_assert_eq!(
        total_invoiced,
        total_balance + total_paid,
        "lo facturado no cuadra con saldos + abonos vigentes"
    );

    Ok(())
}

#[test]
fn los_saldos_de_factura_y_caja_cuadran_con_sus_movimientos() {
    let (mut conn, db_path) = setup_test_db();
    conn.execute(
        "INSERT INTO OWNERS (ID, DOCUMENT_TYPE, DOCUMENT_NUMBER, FULL_NAME)
         VALUES (1, 'CC', '111', 'Tutor Uno')",
        (),
    )
    .unwrap();

    // `TestRunner::run` exige una `Fn`, así que la conexión se presta por celda
    // (los casos se ejecutan en serie, uno detrás de otro).
    let conn = std::cell::RefCell::new(conn);
    let mut runner = TestRunner::new(Config {
        cases: 48,
        ..Config::default()
    });
    let result = runner.run(&case_strategy(), |(totals, steps, opening_cents)| {
        let mut conn = conn.borrow_mut();
        run_case(&mut conn, &totals, &steps, opening_cents)
    });

    cleanup_test_db(&db_path);
    if let Err(failure) = result {
        panic!("{failure}");
    }
}
