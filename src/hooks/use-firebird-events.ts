import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import type {
  LabResultChangedEvent,
  SampleChangedEvent,
} from "@/bindings";

/**
 * Ventana de agrupación (ms): los eventos que llegan dentro de la ventana
 * comparten una sola invalidación. Guardar un panel de 30 analitos dispara
 * 30 eventos LAB_RESULT_CHANGED en ráfaga; sin agrupación, cada uno
 * re-fetcheaba todos los listados montados (30 × 4 queries redundantes).
 * 150 ms es imperceptible para trazabilidad en vivo y sobra para que llegue
 * la ráfaga completa (los eventos son locales, del mismo proceso).
 */
const FLUSH_WINDOW_MS = 150;

/**
 * Invalida las queries afectadas por un evento, recibiendo el conjunto de
 * patientIds acumulados en la ventana (para no repetir con varios eventos
 * del mismo paciente).
 */
type InvalidateFn = (qc: QueryClient, patientIds: Set<number>, sampleIds: Set<number>) => void;

const invalidateSampleEvent: InvalidateFn = (qc, patientIds) => {
  qc.invalidateQueries({ queryKey: ["samples"] });
  qc.invalidateQueries({ queryKey: ["sample-counts"] });
  qc.invalidateQueries({ queryKey: ["worklist"] });
  qc.invalidateQueries({ queryKey: ["dashboard"] });
  for (const patientId of patientIds) {
    qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
    qc.invalidateQueries({ queryKey: ["patient", patientId] });
  }
};

const invalidateLabResultEvent: InvalidateFn = (qc, patientIds, sampleIds) => {
  qc.invalidateQueries({ queryKey: ["samples"] });
  qc.invalidateQueries({ queryKey: ["sample-counts"] });
  qc.invalidateQueries({ queryKey: ["worklist"] });
  qc.invalidateQueries({ queryKey: ["dashboard"] });
  for (const patientId of patientIds) {
    qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
  }
  for (const sampleId of sampleIds) {
    qc.invalidateQueries({ queryKey: ["sample", sampleId] });
  }
};

/**
 * Temporizador por tipo de evento: acumula claves en Sets y agenda un único
 * flush al final de la ventana. Un flush nuevo durante la ventana reutiliza
 * el temporizador ya agendado (no lo retrasa): la ráfaga completa se invalida
 * junta, una sola vez.
 */
class EventBatcher {
  private samplePatientIds = new Set<number>();
  private resultPatientIds = new Set<number>();
  private resultSampleIds = new Set<number>();
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(private qc: QueryClient) {}

  /** Encola un evento de muestra y agenda (o reutiliza) el flush. */
  pushSample(patientId: number) {
    this.samplePatientIds.add(patientId);
    this.scheduleFlush();
  }

  /** Encola un evento de resultado y agenda (o reutiliza) el flush. */
  pushLabResult(patientId: number, sampleId: number) {
    this.resultPatientIds.add(patientId);
    this.resultSampleIds.add(sampleId);
    this.scheduleFlush();
  }

  /** Vacía lo acumulado, invalidando cada tipo una sola vez. */
  flush = () => {
    this.timer = null;
    if (this.samplePatientIds.size > 0) {
      invalidateSampleEvent(this.qc, this.samplePatientIds, new Set());
      this.samplePatientIds.clear();
    }
    if (this.resultPatientIds.size > 0 || this.resultSampleIds.size > 0) {
      invalidateLabResultEvent(this.qc, this.resultPatientIds, this.resultSampleIds);
      this.resultPatientIds.clear();
      this.resultSampleIds.clear();
    }
  };

  /** Libera el temporizador pendiente (cleanup del efecto/unmount). */
  dispose() {
    if (this.timer != null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private scheduleFlush() {
    if (this.timer != null) return; // ya hay un flush agendado: se comparte
    this.timer = setTimeout(this.flush, FLUSH_WINDOW_MS);
  }
}

/**
 * Puente Firebird → frontend:
 * el backend Rust escucha eventos nativos de Firebird (POST_EVENT) y los
 * re-emite como eventos de Tauri; aquí se invalidan las queries afectadas
 * para que la UI se actualice en tiempo real (trazabilidad de muestras).
 *
 * Las invalidaciones se agrupan en una ventana de FLUSH_WINDOW_MS: una
 * ráfaga de N eventos (p. ej. guardar un panel de 30 analitos) produce UNA
 * invalidación por query afectada en lugar de N. La carga por evento es
 * mínima (dos `Set.add`), por lo que no hay complejidad adicional apreciable.
 *
 * El dashboard también se invalida aquí: sus métricas derivan de
 * muestras/resultados y las escrituras en segundo plano del backend
 * (importación por carpeta vigilada del analizador) solo llegan al frontend
 * vía estos eventos — no hay un mutation que refresque esa pantalla.
 */
export function useFirebirdEvents() {
  const qc = useQueryClient();

  useEffect(() => {
    const batcher = new EventBatcher(qc);

    // `listen()` devuelve una Promise: si el componente se desmonta (o en
    // HMR de Vite) antes de que se resuelva, la limpieza correría sobre un
    // arreglo vacío y los listeners quedarían huérfanos. Con `cancelled`
    // garantizamos que, si la limpieza ocurre primero, cada listener recién
    // resuelto se elimine de inmediato; y si no, se acumule para el cleanup.
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    listen<SampleChangedEvent>("sample-changed", (event) => {
      batcher.pushSample(event.payload.patientId);
    }).then((un) => {
      if (cancelled) un();
      else unlisteners.push(un);
    });

    listen<LabResultChangedEvent>("lab-result-changed", (event) => {
      batcher.pushLabResult(event.payload.patientId, event.payload.sampleId);
    }).then((un) => {
      if (cancelled) un();
      else unlisteners.push(un);
    });

    return () => {
      cancelled = true;
      batcher.dispose();
      for (const un of unlisteners) un();
    };
  }, [qc]);
}
