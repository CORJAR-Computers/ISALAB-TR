import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { StaysPage } from "./StaysPage";
import type { ResourceBoardItem, StayListItem } from "@/bindings";

const mockUseResourceBoard = vi.fn();
const mockUseResources = vi.fn();
const mockUseStays = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useResourceBoard: () => mockUseResourceBoard(),
  useResources: () => mockUseResources(),
  useStays: () => mockUseStays(),
  usePatients: () => ({ data: [] }),
  useStay: () => ({ data: null }),
  useStayEvents: () => ({ data: [] }),
  useSaveResource: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useCreateStay: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useCheckOutStay: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useCancelStay: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useAddStayEvent: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

const mockUsePermissions = vi.fn();

vi.mock("@/hooks/use-permissions", () => ({
  usePermissions: () => mockUsePermissions(),
}));

function boardItem(
  overrides: Partial<ResourceBoardItem> = {},
): ResourceBoardItem {
  return {
    resource: {
      id: 1,
      name: "Canil 1",
      kind: "CANIL",
      capacity: 1,
      active: true,
      notes: null,
    },
    occupied: 0,
    needsVet: 0,
    ...overrides,
  };
}

function stayItem(overrides: Partial<StayListItem> = {}): StayListItem {
  return {
    id: 1,
    code: "E-2026-0001",
    patientId: 1,
    patientName: "Luna",
    ownerName: "Juan Pérez",
    resourceName: "Canil 1",
    kind: "HOSPEDAJE",
    status: "ABIERTA",
    checkInAt: "2026-11-15 08:00:00",
    checkOutAt: null,
    days: 2,
    estimatedTotal: 90000,
    ...overrides,
  };
}

describe("StaysPage", () => {
  beforeEach(() => {
    mockUsePermissions.mockReturnValue({ canStay: true, isAdmin: true });
    mockUseResourceBoard.mockReturnValue({
      data: [boardItem()],
      isLoading: false,
    });
    mockUseResources.mockReturnValue({ data: [boardItem().resource] });
    mockUseStays.mockReturnValue({
      data: [stayItem()],
      isLoading: false,
    });
  });

  it("muestra el tablero de ocupación y el listado de estancias", () => {
    render(<StaysPage />);

    expect(screen.getByText("Estancia y hospedaje")).toBeInTheDocument();
    expect(screen.getByText("Ocupación de recursos")).toBeInTheDocument();
    // "Canil 1" aparece en el tablero y en la fila de la estancia.
    expect(screen.getAllByText("Canil 1").length).toBeGreaterThan(0);
    expect(screen.getByText("0/1")).toBeInTheDocument();

    // La estancia abierta aparece en el listado con su código y paciente.
    expect(screen.getByText("E-2026-0001")).toBeInTheDocument();
    expect(screen.getByText("Luna")).toBeInTheDocument();
    // "Abierta" aparece como filtro y como badge de la fila.
    expect(screen.getAllByText("Abierta").length).toBeGreaterThan(0);
  });

  it("cuenta las estancias abiertas y la tarifa estimada", () => {
    render(<StaysPage />);

    expect(screen.getByText("Estancias abiertas")).toBeInTheDocument();
    // 1 estancia abierta.
    expect(screen.getByText("Recursos ocupados")).toBeInTheDocument();
    // El formato de COP usa un espacio duro entre el símbolo y el número.
    // El importe aparece en el indicador y en la fila de la estancia.
    expect(screen.getAllByText(/\$\s90\.000/).length).toBeGreaterThan(0);
  });

  it("avisa cuando el recurso está al límite de capacidad", () => {
    mockUseResourceBoard.mockReturnValue({
      data: [boardItem({ occupied: 1 })],
      isLoading: false,
    });
    render(<StaysPage />);
    expect(screen.getByText("1/1")).toBeInTheDocument();
  });

  it("oculta el módulo a los roles sin capacidad de estancia", () => {
    mockUsePermissions.mockReturnValue({ canStay: false, isAdmin: false });
    render(<StaysPage />);
    expect(
      screen.getByText("Tu rol no tiene acceso al módulo de estancia."),
    ).toBeInTheDocument();
  });

  it("muestra un estado vacío cuando no hay estancias", () => {
    mockUseStays.mockReturnValue({ data: [], isLoading: false });
    render(<StaysPage />);
    expect(
      screen.getByText("No hay estancias con esos filtros."),
    ).toBeInTheDocument();
  });
});
