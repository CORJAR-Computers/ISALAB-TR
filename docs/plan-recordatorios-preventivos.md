# Plan de implementación — Motor de recordatorios y protocolos preventivos

**Estado:** propuesta (no implementada)
**Base:** schema 0035 · `VACCINES.NEXT_DOSE_AT`, `GROOMING_SESSIONS.NEXT_SUGGESTED_AT`, `STAYS`, SMTP (`mail.rs`) y `wa.me` (`src/lib/whatsapp.ts`) ya existen.
**Objetivo:** convertir datos que hoy solo se muestran (refuerzos vencidos, próximo baño, estancia abierta) en una **bandeja diaria de trabajo** con contacto registrado, mensajes listos y protocolos preventivos por especie/edad.

---

## 1. Alcance y principios

### 1.1 Qué entra

| Pieza | Contenido |
| --- | --- |
| Bandeja diaria | Recordatorios vencidos, de hoy y próximos; filtros y contadores |
| Registro de contactos | Cada llamada/WhatsApp/email con resultado, usuario y fecha |
| Origen de datos | Vacunas, peluquería, estancias y protocolos preventivos |
| Email | Envío por SMTP con plantilla y registro `SENT`/`FAILED` |
| WhatsApp | Enlace `wa.me` con mensaje renderizado; **no hay envío automático** |
| Protocolos | Plantillas por especie/edad con pasos, intervalo y próxima dosis |
| Agendado | Crear cita desde el recordatorio y marcarlo `AGENDADO` |

### 1.2 Qué NO entra (MVP)

- WhatsApp Business API / SMS / llamadas automáticas.
- Envío masivo o campañas de marketing automáticas.
- Portal del tutor o agendamiento online.
- Recordatorios de pago/cartera (otra iniciativa).

### 1.3 Principios

1. **No duplicar la fuente de verdad**: el recordatorio apunta a la vacuna, sesión de peluquería, estancia o paso de protocolo; no copia la historia clínica.
2. **Generación idempotente**: `DEDUPE_KEY` único + `UPDATE OR INSERT`; ejecutar `refresh` cien veces no crea duplicados.
3. **Nada se envía solo**: email y WhatsApp se disparan desde la bandeja con confirmación humana. Se registra el intento, no se promete entrega.
4. **El veterinario manda en lo clínico**: los protocolos vienen precargados como sugerencia editable; la clínica los revisa antes de usarlos.
5. **Reutilizar infraestructura**: migraciones con generadores/triggers, `mail::send_email`, `sendWhatsAppMessage`, auditoría y el patrón de invalidación de TanStack Query.

---

## 2. Modelo de datos

Migración **`0039_reminders.sql`**, que incluye esquema y seed en el mismo archivo.
Orden acordado: abonos/caja (`0036`), receta (`0037`–`0038`) y recordatorios (`0039`).
Si cambia el orden de implementación, renumerar los tres planes antes de crear la migración.

### 2.1 Protocolos

```sql
CREATE TABLE CARE_PROTOCOLS (
    ID          D_PK NOT NULL PRIMARY KEY,
    CODE        D_CODE NOT NULL UNIQUE,        -- CACHORRO_CANINO, ADULTO_FELINO…
    NAME        D_NAME NOT NULL,
    SPECIES_ID  D_PK REFERENCES SPECIES (ID) ON DELETE SET NULL, -- NULL = todas
    DESCRIPTION D_NOTES,
    ACTIVE      BOOLEAN DEFAULT TRUE NOT NULL,
    CREATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT  TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE PROTOCOL_STEPS (
    ID              D_PK NOT NULL PRIMARY KEY,
    PROTOCOL_ID     D_PK NOT NULL REFERENCES CARE_PROTOCOLS (ID) ON DELETE CASCADE,
    SEQ             INTEGER DEFAULT 0 NOT NULL,
    TITLE           D_NAME NOT NULL,           -- "Refuerzo polivalente", "Desparasitación"
    -- VACUNA | DESPARASITACION | CONTROL | ESTETICA | OTRO
    KIND            VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'CONTROL' NOT NULL,
    VACCINE_TYPE_ID D_PK REFERENCES VACCINE_TYPES (ID) ON DELETE SET NULL,
    SERVICE_ID      D_PK REFERENCES SERVICES (ID) ON DELETE SET NULL,
    AGE_MIN_MONTHS  INTEGER DEFAULT 0 NOT NULL,  -- edad de la primera dosis
    INTERVAL_MONTHS INTEGER,                     -- NULL = una sola vez
    LEAD_DAYS       INTEGER DEFAULT 14 NOT NULL, -- antelación del aviso
    NOTES           D_NOTES,
    UNIQUE (PROTOCOL_ID, SEQ)
);

-- Inscripción del paciente en un protocolo (se crea de forma perezosa).
CREATE TABLE PATIENT_PROTOCOLS (
    ID           D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID   D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    PROTOCOL_ID  D_PK NOT NULL REFERENCES CARE_PROTOCOLS (ID) ON DELETE CASCADE,
    STARTED_AT   DATE DEFAULT CURRENT_DATE NOT NULL,
    ACTIVE       BOOLEAN DEFAULT TRUE NOT NULL,
    UNIQUE (PATIENT_ID, PROTOCOL_ID)
);
```

