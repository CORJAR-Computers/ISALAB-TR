# Plan de implementación — Receta estructurada, catálogo de fármacos y dispensación

**Estado:** propuesta (no implementada)
**Base:** schema 0036 · consultas con `TREATMENT_PLAN` libre, `PRODUCTS`/`STOCK_MOVEMENTS`, `INVOICE_ITEMS` sin producto, RBAC por capacidades, PDFs de fórmula y etiquetas.
**Objetivo:** sustituir la receta en texto libre por una receta con fármacos del catálogo, dosis calculada por peso, verificación de alergias y dispensación trazable contra inventario.

> **Numeración reservada:** `0037_prescriptions.sql` y, si se usa el seed de demo,
> `0038_seed_drugs_demo.sql`. Orden acordado: abonos/caja (`0036`), receta (`0037`–`0038`),
> recordatorios (`0039`), lotes/vencimientos (`0040`) y cartera (`0041`). Si cambia el
> orden, renumerar los cinco planes antes de crear la migración.

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Catálogo de fármacos | Principio activo, presentaciones, rango de dosis mg/kg, vía, frecuencia, controlado |
| Peso del paciente | Registro de peso con fecha; último peso disponible en la consulta |
| Receta | Cabecera + líneas con dosis, cantidad, indicaciones y snapshot del fármaco |
| Cálculo | Dosis por kg → mg por toma → unidades/mL → cantidad total |
| Alergias | Registro por paciente; bloqueo al prescribir con override auditado |
| Dispensación | Descuento de stock por línea, estado dispensado, etiquetas |
| Facturación | Factura de dispensación con líneas de producto |
| Impresión | Receta médica estructurada y etiquetas por medicamento |

### 1.2 No entra (MVP)

- Interacciones medicamentosas automáticas (solo alergias y contraindicación por especie).
- Fórmulas magistrales, mezclas IV y cálculo de diluciones.
- Recetario electrónico legal / firma digital certificada.
- Fraccionamiento de frascos (el stock se descuenta por presentación completa).
- Sugerencia automática de dosis sin peso ni validación del veterinario.

### 1.3 Principios

1. **El catálogo no inventa dosis.** Los rangos se cargan y se marcan como revisados por el
   veterinario (`REVIEWED`). Mientras no estén revisados, la UI advierte.
2. **La receta congela lo recetado.** Cada línea guarda nombre, concentración y dosis
   aplicada; cambios posteriores del catálogo no alteran recetas históricas.
3. **La alergia bloquea, no solo avisa.** El backend rechaza la línea salvo override con
   motivo, y el override queda auditado.
4. **Dispensar es una transacción única:** stock, estado de la receta y factura se
   confirman juntos o no se confirma nada.
5. **Compatibilidad:** las consultas antiguas con `TREATMENT_PLAN` siguen imprimiendo su
   fórmula en texto libre; la receta estructurada es adicional.

---

## 2. Migración `0037_prescriptions.sql`

### 2.1 Peso del paciente

```sql
CREATE TABLE PATIENT_WEIGHTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID      D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    MEASURED_AT     TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    WEIGHT_KG       D_AMOUNT NOT NULL CHECK (WEIGHT_KG > 0),
    CONSULTATION_ID D_PK REFERENCES CONSULTATIONS (ID) ON DELETE SET NULL,
    CREATED_BY      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    NOTES           D_NOTES
);
CREATE GENERATOR GEN_PATIENT_WEIGHTS_ID;
CREATE INDEX IX_PATIENT_WEIGHTS_PATIENT ON PATIENT_WEIGHTS (PATIENT_ID, MEASURED_AT);
```

El peso de peluquería (`GROOMING_SESSIONS.WEIGHT_KG`) se usa solo como referencia visual,
no como peso clínico para dosificar.

### 2.2 Alergias

