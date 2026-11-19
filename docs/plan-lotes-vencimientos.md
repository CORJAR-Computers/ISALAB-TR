# Plan de implementación — Lotes, vencimientos y trazabilidad de fármacos y vacunas

**Estado:** propuesta (no implementada)
**Base:** schema 0036 · `PRODUCTS`/`STOCK_MOVEMENTS` con stock derivado por producto y cantidad `INTEGER`, `VACCINES.LOT` como texto libre, `VACCINE_TYPES` sin vínculo a producto, PDF de carnet de vacunación, RBAC por capacidades.
**Depende de:** `docs/plan-receta-estructurada.md` (`0037`–`0038`) para `PRESCRIPTION_ITEMS.PRODUCT_ID`, `STOCK_MOVEMENTS.PRESCRIPTION_ITEM_ID` e `INVOICE_ITEMS.PRODUCT_ID`.
**Objetivo:** que cada unidad de un fármaco o vacuna que entra, se dispensa, se aplica o se factura tenga **lote y vencimiento**, con consumo FEFO, bloqueo de lotes vencidos y trazabilidad bidireccional (lote → pacientes, paciente → lote) para retiros y auditoría.

> **Numeración reservada:** `0040_product_lots.sql`. Orden acordado: abonos/caja
> (`0036`), receta (`0037`–`0038`), recordatorios (`0039`), lotes/vencimientos (`0040`) y
> cartera (`0041`). Con ese orden `0040` se aplica siempre después de `0037`–`0039`, así
> que no hay variante condicional: usa las columnas de receta sin preguntar si existen. Si
> lotes se implementa antes, **se renumera antes de crear la migración**.
> Sin seed de demo: los lotes son datos reales de la clínica.

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Lotes | Alta de lote por producto: número, vencimiento, proveedor, costo, cantidad inicial |
| Stock por lote | `STOCK_MOVEMENTS.LOT_ID`; el stock del producto sigue siendo la suma total |
| FEFO | Consumo por vencimiento más próximo; propuesta automática y ajuste manual |
| Vencimientos | Bandeja de lotes vencidos, por vencer (30/60/90 d) y agotados |
| Dispensación | La receta descuenta por lote; una línea puede repartirse entre varios lotes |
| Vacunas | Selector de lote del producto de la vacuna; aplica FEFO; descuento opcional de stock |
| Trazabilidad | Lote → movimientos, recetas, vacunas, pacientes y facturas; paciente → lotes recibidos |
| Retiro / cuarentena | Retiro de lote con motivo, bloqueo de dispensación y salida del stock restante |
| Alertas | Contador de vencimientos en inventario y panel; aviso al dispensar o vacunar |
| Auditoría | `LOT_RECEIVED`, `LOT_RETIRED`, `LOT_EXPIRED_DISPENSE_BLOCKED`, `VACCINE_LOT_LINKED` |

### 1.2 No entra (MVP)

- Compras y proveedores como entidad (se guarda proveedor y factura como texto; el
  módulo de compras es una iniciativa aparte).
- Fraccionamiento de frascos: el stock sigue siendo `INTEGER` por presentación completa
  (misma decisión que el plan de receta). El volumen administrado se guarda en la receta.
- Trazabilidad por unidad serializada (cada tableta o ampolleta con identificador propio).
- Cadena de frío, temperatura o registro sanitario por lote más allá de INVIMA/registro.
- Devolución al proveedor con nota crédito y costo promedio ponderado.
- Recalls automáticos desde INVIMA o correo; el retiro es manual y auditado.
- Vencimientos de insumos de laboratorio reactivos fuera de `PRODUCTS` (los controles QC
  ya tienen su propio `EXPIRES_AT`).

### 1.3 Principios

1. **El stock sigue siendo derivado.** No se crea una columna "stock por lote" que pueda
   desincronizarse: es `SUM(STOCK_MOVEMENTS.QUANTITY)` agrupado por `LOT_ID`.
2. **El estado del lote también se deriva.** Solo se persiste `ACTIVA` / `RETIRADA`
   (cuarentena); "vencido", "por vencer" y "agotado" se calculan con fecha y saldo.
3. **Nada vencido se administra.** Un lote vencido no puede dispensarse ni aplicarse en una
   vacuna; solo admite baja o devolución con motivo.
4. **Trazabilidad sin recetas nuevas.** El vínculo se guarda en los movimientos de stock,
   que ya son el libro mayor del inventario; no se duplica en tablas espejo.
5. **Compatibilidad total.** Los movimientos históricos sin lote siguen contando en el
   stock del producto y se muestran como "sin lote"; no se inventan lotes retroactivos.
