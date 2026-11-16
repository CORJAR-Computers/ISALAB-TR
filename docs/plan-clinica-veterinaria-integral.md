# ISALAB — Plan: de laboratorio a clínica veterinaria integral

**Estado:** aprobado — decisión de producto confirmada (MVP = fases 1–5)
**Base:** v1.1.0 (schema 0024)
**Stack actual:** Tauri v2 (Rust) + React 19 + TypeScript + Vite + Tailwind v4 + shadcn/ui + TanStack Query + Firebird 5 Embedded (`rsfbclient`) + `printpdf`

---

## 1. Resumen ejecutivo

ISALAB nació como sistema de laboratorio: el eje del modelo es la **muestra**
(`SAMPLES` → `LAB_RESULTS`) y la recepción que existe hoy es la *recepción de
muestras*, no la recepción del paciente. La clínica necesita lo contrario: un
**paciente que llega, pide uno o varios servicios y se va atendido y
facturado**, con el laboratorio como uno más de esos servicios.

La propuesta es introducir tres piezas transversales y reutilizar todo lo que
ya funciona:

1. **Catálogo de servicios y precios** (`SERVICES`) — única fuente de precio y
   duración para consulta, peluquería, vacunación, cirugía, laboratorio,
   imágenes, domicilio, etc.
2. **Visita / encuentro** (`VISITS` + `VISIT_SERVICES`) — la columna vertebral
   de la recepción: cola de espera, servicios solicitados, profesional
   asignado, estado y cobro.
3. **Agenda generalizada** (`APPOINTMENTS`) — citas de cualquier servicio, no
   solo consultas, con detección de solapamiento por profesional/recurso.

Sobre esa base se agregan los módulos de nicho: **peluquería canina**
(ficha de sesión, fotos antes/después, próximo turno), **recepción con cola de
espera**, **facturación desde la visita** y **métricas por línea de servicio**.
El laboratorio, las consultas, cirugías y vacunas existentes se **vinculan** a
la visita en lugar de reescribirse.

**Alcance MVP (fases 1–5): ~8 semanas.** Hospitalización/guardería e inventario
quedan como fase 6 opcional.

---

## 2. Alcance funcional

### 2.1 Servicios a cubrir

| Categoría | Servicios | Módulo | Fase |
| --- | --- | --- | --- |
| Consulta | Consulta general, especialidades, revisión, segunda opinión, domicilio | `CONSULTATIONS` (existente) + `SERVICES` | 1–2 |
| Laboratorio | Perfiles, paneles, analitos sueltos, muestras | `SAMPLES` / `LAB_ORDERS` (existente) | 1–2 |
| Preventivo | Vacunación, desparasitación, microchip, certificados | `VACCINES` (existente) | 2 |
| Quirúrgico | Cirugías, procedimientos, anestesia | `SURGERIES` (existente) | 2 |
| Estética | Baño, peluquería de raza, corte higiénico, spa, deslanado, limpieza dental estética | **`GROOMING` (nuevo)** | 4 |
| Estancia | Guardería, hospedaje, hospitalización | `STAYS` (fase 6) | 6 |
| Comercio | Alimentos, medicamentos, accesorios | `PRODUCTS` + inventario (fase 6) | 6 |

### 2.2 Fuera de alcance (esta iteración)

- Facturación electrónica DIAN / integración con proveedor tecnológico.
- Multi-sede y multi-terminal con sincronización (la app es local, una ventana).
- Portal del cliente / app móvil / recordatorios automáticos por WhatsApp API.
- Nómina, comisiones de peluqueros, cuentas por cobrar con cartera detallada.
- Imágenes diagnósticas (rayos X/ecografía) más allá de adjuntos en la ficha.

> Estas exclusiones se documentan para no bloquear la arquitectura: el catálogo
> de servicios y la visita son el punto de extensión natural de todas ellas.

---

## 3. Diagnóstico del código actual

