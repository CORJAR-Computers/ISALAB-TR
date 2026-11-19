# Plan de implementación — Cartera y cobranza

**Estado:** propuesta (no implementada)
**Base:** schema `0040` · pagos, abonos y saldos ya implementados en `0036` (`PAYMENTS`, `PAYMENT_LINES`, `PAYMENT_ALLOCATIONS`, `PAYMENT_METHODS`, `CASH_SESSIONS`), saldo y estado de cobro derivados en SQL, `list_invoices` con `paidAmount`/`balance`/`paymentStatus`, `PaymentDialog` con pago mixto, `INVOICES.STATUS` sin fecha de vencimiento, cobro exige caja abierta.
**Objetivo:** convertir el saldo por factura en una **cartera gestionable**: antigüedad de saldos por tutor, acuerdos de pago con cuotas y seguimiento de cobranza (contactos, promesas y escalamiento), sin pasarela ni cobro automático.

> **Numeración reservada:** `0041_collections.sql`. Orden acordado: abonos/caja
> (`0036`), receta (`0037`–`0038`), recordatorios (`0039`), lotes (`0040`) y cartera
> (`0041`). Si cartera se implementa antes, se renumera antes de crear la migración.
> **No depende** de `0039`: los recordatorios clínicos son por paciente y esta iniciativa
> es por tutor/factura; se comparten canales y helpers, no las tablas.

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Vencimiento de factura | Fecha de vencimiento por factura (contado o plazo configurable) |
| Antigüedad | Cubetas 0–30 / 31–60 / 61–90 / +90 días por tutor y por factura |
| Cartera por tutor | Saldo total, vencido, próximo a vencer, última gestión, estado de cuenta |
| Acuerdos | Plan de pagos sobre una o varias facturas del mismo tutor, con cuotas |
| Cuotas | Vencimiento, importe, estado derivado (pendiente/parcial/pagada/vencida) |
| Cobros | El cobro existente se enlaza a la cuota; saldo y recibo no cambian |
| Recordatorios | Bandeja de cobranza por factura/cuota con escalamiento por días de atraso |
| Contactos | Llamada, WhatsApp (`wa.me`) o email (SMTP) con resultado y promesa de pago |
| Promesas | Fecha prometida, alerta cuando se incumple |
| Estado de cuenta | Documento por tutor con facturas, abonos, cuotas y saldo |
| Reportes | Cartera por antigüedad, recuperación del período, efectividad de cobranza |
| Auditoría | `PAYMENT_PLAN_CREATED/RESCHEDULED/CANCELLED`, `COLLECTION_CONTACT`, `DUE_DATE_CHANGED` |

### 1.2 No entra (MVP)

- Intereses, recargos por mora o indexación (requiere decisión contable y legal).
- Castigo de cartera / provisión de incobrables y nota crédito.
- Pasarela de pago, débito automático, links de pago o conciliación bancaria.
- WhatsApp Business API, SMS o llamadas automáticas (igual que `0039`: enlace manual).
- Portal del tutor o envío masivo de campañas.
- Acuerdos con entidades externas (aseguradoras, convenios corporativos).
- Facturación electrónica / DIAN y cobro jurídico automatizado.

### 1.3 Principios

1. **La cartera no es una tabla nueva de deudas.** Es el saldo ya derivado en `0036`:
   factura no anulada con `balance > 0.01`. No se duplica el saldo en ninguna parte.
2. **Un acuerdo no crea dinero.** Cubre facturas existentes y reparte su saldo en cuotas;
   el pago sigue siendo `register_payment` con una asignación por factura.
3. **El estado se deriva.** Cuota, acuerdo y recordatorio calculan su estado con pagos,
   fechas y contactos; solo se persiste lo que no se puede deducir (cancelado, retirado).
4. **Cobranza con registro, no con promesas de entrega.** Cada intento guarda canal,
   resultado, usuario y fecha; el envío de email reporta `SENT`/`FAILED`.
5. **Una sola fuente de cálculo de saldo.** La vista `V_INVOICE_BALANCES` centraliza
   `PAID`, `BALANCE` y `PAYMENT_STATUS`; el listado, la cartera y los acuerdos la usan.
6. **Nada se cobra fuera de caja.** Cobrar una cuota exige caja abierta (regla de `0036`);
   consultar cartera y contactar no la exige.
7. **Compatibilidad:** facturas históricas sin vencimiento usan su fecha de emisión como
   vencimiento; no se recalculan pagos ni estados.

---

## 2. Modelo de datos — migración `0041_collections.sql`

### 2.1 Vencimiento de factura

```sql
ALTER TABLE INVOICES ADD DUE_AT D_DATE;
CREATE INDEX IX_INVOICES_DUE ON INVOICES (DUE_AT, STATUS);

-- Backfill: las facturas existentes vencen el día en que se emitieron (contado).
UPDATE INVOICES
   SET DUE_AT = CAST(ISSUE_DATE AS DATE)
 WHERE DUE_AT IS NULL;
```

- `DUE_AT` se calcula al emitir: `fecha de emisión + CLINIC_SETTINGS 'billing.due_days'`
  (por defecto `0` = contado). Se puede cambiar la fecha de una factura con saldo desde la
  UI (solo ADMIN, con motivo y auditoría `DUE_DATE_CHANGED`).
- La antigüedad se mide contra `DUE_AT`, no contra la emisión: una factura a 30 días no está
  vencida al día 5.

### 2.2 Vista única de saldos

