# Revisión de escalabilidad — ISALAB (con la función de rangos definidos por el veterinario)

> Fecha: 2026-09-10 · Alcance: pool de conexiones, cachés, invalidaciones,
> consultas, índices y la nueva función de rangos manuales (migración 0022).

## Contexto de la aplicación

ISALAB es una app de escritorio **monousuario (Tauri + Firebird Embedded)**
para laboratorios veterinarios. El horizonte de datos razonable es de
**decenas de miles de resultados por instalación** (≈50–150 muestras/día ×
20–40 analitos × años). La "escalabilidad" relevante no es de concurrencia
multiusuario sino de: crecimiento de datos sin degradación, memoria acotada
en sesiones largas, y tiempos de arranque de pantalla estables. Con ese
criterio se revisó el código.

---

## 1. La nueva función (0022) y su impacto en escalabilidad

**Diseño elegido (bien para escalar):** el rango manual vive en el propio
`LAB_RESULTS` (columnas `CUSTOM_REF_MIN/MAX`) y **no** crea filas en
`REFERENCE_RANGES`. Consecuencias positivas:

- El catálogo (44 analitos × 124+ rangos sembrados) no crece con el uso: los
  rangos manuales escalan linealmente con los resultados, que ya crecen de
  todos modos. Sin explosión de filas en el catálogo ni basura de rangos
  one-off que habría que limpiar.
- La validación sigue siendo una sola llamada al SP (el SP recibe los límites
  como parámetros), es decir **cero queries extra** por resultado guardado.
- El `CHECK (min <= max)` en BD garantiza integridad aunque se guarde por
  importación CSV.
- Los rangos abiertos (solo min o solo max) están soportados de punta a punta
  (SP, UI, PDF, CSV, timeline).

**Riesgo asumido y mitigado:** al editar el catálogo futuro no se revalidan
resultados antiguos con rango manual — correcto clínicamente: el rango que
validó el veterinario debe conservarse tal cual, y el PDF imprime la nota
"Rango de referencia definido por el veterinario" para transparencia.

---

## 2. Hallazgos (ordenados por impacto)

### H1 — `AiCache` sin tope de tamaño ni limpieza en producción 🔴

`src-tauri/src/ai_cache.rs`: `HashMap<i32, CacheEntry>` en memoria, TTL 24 h.
Problemas:

- `cleanup()` **existe pero nadie lo llama** (solo los tests).
- No hay límite de entradas. En una sesión larga con muchas muestras, el mapa
  crece sin acotar (una interpretación por muestra, texto LLM que puede ser
  de varios KB cada una).

Con uso intensivo (p. ej. 300 interpretaciones × 8 KB ≈ 2.4 MB) no es un
desastre, pero es memoria que **nunca** se libera en la sesión. La API ya lo
prevé (`cleanup` escrito y probado); falta conectarlo.

**Recomendación (S):** llamar `cleanup()` al comienzo de `get()`/`set()`
(barato: O(n) con n acotado), y opcionalmente un tope blando (p. ej. 500
entradas, evictando las más viejas). Una línea en `set`:

```rust
// dentro de set(), antes de insertar:
entries.retain(|_, e| e.created_at.elapsed() <= self.ttl);
```

### H2 — `EVENT_LOG` crece sin poda 🔴

Cada insert/update de `SAMPLES` o `LAB_RESULTS` escribe una fila en
`EVENT_LOG` (triggers AI_SAMPLES / AI_LAB_RESULTS). Nadie la borra nunca. A
5 años × 100 muestras/día × ~35 analitos son **~6–13 M de filas** de
telemetría que ya nadie lee (el listener solo lee `FIRST 1` por evento
recibido). Firebird lo aguanta, pero:

- Cada backup crece innecesariamente (y los backups locales son parte del
  producto: `create_local_backup`).
- `SELECT FIRST 1 ... ORDER BY` sobre la tabla (events.rs) se apoya en el
  natural order — con millones de filas el índice/natural scan se encarece.

**Recomendación (S):** migración de mantenimiento que borre lo viejo, p. ej.
al arrancar: `DELETE FROM EVENT_LOG WHERE CREATED_AT < CURRENT_TIMESTAMP - 30`
(y un índice sobre `CREATED_AT`). Alternativa: podar en el mismo trigger con
probabilidad 1/1000 (menos limpio).

### H3 — `delta_variation` y `list_results` hacen N+1 queries por muestra 🟡

`list_results()` (samples.rs) hacía, **por cada resultado**: 1 query de
adjuntos + 1 query de delta check (`delta_variation`), más otra vuelta igual
en `register_lab_result`. Para la ficha de una muestra con 30 analitos eran
~61 queries — imperceptible con Firebird embedded local (latencia de
proceso, no de red), pero era el patrón que más crecía con los datos.
El historial clínico (`list_samples`) repite ese patrón por cada muestra del
paciente.

**Resolución (hecha):** adjuntos en una sola query agrupada
(`list_for_results_of_sample`) y delta check en una sola query con
`ROW_NUMBER()` (`delta_variations`, partición por analito con el mismo
orden de desempate que el original). Una muestra de 30 analitos pasó de
~61 a **3 queries** (resultados + adjuntos + delta); un paciente con 40
muestras, de ~2.400 a ~122. `delta_variation` (single) queda como
envoltorio sobre la versión en lote — un solo camino SQL.

### H4 — Índices faltantes para las rutas de consulta principales 🟡

Solo hay 6 índices explícitos en 22 migraciones. Los siguientes accesos
hacen scan natural y degradan con el volumen:

