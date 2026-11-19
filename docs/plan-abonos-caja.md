# Plan de implementación — Abonos, pagos mixtos y cierre de caja

**Estado:** propuesta (no implementada)
**Base:** schema 0035 · `INVOICES` con un único `PAYMENT_METHOD` y estados `EMITIDA/PAGADA/ANULADA`, cobro total desde el detalle o `checkout_visit`, sin recepciones de pago ni caja.
**Objetivo:** registrar cobros parciales y mixtos con trazabilidad, calcular saldos por factura y cerrar la caja diaria con arqueo y diferencia.

> **Numeración reservada:** `0036_payments_cash.sql`. Orden acordado: abonos/caja
> (`0036`), receta (`0037`–`0038`) y recordatorios (`0039`). Si cambia el orden de
> implementación, renumerar los tres planes antes de crear la migración.

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Abonos | Varios pagos por factura; saldo pendiente; estado de cobro derivado |
| Pago mixto | Una recepción con varias líneas (efectivo + transferencia + tarjeta…) |
| Aplicación | El pago se distribuye entre una o varias facturas del mismo tutor |
| Caja | Apertura con base, movimientos, gastos/retiros, arqueo por denominación y cierre |
| Recibos | Comprobante por recepción y PDF de cierre de caja |
| Reportes | Cobrado hoy/mes, saldo pendiente, medios de pago, diferencia de caja |
| Legado | Backfill de facturas `PAGADA` existentes a una recepción histórica |

### 1.2 No entra (MVP)

- Crédito a favor del cliente o pagos a cuenta sin factura.
- Facturación electrónica / notas crédito DIAN.
- Pagos con pasarela en línea o datáfono integrado.
- Apertura simultánea de varias cajas por sede (una caja lógica por instalación).
- Reapertura de cierres (solo ajuste auditado en la caja siguiente).

### 1.3 Principios

1. **La factura no se reescribe.** El total es inmutable; los pagos se aplican contra él y
   el saldo se deriva de las asignaciones.
2. **Una recepción, varias formas de pago.** El dinero entra una vez y se reparte en
   líneas de método; la suma debe cuadrar exactamente.
3. **Nada se cobra fuera de una caja abierta.** Todo pago pertenece a una sesión, para que
   el cierre sea reproducible.
4. **Estados derivados, no duplicados.** `PAID_AMOUNT`, `BALANCE` y `PAYMENT_STATUS` se
   calculan; no se guardan columnas que puedan desincronizarse.
5. **Caja cerrada = inmutable.** Después del cierre solo se corrige con un ajuste auditado
   en la sesión siguiente.

---

## 2. Modelo de datos

Migración **`0036_payments_cash.sql`**.

### 2.1 Métodos de pago

```sql
CREATE TABLE PAYMENT_METHODS (
    CODE               VARCHAR(30) CHARACTER SET UTF8 NOT NULL PRIMARY KEY,
    NAME               VARCHAR(60) CHARACTER SET UTF8 NOT NULL,
    -- EFECTIVO | ELECTRONICO | OTRO (define qué entra al arqueo)
    KIND               VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'OTRO' NOT NULL,
    REQUIRES_REFERENCE BOOLEAN DEFAULT FALSE NOT NULL,
    ACTIVE             BOOLEAN DEFAULT TRUE NOT NULL,
    SORT_ORDER         INTEGER DEFAULT 0 NOT NULL
);

INSERT INTO PAYMENT_METHODS (CODE, NAME, KIND, REQUIRES_REFERENCE, SORT_ORDER) VALUES
 ('EFECTIVO',        'Efectivo',          'EFECTIVO',   FALSE, 10),
 ('TRANSFERENCIA',   'Transferencia',     'ELECTRONICO',TRUE,  20),
 ('TARJETA_CREDITO', 'Tarjeta crédito',   'ELECTRONICO',TRUE,  30),
 ('TARJETA_DEBITO',  'Tarjeta débito',    'ELECTRONICO',TRUE,  40),
 ('NEQUI',           'Nequi',             'ELECTRONICO',TRUE,  50),
 ('DAVIPLATA',       'Daviplata',         'ELECTRONICO',TRUE,  60),
 ('OTRO',            'Otro',              'OTRO',       FALSE, 90);
```

Los códigos existentes (`EFECTIVO`, `TRANSFERENCIA`, `TARJETA_CREDITO`,
`TARJETA_DEBITO`) se conservan; `INVOICES.PAYMENT_METHOD` queda como dato legado.

