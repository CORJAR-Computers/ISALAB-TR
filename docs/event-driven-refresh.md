# Modelo de refresco de UI dirigido por eventos — y cuándo sondear

> 2026-09-10 · Complementa `docs/scalability-review-2026-09-10.md` (§H5, H6).
> Objetivo: documentar cómo se entera la UI de los cambios de datos y dar una
> regla de decisión explícita para saber cuándo un `refetchInterval` está
> justificado y cuándo es polling redundante.

## 1. La regla

**La base de datos es la fuente de verdad del refresco.** Toda escritura de
muestras/resultados —venga de la UI, de la importación del analizador o de
cualquier código Rust futuro— dispara un evento Firebird que llega a la UI y
invalida las queries afectadas. Por lo tanto:

- Un `refetchInterval` para datos cuya única forma de cambiar es una
  escritura en la BD es **redundante**: lo elimina el evento (H6).
- Un `refetchInterval` para datos que **cambian con el reloj** sin que nada
  cambie en la BD está **justificado**: ningún evento puede empujar el paso
  del tiempo. Es el caso único actual: `useWorklist` (60 s).

Regla de decisión para una query nueva:

1. ¿Sus datos cambian solo cuando alguien escribe en la BD? → sin intervalo;
   confía en eventos + invalidaciones de mutations.
2. ¿Se calcula a partir del reloj (tiempos transcurridos, "hoy", vencimientos
   por hora/minuto)? → intervalo, con la granularidad mínima aceptable para
   la UI (el worklist ordena urgencias por rangos de horas: 60 s sobra).
3. ¿Deriva de datos externos (red, actualizaciones, relojes remotos)? →
   intervalo propio justificado, fuera del alcance de este modelo.

En el código actual queda exactamente un `refetchInterval` (worklist, 60 s)
y un `setInterval` de fondo (re-chequeo del updater cada 4 h): ambos pasan la
regla. Todo lo demás se refresca por eventos o por invalidaciones explícitas
de mutations.

## 2. La tubería completa (backend → UI)

```
INSERT/UPDATE en SAMPLES o LAB_RESULTS            (cualquier origen)
  │
  ├─ Trigger AFTER INSERT OR UPDATE               (0001_initial_schema.sql)
  │    ├─ escribe fila en EVENT_LOG               (trazabilidad + payload)
  │    └─ POST_EVENT 'SAMPLE_CHANGED' | 'LAB_RESULT_CHANGED'
  │
  ├─ Listener dedicado de Firebird                (db/events.rs, 1 conexión
  │    propia por evento, fuera del pool)          lee FIRST 1 de EVENT_LOG
  │    └─ app.emit("sample-changed" |              → evento Tauri con payload
  │              "lab-result-changed", {sampleId, patientId, status})
  │
  └─ useFirebirdEvents()                          (App.tsx, montado 1 vez)
       └─ EventBatcher: acumula 150 ms (FLUSH_WINDOW_MS) en Sets (dedupe
            por paciente/muestra) y hace UN flush por ráfaga:
            samples, sample-counts, worklist, dashboard  ← 1 invalidación c/u
            clinical-history/{patientId}, patient/{patientId},
            sample/{sampleId}                            ← 1 por id distinto
```

Los triggers escriben en `EVENT_LOG` y la poda de `db/maintenance.rs`
mantiene esa tabla en una ventana de 30 días: los eventos son baratos y el
histórico de telemetría no crece sin límite.

## 3. Por qué la agrupación de 150 ms

Guardar un panel de 30 analitos produce 30 eventos `LAB_RESULT_CHANGED` en
ráfaga (uno por trigger). Sin agrupación, cada evento invalidaba todos los
listados montados: ~150 invalidaciones y refetches simultáneos. Con la
ventana:

- la ráfaga completa comparte **un solo flush** (el temporizador se agenda
  al primer evento y no se retrasa: los eventos de la misma ráfaga caen
  dentro de la ventana);
- por cada query afectada hay **una** invalidación por flush;
- los ids repetidos (mismo paciente/muestra) se deduplican en `Set`s.

150 ms es imperceptible para trazabilidad en vivo y sobra para que llegue la
ráfaga completa: los eventos son locales, del mismo proceso. Los tests de
`use-firebird-events.test.tsx` fijan este contrato (ráfaga→1 flush, dedupe,
ventanas separadas, cancelación en unmount).

## 4. Mutations: la otra mitad del modelo

Los eventos cubren lo que la BD sabe. Las mutations de react-query cubren lo
que **esta UI acaba de hacer** y lo hacen con dos ventajas: refresco
inmediato (sin esperar el evento) e invalidación dirigida (incluyen claves
con id concreto: `["sample", id]`, `["clinical-history", patientId]`).
Conviven sin duplicar trabajo: react-query deduplica el refetch de una query
ya invalidada en la misma ventana. Reglas:

- Toda mutation invalida las listas globales que pinta su dominio + las
  claves con id (`use-samples.ts` es el patrón de referencia).
- Una nueva **vía de escritura en Rust sin mutation asociada** (hoy: la
  importación por carpeta vigilada) debe verificar que los eventos que
  dispara llegan a todas las pantallas afectadas — así se añadió
  `["dashboard"]` al flush de eventos: era la única pantalla que ninguna
  mutation refrescaba para esa vía.
- No invalidar desde mutations datos que solo otro subsistema escribe (el
  updater, el reloj): para eso están sus propios mecanismos.

## 5. Cuándo revisar este documento

- Nueva pantalla con datos de muestras/resultados: no añadas polling;
  verifica que las claves que pinta están en el flush de eventos (o en las
  mutations de su dominio).
- Nueva vía de escritura en el backend: recorre el checklist del §4.
- Si algún día la app es multiusuario o remota, este modelo sigue valiendo
  (los eventos son de la BD, no del proceso), pero la ventana de agrupación
  y la poda de EVENT_LOG deben re-evaluarse.