> No se crea una tabla de “pasos cumplidos”: el propio recordatorio completado es el
> registro de cumplimiento. Así no hay dos verdades ni riesgo de desincronización.

### 2.2 Recordatorios y contactos

```sql
CREATE TABLE REMINDERS (
    ID               D_PK NOT NULL PRIMARY KEY,
    PATIENT_ID       D_PK NOT NULL REFERENCES PATIENTS (ID) ON DELETE CASCADE,
    OWNER_ID         D_PK NOT NULL REFERENCES OWNERS (ID),
    -- VACUNA | PELUQUERIA | ESTANCIA | PROTOCOLO | MANUAL
    KIND             VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    TITLE            D_NAME NOT NULL,
    DETAIL           D_NOTES,
    DUE_AT           DATE NOT NULL,
    -- PENDIENTE | CONTACTADO | AGENDADO | COMPLETADO | DESCARTADO
    STATUS           VARCHAR(20) CHARACTER SET UTF8 DEFAULT 'PENDIENTE' NOT NULL,
    -- Origen: VACCINE | GROOMING_SESSION | STAY | PROTOCOL_STEP | MANUAL
    SOURCE_KIND      VARCHAR(24) CHARACTER SET UTF8,
    SOURCE_ID        INTEGER,
    PROTOCOL_STEP_ID D_PK REFERENCES PROTOCOL_STEPS (ID) ON DELETE SET NULL,
    APPOINTMENT_ID   D_PK REFERENCES APPOINTMENTS (ID) ON DELETE SET NULL,
    -- Clave estable y única por origen+fecha: VAC:12 · GRM:45 · STY:9:2026-11-20
    -- · PRT:3:7:2026-11-20
    DEDUPE_KEY       VARCHAR(80) CHARACTER SET ASCII NOT NULL UNIQUE,
    LAST_CONTACT_AT  TIMESTAMP,
    CONTACT_ATTEMPTS INTEGER DEFAULT 0 NOT NULL,
    COMPLETED_AT     TIMESTAMP,
    COMPLETED_BY     D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    NOTES            D_NOTES,
    CREATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UPDATED_AT       TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IX_REMINDERS_STATUS_DUE ON REMINDERS (STATUS, DUE_AT);
CREATE INDEX IX_REMINDERS_PATIENT ON REMINDERS (PATIENT_ID);
CREATE INDEX IX_REMINDERS_KIND ON REMINDERS (KIND);

CREATE TABLE REMINDER_CONTACTS (
    ID           D_PK NOT NULL PRIMARY KEY,
    REMINDER_ID  D_PK NOT NULL REFERENCES REMINDERS (ID) ON DELETE CASCADE,
    -- LLAMADA | WHATSAPP | EMAIL | SMS | PRESENCIAL
    CHANNEL      VARCHAR(16) CHARACTER SET UTF8 NOT NULL,
    -- SIN_RESPUESTA | CONTACTADO | AGENDO | NO_INTERESADO | NUMERO_INVALIDO | OTRO
    OUTCOME      VARCHAR(20) CHARACTER SET UTF8 NOT NULL,
    RECIPIENT    VARCHAR(120) CHARACTER SET UTF8,
    NOTES        D_NOTES,
    -- SENT | FAILED | NOT_APPLICABLE (email); NULL para WhatsApp/llamada
    DELIVERY     VARCHAR(16) CHARACTER SET UTF8,
    CREATED_BY   D_PK REFERENCES USERS (ID) ON DELETE SET NULL,
    CREATED_AT   TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IX_REMINDER_CONTACTS_REMINDER ON REMINDER_CONTACTS (REMINDER_ID, CREATED_AT);
```