Centraliza lo que hoy vive en `repositories/invoices.rs` y `repositories/payments.rs`:

```sql
CREATE VIEW V_INVOICE_BALANCES AS
SELECT i.ID, i.INVOICE_NUMBER, i.OWNER_ID, i.PATIENT_ID, i.ISSUE_DATE, i.DUE_AT,
       CAST(i.TOTAL AS DOUBLE PRECISION) AS TOTAL, i.STATUS,
       CAST(COALESCE((
           SELECT SUM(a.AMOUNT)
           FROM PAYMENT_ALLOCATIONS a
           JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
           WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
       ), 0) AS DOUBLE PRECISION) AS PAID,
       CAST(CASE WHEN i.TOTAL - COALESCE((
           SELECT SUM(a.AMOUNT)
           FROM PAYMENT_ALLOCATIONS a
           JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
           WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
       ), 0) < 0 THEN 0 ELSE i.TOTAL - COALESCE((
           SELECT SUM(a.AMOUNT)
           FROM PAYMENT_ALLOCATIONS a
           JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
           WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'
       ), 0) END AS DOUBLE PRECISION) AS BALANCE,
       CASE
           WHEN i.STATUS = 'ANULADA' THEN 'ANULADA'
           WHEN i.TOTAL - COALESCE((
               SELECT SUM(a.AMOUNT) FROM PAYMENT_ALLOCATIONS a
               JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
               WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'), 0) <= 0.01 THEN 'PAGADA'
           WHEN COALESCE((
               SELECT SUM(a.AMOUNT) FROM PAYMENT_ALLOCATIONS a
               JOIN PAYMENTS p ON p.ID = a.PAYMENT_ID
               WHERE a.INVOICE_ID = i.ID AND p.STATUS = 'APLICADO'), 0) > 0.01 THEN 'ABONADA'
           ELSE 'PENDIENTE'
       END AS PAYMENT_STATUS
FROM INVOICES i;
```

Refactor incluido en esta misma fase (sin cambio funcional):

- `invoices::list` / `count_by_status` y `payments::invoice_balance` leen la vista.
- `Product`/pagos no cambian; solo se elimina la subconsulta repetida.
- Los tests existentes de `repositories::invoices` y `repositories::payments` deben pasar sin
  cambios de expectativas (es el criterio de que la vista es equivalente).

### 2.3 Acuerdos de pago

```sql
CREATE TABLE PAYMENT_PLANS (
    ID           D_PK NOT NULL PRIMARY KEY,
    CODE         D_CODE NOT NULL UNIQUE,        -- ACP-000001 (trigger)
    OWNER_ID     D_PK NOT NULL REFERENCES OWNERS (ID),
    -- ACTIVO | CANCELADO (el cumplimiento se deriva de las cuotas)
    STATUS       VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'ACTIVO' NOT NULL
                 CHECK (STATUS IN ('ACTIVO', 'CANCELADO')),
    STARTED_AT   DATE DEFAULT CURRENT_DATE NOT NULL,
    TOTAL_AMOUNT D_AMOUNT NOT NULL CHECK (TOTAL_AMOUNT > 0),
    -- SEMANAL | QUINCENAL | MENSUAL | PERSONALIZADO
    FREQUENCY    VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'MENSUAL' NOT NULL,
    NOTES        D_NOTES,
    CANCEL_REASON D_NOTES,
    CANCELLED_AT TIMESTAMP,
    CANCELLED_BY D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_BY   D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_PAYMENT_PLANS_ID;
CREATE INDEX IX_PAYMENT_PLANS_OWNER ON PAYMENT_PLANS (OWNER_ID, STATUS);

-- Facturas cubiertas. Una factura solo puede estar en un acuerdo ACTIVO a la vez.
CREATE TABLE PAYMENT_PLAN_INVOICES (
    ID            D_PK NOT NULL PRIMARY KEY,
    PLAN_ID       D_PK NOT NULL REFERENCES PAYMENT_PLANS (ID) ON DELETE CASCADE,
    INVOICE_ID    D_PK NOT NULL REFERENCES INVOICES (ID),
    -- Saldo de la factura al firmar el acuerdo (snapshot informativo).
    OPENING_BALANCE D_AMOUNT NOT NULL,
    UNIQUE (PLAN_ID, INVOICE_ID)
);
CREATE GENERATOR GEN_PAYMENT_PLAN_INVOICES_ID;
CREATE INDEX IX_PLAN_INVOICES_INVOICE ON PAYMENT_PLAN_INVOICES (INVOICE_ID);

CREATE TABLE PAYMENT_PLAN_INSTALLMENTS (
    ID          D_PK NOT NULL PRIMARY KEY,
    PLAN_ID     D_PK NOT NULL REFERENCES PAYMENT_PLANS (ID) ON DELETE CASCADE,
    SEQ         INTEGER NOT NULL,
    DUE_AT      D_DATE NOT NULL,
    AMOUNT      D_AMOUNT NOT NULL CHECK (AMOUNT > 0),
    -- ACTIVA | REESTRUCTURADA (reemplazada al reprogramar)
    STATUS      VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'ACTIVA' NOT NULL
                CHECK (STATUS IN ('ACTIVA', 'REESTRUCTURADA')),
    NOTES       D_NOTES,
    UNIQUE (PLAN_ID, SEQ)
);
CREATE GENERATOR GEN_PLAN_INSTALLMENTS_ID;
CREATE INDEX IX_PLAN_INSTALLMENTS_DUE ON PAYMENT_PLAN_INSTALLMENTS (DUE_AT, STATUS);

-- Vínculo del abono con la cuota que salda. Un abono puede cubrir una cuota parcial.
ALTER TABLE PAYMENT_ALLOCATIONS ADD PLAN_INSTALLMENT_ID D_PK
    REFERENCES PAYMENT_PLAN_INSTALLMENTS (ID) ON DELETE SET NULL;
CREATE INDEX IX_PAYMENT_ALLOCATIONS_INSTALLMENT ON PAYMENT_ALLOCATIONS (PLAN_INSTALLMENT_ID);
```

