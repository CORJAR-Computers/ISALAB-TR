import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import type {
  LabResultChangedEvent,
  SampleChangedEvent,
} from "@/bindings";

/**
 * Puente Firebird → frontend:
 * el backend Rust escucha eventos nativos de Firebird (POST_EVENT) y los
 * re-emite como eventos de Tauri; aquí se invalidan las queries afectadas
 * para que la UI se actualice en tiempo real (trazabilidad de muestras).
 */
export function useFirebirdEvents() {
  const qc = useQueryClient();

  useEffect(() => {
    // `listen()` devuelve una Promise: si el componente se desmonta (o en
    // HMR de Vite) antes de que se resuelva, la limpieza correría sobre un
    // arreglo vacío y los listeners quedarían huérfanos. Con `cancelled`
    // garantizamos que, si la limpieza ocurre primero, cada listener recién
    // resuelto se elimine de inmediato; y si no, se acumule para el cleanup.
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    listen<SampleChangedEvent>("sample-changed", (event) => {
      const { patientId } = event.payload;
      qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["patient", patientId] });
    }).then((un) => {
      if (cancelled) un();
      else unlisteners.push(un);
    });

    listen<LabResultChangedEvent>("lab-result-changed", (event) => {
      const { patientId } = event.payload;
      qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", event.payload.sampleId] });
    }).then((un) => {
      if (cancelled) un();
      else unlisteners.push(un);
    });

    return () => {
      cancelled = true;
      for (const un of unlisteners) un();
    };
  }, [qc]);
}