### 2.3 Plantillas

```sql
CREATE TABLE REMINDER_TEMPLATES (
    ID      D_PK NOT NULL PRIMARY KEY,
    CODE    D_CODE NOT NULL,          -- VACUNA, PELUQUERIA, ESTANCIA, PROTOCOLO
    CHANNEL VARCHAR(12) CHARACTER SET UTF8 NOT NULL, -- EMAIL | WHATSAPP
    SUBJECT VARCHAR(160) CHARACTER SET UTF8,
    BODY    D_TEXT NOT NULL,
    ACTIVE  BOOLEAN DEFAULT TRUE NOT NULL,
    UNIQUE (CODE, CHANNEL)
);
```

Placeholders soportados: `{tutor}`, `{paciente}`, `{fecha}`, `{titulo}`, `{detalle}`,
`{clinica}`, `{telefono}`, `{direccion}`. El render ocurre en Rust para email y en
un comando para WhatsApp, con la misma plantilla.

### 2.4 Estancia: fecha esperada de salida

Hoy `STAYS` no guarda cuándo debe retirarse el paciente, así que no se puede avisar el retiro.

```sql
ALTER TABLE STAYS ADD EXPECTED_CHECK_OUT_AT TIMESTAMP;
```

Se expone en `CreateStayInput`, en el diálogo de ingreso y en la ficha de la estancia.
Es opcional: sin fecha no se genera recordatorio de retiro.

### 2.5 Seed (incluido en `0039`)

- Plantillas email + WhatsApp para `VACUNA`, `PELUQUERIA`, `ESTANCIA`, `PROTOCOLO`.
- Protocolos sugeridos, editables:
  - **Cachorro canino**: polivalente 2/3/4 meses + refuerzo anual, rabia anual, desparasitación trimestral.
  - **Gatito felino**: TCRP 2/3/4 meses + anual, rabia anual, FeLV según riesgo.
  - **Adulto canino/felino**: refuerzo anual y desparasitación semestral.
  - **Sénior (≥ 8 años)**: control clínico + perfil de laboratorio anual.
- Cada paso referencia `VACCINE_TYPES` o `SERVICES` existentes; si no hay servicio,
  queda como `CONTROL` con título libre.

---

## 3. Generación de recordatorios

### 3.1 Algoritmo (`refresh_reminders`)

Se ejecuta al abrir la bandeja, al iniciar la app y tras registrar vacuna/sesión/estancia
(no hace falta un daemon). Es idempotente por `DEDUPE_KEY`.

```text
para cada vacuna con NEXT_DOSE_AT <= hoy + lead:
    upsert REMINDERS  key=VAC:{vaccine_id}  due=NEXT_DOSE_AT
                      kind=VACUNA  source=VACCINE:{id}
                      title="Refuerzo: {VACCINE_NAME}"

para cada sesión de peluquería con NEXT_SUGGESTED_AT <= hoy + lead:
    upsert  key=GRM:{session_id}  due=NEXT_SUGGESTED_AT
            kind=PELUQUERIA  source=GROOMING_SESSION:{id}
            title="Próximo turno: {service_name}"

para cada estancia ABIERTA con EXPECTED_CHECK_OUT_AT no nulo
     y EXPECTED_CHECK_OUT_AT <= hoy + 1:
    upsert  key=STY:{stay_id}:{fecha}  due=EXPECTED_CHECK_OUT_AT
            kind=ESTANCIA  title="Retiro de {kind}"

para cada paciente ACTIVE × protocolo ACTIVO de su especie × paso ACTIVO:
    si el paciente no está inscrito → inscribir PATIENT_PROTOCOLS (ACTIVE)
    due = primera vez: BIRTH_DATE + AGE_MIN_MONTHS
          siguientes: COMPLETED_AT del recordatorio anterior + INTERVAL_MONTHS
    si due <= hoy + LEAD_DAYS:
        upsert  key=PRT:{patient_id}:{step_id}:{due}
                kind=PROTOCOLO  source=PROTOCOL_STEP:{id}
                title="{step.title}"
```

Reglas adicionales:

- Ignorar pacientes `ACTIVE = FALSE`.
- No generar si el paciente ya tiene un recordatorio del mismo `SOURCE_KIND` y
  `SOURCE_ID` en estado `PENDIENTE/CONTACTADO/AGENDADO` (cubierto por la clave).
- Si la fuente desaparece (vacuna eliminada, sesión borrada), marcar el recordatorio
  `DESCARTADO` con nota `Fuente eliminada` en el siguiente refresh.
- Fechas vencidas se generan con `DUE_AT` original (muestran días de atraso), no con la
  fecha de hoy.
- `refresh` devuelve un resumen (`creados`, `actualizados`, `descartados`) para mostrar
  “N recordatorios actualizados”.

### 3.2 Completar y recurrir

- `COMPLETADO` guarda `COMPLETED_AT/COMPLETED_BY`. En el siguiente refresh, un paso
  recurrente programa su próxima ocurrencia (`completed_at + interval_months`).
- Un recordatorio de vacuna se completa solo cuando se registra una vacuna nueva con
  `NEXT_DOSE_AT` posterior (o manualmente desde la bandeja).
- `DESCARTADO` no vuelve a generarse para esa misma clave.

---

## 4. Backend

### 4.1 Archivos

```text
src-tauri/src/models/reminder.rs        Reminder, ReminderListItem, ReminderContact,
                                        ReminderTemplate, ReminderFilter, inputs
src-tauri/src/models/care_protocol.rs   CareProtocol, ProtocolStep, inputs
src-tauri/src/repositories/reminders.rs Generación, consultas, estados, contactos
src-tauri/src/repositories/protocols.rs CRUD de protocolos y pasos
src-tauri/src/commands/reminders.rs
src-tauri/src/commands/protocols.rs
src-tauri/src/repositories/stays.rs     (+ EXPECTED_CHECK_OUT_AT en SELECT/INSERT)
src-tauri/src/models/stay.rs            (+ expected_check_out_at)
```

Registrar en `models/mod.rs`, `repositories/mod.rs`, `commands/mod.rs` y `lib.rs`
(`collect_commands!` + `.typ::<>`). Regenerar `src/bindings.ts`.

### 4.2 Comandos

| Comando | Capacidad | Descripción |
| --- | --- | --- |
| `list_reminders(filter)` | sesión | Filtros: estado, tipo, rango de fechas, búsqueda, paciente |
| `refresh_reminders()` | sesión | Generación idempotente; devuelve resumen |
| `get_reminder(id)` | sesión | Detalle + fuente + contactos |
| `set_reminder_status(id, status, notes)` | recepción/vet | Transiciones validadas |
| `log_reminder_contact(input)` | recepción/vet | Canal, resultado, nota; actualiza último contacto e intentos |
| `list_reminder_contacts(reminder_id)` | sesión | Historial |
| `render_reminder_message(id, channel)` | sesión | Plantilla + datos reales |
| `send_reminder_email(id)` | recepción/vet | SMTP + registro `SENT/FAILED` (nunca silencioso) |
| `schedule_reminder_appointment(id, input)` | recepción | Crea cita, enlaza `APPOINTMENT_ID`, estado `AGENDADO` |
| `list_patient_reminders(patient_id)` | sesión | Plan preventivo del paciente |
| `list_care_protocols()`, `save_care_protocol()`, `save_protocol_step()`, `delete_protocol_step()`, `set_protocol_active()` | admin | Configuración |
| `list_reminder_templates()`, `save_reminder_template()` | admin | Plantillas |

Transiciones permitidas:

```text
PENDIENTE → CONTACTADO → AGENDADO → COMPLETADO
     └────────→ DESCARTADO ←──────────┘ (desde cualquier estado previo)
```

Cada transición y cada contacto se registran en `USER_AUDIT_LOG`
(`REMINDER_CONTACTED`, `REMINDER_SCHEDULED`, `REMINDER_COMPLETED`, `REMINDER_DISCARDED`).

### 4.3 Email

Reutilizar `mail::send_email` y `settings_repo::get` como en `send_critical_email`:

1. Validar que el propietario tenga email.
2. Renderizar plantilla (`{tutor}`, `{paciente}`, `{fecha}`…).
3. Enviar; guardar `REMINDER_CONTACTS` con `CHANNEL='EMAIL'` y `DELIVERY='SENT'|'FAILED'`.
4. Devolver el contacto para que la UI muestre el resultado real.
5. Si falla el SMTP, no cambiar el estado del recordatorio: se informa y se puede reintentar.