### 2.2 Recepciones de pago

```sql
CREATE TABLE PAYMENTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    CODE            D_CODE NOT NULL UNIQUE,        -- REC-000001 (trigger)
    OWNER_ID        D_PK NOT NULL REFERENCES OWNERS (ID),
    PATIENT_ID      D_PK REFERENCES PATIENTS (ID) ON DELETE SET NULL,
    CASH_SESSION_ID D_PK REFERENCES CASH_SESSIONS (ID) ON DELETE SET NULL,
    RECEIVED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    TOTAL           D_AMOUNT NOT NULL CHECK (TOTAL > 0),
    -- APLICADO | ANULADO
    STATUS          VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'APLICADO' NOT NULL,
    NOTES           D_NOTES,
    VOIDED_AT       TIMESTAMP,
    VOIDED_BY       D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    VOID_REASON     D_NOTES,
    CREATED_BY      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_PAYMENTS_ID;

CREATE TABLE PAYMENT_LINES (
    ID          D_PK NOT NULL PRIMARY KEY,
    PAYMENT_ID  D_PK NOT NULL REFERENCES PAYMENTS (ID) ON DELETE CASCADE,
    METHOD_CODE VARCHAR(30) CHARACTER SET UTF8 NOT NULL
                REFERENCES PAYMENT_METHODS (CODE),
    AMOUNT      D_AMOUNT NOT NULL CHECK (AMOUNT > 0),
    REFERENCE   VARCHAR(80) CHARACTER SET UTF8,   -- aprobación, transferencia…
    NOTES       D_NOTES
);
CREATE GENERATOR GEN_PAYMENT_LINES_ID;

CREATE TABLE PAYMENT_ALLOCATIONS (
    ID          D_PK NOT NULL PRIMARY KEY,
    PAYMENT_ID  D_PK NOT NULL REFERENCES PAYMENTS (ID) ON DELETE CASCADE,
    INVOICE_ID  D_PK NOT NULL REFERENCES INVOICES (ID),
    AMOUNT      D_AMOUNT NOT NULL CHECK (AMOUNT > 0),
    UNIQUE (PAYMENT_ID, INVOICE_ID)
);
CREATE GENERATOR GEN_PAYMENT_ALLOCATIONS_ID;

CREATE INDEX IX_PAYMENTS_OWNER ON PAYMENTS (OWNER_ID, RECEIVED_AT);
CREATE INDEX IX_PAYMENTS_SESSION ON PAYMENTS (CASH_SESSION_ID);
CREATE INDEX IX_PAYMENT_ALLOCATIONS_INVOICE ON PAYMENT_ALLOCATIONS (INVOICE_ID);
```

### 2.3 Caja

```sql
CREATE TABLE CASH_SESSIONS (
    ID            D_PK NOT NULL PRIMARY KEY,
    CODE          D_CODE NOT NULL UNIQUE,          -- CAJ-20261119-01
    OPENED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    OPENED_BY     D_PK NOT NULL REFERENCES USERS (ID),
    OPENING_FLOAT D_AMOUNT DEFAULT 0 NOT NULL,
    STATUS        VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'ABIERTA' NOT NULL
                  CHECK (STATUS IN ('ABIERTA', 'CERRADA')),
    CLOSED_AT     TIMESTAMP,
    CLOSED_BY     D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    -- Snapshot del cierre (no se recalcula después).
    EXPECTED_CASH D_AMOUNT,
    COUNTED_CASH  D_AMOUNT,
    DIFFERENCE    D_AMOUNT,
    TOTAL_BILLED  D_AMOUNT,
    TOTAL_COLLECTED D_AMOUNT,
    NOTES         D_NOTES,
    -- Solo puede existir una caja ABIERTA: índice único sobre 1/NULL.
    OPEN_KEY      COMPUTED BY (CASE WHEN STATUS = 'ABIERTA' THEN 1 ELSE NULL END)
);
CREATE GENERATOR GEN_CASH_SESSIONS_ID;
CREATE UNIQUE INDEX UX_CASH_SESSIONS_OPEN ON CASH_SESSIONS (OPEN_KEY);

CREATE TABLE CASH_MOVEMENTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    CASH_SESSION_ID D_PK NOT NULL REFERENCES CASH_SESSIONS (ID),
    -- PAGO_EFECTIVO | GASTO | RETIRO | DEVOLUCION | AJUSTE
    KIND            VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    -- Firmado: pagos/ingresos positivos; gastos/retiros/devoluciones negativos.
    AMOUNT          D_AMOUNT NOT NULL CHECK (AMOUNT <> 0),
    PAYMENT_ID      D_PK REFERENCES PAYMENTS (ID) ON DELETE SET NULL,
    REASON          D_NOTES,
    CREATED_BY      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_CASH_MOVEMENTS_ID;
CREATE INDEX IX_CASH_MOVEMENTS_SESSION ON CASH_MOVEMENTS (CASH_SESSION_ID, CREATED_AT);

CREATE TABLE CASH_COUNT_LINES (
    ID              D_PK NOT NULL PRIMARY KEY,
    CASH_SESSION_ID D_PK NOT NULL REFERENCES CASH_SESSIONS (ID) ON DELETE CASCADE,
    DENOMINATION    D_AMOUNT NOT NULL CHECK (DENOMINATION > 0),
    QUANTITY        INTEGER NOT NULL CHECK (QUANTITY >= 0),
    SUBTOTAL        D_AMOUNT NOT NULL
);
CREATE GENERATOR GEN_CASH_COUNT_LINES_ID;

-- Totales por método congelados al cerrar (para el PDF y auditoría).
CREATE TABLE CASH_SESSION_TOTALS (
    ID              D_PK NOT NULL PRIMARY KEY,
    CASH_SESSION_ID D_PK NOT NULL REFERENCES CASH_SESSIONS (ID) ON DELETE CASCADE,
    METHOD_CODE     VARCHAR(30) CHARACTER SET UTF8 NOT NULL,
    AMOUNT          D_AMOUNT NOT NULL
);
CREATE GENERATOR GEN_CASH_SESSION_TOTALS_ID;
```