| Pieza | Hoy | Qué falta para la clínica | Dónde |
| --- | --- | --- | --- |
| Recepción | Solo recepción **de muestras** (`NewSampleDialog`, estado `RECIBIDA`) | Recepción **de pacientes**: check-in, cola de espera, servicios múltiples, turno | `src/features/samples/`, `src/features/worklist/` |
| Servicios/precios | No existe catálogo; `INVOICE_ITEMS` es **texto libre** (`description`, `unit_price`) | Catálogo con precio, duración, IVA, profesional requerido | `0001_initial_schema.sql` (líneas 226–255), `NewInvoiceDialog.tsx` |
| Agenda | `CONSULTATIONS` con `CONSULTATION_DATE` y estados `PENDIENTE/COMPLETADA/CANCELADA`; sirve de agenda | Citas de cualquier servicio, duración, profesional, recurso, no-show | `models/consultation.rs`, `features/agenda/` |
| Peluquería | No existe | Ficha de sesión, estilo, productos, fotos, próximo turno | — |
| Facturación | Factura ligada a paciente/propietario/consulta; ítems libres; solo `ORDER_ID` como vínculo | Líneas desde catálogo y desde la visita; cobro en un clic | `repositories/invoices.rs`, `NewInvoiceDialog.tsx` |
| Roles | `ADMIN / VETERINARIO / AUXILIAR` con `CHECK` en BD y `ROLES` en Rust | Rol de **recepción** y de **peluquero** (o permisos por capacidad) | `repositories/users.rs:9`, `auth.rs`, `0001` (USERS) |
| Panel | Métricas centradas en muestras y resultados | Visitas del día, cola de espera, ingresos por categoría, ocupación de agenda | `models/dashboard.rs`, `repositories/dashboard.rs` |
| Historial | Línea de tiempo con consultas, muestras, órdenes, vacunas | Mostrar visitas, sesiones de peluquería y cobros | `features/clinical-history/` |
| Marca | Textos fijos "Laboratorio Veterinario" | "Clínica Veterinaria" configurable | `LoginPage.tsx`, `Sidebar.tsx`, `AboutDialog.tsx`, `package.json` |
| Auditoría | Transiciones de muestra/factura/consulta/cirugía | Transiciones de visita y de sesión de peluquería | `repositories/samples.rs` (patrón), `004_user_audit_log` |

**Conclusión:** no hay que rehacer el laboratorio. Hay que agregar el nivel
"visita/servicio" por encima y conectar los módulos existentes a él.

---

## 4. Decisiones de diseño

### D1 — El catálogo de servicios es la única fuente de precio

`SERVICES` reemplaza el texto libre en facturación. Cada línea de factura
guarda una **copia** del precio (`UNIT_PRICE` ya existe), de modo que cambiar
la lista de precios nunca reescribe facturas históricas. La duración y el
profesional requerido viven en el servicio y alimentan la agenda.

### D2 — `VISITS` es la columna vertebral de la recepción; no se reutiliza `SAMPLES`

Una visita puede incluir **varios servicios** (consulta + baño + vacuna), y una
muestra puede originarse en una visita o en una orden de laboratorio. Mezclar
ambos conceptos en `SAMPLES` rompería la trazabilidad existente y el flujo de
worklist. La visita es deliberadamente genérica y liviana; los detalles viven
en cada módulo (`CONSULTATIONS`, `GROOMING_SESSIONS`, `SAMPLES`, `VACCINES`).

### D3 — Agenda nueva, consultas intactas

`APPOINTMENTS` agenda **servicios**; `CONSULTATIONS` sigue siendo el registro
clínico. Una cita de consulta, al hacer check-in, crea la visita y desde ella
la consulta. Las consultas `PENDIENTE` existentes se migran a citas con el
servicio "Consulta general" para no perder la agenda actual.

### D4 — Roles nuevos, permisos por capacidad

Se agregan `RECEPCION` y `PELUQUERO` al `CHECK` de `USERS.ROLE` y a la
validación de Rust. Para no multiplicar condicionales por rol, se introduce un
mapa de **capacidades** en Rust (`can_receive`, `can_clinical`,
`can_groom`, `can_bill`, `is_admin`) y los comandos exigen capacidades, no
roles literales. Los tres roles actuales conservan su comportamiento.

### D5 — La factura se arma desde la visita

`INVOICE_ITEMS` gana FKs opcionales a `SERVICE_ID` y `VISIT_ID`; `INVOICES`
gana `VISIT_ID`. Al cobrar una visita, las líneas se generan desde
`VISIT_SERVICES` con su precio capturado al momento del check-in. La factura
manual sigue existiendo, pero ahora con buscador de catálogo.

---

## 5. Modelo de datos (migraciones 0025–0031)

Sigue el patrón del repo: `D_PK`, generadores + trigger `BI_*`, `SET TERM ^`,
códigos legibles generados en trigger, y registro en `src-tauri/src/db/migrations.rs`.

### 0025 — Catálogo de servicios