6. **Los datos del lote no se reescriben.** Número y vencimiento se corrigen solo con
   auditoría y mientras el lote no tenga consumos; después se retira y se recibe otro.

---

## 2. Modelo de datos — migración `0040_product_lots.sql`

### 2.1 Lotes

```sql
CREATE TABLE PRODUCT_LOTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    PRODUCT_ID      D_PK NOT NULL REFERENCES PRODUCTS (ID) ON DELETE CASCADE,
    -- Número de lote impreso por el fabricante (no único global: se repite entre productos).
    LOT_NUMBER      VARCHAR(60) CHARACTER SET UTF8 NOT NULL,
    -- Vencimiento tal como lo imprime el fabricante. NULL solo para no medicamentos.
    EXPIRES_AT      D_DATE,
    RECEIVED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- Recepción: costo del lote (para valorar stock y comparar con el costo del producto).
    UNIT_COST       D_AMOUNT,
    INITIAL_QUANTITY INTEGER,
    -- Proveedor y soporte como texto; el módulo de compras los normalizará después.
    SUPPLIER_NAME   D_NAME,
    SUPPLIER_INVOICE VARCHAR(60) CHARACTER SET UTF8,
    -- Registro sanitario / INVIMA del lote, si aplica.
    SANITARY_REGISTRY VARCHAR(60) CHARACTER SET UTF8,
    -- ACTIVA | RETIRADA (cuarentena o retiro del mercado).
    STATUS          VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'ACTIVA' NOT NULL
                    CHECK (STATUS IN ('ACTIVA', 'RETIRADA')),
    RETIRED_AT      TIMESTAMP,
    RETIRED_BY      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    RETIRE_REASON   D_NOTES,
    NOTES           D_NOTES,
    CREATED_BY      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE (PRODUCT_ID, LOT_NUMBER)
);
CREATE GENERATOR GEN_PRODUCT_LOTS_ID;
CREATE INDEX IX_PRODUCT_LOTS_PRODUCT ON PRODUCT_LOTS (PRODUCT_ID, STATUS);
CREATE INDEX IX_PRODUCT_LOTS_EXPIRY ON PRODUCT_LOTS (EXPIRES_AT, STATUS);
```

Reglas de integridad:

- `EXPIRES_AT` obligatorio cuando el producto es `MEDICAMENTO` (validado en repositorio y,
  como red de seguridad, en el trigger `BI_PRODUCT_LOTS`).
- `UNIQUE (PRODUCT_ID, LOT_NUMBER)`: recibir dos veces el mismo lote es un error de
  digitación, no un caso válido; se corrige con un movimiento de ajuste.
- No se `DELETE` un lote con movimientos: se `RETIRA`. `ON DELETE CASCADE` solo aplica al
  borrado del producto (que ya no se borra en la práctica, se desactiva).

### 2.2 Movimientos por lote

```sql
ALTER TABLE STOCK_MOVEMENTS ADD LOT_ID D_PK
    REFERENCES PRODUCT_LOTS (ID) ON DELETE SET NULL;

-- Vínculo con la vacuna aplicada (trazabilidad inversa lote → paciente).
ALTER TABLE STOCK_MOVEMENTS ADD VACCINE_ID D_PK
    REFERENCES VACCINES (ID) ON DELETE SET NULL;

CREATE INDEX IX_STOCK_MOVEMENTS_LOT ON STOCK_MOVEMENTS (LOT_ID, CREATED_AT);
CREATE INDEX IX_STOCK_MOVEMENTS_VACCINE ON STOCK_MOVEMENTS (VACCINE_ID);
```

`STOCK_MOVEMENTS.LOT_ID` es **NULL** para:

- movimientos históricos (anteriores a `0040`): stock "sin lote";
- productos que no manejan lote (accesorios, alimentos sin vencimiento obligatorio);
- ajustes globales de inventario físico.

Invariante que se valida en un trigger (`BI_STOCK_MOVEMENTS_LOT`): si `LOT_ID` no es NULL,
el lote debe existir, estar `ACTIVA` para movimientos negativos y pertenecer al mismo
`PRODUCT_ID` del movimiento. Firebird no permite `CHECK` con subconsulta, por eso el
trigger.

### 2.3 Vínculo con la receta y la factura

`0040` se aplica después de `0037`–`0039`, así que `STOCK_MOVEMENTS.PRESCRIPTION_ITEM_ID` e
`INVOICE_ITEMS.PRODUCT_ID` ya existen:

