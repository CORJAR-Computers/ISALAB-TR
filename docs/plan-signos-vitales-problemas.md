# Plan de implementación — Signos vitales y lista de problemas del paciente

**Estado:** propuesta (no implementada)
**Base:** schema `0036` · `CONSULTATIONS` con signos clínicos en texto libre (`PHYSICAL_EXAM`, `DIAGNOSIS`, `TREATMENT_PLAN`), sin peso clínico (solo `GROOMING_SESSIONS.WEIGHT_KG`, estético), sin diagnóstico codificado y sin lista de problemas; `ClinicalHistory` agrega consultas, vacunas, muestras, visitas, estética y cobros.
**Objetivo:** registrar las constantes del paciente como dato estructurado y con fecha (peso, temperatura, FC, FR, condición corporal y mucosas) y mantener una **lista de problemas** con diagnóstico y código opcional, para seguir al paciente en el tiempo y alimentar la futura receta por peso, la hospitalización y las métricas clínicas.

> **Numeración reservada:** `0042_vitals_problems.sql`. Orden acordado: abonos/caja (`0036`), receta (`0037`–`0038`), recordatorios (`0039`), lotes (`0040`), cartera (`0041`) y esta iniciativa (`0042`). **No depende de `0037`–`0041`**: usa solo tablas existentes en `0036` (`PATIENTS`, `CONSULTATIONS`, `VISITS`, `STAYS`, `USERS`), así que se aplica igual antes o después de esos planes. Si se implementa antes, no renumerar nada; si se reordena el conjunto, actualizar esta línea y las de los otros planes en el mismo commit.

---

## 1. Alcance

### 1.1 Entra

| Pieza | Contenido |
| --- | --- |
| Vitales | Toma de constantes con fecha, autor y origen (consulta, visita o estancia) |
| Peso | Peso clínico en kg, con último valor y curva de tendencia |
| Constantes | Temperatura, FC, FR, condición corporal (1–9), mucosas y TLLC |
| Problemas | Diagnósticos activos y resueltos, con severidad y cronicidad |
| Codificación | Código opcional con sistema (`CIE10`, `SNOMED`, `OTRO`) |
| Trazabilidad | Resolución y reapertura auditadas; corrección de una toma solo con auditoría |
| Historial | Vitales y problemas dentro de `get_clinical_history` y de la línea de tiempo |
| Recepción | Toma de constantes desde la visita (triage) sin salir de la cola |

### 1.2 No entra (MVP)

- Catálogo oficial de diagnósticos veterinarios: el código se guarda libre, sin validar contra una tabla externa.
- Dosis, recetas o cálculos: esta iniciativa solo deja el peso disponible; la receta es `0037`.
- Signos vitales seriados por hora y órdenes de medicación de hospitalización (hoja de monitoreo): consumen estos datos, pero son otra iniciativa.
- Cálculo automático de índices (IMC, score de dolor, shock index) o alertas por rango anormal.
- Registro de constantes por el tutor o desde dispositivos/básculas conectadas.
- Importación histórica de pesos desde texto libre: lo que no está estructurado no se inventa.

### 1.3 Principios

1. **Una sola fuente para el peso.** El peso clínico vive en los vitales; el de peluquería (`GROOMING_SESSIONS.WEIGHT_KG`) sigue siendo una referencia estética y no se usa para dosificar.
2. **El acto clínico no se reescribe.** Una toma de constantes no se edita: se corrige con una toma nueva o se elimina con auditoría de ADMIN. Un problema no se borra: se resuelve (y se puede reabrir, auditado).
3. **El estado se deriva.** Un problema está activo si `RESOLVED_AT` es `NULL`; no hay columna `STATUS` que pueda desincronizarse.
4. **Los rangos en BD son anchos.** El `CHECK` impide absurdos (peso ≤ 0, temperatura de 200 °C); la validación clínica fina y las advertencias son de la UI y del veterinario.
5. **Cero menú nuevo.** Vitales y problemas viven en la historia clínica del paciente; la recepción solo gana un atajo.
6. **Compatibilidad total.** Las consultas históricas con texto libre siguen imprimiéndose y mostrándose igual; no se reescriben datos previos.