Índice único parcial "una factura en un solo acuerdo activo": Firebird no tiene índices
filtrados. Se resuelve con una columna `ACTIVE_SLOT` en `PAYMENT_PLAN_INVOICES` mantenida por
trigger (`1` si el plan está ACTIVO, `NULL` si está cancelado) e índice único sobre
`(INVOICE_ID, ACTIVE_SLOT)` — el mismo patrón ya usado en `CASH_SESSIONS.OPEN_SLOT`:

```sql
ALTER TABLE PAYMENT_PLAN_INVOICES ADD ACTIVE_SLOT INTEGER;
CREATE UNIQUE INDEX UX_PLAN_INVOICES_ACTIVE ON PAYMENT_PLAN_INVOICES (INVOICE_ID, ACTIVE_SLOT);
```

El trigger `BIU_PAYMENT_PLAN_INVOICES` lo llena con `(SELECT STATUS FROM PAYMENT_PLANS …)` y
`BIU_PAYMENT_PLANS` limpia los slots al cancelar.

### 2.4 Recordatorios de cobro

```sql
CREATE TABLE COLLECTION_REMINDERS (
    ID            D_PK NOT NULL PRIMARY KEY,
    OWNER_ID      D_PK NOT NULL REFERENCES OWNERS (ID),
    INVOICE_ID    D_PK REFERENCES INVOICES (ID) ON DELETE CASCADE,
    INSTALLMENT_ID D_PK REFERENCES PAYMENT_PLAN_INSTALLMENTS (ID) ON DELETE CASCADE,
    -- FACTURA | CUOTA
    KIND          VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'FACTURA' NOT NULL,
    -- 1 = aviso previo, 2 = vencida, 3 = atraso, 4 = escalamiento
    STAGE         INTEGER DEFAULT 1 NOT NULL,
    DUE_AT        D_DATE NOT NULL,
    -- PENDIENTE | CONTACTADO | PROMETIDO | CUMPLIDO | DESCARTADO
    STATUS        VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'PENDIENTE' NOT NULL,
    PROMISE_AT    D_DATE,
    TITLE         D_NAME NOT NULL,
    DETAIL        D_NOTES,
    -- INV:{invoice}:{stage} · CTA:{installment}:{stage}
    DEDUPE_KEY    VARCHAR(80) CHARACTER SET ASCII NOT NULL UNIQUE,
    LAST_CONTACT_AT TIMESTAMP,
    CONTACT_ATTEMPTS INTEGER DEFAULT 0 NOT NULL,
    CLOSED_AT     TIMESTAMP,
    CLOSED_BY     D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    NOTES         D_NOTES,
    CREATED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX IX_COLLECTION_REMINDERS_STATUS ON COLLECTION_REMINDERS (STATUS, DUE_AT);
CREATE INDEX IX_COLLECTION_REMINDERS_OWNER ON COLLECTION_REMINDERS (OWNER_ID);
CREATE INDEX IX_COLLECTION_REMINDERS_INVOICE ON COLLECTION_REMINDERS (INVOICE_ID);

CREATE TABLE COLLECTION_CONTACTS (
    ID            D_PK NOT NULL PRIMARY KEY,
    REMINDER_ID   D_PK NOT NULL REFERENCES COLLECTION_REMINDERS (ID) ON DELETE CASCADE,
    -- LLAMADA | WHATSAPP | EMAIL | PRESENCIAL
    CHANNEL       VARCHAR(16) CHARACTER SET UTF8 NOT NULL,
    -- SIN_RESPUESTA | CONTACTADO | PROMETIO | PAGO | RECHAZO | NUMERO_INVALIDO
    OUTCOME       VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    RECIPIENT     VARCHAR(120) CHARACTER SET UTF8,
    PROMISE_AT    D_DATE,
    -- SENT | FAILED | NOT_APPLICABLE
    DELIVERY      VARCHAR(16) CHARACTER SET UTF8,
    TEMPLATE_CODE VARCHAR(30) CHARACTER SET UTF8,
    NOTES         D_NOTES,
    CREATED_BY    D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX IX_COLLECTION_CONTACTS_REMINDER ON COLLECTION_CONTACTS (REMINDER_ID, CREATED_AT);

CREATE TABLE COLLECTION_TEMPLATES (
    ID      D_PK NOT NULL PRIMARY KEY,
    CODE    D_CODE NOT NULL,                 -- COBRO_PREVIO, COBRO_VENCIDO, COBRO_ACUERDO
    CHANNEL VARCHAR(12) CHARACTER SET UTF8 NOT NULL, -- EMAIL | WHATSAPP
    SUBJECT VARCHAR(160) CHARACTER SET UTF8,
    BODY    D_TEXT NOT NULL,
    ACTIVE  BOOLEAN DEFAULT TRUE NOT NULL,
    UNIQUE (CODE, CHANNEL)
);
```

