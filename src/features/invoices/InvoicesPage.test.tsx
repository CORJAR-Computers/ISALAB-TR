import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { InvoicesPage } from "./InvoicesPage";
import type { InvoiceListItem } from "@/bindings";

/**
 * Facturación: los totales del listado son la cifra que el administrador lee
 * al cerrar el día. Las anuladas no deben sumar en «Facturado» ni en «Saldo
 * pendiente» aunque sigan visibles en la tabla.
 */

const mockUseInvoices = vi.fn();
const mockUseInvoiceCounts = vi.fn();
const mockUsePermissions = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useInvoices: (status: string | null, search: string) =>
    mockUseInvoices(status, search),
  useInvoiceCounts: () => mockUseInvoiceCounts(),
}));

vi.mock("@/hooks/use-permissions", () => ({
  usePermissions: () => mockUsePermissions(),
}));

vi.mock("@/stores/ui-store", () => ({
  useUiStore: (
    selector: (state: {
      entityRequest: null;
      consumeEntityRequest: () => void;
    }) => unknown,
  ) => selector({ entityRequest: null, consumeEntityRequest: () => {} }),
}));

// Los diálogos se abren solo al pulsar; se sustituyen para no arrastrar sus
// consultas al test del listado.
vi.mock("./NewInvoiceDialog", () => ({ NewInvoiceDialog: () => null }));
vi.mock("./InvoiceDetailDialog", () => ({ InvoiceDetailDialog: () => null }));
vi.mock("./PaymentDialog", () => ({ PaymentDialog: () => null }));

function invoice(overrides: Partial<InvoiceListItem> = {}): InvoiceListItem {
  return {
    id: 1,
    invoiceNumber: "FAC-000001",
    ownerName: "Ana Pérez",
    patientName: "Luna",
    issueDate: "2026-11-21 10:00:00",
    total: 119_000,
    status: "EMITIDA",
    paymentMethod: null,
    itemCount: 1,
    paidAmount: 0,
    balance: 119_000,
    paymentStatus: "PENDIENTE",
    ...overrides,
  };
}

beforeEach(() => {
  mockUseInvoices.mockReset();
  mockUseInvoiceCounts.mockReset();
  mockUsePermissions.mockResolvedValue?.(undefined);
  mockUsePermissions.mockReturnValue({ canBill: true, isAdmin: true });
  mockUseInvoiceCounts.mockReturnValue({ data: [] });
  mockUseInvoices.mockReturnValue({
    data: [invoice()],
    isLoading: false,
    isError: false,
  });
});

describe("InvoicesPage", () => {
  it("suma solo las facturas vigentes en los totales", () => {
    mockUseInvoices.mockReturnValue({
      data: [
        invoice(),
        invoice({
          id: 2,
          invoiceNumber: "FAC-000002",
          total: 50_000,
          status: "ANULADA",
          paymentStatus: "ANULADA",
          balance: 0,
        }),
      ],
      isLoading: false,
      isError: false,
    });

    render(<InvoicesPage />);

    // 119.000 facturado (en la tarjeta y en la fila); la anulada de 50.000
    // no entra en los totales, así que nunca aparece 169.000.
    expect(screen.getAllByText(/119\.000/).length).toBeGreaterThan(0);
    expect(screen.queryByText(/169\.000/)).not.toBeInTheDocument();
    expect(screen.getByText("FAC-000002")).toBeInTheDocument();
  });

  it("no ofrece emitir facturas a un rol sin capacidad de cobro", () => {
    mockUsePermissions.mockReturnValue({ canBill: false, isAdmin: false });
    render(<InvoicesPage />);
    expect(
      screen.queryByRole("button", { name: "Nueva factura" }),
    ).not.toBeInTheDocument();
  });

  it("muestra el estado vacío con la invitación a emitir la primera", () => {
    mockUseInvoices.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    });
    render(<InvoicesPage />);
    expect(screen.getByText(/Sin facturas/)).toBeInTheDocument();
  });

  it("distingue el error de carga de un listado vacío", () => {
    mockUseInvoices.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    });
    render(<InvoicesPage />);
    expect(
      screen.getByText("No se pudieron cargar las facturas."),
    ).toBeInTheDocument();
  });
});