```sql
CREATE TABLE SERVICE_CATEGORIES (
    ID         D_PK NOT NULL PRIMARY KEY,
    CODE       D_CODE NOT NULL UNIQUE,   -- CONSULTA, PELUQUERIA, LAB, ...
    NAME       D_NAME NOT NULL,
    SORT_ORDER INTEGER DEFAULT 0 NOT NULL,
    IS_ACTIVE  BOOLEAN DEFAULT TRUE NOT NULL
);

CREATE TABLE SERVICES (
    ID               D_PK NOT NULL PRIMARY KEY,
    CODE             D_CODE NOT NULL UNIQUE,
    NAME             D_NAME NOT NULL,
    CATEGORY_ID      D_PK NOT NULL REFERENCES SERVICE_CATEGORIES (ID),
    DESCRIPTION      D_NOTES,
    DURATION_MIN     INTEGER DEFAULT 30 NOT NULL CHECK (DURATION_MIN > 0),
    PRICE            D_AMOUNT DEFAULT 0 NOT NULL,
    -- NULL = usa el IVA global de CLINIC_SETTINGS ('invoice.tax_rate').
    TAX_RATE         D_PERCENT,
    REQUIRES_VET     BOOLEAN DEFAULT FALSE NOT NULL,
    REQUIRES_GROOMER BOOLEAN DEFAULT FALSE NOT NULL,
    SPECIES_ID       D_PK REFERENCES SPECIES (ID) ON DELETE SET NULL, -- NULL = todas
    SORT_ORDER       INTEGER DEFAULT 0 NOT NULL,
    IS_ACTIVE        BOOLEAN DEFAULT TRUE NOT NULL,
    CREATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
```

Seed inicial: consulta general, consulta de especialidad, revisión, consulta a
domicilio, vacunación, desparasitación, implantación de microchip, cirugía
menor, perfil básico de laboratorio, perfil completo, baño, baño medicado,
peluquería de raza, corte higiénico, deslanado, spa, limpieza dental estética.
**Decisión confirmada:** se siembra una **propuesta de precios de referencia**
(la clínica los ajusta desde Configuración → Servicios antes de operar).

### 0026 — Roles de clínica

```sql
-- Firebird no permite ALTER de un CHECK: se localiza y elimina el constraint
-- RDB$CHECK_CONSTRAINTS de USERS.ROLE y se recrea con los 5 roles.
-- Valor nuevo: 'RECEPCION', 'PELUQUERO'.
```

Cambios acompañantes:

- `src-tauri/src/repositories/users.rs`: `ROLES` pasa a 5 y `validate_role`.
- `src-tauri/src/models/auth.rs`: comentarios de rol.
- `src/hooks/use-permissions.ts`: exponer `canReceive`, `canGroom`,
  `canClinical`, `canBill`.
- `src/features/users/`: formulario de usuario con los roles nuevos.

### 0027 — Visitas y servicios de la visita

```sql
CREATE TABLE VISITS (
    ID           D_PK NOT NULL PRIMARY KEY,
    CODE         D_CODE NOT NULL UNIQUE,               -- V-YYYY-NNNN
    PATIENT_ID   D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    OWNER_ID     D_PK NOT NULL REFERENCES OWNERS (ID),
    APPOINTMENT_ID D_PK REFERENCES APPOINTMENTS (ID) ON DELETE SET NULL,
    ARRIVED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIORITY     VARCHAR(12) CHARACTER SET UTF8 DEFAULT 'NORMAL' NOT NULL
                 CHECK (PRIORITY IN ('NORMAL', 'URGENTE')),
    STATUS       VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'EN_ESPERA' NOT NULL,
                 -- EN_ESPERA | EN_ATENCION | FINALIZADA | FACTURADA | CANCELADA
    NOTES        D_NOTES,
    CREATED_BY   D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE VISIT_SERVICES (
    ID              D_PK NOT NULL PRIMARY KEY,
    VISIT_ID        D_PK NOT NULL REFERENCES VISITS (ID) ON DELETE CASCADE,
    SERVICE_ID      D_PK NOT NULL REFERENCES SERVICES (ID),
    PROFESSIONAL_ID D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    QUANTITY        INTEGER DEFAULT 1 NOT NULL CHECK (QUANTITY > 0),
    -- Snapshot del precio al hacer check-in.
    UNIT_PRICE      D_AMOUNT NOT NULL,
    TAX_RATE        D_PERCENT,
    STATUS          VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'PENDIENTE' NOT NULL,
                    -- PENDIENTE | EN_PROCESO | COMPLETADA | CANCELADA
    STARTED_AT      TIMESTAMP,
    FINISHED_AT     TIMESTAMP,
    NOTES           D_NOTES,
    SEQ             INTEGER DEFAULT 0 NOT NULL
);

ALTER TABLE CONSULTATIONS ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;
ALTER TABLE CONSULTATIONS ADD SERVICE_ID D_PK REFERENCES SERVICES (ID) ON DELETE SET NULL;
ALTER TABLE SAMPLES ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;
ALTER TABLE SURGERIES ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;
ALTER TABLE VACCINES ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;

ALTER TABLE INVOICES ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD SERVICE_ID D_PK REFERENCES SERVICES (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD VISIT_ID D_PK REFERENCES VISITS (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD CONSULTATION_ID D_PK REFERENCES CONSULTATIONS (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD SURGERY_ID D_PK REFERENCES SURGERIES (ID) ON DELETE SET NULL;
ALTER TABLE INVOICE_ITEMS ADD VACCINE_ID D_PK REFERENCES VACCINES (ID) ON DELETE SET NULL;
```