```sql
CREATE TABLE PATIENT_ALLERGIES (
    ID          D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID  D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    -- Vínculo opcional al fármaco; si es NULL se compara por SUSTANCIA.
    DRUG_ID     D_PK REFERENCES DRUGS (ID) ON DELETE SET NULL,
    SUBSTANCE   D_NAME NOT NULL,           -- "AMOXICILINA", "PENICILINA", "POLLO"
    REACTION    D_NOTES,
    -- LEVE | MODERADA | GRAVE | ANAFILAXIA
    SEVERITY    VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'MODERADA' NOT NULL,
    ACTIVE      BOOLEAN DEFAULT TRUE NOT NULL,
    NOTED_AT    TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CREATED_BY  D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    NOTES       D_NOTES
);
CREATE GENERATOR GEN_PATIENT_ALLERGIES_ID;
CREATE INDEX IX_PATIENT_ALLERGIES_PATIENT ON PATIENT_ALLERGIES (PATIENT_ID, ACTIVE);
```

### 2.3 Catálogo

```sql
CREATE TABLE DRUGS (
    ID                    D_PK NOT NULL PRIMARY KEY,
    CODE                  D_CODE NOT NULL UNIQUE,       -- AMOX-250
    NAME                  D_NAME NOT NULL,              -- "Amoxicilina"
    ACTIVE_INGREDIENT     D_NAME NOT NULL,              -- "Amoxicilina trihidrato"
    -- Restricción por especie; NULL = uso general.
    SPECIES_ID            D_PK REFERENCES SPECIES (ID) ON DELETE SET NULL,
    DOSE_MIN_MG_KG        D_AMOUNT,
    DOSE_MAX_MG_KG        D_AMOUNT,
    DEFAULT_ROUTE         VARCHAR(20) CHARACTER SET UTF8, -- ORAL | SC | IM | IV | TOPICA
    DEFAULT_FREQUENCY     D_NAME,                          -- "Cada 12 horas"
    DEFAULT_DOSES_PER_DAY D_AMOUNT,
    DEFAULT_DURATION_DAYS INTEGER,
    -- Requiere receta y control adicional de auditoría.
    CONTROLLED            BOOLEAN DEFAULT FALSE NOT NULL,
    -- El veterinario confirmó los rangos de dosis.
    REVIEWED              BOOLEAN DEFAULT FALSE NOT NULL,
    ACTIVE                BOOLEAN DEFAULT TRUE NOT NULL,
    NOTES                 D_NOTES,
    CREATED_AT            TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT            TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_DRUGS_ID;

CREATE TABLE DRUG_PRESENTATIONS (
    ID           D_PK NOT NULL PRIMARY KEY,
    DRUG_ID      D_PK NOT NULL REFERENCES DRUGS (ID) ON DELETE CASCADE,
    -- Producto de inventario que se descuenta al dispensar (opcional).
    PRODUCT_ID   D_PK REFERENCES PRODUCTS (ID) ON DELETE SET NULL,
    DESCRIPTION  D_NAME NOT NULL,          -- "Tableta 250 mg", "Jarabe 50 mg/mL"
    -- mg por unidad (tableta) o mg por mL (líquido).
    STRENGTH_MG  D_AMOUNT NOT NULL CHECK (STRENGTH_MG > 0),
    -- TABLETA | CAPSULA | ML | AMPOLLA | SOBRE | FRASCO
    UNIT         VARCHAR(16) CHARACTER SET UTF8 NOT NULL,
    VOLUME_ML    D_AMOUNT,                  -- frasco de 100 mL, ampolla de 10 mL
    ACTIVE       BOOLEAN DEFAULT TRUE NOT NULL,
    UNIQUE (DRUG_ID, DESCRIPTION)
);
CREATE GENERATOR GEN_DRUG_PRESENTATIONS_ID;
CREATE INDEX IX_DRUG_PRESENTATIONS_DRUG ON DRUG_PRESENTATIONS (DRUG_ID, ACTIVE);
```

### 2.4 Receta