La llamada SMTP puede tardar; ejecutarla fuera del hilo de UI (comando `async` con
`spawn_blocking` o el patrón que ya use el proyecto).

### 4.4 WhatsApp

- `render_reminder_message(id, 'WHATSAPP')` devuelve el texto.
- El frontend usa `sendWhatsAppMessage(owner.phone, texto)` (ya existe y está probado).
- Después de abrir el enlace se abre un pequeño diálogo para registrar el resultado
  (`CONTACTADO`, `AGENDO`, `SIN_RESPUESTA`…). Abrir `wa.me` **no** se registra como
  entregado: no hay confirmación técnica.

---

## 5. Frontend

### 5.1 Navegación

- `View`: `"reminders"`. Entrada en `Sidebar` (icono `BellRing`) y `TopBar`.
- Badge opcional con `vencidos + hoy` obtenido del resumen del dashboard.
- Ruta lazy en `App.tsx`.

### 5.2 Bandeja (`src/features/reminders/`)

```text
RemindersPage.tsx        Tabs: Vencidos · Hoy · Próximos 7 días · Todos
ReminderRow.tsx          Fecha + días de atraso, paciente, tutor, tipo, estado, último contacto
ReminderDetailDialog.tsx Fuente, historial de contactos, notas, acciones
ContactDialog.tsx        Canal, resultado, nota; email con confirmación de envío
ScheduleFromReminder.tsx Reutiliza NewAppointmentDialog con paciente/servicio precargados
ProtocolsCard.tsx        Plan preventivo del paciente (ficha clínica)
```

Acciones por fila: **Llamar** (`tel:`), **WhatsApp**, **Email**, **Agendar**,
**Registrar contacto**, **Completar**, **Descartar**.

Filtros: tipo (vacuna/peluquería/estancia/protocolo), estado, especie, rango de fechas y
búsqueda por paciente/tutor/teléfono. Contadores por tab.

### 5.3 Configuración

- Sección **Protocolos preventivos**: protocolos por especie, pasos ordenados con edad,
  intervalo, servicio/vacuna y días de antelación. Solo ADMIN.
- Sección **Plantillas de recordatorio**: asunto/cuerpo por tipo y canal, con vista previa
  de placeholders. Solo ADMIN.

### 5.4 Otros módulos

- `ClinicalHistoryPage`/`PatientCard`: “Plan preventivo” con próximos pasos y botón
  “Crear recordatorio”.
- `DashboardPage`: tarjetas de vencidos, para hoy, contactados y agendados esta semana.
- `GroomingPage`: el bloque de recordatorios actual se reemplaza por un enlace a la
  bandeja filtrada por `PELUQUERIA` (o se mantiene como vista rápida, sin duplicar lógica).
- `StaysPage`: campo “salida prevista” en el ingreso y edición; aviso si la estancia está
  vencida.

---

## 6. Fases y estimación

| Fase | Entregable | Estimación |
| --- | --- | --- |
| **A. Esquema y generación** | 0039 (esquema + seed), modelos, repositorios, `refresh_reminders`, fuentes vacuna/peluquería/estancia, tests | 3–4 días |
| **B. Bandeja y contactos** | Vista Recordatorios, filtros, detalle, registro de contactos, WhatsApp y email, agendar | 4–5 días |
| **C. Protocolos** | Protocolos/pasos, inscripción del paciente, recurrencia, plan en la ficha | 5–6 días |
| **D. Cierre** | Dashboard, plantillas editables, auditoría, E2E y pulido | 2–3 días |

Total MVP: **~3 semanas**, en commits por fase con migración y CI verde.

---

## 7. Criterios de aceptación

### Fase A

- [ ] Con una vacuna `NEXT_DOSE_AT` vencida, `refresh_reminders` crea un único recordatorio;
      ejecutarlo de nuevo no duplica.
- [ ] Una sesión de peluquería con `NEXT_SUGGESTED_AT` genera su recordatorio.
- [ ] Una estancia abierta con salida prevista genera aviso; sin fecha, no.
- [ ] Un paciente inactivo no genera recordatorios.
- [ ] Si se elimina la fuente, el recordatorio se descarta en el siguiente refresh.
- [ ] Migración probada desde una BD en 0035.