---

## 2. Modelo de datos — migración `0042_vitals_problems.sql`

Sigue el patrón del repo: dominios existentes (`D_PK`, `D_NAME`, `D_DATE`, `D_VALUE`, `D_AMOUNT`, `D_NOTES`), `CREATE GENERATOR`, triggers `BI_*`/`BU_*` para ID y columnas derivadas, `SET TERM ^` y registro en `src-tauri/src/db/migrations.rs`.

### 2.1 Signos vitales

```sql
CREATE TABLE PATIENT_VITALS (
    ID                 D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID         D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    -- Origen clínico de la toma (a lo sumo uno de los tres).
    CONSULTATION_ID    D_PK REFERENCES CONSULTATIONS (ID) ON DELETE SET NULL,
    VISIT_ID           D_PK REFERENCES VISITS (ID) ON DELETE SET NULL,
    STAY_ID            D_PK REFERENCES STAYS (ID) ON DELETE SET NULL,
    RECORDED_AT        TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- Métrico siempre (kg, °C, latidos/min, respiraciones/min, segundos).
    WEIGHT_KG          D_AMOUNT,
    TEMPERATURE_C      D_VALUE,
    HEART_RATE_BPM     INTEGER,
    RESPIRATORY_RATE_BPM INTEGER,
    -- 1–9, con 5 como ideal (escala estándar de condición corporal).
    BODY_CONDITION     SMALLINT,
    -- NORMAL | PALIDA | CONGESTIVA | ICTERICA | CIANOTICA
    MUCOUS_MEMBRANE    VARCHAR(16) CHARACTER SET UTF8,
    CAPILLARY_REFILL_S D_VALUE,
    NOTES              D_NOTES,
    RECORDED_BY        D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT         TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    -- Una toma sin ninguna medida no es una toma.
    CONSTRAINT CHK_VITALS_ANY_MEASURE CHECK (
        WEIGHT_KG IS NOT NULL OR TEMPERATURE_C IS NOT NULL
        OR HEART_RATE_BPM IS NOT NULL OR RESPIRATORY_RATE_BPM IS NOT NULL
        OR BODY_CONDITION IS NOT NULL
    ),
    CONSTRAINT CHK_VITALS_RANGES CHECK (
        (WEIGHT_KG IS NULL OR WEIGHT_KG > 0)
        AND (TEMPERATURE_C IS NULL OR TEMPERATURE_C BETWEEN 20 AND 50)
        AND (HEART_RATE_BPM IS NULL OR HEART_RATE_BPM BETWEEN 0 AND 400)
        AND (RESPIRATORY_RATE_BPM IS NULL OR RESPIRATORY_RATE_BPM BETWEEN 0 AND 200)
        AND (BODY_CONDITION IS NULL OR BODY_CONDITION BETWEEN 1 AND 9)
        AND (MUCOUS_MEMBRANE IS NULL OR MUCOUS_MEMBRANE IN
             ('NORMAL', 'PALIDA', 'CONGESTIVA', 'ICTERICA', 'CIANOTICA'))
    )
);
CREATE GENERATOR GEN_PATIENT_VITALS_ID;
CREATE INDEX IX_PATIENT_VITALS_PATIENT ON PATIENT_VITALS (PATIENT_ID, RECORDED_AT);
```

Decisiones:

- `WEIGHT_KG` es `D_AMOUNT` (2 decimales) porque se usa para dosificar; las demás constantes son `D_VALUE`/`INTEGER`.
- Los rangos son deliberadamente amplios: cubren perro, gato y exóticos. Un gato con 42 °C es un dato real y no debe rebotar en la BD; lo que no puede entrar es un cero o un negativo.
- El origen (`CONSULTATION_ID`/`VISIT_ID`/`STAY_ID`) es informativo; la fila sigue siendo válida si el origen se borra (`ON DELETE SET NULL`).
- Sin `UPDATED_AT`: la toma es inmutable. No hay `UPDATE` en la API.