```sql
CREATE TABLE PRESCRIPTIONS (
    ID              D_PK NOT NULL PRIMARY KEY,
    CODE            D_CODE NOT NULL UNIQUE,             -- RX-000123 (trigger)
    CONSULTATION_ID D_PK NOT NULL REFERENCES CONSULTATIONS (ID) ON DELETE CASCADE,
    PATIENT_ID      D_PK NOT NULL REFERENCES PATIENTS (ID),
    VETERINARIAN_ID D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    ISSUED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- Peso con el que se calculó (snapshot).
    WEIGHT_KG       D_AMOUNT,
    -- EMITIDA | DISPENSADA | ANULADA
    STATUS          VARCHAR(16) CHARACTER SET UTF8 DEFAULT 'EMITIDA' NOT NULL,
    INVOICE_ID      D_PK REFERENCES INVOICES (ID) ON DELETE SET NULL,
    NOTES           D_NOTES,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE GENERATOR GEN_PRESCRIPTIONS_ID;

CREATE TABLE PRESCRIPTION_ITEMS (
    ID                 D_PK NOT NULL PRIMARY KEY,
    PRESCRIPTION_ID    D_PK NOT NULL REFERENCES PRESCRIPTIONS (ID) ON DELETE CASCADE,
    SEQ                INTEGER DEFAULT 0 NOT NULL,
    DRUG_ID            D_PK REFERENCES DRUGS (ID) ON DELETE SET NULL,
    PRESENTATION_ID    D_PK REFERENCES DRUG_PRESENTATIONS (ID) ON DELETE SET NULL,
    -- Snapshots: la receta no cambia si el catálogo cambia.
    DRUG_NAME          D_NAME NOT NULL,
    CONCENTRATION      D_NAME NOT NULL,       -- "250 mg/tableta", "50 mg/mL"
    DOSE_MG_KG         D_AMOUNT NOT NULL,
    DOSE_MG            D_AMOUNT NOT NULL,     -- por toma
    QUANTITY_PER_DOSE  D_AMOUNT NOT NULL,     -- tabletas o mL por toma
    DOSES_PER_DAY      D_AMOUNT NOT NULL,
    DURATION_DAYS      INTEGER NOT NULL,
    TOTAL_TO_DISPENSE  D_AMOUNT NOT NULL,     -- unidades de presentación
    UNIT               VARCHAR(16) CHARACTER SET UTF8 NOT NULL,
    ROUTE              VARCHAR(20) CHARACTER SET UTF8,
    FREQUENCY          D_NAME,
    INSTRUCTIONS       D_NOTES,
    -- Override de alergia con motivo (auditado).
    ALERT_OVERRIDE     BOOLEAN DEFAULT FALSE NOT NULL,
    OVERRIDE_REASON    D_NOTES,
    DISPENSED_QUANTITY D_AMOUNT,
    DISPENSED_AT       TIMESTAMP,
    DISPENSED_BY       D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    STOCK_MOVEMENT_ID  D_PK REFERENCES STOCK_MOVEMENTS (ID) ON DELETE SET NULL,
    UNIQUE (PRESCRIPTION_ID, SEQ)
);
CREATE GENERATOR GEN_PRESCRIPTION_ITEMS_ID;
CREATE INDEX IX_PRESCRIPTION_ITEMS_PRESCRIPTION ON PRESCRIPTION_ITEMS (PRESCRIPTION_ID);
```

### 2.5 Trazabilidad en inventario y factura

```sql
-- Movimiento de stock originado por una línea de receta.
ALTER TABLE STOCK_MOVEMENTS ADD PRESCRIPTION_ITEM_ID D_PK
    REFERENCES PRESCRIPTION_ITEMS (ID) ON DELETE SET NULL;
CREATE INDEX IX_STOCK_MOVEMENTS_PRESCRIPTION_ITEM ON STOCK_MOVEMENTS (PRESCRIPTION_ITEM_ID);

-- Líneas de factura que corresponden a un producto dispensado.
ALTER TABLE INVOICE_ITEMS ADD PRODUCT_ID D_PK
    REFERENCES PRODUCTS (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD PRESCRIPTION_ITEM_ID D_PK
    REFERENCES PRESCRIPTION_ITEMS (ID) ON DELETE SET NULL;
```

> `STOCK_MOVEMENTS.QUANTITY` es `INTEGER`. En el MVP se dispensa por **presentación
> completa** (caja, frasco, tableta) y se registra la cantidad entera; el volumen
> administrado se guarda en la línea de receta, no como fracción de stock.

### 2.6 Seed