Índices: `VISITS (STATUS, ARRIVED_AT)`, `VISITS (PATIENT_ID)`,
`VISIT_SERVICES (VISIT_ID)`, `VISIT_SERVICES (SERVICE_ID, STATUS)`.

### 0028 — Agenda generalizada

```sql
CREATE TABLE APPOINTMENTS (
    ID              D_PK NOT NULL PRIMARY KEY,
    CODE            D_CODE NOT NULL UNIQUE,            -- C-YYYY-NNNN
    PATIENT_ID      D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    SERVICE_ID      D_PK NOT NULL REFERENCES SERVICES (ID),
    PROFESSIONAL_ID D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    SCHEDULED_AT    TIMESTAMP NOT NULL,
    DURATION_MIN    INTEGER NOT NULL CHECK (DURATION_MIN > 0),
    STATUS          VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'PROGRAMADA' NOT NULL,
                    -- PROGRAMADA | CONFIRMADA | EN_ATENCION | ATENDIDA | NO_ASISTIO | CANCELADA
    NOTES           D_NOTES,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
```

- Índice `APPOINTMENTS (SCHEDULED_AT, PROFESSIONAL_ID)`.
- Backfill: cada `CONSULTATIONS` con estado `PENDIENTE` y fecha futura genera
  una cita con el servicio "Consulta general"; las históricas no se duplican.
- Validación de solapamiento en Rust (mismo profesional, intervalo que se
  cruza, estados no cancelados). En BD no se puede expresar con un `CHECK`.

### 0029 — Peluquería

```sql
CREATE TABLE GROOMING_SESSIONS (
    ID              D_PK NOT NULL PRIMARY KEY,
    VISIT_SERVICE_ID D_PK NOT NULL UNIQUE
                     REFERENCES VISIT_SERVICES (ID) ON DELETE CASCADE,
    GROOMER_ID      D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    STYLE           D_NAME,          -- corte de raza, higiénico, cachorro…
    BATH_TYPE       VARCHAR(40) CHARACTER SET UTF8,  -- medicado, antipulgas…
    COAT_CONDITION  D_NOTES,
    MATTING_LEVEL   SMALLINT DEFAULT 0 NOT NULL CHECK (MATTING_LEVEL BETWEEN 0 AND 3),
    WEIGHT_KG       D_VALUE,
    BEHAVIOR        VARCHAR(40) CHARACTER SET UTF8,  -- tranquilo, nervioso, agresivo
    PRODUCTS_USED   D_NOTES,
    STARTED_AT      TIMESTAMP,
    FINISHED_AT     TIMESTAMP,
    NEXT_SUGGESTED_AT D_DATE,
    NOTES           D_NOTES,
    CREATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT      TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Fotos antes/después reutilizando el patrón de RESULT_ATTACHMENTS (0009).
CREATE TABLE VISIT_ATTACHMENTS (
    ID         D_PK NOT NULL PRIMARY KEY,
    VISIT_ID   D_PK NOT NULL REFERENCES VISITS (ID) ON DELETE CASCADE,
    FILE_NAME  VARCHAR(255) CHARACTER SET UTF8 NOT NULL,
    FILE_PATH  VARCHAR(1000) CHARACTER SET UTF8 NOT NULL,
    MIME_TYPE  VARCHAR(100) CHARACTER SET UTF8,
    KIND       VARCHAR(20) CHARACTER SET UTF8,  -- BEFORE | AFTER | DOCUMENT
    CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
```