### 2.2 Lista de problemas

```sql
CREATE TABLE PATIENT_PROBLEMS (
    ID               D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID       D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    DESCRIPTION      D_NAME NOT NULL,
    -- Codificación opcional. Sin catálogo propio: se guarda tal cual.
    CODE             VARCHAR(20) CHARACTER SET UTF8,
    -- CIE10 | SNOMED | OTRO
    CODING_SYSTEM    VARCHAR(16) CHARACTER SET UTF8,
    DIAGNOSED_AT     D_DATE DEFAULT CURRENT_DATE NOT NULL,
    -- LEVE | MODERADA | GRAVE
    SEVERITY         VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'MODERADA' NOT NULL,
    CHRONIC          BOOLEAN DEFAULT FALSE NOT NULL,
    RESOLVED_AT      D_DATE,
    RESOLUTION_NOTES D_NOTES,
    -- Consulta donde se diagnosticó (opcional).
    CONSULTATION_ID  D_PK REFERENCES CONSULTATIONS (ID) ON DELETE SET NULL,
    -- 1 mientras está activo, NULL al resolver. Mantenido por trigger: es la
    -- forma de tener "un solo problema activo por descripción" en Firebird
    -- (no hay índices filtrados), el mismo patrón de CASH_SESSIONS.OPEN_SLOT.
    ACTIVE_SLOT      SMALLINT,
    NOTES            D_NOTES,
    CREATED_BY       D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT CHK_PROBLEM_SEVERITY CHECK (SEVERITY IN ('LEVE', 'MODERADA', 'GRAVE')),
    CONSTRAINT CHK_PROBLEM_CODING CHECK (
        (CODE IS NULL AND CODING_SYSTEM IS NULL)
        OR (CODE IS NOT NULL AND CODING_SYSTEM IS NOT NULL)
    ),
    CONSTRAINT CHK_PROBLEM_SLOT CHECK (
        (RESOLVED_AT IS NULL AND ACTIVE_SLOT = 1)
        OR (RESOLVED_AT IS NOT NULL AND ACTIVE_SLOT IS NULL)
    )
);
CREATE GENERATOR GEN_PATIENT_PROBLEMS_ID;
CREATE UNIQUE INDEX UX_PATIENT_PROBLEMS_ACTIVE
    ON PATIENT_PROBLEMS (PATIENT_ID, DESCRIPTION, ACTIVE_SLOT);
CREATE INDEX IX_PATIENT_PROBLEMS_PATIENT
    ON PATIENT_PROBLEMS (PATIENT_ID, ACTIVE_SLOT);
CREATE INDEX IX_PATIENT_PROBLEMS_CODE
    ON PATIENT_PROBLEMS (CODE);

SET TERM ^ ;

CREATE TRIGGER BI_PATIENT_PROBLEMS FOR PATIENT_PROBLEMS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_PATIENT_PROBLEMS_ID, 1);
    IF (NEW.ACTIVE_SLOT IS NULL AND NEW.RESOLVED_AT IS NULL) THEN
        NEW.ACTIVE_SLOT = 1;
END^

CREATE TRIGGER BU_PATIENT_PROBLEMS FOR PATIENT_PROBLEMS BEFORE UPDATE AS
BEGIN
    -- El slot acompaña a RESOLVED_AT; nunca se escribe desde la aplicación.
    IF (NEW.RESOLVED_AT IS NULL) THEN NEW.ACTIVE_SLOT = 1;
    ELSE NEW.ACTIVE_SLOT = NULL;
    NEW.UPDATED_AT = CURRENT_TIMESTAMP;
END^

CREATE TRIGGER BI_PATIENT_VITALS FOR PATIENT_VITALS BEFORE INSERT AS
BEGIN
    IF (NEW.ID IS NULL) THEN NEW.ID = GEN_ID(GEN_PATIENT_VITALS_ID, 1);
END^

SET TERM ; ^
```

Reglas de integridad:

