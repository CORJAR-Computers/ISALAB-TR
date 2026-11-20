import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { DashboardPage } from "./DashboardPage";

/**
 * Estabilidad del panel: con la base caída la consulta falla y la página se
 * quedaba en el esqueleto de carga para siempre. Ahora debe mostrar el motivo
 * y ofrecer reintentar.
 */
const mockUseDashboardStats = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useDashboardStats: () => mockUseDashboardStats(),
  useSetConsultationStatus: () => ({
    mutateAsync: vi.fn(),
    isPending: false,
  }),
}));

beforeEach(() => {
  mockUseDashboardStats.mockReset();
});

describe("DashboardPage (fallo de la consulta)", () => {
  it("muestra el error concreto y permite reintentar", () => {
    const refetch = vi.fn();
    mockUseDashboardStats.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: { type: "Database", data: "Firebird no responde" },
      refetch,
    });

    render(<DashboardPage />);

    expect(
      screen.getByText(/No se pudo cargar el panel de control/i),
    ).toBeInTheDocument();
    // role=alert expone el mensaje técnico a lectores de pantalla y tests.
    expect(screen.getByRole("alert")).toHaveTextContent(/Firebird no responde/);

    fireEvent.click(screen.getByRole("button", { name: /Reintentar/i }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it("no se queda cargando: el error tiene prioridad sobre los skeletons", () => {
    mockUseDashboardStats.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: true,
      error: new Error("boom"),
      refetch: vi.fn(),
    });

    render(<DashboardPage />);

    expect(
      screen.queryByText(/No se pudo cargar el panel de control/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(/boom/);
  });
});