Checklist de tareas (uñas, oídos, glándulas, secado) se resuelve con una tabla
simple `GROOMING_TASKS (SESSION_ID, TASK_CODE, DONE)` si el equipo la pide; no
es necesaria para el MVP.

### 0030 — Índices y métricas

Índices para panel y listados: `VISITS (ARRIVED_AT)`, `VISIT_SERVICES (STATUS)`,
`APPOINTMENTS (SCHEDULED_AT)`, `GROOMING_SESSIONS (NEXT_SUGGESTED_AT)`.

### 0031 — Ajustes de configuración

Nuevas claves en `CLINIC_SETTINGS`:

| Clave | Uso |
| --- | --- |
| `clinic.tagline` | "Clínica Veterinaria" en login/sidebar/About (reemplaza textos fijos) |
| `grooming.default_duration` | Duración por defecto si el servicio no la define |
| `reception.ticket_prefix` | Prefijo del turno impreso (opcional) |

---

## 6. Backend (Rust)

### 6.1 Archivos nuevos

```text
src-tauri/src/models/service.rs        Service, ServiceCategory, inputs CRUD
src-tauri/src/models/visit.rs          Visit, VisitService, VisitListItem, inputs
src-tauri/src/models/appointment.rs    Appointment, AppointmentListItem, inputs
src-tauri/src/models/grooming.rs       GroomingSession, inputs
src-tauri/src/repositories/services.rs
src-tauri/src/repositories/visits.rs
src-tauri/src/repositories/appointments.rs
src-tauri/src/repositories/grooming.rs
src-tauri/src/commands/services.rs
src-tauri/src/commands/visits.rs
src-tauri/src/commands/appointments.rs
src-tauri/src/commands/grooming.rs
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs` y en
`lib.rs` (`collect_commands!`). Regenerar `src/bindings.ts` y `src/lib/api.ts`.

### 6.2 Comandos propuestos

| Comando | Capacidad | Notas |
| --- | --- | --- |
| `list_service_categories`, `upsert_service_category`, `delete_service_category` | catálogo (ADMIN) | Baja lógica si tiene servicios |
| `list_services` (filtro categoría/activo/búsqueda), `get_service`, `create_service`, `update_service`, `set_service_active` | lectura clínica / escritura ADMIN | Precio y duración |
| `list_visits` (fecha, estado, búsqueda), `get_visit`, `create_visit`, `update_visit_status`, `cancel_visit`, `add_visit_service`, `remove_visit_service`, `update_visit_service_status` | recepción | Transiciones validadas en Rust |
| `checkout_visit` | facturación | Crea factura con líneas desde `VISIT_SERVICES` y marca `FACTURADA` |
| `list_appointments` (rango, categoría, profesional), `create_appointment`, `update_appointment`, `cancel_appointment`, `check_in_appointment` | recepción/agenda | Check-in crea visita |
| `get_grooming_session`, `upsert_grooming_session`, `finish_grooming_session` | peluquería | Crea recordatorio (`NEXT_SUGGESTED_AT`) |
| `add_visit_attachment`, `list_visit_attachments`, `delete_visit_attachment` | clínica | Copia archivo a `app_data/attachments` |
| `get_clinic_dashboard_stats` (extender) | todas | Nuevas métricas |

### 6.3 Reglas de negocio a implementar

- **Máquina de estados de la visita**: `EN_ESPERA → EN_ATENCION → FINALIZADA
  → FACTURADA`; `CANCELADA` desde cualquier estado previo al cobro. Toda
  transición se registra en `EVENT_LOG`/`USER_AUDIT_LOG` (patrón de
  `repositories/samples.rs`).
- **Precio capturado**: `VISIT_SERVICES.UNIT_PRICE` se congela al agregar el
  servicio; si el catálogo cambia, la visita y la factura no cambian.
- **Cobro**: solo visitas `FINALIZADA` se facturan; no se puede facturar dos
  veces la misma visita (`INVOICES.VISIT_ID` único lógico validado en Rust).
- **IVA por servicio**: `SERVICES.TAX_RATE` si existe; si no, el global. La
  factura guarda su `TAX_RATE` calculado como hoy.
- **Agenda**: duración = `SERVICES.DURATION_MIN`; se rechaza solapamiento del
  mismo profesional (salvo `CANCELADA`/`NO_ASISTIO`).
- **Peluquería**: una sesión por línea de servicio de categoría peluquería; no
  se cierra sin profesional asignado; `FINISHED_AT` cierra también la línea.
- **RBAC**: los comandos de catálogo/configuración exigen `require_admin`; los
  de recepción exigen `can_receive`; los clínicos `can_clinical`; peluquería
  `can_groom`. `AUXILIAR` conserva acceso a recepción y peluquería.