- `UX_PATIENT_PROBLEMS_ACTIVE` impide dos problemas **activos** con la misma descripción; los resueltos conviven sin problema (`NULL` no colisiona). Un diagnóstico que reaparece se reabre en vez de duplicarse.
- `CHK_PROBLEM_CODING` evita el caso a medias: código sin sistema o sistema sin código.
- No se `DELETE` un problema desde la API; se resuelve. La baja física solo existe para corregir una digitación con auditoría de ADMIN (decisión de implementación, ver §4).

### 2.3 Coordinación con `0037` (receta) — decisión

El plan de receta define `PATIENT_WEIGHTS`. **Esta iniciativa la reemplaza**: el peso clínico es `PATIENT_VITALS.WEIGHT_KG`, y la dosis por kg debe leer la toma más reciente **anterior o igual** a la consulta:

```sql
SELECT FIRST 1 CAST(WEIGHT_KG AS DOUBLE PRECISION)
FROM PATIENT_VITALS
WHERE PATIENT_ID = ? AND WEIGHT_KG IS NOT NULL AND RECORDED_AT <= ?
ORDER BY RECORDED_AT DESC, ID DESC
```

Consecuencias que el PR de `0037` debe respetar:

- `0037` **no crea** `PATIENT_WEIGHTS`; guarda en la receta el `WEIGHT_KG` usado como snapshot (la columna `PRESCRIPTIONS.WEIGHT_KG` ya está prevista).
- Si `0037` se aplicara primero, esta migración no crea una tabla de pesos duplicada: el peso sigue siendo solo `PATIENT_VITALS`.
- La ficha de peluquería no alimenta la dosis (ver §1.3.1).

---

## 3. Backend (Rust)

### 3.1 Archivos nuevos

```text
src-tauri/src/models/vitals.rs        PatientVitals, CreateVitalsInput, LatestVitals
src-tauri/src/models/problem.rs       PatientProblem, CreateProblemInput, UpdateProblemInput
src-tauri/src/repositories/vitals.rs  Alta, listado, último peso, agregados
src-tauri/src/repositories/problems.rs Alta, edición, resolución, reapertura, listado
src-tauri/src/commands/vitals.rs
src-tauri/src/commands/problems.rs
```

Archivos que se tocan:

```text
src-tauri/src/models/clinical_history.rs      + vitals, problems, latest_vitals
src-tauri/src/repositories/clinical_history.rs consulta los dos listados
src-tauri/src/models/mod.rs · repositories/mod.rs · commands/mod.rs
src-tauri/src/auth.rs                          helper require_vitals
src-tauri/src/lib.rs                           collect_commands! + .typ::<> y auditoría
src/bindings.ts · src/lib/api.ts               tipos y llamadas
```

`src/bindings.ts` se mantiene a mano en este repo (el export de dev lo reescribe con `number | null`): tras registrar los comandos, añadir las firmas y tipos a mano y validar con `npm run check:bindings`.

### 3.2 Comandos

| Comando | Capacidad | Notas |
| --- | --- | --- |
| `list_patient_vitals(patient_id, limit?)` | sesión | Más reciente primero; `limit` por defecto 50, máx. 500 |
| `create_patient_vitals(input)` | `can_receive` **o** `can_clinical` | Valida rangos y "al menos una medida"; audita `VITALS_RECORDED` |
| `delete_patient_vitals(id, reason)` | ADMIN | Corrección de digitación; audita `VITALS_DELETED` con el detalle de la toma |
| `list_patient_problems(patient_id, include_resolved?)` | sesión | Activos primero, luego resueltos por fecha |
| `create_patient_problem(input)` | `can_clinical` | Rechaza duplicado activo con mensaje claro; audita `PROBLEM_ADDED` |
| `update_patient_problem(input)` | `can_clinical` | Solo descripción, código, severidad, cronicidad y notas; no toca `RESOLVED_AT` |
| `resolve_patient_problem(id, notes?)` | `can_clinical` | Exige que esté activo; audita `PROBLEM_RESOLVED` |
| `reopen_patient_problem(id, reason)` | `can_clinical` | Exige que esté resuelto; motivo obligatorio; audita `PROBLEM_REOPENED` |