Placeholders: `{tutor}`, `{factura}`, `{vencimiento}`, `{saldo}`, `{dias}`, `{cuota}`,
`{fecha_promesa}`, `{clinica}`, `{telefono}`. Se renderizan en Rust para email y mediante
un comando para WhatsApp, igual que en `0039`.

Seed incluido en `0041` (textos sobrios, editables): `COBRO_PREVIO`, `COBRO_VENCIDO`,
`COBRO_ACUERDO` para EMAIL y WHATSAPP. Sin importes inventados: el render toma el saldo real.

### 2.5 Ajustes de clínica

```sql
INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT) VALUES ('billing.due_days', '0');
INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT) VALUES ('collections.stage2_days', '1');
INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT) VALUES ('collections.stage3_days', '15');
INSERT INTO CLINIC_SETTINGS (KEY_NAME, VALUE_TEXT) VALUES ('collections.stage4_days', '30');
```

Se leen con el mismo helper de `invoice.tax_rate` y se editan en Ajustes → Facturación.

---

## 3. Reglas de negocio

### 3.1 Qué es cartera

```text
cartera = facturas con STATUS <> 'ANULADA' y BALANCE > 0.01
```

- Una factura `PAGADA` con saldo 0 no aparece; una `EMITIDA` con abonos sí, por el saldo.
- `días_vencida = MAX(0, hoy - DUE_AT)`: 0 significa "por vencer o vence hoy".
- Cubetas fijas por días vencidos: **Por vencer (0)**, **1–30**, **31–60**, **61–90**, **+90**.
  La UI puede mostrar "0–30" agrupando por vencer y 1–30 para coincidir con la práctica
  contable; el backend expone las dos cifras (`overdue_days` exacto, `bucket`).
- Facturas en acuerdo activo siguen contando en el total de cartera, marcadas `EN_ACUERDO`,
  y se excluyen de los recordatorios genéricos de factura (solo generan recordatorios de cuota).

### 3.2 Cambio de vencimiento

```text
set_invoice_due_date(invoice_id, due_at, reason):
  exigir ADMIN y motivo
  exigir factura con saldo y no anulada
  rechazar due_at anterior a la emisión
  si la factura está en un acuerdo activo → rechazar (se reprograma el acuerdo)
  actualizar DUE_AT
  auditar DUE_DATE_CHANGED (antes/después/motivo)
```

No se permite "mover" el vencimiento repetidamente sin rastro: cada cambio queda auditado y
la antigüedad se recalcula sola.

### 3.3 Acuerdo de pago

```text
create_payment_plan(input):
  validar mismo OWNER_ID en todas las facturas
  cada factura: no anulada, saldo > 0.01, sin acuerdo activo
  suma de saldos = total del acuerdo (±0.01)
  cuotas: suma exacta del total (±0.01), fechas crecientes, importe > 0
  transacción:
    insertar PAYMENT_PLANS + PAYMENT_PLAN_INVOICES (OPENING_BALANCE)
    insertar PAYMENT_PLAN_INSTALLMENTS
  auditar PAYMENT_PLAN_CREATED
```

- **Fechas**: `PERSONALIZADO` recibe las fechas explícitas; el resto las calcula desde
  `STARTED_AT` (semanal +7 d, quincenal +15 d, mensual +1 mes calendario).
- **Sin sobre-acuerdo**: no se incluyen facturas ya cubiertas; para renegociar se cancela el
  acuerdo anterior (o se reprograma) y las facturas quedan libres.
- **Reprogramar** (`reschedule_payment_plan`): marca las cuotas `ACTIVA` como
  `REESTRUCTURADA`, crea las nuevas y deja intactos los pagos ya aplicados. El nuevo total
  es el saldo actual de las facturas cubiertas, no el original. Auditado.
- **Cancelar** (`cancel_payment_plan`): libera los slots de factura, deja las cuotas como
  histórico y marca `CANCELADO` con motivo. Las facturas vuelven a la cobranza normal.
- No se permite añadir facturas nuevas a un acuerdo existente en el MVP: se cancela y se
  crea otro (evita recalcular cuotas ya contactadas).

### 3.4 Estado de una cuota (derivado)

```text
pagado_cuota   = Σ PAYMENT_ALLOCATIONS.AMOUNT con PLAN_INSTALLMENT_ID = cuota
                 y PAYMENT.STATUS = 'APLICADO'
estado_cuota   = REESTRUCTURADA                      si STATUS = 'REESTRUCTURADA'
                 PAGADA                              si pagado >= AMOUNT - 0.01
                 PARCIAL                             si 0.01 < pagado < AMOUNT - 0.01
                 VENCIDA                             si PENDIENTE y DUE_AT < hoy
                 PENDIENTE                           en otro caso
estado_acuerdo = CANCELADO                            si STATUS = 'CANCELADO'
                 CUMPLIDO                             si todas las cuotas ACTIVA están PAGADAS
                 EN_MORA                              si alguna cuota ACTIVA está VENCIDA
                 ACTIVO                               en otro caso
```

`PAYMENT_PLANS.STATUS` solo guarda `ACTIVO`/`CANCELADO`; lo demás se calcula. Así un abono
registrado desde cualquier pantalla actualiza el acuerdo sin escribir en dos sitios.

### 3.5 Cobro de una cuota

Extiende `register_payment` (misma transacción y validaciones de `0036`):