```sql
-- La receta ya añade STOCK_MOVEMENTS.PRESCRIPTION_ITEM_ID e INVOICE_ITEMS.PRODUCT_ID.
-- Este plan añade el lote a la línea facturada para ventas de mostrador y para el
-- comprobante de dispensación.
ALTER TABLE INVOICE_ITEMS ADD LOT_ID D_PK
    REFERENCES PRODUCT_LOTS (ID) ON DELETE SET NULL;
CREATE INDEX IX_INVOICE_ITEMS_LOT ON INVOICE_ITEMS (LOT_ID);
```

La trazabilidad básica (lote → movimiento) no depende de la receta; el vínculo con la línea
de receta y con la factura de dispensación sí.

### 2.4 Vacunas

```sql
-- Producto de inventario que representa la vacuna (para descontar stock y elegir lote).
ALTER TABLE VACCINE_TYPES ADD PRODUCT_ID D_PK
    REFERENCES PRODUCTS (ID) ON DELETE SET NULL;

-- Lote aplicado. LOT (texto) y MANUFACTURER se conservan como snapshot legado.
ALTER TABLE VACCINES ADD LOT_ID D_PK
    REFERENCES PRODUCT_LOTS (ID) ON DELETE SET NULL;
ALTER TABLE VACCINES ADD STOCK_MOVEMENT_ID D_PK
    REFERENCES STOCK_MOVEMENTS (ID) ON DELETE SET NULL;
CREATE INDEX IX_VACCINES_LOT ON VACCINES (LOT_ID);
```

Backfill y convivencia con el legado:

- `VACCINE_TYPES.PRODUCT_ID` queda NULL; el ADMIN la vincula desde el catálogo
  (`Rabia` → producto `VAC-RABIA`). Mientras no haya vínculo, la vacuna se registra como
  hoy (texto libre) y no descuenta stock.
- `VACCINES.LOT` (texto) **no** se convierte en lote. Los registros históricos muestran su
  lote como texto y `LOT_ID = NULL`; existe una acción "vincular lote" que solo ADMIN puede
  usar y que exige auditoría, para casos puntuales sin reescribir la historia.
- `VACCINES.LOT_ID` es solo de lectura después de guardar: la vacuna es un acto clínico
  inmutable; corregir el lote exige anular y volver a registrar.

### 2.5 Vistas de vencimiento

No se crean tablas de alertas: se calculan al consultar.

```sql
-- Vista de referencia (la consulta real vive en el repositorio con filtros).
CREATE VIEW V_PRODUCT_LOT_STOCK AS
SELECT l.ID AS LOT_ID, l.PRODUCT_ID, l.LOT_NUMBER, l.EXPIRES_AT,
       l.STATUS, l.RECEIVED_AT,
       CAST(COALESCE(SUM(m.QUANTITY), 0) AS INTEGER) AS STOCK
FROM PRODUCT_LOTS l
LEFT JOIN STOCK_MOVEMENTS m ON m.LOT_ID = l.ID
GROUP BY l.ID, l.PRODUCT_ID, l.LOT_NUMBER, l.EXPIRES_AT, l.STATUS, l.RECEIVED_AT;
```

El estado visible se deriva en SQL:

```text
VENCIDO     = STATUS='ACTIVA' AND STOCK>0 AND EXPIRES_AT < CURRENT_DATE
POR_VENCER  = STATUS='ACTIVA' AND STOCK>0 AND EXPIRES_AT BETWEEN hoy AND hoy+30/60/90
AGOTADO     = STOCK<=0
RETIRADO    = STATUS='RETIRADA'
DISPONIBLE  = STATUS='ACTIVA' AND STOCK>0 AND (EXPIRES_AT IS NULL OR EXPIRES_AT>=hoy)
```

### 2.6 Índices y mantenimiento

- `IX_PRODUCT_LOTS_EXPIRY` cubre la bandeja de vencimientos.
- `IX_STOCK_MOVEMENTS_LOT` cubre la trazabilidad de un lote y el cálculo de saldo por lote.
- Revisar `docs/scalability-review-2026-09-10.md`: las consultas de inventario siguen sin
  cambio; el costo extra es una subconsulta por lote al abrir el detalle del producto, no
  en el listado.
- `test_helpers.rs` limpia `PRODUCT_LOTS` antes de `PRODUCTS` (por la FK) después de
  `STOCK_MOVEMENTS`.

---

## 3. Reglas de negocio

### 3.1 Recepción de un lote