---

## 7. Frontend (React)

### 7.1 Navegación y tipos

- `src/stores/ui-store.ts`: nuevos `View`: `reception`, `appointments`,
  `grooming`, `services`. `EntityKind`: `visit`, `appointment`, `grooming`.
- `src/components/layout/Sidebar.tsx` y `TopBar.tsx`: entradas y títulos.
- `src/App.tsx`: imports `lazy` y render por vista.
- `src/lib/status.ts`: `VISIT_STATUS`, `VISIT_SERVICE_STATUS`,
  `APPOINTMENT_STATUS`, `GROOMING_MATTING`, colores y etiquetas.
- `src/hooks/queries/use-visits.ts`, `use-appointments.ts`, `use-services.ts`,
  `use-grooming.ts` siguiendo el patrón de `use-samples.ts` (invalidate por
  eventos Firebird).

### 7.2 Recepción (nuevo, prioridad 1)

`src/features/reception/`

- `ReceptionPage.tsx`: dos columnas.
  - **Izquierda**: buscador de paciente (reutiliza `PatientScanner`, lectura de
    código PAC-/chip), botón "Nuevo paciente" (reutiliza `NewPatientDialog`).
  - **Derecha**: **cola de espera del día** con tarjetas por visita: hora de
    llegada, paciente, propietario, servicios, prioridad (urgente resaltado),
    profesional, estado y acciones (`Iniciar atención`, `Finalizar`, `Cobrar`).
- `NewVisitDialog.tsx`: selección múltiple de servicios agrupados por categoría
  con precio y duración; profesional por servicio; prioridad; notas; total
  estimado en vivo.
- `VisitDetailDialog.tsx`: servicios y estados, vínculos a consulta, muestra,
  orden de laboratorio, cirugía, vacuna y sesión de peluquería; adjuntos;
  historial de la visita.
- `CheckoutDialog.tsx`: resumen de líneas con precio congelado, IVA, total,
  método de pago; emite factura y abre el recibo existente.
- `PrintTicketDialog.tsx` (opcional, fase 2): turno con código de visita y
  código de barras, reutilizando `printpdf`.

**Flujo:** escanear/buscar → "Nueva visita" → marcar servicios → guardar →
aparece en cola → iniciar/finalizar cada servicio → cobrar → factura.

### 7.3 Agenda (nuevo)

`src/features/agenda/AppointmentsPage.tsx`

- Vista día/semana con filtros por categoría de servicio y profesional.
- Crear cita con paciente, servicio (duración automática), profesional, fecha,
  notas; validación de choque en el backend.
- Acciones: confirmar, check-in (crea visita y navega a recepción), marcar
  no-show, cancelar.
- Mantener `ConsultationsPage` como historial clínico de consultas; enlazarla
  desde la cita/visita.

### 7.4 Peluquería (nuevo)

`src/features/grooming/`

- `GroomingPage.tsx`: tablero del día (citas y visitas de peluquería), estado
  por sesión, próximo turno sugerido.
- `GroomingSessionDialog.tsx`: estilo, tipo de baño, condición del pelaje,
  nivel de enredos, peso, comportamiento, productos, hora inicio/fin, próxima
  fecha sugerida, fotos antes/después (`add_visit_attachment`).
- Al finalizar: ofrece agendar el próximo turno con `NEXT_SUGGESTED_AT`.

### 7.5 Catálogo de servicios

`src/features/services/ServicesPage.tsx` (visible para ADMIN; lectura para el
resto en el selector de servicios).

- Tabla por categoría con búsqueda, activo/inactivo, precio, duración, IVA,
  profesional requerido.
- Diálogo de alta/edición con validación Zod.
- Acceso desde Configuración además de la barra lateral.

### 7.6 Facturación

`NewInvoiceDialog.tsx`:

- Buscador de conceptos sobre `SERVICES` que autocompleta descripción, precio
  e IVA; se conserva la edición manual del precio (descuentos).
- Botón "Cargar desde visita" para prellenar líneas.
- `InvoiceDetailDialog.tsx`: mostrar servicio y visita de cada línea.

### 7.7 Historial clínico y búsqueda

- `ClinicalTimeline.tsx`: nuevos eventos — visita, sesión de peluquería, cobro.
- `ClinicalHistoryPage.tsx`: botón "Nueva visita" junto a los existentes.
- `GlobalSearchPalette.tsx`: buscar visitas (V-), citas (C-) y servicios.
- `PatientCard.tsx`: última visita y próximo turno de peluquería.