### 2.4 Backfill de facturas pagadas

Las facturas `PAGADA` anteriores no tienen recepción. La migración crea una recepción
histórica por factura:

- `PAYMENTS.CODE = 'LEG-' || INVOICE_NUMBER`, `CASH_SESSION_ID = NULL`,
  `RECEIVED_AT = INVOICE.ISSUE_DATE`, `TOTAL = INVOICE.TOTAL`.
- Una línea con el `PAYMENT_METHOD` existente (o `OTRO` si no está en el catálogo).
- Una asignación por el total.
- `NOTES = 'Migrado desde facturación anterior'`.

Consecuencia: el arqueo histórico **no** se reconstruye; el cierre de caja empieza a
contar desde la implementación. Los reportes de cobros sí incluyen el legado.

### 2.5 Estado de cobro derivado

No se altera el `CHECK` de `INVOICES.STATUS` (evita cirugía de constraints). Se mantiene:

- `EMITIDA`: factura con saldo (aunque tenga abonos).
- `PAGADA`: saldo cero.
- `ANULADA`: anulada sin pagos activos.

En los DTO se agrega:

```text
paidAmount = Σ asignaciones de pagos APLICADO
balance    = TOTAL - paidAmount
paymentStatus = ANULADA | PAGADA (balance ≈ 0) | ABONADA (paidAmount > 0) | PENDIENTE
```

La UI muestra `paymentStatus`; `status` queda para compatibilidad y reportes históricos.

---

## 3. Reglas de negocio

### 3.1 Registrar una recepción

```text
register_payment(input):
  validar caja ABIERTA
  validar que todas las facturas sean del mismo OWNER_ID, no ANULADAS
  total_pago = Σ líneas
  total_asignado = Σ asignaciones
  exigir total_asignado == total_pago (±0.01)
  por factura: asignación <= saldo actual (±0.01)
  insertar PAYMENTS + PAYMENT_LINES + PAYMENT_ALLOCATIONS
  si hay línea EFECTIVO: CASH_MOVEMENTS (PAGO_EFECTIVO, +importe)
  recalcular STATUS de cada factura afectada
  auditar PAYMENT_REGISTERED
```

- Un pago puede cubrir **varias facturas** del mismo tutor (p. ej. consulta + farmacia).
- No se permite sobrepago ni saldo a favor en el MVP: el excedente se rechaza y la UI
  sugiere ajustar la asignación.
- El cambio de efectivo no se modela: la línea de caja es el importe **aplicado**, no el
  billete recibido.
- Se permiten varios pagos parciales sobre la misma factura; cada uno genera recibo.

### 3.2 Anulación de pagos

- Solo con la caja de la recepción **abierta**; revierte la asignación, el movimiento de
  caja y el estado de las facturas.