- **Sin dosis clínicas inventadas.** El catálogo arranca vacío o con un CSV/import del
  veterinario. Si se quiere demo, `0038_seed_drugs_demo.sql` solo en entorno de
  demostración y con `REVIEWED = FALSE`.
- Importación CSV (comando `import_drugs_csv`) con columnas: `code,name,ingredient,
  species,dose_min,dose_max,route,frequency,doses_per_day,duration,presentation,
  strength,unit,volume,product_code`. Errores por fila, sin abortar todo el archivo.

---

## 3. Reglas de cálculo y seguridad

### 3.1 Cálculo de dosis

```text
dosis_por_toma_mg = peso_kg × dosis_mg_kg
cantidad_por_toma = dosis_por_toma_mg / STRENGTH_MG     (tableta)
                    dosis_por_toma_mg / STRENGTH_MG     (mL, si mg/mL)
total_a_dispensar = cantidad_por_toma × tomas_por_día × días
```

- El cálculo se hace en el backend al guardar (autoritativo) y se replica en el editor
  para feedback inmediato.
- Se permiten decimales (0,25 tabletas; 0,1 mL), con redondeo visual a 2 decimales.
  **El veterinario puede ajustar la cantidad final**; se guarda lo ajustado.
- Sin peso no se calcula: exige peso del día o confirmar el último registrado.
- Dosis fuera de `[DOSE_MIN_MG_KG, DOSE_MAX_MG_KG]` → advertencia con motivo obligatorio
  para continuar; se audita.
- `DRUGS.REVIEWED = FALSE` → banner “dosis sin validar” y confirmación.
- Especie distinta a `DRUGS.SPECIES_ID` → advertencia bloqueante salvo override.

### 3.2 Alertas de alergia

Al agregar una línea se consulta:

```sql
SELECT a.* FROM PATIENT_ALLERGIES a
WHERE a.PATIENT_ID = ? AND a.ACTIVE = TRUE
  AND (a.DRUG_ID = ?                                   -- mismo fármaco
       OR UPPER(a.SUBSTANCE) = UPPER(?)                -- principio activo
       OR UPPER(?) CONTAINING UPPER(a.SUBSTANCE))      -- "PENICILINA" en el nombre
```

- Coincidencia → `create_prescription` rechaza con `AppError::Validation` y devuelve las
  alertas para que la UI muestre el diálogo de override.
- Override: requiere motivo no vacío y capacidad clínica; guarda `ALERT_OVERRIDE` y
  `OVERRIDE_REASON`, y registra `PRESCRIPTION_ALLERGY_OVERRIDE` en `USER_AUDIT_LOG`.
- Las alergias se muestran en la ficha, en la consulta y al hacer check-in de la visita.

### 3.3 Estados

```text
EMITIDA ──dispensar todo──► DISPENSADA
   │
   └──anular (solo si no dispensada)──► ANULADA
```

- Una receta dispensada es inmutable; una corrección se hace anulando y creando otra.
- `dispense_prescription` es idempotente por línea: si `DISPENSED_AT` ya existe, rechaza
  la línea en vez de descontar dos veces.

---

## 4. Backend

### 4.1 Archivos

```text
src-tauri/src/models/drug.rs             Drug, DrugPresentation, inputs, CSV row
src-tauri/src/models/prescription.rs     Prescription, PrescriptionItem, alerts, inputs
src-tauri/src/models/patient_weight.rs   PatientWeight, CreateWeightInput
src-tauri/src/models/patient_allergy.rs  PatientAllergy, CreateAllergyInput
src-tauri/src/repositories/drugs.rs      Catálogo, presentaciones, búsqueda, CSV
src-tauri/src/repositories/prescriptions.rs  Receta, cálculo, alertas, dispensación
src-tauri/src/repositories/patient_health.rs Pesos y alergias
src-tauri/src/commands/drugs.rs
src-tauri/src/commands/prescriptions.rs
src-tauri/src/commands/patient_health.rs
src-tauri/src/pdf_templates/prescription.rs  Receta y etiquetas
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs`, `lib.rs`
(`collect_commands!` + `.typ::<>`), `pdf_templates/mod.rs`; regenerar `src/bindings.ts`.
Añadir `require_clinical` en `auth.rs` (capacidad `can_clinical`).