### 7.8 Panel de control

`DashboardPage.tsx` y `models/dashboard.rs`:

- Visitas de hoy por estado, tiempo promedio de espera, urgentes en cola.
- Citas de hoy por categoría, próximas 5 (todas las categorías).
- Ingresos del mes por categoría de servicio (nuevo `revenue_by_category`).
- Ocupación de peluquería (sesiones del día / capacidad estimada).
- Conservar todas las métricas de laboratorio actuales.

### 7.9 Marca y textos

- `LoginPage.tsx`, `Sidebar.tsx` (alt de logo), `AboutDialog.tsx`: leer
  `clinic.tagline` o usar "Clínica Veterinaria" por defecto.
- `package.json` (`description`), `README.md` y `tauri.conf.json` (título de
  ventana si aplica): actualizar denominación. **No** cambiar el identificador
  de la app ni las claves de firma (rompería el auto-updater).

---

## 8. Fases, entregables y estimación

| Fase | Contenido | Migraciones | Estimación |
| --- | --- | --- | --- |
| **1. Catálogo y facturación por servicio** | `SERVICES` + admin UI, selector en factura, precios congelados, IVA por servicio | 0025, parte de 0027 (FK en ítems) | 1.5–2 sem |
| **2. Recepción y cola de espera** | `VISITS`, `VISIT_SERVICES`, roles, check-in, estados, cobro desde visita, ticket | 0026, 0027 | 2 sem |
| **3. Agenda generalizada** | `APPOINTMENTS`, calendario, check-in, no-show, backfill de consultas pendientes | 0028 | 1.5 sem |
| **4. Peluquería** | `GROOMING_SESSIONS`, adjuntos, tablero, próxima cita | 0029 | 2 sem |
| **5. Panel, historial y marca** | Métricas por servicio, timeline, búsqueda, textos de clínica | 0030, 0031 | 1 sem |
| **6. Opcional: estancia e inventario** | `RESOURCES`, `STAYS`, `PRODUCTS`, movimientos de stock | 0032+ | 3–4 sem |

**MVP clínico: fases 1–5 (~8 semanas).** Cada fase es un commit/release propio
con CI verde, siguiendo el proceso de `docs/release-process.md`.

### Orden de implementación dentro de la fase 1

1. Migración 0025 + seed de categorías/servicios con precios de referencia.
2. Modelos, repositorio y comandos con tests de repositorio (Firebird real).
3. `ServicesPage` + hooks + navegación.
4. `NewInvoiceDialog` con catálogo (sin romper la creación manual).
5. `INVOICE_ITEMS.SERVICE_ID` y migración de compatibilidad.

---

## 9. Criterios de aceptación

### Fase 1

- [ ] Un ADMIN crea/edita/desactiva servicios con categoría, precio, duración
      e IVA; un servicio inactivo no aparece en facturación.
- [ ] En facturación se busca "baño" y se agrega con precio e IVA correctos;
      el usuario puede ajustar el precio de la línea.
- [ ] Cambiar el precio del catálogo no altera facturas ya emitidas.
- [ ] Los ítems de factura guardan `SERVICE_ID` cuando vienen del catálogo.

### Fase 2

- [ ] Recepción registra una visita con 3 servicios distintos y esta aparece
      en la cola con hora, paciente, servicios y prioridad.
- [ ] Se puede iniciar y finalizar cada servicio por separado.
- [ ] "Cobrar" genera una factura con una línea por servicio y totales
      correctos; la visita queda `FACTURADA` y no se puede volver a cobrar.
- [ ] `RECEPCION` puede registrar visitas y cobrar, pero no administrar
      usuarios ni configuración.
- [ ] Cada transición de estado queda auditada.
- [ ] Cancelar una visita la saca de la cola sin borrar su historial.

### Fase 3

- [ ] Se agenda una cita de "Peluquería de raza" con duración tomada del
      servicio; el backend rechaza solapamiento del mismo peluquero.
- [ ] Check-in de una cita crea la visita con sus servicios y abre recepción.
- [ ] Marcar no-show y cancelar actualizan la agenda y las métricas.
- [ ] Las consultas pendientes previas aparecen como citas tras migrar.

### Fase 4

- [ ] Una visita de peluquería abre ficha con campos de sesión y guarda
      fotos antes/después.
- [ ] Finalizar la sesión cierra la línea de servicio y sugiere el próximo
      turno en la fecha calculada.