```text
receive_product_lot(input):
  validar producto activo y categoría
  exigir LOT_NUMBER no vacío
  exigir EXPIRES_AT si PRODUCTS.CATEGORY = 'MEDICAMENTO'
  rechazar EXPIRES_AT < hoy (no se recibe vencido)
  rechazar (PRODUCT_ID, LOT_NUMBER) duplicado
  transacción:
    insertar PRODUCT_LOTS (STATUS='ACTIVA', INITIAL_QUANTITY)
    insertar STOCK_MOVEMENTS (ENTRADA, +cantidad, LOT_ID, UNIT_COST, REASON='Recepción lote …')
  auditar LOT_RECEIVED
```

- La recepción es la **única** vía para crear stock con lote. `add_stock_movement` con
  `kind='ENTRADA'` sobre un `MEDICAMENTO` exige `lot_id` (mensaje que redirige a
  "Recibir lote"); para las demás categorías el lote es opcional.
- `AJUSTE` positivo sin lote sigue permitido para inventario físico de no medicamentos.
- El costo del lote no reescribe `PRODUCTS.COST_PRICE`: la valoración usa el costo del
  movimiento (ya existente, `STOCK_MOVEMENTS.UNIT_COST`), y el costo del producto queda
  como referencia de venta/costo estándar.

### 3.2 Consumo FEFO

```text
stock_disponible(product_id) =
  Σ STOCK_MOVEMENTS.QUANTITY WHERE PRODUCT_ID = ? AND LOT_ID IS NULL
  + Σ por lote ACTIVA con saldo > 0 y no vencido

fefo_plan(product_id, cantidad):
  lotes = ACTIVA, saldo > 0, EXPIRES_AT >= hoy (o NULL al final)
          ORDER BY EXPIRES_AT ASC, RECEIVED_AT ASC
  repartir `cantidad` entre lotes en ese orden
  si la suma no alcanza → error con faltante y disponible
```

- El plan FEFO es una **propuesta**: el operador puede cambiarla (lote dañado, frasco ya
  abierto) y debe indicar motivo si elige un lote distinto del propuesto.
- El stock sin lote (`LOT_ID IS NULL`) se consume **al final**, después de los lotes
  trazables, para no perder trazabilidad nueva.
- Los lotes `RETIRADA` y los vencidos nunca entran al plan.
- En el MVP la cantidad es entera; si la receta pide 0,6 frascos, se dispensa 1 (regla ya
  documentada en `plan-receta-estructurada.md`).

### 3.3 Dispensación por lote

Extiende `dispense_prescription` del plan de receta (misma transacción):

```text
por cada línea con PRODUCT_ID:
  cantidad = línea.total_to_dispense (o la ajustada a la baja)
  asignaciones = input.lot_allocations ?? fefo_plan(producto, cantidad)
  validar Σ asignaciones = cantidad
  validar saldo de cada lote (WITH LOCK sobre el producto/lote)
  por asignación:
    insertar STOCK_MOVEMENTS (SALIDA, -qty, LOT_ID, REASON='Receta RX-…',
                              PRESCRIPTION_ITEM_ID, CREATED_BY)
  INVOICE_ITEMS.LOT_ID = lote cuando la línea proviene de un único lote;
                         NULL si se repartió (el detalle queda en los movimientos)
  si queda stock insuficiente en lotes válidos → rechazar la dispensación completa
```

- **Una línea de receta puede generar varios movimientos** (uno por lote). No se crea una
  tabla de reparto: los movimientos ya son el detalle y se consultan por
  `PRESCRIPTION_ITEM_ID`.
- Si la receta se anula después de dispensar (solo ADMIN, misma caja/día), se inserta un
  `DEVOLUCION` por cada lote consumido; el stock vuelve al lote de origen y nunca se
  reasigna silenciosamente a otro.
- La factura de dispensación no cambia de diseño (una por dispensación, independiente de la
  visita).

### 3.4 Vacunación

```text
create_vaccine(input):
  si input.lot_id:
    validar que el lote existe, está ACTIVA y no vencido
    si VACCINE_TYPES.PRODUCT_ID existe → el lote debe ser de ese producto
    guardar VACCINES.LOT_ID y VACCINES.LOT (= LOT_NUMBER, snapshot legible en PDF)
    si input.discount_stock:
      exigir capacidad de inventario
      insertar STOCK_MOVEMENTS (SALIDA, -1, LOT_ID, VACCINE_ID, REASON='Vacuna …')
      enlazar VACCINES.STOCK_MOVEMENT_ID
```

- Sin lote vinculado, el registro funciona como hoy (texto libre), pero la UI muestra
  "sin trazabilidad de inventario" y no descuenta stock.
