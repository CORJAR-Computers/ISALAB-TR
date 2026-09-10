import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useFirebirdEvents } from "./use-firebird-events";

/**
 * Mock del módulo de eventos de Tauri: captura los handlers registrados para
 * poder dispararles eventos de prueba de forma síncrona.
 */
const registeredHandlers = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    (name: string, handler: (event: { payload: unknown }) => void) => {
      registeredHandlers.set(name, handler);
      return Promise.resolve(() => registeredHandlers.delete(name));
    },
  ),
}));

function emit(name: string, payload: unknown) {
  const handler = registeredHandlers.get(name);
  if (!handler) throw new Error(`Sin listener para ${name}`);
  handler({ payload });
}

const PATIENT_PAYLOAD = { sampleId: 7, patientId: 42, status: "EN_PROCESO" };

/** Wrapper con QueryClient espía: cuenta invalidaciones por clave base. */
function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const invalidateSpy = vi.spyOn(qc, "invalidateQueries");
  return {
    qc,
    invalidateSpy,
    wrapper: ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={qc}>{children}</QueryClientProvider>
    ),
  };
}

/** Invalidaciones recibidas por clave base (primer elemento del array). */
function invalidationKeys(spy: ReturnType<typeof vi.spyOn>): string[] {
  return spy.mock.calls.map((call: unknown[]) => String((call[0] as { queryKey?: unknown[] })?.queryKey?.[0]));
}

beforeEach(() => {
  vi.useFakeTimers();
  registeredHandlers.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useFirebirdEvents (agrupación de invalidaciones)", () => {
  it("una ráfaga de 30 eventos de resultado produce UNA invalidación por query", async () => {
    const { invalidateSpy, wrapper } = makeWrapper();
    renderHook(() => useFirebirdEvents(), { wrapper });

    // Simula el guardado de un panel de 30 analitos de la misma muestra:
    // 30 eventos lab-result-changed en ráfaga (mismo paciente/muestra).
    await act(async () => {
      for (let i = 0; i < 30; i += 1) {
        emit("lab-result-changed", PATIENT_PAYLOAD);
      }
    });

    // Antes de la ventana: nada se ha invalidado todavía.
    expect(invalidateSpy).not.toHaveBeenCalled();

    // Al vencer la ventana, un único flush.
    await act(async () => {
      vi.advanceTimersByTime(150);
    });

    const keys = invalidationKeys(invalidateSpy);
    expect(keys.filter((k) => k === "samples")).toHaveLength(1);
    expect(keys.filter((k) => k === "sample-counts")).toHaveLength(1);
    expect(keys.filter((k) => k === "worklist")).toHaveLength(1);
    expect(keys.filter((k) => k === "dashboard")).toHaveLength(1);
    expect(keys.filter((k) => k === "clinical-history")).toHaveLength(1);
    expect(keys.filter((k) => k === "sample")).toHaveLength(1);
    expect(invalidateSpy).toHaveBeenCalledTimes(6);
  });

  it("varios pacientes en la misma ventana se deduplican por id", async () => {
    const { invalidateSpy, wrapper } = makeWrapper();
    renderHook(() => useFirebirdEvents(), { wrapper });

    await act(async () => {
      for (const patientId of [1, 2, 3, 1, 2, 3, 1]) {
        emit("sample-changed", { sampleId: 10, patientId, status: "RECIBIDA" });
      }
    });
    await act(async () => {
      vi.advanceTimersByTime(150);
    });

    const keys = invalidationKeys(invalidateSpy);
    // Listados globales: una vez; por paciente: una vez cada uno (dedupe).
    expect(keys.filter((k) => k === "samples")).toHaveLength(1);
    expect(keys.filter((k) => k === "clinical-history")).toHaveLength(3);
    expect(keys.filter((k) => k === "patient")).toHaveLength(3);
  });

  it("mezcla de eventos de muestra y de resultado en la misma ventana", async () => {
    const { invalidateSpy, wrapper } = makeWrapper();
    renderHook(() => useFirebirdEvents(), { wrapper });

    await act(async () => {
      emit("sample-changed", PATIENT_PAYLOAD);
      emit("lab-result-changed", PATIENT_PAYLOAD);
      emit("lab-result-changed", { ...PATIENT_PAYLOAD, sampleId: 8 });
    });
    await act(async () => {
      vi.advanceTimersByTime(150);
    });

    const keys = invalidationKeys(invalidateSpy);
    // Cada tipo de evento invalida una vez, aunque lleguen varios eventos.
    expect(keys.filter((k) => k === "samples")).toHaveLength(2); // 1 por tipo
    expect(keys.filter((k) => k === "worklist")).toHaveLength(2);
    expect(keys.filter((k) => k === "patient")).toHaveLength(1); // solo sample
    expect(keys.filter((k) => k === "sample")).toHaveLength(2); // muestras 7 y 8
  });

  it("eventos fuera de la ventana se invalidan en flushes separados", async () => {
    const { invalidateSpy, wrapper } = makeWrapper();
    renderHook(() => useFirebirdEvents(), { wrapper });

    await act(async () => {
      emit("lab-result-changed", PATIENT_PAYLOAD);
    });
    await act(async () => {
      vi.advanceTimersByTime(150);
    });
    expect(invalidateSpy).toHaveBeenCalledTimes(6);

    invalidateSpy.mockClear();
    await act(async () => {
      emit("lab-result-changed", { ...PATIENT_PAYLOAD, sampleId: 9 });
    });
    await act(async () => {
      vi.advanceTimersByTime(150);
    });
    const keys = invalidationKeys(invalidateSpy);
    expect(keys.filter((k) => k === "sample")).toHaveLength(1);
    expect(invalidateSpy).toHaveBeenCalledTimes(6); // flush completo de nuevo
  });

  it("el unmount cancela el flush pendiente sin invalidar", async () => {
    const { invalidateSpy, wrapper } = makeWrapper();
    const { unmount } = renderHook(() => useFirebirdEvents(), { wrapper });

    await act(async () => {
      emit("lab-result-changed", PATIENT_PAYLOAD);
    });
    unmount();

    await act(async () => {
      vi.advanceTimersByTime(300);
    });
    expect(invalidateSpy).not.toHaveBeenCalled();
  });
});
