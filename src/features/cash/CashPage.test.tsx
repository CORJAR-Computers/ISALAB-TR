import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CashPage } from "./CashPage";

/**
 * Caja: la apertura es el primer paso de cualquier cobro. Se comprueba que la
 * base inicial se valida y se envía como número (no como texto con separadores)
 * y que la pantalla distingue cargando, sin sesión y con sesión abierta.
 */

const mockUseOpenCashSession = vi.fn();
const mockUseCashSummary = vi.fn();
const mockUseCashSessions = vi.fn();
const mockOpenCash = vi.fn();
const mockGeneratePdf = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useOpenCashSession: () => mockUseOpenCashSession(),
  useCashSummary: () => mockUseCashSummary(),
  useCashSessions: () => mockUseCashSessions(),
  useOpenCash: () => mockOpenCash(),
  useGenerateCashClosePdf: () => ({
    mutateAsync: mockGeneratePdf,
    isPending: false,
  }),
}));

vi.mock("@/lib/api", () => ({
  api: { openReportFile: vi.fn() },
  getErrorMessage: (error: unknown) => String(error),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openPath: vi.fn() }));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("./CashMovementDialog", () => ({ CashMovementDialog: () => null }));
vi.mock("./CloseCashDialog", () => ({ CloseCashDialog: () => null }));

beforeEach(() => {
  mockUseOpenCashSession.mockReset();
  mockUseCashSummary.mockReset();
  mockUseCashSessions.mockReset();
  mockOpenCash.mockReset();
  mockGeneratePdf.mockReset();
  mockGeneratePdf.mockResolvedValue({ path: "C:/reportes/cierre.pdf" });

  mockUseOpenCashSession.mockReturnValue({ data: null, isLoading: false });
  mockUseCashSummary.mockReturnValue({ data: null, isLoading: false });
  mockUseCashSessions.mockReturnValue({ data: [] });
  mockOpenCash.mockReturnValue({
    mutateAsync: vi.fn().mockResolvedValue(undefined),
    isPending: false,
  });
});

describe("CashPage", () => {
  it("sin sesión abierta pide la base inicial y abre la caja", async () => {
    render(<CashPage />);

    // "Abrir caja" es el título de la tarjeta y el botón: existen ambos.
    expect(screen.getAllByText("Abrir caja").length).toBe(2);
    const base = screen.getByLabelText("Base inicial");
    await userEvent.clear(base);
    await userEvent.type(base, "150.000");
    await userEvent.click(screen.getByRole("button", { name: "Abrir caja" }));

    const mutateAsync = mockOpenCash.mock.results[0].value.mutateAsync;
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith({
        openingFloat: 150_000,
        notes: null,
      }),
    );
  });

  it("no pide abrir caja cuando ya hay una sesión abierta", () => {
    mockUseOpenCashSession.mockReturnValue({
      data: { id: 1, code: "CAJ-00000001", status: "ABIERTA" },
      isLoading: false,
    });
    render(<CashPage />);

    expect(
      screen.queryByRole("button", { name: "Abrir caja" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar caja" })).toBeInTheDocument();
    // Sin resumen cargado el cierre está deshabilitado: no se cierra a ciegas.
    expect(screen.getByRole("button", { name: "Cerrar caja" })).toBeDisabled();
  });

  it("con sesión abierta muestra el resumen y permite cerrar", () => {
    mockUseOpenCashSession.mockReturnValue({
      data: { id: 1, code: "CAJ-00000001", status: "ABIERTA" },
      isLoading: false,
    });
    mockUseCashSummary.mockReturnValue({
      isLoading: false,
      data: {
        session: {
          id: 1,
          code: "CAJ-00000001",
          openedAt: "2026-11-21 08:00:00",
          openedByName: "Administrador",
          openingFloat: 100_000,
          status: "ABIERTA",
        },
        openingFloat: 100_000,
        expectedCash: 250_000,
        countedCash: null,
        difference: null,
        totalBilled: 1_190_000,
        totalCollected: 1_190_000,
        cashIn: 1_190_000,
        cashOut: 100_000,
        paymentsCount: 3,
        methods: [{ methodCode: "EFECTIVO", methodName: "Efectivo", amount: 1_190_000 }],
        movements: [
          {
            id: 9,
            kind: "GASTO",
            amount: -100_000,
            reason: "Compra de insumos",
            createdAt: "2026-11-21 12:00:00",
            createdByName: "Administrador",
          },
        ],
        countLines: [],
      },
    });

    render(<CashPage />);

    expect(screen.getByText("Sesión CAJ-00000001")).toBeInTheDocument();
    expect(screen.getByText("Efectivo")).toBeInTheDocument();
    expect(screen.getByText("Compra de insumos")).toBeInTheDocument();
    // 1.190.000 cobrado + 100.000 de base = 250.000 esperado ?? no: el
    // esperado lo calcula el backend; aquí se muestra tal cual.
    expect(screen.getAllByText(/250\.000/).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Cerrar caja" })).toBeEnabled();
  });

  it("genera el PDF de una caja cerrada desde el historial", async () => {
    mockUseCashSessions.mockReturnValue({
      data: [
        {
          id: 7,
          code: "CAJ-00000001",
          openedAt: "2026-11-20 08:00:00",
          closedAt: "2026-11-20 18:00:00",
          status: "CERRADA",
          expectedCash: 250_000,
          countedCash: 245_000,
          difference: -5_000,
        },
      ],
    });

    render(<CashPage />);

    fireEvent.click(screen.getByRole("button", { name: /Cierre/ }));
    await waitFor(() => expect(mockGeneratePdf).toHaveBeenCalledWith(7));
    // El faltante del arqueo se muestra con su signo.
    expect(screen.getByText(/-\$\s?5\.000/)).toBeInTheDocument();
  });

  it("mientras carga la sesión muestra el esqueleto, no un formulario", () => {
    mockUseOpenCashSession.mockReturnValue({ data: null, isLoading: true });
    const { container } = render(<CashPage />);

    expect(screen.queryByText("Abrir caja")).not.toBeInTheDocument();
    expect(container.querySelectorAll('[data-slot="skeleton"]').length).toBeGreaterThan(
      0,
    );
  });

  it("acepta la base escrita con separador de miles y coma decimal", async () => {
    render(<CashPage />);

    const base = screen.getByLabelText("Base inicial");
    await userEvent.clear(base);
    await userEvent.type(base, "1.250,50");
    await userEvent.click(screen.getByRole("button", { name: "Abrir caja" }));

    const mutateAsync = mockOpenCash.mock.results[0].value.mutateAsync;
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith({
        openingFloat: 1250.5,
        notes: null,
      }),
    );
  });
});