- Descontar stock es opcional e independiente de cobrar la vacuna: hay clínicas que
  facturan la vacuna desde la visita y descuentan el frasco en farmacia.
- El carnet de vacunación (`pdf_templates/vaccines.rs`) añade el lote y su vencimiento
  cuando `LOT_ID` existe; si no, imprime el texto legado.
- Vencimiento del lote ≤ 30 días al vacunar → advertencia en la UI, no bloqueo (se puede
  usar un lote próximo a vencer si aún es válido).

### 3.5 Retiro, cuarentena y baja

```text
retire_product_lot(id, reason):
  exigir ADMIN (o VETERINARIO para cuarentena clínica)
  exigir motivo no vacío
  transacción:
    UPDATE PRODUCT_LOTS SET STATUS='RETIRADA', RETIRED_*  (WITH LOCK)
    si saldo > 0: STOCK_MOVEMENTS (SALIDA, -saldo, LOT_ID, REASON=reason)
  auditar LOT_RETIRED
```

- Un lote retirado no puede dispensarse ni aplicarse; si reaparece físicamente se crea un
  lote nuevo, no se reactiva el anterior (evita reutilizar un número ya comprometido).
- Un lote **vencido con saldo** no se retira automáticamente: aparece en la bandeja y el
  ADMIN decide entre baja por vencimiento, devolución o destrucción (siempre con motivo).
  Esto conserva el histórico para auditoría sanitaria.
- La baja de un lote no toca facturas ni recetas históricas.

### 3.6 Corrección de datos

- Mientras un lote no tenga consumos (solo su `ENTRADA` de recepción), se puede corregir
  número, vencimiento, proveedor y costo con auditoría (`LOT_UPDATED`).
- Con consumos, esas correcciones se bloquean: el lote es parte del historial clínico y de
  inventario. Se corrige retirando y recibiendo de nuevo.
- Nunca se modifica `INITIAL_QUANTITY`; el stock real es la suma de movimientos.

### 3.7 Redondeo y unidades

- `STOCK_MOVEMENTS.QUANTITY` es `INTEGER`: lotes se reciben y consumen en unidades de
  presentación.
- `INITIAL_QUANTITY` es informativo; si se digita mal, se corrige con `AJUSTE` auditado,
  no editando la recepción.

---

## 4. Backend

### 4.1 Archivos

```text
src-tauri/src/models/product_lot.rs        ProductLot, ProductLotStock, LotTrace, inputs
src-tauri/src/repositories/product_lots.rs Recepción, FEFO, saldos, retiro, trazabilidad
src-tauri/src/commands/product_lots.rs
src-tauri/src/repositories/products.rs     add_stock_movement con lot_id; stock por lote
src-tauri/src/repositories/vaccines.rs     lot_id, vencimiento, descuento de stock
src-tauri/src/models/vaccine.rs            lotId, lotExpiresAt, stockMovementId
src-tauri/src/pdf_templates/vaccines.rs    lote + vencimiento en el carnet
src-tauri/src/pdf_templates/prescription.rs Etiquetas con lote/vencimiento
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs` y `lib.rs`
(`collect_commands!` + `.typ::<>`); actualizar `src/bindings.ts` (hoy se mantiene a mano y
`npm run check:bindings` valida el conteo). Añadir la migración en `db/migrations.rs`.

Capacidades: lectura con `require_session`; recepción/retiro con `require_inventory`;
vacunación con `require_clinical` (el descuento de stock exige además inventario); vincular
lote legado y corregir datos, solo ADMIN.

### 4.2 Comandos

| Comando | Capacidad | Descripción |
| --- | --- | --- |
| `list_product_lots(product_id?, status?, expiring_within_days?, search?)` | sesión | Lotes con saldo y estado derivado |
| `get_product_lot(id)` | sesión | Detalle del lote y su saldo |
| `receive_product_lot(input)` | inventario | Crea lote + movimiento `ENTRADA` atómico |
| `update_product_lot(id, input)` | admin | Corrección sin consumos, auditada |
| `retire_product_lot(id, reason)` | inventario/admin | Cuarentena/retiro y salida del saldo |
| `list_expiring_lots(days?)` | sesión | Bandeja de vencimientos (0 = vencidos) |
| `list_product_lot_stock(product_id)` | sesión | Saldo por lote (para el detalle del producto) |
| `suggest_fefo(product_id, quantity)` | sesión | Plan FEFO propuesto para una cantidad |
| `get_lot_traceability(lot_id)` | sesión/admin | Movimientos, recetas, vacunas, pacientes y facturas |
| `list_patient_lot_history(patient_id)` | sesión | Lotes aplicados/recibidos por el paciente |
| `list_vaccine_lots(vaccine_type_id?)` | sesión | Lotes disponibles para vacunar (FEFO) |
| `link_legacy_vaccine_lot(vaccine_id, lot_id, reason)` | admin | Vincula un registro histórico |