```text
RegisterPaymentInput += plan_installment_id? (por asignación)

por cada asignación con plan_installment_id:
  validar que la cuota existe, es ACTIVA y pertenece a un plan ACTIVO
  validar que la factura asignada está cubierta por ese plan
  validar que la asignación no excede el saldo de la cuota (±0.01)
  guardar PLAN_INSTALLMENT_ID en PAYMENT_ALLOCATIONS
```

- Si la factura tiene un acuerdo activo y la asignación **no** trae cuota, se rechaza con un
  mensaje que indica las cuotas pendientes. Sin esto el acuerdo sería decorativo.
- Un pago puede cubrir **varias cuotas** del mismo acuerdo (varias asignaciones, cada una con
  su cuota) y mezclar métodos como ya hace el `PaymentDialog`.
- El pago sigue exigiendo caja abierta; el recibo y las líneas no cambian.
- Pagar la última cuota deja las facturas en `PAGADA` por el flujo existente; el acuerdo pasa
  a `CUMPLIDO` sin escribir nada.

### 3.6 Promesas de pago

- Se registran desde un contacto con `OUTCOME='PROMETIO'` y `PROMISE_AT`.
- El recordatorio pasa a `PROMETIDO` y muestra la fecha; si la fecha pasa sin pago, el
  refresh lo devuelve a `CONTACTADO` con una alerta "promesa incumplida" y cuenta el intento.
- Promesa vencida no genera una cuota ni altera el acuerdo: es información de gestión.

### 3.7 Generación de recordatorios (`refresh_collection_reminders`)

Idempotente por `DEDUPE_KEY`; se ejecuta al abrir Cartera, al iniciar la app y tras registrar
un pago (no hay daemon):

```text
para cada factura en cartera sin acuerdo activo:
    dias = hoy - DUE_AT
    stage = 1 si dias >= -3   (aviso previo)
            2 si dias >= stage2_days
            3 si dias >= stage3_days
            4 si dias >= stage4_days
    upsert key = INV:{invoice_id}:{stage}   due = DUE_AT
           title = "Cobro FAC-… ({saldo})"

para cada cuota ACTIVA de acuerdo ACTIVO:
    dias = hoy - DUE_AT
    stage = 2 si dias >= 0, 3 si dias >= stage3_days, 4 si dias >= stage4_days
    upsert key = CTA:{installment_id}:{stage}
           title = "Cuota {seq} de {plan_code}"
```

Reglas:

- Un recordatorio `CUMPLIDO`/`DESCARTADO` no se reabre para la misma clave; un escalamiento
  crea una fila nueva con su propio `STAGE` (historial de gestión intacto).
- Si la factura/cuota se paga, los recordatorios abiertos se cierran como `CUMPLIDO` en el
  mismo refresh.
- Pacientes/tutores sin teléfono ni email: el recordatorio existe igual; la UI indica "sin
  contacto registrado" y permite registrar una llamada presencial.
- `refresh` devuelve `{creados, cerrados, actualizados}` para el toast.

### 3.8 Contacto y plantillas

- Canales: **llamada** y **presencial** (solo registro), **WhatsApp** (enlace `wa.me` con el
  mensaje renderizado, sin API) y **email** (SMTP existente, con `SENT`/`FAILED`).
- Antes de enviar, la UI muestra el mensaje final editable; lo enviado se guarda en
  `COLLECTION_CONTACTS.NOTES` (texto renderizado) junto a la plantilla usada.
- El contacto actualiza `LAST_CONTACT_AT` y `CONTACT_ATTEMPTS`, y el estado del recordatorio
  (`CONTACTADO` / `PROMETIDO`).
- Nunca se envía nada automáticamente ni en lote en el MVP: siempre hay confirmación humana.

### 3.9 Redondeo

Mismas reglas de `0036`: `D_AMOUNT` con 2 decimales y tolerancia `0.01` para comparar saldos,
cuotas y totales del acuerdo.

---

## 4. Backend

### 4.1 Archivos

```text
src-tauri/src/models/collections.rs        AgingRow, AgingSummary, OwnerStatement, Plan, inputs
src-tauri/src/repositories/collections.rs  Aging, estado de cuenta, acuerdos y cuotas
src-tauri/src/repositories/collection_reminders.rs  Refresh, contactos, plantillas
src-tauri/src/commands/collections.rs
src-tauri/src/commands/collection_reminders.rs
src-tauri/src/repositories/payments.rs     plan_installment_id en asignaciones
src-tauri/src/pdf_templates/collections.rs Estado de cuenta y cartera por antigüedad
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs`, `lib.rs`
(`collect_commands!` + `.typ::<>`), `pdf_templates/mod.rs`; actualizar `src/bindings.ts` y
`npm run check:bindings`. Migración en `db/migrations.rs`. `test_helpers.rs` limpia
`COLLECTION_CONTACTS`, `COLLECTION_REMINDERS`, `PAYMENT_PLAN_INSTALLMENTS`,
`PAYMENT_PLAN_INVOICES`, `PAYMENT_PLANS` antes de `PAYMENTS`/`INVOICES`.

Capacidades: lectura de cartera con `require_session`; acuerdos, cobros y contactos con
`require_bill`; cambiar vencimientos, cancelar acuerdos y ver reportes globales con
`require_admin` (el veterinario puede crear acuerdos si tiene facturación, según `auth.rs`).

### 4.2 Comandos