`CreateVitalsInput`:

```text
patient_id, consultation_id?, visit_id?, stay_id?,
weight_kg?, temperature_c?, heart_rate_bpm?, respiratory_rate_bpm?,
body_condition?, mucous_membrane?, capillary_refill_s?, notes?
```

`PatientProblem` expone `active: bool` derivado de `RESOLVED_AT IS NULL` (no se envía en el input).

### 3.3 Reglas de negocio

- **Validar en Rust antes que en la BD.** El `CHECK` es la red de seguridad; el mensaje útil ("la temperatura debe estar entre 20 y 50 °C") sale de la validación del repositorio.
- **Toma vacía:** al menos una de peso, temperatura, FC, FR o condición corporal. Mucosas o TLLC solas no constituyen una toma.
- **Un solo origen:** si llegan dos de `consultation_id`/`visit_id`/`stay_id`, rechazar; el origen es uno.
- **Paciente existente:** validar `PATIENTS.ID` y devolver `NotFound` con mensaje.
- **Problema duplicado:** detectar por `(PATIENT_ID, DESCRIPTION, ACTIVE_SLOT)` y responder "ya existe un problema activo con esa descripción; reábrelo o usa otro nombre".
- **Descripción:** recortar espacios; 3–80 caracteres. Código ≤ 20; si hay código, `CODING_SYSTEM` obligatorio.
- **Resolución:** `RESOLVED_AT = CURRENT_DATE` y notas opcionales; resolver dos veces es un `Validation`.
- **Reapertura:** limpia `RESOLVED_AT`/`RESOLUTION_NOTES` y conserva el historial en la auditoría; si ya hay otro activo con la misma descripción, rechazar.
- **Inmutabilidad:** no hay comando de edición de vitales; `delete_patient_vitals` es la única corrección y exige ADMIN + motivo.
- **RBAC:** `require_vitals` es nuevo en `auth.rs` y equivale a `can_receive || can_clinical` (Recepción, Auxiliar, Veterinario y Admin pueden tomar constantes; Peluquero no). Los problemas son acto clínico: `can_clinical`.

### 3.4 Historial clínico

`ClinicalHistory` gana tres campos y el repositorio tres consultas:

```rust
pub vitals: Vec<PatientVitals>,          // últimas 100 tomas, recientes primero
pub problems: Vec<PatientProblem>,       // activos y resueltos
pub latest: Option<LatestVitals>,        // última toma con peso, para la ficha
```

`LatestVitals` lleva `recorded_at`, `weight_kg`, `temperature_c`, `heart_rate_bpm`, `respiratory_rate_bpm`, `body_condition`, `mucous_membrane`. Se calcula con una sola consulta `FIRST 1` y evita que el frontend ordene para mostrar el resumen.

---

## 4. Frontend (React)

### 4.1 Navegación

Sin vista nueva en la barra lateral. Todo se integra en la historia clínica del paciente:

- `src/features/clinical-history/ClinicalHistoryPage.tsx`: tarjeta de resumen de constantes y panel de problemas junto a la ficha.
- `src/lib/status.ts`: `MUCOUS_MEMBRANE`, `PROBLEM_SEVERITY` y etiquetas/colores.
- `src/hooks/queries/use-vitals.ts`, `src/hooks/queries/use-problems.ts` y barrel `src/hooks/queries/index.ts` + `src/hooks/use-queries.ts`.
- `src/lib/api.ts` y `src/bindings.ts`.

### 4.2 Registro de constantes

`VitalsDialog.tsx` (nuevo, en `clinical-history/`):

- Campos numéricos: peso (kg), temperatura (°C), FC (lpm), FR (rpm), condición corporal (1–9), TLLC (s) y mucosas (select).
- Muestra el **último valor** de cada constante a la derecha del campo, para comparar sin salir.
- Avisos suaves (no bloqueantes) cuando un valor cae fuera del rango típico de la especie; el `CHECK` duro solo impide absurdos.
- "Sin datos" no es cero: un campo vacío no se envía.
- Botón **Guardar toma** deshabilitado si no hay ninguna medida.
- Fuente del origen: si se abre desde una consulta o una visita, viaja el id correspondiente; desde la ficha va sin origen.