### Fase B

- [ ] La bandeja muestra vencidos/hoy/próximos con días de atraso y contadores.
- [ ] Registrar una llamada guarda canal, resultado, usuario y actualiza último contacto.
- [ ] Enviar email registra `SENT` o `FAILED` y muestra el resultado real.
- [ ] WhatsApp abre el mensaje renderizado y permite registrar el desenlace.
- [ ] Agendar desde el recordatorio crea la cita y deja el recordatorio `AGENDADO`.
- [ ] Recepción puede contactar/agendar, pero no editar protocolos.

### Fase C

- [ ] Un cachorro canino recibe el plan con pasos a 2/3/4 meses y refuerzo anual.
- [ ] Al completar un paso recurrente aparece el siguiente con la fecha calculada.
- [ ] Los pasos se pueden editar sin afectar recordatorios históricos.
- [ ] La ficha del paciente muestra su plan y próximos pasos.

### Fase D

- [ ] El panel muestra vencidos, para hoy, contactados y agendados.
- [ ] Las plantillas se editan desde Configuración y se reflejan en email/WhatsApp.
- [ ] Cada transición y contacto queda auditado.

---

## 8. Pruebas

| Nivel | Casos |
| --- | --- |
| Rust (Firebird real) | Idempotencia y `DEDUPE_KEY`; generación desde vacuna/peluquería/estancia; exclusión de inactivos; recurrencia de pasos; transiciones válidas; email fallido registra `FAILED`; RBAC (recepción vs admin); refresh concurrente sin duplicados. |
| Frontend (Vitest) | Render de placeholders; etiquetas/colores de estado; cálculo de días de atraso; filtros y contadores; URL de WhatsApp (ya existe). |
| E2E (Playwright) | Vacuna vencida → aparece en bandeja → registrar WhatsApp → agendar → estado `AGENDADO` → completar. Estancia con salida prevista. Protocolo de cachorro. |

No se añaden dependencias nuevas: email usa `lettre`, WhatsApp usa el helper existente.

---

## 9. Riesgos y decisiones

| Riesgo | Mitigación |
| --- | --- |
| Duplicados por ejecuciones concurrentes | Índice único `DEDUPE_KEY` + `UPDATE OR INSERT`; probar dos refresh simultáneos. |
| WhatsApp no confirma entrega | Registrar solo el intento/resultado; nunca marcar enviado automáticamente. |
| Protocolos con indicaciones clínicas incorrectas | Seed marcado como sugerencia editable; validación del veterinario antes de activar en producción. |
| Correos masivos no deseados | Envío manual desde la bandeja; sin campañas ni automatismos en el MVP. |
| Crecimiento de la tabla | Índices por estado/fecha; purga opcional de `DESCARTADO` con más de 24 meses. |
| Fechas y zona horaria | Usar `CURRENT_DATE` de Firebird y fechas locales; no comparar timestamps UTC. |
| Pacientes fallecidos | Hoy solo existe `ACTIVE`; se documenta desactivar al paciente. Un campo `DECEASED_AT` queda para otra fase. |
| Estancias sin salida prevista | Campo opcional; sin fecha no hay aviso de retiro. |

### Decisiones abiertas

1. ¿La clínica quiere los protocolos precargados o parte de una lista vacía? (Recomendado: precargados y editables).
2. ¿El aviso de retiro debe generarse 1 día antes, el mismo día o ambos? (Recomendado: ambos).
3. ¿La bandeja debe cargar recordatorios de pacientes inactivos para reactivarlos? (Recomendado: no en el MVP).
4. ¿Se permite completar un recordatorio de vacuna sin registrar la vacuna? (Recomendado: sí, con nota y auditoría).

---

## 10. Orden de implementación (primer PR)

1. `0039_reminders.sql` (esquema + seed) y registro en `db/migrations.rs`.
2. `models/reminder.rs` + `models/care_protocol.rs`; `repositories/reminders.rs` con generación de vacuna y peluquería; tests.
3. `commands/reminders.rs` (`list`, `refresh`, `set_status`), registro en `lib.rs`; regenerar bindings.
4. `use-reminders.ts`, `api.ts`, vista mínima `RemindersPage` con tabs y contadores.
5. Estancia con `EXPECTED_CHECK_OUT_AT` y su fuente de recordatorio.
6. Contactos + WhatsApp/email en el PR siguiente.
