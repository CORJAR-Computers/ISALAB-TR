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
    const unlisteners: Array<() => void> = [];

    listen<SampleChangedEvent>("sample-changed", (event) => {
      const { patientId } = event.payload;
      qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["patient", patientId] });
    }).then((un) => unlisteners.push(un));

    listen<LabResultChangedEvent>("lab-result-changed", (event) => {
      const { patientId } = event.payload;
      qc.invalidateQueries({ queryKey: ["clinical-history", patientId] });
      qc.invalidateQueries({ queryKey: ["samples"] });
      qc.invalidateQueries({ queryKey: ["sample-counts"] });
      qc.invalidateQueries({ queryKey: ["worklist"] });
      qc.invalidateQueries({ queryKey: ["sample", event.payload.sampleId] });
    }).then((un) => unlisteners.push(un));

    return () => {
      for (const un of unlisteners) un();
    };
  }, [qc]);
}