- Si la caja ya cerró: solo ADMIN, y genera un `CASH_MOVEMENTS` de `DEVOLUCION`/`AJUSTE`
  negativo en la caja abierta actual, más auditoría. Nunca se edita el cierre histórico.
- Una factura con pagos activos no se puede anular: primero se anulan los pagos.

### 3.3 Caja

- Solo puede haber **una sesión ABIERTA** (índice único sobre `OPEN_KEY`).
- Todo pago nuevo exige sesión abierta; si no hay, la UI ofrece abrirla con base inicial.
- Movimientos manuales permitidos: `GASTO`, `RETIRO`, `AJUSTE` (con motivo); requieren
  capacidad de facturación.
- `expected_cash = opening_float + Σ CASH_MOVEMENTS.AMOUNT` (importes firmados).
- `counted_cash = Σ CASH_COUNT_LINES.SUBTOTAL`.
- `difference = counted_cash - expected_cash`.
- Al cerrar: guardar snapshot (`EXPECTED_CASH`, `COUNTED_CASH`, `DIFFERENCE`,
  `TOTAL_BILLED`, `TOTAL_COLLECTED`), totales por método en `CASH_SESSION_TOTALS`,
  estado `CERRADA`; disparar PDF de cierre.
- Un trigger en `PAYMENTS`/`CASH_MOVEMENTS` rechaza inserciones contra una sesión cerrada.

### 3.4 Redondeo

`D_AMOUNT` es decimal con 2 posiciones. Comparaciones de saldo con tolerancia `0.01`.
Los pagos en COP normalmente no usan centavos, pero se conserva la precisión.

---

## 4. Backend

### 4.1 Archivos

```text
src-tauri/src/models/payment.rs          Payment, PaymentLine, Allocation, inputs, methods
src-tauri/src/models/cash.rs             CashSession, CashMovement, CashCountLine, summary
src-tauri/src/repositories/payments.rs   Registro, anulación, saldos, recibos
src-tauri/src/repositories/cash.rs       Apertura, movimientos, cierre, arqueo
src-tauri/src/repositories/invoices.rs   Saldos y estado derivado; bloquear anulación con pagos
src-tauri/src/commands/payments.rs
src-tauri/src/commands/cash.rs
src-tauri/src/pdf_templates/financial.rs Recibo de pago y cierre de caja
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs`, `lib.rs`
(`collect_commands!` + `.typ::<>`); regenerar `src/bindings.ts`.
Añadir `require_bill` (ya existe) para cobrar/cerrar y `require_admin` para ajustes
posteriores al cierre.

### 4.2 Comandos

| Comando | Capacidad | Descripción |
| --- | --- | --- |
| `list_payment_methods(include_inactive)` | sesión | Catálogo de métodos |
| `register_payment(input)` | facturación | Recepción + líneas + asignaciones; devuelve recibo |
| `void_payment(id, reason)` | facturación/admin | Anulación con reverso de caja y saldos |
| `list_invoice_payments(invoice_id)` | sesión | Historial de abonos de una factura |
| `get_invoice_balance(invoice_id)` | sesión | Total, pagado, saldo, estado de cobro |
| `list_receivables(search?, owner_id?)` | sesión | Facturas con saldo > 0 |
| `generate_payment_receipt_pdf(payment_id)` | sesión | Recibo `REC-…` |
| `get_open_cash_session()` | sesión | Sesión abierta o `null` |
| `open_cash_session(opening_float, notes?)` | facturación | Abre caja (falla si ya hay una) |
| `add_cash_movement(kind, amount, reason)` | facturación | Gasto, retiro o ajuste |
| `list_cash_movements(session_id?)` | sesión | Movimientos de la sesión |
| `close_cash_session(count_lines, notes?)` | facturación | Arqueo, snapshot y cierre |
| `list_cash_sessions(limit?)` | sesión | Historial de cierres |
| `get_cash_session_summary(id)` | sesión | Totales por método, facturado, cobrado, diferencia |
| `generate_cash_close_pdf(session_id)` | sesión | “Cierre Z” |

`RegisterPaymentInput`:

```text
owner_id, patient_id?, notes?,
lines: [{ method_code, amount, reference? }],
allocations: [{ invoice_id, amount }]
```

`checkout_visit` se mantiene: emite la factura. El cobro se hace después con
`register_payment`; si el pago falla, la factura queda `EMITIDA` y visible en cartera.

### 4.3 Consultas de saldo

```sql
SELECT i.TOTAL - COALESCE((
    SELECT SUM(a.AMOUNT)
    FROM PAYMENT_ALLOCATIONS a
    JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
    WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
), 0) AS BALANCE
FROM INVOICES i
WHERE i.ID = ?
```