`VitalsSummaryCard.tsx`: última toma en una línea ("12,4 kg · 38,9 °C · FC 120 · FR 28 · CC 5/9 · hace 3 días") con botón **Registrar toma** y acceso al historial completo.

`WeightTrendChart.tsx`: curva de peso con las últimas tomas (reutiliza el estilo de `PatientTrendsChart`, que hoy es específico de analitos). Solo se dibuja con dos o más pesos.

### 4.3 Lista de problemas

`ProblemsPanel.tsx`:

- Activos arriba con chip de severidad, marca de crónico y código (`CIE10 K29.3`) cuando exista.
- Resueltos plegados con su fecha de resolución.
- Acciones: **Agregar**, **Editar**, **Resolver** (con notas) y **Reabrir** (con motivo) según `canClinical`.
- Estado vacío útil: "Sin problemas activos. Los diagnósticos de las consultas no se convierten solos: agrégalos aquí para seguirlos."

`ProblemDialog.tsx`: descripción con sugerencias de las ya usadas en el paciente, código + sistema opcionales, severidad, crónico, fecha de diagnóstico y notas.

### 4.4 Línea de tiempo

`ClinicalTimeline.tsx` gana dos tipos de evento:

- `vitals`: "Peso 12,4 kg · T 38,9 °C · FC 120" con el autor y el origen (consulta/visita/estancia).
- `problem`: "Problema agregado / resuelto / reabierto: …" con severidad.

Se actualizan la unión `TimelineItem`, los iconos (`Activity` para vitales, `Stethoscope` para problemas) y el test de la línea de tiempo.

### 4.5 Recepción (fase 3)

En la cola de recepción y en el detalle de la visita: botón **Signos vitales** que abre `VitalsDialog` con `visitId` precargado. Es el caso de triage: el auxiliar pesa y toma temperatura al llegar, antes de que el veterinario abra la consulta.

---

## 5. Fases y estimación

| Fase | Contenido | Estimación |
| --- | --- | --- |
| **1. Esquema y backend** | `0042`, modelos, repositorios, comandos, auditoría, extensión del historial, tests | 3–4 días |
| **2. Historia clínica** | Resumen, diálogo de toma, panel de problemas, curva de peso, línea de tiempo, API tipada | 3–4 días |
| **3. Recepción y consulta** | Toma desde la visita y desde la consulta; accesos rápidos | 1–2 días |

Total MVP: **~2 semanas**. Cada fase con CI verde; un commit por fase.

---

## 6. Criterios de aceptación

### Vitales

- [ ] Se registra una toma con solo el peso y aparece en la historia, en la línea de tiempo y como último peso de la ficha.
- [ ] Una toma con peso `0`, temperatura `60`, FC `500`, FR `300` o condición corporal `0`/`10` se rechaza con mensaje de validación, no con un error de BD.
- [ ] Una toma sin ninguna medida se rechaza.
- [ ] Una toma con dos orígenes (consulta y visita a la vez) se rechaza.
- [ ] El diálogo no envía campos vacíos como cero y deshabilita Guardar sin medidas.
- [ ] La ficha muestra la última toma con su fecha relativa y la curva de peso aparece con dos o más tomas.
- [ ] Corregir una toma exige ADMIN y motivo, y deja `VITALS_DELETED` en la auditoría con el detalle anterior.
- [ ] Un AUXILIAR o RECEPCION puede registrar constantes; un PELUQUERO recibe `Forbidden`.

### Problemas

- [ ] Se agrega un problema sin código y otro con código `CIE10`; ambos se muestran con su sistema.
- [ ] Código sin sistema (o al revés) se rechaza.
- [ ] Un segundo problema activo con la misma descripción se rechaza con un mensaje que sugiere reabrirlo.
- [ ] Resolver un problema lo mueve a resueltos con su fecha y notas; resolverlo dos veces falla con validación.
- [ ] Reabrir exige motivo, lo devuelve a activos y deja `PROBLEM_REOPENED` en la auditoría.
- [ ] Editar un problema no altera su estado de resolución.
- [ ] Solo `canClinical` crea, edita, resuelve y reabre; recepción puede leer pero no escribir.

