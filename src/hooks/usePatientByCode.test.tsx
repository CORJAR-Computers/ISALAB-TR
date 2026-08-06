import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { usePatientByCode } from "./use-queries";
import { api } from "@/lib/api";
import type { Patient } from "@/bindings";

// Solo se ejercita getPatientByCode desde usePatientByCode.
vi.mock("@/lib/api", () => ({
  api: {
    getPatientByCode: vi.fn(),
  },
}));

const mockedGetPatientByCode = vi.mocked(api.getPatientByCode);

const PATIENT: Patient = {
  id: 1,
  code: "PAC-2026-0001",
  ownerId: 10,
  speciesId: 1,
  breedId: 1,
  name: "Rocky",
  sex: "M",
  birthDate: "2022-03-15",
  neutered: true,
  color: "Dorado",
  microchip: "1234567890",
  active: true,
  notes: null,
  preferredLogoId: null,
  speciesName: "Canino",
  breedName: "Labrador",
  ownerName: "Juan Pérez",
  ownerPhone: "3001234567",
  ageMonths: 52,
};

/** Wrapper con un QueryClient nuevo por test (caché limpia, sin retries). */
function makeWrapper() {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return {
    wrapper: ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={qc}>{children}</QueryClientProvider>
    ),
  };
}

beforeEach(() => {
  mockedGetPatientByCode.mockReset();
  mockedGetPatientByCode.mockResolvedValue(PATIENT);
});

describe("usePatientByCode", () => {
  it("normaliza con trim antes de consultar y devuelve los datos del paciente", async () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => usePatientByCode("  PAC-2026-0001  "), {
      wrapper,
    });

    expect(result.current.isLoading).toBe(true);
    await waitFor(() => expect(result.current.data).toEqual(PATIENT));

    // El código llega a la API ya normalizado (sin espacios).
    expect(mockedGetPatientByCode).toHaveBeenCalledTimes(1);
    expect(mockedGetPatientByCode).toHaveBeenCalledWith("PAC-2026-0001");
    expect(result.current.isSuccess).toBe(true);
  });

  it("devuelve null (no encontrado) cuando la API no tiene el paciente", async () => {
    mockedGetPatientByCode.mockResolvedValueOnce(null);
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => usePatientByCode("PAC-2026-9999"), {
      wrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toBeNull();
  });

  it("no consulta con código null", () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => usePatientByCode(null), { wrapper });

    // Query deshabilitada: nunca llama a la API y no hay datos.
    expect(result.current.data).toBeUndefined();
    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedGetPatientByCode).not.toHaveBeenCalled();
  });

  it("no consulta con código vacío o solo espacios", () => {
    const { wrapper } = makeWrapper();
    const { result } = renderHook(() => usePatientByCode("   "), { wrapper });

    expect(result.current.data).toBeUndefined();
    expect(result.current.fetchStatus).toBe("idle");
    expect(mockedGetPatientByCode).not.toHaveBeenCalled();
  });

  it("reusa la caché para códigos equivalentes tras el trim (una sola llamada)", async () => {
    const { wrapper } = makeWrapper();
    const { result, rerender } = renderHook(
      ({ code }: { code: string | null }) => usePatientByCode(code),
      { initialProps: { code: "  PAC-2026-0001 " }, wrapper },
    );

    await waitFor(() => expect(result.current.data).toEqual(PATIENT));
    expect(mockedGetPatientByCode).toHaveBeenCalledTimes(1);

    // Mismo código normalizado → misma queryKey → sin llamada nueva.
    rerender({ code: "PAC-2026-0001  " });
    await waitFor(() => expect(result.current.data).toEqual(PATIENT));
    expect(mockedGetPatientByCode).toHaveBeenCalledTimes(1);
  });
});