`ReceiveLotInput`:

```text
product_id, lot_number, expires_at?, quantity, unit_cost?,
supplier_name?, supplier_invoice?, sanitary_registry?, notes?
```

`UpdateLotInput`:

```text
lot_number?, expires_at?, unit_cost?, supplier_name?, supplier_invoice?, notes?
```

`RetireLotInput`: `reason` (obligatorio), `kind` (`RETIRO` | `VENCIMIENTO` | `DEVOLUCION`).

`StockMovement` (DTO existente) gana `lot_id`, `lot_number`, `lot_expires_at`,
`prescription_item_id`, `vaccine_id`; los listados que hoy no los llenan los devuelven en
`None` (compatibilidad con el frontend actual).

### 4.3 Consultas clave

Stock por lote:

```sql
SELECT l.ID, l.LOT_NUMBER, l.EXPIRES_AT, l.STATUS,
       COALESCE((SELECT SUM(m.QUANTITY) FROM STOCK_MOVEMENTS m WHERE m.LOT_ID = l.ID), 0)
FROM PRODUCT_LOTS l
WHERE l.PRODUCT_ID = ?
ORDER BY l.EXPIRES_AT NULLS LAST, l.RECEIVED_AT
```

> Firebird 5 no soporta `NULLS LAST` en todas las formas; si el parser lo rechaza, se usa
> `ORDER BY CASE WHEN l.EXPIRES_AT IS NULL THEN 1 ELSE 0 END, l.EXPIRES_AT, l.RECEIVED_AT`.
> Verificar en la prueba de migración, no asumirlo.

Trazabilidad de un lote:

```sql
SELECT m.ID, m.KIND, m.QUANTITY, m.CREATED_AT,
       p.NAME AS PATIENT_NAME, v.VACCINE_NAME, rx.CODE AS PRESCRIPTION_CODE,
       o.FULL_NAME AS OWNER_NAME
FROM STOCK_MOVEMENTS m
LEFT JOIN VACCINES v ON v.ID = m.VACCINE_ID
LEFT JOIN PRESCRIPTION_ITEMS pi ON pi.ID = m.PRESCRIPTION_ITEM_ID
LEFT JOIN PRESCRIPTIONS rx ON rx.ID = pi.PRESCRIPTION_ID
LEFT JOIN PATIENTS p ON p.ID = COALESCE(v.PATIENT_ID, rx.PATIENT_ID)
LEFT JOIN OWNERS o ON o.ID = p.OWNER_ID
WHERE m.LOT_ID = ?
ORDER BY m.CREATED_AT DESC
```

Bandeja de vencimientos: una sola consulta sobre `V_PRODUCT_LOT_STOCK` con `DAYS = hoy −
EXPIRES_AT`, agrupando en `VENCIDO`, `≤30`, `≤60`, `≤90` y `SIN_VENCIMIENTO`.

### 4.4 Concurrencia

- Recepción y salida por lote en una transacción; la validación de saldo del lote se hace
  con `SELECT … WITH LOCK` sobre `PRODUCT_LOTS` (mismo patrón que las facturas en pagos),
  no solo con la suma leída, para que dos PCs no dejen un lote en negativo.
- La validación de stock negativo existente en `add_stock_movement` se mantiene para el
  total del producto; se añade la equivalente por lote.
- `UNIQUE (PRODUCT_ID, LOT_NUMBER)` es la garantía final contra duplicados por concurrencia.

---

## 5. Cambios de UI

### 5.1 Inventario (`ProductsPage`)

- El listado de productos añade una columna **Lotes** con el número de lotes disponibles y
  un badge de alerta (`vence en X d`) cuando el próximo vencimiento está dentro de 30 días.
- `ProductDialog` no cambia; el stock deja de editarse "a mano" para medicamentos.
- Nuevo **detalle del producto** (drawer o diálogo): stock total, stock sin lote, tabla de
  lotes (número, vencimiento, estado, saldo, proveedor), acciones **Recibir lote**,
  **Ver trazabilidad**, **Retirar**.
- `StockMovementDialog`: selector de lote opcional; obligatorio para `MEDICAMENTO` en
  `ENTRADA`/`SALIDA`. Al elegir producto muestra el FEFO sugerido.