- [ ] La ficha del paciente muestra la última sesión y el próximo turno.

### Fase 5

- [ ] El panel muestra visitas de hoy, espera promedio e ingresos por
      categoría sin perder las métricas de laboratorio.
- [ ] El historial del paciente lista visitas, sesiones y cobros.
- [ ] Login, sidebar y About dicen "Clínica Veterinaria" (o el tagline
      configurado).

---

## 10. Pruebas

| Nivel | Qué cubrir |
| --- | --- |
| Rust (`cargo test`, Firebird real) | CRUD de servicios y validaciones; máquina de estados de visita; precio congelado; cobro único; solapamiento de citas; RBAC por capacidad; sesión de peluquería; métricas del panel. |
| Frontend (Vitest) | Esquemas Zod de servicio/visita/cita; etiquetas y colores de estado; cálculo de totales con IVA por servicio; render de la cola. |
| E2E (Playwright) | Recepción: buscar paciente → visita con 2 servicios → finalizar → cobrar → factura visible. Agenda: crear cita → check-in → visita. Peluquería: sesión con próxima fecha. |
| Regresión | Los 17 E2E actuales y los flujos de laboratorio deben seguir pasando; el menú actual no cambia de orden salvo las entradas nuevas. |

---

## 11. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| Modificar el `CHECK` de `USERS.ROLE` en Firebird | Alto (bloquea login) | Migración probada sobre copia de BD real; `EXECUTE BLOCK` que localiza el constraint en `RDB$CHECK_CONSTRAINTS`; test de arranque con BD migrada. |
| Visitas y muestras se confunden | Alto (trazabilidad) | `VISITS` no reemplaza `SAMPLES`; `SAMPLES.VISIT_ID` es opcional; la worklist no cambia. |
| Precio histórico alterado | Medio (facturación) | Snapshot `UNIT_PRICE` y `TAX_RATE` en `VISIT_SERVICES` y en `INVOICE_ITEMS`. |
| IVA de servicios veterinarios (Colombia) | Medio (legal) | IVA por servicio con anulación manual; validar con contador antes de fijar seeds. |
| Roles nuevos rompen `require_admin` | Medio | Mapa de capacidades + tests de permisos por rol. |
| Agenda duplicada (consultas y citas) | Medio (UX) | Una sola agenda visible; consultas quedan como registro clínico; backfill único. |
| Migración de datos en instalaciones existentes | Alto | Migraciones idempotentes y probadas desde BD 0024; backup automático antes de migrar si ya existe. |
| Crecimiento del menú lateral | Bajo | Agrupar en secciones: Recepción, Clínica, Laboratorio, Estética, Administración. |
| Impresión de turnos en impresora térmica | Bajo | Ticket PDF A4/80 mm en fase 2; no bloquea el MVP. |

---

## 12. Decisiones confirmadas (2026-11-16)

1. **Alcance del MVP:** fases 1–5 (~8 semanas). Guardería/hospedaje/
   hospitalización e inventario quedan en **fase 6**, fuera del MVP.
2. **Roles:** se crean `RECEPCION` y `PELUQUERO` + mapa de capacidades
   (`can_receive`, `can_clinical`, `can_groom`, `can_bill`, `is_admin`).
3. **Guardería/hospedaje:** fase 6.
4. **Inventario/farmacia:** no en el MVP; la venta de productos se cobra como
   línea manual en la factura mientras tanto.
5. **Recursos físicos** (mesas de peluquería, jaulas): solo profesional en el
   MVP, sin control de ocupación por recurso.
6. **Facturación electrónica DIAN:** fuera de alcance; la factura
   impresa/POS actual es suficiente por ahora.
7. **Precios iniciales:** se siembra una **propuesta de precios de
   referencia** en la migración 0025; la clínica los ajusta desde
   Configuración → Servicios antes de operar.

---

## 13. Primeras tareas concretas (siguiente PR)

1. `0025_service_catalog.sql` + seed de categorías y servicios con precios de
   referencia.
2. `models/service.rs`, `repositories/services.rs`, `commands/services.rs` con
   tests de repositorio.
3. Registrar en `lib.rs`, `models/mod.rs`, `repositories/mod.rs`,
   `commands/mod.rs`; regenerar `bindings.ts`.
4. `src/hooks/queries/use-services.ts` y `src/lib/api.ts`.
5. `ServicesPage` + entrada en Sidebar/TopBar/App.
6. `NewInvoiceDialog`: selector de catálogo con precio/IVA.
7. `cargo test` + `npm test` + `npm run lint` + `npm run build`.