### 4.2 Comandos

| Comando | Capacidad | Descripción |
| --- | --- | --- |
| `search_drugs(query, species_id?)` | sesión | Catálogo activo con presentaciones y rangos |
| `list_drugs(include_inactive)` | sesión | Administración del catálogo |
| `save_drug(input)` / `set_drug_active(id, active)` | admin | Alta/edición; `REVIEWED` solo admin/vet |
| `save_drug_presentation(input)` / `delete_drug_presentation(id)` | admin | Presentaciones y vínculo a producto |
| `import_drugs_csv(content)` | admin | Importación con errores por fila |
| `list_patient_weights(patient_id)` / `create_patient_weight(input)` | clínica | Peso con fecha |
| `list_patient_allergies(patient_id)` | sesión | Visible para recepción |
| `save_patient_allergy(input)` / `delete_patient_allergy(id)` | clínica | Alergias |
| `check_prescription_alerts(patient_id, drug_id, dose_mg_kg?)` | clínica | Alergias, especie, rango, catálogo sin revisar |
| `create_prescription(input)` | clínica | Cabecera + líneas, cálculo y validación |
| `get_prescription(id)` / `list_prescriptions(patient_id?, consultation_id?, status?)` | sesión | Detalle e historial |
| `update_prescription(id, input)` | clínica | Solo `EMITIDA` y sin dispensar |
| `cancel_prescription(id, reason)` | clínica | Solo `EMITIDA` |
| `dispense_prescription(id, input)` | facturación/inventario | Stock + estado + factura |
| `generate_prescription_pdf(id)` | sesión | PDF receta |
| `generate_medication_labels(id, item_ids)` | sesión | Etiquetas por medicamento |

`CreatePrescriptionInput`:

```text
consultation_id, weight_kg?, notes?,
items: [{ drug_id, presentation_id, dose_mg_kg, doses_per_day, duration_days,
          quantity_per_dose?, total_to_dispense?, route?, frequency?,
          instructions?, override_reason? }]
```

El backend recalcula `dose_mg`, `quantity_per_dose` y `total_to_dispense` si no vienen;
si vienen, valida que no se aparten del cálculo más de un margen configurable (p. ej. 50 %)
y registra el ajuste.

### 4.3 Dispensación

`dispense_prescription` en una sola transacción:

1. Cargar receta `EMITIDA` con líneas no dispensadas.
2. Por línea con `PRODUCT_ID`:
   - verificar stock con `SUM(QUANTITY) >= total_to_dispense`;
   - insertar `STOCK_MOVEMENTS` (`KIND='SALIDA'`, cantidad negativa, `REASON='Receta RX-…'`,
     `PRESCRIPTION_ITEM_ID`, `CREATED_BY`);
   - actualizar `PRESCRIPTION_ITEMS.DISPENSED_*` y `STOCK_MOVEMENT_ID`.
3. Si `charge = true`: crear `INVOICES` (sin `VISIT_ID`, `NOTES='Dispensación RX-…'`) con
   líneas `INVOICE_ITEMS` (`PRODUCT_ID`, `PRESCRIPTION_ITEM_ID`, precio de venta,
   impuesto global de la clínica) y enlazar `PRESCRIPTIONS.INVOICE_ID`.
4. Si todas las líneas quedaron dispensadas → `STATUS='DISPENSADA'`; si no, sigue
   `EMITIDA` y la UI muestra “parcial”.
5. Auditar `PRESCRIPTION_DISPENSED` con receta, factura y líneas.

Decisión MVP: **factura de farmacia independiente** de la factura de la visita, para no
romper la regla “una factura por visita” ni el checkout existente. Consolidarla con la
visita queda como fase posterior.

### 4.4 PDF y etiquetas

- **Receta:** datos de clínica, paciente, tutor, peso, fecha, veterinario, tabla de
  medicamentos (fármaco, concentración, dosis por toma, frecuencia, duración,
  indicaciones), notas y firma. Reutiliza el layout/firma de `pdf_templates/clinical.rs`.