El listado de facturas incorpora `paidAmount`, `balance` y `paymentStatus` con subconsultas
equivalentes; los filtros “Abonadas” y “Con saldo” se resuelven en SQL, no en el cliente.

### 4.4 Concurrencia

- Registrar pago y recalcular estado en una sola transacción.
- Bloquear la factura (`SELECT … WITH LOCK`) antes de validar el saldo, o usar
  compare-and-set sobre el saldo calculado; dos cajas no pueden sobrepasar el total.
- La unicidad de caja abierta la garantiza el índice, no solo la consulta previa.
- El cierre bloquea la sesión y verifica que siga `ABIERTA` antes de escribir el snapshot.

---

## 5. Cambios de UI

### 5.1 Facturación (`InvoicesPage`)

- Tabs: Todas · Pendientes · Abonadas · Pagadas · Anuladas (por `paymentStatus`).
- Columnas: Total · Abonado · Saldo · Estado · Último pago.
- Acción rápida **Cobrar** en filas con saldo.
- Tarjeta resumen: facturado del período, cobrado, saldo pendiente.

### 5.2 Detalle (`InvoiceDetailDialog`)

- Totales con Abonado y Saldo.
- Sección **Pagos**: fecha, recibo, métodos, importe, usuario; botón “Ver recibo” y
  “Anular” (según caja/rol).
- Botón **Registrar pago** (abre `PaymentDialog` con el saldo precargado).
- Se elimina “Marcar como pagada” directo: cobrar siempre crea una recepción.
- “Anular” deshabilitado si hay pagos activos, con explicación.

### 5.3 `PaymentDialog` (reutilizable)

- Cabecera: tutor, paciente, facturas a cobrar (por defecto la abierta), saldo.
- Distribución del pago entre facturas del mismo tutor (si hay varias pendientes).
- Líneas de método con importe y referencia; botón “Agregar método”.
- Totales en vivo: total a cobrar, suma de líneas, diferencia (debe ser 0).
- Aviso y acceso directo a **Abrir caja** si no hay sesión.
- Al confirmar: toast con recibo y opción de imprimir.

### 5.4 Cobro de visita (`CheckoutDialog`)

- Tras emitir la factura, abre `PaymentDialog` con el total.
- Si se cancela el pago, la factura queda pendiente y se cobra desde Facturación.

### 5.5 Nueva vista **Caja** (`cash`)

- Entrada en `Sidebar`/`TopBar`/`App.tsx` y `ui-store`; capacidad de facturación.
- Sin sesión: formulario de apertura (base inicial, notas).
- Con sesión: tarjetas de base, efectivo esperado, cobrado por método, gastos/retiros,
  número de recibos; tabla de movimientos; acciones **Gasto**, **Retiro**,
  **Cerrar caja**.
- `CloseCashDialog`: denominaciones (100.000 … 100), cantidad, subtotal; total contado,
  esperado y diferencia con color; notas; confirmación e impresión del cierre.
- Historial de sesiones cerradas con PDF.

### 5.6 Panel y reportes

- Dashboard: “Cobrado hoy”, “Saldo pendiente”, “Caja abierta/cerrada” y diferencia del
  último cierre.
- Reportes: recibo de pago y cierre de caja.
- Los reportes de ingresos pasan a distinguir **facturado** (por fecha de emisión, sin
  anuladas) de **cobrado** (por fecha de recepción). El gráfico por categoría se mantiene
  como facturado; se agrega el cobrado real.

---

## 6. Fases y estimación

| Fase | Entregable | Estimación |
| --- | --- | --- |
| **A. Esquema y recepciones** | Migración, backfill, modelos, `register_payment`, saldos, tests | 4–5 días |
| **B. Cobros en UI** | `PaymentDialog`, detalle, listado con saldos, checkout, recibos | 4–5 días |
| **C. Caja** | Apertura, movimientos, arqueo, cierre, PDF, vista Caja | 4–5 días |
| **D. Cierre** | Dashboard, reportes, anulaciones, E2E y auditoría | 2–3 días |

Total MVP: **~4 semanas**.

---

## 7. Criterios de aceptación

### Abonos y mixtos

- [ ] Factura de 100.000 con abono de 40.000 queda `ABONADA`, saldo 60.000, y la factura
      conserva `STATUS='EMITIDA'`.
