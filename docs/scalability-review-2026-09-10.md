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
| H9 | Corregir premisa FB4 de la 0023: FK sí auto-indexan en FB5 | S | escritura más ligera | ✅ hecho (0024, ver §5) |

H1 + H2 + H4 quedaron implementados (migración `0023_maintenance_indexes`,
`db/maintenance.rs` y el tope en `AiCache::set`). H3, H5 y H6 también
cerrados: las únicas queries por-fila que quedan son el SP de validación y
el upsert en `register_results_batch` (inherentes: cada resultado necesita
su propia llamada al SP y su fila), y el único intervalo que queda en la UI
es el del worklist (60 s, justificado: el tiempo transcurrido avanza con el
reloj). **Todos los hallazgos de la revisión quedan resueltos.**

---

## 5. Perfilado con EXPLAIN PLAN sobre ~100k resultados (2026-09-10)

Fixture: base temporal generada con `src-tauri/src/bin/profile_seed.rs` —
2 000 pacientes, 3 400 muestras en ~3 años, **102 000 LAB_RESULTS** (30
analitos por muestra, catálogo real de 0021), 105 400 EVENT_LOG, estadísticas
frescas. Firebird **5.0.3** embebido, planes capturados con `SET PLAN ON` +
`SET STATS ON` en isql. Tres queries de producción, verbatim:

| Query | Plan elegido | Tiempo | Fetches |
|---|---|---|---|
| `list_results` (30 filas) | `R INDEX (RDB$39)` — UNIQUE(SAMPLE_ID, ANALYTE_ID) | **0.010 s** | 1 409 |
| delta en lote (`ROW_NUMBER`, 30 analitos) | `R INDEX (RDB$FOREIGN36)`, `S INDEX (RDB$FOREIGN28)` | 0.20–0.24 s | ~103 000 |
| worklist (680 pendientes de 3 400) | subqueries `LR INDEX (RDB$FOREIGN35)`, hash join | 0.31 s | 110 028 |

La suposición "las FK de Firebird no crean índice" (base de H4) es cierta en
FB≤4 pero **falsa en Firebird 5**: el optimizador usó exclusivamente los
índices de sistema de las FK (`RDB$FOREIGN36` = LAB_RESULTS.ANALYTE_ID,
`RDB$FOREIGN28` = SAMPLES.PATIENT_ID, `RDB$FOREIGN35` =
LAB_RESULTS.SAMPLE_ID). El A/B quitando los cuatro índices de la 0023 dio
**planes y tiempos idénticos** (0.010/0.20/0.31 s en ambas copias): nunca
eligió `IX_LAB_RESULTS_ANALYTE` ni `IX_SAMPLES_PATIENT`. Tampoco eligió
`IX_LAB_RESULTS_ANALYZED_AT` para el `ORDER BY r.ANALYZED_AT DESC ROWS 20`
del prompt de IA (prefiere R NATURAL + SORT: el índice de una sola columna
no cubre el filtro por paciente). Y una variante del delta que sondea por
SAMPLE_ID fue peor (0.81 s, 515k fetches): el forma original con índice de
FK por ANALYTE_ID es la correcta.

**Corrección aplicada (migración 0024):** se eliminan
`IX_LAB_RESULTS_ANALYTE` e `IX_SAMPLES_PATIENT` (duplicados exactos de los
índices de sistema de sus FK: solo coste de escritura sin ningún plan que
los use). Se conservan `IX_EVENT_LOG_CREATED_AT` (soporta la poda/consultas
por fecha; el prune sobre 102k filas corre en ~1.3 s con o sin índice porque
toca el 90 % de la tabla, pero el índice evita el scan cuando el retenedor
sea menor) e `IX_LAB_RESULTS_ANALYZED_AT` (único orden por fecha de
análisis disponible para volúmenes mayores). La corrección del review:
H4 no era "falta índices" sino "FB5 ya los trae"; el hallazgo real de
escritura es que la 0023 añadió dos de más.

**Lectura de capacidad:** con 3 años de datos en una laptop, la ficha de
una muestra abre en ~10 ms, la historia clínica de un paciente pesado
(~3 400 resultados vía list_samples) queda dominada por los 0.2–0.3 s de
los lotes delta/adjuntos ya optimizados, y la bandeja de trabajo completa
en 0.31 s. El motor tiene margen sobrado para el horizonte de decenas de
miles de resultados por instalación; no se requiere acción adicional.