| Comando | Capacidad | Descripción |
| --- | --- | --- |
| `get_receivables_summary(as_of?)` | sesión | Totales por cubeta y por estado (vencido, por vencer, en acuerdo) |
| `list_receivables(aging?, search?, min_balance?)` | sesión | Cartera por tutor con cubetas |
| `list_owner_receivables(owner_id)` | sesión | Facturas con saldo del tutor |
| `get_owner_statement(owner_id, from?, to?)` | sesión | Estado de cuenta (facturas, abonos, cuotas, saldo) |
| `set_invoice_due_date(invoice_id, due_at, reason)` | admin | Cambia vencimiento con auditoría |
| `create_payment_plan(input)` | facturación | Acuerdo + cuotas |
| `list_payment_plans(status?, owner_id?)` | sesión | Acuerdos con estado derivado |
| `get_payment_plan(id)` | sesión | Detalle con cuotas y pagos |
| `reschedule_payment_plan(id, input)` | facturación | Reestructura cuotas pendientes |
| `cancel_payment_plan(id, reason)` | admin | Libera facturas y conserva historial |
| `refresh_collection_reminders()` | facturación | Genera/cierra recordatorios (idempotente) |
| `list_collection_reminders(status?, kind?, overdue_only?)` | sesión | Bandeja de cobranza |
| `get_reminder_message(reminder_id, channel)` | sesión | Render de plantilla para WhatsApp/email |
| `log_collection_contact(input)` | facturación | Registra llamada/WhatsApp/email y resultado |
| `send_collection_email(input)` | facturación | Email SMTP + registro `SENT`/`FAILED` |
| `list_collection_contacts(reminder_id)` | sesión | Historial de gestión |
| `list_collection_templates()` / `save_collection_template(input)` | admin | Plantillas de cobro |
| `generate_owner_statement_pdf(owner_id)` | sesión | Estado de cuenta imprimible |
| `generate_aging_report_pdf(as_of?)` | admin | Cartera por antigüedad |
| `export_receivables_csv()` | sesión | Exportación de cartera (patrón existente) |

`CreatePaymentPlanInput`:

```text
owner_id, started_at?, frequency, notes?,
invoices: [{ invoice_id }],
installments: [{ due_at, amount }]        // explícitas; la UI las previsualiza
```

`ReschedulePlanInput`: `started_at?`, `frequency`, `installments[]`, `reason`.

`LogCollectionContactInput`:

```text
reminder_id, channel, outcome, recipient?, promise_at?, template_code?, notes?,
message_text?
```

`RegisterPaymentInput` gana `allocations[].plan_installment_id?` (retrocompatible: `null`
para cobros normales).

### 4.3 Consultas de antigüedad

```sql
SELECT o.ID AS OWNER_ID, o.FULL_NAME, o.PHONE, o.EMAIL,
       COUNT(b.ID) AS INVOICES,
       SUM(b.BALANCE) AS TOTAL_DUE,
       SUM(CASE WHEN b.DAYS_OVERDUE <= 0 THEN b.BALANCE ELSE 0 END) AS NOT_DUE,
       SUM(CASE WHEN b.DAYS_OVERDUE BETWEEN 1 AND 30 THEN b.BALANCE ELSE 0 END) AS B1_30,
       SUM(CASE WHEN b.DAYS_OVERDUE BETWEEN 31 AND 60 THEN b.BALANCE ELSE 0 END) AS B31_60,
       SUM(CASE WHEN b.DAYS_OVERDUE BETWEEN 61 AND 90 THEN b.BALANCE ELSE 0 END) AS B61_90,
       SUM(CASE WHEN b.DAYS_OVERDUE > 90 THEN b.BALANCE ELSE 0 END) AS B90_PLUS
FROM (
    SELECT v.*, DATEDIFF(DAY, v.DUE_AT, CURRENT_DATE) AS DAYS_OVERDUE
    FROM V_INVOICE_BALANCES v
    WHERE v.PAYMENT_STATUS IN ('PENDIENTE', 'ABONADA') AND v.BALANCE > 0.01
) b
JOIN OWNERS o ON o.ID = b.OWNER_ID
GROUP BY o.ID, o.FULL_NAME, o.PHONE, o.EMAIL
HAVING SUM(b.BALANCE) > 0.01
ORDER BY TOTAL_DUE DESC
```

> Se usa `DATEDIFF(DAY, …)` de Firebird en lugar de restar fechas: la resta directa de dos
> `DATE` no es portable entre versiones. `DAYS_OVERDUE <= 0` incluye "vence hoy" y futuras.
> Verificar el plan de ejecución en la prueba de migración, no asumirlo.

- Filtro "en acuerdo": `EXISTS (SELECT 1 FROM PAYMENT_PLAN_INVOICES pi JOIN PAYMENT_PLANS pp
  ON pp.ID = pi.PLAN_ID WHERE pi.INVOICE_ID = v.ID AND pp.STATUS = 'ACTIVO')`.
- El listado de facturas por tutor reutiliza `V_INVOICE_BALANCES` y expone `overdue_days`
  con el mismo `DATEDIFF(DAY, DUE_AT, CURRENT_DATE)`, calculado una sola vez en SQL.
- Estado de cuenta: facturas con saldo, pagos aplicados (con cuota si existe) y cuotas
  pendientes; el saldo final debe cuadrar con `SUM(v.BALANCE)` (se valida en test).

### 4.4 Concurrencia

- Crear pago y actualizar estados ya es una transacción (`0036`); el vínculo a cuota se
  inserta en la misma.
- Dos PCs cobrando la misma cuota: el bloqueo `WITH LOCK` de la factura en `register` se
  complementa con validación dentro de la transacción del saldo de la cuota; el índice
  `UX_PLAN_INVOICES_ACTIVE` impide dos acuerdos sobre la misma factura.