- **Etiquetas:** una por línea dispensada, 80 mm: paciente, tutor, fármaco,
  “Administrar X tabletas/mL cada Y horas por Z días”, fecha, clínica y código RX.
  Reutiliza el generador de etiquetas existente.
- `generate_formula_medica` se mantiene: si la consulta tiene receta estructurada, imprime
  la receta; si no, el `TREATMENT_PLAN` histórico.

---

## 5. Pantallas

### 5.1 Consulta (`NewConsultationDialog`)

- Nuevo campo **Peso (kg)** con “último peso” y fecha; al escribir se guarda en
  `PATIENT_WEIGHTS` al guardar la consulta.
- Reemplazar la sección de medicación del `TREATMENT_PLAN` por **Prescripción**:
  - buscador de fármacos por nombre/principio activo;
  - selector de presentación;
  - dosis mg/kg con rango visible, tomas/día, días;
  - resultado calculado: mg por toma, tabletas/mL por toma, total;
  - indicaciones libres;
  - alertas en línea (alergia, especie, dosis fuera de rango, catálogo sin revisar).
- El campo “Plan de tratamiento” queda para indicaciones no farmacológicas.
- Guardar consulta + receta en una sola operación desde la UI (crear consulta y luego
  receta; si falla la receta, se informa y la consulta queda guardada).

### 5.2 Ficha clínica

- **Alergias**: tarjeta con severidad, reacción, alta/baja; visible también para recepción.
- **Peso**: última medición y mini-historial.
- **Recetas**: lista con estado, “Ver”, “Imprimir”, “Dispensar”, “Anular”.
- Banner de alergia en la cabecera del paciente.

### 5.3 Farmacia (nueva vista `pharmacy`)

Visible para ADMIN, VETERINARIO, AUXILIAR y RECEPCION (según capacidad):

- **Por dispensar**: recetas `EMITIDA`, con paciente, tutor, fecha, líneas y stock
  disponible; acción “Dispensar”.
- **Catálogo**: fármacos, presentaciones, rango de dosis, `REVIEWED`, vínculo a producto;
  edición solo ADMIN.
- **Movimientos**: los `STOCK_MOVEMENTS` con referencia a receta.

### 5.4 Dispensación (`DispenseDialog`)

- Lista de líneas con cantidad prescrita, stock actual y cantidad a dispensar (editable
  solo a la baja).
- Aviso si falta stock, con opción de dispensar el resto.
- Checkbox “Generar factura”; vista previa de totales.
- Al confirmar: resultado, número de factura y botón “Imprimir etiquetas”.

### 5.5 Recepción / visita

- Al hacer check-in, banner “Alergias: …” si el paciente tiene alguna activa.
- En la cola y en la ficha, ícono de alerta.

### 5.6 Reportes

- El diálogo de reportes agrega **Receta médica** y **Etiquetas de medicamentos**.
- La vista previa usa la receta estructurada cuando existe.

---

## 6. Fases y estimación

| Fase | Entregable | Estimación |
| --- | --- | --- |
| **A. Catálogo, peso y alergias** | `0037`, modelos, repos, CRUD, import CSV, tarjetas en ficha, tests | 4–5 días |
| **B. Receta y cálculo** | Editor en consulta, `create/update/cancel`, alertas, PDF de receta | 5–6 días |
| **C. Dispensación** | Stock, factura, estados, etiquetas, vista Farmacia | 4–5 días |
| **D. Integración y cierre** | Banner en recepción, reportes, legado, E2E, auditoría | 2–3 días |

Total MVP: **~4 semanas**.

---

## 7. Criterios de aceptación

### Catálogo y datos

- [ ] Se crea un fármaco con rango de dosis y dos presentaciones; se marca `REVIEWED`.
- [ ] La importación CSV informa filas válidas y errores sin abortar el resto.
- [ ] El último peso del paciente aparece en la consulta con su fecha.
- [ ] Una alergia activa se ve en ficha y en el check-in.

### Receta

- [ ] Paciente de 10 kg, fármaco de 15 mg/kg y tableta de 250 mg → 150 mg/toma,
      0,6 tableta/toma y total según tomas/día × días.