- [ ] Un segundo pago de 60.000 la deja `PAGADA` y saldo 0.
- [ ] Un pago de 100.000 repartido en 60.000 efectivo + 40.000 transferencia genera dos
      líneas y un solo recibo.
- [ ] Un pago puede aplicarse a dos facturas del mismo tutor y no a facturas de otro.
- [ ] Se rechaza un pago cuya suma de líneas no coincide con las asignaciones.
- [ ] Se rechaza una asignación mayor al saldo.
- [ ] Anular un pago devuelve el saldo y revierte la factura a `ABONADA`/`PENDIENTE`.
- [ ] Una factura con pagos activos no se puede anular.

### Caja

- [ ] No se puede abrir una segunda caja mientras haya una `ABIERTA`.
- [ ] Un pago en efectivo aumenta el efectivo esperado; una transferencia no.
- [ ] Un gasto de 20.000 reduce el esperado en 20.000.
- [ ] El cierre guarda base, esperado, contado y diferencia, y bloquea nuevos pagos.
- [ ] El PDF de cierre muestra totales por método y la diferencia.
- [ ] Un pago en efectivo de una sesión cerrada no puede registrarse.

### Legado y reportes

- [ ] Las facturas `PAGADA` previas aparecen con su pago migrado y saldo 0.
- [ ] El listado y el dashboard muestran cobrado y pendiente correctamente.
- [ ] El recibo imprime métodos mixtos y facturas aplicadas.
- [ ] Recepción/auxiliar cobran y cierran; solo ADMIN hace ajustes post-cierre.

---

## 8. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Saldo derivado; pagos múltiples; mixto con suma exacta; sobrepago; multi-factura; mismo tutor; anulación; factura anulada; caja única; esperado con gastos/retiros; cierre y bloqueo; backfill; concurrencia de dos pagos; RBAC. |
| Frontend (Vitest) | Suma de líneas vs asignaciones; diferencia en vivo; formato de saldo/estado; filtros por estado de cobro; cálculo de arqueo por denominación. |
| E2E (Playwright) | Abrir caja → cobrar mixto → abono parcial → segundo pago → recibo → gasto → cierre con diferencia → PDF; intento de cobro con caja cerrada. |
| Migración | BD en 0035 → 0036 con facturas pagadas, pendientes y anuladas; verificar recepciones migradas y totales. |

---

## 9. Riesgos y decisiones

| Riesgo | Mitigación |
| --- | --- |
| Doble cobro por concurrencia | Transacción + bloqueo de factura / compare-and-set del saldo. |
| Caja abierta duplicada en varias PCs | Índice único sobre `OPEN_KEY`; la segunda apertura falla con mensaje claro. |
| Diferencia de caja por redondeo | Tolerancia de 0.01 y comparación explícita; nunca autoajustar sin motivo. |
| Anulación después del cierre | Solo ADMIN, con movimiento de ajuste en la caja actual y auditoría. |
| Histórico sin caja | Backfill de pagos con `CASH_SESSION_ID NULL`; no se reconstruyen arqueos. |
| Métodos personalizados | Catálogo `PAYMENT_METHODS` editable; `OTRO` como respaldo del legado. |
| Cambio de efectivo | No se modela; se registra el importe aplicado, el cambio se entrega fuera del sistema. |
| Reportes de ingresos | Separar facturado (emisión) de cobrado (recepción); evitar leer `STATUS='PAGADA'` como sinónimo de ingreso del mes. |
| Facturas antiguas “pagadas” sin método | Se migran como `OTRO` si el texto no existe en el catálogo. |

### Decisiones abiertas

1. ¿Una sola caja por instalación o una por usuario/turno? (Recomendado MVP: una).
2. ¿Permitir pagos a cuenta sin factura? (Recomendado: no en MVP).
3. ¿Quién puede anular un pago con caja cerrada? (Recomendado: solo ADMIN).
4. ¿Se imprimen recibos por cada abono o solo el comprobante final? (Recomendado: cada recepción).
5. ¿El cierre se hace por día calendario o por turno? (Recomendado: por turno/sesión).

---

## 10. Orden de implementación (primer PR)

1. `0036_payments_cash.sql` y backfill; pruebas de migración.
2. Modelos y repositorios de pagos; `register_payment` + saldos + tests.
3. Comandos y bindings; `PaymentDialog` y detalle de factura con abonos.
4. Listado con saldo/estado de cobro y cobro desde checkout.
5. Caja: apertura, movimientos, cierre, PDF y vista.
6. Dashboard/reportes y E2E en el PR de cierre.