- Reprogramar bloquea el acuerdo (`WITH LOCK`) y verifica que siga `ACTIVO`.
- `refresh` no bloquea pagos: inserta/actualiza recordatorios; si un pago entra a la vez, el
  siguiente refresh cierra el recordatorio.

---

## 5. Cambios de UI

### 5.1 Nueva vista **Cartera** (`collections`)

Entrada en `Sidebar`/`TopBar`/`App.tsx` y `ui-store`, visible con capacidad de facturación.

- **Tarjetas**: total cartera, vencido, por vencer 7 días, en acuerdo, promesas vigentes.
- **Tabs**: Por cobrar · Acuerdos · Recordatorios · Promesas.
- **Por cobrar**: tabla por tutor (tutor, teléfono, nº facturas, total, cubetas como barras,
  última gestión, estado). Acciones: **Estado de cuenta**, **Acuerdo**, **Cobrar**,
  **Recordar**.
- Filtros: cubeta, "solo vencidas", "en acuerdo", búsqueda por tutor/documento.
- Exportar CSV y PDF de antigüedad.

### 5.2 Estado de cuenta (`OwnerStatementDialog`)

- Cabecera con tutor, contacto, saldo total y cubetas.
- Facturas con emisión, vencimiento, total, abonado y saldo; debajo, pagos del período.
- Acuerdos activos con sus cuotas.
- Botones: **Imprimir/PDF**, **Cobrar saldo**, **Crear acuerdo**.

### 5.3 Acuerdo de pago (`PaymentPlanDialog`)

- Selección de facturas del tutor con saldo (checkboxes; total seleccionado).
- Frecuencia y fecha de inicio; vista previa editable de cuotas con fecha e importe.
- Validación en vivo: la suma de cuotas debe igualar el saldo seleccionado (diferencia
  visible, botón deshabilitado si no cuadra).
- Notas y confirmación; toast con el código `ACP-…`.

### 5.4 Detalle del acuerdo (`PaymentPlanDialog` en modo detalle)

- Cuotas con estado, pagado y saldo; "Registrar pago" abre el `PaymentDialog` existente con
  la cuota precargada (un solo `step`, sin duplicar el cobro).
- Facturas cubiertas con su saldo de apertura.
- Acciones: **Reprogramar** (solo cuotas pendientes), **Cancelar acuerdo** (ADMIN, motivo).

### 5.5 Recordatorios de cobro

- Bandeja con filtros: vencidos, promesa incumplida, sin contacto, por canal.
- Cada fila: tutor, factura/cuota, saldo, días de atraso, etapa, último contacto, promesa.
- Acciones: **WhatsApp** (mensaje renderizado y editable), **Email** (asunto/cuerpo),
  **Registrar llamada** (resultado + promesa), **Ver historial**.
- Al enviar se registra el intento y se muestra el resultado del SMTP; nunca se afirma
  entrega en WhatsApp.
- Marcar `DESCARTADO` exige motivo (ej. "deuda en disputa").

### 5.6 Facturación y detalle de factura

- `InvoicesPage`: columna **Vence** y badge de atraso; filtro "Vencidas"; acción **Cobrar**
  ya existente.
- `InvoiceDetailDialog`: vencimiento (editable por ADMIN con motivo), días de atraso, badge
  `EN_ACUERDO` con enlace al plan; los abonos muestran la cuota cuando aplica.
- `PaymentDialog`: cuando la factura está en un acuerdo, muestra las cuotas pendientes y
  obliga a elegir una; el importe se precarga con el saldo de la cuota.

### 5.7 Panel y ajustes

- Dashboard: "Cartera total", "Vencido", "Promesas de hoy" (reemplaza/complementa
  `invoices_unpaid`).
- Ajustes → Facturación: días de plazo por defecto y umbrales de escalamiento.

---

## 6. Fases y estimación

| Fase | Entregable | Estimación |
| --- | --- | --- |
| **A. Vencimientos y vista de saldos** | `0041` (parte 1), `DUE_AT`, `V_INVOICE_BALANCES`, refactor de consultas, tests de equivalencia | 2–3 días |
| **B. Cartera y antigüedad** | Repositorio de aging, estado de cuenta, PDF/CSV, vista **Cartera** | 3–4 días |
| **C. Acuerdos y cuotas** | Planes, cuotas, reprogramación, cobro vinculado a cuota, UI de acuerdo | 4–5 días |
| **D. Cobranza** | Recordatorios, escalamiento, contactos, promesas, plantillas, panel y ajustes | 4–5 días |

Total MVP: **~3 semanas** (después de caja; independiente de receta, lotes y recordatorios
clínicos).

---

## 7. Criterios de aceptación

### Antigüedad

- [ ] Una factura de 100.000 con abono de 40.000 y vencimiento hace 45 días aparece con
      saldo 60.000 en la cubeta 31–60 y en el total del tutor.
- [ ] Una factura que vence hoy no cuenta como vencida (cubeta "por vencer").
- [ ] Cambiar el vencimiento exige motivo, queda auditado y mueve la factura de cubeta.
- [ ] El estado de cuenta cuadra con la suma de saldos de sus facturas.
- [ ] Una factura anulada nunca aparece en cartera.

### Acuerdos

- [ ] Se crea un acuerdo de 3 cuotas que suman exactamente el saldo de dos facturas del
      mismo tutor; el estado inicial es `ACTIVO` y las cuotas `PENDIENTE`.