- [ ] Cambiar la dosis fuera de rango exige motivo y queda auditado.
- [ ] Un fármaco sin `REVIEWED` muestra advertencia antes de guardar.
- [ ] Una alergia coincidente bloquea el guardado; con motivo se guarda y audita.
- [ ] La receta guarda snapshots y no cambia al editar el catálogo.
- [ ] El PDF muestra dosis, frecuencia, duración e indicaciones.

### Dispensación

- [ ] Dispensar descuenta stock por presentación y deja el movimiento vinculado.
- [ ] No se permite stock negativo ni dispensar dos veces la misma línea.
- [ ] Con “Generar factura”, se crea una factura con líneas de producto y total correcto.
- [ ] La receta completa pasa a `DISPENSADA`; la parcial sigue `EMITIDA`.
- [ ] Las etiquetas imprimen una por medicamento con la dosis correcta.
- [ ] Recepción no puede prescribir; auxiliar/recepción pueden dispensar; solo admin
      edita el catálogo.

---

## 8. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Cálculo exacto y redondeo; rango de dosis; alergia por fármaco, principio activo y texto; override auditado; snapshots inmutables; dispensación transaccional; stock insuficiente; doble dispensación; factura y totales; anulación; RBAC por capacidad. |
| Frontend (Vitest) | Calculadora (kg, mg, tabletas, mL); formato de dosis; alertas y bloqueo; estados de receta; totales del diálogo de dispensación. |
| E2E (Playwright) | Consulta con peso → receta con dos fármacos → alerta de alergia → override → PDF → dispensar → stock y factura → etiquetas. |
| Migración | BD en 0036 → 0037 conserva consultas, productos, facturas y stock; rollback no requerido. |

---

## 9. Riesgos y decisiones

| Riesgo | Mitigación |
| --- | --- |
| Dosis erróneas por datos del catálogo | `REVIEWED` obligatorio para uso normal; sin seed clínico; auditoría de cambios. |
| Confusión mg / mL / tabletas | Unidad explícita en cada presentación y en la línea; cálculo backend autoritativo. |
| Peso desactualizado | Mostrar fecha del peso; exigir confirmación si tiene más de 30 días. |
| Alergia no detectada por texto | Vínculo `DRUG_ID` + comparación por principio activo; permitir varias alergias. |
| Stock fraccionado | MVP solo presentación completa; documentado en la UI. |
| Doble dispensación | Transacción + `DISPENSED_AT` + validación de estado. |
| Factura duplicada | Una factura por dispensación enlazada a la receta; no se reutiliza la de la visita. |
| Medicamentos controlados | `CONTROLLED` exige veterinario, auditoría y número de receta visible. |
| IVA por producto | MVP usa el IVA global de la clínica; IVA por línea queda para otra fase. |
| Datos legales | La receta no sustituye el recetario oficial; se imprime con firma y datos de la clínica. |

### Decisiones abiertas

1. ¿Catálogo inicial vacío con importación o seed de cortesía sin dosis? (Recomendado: vacío + CSV).
2. ¿Factura de farmacia separada o consolidada con la visita? (Recomendado MVP: separada).
3. ¿Quién puede hacer override de alergia? (Recomendado: solo VETERINARIO/ADMIN).
4. ¿Etiquetas en 80 mm o hoja carta? (Recomendado: 80 mm, reutilizando el generador actual).
5. ¿Se permite editar una receta ya impresa pero no dispensada? (Recomendado: sí, con auditoría).

---

## 10. Orden de implementación (primer PR)

1. `0037_prescriptions.sql` + registro en `db/migrations.rs`; pruebas de migración.
2. Modelos y repositorios de fármacos, presentaciones, pesos y alergias.
3. Comandos de catálogo, alergias y peso; `require_clinical`; bindings.
4. Pantalla de catálogo en Configuración/Farmacia y tarjetas de alergia/peso en la ficha.
5. Receta: cálculo, validación y `create_prescription`; editor mínimo en la consulta.
6. PDF de receta y etiquetas; luego dispensación y factura en el PR siguiente.