### Integración y regresión

- [ ] `get_clinical_history` devuelve `vitals`, `problems` y `latest` en una sola llamada.
- [ ] La línea de tiempo ordena mezclados consultas, vitales, problemas, muestras, visitas, estética y cobros.
- [ ] La migración `0042` se aplica sobre una base en `0036` y sobre una en `0041` sin cambios.
- [ ] La suite existente (Rust, Vitest y los E2E actuales) sigue pasando; el menú lateral no cambia.

---

## 7. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Alta con una y con todas las medidas; rangos fuera de límite; toma vacía; dos orígenes; paciente inexistente; listado y último peso; borrado ADMIN auditado; alta de problema con y sin código; código a medias; duplicado activo; resolución y doble resolución; reapertura con motivo y conflicto con otro activo; `get_clinical_history` con ambos listados; RBAC por capacidad. |
| Frontend (Vitest) | Validación del formulario (sin medidas, rangos, campos vacíos no enviados); formato de la última toma; orden activos/resueltos; etiquetas y colores de severidad y mucosas; eventos nuevos de la línea de tiempo. |
| Migración | Base en `0036` → `0042` y base en `0041` → `0042`; verificar generadores, triggers y que `UX_PATIENT_PROBLEMS_ACTIVE` impide el duplicado activo y permite el resuelto. |
| Regresión | Historial de un paciente con consultas antiguas en texto libre sigue renderizando sin cambios; E2E existentes en verde. |

Comandos de verificación previstos:

```bash
cd src-tauri && cargo test --lib repositories::vitals repositories::problems repositories::clinical_history
npm run check:bindings && npx tsc -b && npm run lint && npm test && npm run build
```

---

## 8. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| Dos fuentes de peso (esta y `0037`) | Alto (dosis) | Decisión §2.3: `PATIENT_VITALS` es la única; `0037` no crea `PATIENT_WEIGHTS` |
| Unidades (kg/lb, °C/°F) | Medio (dosis) | Guardar métrico siempre; la UI rotula kg y °C; conversión solo de presentación si algún día se pide |
| Rangos por especie | Medio (bloqueo de datos reales) | `CHECK` amplio + avisos suaves en la UI; no bloquear por especie en el MVP |
| El usuario espera que el diagnóstico de la consulta cree el problema solo | Medio (UX) | Estado vacío explicativo; fase 3 puede ofrecer "convertir diagnóstico en problema" desde la consulta |
| Crecimiento de tomas (una por visita) | Bajo | Índice por paciente y fecha; listados con `limit` (50/100) |
| Códigos sin catálogo | Bajo (calidad de dato) | Campo libre, sistema explícito y sugerencias de descripciones ya usadas |
| Scope creep hacia triage/hospitalización | Medio | Fase 3 acotada a registrar la toma; monitoreo seriado es otra iniciativa |

---

## 9. Orden de implementación (primer PR)

1. `0042_vitals_problems.sql` con tablas, generadores, triggers e índices; registrar en `db/migrations.rs` y probar la migración sobre `0036` y `0041`.
2. `models/vitals.rs`, `models/problem.rs`, `repositories/vitals.rs`, `repositories/problems.rs` con tests de repositorio.
3. `commands/vitals.rs`, `commands/problems.rs`, helper `require_vitals` en `auth.rs`, auditoría y registro en `lib.rs` (`collect_commands!` + `.typ::<>`).
4. Extender `ClinicalHistory` y su repositorio; test de agregación.
5. `src/bindings.ts` y `src/lib/api.ts` a mano; `npm run check:bindings`.
6. Hooks `use-vitals.ts`/`use-problems.ts`, resumen, diálogo, panel de problemas, curva y línea de tiempo.
7. `cargo test` filtrado, `tsc -b`, `lint`, `vitest`, `build`; E2E de regresión.
