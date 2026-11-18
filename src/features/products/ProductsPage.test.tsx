import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { ProductsPage } from "./ProductsPage";
import type { ProductStockItem } from "@/bindings";

const mockUseProducts = vi.fn();
const mockUseStockMovements = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useProducts: () => mockUseProducts(),
  useStockMovements: () => mockUseStockMovements(),
  useProduct: () => ({ data: null }),
  useSaveProduct: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useSetProductActive: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useAddStockMovement: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

const mockUsePermissions = vi.fn();

vi.mock("@/hooks/use-permissions", () => ({
  usePermissions: () => mockUsePermissions(),
}));

function stockItem(overrides: Partial<ProductStockItem> = {}): ProductStockItem {
  return {
    product: {
      id: 1,
      code: "ALIM-001",
      name: "Alimento adulto 3 kg",
      category: "ALIMENTO",
      unit: "BOLSA",
      costPrice: 28000,
      salePrice: 45000,
      taxRate: 19,
      minStock: 5,
      active: true,
      notes: null,
    },
    stock: 12,
    lowStock: false,
    stockValue: 336000,
    ...overrides,
  };
}

describe("ProductsPage", () => {
  beforeEach(() => {
    mockUsePermissions.mockReturnValue({ canInventory: true, isAdmin: true });
    mockUseProducts.mockReturnValue({
      data: [stockItem()],
      isLoading: false,
      isError: false,
    });
    mockUseStockMovements.mockReturnValue({ data: [] });
  });

  it("muestra el catálogo con el stock actual", () => {
    render(<ProductsPage />);

    expect(screen.getByText("Inventario y productos")).toBeInTheDocument();
    expect(screen.getByText("ALIM-001")).toBeInTheDocument();
    expect(screen.getByText("Alimento adulto 3 kg")).toBeInTheDocument();
    // "Alimento" aparece como filtro de categoría y como badge.
    expect(screen.getAllByText("Alimento").length).toBeGreaterThan(0);
    // Stock 12 y valor 12 × 28.000.
    expect(screen.getByText("12")).toBeInTheDocument();
    // El formato de COP usa un espacio duro entre el símbolo y el número.
    expect(screen.getByText(/\$\s336\.000/)).toBeInTheDocument();
  });

  it("avisa de los productos por reponer", () => {
    mockUseProducts.mockReturnValue({
      data: [stockItem({ stock: 2, lowStock: true, stockValue: 56000 })],
      isLoading: false,
      isError: false,
    });
    render(<ProductsPage />);

    expect(screen.getByText("Stock por reponer")).toBeInTheDocument();
    expect(
      screen.getByText("Alimento adulto 3 kg (2/5)"),
    ).toBeInTheDocument();
  });

  it("oculta el módulo a los roles sin capacidad de inventario", () => {
    mockUsePermissions.mockReturnValue({ canInventory: false, isAdmin: false });
    render(<ProductsPage />);
    expect(
      screen.getByText("Tu rol no tiene acceso al inventario de la clínica."),
    ).toBeInTheDocument();
  });

  it("muestra un estado vacío cuando el catálogo no tiene productos", () => {
    mockUseProducts.mockReturnValue({
      data: [],
      isLoading: false,
      isError: false,
    });
    render(<ProductsPage />);
    expect(
      screen.getByText(/No hay productos con esos filtros/),
    ).toBeInTheDocument();
  });
});