### 5.2 Vencimientos (nueva pestaña en Inventario)

- Contadores: **Vencidos**, **≤30 días**, **≤60 días**, **≤90 días**.
- Tabla: producto, lote, vencimiento, días restantes, saldo, valor a costo, estado.
- Acciones por fila: **Retirar** (con motivo), **Ver trazabilidad**, **Ajustar saldo**.
- Carretera de salida: exportar CSV de vencimientos (reutiliza el patrón de exportación).

### 5.3 Farmacia / dispensación (`DispenseDialog`, plan de receta)

- Cada línea muestra el **reparto FEFO propuesto** por lote (lote, vence, cantidad) con
  opción de editar cantidades y elegir otro lote (motivo obligatorio si se aparta del FEFO).
- Aviso visible si el lote elegido está retirado o vencido (no seleccionable).
- Totales por lote y total de la dispensación; el botón de confirmar se bloquea si algún
  lote no tiene saldo.
- Al terminar: "Etiquetas" imprimen lote y vencimiento por medicamento.

### 5.4 Vacunación (`NewVaccineDialog`)

- Al elegir el tipo de vacuna, si tiene producto vinculado se habilita **Lote** como
  selector (FEFO, con vencimiento visible), y un checkbox **Descontar del inventario**
  (marcado por defecto cuando hay lote).
- Si el tipo no tiene producto vinculado, el campo Lote sigue siendo texto libre como hoy,
  con la nota "sin trazabilidad de inventario".
- Lote vencido no aparece; lote retirado tampoco. Próximo a vencer: badge ámbar.
- El carnet PDF (`generate_carnet_vacunacion`) añade lote y vencimiento.

### 5.5 Ficha clínica y reportes

- Historial clínico: en cada vacuna aplicada se muestra el lote (texto o vínculo).
- Recetas dispensadas: detalle con los lotes consumidos por línea.
- Diálogo de reportes: nueva opción **Trazabilidad de lote** (PDF con movimientos y
  pacientes afectados) para responder a un retiro del fabricante.
- Panel: tarjeta "Lotes vencidos / por vencer" con enlace a Inventario → Vencimientos.

### 5.6 Recepción

- Aviso al aplicar una vacuna sin lote trazable (solo informativo, no bloquea).
- El check-in no cambia.

---

## 6. Fases y estimación

| Fase | Entregable | Estimación |
| --- | --- | --- |
| **A. Esquema y recepción** | `0040`, modelos, repo de lotes, `receive_product_lot`, saldo por lote, triggers y tests | 3–4 días |
| **B. Inventario y vencimientos** | Detalle de producto, pestaña Vencimientos, retiro/cuarentena, alertas, CSV | 3–4 días |
| **C. Dispensación FEFO** | `dispense_prescription` por lote, reparto en `DispenseDialog`, etiquetas con lote | 4–5 días |
| **D. Vacunas y trazabilidad** | Lote en vacunación, descuento de stock, carnet, trazabilidad inversa, PDF y auditoría | 3–4 días |

Total MVP: **~3 semanas** (después de receta y caja).

---

## 7. Criterios de aceptación

### Lotes y stock

- [ ] Recibir un lote de 10 unidades crea el lote y una `ENTRADA` de 10; el stock del
      producto sube en 10 y el saldo del lote es 10.
- [ ] No se puede recibir un medicamento sin vencimiento ni con vencimiento pasado.
- [ ] Recibir dos veces el mismo `(producto, lote)` falla con mensaje claro.
- [ ] El stock del producto sigue siendo la suma de todos los movimientos, con y sin lote.
- [ ] Un movimiento con lote de otro producto es rechazado por el trigger.
- [ ] Un lote retirado no admite movimientos negativos nuevos.

### FEFO y dispensación

- [ ] Con dos lotes (vence marzo y vence junio), una receta de 12 unidades consume primero
      el de marzo y reparte el resto en junio.
- [ ] El lote vencido no aparece en el plan FEFO ni puede seleccionarse manualmente.
- [ ] Si el total de lotes válidos no alcanza, la dispensación falla completa (no deja
      stock descontado a medias).
- [ ] La dispensación genera un movimiento por lote, todos con `PRESCRIPTION_ITEM_ID`.
- [ ] El detalle de la receta muestra de qué lote salió cada unidad dispensada.
- [ ] Anular una dispensación devuelve el stock a los mismos lotes.

### Vacunas