- `LAB_RESULTS.ANALYTE_ID` y `LAB_RESULTS.ANALYZED_AT` — tendencias por
  paciente/analito, delta check, "resultados previos" del prompt IA
  (`WHERE s.PATIENT_ID = ? AND r.ANALYTE_ID = ? ... ORDER BY r.ANALYZED_AT DESC`).
- `SAMPLES.PATIENT_ID` — historial clínico por paciente (¿lo cubre la FK? En
  Firebird **las FK no crean índice automáticamente**).
- `EVENT_LOG.CREATED_AT` (ver H2) y `EVENT_LOG.EVENT_NAME`.

**Recomendación (S):** migración 0023 con
`CREATE INDEX IX_LAB_RESULTS_PATIENT_ANALYTE ON LAB_RESULTS (ANALYTE_ID)` +
`IX_SAMPLES_PATIENT (PATIENT_ID)` + `IX_EVENT_LOG_CREATED (CREATED_AT)`.
Barato en escritura (monousuario) y transforma los scans en búsquedas
indexadas.

### H5 — Invalidaciones de react-query muy anchas 🟡

`use-firebird-events.ts` y las mutaciones invalidan `["samples"]`,
`["clinical-history"]`, `["worklist"]`, `["dashboard"]` **sin argumentos**:
cada evento de un resultado re-fetchea todos los listados montados. Con los
eventos Firebird llegando por cada analito guardado (el batch de 30
dispara 30 eventos), la UI revalida varias veces seguidas.

**Recomendación (M):** agrupar (debounce ~300 ms) las invalidaciones de un
ráfaga de eventos, y usar claves parciales (`["samples", { status }]`) para
no tirar listados no montados. react-query ya deduplica refetches
simultáneos; el win real es evitar la ráfaga de 30→1.

### H6 — `refetchInterval` fijo de 30–60 s 🟢 (menor)

Dashboard (30 s) y worklist (60 s) hacían polling con los eventos Firebird
ya activos.

**Resolución (hecha):** el polling del dashboard (30 s) se eliminó: las
mutaciones de la UI ya invalidaban `["dashboard"]`, y ahora los eventos
Firebird también (cubre la única vía que quedaba sin mutación: la
importación por carpeta vigilada del analizador en segundo plano). El
worklist **conserva** su refresco de 60 s a propósito: muestra el tiempo
desde la recepción, que avanza con el reloj sin que cambie nada en la BD —
ningún evento lo empuja, y la columna de urgencia quedaría congelada sin
el intervalo.

### H7 — CSV/PDF grandes en memoria 🟢 (menor)

`results_to_csv` y los PDF construyen un `String` completo en memoria. Para
exportaciones de 100k filas (~20 MB) está bien en desktop; si algún día se
 exporta a disco en streaming, usar `BufWriter` sobre el archivo. No urgente.

### H8 — Pool de 4 conexiones es correcto para monousuario ✅

`POOL_SIZE = 4` con `SimpleConnection` reciclado por Drop es adecuado para
la app (los comandos Tauri son cortos). Los listeners de eventos abren sus
propias conexiones dedicadas, fuera del pool — correcto. No tocar.

---

## 3. Qué NO es un problema hoy (verificado)

- **Tamaño de catálogo:** 44 analitos / 124+ rangos — trivial, todo indexado
  por PK. Los rangos manuales no crecen el catálogo (ver §1).
- **SP de validación:** 1 query por resultado, indexada por PK. La resolución
  de rango (equipo → sexo → edad) es un solo SELECT ordenado.
- **Frontend de la grilla:** `displayedAnalytes` y `referenceRanges` están
  memoizados; la evaluación en vivo por tecla es O(analitos visibles).
- **Fotos de resultados** van a disco (`app_data/attachments`), no a la BD.
- **Tests:** la suite (295 Rust + 120 frontend + 14 E2E) corre contra
  Firebird real; el costo de CI no escala con los datos.

## 4. Resumen de acciones recomendadas

| # | Acción | Esfuerzo | Impacto | Estado |
|---|--------|----------|---------|--------|
| H1 | `AiCache`: cleanup en `set()` + tope de entradas | S | memoria acotada | ✅ hecho |
| H2 | Poda de `EVENT_LOG` (>30 días) + índice `CREATED_AT` | S | backups y eventos | ✅ hecho (0023 + `db/maintenance.rs`) |
| H4 | Índices: `LAB_RESULTS(ANALYTE_ID)`, `SAMPLES(PATIENT_ID)` | S | consultas clave | ✅ hecho (0023) |
| H3 | Eliminar N+1 en `list_results` (adjuntos y delta en 1 query c/u) | M | historial grande | ✅ hecho (`list_for_results_of_sample` + `delta_variations` con `ROW_NUMBER`) |
| H5 | Debounce de invalidaciones de eventos Firebird | M | UI bajo ráfagas | ✅ hecho (`use-firebird-events.ts`, ventana 150 ms + dedupe) |
| H6 | Quitar polling redundante si events bastan | S | CPU de fondo | ✅ hecho (dashboard 30 s eliminado; worklist 60 s se conserva: tiempo transcurrido) |

H1 + H2 + H4 quedaron implementados (migración `0023_maintenance_indexes`,
`db/maintenance.rs` y el tope en `AiCache::set`). H3, H5 y H6 también
cerrados: las únicas queries por-fila que quedan son el SP de validación y
el upsert en `register_results_batch` (inherentes: cada resultado necesita
su propia llamada al SP y su fila), y el único intervalo que queda en la UI
es el del worklist (60 s, justificado: el tiempo transcurrido avanza con el
reloj). **Todos los hallazgos de la revisión quedan resueltos.**