- [ ] No se puede incluir una factura de otro tutor ni una ya cubierta por otro acuerdo
      activo.
- [ ] Cobrar una cuota exige elegirla; el abono queda enlazado y la cuota pasa a `PAGADA`.
- [ ] Un abono parcial deja la cuota `PARCIAL`; al completarla cambia a `PAGADA`.
- [ ] Al pagar todas las cuotas, el acuerdo aparece `CUMPLIDO` y las facturas `PAGADA`.
- [ ] Reprogramar conserva los pagos aplicados y reemplaza solo las cuotas pendientes.
- [ ] Cancelar el acuerdo libera las facturas para uno nuevo y deja historial.
- [ ] Un cobro sobre una factura en acuerdo sin indicar cuota es rechazado.

### Cobranza

- [ ] El refresh es idempotente: ejecutarlo dos veces no duplica recordatorios.
- [ ] Al pagar una factura, su recordatorio abierto se cierra como `CUMPLIDO`.
- [ ] Una factura con 16 días de atraso genera etapa 3 y no vuelve a crear la etapa 2.
- [ ] Registrar una promesa la muestra hasta la fecha; al incumplirse se alerta.
- [ ] El email registra `SENT` o `FAILED`; WhatsApp abre el enlace y registra el intento.
- [ ] Un tutor sin teléfono/email puede gestionarse con llamada o presencial.
- [ ] Recepción/auxiliar con facturación puede contactar y cobrar; solo ADMIN cambia
      vencimientos y cancela acuerdos.

---

## 8. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Equivalencia de `V_INVOICE_BALANCES` con el cálculo anterior; antigüedad por cubetas y por tutor; factura vence hoy; cambio de vencimiento auditado; creación de acuerdo con sumas exactas e inexactas; factura duplicada en acuerdo; mismo tutor; cobro de cuota parcial/total; cobro sin cuota rechazado; pago de varias cuotas; reprogramación; cancelación; estados derivados de cuota y acuerdo; refresh idempotente; cierre por pago; promesa vencida; RBAC. |
| Frontend (Vitest) | Cálculo de cubetas y días; suma de cuotas vs saldo seleccionado; estado de cuota/acuerdo; selección obligatoria de cuota; filtros de bandeja; render de plantillas. |
| E2E (Playwright) | Factura vencida → cartera → estado de cuenta → acuerdo 3 cuotas → cobrar cuota 1 → recordatorio de cuota 2 → promesa → pago final → acuerdo cumplido. |
| Migración | BD en `0040` → `0041`: `DUE_AT` retroactivo correcto, saldos intactos, ninguna factura en dos acuerdos, vistas creadas. |

---

## 9. Riesgos y decisiones

| Riesgo | Mitigación |
| --- | --- |
| Cartera distinta de la suma de pagos | Una sola vista de saldos y tests de equivalencia con el cálculo previo. |
| Acuerdo decorativo (pagos por fuera) | El cobro de una factura en acuerdo exige cuota; validación en backend, no solo en UI. |
| Doble acuerdo sobre la misma factura | `UX_PLAN_INVOICES_ACTIVE` con `ACTIVE_SLOT` mantenido por trigger. |
| Cuotas que no suman el saldo | Validación exacta con tolerancia 0.01 y vista previa en la UI. |
| Recordatorios duplicados | `DEDUPE_KEY` único por origen+etapa y refresh idempotente. |
| Escalamiento que borra historial | Una fila por etapa; las anteriores se conservan con sus contactos. |
| Datos de contacto obsoletos | La bandeja muestra teléfono/email y permite corregirlos desde el estado de cuenta (auditado). |
| Envío masivo accidental | Sin envío automático ni selección múltiple; confirmación por contacto. |
| Promesas usadas como excusa | Alerta de promesa incumplida y contador de intentos; no cambia el saldo ni el vencimiento. |
| Cambios de vencimiento para "maquillar" la cartera | Solo ADMIN, con motivo, auditado y con historial visible en el estado de cuenta. |
| Crecimiento de contactos | Índices por recordatorio y fecha; la bandeja filtra por estado y etapa. |

### Decisiones abiertas

1. ¿Plazo por defecto de factura? (Recomendado: 0 días contado, configurable.)
2. ¿La antigüedad se mide por vencimiento o por emisión? (Recomendado: vencimiento; emisión
   queda como dato secundario en el reporte.)
3. ¿Quién puede crear acuerdos? (Recomendado: cualquier rol con facturación; cancelar solo ADMIN.)
4. ¿Los acuerdos generan una factura nueva o cubren las existentes? (Recomendado: cubren las
   existentes; no se reemiten facturas.)
5. ¿Se permite un pago mayor a la cuota? (Recomendado: no en el MVP; se aplica a la cuota
   exacta o se reparte en varias cuotas del mismo acuerdo.)

---

## 10. Orden de implementación (primer PR)

1. `0041_collections.sql` (vencimiento, vista de saldos, tablas de acuerdo y cobranza),
   registro en `db/migrations.rs` y pruebas de migración.
2. Refactor de `invoices`/`payments` a `V_INVOICE_BALANCES`; tests existentes en verde.
3. Repositorio de antigüedad y estado de cuenta; comandos y bindings.
4. Vista **Cartera** con tarjetas, tabla por tutor y estado de cuenta imprimible.
5. Acuerdos: creación, cuotas, cobro vinculado y reprogramación.
6. Cobranza: refresh, bandeja, plantillas, contactos, promesas y alertas en panel.