- [ ] Una vacuna con lote descuenta 1 unidad de ese lote y enlaza el movimiento.
- [ ] Sin lote, la vacuna se registra como hoy y no descuenta stock.
- [ ] Al vincular `VACCINE_TYPES.PRODUCT_ID`, el selector solo ofrece lotes de ese producto.
- [ ] El carnet imprime lote y vencimiento cuando hay vínculo.

### Vencimientos y trazabilidad

- [ ] La bandeja clasifica correctamente vencidos, ≤30, ≤60 y ≤90 días.
- [ ] Retirar un lote con saldo genera salida completa y queda auditado con motivo.
- [ ] La trazabilidad de un lote lista vacunas, recetas y pacientes afectados.
- [ ] Un usuario sin capacidad de inventario no puede recibir ni retirar lotes; solo ADMIN
      vincula lotes históricos.

---

## 8. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Recepción atómica; duplicado de lote; vencimiento obligatorio en medicamentos; saldo por lote; stock sin lote; FEFO con varios lotes; FEFO insuficiente; lote vencido/retirado bloqueado; trigger de producto cruzado; dispensación multi-lote; anulación devuelve a los mismos lotes; vacuna con y sin lote; retiro con motivo; trazabilidad inversa; RBAC. |
| Frontend (Vitest) | Cálculo del reparto FEFO en pantalla; formato de días restantes y estados; bloqueo de lotes vencidos en el selector; suma de asignaciones por línea; alertas de vencimiento. |
| E2E (Playwright) | Recibir dos lotes → dispensar receta → verificar movimientos → vacunar con lote → bandeja de vencimientos → retirar lote → trazabilidad. |
| Migración | BD en 0039 → 0040 conserva productos, stock, vacunas y facturas; movimientos históricos quedan con `LOT_ID NULL`; `VACCINES.LOT` intacto. |

---

## 9. Riesgos y decisiones

| Riesgo | Mitigación |
| --- | --- |
| Lotes mal digitados (número/vencimiento) | Corrección auditada solo sin consumos; después se retira y se recibe otro. |
| Dispensar vencido por prisa | FEFO excluye vencidos; el selector no los muestra; el backend revalida. |
| Doble consumo del mismo lote en dos PCs | Transacción + `WITH LOCK` sobre el lote + validación de saldo. |
| Trazabilidad incompleta del stock histórico | Se muestra "sin lote"; no se inventan lotes retroactivos. |
| Vacunas antiguas sin lote | Se conserva el texto; vínculo manual solo ADMIN y auditado. |
| Recalls del fabricante | `get_lot_traceability` + PDF con pacientes y fechas; retiro bloquea el lote. |
| Facturación de la vacuna duplicada | Descontar stock y cobrar son acciones separadas y explícitas. |
| Lotes huérfanos de productos desactivados | Los lotes siguen consultables; no se ofrecen en selectores de dispensación. |
| Crecimiento de `STOCK_MOVEMENTS` | Índices por lote y por fecha; la bandeja filtra por `EXPIRES_AT`, no escanea todo el libro. |
| Regulación sanitaria | Se guardan lote, vencimiento y registro sanitario; el sistema no sustituye el expediente legal. |

### Decisiones abiertas

1. ¿La recepción de lote reemplaza por completo el movimiento `ENTRADA` manual para
   medicamentos? (Recomendado: sí; el manual queda para no medicamentos y ajustes.)
2. ¿El stock sin lote se consume antes o después de los lotes? (Recomendado: después, para
   priorizar la trazabilidad nueva y el FEFO.)
3. ¿Quién puede retirar un lote? (Recomendado: ADMIN; VETERINARIO solo cuarentena clínica.)
4. ¿El descuento de stock al vacunar es obligatorio cuando hay lote? (Recomendado: no,
   checkbox marcado por defecto.)
5. ¿Se permite editar la cantidad dispensada a la baja? (Recomendado: sí, como ya prevé el
   plan de receta; nunca al alza del saldo del lote.)

---

## 10. Orden de implementación (primer PR)

1. `0040_product_lots.sql` + registro en `db/migrations.rs`; pruebas de migración y de
   triggers con lotes cruzados.
2. Modelos/repositorio de lotes: recepción, saldo por lote, FEFO y retiro; tests Rust.
3. Comandos y bindings; detalle de producto con lotes y `StockMovementDialog` con lote.
4. Pestaña **Vencimientos** y alertas en panel.
5. Dispensación FEFO y etiquetas (requiere `0037`–`0038` ya aplicadas).
6. Vacunación con lote, descuento de stock, carnet y trazabilidad inversa con PDF.
